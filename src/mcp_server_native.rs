//! Native MCP Server — WebSocket Server for Sassy Browser
//!
//! Pure Rust, bincode over WebSocket. NO JSON-RPC.
//!
//! This server exposes browser automation capabilities to external MCP clients
//! (Claude Desktop, custom agents, etc.) using our native binary protocol.
//!
//! ARCHITECTURE:
//! ─────────────────────────────────────────────────────────────────────────
//! External MCP Client ──WebSocket──▶ McpNativeServer
//!                                       │
//!                                       ▼
//!                                  Command Channel
//!                                       │
//!                                       ▼
//!                                  BrowserApp (egui)
//!                                       │
//!                                       ▼
//!                                  Response Channel
//!                                       │
//!                                       ▼
//! External MCP Client ◀──WebSocket── McpNativeServer
//!
//! The server runs on a tokio runtime in a background thread.
//! Commands and responses are bridged to the egui render loop via channels.

use crate::mcp_codec::{build_frame_body, parse_frame_body};
use crate::mcp_protocol::{
    McpCommand, McpResponse, ErrorCode, NotificationType, PROTOCOL_VERSION,
};
use crate::detection::DetectionAlertPayload;
use futures::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as WsMessage;

// ═══════════════════════════════════════════════════════════════════════════════
// SERVER CONFIGURATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Native MCP server configuration
#[derive(Debug, Clone)]
pub struct NativeServerConfig {
    /// Port to listen on (default: 9998 — one below the JSON-RPC MCP server)
    pub port: u16,
    /// Bind address (default: 127.0.0.1 — localhost only)
    pub bind_addr: String,
    /// Maximum concurrent clients
    pub max_clients: usize,
    /// Enable detection alert forwarding
    pub forward_detection_alerts: bool,
}

