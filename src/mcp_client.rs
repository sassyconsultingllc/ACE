//! MCP WebSocket Client — Connects to Sassy Browser's Native MCP Server
//!
//! Pure Rust, no JSON. Uses bincode over WebSocket via tokio-tungstenite.
#![allow(dead_code)]
//!
//! USAGE:
//! ─────────────────────────────────────────────────────────────────────────
//! ```ignore
//! let mut client = McpClient::connect("ws://127.0.0.1:9999").await?;
//!
//! // Send a command
//! client.send(McpCommand::Navigate {
//!     url: "https://example.com".into(),
//!     wait_for_load: true,
//! }).await?;
//!
//! // Receive the response
//! let resp = client.recv().await?;
//! ```
//!
//! The client handles:
//! - WebSocket connection with auto-reconnect
//! - Bincode serialization (no JSON!)
//! - Ping/pong keepalive
//! - Graceful shutdown

use crate::mcp_codec::{build_frame_body, parse_frame_body};
use crate::mcp_protocol::{McpCommand, McpResponse, PROTOCOL_VERSION, ProtocolError};
use futures::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as WsMessage;

/// MCP Client connection state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Shutdown,
}

/// MCP Client errors
#[derive(Debug)]
pub enum McpClientError {
    /// WebSocket connection failed
    ConnectionFailed(String),
    /// WebSocket transport error
    TransportError(String),
    /// Protocol error (bad magic, deserialization, etc.)
    ProtocolError(ProtocolError),
    /// Server rejected handshake
    HandshakeRejected(String),
    /// Connection closed by server
    ServerClosed,
    /// Operation timed out
    Timeout,
    /// Client has been shut down
    Shutdown,
    /// Channel error (internal)
    ChannelError(String),
}

impl std::fmt::Display for McpClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionFailed(e) => write!(f, "connection failed: {}", e),
            Self::TransportError(e) => write!(f, "transport error: {}", e),
            Self::ProtocolError(e) => write!(f, "protocol error: {}", e),
            Self::HandshakeRejected(e) => write!(f, "handshake rejected: {}", e),
            Self::ServerClosed => write!(f, "server closed connection"),
            Self::Timeout => write!(f, "operation timed out"),
            Self::Shutdown => write!(f, "client shut down"),
            Self::ChannelError(e) => write!(f, "channel error: {}", e),
        }
    }
}

impl std::error::Error for McpClientError {}

/// MCP WebSocket Client
///
/// Manages a persistent WebSocket connection to the Sassy Browser MCP server.
/// All communication uses bincode (no JSON).
pub struct McpClient {
    /// Channel to send commands to the writer task
    cmd_tx: mpsc::Sender<McpCommand>,
    /// Channel to receive responses from the reader task
    resp_rx: mpsc::Receiver<McpResponse>,
    /// Current connection state
    state: ConnectionState,
    /// Handle to the background connection task
    task_handle: Option<tokio::task::JoinHandle<()>>,
    /// Server URL
    url: String,
}

