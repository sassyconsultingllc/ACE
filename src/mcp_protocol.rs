//! Native MCP Protocol — Pure Rust, Bincode over WebSocket
//!
//! NO JSON. NO serde_json. Pure binary protocol using bincode.
//!
//! ARCHITECTURE:
//! ─────────────────────────────────────────────────────────────────────────
//! McpCommand  — Client → Server commands (navigate, click, read, etc.)
//! McpResponse — Server → Client responses (content, screenshots, errors)
//!
//! Messages are serialized with bincode (compact, fast, no parsing overhead)
//! and framed with a 4-byte big-endian length prefix for WebSocket transport.
//!
//! WHY NOT JSON:
//! - bincode is 5-10x faster than serde_json for ser/de
//! - No string escaping overhead for binary data (screenshots)
//! - Type-safe: deserialization catches mismatches at compile time
//! - Smaller wire size (no field names repeated per message)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Protocol version for compatibility checks
pub const PROTOCOL_VERSION: u32 = 1;

/// Magic bytes to identify Sassy MCP binary frames
pub const MAGIC: [u8; 4] = [0x53, 0x41, 0x53, 0x59]; // "SASY"

// ═══════════════════════════════════════════════════════════════════════════════
// COMMANDS — Client → Server
// ═══════════════════════════════════════════════════════════════════════════════

/// Commands sent from MCP client to Sassy Browser server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum McpCommand {
    /// Handshake — first message after connection
    Hello {
        client_name: String,
        protocol_version: u32,
    },

    /// Navigate to a URL
    Navigate { url: String, wait_for_load: bool },

    /// Go back in history
    GoBack,

    /// Go forward in history
    GoForward,

    /// Reload the current page
    Reload,

    /// Read the current page content (DOM text, title, URL)
    ReadPage,

    /// Take a screenshot (returns PNG bytes)
    Screenshot { full_page: bool },

    /// Click at coordinates
    Click { x: f32, y: f32 },

    /// Type text into the focused element
    TypeText {
        text: String,
        element_ref: Option<String>,
    },

    /// Execute SassyScript (our JS engine) on the page
    ExecuteScript { code: String },

    /// Get the current page's security/trust status
    GetSecurityStatus,

    /// Get detection engine alerts
    GetDetectionAlerts,

    /// Clear detection alerts
    ClearDetectionAlerts,

    /// Get honeypot status for active tab
    GetHoneypotStatus,

    /// Open a local file in the viewer
    OpenFile { path: String },

    /// Get list of open tabs
    ListTabs,

    /// Switch to a specific tab
    SwitchTab { tab_index: usize },

    /// Create a new tab
    NewTab { url: Option<String> },

    /// Close a tab
    CloseTab { tab_index: usize },

    /// Get browser info (version, features, etc.)
    GetBrowserInfo,

    /// Scroll the page
    Scroll { delta_x: f32, delta_y: f32 },

    /// Find text on the page
    FindText { query: String, case_sensitive: bool },

    /// Get/set a cookie
    Cookie {
        action: CookieAction,
        domain: String,
        name: String,
        value: Option<String>,
    },

    /// Query the password vault (requires trust)
    VaultQuery { domain: String },

    /// Custom extension command
    Extension { name: String, payload: Vec<u8> },

    /// Ping — keepalive
    Ping { seq: u64 },

    /// Graceful disconnect
    Goodbye,
}

/// Cookie action for the Cookie command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CookieAction {
    Get,
    Set,
    Delete,
    List,
}

// ═══════════════════════════════════════════════════════════════════════════════
// RESPONSES — Server → Client
// ═══════════════════════════════════════════════════════════════════════════════

