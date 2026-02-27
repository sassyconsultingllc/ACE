//! MCP CLI Client — connects to Sassy Browser's Native MCP Server
//!
//! Standalone binary with minimal protocol types (no lib dependency).
//! Connects via WebSocket, sends bincode-framed commands, prints responses.
//!
//! Usage: cargo run --bin mcp-client

use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::Message as WsMessage;

// ── Minimal protocol types (mirror of mcp_protocol.rs) ──────────────────

const PROTOCOL_VERSION: u32 = 1;
const MAGIC: [u8; 4] = [0x53, 0x41, 0x53, 0x59]; // "SASY"

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum McpCommand {
    Hello { client_name: String, protocol_version: u32 },
    Navigate { url: String, wait_for_load: bool },
    GoBack,
    GoForward,
    Reload,
    ReadPage,
    Screenshot { full_page: bool },
    Click { x: f32, y: f32 },
    TypeText { text: String, element_ref: Option<String> },
    ExecuteScript { code: String },
    GetSecurityStatus,
    GetDetectionAlerts,
    ClearDetectionAlerts,
    GetHoneypotStatus,
    OpenFile { path: String },
    ListTabs,
    SwitchTab { tab_index: usize },
    NewTab { url: Option<String> },
    CloseTab { tab_index: usize },
    GetBrowserInfo,
    Scroll { delta_x: f32, delta_y: f32 },
    FindText { query: String, case_sensitive: bool },
    Ping { seq: u64 },
    Goodbye,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorCode {
    InvalidCommand,
    NavigationFailed,
    TabNotFound,
    SecurityViolation,
    InternalError,
    VersionMismatch,
    NotImplemented,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    DetectionAlert,
    PageLoaded,
    TabChanged,
    SecurityEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum McpResponse {
    Welcome { server_name: String, protocol_version: u32, capabilities: Vec<String> },
    PageContent { url: String, title: String, text_content: String, word_count: usize },
    ScreenshotData { png_bytes: Vec<u8>, width: u32, height: u32 },
    SecurityStatus { url: String, trust_level: String, threats: Vec<String>, is_secure: bool },
    DetectionAlerts { alerts: Vec<String> },
    HoneypotStatus { active: bool, url: String, alerts: Vec<String> },
    TabList { tabs: Vec<TabInfo> },
    BrowserInfo { version: String, engine: String, features: Vec<String> },
    NavigationOk { url: String },
    Ok { message: String },
    Error { code: ErrorCode, message: String },
    Pong { seq: u64 },
    Notification { notification_type: NotificationType, data: String },
    FindResults { query: String, count: usize },
    FileInfo { path: String, size: u64, file_type: String },
    ScrollOk { x: f32, y: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabInfo {
    pub index: usize,
    pub title: String,
    pub url: String,
    pub is_active: bool,
}

fn build_frame(cmd: &McpCommand) -> Vec<u8> {
    let payload = bincode::serialize(cmd).expect("serialize failed");
    let mut body = Vec::with_capacity(MAGIC.len() + payload.len());
    body.extend_from_slice(&MAGIC);
    body.extend_from_slice(&payload);
    body
}

fn parse_response(data: &[u8]) -> Result<McpResponse, String> {
    if data.len() < MAGIC.len() {
        return Err("frame too short".into());
    }
    if &data[..MAGIC.len()] != &MAGIC {
        return Err("bad magic bytes".into());
    }
    bincode::deserialize(&data[MAGIC.len()..])
        .map_err(|e| format!("deserialize error: {}", e))
}

async fn send_and_recv(
    write: &mut futures::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        WsMessage,
    >,
    read: &mut futures::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    >,
    cmd: McpCommand,
) -> Result<McpResponse, Box<dyn std::error::Error>> {
    write.send(WsMessage::Binary(build_frame(&cmd).into())).await?;
    match tokio::time::timeout(std::time::Duration::from_secs(10), read.next()).await {
        Ok(Some(Ok(WsMessage::Binary(data)))) => Ok(parse_response(&data)?),
        Ok(Some(Ok(_))) => Err("non-binary message".into()),
        Ok(Some(Err(e))) => Err(e.into()),
        Ok(None) => Err("connection closed".into()),
        Err(_) => Err("timeout".into()),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "ws://127.0.0.1:9998";
    println!("Connecting to Sassy Browser MCP server at {}...", url);

    let (ws_stream, _) = tokio_tungstenite::connect_async(url).await?;
    let (mut write, mut read) = ws_stream.split();

    println!("Connected! Sending Hello...\n");

    // Hello handshake
    let hello = McpCommand::Hello {
        client_name: "mcp-cli-client".into(),
        protocol_version: PROTOCOL_VERSION,
    };
    let resp = send_and_recv(&mut write, &mut read, hello).await?;
    println!("Server: {:?}\n", resp);

    // GetBrowserInfo
    println!(">>> GetBrowserInfo");
    let resp = send_and_recv(&mut write, &mut read, McpCommand::GetBrowserInfo).await?;
    println!("<<< {:?}\n", resp);

    // ListTabs
    println!(">>> ListTabs");
    let resp = send_and_recv(&mut write, &mut read, McpCommand::ListTabs).await?;
    println!("<<< {:?}\n", resp);

    // ReadPage
    println!(">>> ReadPage");
    let resp = send_and_recv(&mut write, &mut read, McpCommand::ReadPage).await?;
    println!("<<< {:?}\n", resp);

    // Ping
    println!(">>> Ping(42)");
    let resp = send_and_recv(&mut write, &mut read, McpCommand::Ping { seq: 42 }).await?;
    println!("<<< {:?}\n", resp);

    // Interactive loop using blocking stdin in a spawn_blocking
    println!("=== Interactive mode ===");
    println!("Commands: navigate <url> | tabs | read | info | security | ping | quit\n");

    loop {
        print!("> ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let line = tokio::task::spawn_blocking(|| {
            let mut buf = String::new();
            std::io::BufRead::read_line(&mut std::io::stdin().lock(), &mut buf).ok();
            buf
        }).await?;

        let input = line.trim();
        if input.is_empty() { continue; }

        let cmd = match input {
            "tabs" => McpCommand::ListTabs,
            "read" => McpCommand::ReadPage,
            "info" => McpCommand::GetBrowserInfo,
            "security" => McpCommand::GetSecurityStatus,
            "ping" => McpCommand::Ping { seq: 99 },
            "quit" | "exit" => {
                let _ = send_and_recv(&mut write, &mut read, McpCommand::Goodbye).await;
                println!("Goodbye!");
                break;
            }
            s if s.starts_with("navigate ") => {
                let url = s.strip_prefix("navigate ").unwrap().to_string();
                McpCommand::Navigate { url, wait_for_load: true }
            }
            s if s.starts_with("find ") => {
                let query = s.strip_prefix("find ").unwrap().to_string();
                McpCommand::FindText { query, case_sensitive: false }
            }
            _ => {
                println!("Unknown: {}", input);
                continue;
            }
        };

        match send_and_recv(&mut write, &mut read, cmd).await {
            Ok(resp) => println!("{:?}\n", resp),
            Err(e) => println!("Error: {}\n", e),
        }
    }

    Ok(())
}
