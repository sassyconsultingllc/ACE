//! MCP CLI Client — Quick test client for Sassy Browser's Native MCP Server
//!
//! Usage: cargo run --bin mcp-client
//!
//! Connects to ws://127.0.0.1:9998, sends Hello + GetBrowserInfo, prints responses.

use futures::{SinkExt, StreamExt};
use sassy_browser::mcp_codec::{build_frame_body, parse_frame_body};
use sassy_browser::mcp_protocol::{McpCommand, McpResponse, PROTOCOL_VERSION};
use tokio_tungstenite::tungstenite::Message as WsMessage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔌 Connecting to Sassy Browser MCP server at ws://127.0.0.1:9998...");

    let (ws_stream, _) = tokio_tungstenite::connect_async("ws://127.0.0.1:9998").await?;
    let (mut write, mut read) = ws_stream.split();

    println!("✅ Connected! Sending Hello...");

    // Send Hello handshake
    let hello = McpCommand::Hello {
        client_name: "mcp-cli-test".into(),
        protocol_version: PROTOCOL_VERSION,
    };
    let body = build_frame_body(&hello)?;
    write.send(WsMessage::Binary(body)).await?;

    // Read Hello response
    if let Some(Ok(msg)) = read.next().await {
        if let WsMessage::Binary(data) = msg {
            let resp: McpResponse = parse_frame_body(&data)?;
            println!("📨 Hello response: {:?}", resp);
        }
    }

    // Send GetBrowserInfo
    println!("📤 Sending GetBrowserInfo...");
    let cmd = McpCommand::GetBrowserInfo;
    let body = build_frame_body(&cmd)?;
    write.send(WsMessage::Binary(body)).await?;

    if let Some(Ok(msg)) = read.next().await {
        if let WsMessage::Binary(data) = msg {
            let resp: McpResponse = parse_frame_body(&data)?;
            println!("📨 BrowserInfo: {:?}", resp);
        }
    }

    // Send ReadPage
    println!("📤 Sending ReadPage...");
    let cmd = McpCommand::ReadPage;
    let body = build_frame_body(&cmd)?;
    write.send(WsMessage::Binary(body)).await?;

    if let Some(Ok(msg)) = read.next().await {
        if let WsMessage::Binary(data) = msg {
            let resp: McpResponse = parse_frame_body(&data)?;
            println!("📨 PageContent: {:?}", resp);
        }
    }

    // Send ListTabs
    println!("📤 Sending ListTabs...");
    let cmd = McpCommand::ListTabs;
    let body = build_frame_body(&cmd)?;
    write.send(WsMessage::Binary(body)).await?;

    if let Some(Ok(msg)) = read.next().await {
        if let WsMessage::Binary(data) = msg {
            let resp: McpResponse = parse_frame_body(&data)?;
            println!("📨 Tabs: {:?}", resp);
        }
    }

    println!("✅ All commands sent and responses received. Done.");
    Ok(())
}