/// Responses sent from Sassy Browser server to MCP client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum McpResponse {
    /// Handshake response
    Welcome {
        server_name: String,
        protocol_version: u32,
        capabilities: Vec<String>,
    },

    /// Navigation result
    NavigationComplete {
        url: String,
        title: String,
        status_code: u16,
        load_time_ms: u64,
    },

    /// Page content (from ReadPage)
    PageContent {
        url: String,
        title: String,
        text_content: String,
        html_snippet: Option<String>,
        trust_level: u8,
    },

    /// Screenshot data (PNG bytes)
    ScreenshotData {
        png_bytes: Vec<u8>,
        width: u32,
        height: u32,
    },

    /// Result of a click action
    ClickResult {
        success: bool,
        element_info: Option<String>,
    },

    /// Result of text typing
    TypeResult { success: bool },

    /// Script execution result
    ScriptResult {
        output: String,
        error: Option<String>,
    },

    /// Security/trust status
    SecurityStatus {
        url: String,
        trust_level: u8,
        trust_description: String,
        violation_count: usize,
        honeypots_active: bool,
        detection_alerts: usize,
        cumulative_score: f32,
    },

    /// Detection alerts from honeypot/behavior engine
    DetectionAlerts {
        alerts: Vec<DetectionAlertWire>,
        total_emitted: u64,
        cumulative_score: f32,
    },

    /// Honeypot status for current tab
    HoneypotStatus {
        active: bool,
        traps: Vec<String>,
        triggers: HashMap<String, bool>,
    },

    /// List of open tabs
    TabList {
        tabs: Vec<TabInfo>,
        active_index: usize,
    },

    /// Browser info
    BrowserInfo {
        name: String,
        version: String,
        engine: String,
        features: Vec<String>,
    },

    /// Find results
    FindResults { query: String, match_count: usize },

    /// Cookie result
    CookieResult { cookies: Vec<CookieWire> },

    /// Vault credentials (sanitized — never sends passwords over wire)
    VaultResult {
        domain: String,
        has_credentials: bool,
        username: Option<String>,
        // NOTE: Password is NEVER sent. User must autofill locally.
    },

    /// Generic success
    Ok { message: String },

    /// Error response
    Error { code: ErrorCode, message: String },

    /// Pong — keepalive response
    Pong { seq: u64 },

    /// Server-initiated notification (detection alert, page event, etc.)
    Notification {
        event_type: NotificationType,
        payload: Vec<u8>,
    },
}

/// Error codes for MCP responses
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ErrorCode {
    /// Unknown/generic error
    Unknown = 0,
    /// Invalid command
    InvalidCommand = 1,
    /// Permission denied (trust level too low)
    PermissionDenied = 2,
    /// Resource not found
    NotFound = 3,
    /// Operation timed out
    Timeout = 4,
    /// Protocol version mismatch
    VersionMismatch = 5,
    /// Internal browser error
    InternalError = 6,
    /// Rate limited
    RateLimited = 7,
    /// Feature not available
    NotAvailable = 8,
}

/// Notification types for server-push events
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum NotificationType {
    /// A detection alert was triggered
    DetectionAlert,
    /// Page navigation occurred
    PageNavigated,
    /// Tab opened/closed
    TabChanged,
    /// Trust level changed
    TrustChanged,
    /// Download started
    DownloadStarted,
    /// Honeypot was triggered
    HoneypotTriggered,
}

// ═══════════════════════════════════════════════════════════════════════════════
// WIRE TYPES — Serializable versions of internal types
// ═══════════════════════════════════════════════════════════════════════════════

/// Detection alert for wire transport (no Instant — uses epoch millis)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionAlertWire {
    pub rule_name: String,
    pub level: u8, // SuspicionLevel as u8
    pub description: String,
    pub url: String,
    pub domain: String,
    pub score: f32,
    pub honeypot_triggered: bool,
    pub action: u8, // DetectionAction as u8
}

/// Tab info for wire transport
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabInfo {
    pub index: usize,
    pub title: String,
    pub url: String,
    pub loading: bool,
    pub trust_level: u8,
}

/// Cookie for wire transport
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieWire {
    pub domain: String,
    pub name: String,
    pub value: String,
    pub secure: bool,
    pub http_only: bool,
}

// ═══════════════════════════════════════════════════════════════════════════════
// FRAME ENCODING — Length-prefixed bincode
// ═══════════════════════════════════════════════════════════════════════════════

/// Encode a command into a length-prefixed bincode frame
pub fn encode_command(cmd: &McpCommand) -> Result<Vec<u8>, bincode::Error> {
    let payload = bincode::serialize(cmd)?;
    let mut frame = Vec::with_capacity(4 + MAGIC.len() + payload.len());
    // 4 bytes: total frame length (excluding this length field itself)
    let frame_len = (MAGIC.len() + payload.len()) as u32;
    frame.extend_from_slice(&frame_len.to_be_bytes());
    frame.extend_from_slice(&MAGIC);
    frame.extend_from_slice(&payload);
    Ok(frame)
}