impl Default for NativeServerConfig {
    fn default() -> Self {
        Self {
            port: 9998,
            bind_addr: "127.0.0.1".into(),
            max_clients: 4,
            forward_detection_alerts: true,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// COMMAND/RESPONSE BRIDGE
// ═══════════════════════════════════════════════════════════════════════════════

/// Bridge between the async MCP server and the sync egui render loop.
///
/// The server pushes commands into `pending_commands`.
/// The app polls `pending_commands` each frame and pushes responses into `pending_responses`.
/// The server reads `pending_responses` and sends them back to the client.
pub struct McpBridge {
    /// Commands waiting to be processed by the app
    pub pending_commands: Arc<Mutex<Vec<(u64, McpCommand)>>>,
    /// Responses waiting to be sent back to clients
    pub pending_responses: Arc<Mutex<Vec<(u64, McpResponse)>>>,
    /// Detection alerts to forward to connected clients
    pub detection_alerts: Arc<Mutex<Vec<DetectionAlertPayload>>>,
    /// Sequence counter for command/response correlation
    next_seq: Arc<Mutex<u64>>,
    /// Whether the server is running
    pub running: Arc<AtomicBool>,
}

impl McpBridge {
    pub fn new() -> Self {
        Self {
            pending_commands: Arc::new(Mutex::new(Vec::new())),
            pending_responses: Arc::new(Mutex::new(Vec::new())),
            detection_alerts: Arc::new(Mutex::new(Vec::new())),
            next_seq: Arc::new(Mutex::new(1)),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Push a command into the bridge (called by the server)
    pub fn push_command(&self, cmd: McpCommand) -> u64 {
        let mut seq = self.next_seq.lock().unwrap();
        let id = *seq;
        *seq += 1;
        drop(seq);

        if let Ok(mut cmds) = self.pending_commands.lock() {
            cmds.push((id, cmd));
        }
        id
    }

    /// Take all pending commands (called by the app each frame)
    pub fn take_commands(&self) -> Vec<(u64, McpCommand)> {
        if let Ok(mut cmds) = self.pending_commands.lock() {
            std::mem::take(&mut *cmds)
        } else {
            Vec::new()
        }
    }

    /// Push a response (called by the app after processing a command)
    pub fn push_response(&self, seq: u64, resp: McpResponse) {
        if let Ok(mut resps) = self.pending_responses.lock() {
            resps.push((seq, resp));
        }
    }

    /// Take all pending responses (called by the server to send back to clients)
    pub fn take_responses(&self) -> Vec<(u64, McpResponse)> {
        if let Ok(mut resps) = self.pending_responses.lock() {
            std::mem::take(&mut *resps)
        } else {
            Vec::new()
        }
    }

    /// Push detection alerts for forwarding
    pub fn push_detection_alerts(&self, alerts: Vec<DetectionAlertPayload>) {
        if alerts.is_empty() { return; }
        if let Ok(mut queue) = self.detection_alerts.lock() {
            queue.extend(alerts);
            // Cap at 100 pending
            let len = queue.len();
            if len > 100 {
                queue.drain(0..len - 100);
            }
        }
    }

    /// Take pending detection alerts
    pub fn take_detection_alerts(&self) -> Vec<DetectionAlertPayload> {
        if let Ok(mut queue) = self.detection_alerts.lock() {
            std::mem::take(&mut *queue)
        } else {
            Vec::new()
        }
    }
}

impl Default for McpBridge {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// NATIVE MCP SERVER
// ═══════════════════════════════════════════════════════════════════════════════

/// Native MCP WebSocket server.
///
/// Runs in a background tokio runtime thread.
/// Communicates with the egui app via McpBridge.
pub struct McpNativeServer {
    config: NativeServerConfig,
    bridge: Arc<McpBridge>,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl McpNativeServer {
    pub fn new(config: NativeServerConfig) -> Self {
        Self {
            config,
            bridge: Arc::new(McpBridge::new()),
            thread_handle: None,
        }
    }

    /// Get the bridge for app-side communication
    pub fn bridge(&self) -> Arc<McpBridge> {
        self.bridge.clone()
    }

    /// Start the server in a background thread with its own tokio runtime.
    pub fn start(&mut self) -> Result<(), String> {
        if self.bridge.running.load(Ordering::SeqCst) {
            return Err("server already running".into());
        }

        let config = self.config.clone();
        let bridge = self.bridge.clone();
        bridge.running.store(true, Ordering::SeqCst);

        let handle = std::thread::Builder::new()
            .name("mcp-native-server".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("failed to create tokio runtime for MCP server");

                rt.block_on(run_server(config, bridge));
            })
            .map_err(|e| format!("failed to spawn server thread: {}", e))?;

        self.thread_handle = Some(handle);
        tracing::info!("Native MCP server starting on {}:{}", self.config.bind_addr, self.config.port);
        Ok(())
    }

    /// Stop the server.
    pub fn stop(&mut self) {
        self.bridge.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    /// Check if the server is running.
    pub fn is_running(&self) -> bool {
        self.bridge.running.load(Ordering::SeqCst)
    }
}

impl Drop for McpNativeServer {
    fn drop(&mut self) {
        self.stop();
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SERVER LOOP
// ═══════════════════════════════════════════════════════════════════════════════

async fn run_server(config: NativeServerConfig, bridge: Arc<McpBridge>) {
    let addr: SocketAddr = format!("{}:{}", config.bind_addr, config.port)
        .parse()
        .expect("invalid bind address");

    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("Native MCP server failed to bind to {}: {}", addr, e);
            bridge.running.store(false, Ordering::SeqCst);
            return;
        }
    };

    tracing::info!("Native MCP server listening on {}", addr);

    let active_clients = Arc::new(Mutex::new(0usize));

    while bridge.running.load(Ordering::SeqCst) {
        // Accept connections with a timeout so we can check the running flag
        let accept_result = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            listener.accept(),
        ).await;

        match accept_result {
            Ok(Ok((stream, peer_addr))) => {
                // Check client limit
                let count = {
                    let c = active_clients.lock().unwrap();
                    *c
                };
                if count >= config.max_clients {
                    tracing::warn!("Rejecting connection from {} — max clients reached", peer_addr);
                    continue;
                }

                // Increment client count
                {
                    let mut c = active_clients.lock().unwrap();
                    *c += 1;
                }

                let bridge_clone = bridge.clone();
                let active_clone = active_clients.clone();
                let forward_alerts = config.forward_detection_alerts;

                tokio::spawn(async move {
                    tracing::info!("Native MCP client connected: {}", peer_addr);

                    if let Err(e) = handle_client(stream, bridge_clone, forward_alerts).await {
                        tracing::warn!("Client {} disconnected: {}", peer_addr, e);
                    } else {
                        tracing::info!("Client {} disconnected cleanly", peer_addr);
                    }

                    // Decrement client count
                    let mut c = active_clone.lock().unwrap();
                    *c = c.saturating_sub(1);
                });
            }
            Ok(Err(e)) => {
                tracing::error!("Accept error: {}", e);
            }
            Err(_) => {
                // Timeout — loop back to check running flag
            }
        }
    }

    tracing::info!("Native MCP server shutting down");
    bridge.running.store(false, Ordering::SeqCst);
}

// ═══════════════════════════════════════════════════════════════════════════════
// CLIENT HANDLER
// ═══════════════════════════════════════════════════════════════════════════════

async fn handle_client(
    stream: tokio::net::TcpStream,
    bridge: Arc<McpBridge>,
    forward_alerts: bool,
) -> Result<(), String> {
    // Upgrade TCP to WebSocket
    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .map_err(|e| format!("WebSocket upgrade failed: {}", e))?;

    let (mut ws_sink, mut ws_stream) = ws_stream.split();

    // Wait for Hello handshake
    let hello_msg = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        ws_stream.next(),
    )
    .await
    .map_err(|_| "handshake timeout".to_string())?
    .ok_or("connection closed before handshake")?
    .map_err(|e| format!("handshake error: {}", e))?;

    let hello_bytes = match hello_msg {
        WsMessage::Binary(b) => b,
        _ => return Err("expected binary Hello message".into()),
    };

    let hello: McpCommand = parse_frame_body(&hello_bytes)
        .map_err(|e| format!("Hello parse error: {}", e))?;

    match hello {
        McpCommand::Hello { client_name, protocol_version } => {
            if protocol_version != PROTOCOL_VERSION {
                let err_resp = McpResponse::Error {
                    code: ErrorCode::VersionMismatch,
                    message: format!(
                        "version mismatch: server={}, client={}",
                        PROTOCOL_VERSION, protocol_version
                    ),
                };
                let body = build_frame_body(&err_resp).map_err(|e| e.to_string())?;
                let _ = ws_sink.send(WsMessage::Binary(body.into())).await;
                return Err("protocol version mismatch".into());
            }

            tracing::info!("MCP client '{}' connected (v{})", client_name, protocol_version);

            // Send Welcome
            let welcome = McpResponse::Welcome {
                server_name: "Sassy Browser v2.0.0".into(),
                protocol_version: PROTOCOL_VERSION,
                capabilities: vec![
                    "navigation".into(),
                    "reading".into(),
                    "interaction".into(),
                    "screenshot".into(),
                    "detection".into(),
                    "honeypot".into(),
                    "tabs".into(),
                    "files".into(),
                ],
            };
            let body = build_frame_body(&welcome).map_err(|e| e.to_string())?;
            ws_sink.send(WsMessage::Binary(body.into())).await
                .map_err(|e| format!("Welcome send failed: {}", e))?;
        }
        _ => {
            return Err("expected Hello command as first message".into());
        }
    }

    // Main message loop — process commands and send responses
    let (_resp_tx, mut resp_rx) = mpsc::channel::<McpResponse>(64);

    // Track pending command sequences for this client
    let pending_seqs: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(Vec::new()));

    loop {
        tokio::select! {
            // Read incoming commands from client
            msg = ws_stream.next() => {
                match msg {
                    Some(Ok(WsMessage::Binary(data))) => {
                        match parse_frame_body::<McpCommand>(&data) {
                            Ok(cmd) => {
                                match cmd {
                                    McpCommand::Ping { seq } => {
                                        // Handle ping inline — no need to bridge
                                        let pong = McpResponse::Pong { seq };
                                        let body = build_frame_body(&pong).map_err(|e| e.to_string())?;
                                        ws_sink.send(WsMessage::Binary(body.into())).await
                                            .map_err(|e| format!("pong send failed: {}", e))?;
                                    }
                                    McpCommand::Goodbye => {
                                        let ok = McpResponse::Ok { message: "goodbye".into() };
                                        let body = build_frame_body(&ok).map_err(|e| e.to_string())?;
                                        let _ = ws_sink.send(WsMessage::Binary(body.into())).await;
                                        return Ok(());
                                    }
                                    _ => {
                                        // Push to bridge for app processing
                                        let seq = bridge.push_command(cmd);
                                        if let Ok(mut seqs) = pending_seqs.lock() {
                                            seqs.push(seq);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Malformed command from client: {}", e);
                                let err_resp = McpResponse::Error {
                                    code: ErrorCode::InvalidCommand,
                                    message: format!("parse error: {}", e),
                                };
                                let body = build_frame_body(&err_resp).map_err(|e| e.to_string())?;
                                ws_sink.send(WsMessage::Binary(body.into())).await
                                    .map_err(|e| format!("error send failed: {}", e))?;
                            }
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) | None => {
                        return Ok(());
                    }
                    Some(Err(e)) => {
                        return Err(format!("WebSocket error: {}", e));
                    }
                    _ => {} // Ignore text, pong, etc.
                }
            }

            // Check for responses from the app
            _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {
                // Poll bridge for responses to our pending commands
                let all_responses = bridge.take_responses();
                let my_seqs: Vec<u64> = if let Ok(seqs) = pending_seqs.lock() {
                    seqs.clone()
                } else {
                    Vec::new()
                };

                for (seq, resp) in all_responses {
                    if my_seqs.contains(&seq) {
                        // This response is for us
                        let body = build_frame_body(&resp).map_err(|e| e.to_string())?;
                        ws_sink.send(WsMessage::Binary(body.into())).await
                            .map_err(|e| format!("response send failed: {}", e))?;

                        // Remove from pending
                        if let Ok(mut seqs) = pending_seqs.lock() {
                            seqs.retain(|s| *s != seq);
                        }
                    } else {
                        // Not for us — put it back
                        bridge.push_response(seq, resp);
                    }
                }

                // Forward detection alerts as notifications
                if forward_alerts {
                    let alerts = bridge.take_detection_alerts();
                    for alert in alerts {
                        let payload = bincode::serialize(&alert).unwrap_or_default();
                        let notif = McpResponse::Notification {
                            event_type: NotificationType::DetectionAlert,
                            payload,
                        };
                        let body = build_frame_body(&notif).map_err(|e| e.to_string())?;
                        if ws_sink.send(WsMessage::Binary(body.into())).await.is_err() {
                            return Ok(());
                        }
                    }
                }
            }

            // Handle responses sent via channel (for inline responses)
            resp = resp_rx.recv() => {
                if let Some(resp) = resp {
                    let body = build_frame_body(&resp).map_err(|e| e.to_string())?;
                    ws_sink.send(WsMessage::Binary(body.into())).await
                        .map_err(|e| format!("inline response send failed: {}", e))?;
                }
            }
        }
    }
}