impl McpClient {
    /// Connect to a Sassy Browser MCP server.
    ///
    /// Performs the WebSocket handshake + MCP Hello/Welcome exchange.
    pub async fn connect(url: &str) -> Result<Self, McpClientError> {
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<McpCommand>(64);
        let (resp_tx, resp_rx) = mpsc::channel::<McpResponse>(64);

        let url_owned = url.to_string();

        // Connect WebSocket
        let (ws_stream, _ws_response) = tokio_tungstenite::connect_async(&url_owned)
            .await
            .map_err(|e| McpClientError::ConnectionFailed(e.to_string()))?;

        let (mut ws_sink, mut ws_stream_rx) = ws_stream.split();

        // Send Hello handshake
        let hello = McpCommand::Hello {
            client_name: "sassy-mcp-client".into(),
            protocol_version: PROTOCOL_VERSION,
        };
        let body = build_frame_body(&hello)
            .map_err(|e| McpClientError::ProtocolError(ProtocolError::DeserializeError(e.to_string())))?;
        ws_sink.send(WsMessage::Binary(body.into()))
            .await
            .map_err(|e| McpClientError::TransportError(e.to_string()))?;

        // Wait for Welcome response (with timeout)
        let welcome_msg = tokio::time::timeout(Duration::from_secs(10), ws_stream_rx.next())
            .await
            .map_err(|_| McpClientError::Timeout)?
            .ok_or(McpClientError::ServerClosed)?
            .map_err(|e| McpClientError::TransportError(e.to_string()))?;

        let welcome_bytes = match welcome_msg {
            WsMessage::Binary(b) => b,
            _ => return Err(McpClientError::HandshakeRejected("expected binary message".into())),
        };

        let welcome: McpResponse = parse_frame_body(&welcome_bytes)
            .map_err(McpClientError::ProtocolError)?;

        match &welcome {
            McpResponse::Welcome { protocol_version, .. } => {
                if *protocol_version != PROTOCOL_VERSION {
                    return Err(McpClientError::HandshakeRejected(
                        format!("version mismatch: server={}, client={}", protocol_version, PROTOCOL_VERSION),
                    ));
                }
            }
            McpResponse::Error { message, .. } => {
                return Err(McpClientError::HandshakeRejected(message.clone()));
            }
            _ => {
                return Err(McpClientError::HandshakeRejected("unexpected response type".into()));
            }
        }

        // Spawn background task for bidirectional message pumping
        let resp_tx_clone = resp_tx.clone();
        let task_handle = tokio::spawn(async move {
            // Writer loop — forwards commands from channel to WebSocket
            let writer_task = async {
                while let Some(cmd) = cmd_rx.recv().await {
                    // Check for graceful shutdown
                    let is_goodbye = matches!(cmd, McpCommand::Goodbye);

                    match build_frame_body(&cmd) {
                        Ok(body) => {
                            if ws_sink.send(WsMessage::Binary(body.into())).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => continue,
                    }

                    if is_goodbye {
                        let _ = ws_sink.close().await;
                        break;
                    }
                }
            };

            // Reader loop — forwards responses from WebSocket to channel
            let reader_task = async {
                while let Some(msg) = ws_stream_rx.next().await {
                    match msg {
                        Ok(WsMessage::Binary(data)) => {
                            match parse_frame_body::<McpResponse>(&data) {
                                Ok(resp) => {
                                    if resp_tx_clone.send(resp).await.is_err() {
                                        break; // Receiver dropped
                                    }
                                }
                                Err(_) => continue, // Skip malformed messages
                            }
                        }
                        Ok(WsMessage::Close(_)) => break,
                        Ok(WsMessage::Ping(data)) => {
                            // Pong is handled automatically by tungstenite
                            let _ = data;
                        }
                        Err(_) => break,
                        _ => {} // Ignore text, pong, etc.
                    }
                }
            };

            // Run both loops concurrently — when either finishes, we're done
            tokio::select! {
                _ = writer_task => {}
                _ = reader_task => {}
            }
        });

        Ok(Self {
            cmd_tx,
            resp_rx,
            state: ConnectionState::Connected,
            task_handle: Some(task_handle),
            url: url_owned,
        })
    }

    /// Send a command to the server.
    pub async fn send(&self, cmd: McpCommand) -> Result<(), McpClientError> {
        if self.state == ConnectionState::Shutdown {
            return Err(McpClientError::Shutdown);
        }
        self.cmd_tx.send(cmd).await
            .map_err(|e| McpClientError::ChannelError(e.to_string()))
    }

    /// Receive a response from the server.
    ///
    /// This blocks until a response is available or the connection is closed.
    pub async fn recv(&mut self) -> Result<McpResponse, McpClientError> {
        self.resp_rx.recv().await
            .ok_or(McpClientError::ServerClosed)
    }

    /// Receive a response with timeout.
    pub async fn recv_timeout(&mut self, timeout: Duration) -> Result<McpResponse, McpClientError> {
        tokio::time::timeout(timeout, self.resp_rx.recv())
            .await
            .map_err(|_| McpClientError::Timeout)?
            .ok_or(McpClientError::ServerClosed)
    }

    /// Send a command and wait for the response.
    pub async fn request(&mut self, cmd: McpCommand) -> Result<McpResponse, McpClientError> {
        self.send(cmd).await?;
        self.recv().await
    }

    /// Send a command and wait for response with timeout.
    pub async fn request_timeout(
        &mut self,
        cmd: McpCommand,
        timeout: Duration,
    ) -> Result<McpResponse, McpClientError> {
        self.send(cmd).await?;
        self.recv_timeout(timeout).await
    }

    /// Get the current connection state.
    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// Get the server URL.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Check if connected.
    pub fn is_connected(&self) -> bool {
        self.state == ConnectionState::Connected
    }

    /// Gracefully disconnect from the server.
    pub async fn shutdown(&mut self) -> Result<(), McpClientError> {
        if self.state == ConnectionState::Shutdown {
            return Ok(());
        }

        self.state = ConnectionState::Shutdown;

        // Send Goodbye
        let _ = self.cmd_tx.send(McpCommand::Goodbye).await;

        // Wait for the background task to finish (with timeout)
        if let Some(handle) = self.task_handle.take() {
            let _ = tokio::time::timeout(Duration::from_secs(5), handle).await;
        }

        Ok(())
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        // If the client is dropped without shutdown, abort the background task
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// CONVENIENCE FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

/// Connect to a local Sassy Browser MCP server on the default port.
pub async fn connect_local() -> Result<McpClient, McpClientError> {
    McpClient::connect("ws://127.0.0.1:9999").await
}

/// Connect to a local Sassy Browser MCP server on a specific port.
pub async fn connect_local_port(port: u16) -> Result<McpClient, McpClientError> {
    McpClient::connect(&format!("ws://127.0.0.1:{}", port)).await
}