/// Encode a response into a length-prefixed bincode frame
pub fn encode_response(resp: &McpResponse) -> Result<Vec<u8>, bincode::Error> {
    let payload = bincode::serialize(resp)?;
    let mut frame = Vec::with_capacity(4 + MAGIC.len() + payload.len());
    let frame_len = (MAGIC.len() + payload.len()) as u32;
    frame.extend_from_slice(&frame_len.to_be_bytes());
    frame.extend_from_slice(&MAGIC);
    frame.extend_from_slice(&payload);
    Ok(frame)
}

/// Decode a command from a raw frame (after length prefix is stripped)
pub fn decode_command(data: &[u8]) -> Result<McpCommand, ProtocolError> {
    if data.len() < MAGIC.len() {
        return Err(ProtocolError::TooShort);
    }
    if &data[..MAGIC.len()] != &MAGIC {
        return Err(ProtocolError::BadMagic);
    }
    bincode::deserialize(&data[MAGIC.len()..])
        .map_err(|e| ProtocolError::DeserializeError(e.to_string()))
}

/// Decode a response from a raw frame (after length prefix is stripped)
pub fn decode_response(data: &[u8]) -> Result<McpResponse, ProtocolError> {
    if data.len() < MAGIC.len() {
        return Err(ProtocolError::TooShort);
    }
    if &data[..MAGIC.len()] != &MAGIC {
        return Err(ProtocolError::BadMagic);
    }
    bincode::deserialize(&data[MAGIC.len()..])
        .map_err(|e| ProtocolError::DeserializeError(e.to_string()))
}

/// Protocol-level errors
#[derive(Debug, Clone)]
pub enum ProtocolError {
    /// Frame is too short to contain magic bytes
    TooShort,
    /// Magic bytes don't match "SASY"
    BadMagic,
    /// Bincode deserialization failed
    DeserializeError(String),
    /// Frame length exceeds maximum
    FrameTooLarge(usize),
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort => write!(f, "frame too short"),
            Self::BadMagic => write!(f, "invalid magic bytes (expected SASY)"),
            Self::DeserializeError(e) => write!(f, "deserialization error: {}", e),
            Self::FrameTooLarge(sz) => write!(f, "frame too large: {} bytes", sz),
        }
    }
}

impl std::error::Error for ProtocolError {}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_roundtrip() {
        let cmd = McpCommand::Navigate {
            url: "https://example.com".into(),
            wait_for_load: true,
        };
        let frame = encode_command(&cmd).unwrap();
        // Skip the 4-byte length prefix
        let decoded = decode_command(&frame[4..]).unwrap();
        match decoded {
            McpCommand::Navigate { url, wait_for_load } => {
                assert_eq!(url, "https://example.com");
                assert!(wait_for_load);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_response_roundtrip() {
        let resp = McpResponse::SecurityStatus {
            url: "https://phish.com".into(),
            trust_level: 0,
            trust_description: "Untrusted".into(),
            violation_count: 3,
            honeypots_active: true,
            detection_alerts: 2,
            cumulative_score: 4.5,
        };
        let frame = encode_response(&resp).unwrap();
        let decoded = decode_response(&frame[4..]).unwrap();
        match decoded {
            McpResponse::SecurityStatus {
                trust_level,
                honeypots_active,
                ..
            } => {
                assert_eq!(trust_level, 0);
                assert!(honeypots_active);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_bad_magic_rejected() {
        let bad_frame = vec![0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x01];
        assert!(matches!(
            decode_command(&bad_frame),
            Err(ProtocolError::BadMagic)
        ));
    }

    #[test]
    fn test_ping_pong() {
        let cmd = McpCommand::Ping { seq: 42 };
        let frame = encode_command(&cmd).unwrap();
        let decoded = decode_command(&frame[4..]).unwrap();
        match decoded {
            McpCommand::Ping { seq } => assert_eq!(seq, 42),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_screenshot_binary_data() {
        let fake_png = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        let resp = McpResponse::ScreenshotData {
            png_bytes: fake_png.clone(),
            width: 1920,
            height: 1080,
        };
        let frame = encode_response(&resp).unwrap();
        let decoded = decode_response(&frame[4..]).unwrap();
        match decoded {
            McpResponse::ScreenshotData {
                png_bytes,
                width,
                height,
            } => {
                assert_eq!(png_bytes, fake_png);
                assert_eq!(width, 1920);
                assert_eq!(height, 1080);
            }
            _ => panic!("wrong variant"),
        }
    }
}
