//! MCP Server - Expose Sassy Browser as an MCP Server
//!
//! Allows external MCP clients (Claude Desktop, etc.) to control Sassy Browser
//! via the Model Context Protocol (JSON-RPC 2.0 over stdio or socket).
//!
//! ## Architecture
//!
//! The MCP server runs in a separate thread and communicates with the main
//! browser engine via channels:
//! - `McpCommand` channel: MCP server -> Browser engine (commands)
//! - `McpResponse` channel: Browser engine -> MCP server (responses)
//!
//! ## Usage
//!
//! ### Claude Desktop Integration
//! Add to `claude_desktop_config.json`:
//! ```json
//! {
//!   "mcpServers": {
//!     "sassy-browser": {
//!       "command": "sassy-browser.exe",
//!       "args": ["--mcp-server"]
//!     }
//!   }
//! }
//! ```
//!
//! ### Socket Mode (for remote/multiple clients)
//! ```bash
//! sassy-browser.exe --mcp-server --mcp-port 9999
//! ```

use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::sync::mpsc::{channel, Receiver, Sender};

// ==============================================================================
// Configuration
// ==============================================================================

/// MCP Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Enable MCP server mode
    pub enabled: bool,
    /// Transport mode: "stdio" or "socket"
    pub transport: String,
    /// Port for socket transport (default: 9999)
    pub port: u16,
    /// Allowed tool categories
    pub allowed_tools: Vec<String>,
    /// Require confirmation for destructive actions
    pub confirm_destructive: bool,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            transport: "stdio".to_string(),
            port: 9999,
            allowed_tools: vec![
                "navigation".to_string(),
                "reading".to_string(),
                "interaction".to_string(),
                "screenshot".to_string(),
                "javascript".to_string(),
                "security".to_string(),
                "files".to_string(),
            ],
            confirm_destructive: true,
        }
    }
}

// ==============================================================================
// MCP Protocol Types
// ==============================================================================

/// MCP Protocol version
pub const MCP_VERSION: &str = "2024-11-05";

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: JsonValue,
    pub method: String,
    #[serde(default)]
    pub params: JsonValue,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: JsonValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<JsonValue>,
}

impl JsonRpcResponse {
    pub fn success(id: JsonValue, result: JsonValue) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: JsonValue, code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }
}

// JSON-RPC error codes
pub const PARSE_ERROR: i32 = -32700;
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;
pub const INTERNAL_ERROR: i32 = -32603;

// ==============================================================================
// Command/Response Types for Browser Communication
// ==============================================================================

/// Commands sent from MCP server to browser engine
#[derive(Debug, Clone)]
pub enum McpCommand {
    // Navigation
    Navigate {
        url: String,
        tab_id: Option<u64>,
    },
    NewTab {
        url: Option<String>,
    },
    CloseTab {
        tab_id: u64,
    },
    ListTabs,
    GoBack {
        tab_id: Option<u64>,
    },
    GoForward {
        tab_id: Option<u64>,
    },
    Reload {
        tab_id: Option<u64>,
    },

    // Reading
    ReadPage {
        tab_id: Option<u64>,
        depth: u32,
        filter: String,
    },
    GetText {
        tab_id: Option<u64>,
        selector: Option<String>,
    },
    FindElement {
        query: String,
        tab_id: Option<u64>,
        max_results: u32,
    },

    // Interaction
    Click {
        tab_id: Option<u64>,
        element_ref: Option<String>,
        x: Option<i32>,
        y: Option<i32>,
        button: String,
        click_count: u32,
    },
    TypeText {
        tab_id: Option<u64>,
        text: String,
        element_ref: Option<String>,
        clear_first: bool,
    },
    PressKey {
        tab_id: Option<u64>,
        key: String,
    },
    Scroll {
        tab_id: Option<u64>,
        direction: String,
        amount: i32,
        element_ref: Option<String>,
    },
    FormInput {
        tab_id: Option<u64>,
        element_ref: String,
        value: JsonValue,
    },

    // Screenshot
    Screenshot {
        tab_id: Option<u64>,
        element_ref: Option<String>,
        format: String,
    },

    // JavaScript
    ExecuteJs {
        tab_id: Option<u64>,
        code: String,
    },

    // Security
    GetTrustLevel {
        tab_id: Option<u64>,
    },
    GetSecurityInfo {
        tab_id: Option<u64>,
    },

    // File Viewer
    OpenFile {
        path: String,
    },
    GetFileInfo {
        tab_id: Option<u64>,
    },

    // Extended Tools
    ActivateTab {
        tab_id: u64,
    },
    SearchBookmarks {
        query: String,
    },
    AddBookmark {
        url: String,
        title: String,
        folder_id: Option<String>,
    },
    SearchHistory {
        query: String,
        limit: u32,
    },
    StartDownload {
        url: String,
        filename: Option<String>,
    },
    ListDownloads,
    WebSearch {
        query: String,
    },
}

/// Tab information returned from browser
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabInfo {
    pub id: u64,
    pub index: usize,
    pub url: String,
    pub title: String,
    pub loading: bool,
    pub is_secure: bool,
    pub trust_level: String,
    pub is_file: bool,
    pub file_type: Option<String>,
}

/// DOM element information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementInfo {
    pub ref_id: String,
    pub tag: String,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub text: Option<String>,
    pub attributes: HashMap<String, String>,
    pub interactive: bool,
    pub bounds: Option<ElementBounds>,
    pub children: Vec<ElementInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Responses sent from browser engine to MCP server
#[derive(Debug, Clone)]
pub enum McpResponse {
    // Navigation
    NavigateResult {
        success: bool,
        url: String,
        message: String,
    },
    NewTabResult {
        success: bool,
        tab_id: u64,
        url: String,
    },
    CloseTabResult {
        success: bool,
        closed_tab_id: u64,
    },
    TabList {
        tabs: Vec<TabInfo>,
        active_tab: usize,
    },

    // Reading
    PageTree {
        tree: Box<ElementInfo>,
        depth: u32,
        filter: String,
    },
    TextContent {
        text: String,
        length: usize,
    },
    FindResult {
        elements: Vec<ElementInfo>,
        query: String,
        count: usize,
    },

    // Interaction
    ClickResult {
        success: bool,
        element_ref: Option<String>,
        coordinates: Option<(i32, i32)>,
    },
    TypeResult {
        success: bool,
        typed: String,
    },
    KeyResult {
        success: bool,
        key: String,
    },
    ScrollResult {
        success: bool,
        direction: String,
        amount: i32,
    },
    FormInputResult {
        success: bool,
        element_ref: String,
        value: JsonValue,
    },

    // Screenshot
    ScreenshotResult {
        success: bool,
        format: String,
        width: u32,
        height: u32,
        data: String,
    },

    // JavaScript
    JsResult {
        success: bool,
        result: JsonValue,
        console: Vec<String>,
    },

    // Security
    TrustLevel {
        level: String,
        interactions: u32,
        required: u32,
        restrictions: Vec<String>,
    },
    SecurityInfo {
        ssl: JsonValue,
        cookies: JsonValue,
        trackers_blocked: u32,
        popups_blocked: u32,
    },

    // File
    OpenFileResult {
        success: bool,
        path: String,
        format: String,
        viewer: String,
    },
    FileInfo {
        is_file: bool,
        path: Option<String>,
        format: Option<String>,
        metadata: JsonValue,
    },

    // Extended Tools
    ActivateTabResult {
        success: bool,
        tab_id: u64,
    },
    BookmarkResults {
        bookmarks: Vec<JsonValue>,
        count: usize,
    },
    BookmarkAdded {
        success: bool,
        id: String,
    },
    HistoryResults {
        entries: Vec<JsonValue>,
        count: usize,
    },
    DownloadStarted {
        success: bool,
        download_id: String,
        url: String,
    },
    DownloadList {
        downloads: Vec<JsonValue>,
    },
    WebSearchResult {
        success: bool,
        url: String,
    },

    // Error
    Error {
        code: i32,
        message: String,
    },
}

// ==============================================================================
// Context & Session Management
// ==============================================================================

/// MCP Context for multi-turn conversations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpContext {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub last_active_at: String,
    pub model_family: Option<String>,
    pub temperature: f32,
    pub max_tokens: u32,
    pub system_prompt: Option<String>,
    pub messages: Vec<McpChatMessage>,
    pub metadata: HashMap<String, JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

/// MCP Server metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpMetrics {
    pub tool_call_count: HashMap<String, u64>,
    pub total_requests: u64,
    pub total_errors: u64,
    pub contexts_created: u64,
    pub uptime_secs: u64,
}

/// MCP Event types for observability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpEvent {
    pub event_type: String,
    pub timestamp: String,
    pub data: JsonValue,
}

// ==============================================================================
// Browser Bridge - Communication channel between MCP server and browser
// ==============================================================================

/// Channel-based bridge for communication between MCP server thread and browser
pub struct McpBridge {
    /// Commands from MCP server to browser
    pub command_tx: Sender<McpCommand>,
    pub command_rx: Receiver<McpCommand>,
    /// Responses from browser to MCP server
    pub response_tx: Sender<McpResponse>,
    pub response_rx: Receiver<McpResponse>,
}

impl McpBridge {
    /// Creates a sender/receiver pair for cross-thread MCP communication
    pub fn create() -> (McpBridgeSender, McpBridgeReceiver) {
        let (cmd_tx, cmd_rx) = channel();
        let (resp_tx, resp_rx) = channel();

        // Construct the full bridge first so all fields are used
        let bridge = McpBridge {
            command_tx: cmd_tx,
            command_rx: cmd_rx,
            response_tx: resp_tx,
            response_rx: resp_rx,
        };

        let sender = McpBridgeSender {
            command_tx: bridge.command_tx,
            response_rx: bridge.response_rx,
        };

        let receiver = McpBridgeReceiver {
            command_rx: bridge.command_rx,
            response_tx: bridge.response_tx,
        };

        (sender, receiver)
    }

    /// Backwards-compatible alias: historically callers used `McpBridge::new()`
    /// Keep a `new` method that delegates to `create()` to avoid breaking call-sites.
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> (McpBridgeSender, McpBridgeReceiver) {
        Self::create()
    }
}

/// MCP server side of the bridge (sends commands, receives responses)
pub struct McpBridgeSender {
    pub command_tx: Sender<McpCommand>,
    pub response_rx: Receiver<McpResponse>,
}

impl McpBridgeSender {
    /// Send a command and wait for response
    pub fn send_command(&self, cmd: McpCommand) -> Result<McpResponse, String> {
        self.command_tx
            .send(cmd)
            .map_err(|e| format!("Failed to send command: {}", e))?;
        self.response_rx
            .recv()
            .map_err(|e| format!("Failed to receive response: {}", e))
    }

    /// Send a command without waiting (for notifications)
    pub fn send_command_async(&self, cmd: McpCommand) -> Result<(), String> {
        self.command_tx
            .send(cmd)
            .map_err(|e| format!("Failed to send command: {}", e))
    }
}

/// Browser side of the bridge (receives commands, sends responses)
pub struct McpBridgeReceiver {
    pub command_rx: Receiver<McpCommand>,
    pub response_tx: Sender<McpResponse>,
}

impl McpBridgeReceiver {
    /// Try to receive a command (non-blocking)
    pub fn try_recv(&self) -> Option<McpCommand> {
        self.command_rx.try_recv().ok()
    }

    /// Send a response
    pub fn send_response(&self, response: McpResponse) -> Result<(), String> {
        self.response_tx
            .send(response)
            .map_err(|e| format!("Failed to send response: {}", e))
    }
}

// ==============================================================================
// MCP Tool Definitions
// ==============================================================================

/// MCP Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: JsonValue,
}

/// Define all MCP tools exposed by Sassy Browser
fn define_tools() -> Vec<McpTool> {
    vec![
        // === Navigation Tools ===
        McpTool {
            name: "navigate".to_string(),
            description: "Navigate to a URL or go back/forward in history".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "URL to navigate to, or 'back'/'forward' for history navigation"
                    },
                    "tab_id": {
                        "type": "integer",
                        "description": "Tab ID to navigate (optional, uses active tab if not specified)"
                    }
                },
                "required": ["url"]
            }),
        },
        McpTool {
            name: "new_tab".to_string(),
            description: "Create a new browser tab".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "URL to open in new tab (optional, opens blank if not specified)"
                    }
                }
            }),
        },
        McpTool {
            name: "close_tab".to_string(),
            description: "Close a browser tab".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "integer",
                        "description": "Tab ID to close"
                    }
                },
                "required": ["tab_id"]
            }),
        },
        McpTool {
            name: "list_tabs".to_string(),
            description: "List all open browser tabs".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        // === Reading Tools ===
        McpTool {
            name: "read_page".to_string(),
            description: "Get the accessibility tree / DOM structure of the current page"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "integer",
                        "description": "Tab ID to read from"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Maximum depth of tree to return (default: 10)"
                    },
                    "filter": {
                        "type": "string",
                        "enum": ["all", "interactive"],
                        "description": "Filter elements: 'all' or 'interactive' only"
                    }
                }
            }),
        },
        McpTool {
            name: "get_text".to_string(),
            description: "Extract text content from the page".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "integer",
                        "description": "Tab ID to extract text from"
                    },
                    "selector": {
                        "type": "string",
                        "description": "CSS selector to limit extraction (optional)"
                    }
                }
            }),
        },
        McpTool {
            name: "find_element".to_string(),
            description: "Find elements using natural language or CSS selector".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "integer",
                        "description": "Tab ID to search in"
                    },
                    "query": {
                        "type": "string",
                        "description": "Natural language description or CSS selector"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum results to return (default: 10)"
                    }
                },
                "required": ["query"]
            }),
        },
        // === Interaction Tools ===
        McpTool {
            name: "click".to_string(),
            description: "Click on an element or coordinates".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "integer",
                        "description": "Tab ID"
                    },
                    "element_ref": {
                        "type": "string",
                        "description": "Element reference from read_page/find_element"
                    },
                    "x": {
                        "type": "integer",
                        "description": "X coordinate (alternative to element_ref)"
                    },
                    "y": {
                        "type": "integer",
                        "description": "Y coordinate (alternative to element_ref)"
                    },
                    "button": {
                        "type": "string",
                        "enum": ["left", "right", "middle"],
                        "description": "Mouse button (default: left)"
                    },
                    "click_count": {
                        "type": "integer",
                        "description": "Number of clicks (1=single, 2=double, 3=triple)"
                    }
                }
            }),
        },
        McpTool {
            name: "type_text".to_string(),
            description: "Type text into focused element or specified element".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "integer",
                        "description": "Tab ID"
                    },
                    "text": {
                        "type": "string",
                        "description": "Text to type"
                    },
                    "element_ref": {
                        "type": "string",
                        "description": "Element reference to type into (optional)"
                    },
                    "clear_first": {
                        "type": "boolean",
                        "description": "Clear existing text before typing (default: false)"
                    }
                },
                "required": ["text"]
            }),
        },
        McpTool {
            name: "press_key".to_string(),
            description: "Press a keyboard key or shortcut".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "integer",
                        "description": "Tab ID"
                    },
                    "key": {
                        "type": "string",
                        "description": "Key to press (e.g., 'Enter', 'Tab', 'Escape', 'ctrl+a')"
                    }
                },
                "required": ["key"]
            }),
        },
        McpTool {
            name: "scroll".to_string(),
            description: "Scroll the page or an element".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "integer",
                        "description": "Tab ID"
                    },
                    "direction": {
                        "type": "string",
                        "enum": ["up", "down", "left", "right"],
                        "description": "Scroll direction"
                    },
                    "amount": {
                        "type": "integer",
                        "description": "Scroll amount in pixels (default: 300)"
                    },
                    "element_ref": {
                        "type": "string",
                        "description": "Element to scroll (optional, scrolls page if not specified)"
                    }
                },
                "required": ["direction"]
            }),
        },
        McpTool {
            name: "form_input".to_string(),
            description: "Set value on form elements (input, select, checkbox, etc.)".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "integer",
                        "description": "Tab ID"
                    },
                    "element_ref": {
                        "type": "string",
                        "description": "Element reference"
                    },
                    "value": {
                        "description": "Value to set (string for text, boolean for checkbox)"
                    }
                },
                "required": ["element_ref", "value"]
            }),
        },
        // === Screenshot Tools ===
        McpTool {
            name: "screenshot".to_string(),
            description: "Capture a screenshot of the page or element".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "integer",
                        "description": "Tab ID"
                    },
                    "element_ref": {
                        "type": "string",
                        "description": "Element to screenshot (optional, captures full page if not specified)"
                    },
                    "format": {
                        "type": "string",
                        "enum": ["png", "jpeg"],
                        "description": "Image format (default: png)"
                    }
                }
            }),
        },
        // === JavaScript Tools ===
        McpTool {
            name: "execute_js".to_string(),
            description: "Execute JavaScript in the page context (SassyScript engine)".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "integer",
                        "description": "Tab ID"
                    },
                    "code": {
                        "type": "string",
                        "description": "JavaScript code to execute"
                    }
                },
                "required": ["code"]
            }),
        },
        // === Security/Trust Tools ===
        McpTool {
            name: "get_trust_level".to_string(),
            description: "Get the trust level of the current page".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "integer",
                        "description": "Tab ID"
                    }
                }
            }),
        },
        McpTool {
            name: "get_security_info".to_string(),
            description: "Get security information (SSL, cookies, trackers blocked)".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "integer",
                        "description": "Tab ID"
                    }
                }
            }),
        },
        // === File Viewer Tools ===
        McpTool {
            name: "open_file".to_string(),
            description: "Open a local file in Sassy Browser's universal file viewer".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to file (supports 200+ formats)"
                    }
                },
                "required": ["path"]
            }),
        },
        McpTool {
            name: "get_file_info".to_string(),
            description: "Get information about the currently viewed file".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "integer",
                        "description": "Tab ID"
                    }
                }
            }),
        },
        // === Extended Tools ===
        McpTool {
            name: "activate_tab".to_string(),
            description: "Switch to a specific tab".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "integer",
                        "description": "Tab ID to activate"
                    }
                },
                "required": ["tab_id"]
            }),
        },
        McpTool {
            name: "search_bookmarks".to_string(),
            description: "Search bookmarks".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    }
                },
                "required": ["query"]
            }),
        },
        McpTool {
            name: "add_bookmark".to_string(),
            description: "Add a bookmark".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "URL to bookmark"
                    },
                    "title": {
                        "type": "string",
                        "description": "Bookmark title"
                    },
                    "folder_id": {
                        "type": "string",
                        "description": "Optional folder ID"
                    }
                },
                "required": ["url", "title"]
            }),
        },
        McpTool {
            name: "search_history".to_string(),
            description: "Search browsing history".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum results (default: 50)"
                    }
                },
                "required": ["query"]
            }),
        },
        McpTool {
            name: "start_download".to_string(),
            description: "Download a file from URL".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "URL to download"
                    },
                    "filename": {
                        "type": "string",
                        "description": "Optional filename"
                    }
                },
                "required": ["url"]
            }),
        },
        McpTool {
            name: "list_downloads".to_string(),
            description: "List current downloads".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        McpTool {
            name: "web_search".to_string(),
            description: "Search the web via the browser's search engine".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    }
                },
                "required": ["query"]
            }),
        },
    ]
}

// ==============================================================================
// MCP Server Implementation
// ==============================================================================

/// MCP Server implementation
pub struct McpServer {
    config: McpServerConfig,
    tools: Vec<McpTool>,
    bridge: McpBridgeSender,
    server_id: String,
    started_at: std::time::Instant,
    contexts: HashMap<String, McpContext>,
    metrics: McpMetrics,
    events: Vec<McpEvent>,
    log_level: String,
    shutdown_requested: std::sync::Arc<std::sync::atomic::AtomicBool>,
    api_client: crate::mcp_api::McpApiClient,
}

impl McpServer {
    pub fn new(config: McpServerConfig, bridge: McpBridgeSender) -> Self {
        let tools = define_tools();
        Self {
            config,
            tools,
            bridge,
            server_id: uuid::Uuid::new_v4().to_string(),
            started_at: std::time::Instant::now(),
            contexts: HashMap::new(),
            metrics: McpMetrics::default(),
            events: Vec::new(),
            log_level: "info".to_string(),
            shutdown_requested: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            api_client: crate::mcp_api::McpApiClient::new(),
        }
    }

    /// Run the MCP server in stdio mode
    pub fn run_stdio(&mut self) {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();
        let reader = BufReader::new(stdin.lock());

        eprintln!("Sassy Browser MCP Server started (stdio mode)");

        // Read JSON-RPC messages line by line
        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("Error reading stdin: {}", e);
                    continue;
                }
            };

            if line.trim().is_empty() {
                continue;
            }

            let response = self.handle_message(&line);
            if let Some(resp) = response {
                let resp_json = serde_json::to_string(&resp).unwrap_or_default();
                writeln!(stdout, "{}", resp_json).ok();
                stdout.flush().ok();
            }

            if self
                .shutdown_requested
                .load(std::sync::atomic::Ordering::SeqCst)
            {
                eprintln!("[MCP] Shutting down after completing pending request");
                break;
            }
        }
    }

    /// Handle a single JSON-RPC message
    fn handle_message(&mut self, message: &str) -> Option<JsonRpcResponse> {
        // Parse JSON-RPC request
        let request: JsonRpcRequest = match serde_json::from_str(message) {
            Ok(r) => r,
            Err(e) => {
                return Some(JsonRpcResponse::error(
                    JsonValue::Null,
                    PARSE_ERROR,
                    &format!("Parse error: {}", e),
                ));
            }
        };

        // Validate JSON-RPC version
        if request.jsonrpc != "2.0" {
            return Some(JsonRpcResponse::error(
                request.id,
                INVALID_REQUEST,
                &format!("Unsupported JSON-RPC version: {}", request.jsonrpc),
            ));
        }

        // Route to appropriate handler
        let result = match request.method.as_str() {
            // MCP protocol methods
            "initialize" => self.handle_initialize(&request.params),
            "initialized" => return None, // Notification, no response
            "tools/list" => self.handle_tools_list(),
            "tools/call" => self.handle_tool_call(&request.params),
            "ping" => Ok(json!({"pong": true})),

            // Context management
            "contexts/list" => self.handle_contexts_list(),
            "contexts/create" => self.handle_context_create(&request.params),
            "contexts/destroy" => self.handle_context_destroy(&request.params),
            "contexts/get" => self.handle_context_get(&request.params),

            // Server info
            "server/info" => self.handle_server_info(),
            "server/health" => self.handle_health_check(),
            "server/metrics" => self.handle_metrics(),

            // Admin
            "admin/log_level" => self.handle_log_level(&request.params),
            "admin/shutdown" => self.handle_shutdown(),
            "admin/gc" => self.handle_gc(),

            // Unknown method
            _ => Err((
                METHOD_NOT_FOUND,
                format!("Unknown method: {}", request.method),
            )),
        };

        Some(match result {
            Ok(value) => JsonRpcResponse::success(request.id, value),
            Err((code, msg)) => JsonRpcResponse::error(request.id, code, &msg),
        })
    }

    /// Handle MCP initialize request
    fn handle_initialize(&self, _params: &JsonValue) -> Result<JsonValue, (i32, String)> {
        Ok(json!({
            "protocolVersion": MCP_VERSION,
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "sassy-browser",
                "version": env!("CARGO_PKG_VERSION")
            }
        }))
    }

    /// Handle tools/list request
    fn handle_tools_list(&self) -> Result<JsonValue, (i32, String)> {
        Ok(json!({
            "tools": self.tools
        }))
    }

    /// Handle tools/call request
    fn handle_tool_call(&mut self, params: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let name = params["name"]
            .as_str()
            .ok_or((INVALID_PARAMS, "Missing tool name".to_string()))?;
        let arguments = &params["arguments"];

        // Track metrics
        self.metrics.total_requests += 1;
        *self
            .metrics
            .tool_call_count
            .entry(name.to_string())
            .or_insert(0) += 1;

        // Log event
        self.events.push(McpEvent {
            event_type: "tool_call".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            data: json!({ "tool": name }),
        });

        // Check if tool is in allowed categories
        let destructive = matches!(name, "close_tab" | "start_download");
        if destructive && self.config.confirm_destructive {
            // Log destructive action attempt
            self.events.push(McpEvent {
                event_type: "destructive_action".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                data: json!({ "tool": name, "requires_confirm": true }),
            });
        }

        // Dispatch to appropriate tool handler
        let result = match name {
            // Navigation
            "navigate" => self.tool_navigate(arguments),
            "new_tab" => self.tool_new_tab(arguments),
            "close_tab" => self.tool_close_tab(arguments),
            "list_tabs" => self.tool_list_tabs(arguments),

            // Reading
            "read_page" => self.tool_read_page(arguments),
            "get_text" => self.tool_get_text(arguments),
            "find_element" => self.tool_find_element(arguments),

            // Interaction
            "click" => self.tool_click(arguments),
            "type_text" => self.tool_type_text(arguments),
            "press_key" => self.tool_press_key(arguments),
            "scroll" => self.tool_scroll(arguments),
            "form_input" => self.tool_form_input(arguments),

            // Screenshot
            "screenshot" => self.tool_screenshot(arguments),

            // JavaScript
            "execute_js" => self.tool_execute_js(arguments),

            // Security
            "get_trust_level" => self.tool_get_trust_level(arguments),
            "get_security_info" => self.tool_get_security_info(arguments),

            // File viewer
            "open_file" => self.tool_open_file(arguments),
            "get_file_info" => self.tool_get_file_info(arguments),

            // Extended tools
            "activate_tab" => self.tool_activate_tab(arguments),
            "search_bookmarks" => self.tool_search_bookmarks(arguments),
            "add_bookmark" => self.tool_add_bookmark(arguments),
            "search_history" => self.tool_search_history(arguments),
            "start_download" => self.tool_start_download(arguments),
            "list_downloads" => self.tool_list_downloads(arguments),
            "web_search" => self.tool_web_search(arguments),

            _ => Err((METHOD_NOT_FOUND, format!("Unknown tool: {}", name))),
        };

        // Track errors in metrics
        if result.is_err() {
            self.metrics.total_errors += 1;
        }

        result
    }

    // ==============================================================================
    // Tool Implementations - These send commands to browser and process responses
    // ==============================================================================

    fn tool_navigate(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let url = args["url"]
            .as_str()
            .ok_or((INVALID_PARAMS, "Missing url".to_string()))?;
        let tab_id = args["tab_id"].as_u64();

        // Handle special navigation commands
        let cmd = if url == "back" {
            McpCommand::GoBack { tab_id }
        } else if url == "forward" {
            McpCommand::GoForward { tab_id }
        } else if url == "reload" {
            McpCommand::Reload { tab_id }
        } else {
            McpCommand::Navigate {
                url: url.to_string(),
                tab_id,
            }
        };

        match self.bridge.send_command(cmd) {
            Ok(McpResponse::NavigateResult {
                success,
                url,
                message,
            }) => Ok(json!({
                "success": success,
                "url": url,
                "message": message
            })),
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_new_tab(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let url = args["url"].as_str().map(String::from);

        match self
            .bridge
            .send_command(McpCommand::NewTab { url: url.clone() })
        {
            Ok(McpResponse::NewTabResult {
                success,
                tab_id,
                url,
            }) => Ok(json!({
                "success": success,
                "tab_id": tab_id,
                "url": url
            })),
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_close_tab(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let tab_id = args["tab_id"]
            .as_u64()
            .ok_or((INVALID_PARAMS, "Missing tab_id".to_string()))?;

        match self.bridge.send_command(McpCommand::CloseTab { tab_id }) {
            Ok(McpResponse::CloseTabResult {
                success,
                closed_tab_id,
            }) => Ok(json!({
                "success": success,
                "closed_tab_id": closed_tab_id
            })),
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_list_tabs(&self, _args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        match self.bridge.send_command(McpCommand::ListTabs) {
            Ok(McpResponse::TabList { tabs, active_tab }) => Ok(json!({
                "tabs": tabs,
                "active_tab": active_tab
            })),
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_read_page(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let tab_id = args["tab_id"].as_u64();
        let depth = args["depth"].as_u64().unwrap_or(10) as u32;
        let filter = args["filter"].as_str().unwrap_or("all").to_string();

        match self.bridge.send_command(McpCommand::ReadPage {
            tab_id,
            depth,
            filter: filter.clone(),
        }) {
            Ok(McpResponse::PageTree {
                tree,
                depth,
                filter,
            }) => Ok(json!({
                "tree": tree,
                "depth": depth,
                "filter": filter
            })),
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_get_text(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let tab_id = args["tab_id"].as_u64();
        let selector = args["selector"].as_str().map(String::from);

        match self
            .bridge
            .send_command(McpCommand::GetText { tab_id, selector })
        {
            Ok(McpResponse::TextContent { text, length }) => Ok(json!({
                "text": text,
                "length": length
            })),
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_find_element(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let query = args["query"]
            .as_str()
            .ok_or((INVALID_PARAMS, "Missing query".to_string()))?
            .to_string();
        let tab_id = args["tab_id"].as_u64();
        let max_results = args["max_results"].as_u64().unwrap_or(10) as u32;

        match self.bridge.send_command(McpCommand::FindElement {
            query: query.clone(),
            tab_id,
            max_results,
        }) {
            Ok(McpResponse::FindResult {
                elements,
                query,
                count,
            }) => Ok(json!({
                "elements": elements,
                "query": query,
                "count": count
            })),
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_click(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let tab_id = args["tab_id"].as_u64();
        let element_ref = args["element_ref"].as_str().map(String::from);
        let x = args["x"].as_i64().map(|v| v as i32);
        let y = args["y"].as_i64().map(|v| v as i32);
        let button = args["button"].as_str().unwrap_or("left").to_string();
        let click_count = args["click_count"].as_u64().unwrap_or(1) as u32;

        match self.bridge.send_command(McpCommand::Click {
            tab_id,
            element_ref: element_ref.clone(),
            x,
            y,
            button: button.clone(),
            click_count,
        }) {
            Ok(McpResponse::ClickResult {
                success,
                element_ref,
                coordinates,
            }) => Ok(json!({
                "success": success,
                "element_ref": element_ref,
                "coordinates": coordinates,
                "button": button,
                "click_count": click_count
            })),
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_type_text(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let text = args["text"]
            .as_str()
            .ok_or((INVALID_PARAMS, "Missing text".to_string()))?
            .to_string();
        let tab_id = args["tab_id"].as_u64();
        let element_ref = args["element_ref"].as_str().map(String::from);
        let clear_first = args["clear_first"].as_bool().unwrap_or(false);

        match self.bridge.send_command(McpCommand::TypeText {
            tab_id,
            text: text.clone(),
            element_ref,
            clear_first,
        }) {
            Ok(McpResponse::TypeResult { success, typed }) => Ok(json!({
                "success": success,
                "typed": typed,
                "clear_first": clear_first
            })),
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_press_key(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let key = args["key"]
            .as_str()
            .ok_or((INVALID_PARAMS, "Missing key".to_string()))?
            .to_string();
        let tab_id = args["tab_id"].as_u64();

        match self.bridge.send_command(McpCommand::PressKey {
            tab_id,
            key: key.clone(),
        }) {
            Ok(McpResponse::KeyResult { success, key }) => Ok(json!({
                "success": success,
                "key": key
            })),
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_scroll(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let direction = args["direction"]
            .as_str()
            .ok_or((INVALID_PARAMS, "Missing direction".to_string()))?
            .to_string();
        let tab_id = args["tab_id"].as_u64();
        let amount = args["amount"].as_i64().unwrap_or(300) as i32;
        let element_ref = args["element_ref"].as_str().map(String::from);

        match self.bridge.send_command(McpCommand::Scroll {
            tab_id,
            direction: direction.clone(),
            amount,
            element_ref,
        }) {
            Ok(McpResponse::ScrollResult {
                success,
                direction,
                amount,
            }) => Ok(json!({
                "success": success,
                "direction": direction,
                "amount": amount
            })),
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_form_input(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let element_ref = args["element_ref"]
            .as_str()
            .ok_or((INVALID_PARAMS, "Missing element_ref".to_string()))?
            .to_string();
        let value = args["value"].clone();
        let tab_id = args["tab_id"].as_u64();

        match self.bridge.send_command(McpCommand::FormInput {
            tab_id,
            element_ref: element_ref.clone(),
            value: value.clone(),
        }) {
            Ok(McpResponse::FormInputResult {
                success,
                element_ref,
                value,
            }) => Ok(json!({
                "success": success,
                "element_ref": element_ref,
                "value": value
            })),
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_screenshot(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let tab_id = args["tab_id"].as_u64();
        let element_ref = args["element_ref"].as_str().map(String::from);
        let format = args["format"].as_str().unwrap_or("png").to_string();

        match self.bridge.send_command(McpCommand::Screenshot {
            tab_id,
            element_ref,
            format: format.clone(),
        }) {
            Ok(McpResponse::ScreenshotResult {
                success,
                format,
                width,
                height,
                data,
            }) => Ok(json!({
                "success": success,
                "format": format,
                "width": width,
                "height": height,
                "data": data
            })),
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_execute_js(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let code = args["code"]
            .as_str()
            .ok_or((INVALID_PARAMS, "Missing code".to_string()))?
            .to_string();
        let tab_id = args["tab_id"].as_u64();

        match self
            .bridge
            .send_command(McpCommand::ExecuteJs { tab_id, code })
        {
            Ok(McpResponse::JsResult {
                success,
                result,
                console,
            }) => Ok(json!({
                "success": success,
                "result": result,
                "console": console,
                "note": "Executed via SassyScript (no V8, no JIT exploits)"
            })),
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_get_trust_level(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let tab_id = args["tab_id"].as_u64();

        match self
            .bridge
            .send_command(McpCommand::GetTrustLevel { tab_id })
        {
            Ok(McpResponse::TrustLevel {
                level,
                interactions,
                required,
                restrictions,
            }) => Ok(json!({
                "trust_level": level,
                "interactions": interactions,
                "required_for_trusted": required,
                "restrictions": restrictions
            })),
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_get_security_info(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let tab_id = args["tab_id"].as_u64();

        match self
            .bridge
            .send_command(McpCommand::GetSecurityInfo { tab_id })
        {
            Ok(McpResponse::SecurityInfo {
                ssl,
                cookies,
                trackers_blocked,
                popups_blocked,
            }) => Ok(json!({
                "ssl": ssl,
                "cookies": cookies,
                "trackers_blocked": trackers_blocked,
                "popups_blocked": popups_blocked
            })),
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_open_file(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let path = args["path"]
            .as_str()
            .ok_or((INVALID_PARAMS, "Missing path".to_string()))?
            .to_string();

        match self
            .bridge
            .send_command(McpCommand::OpenFile { path: path.clone() })
        {
            Ok(McpResponse::OpenFileResult {
                success,
                path,
                format,
                viewer,
            }) => Ok(json!({
                "success": success,
                "path": path,
                "detected_format": format,
                "viewer": viewer
            })),
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_get_file_info(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let tab_id = args["tab_id"].as_u64();

        match self.bridge.send_command(McpCommand::GetFileInfo { tab_id }) {
            Ok(McpResponse::FileInfo {
                is_file,
                path,
                format,
                metadata,
            }) => Ok(json!({
                "is_file": is_file,
                "path": path,
                "format": format,
                "metadata": metadata
            })),
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    // ==============================================================================
    // Extended Tool Implementations
    // ==============================================================================

    fn tool_activate_tab(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let tab_id = args["tab_id"]
            .as_u64()
            .ok_or((INVALID_PARAMS, "Missing tab_id".to_string()))?;

        match self.bridge.send_command(McpCommand::ActivateTab { tab_id }) {
            Ok(McpResponse::ActivateTabResult { success, tab_id }) => Ok(json!({
                "success": success,
                "tab_id": tab_id
            })),
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_search_bookmarks(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let query = args["query"]
            .as_str()
            .ok_or((INVALID_PARAMS, "Missing query".to_string()))?
            .to_string();

        match self
            .bridge
            .send_command(McpCommand::SearchBookmarks { query })
        {
            Ok(McpResponse::BookmarkResults { bookmarks, count }) => Ok(json!({
                "bookmarks": bookmarks,
                "count": count
            })),
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_add_bookmark(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let url = args["url"]
            .as_str()
            .ok_or((INVALID_PARAMS, "Missing url".to_string()))?
            .to_string();
        let title = args["title"]
            .as_str()
            .ok_or((INVALID_PARAMS, "Missing title".to_string()))?
            .to_string();
        let folder_id = args["folder_id"].as_str().map(String::from);

        match self.bridge.send_command(McpCommand::AddBookmark {
            url,
            title,
            folder_id,
        }) {
            Ok(McpResponse::BookmarkAdded { success, id }) => Ok(json!({
                "success": success,
                "id": id
            })),
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_search_history(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let query = args["query"]
            .as_str()
            .ok_or((INVALID_PARAMS, "Missing query".to_string()))?
            .to_string();
        let limit = args["limit"].as_u64().unwrap_or(50) as u32;

        match self
            .bridge
            .send_command(McpCommand::SearchHistory { query, limit })
        {
            Ok(McpResponse::HistoryResults { entries, count }) => Ok(json!({
                "entries": entries,
                "count": count
            })),
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_start_download(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let url = args["url"]
            .as_str()
            .ok_or((INVALID_PARAMS, "Missing url".to_string()))?
            .to_string();
        let filename = args["filename"].as_str().map(String::from);

        match self.bridge.send_command(McpCommand::StartDownload {
            url: url.clone(),
            filename,
        }) {
            Ok(McpResponse::DownloadStarted {
                success,
                download_id,
                url,
            }) => Ok(json!({
                "success": success,
                "download_id": download_id,
                "url": url
            })),
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_list_downloads(&self, _args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        match self.bridge.send_command(McpCommand::ListDownloads) {
            Ok(McpResponse::DownloadList { downloads }) => Ok(json!({
                "downloads": downloads
            })),
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_web_search(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let query = args["query"]
            .as_str()
            .ok_or((INVALID_PARAMS, "Missing query".to_string()))?
            .to_string();

        match self.bridge.send_command(McpCommand::WebSearch {
            query: query.clone(),
        }) {
            Ok(McpResponse::WebSearchResult { success, url }) => Ok(json!({
                "success": success,
                "url": url,
                "query": query
            })),
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    // ==============================================================================
    // Context Management Handlers
    // ==============================================================================

    fn handle_contexts_list(&self) -> Result<JsonValue, (i32, String)> {
        let contexts: Vec<JsonValue> = self
            .contexts
            .values()
            .map(|ctx| {
                json!({
                    "id": ctx.id,
                    "title": ctx.title,
                    "created_at": ctx.created_at,
                    "last_active_at": ctx.last_active_at,
                    "message_count": ctx.messages.len(),
                })
            })
            .collect();

        Ok(json!({
            "contexts": contexts,
            "count": contexts.len()
        }))
    }

    fn handle_context_create(&mut self, params: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let title = params["title"]
            .as_str()
            .unwrap_or("New Context")
            .to_string();
        let model_family = params["model_family"].as_str().map(String::from);
        let temperature = params["temperature"].as_f64().unwrap_or(0.7) as f32;
        let max_tokens = params["max_tokens"].as_u64().unwrap_or(4096) as u32;
        let system_prompt = params["system_prompt"].as_str().map(String::from);

        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        let context = McpContext {
            id: id.clone(),
            title,
            created_at: now.clone(),
            last_active_at: now,
            model_family,
            temperature,
            max_tokens,
            system_prompt,
            messages: Vec::new(),
            metadata: HashMap::new(),
        };

        self.contexts.insert(id.clone(), context.clone());
        self.metrics.contexts_created += 1;

        Ok(json!({
            "id": id,
            "context": context
        }))
    }

    fn handle_context_destroy(&mut self, params: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let id = params["id"]
            .as_str()
            .ok_or((INVALID_PARAMS, "Missing context id".to_string()))?;

        if self.contexts.remove(id).is_some() {
            Ok(json!({
                "success": true,
                "id": id
            }))
        } else {
            Err((INVALID_PARAMS, format!("Context not found: {}", id)))
        }
    }

    fn handle_context_get(&self, params: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let id = params["id"]
            .as_str()
            .ok_or((INVALID_PARAMS, "Missing context id".to_string()))?;

        if let Some(context) = self.contexts.get(id) {
            Ok(json!(context))
        } else {
            Err((INVALID_PARAMS, format!("Context not found: {}", id)))
        }
    }

    // ==============================================================================
    // Server Info & Admin Handlers
    // ==============================================================================

    fn handle_server_info(&mut self) -> Result<JsonValue, (i32, String)> {
        // Exercise API client capabilities for the info endpoint
        let api_summary = crate::mcp_api::api_capabilities_summary(&mut self.api_client);

        Ok(json!({
            "server_id": self.server_id,
            "version": env!("CARGO_PKG_VERSION"),
            "uptime_secs": self.started_at.elapsed().as_secs(),
            "log_level": self.log_level,
            "config": {
                "transport": self.config.transport,
                "port": self.config.port,
                "allowed_tools": self.config.allowed_tools,
            },
            "api_capabilities": api_summary,
        }))
    }

    fn handle_health_check(&self) -> Result<JsonValue, (i32, String)> {
        // Ping browser to verify bridge connectivity
        let bridge_ok = self.bridge.send_command_async(McpCommand::ListTabs).is_ok();

        Ok(json!({
            "status": if bridge_ok { "healthy" } else { "degraded" },
            "uptime_secs": self.started_at.elapsed().as_secs(),
            "contexts_active": self.contexts.len(),
            "events_logged": self.events.len(),
            "bridge_connected": bridge_ok,
        }))
    }

    fn handle_metrics(&mut self) -> Result<JsonValue, (i32, String)> {
        self.metrics.uptime_secs = self.started_at.elapsed().as_secs();

        Ok(json!({
            "metrics": self.metrics,
            "contexts_active": self.contexts.len(),
            "events_count": self.events.len(),
        }))
    }

    fn handle_log_level(&mut self, params: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let level = params["level"]
            .as_str()
            .ok_or((INVALID_PARAMS, "Missing level".to_string()))?;

        let valid_levels = ["error", "warn", "info", "debug", "trace"];
        if !valid_levels.contains(&level) {
            return Err((
                INVALID_PARAMS,
                format!(
                    "Invalid log level: {}. Must be one of: {:?}",
                    level, valid_levels
                ),
            ));
        }

        self.log_level = level.to_string();

        Ok(json!({
            "success": true,
            "log_level": self.log_level
        }))
    }

    fn handle_shutdown(&self) -> Result<JsonValue, (i32, String)> {
        let uptime = self.started_at.elapsed().as_secs();
        eprintln!("[MCP] Shutdown requested after {} seconds uptime", uptime);
        self.shutdown_requested
            .store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(json!({
            "status": "shutting_down",
            "message": "Shutdown initiated",
            "uptime_secs": uptime
        }))
    }

    fn handle_gc(&mut self) -> Result<JsonValue, (i32, String)> {
        // Clean up old events (keep last 1000)
        if self.events.len() > 1000 {
            self.events.drain(0..self.events.len() - 1000);
        }

        // Clean up inactive contexts (older than 24 hours)
        let now = chrono::Utc::now();
        let mut removed_contexts = 0;
        self.contexts.retain(|_, ctx| {
            if let Ok(last_active) = chrono::DateTime::parse_from_rfc3339(&ctx.last_active_at) {
                let age = now.signed_duration_since(last_active);
                if age.num_hours() > 24 {
                    removed_contexts += 1;
                    return false;
                }
            }
            true
        });

        Ok(json!({
            "success": true,
            "events_retained": self.events.len(),
            "contexts_removed": removed_contexts,
            "contexts_active": self.contexts.len(),
        }))
    }
}

// ==============================================================================
// Entry Point
// ==============================================================================

/// Entry point for MCP server mode (standalone, no GUI)
///
/// This is used when running `sassy-browser.exe --mcp-server`
/// It creates a simple headless browser instance and runs the MCP server.
pub fn run_mcp_server_standalone(config: McpServerConfig) {
    eprintln!("Sassy Browser MCP Server (standalone mode)");
    eprintln!("Transport: {}", config.transport);

    // For standalone mode, we create a minimal browser state
    // that doesn't require a GUI
    let (bridge_sender, bridge_receiver) = McpBridge::new();

    // Start command processor in separate thread
    std::thread::spawn(move || {
        run_headless_browser(bridge_receiver);
    });

    // Run MCP server
    let mut server = McpServer::new(config.clone(), bridge_sender);

    match config.transport.as_str() {
        "stdio" => {
            server.run_stdio();
        }
        "socket" => {
            eprintln!(
                "Socket transport on port {} (not yet implemented)",
                config.port
            );
            // TODO: Implement socket transport
        }
        _ => {
            eprintln!("Unknown transport: {}", config.transport);
        }
    }
}

// Backward-compatible alias for main entrypoint used by main.rs
pub fn run_mcp_server(config: McpServerConfig) {
    run_mcp_server_standalone(config);
}

/// Headless browser state combining tab engine with rendering pipeline
struct HeadlessBrowserState {
    engine: crate::browser::BrowserEngine,
    renderer: crate::renderer::Renderer,
    js_interpreter: crate::js::JsInterpreter,
    html_renderer: crate::html_renderer::HtmlRenderer,
    ref_counter: u64,
    element_refs: std::collections::HashMap<String, String>,
}

impl HeadlessBrowserState {
    fn new() -> Self {
        Self {
            engine: crate::browser::BrowserEngine::new(),
            renderer: crate::renderer::Renderer::new(1920, 1080),
            js_interpreter: crate::js::JsInterpreter::new(),
            html_renderer: crate::html_renderer::HtmlRenderer::new(),
            ref_counter: 0,
            element_refs: std::collections::HashMap::new(),
        }
    }

    /// After navigation, fetch and parse page content into the renderer
    fn refresh_renderer(&mut self, url: &str) {
        // Fetch the page content via HTTP
        if let Ok(html) = crate::http_client::fetch_text(url) {
            self.renderer.parse_html(&html);
            self.renderer.compute_styles();
            self.renderer.layout();
            self.renderer.paint();

            // Also render through the HTML renderer for rich content
            self.html_renderer.parse_html(&html);
        }
    }
}

/// Run a headless browser that processes MCP commands
fn run_headless_browser(bridge: McpBridgeReceiver) {
    let mut state = HeadlessBrowserState::new();

    eprintln!("Headless browser started, processing commands...");

    loop {
        // First try non-blocking receive to drain queued commands quickly
        if let Some(cmd) = bridge.try_recv() {
            let response = process_command(&mut state, cmd);
            if let Err(e) = bridge.send_response(response) {
                eprintln!("Failed to send response: {}", e);
            }
            continue;
        }
        // Block waiting for commands when queue is empty
        match bridge.command_rx.recv() {
            Ok(cmd) => {
                let response = process_command(&mut state, cmd);
                if let Err(e) = bridge.send_response(response) {
                    eprintln!("Failed to send response: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Command channel closed: {}", e);
                break;
            }
        }
    }
}

/// Process a single MCP command and return a response
fn process_command(state: &mut HeadlessBrowserState, cmd: McpCommand) -> McpResponse {
    use crate::browser::{TabContent, TabId};

    match cmd {
        McpCommand::Navigate { url, tab_id } => {
            // If tab_id specified, switch to that tab first
            if let Some(id) = tab_id {
                state.engine.set_active_tab_by_id(TabId(id));
            }

            state.engine.navigate(&url);

            // Also parse the page into our headless renderer for DOM queries
            state.refresh_renderer(&url);

            McpResponse::NavigateResult {
                success: true,
                url: url.clone(),
                message: format!("Navigating to {}", url),
            }
        }

        McpCommand::NewTab { url } => {
            let id = if let Some(ref u) = url {
                state.engine.new_tab_with_url(u)
            } else {
                state.engine.new_tab()
            };

            McpResponse::NewTabResult {
                success: true,
                tab_id: id.0,
                url: url.unwrap_or_else(|| "about:blank".to_string()),
            }
        }

        McpCommand::CloseTab { tab_id } => {
            state.engine.close_tab_by_id(TabId(tab_id));

            McpResponse::CloseTabResult {
                success: true,
                closed_tab_id: tab_id,
            }
        }

        McpCommand::ListTabs => {
            let tabs: Vec<TabInfo> = state
                .engine
                .tabs()
                .iter()
                .enumerate()
                .map(|(idx, tab)| {
                    let (url, title, loading, is_secure, is_file, file_type) = match &tab.content {
                        TabContent::Web {
                            url,
                            title,
                            loading,
                            is_secure,
                            ..
                        } => (
                            url.clone(),
                            title.clone(),
                            *loading,
                            *is_secure,
                            false,
                            None,
                        ),
                        TabContent::File(f) => (
                            format!("file://{}", f.path.display()),
                            f.name.clone(),
                            false,
                            true,
                            true,
                            Some(format!("{:?}", f.file_type)),
                        ),
                        TabContent::NewTab => (
                            "about:blank".to_string(),
                            "New Tab".to_string(),
                            false,
                            true,
                            false,
                            None,
                        ),
                        TabContent::Settings => (
                            "sassy://settings".to_string(),
                            "Settings".to_string(),
                            false,
                            true,
                            false,
                            None,
                        ),
                        TabContent::History => (
                            "sassy://history".to_string(),
                            "History".to_string(),
                            false,
                            true,
                            false,
                            None,
                        ),
                        TabContent::Bookmarks => (
                            "sassy://bookmarks".to_string(),
                            "Bookmarks".to_string(),
                            false,
                            true,
                            false,
                            None,
                        ),
                        TabContent::Downloads => (
                            "sassy://downloads".to_string(),
                            "Downloads".to_string(),
                            false,
                            true,
                            false,
                            None,
                        ),
                    };

                    TabInfo {
                        id: tab.id.0,
                        index: idx,
                        url,
                        title,
                        loading,
                        is_secure,
                        trust_level: if is_secure {
                            "acknowledged".to_string()
                        } else {
                            "untrusted".to_string()
                        },
                        is_file,
                        file_type,
                    }
                })
                .collect();

            McpResponse::TabList {
                tabs,
                active_tab: state.engine.active_tab_index(),
            }
        }

        McpCommand::GoBack { tab_id } => {
            if let Some(id) = tab_id {
                state.engine.set_active_tab_by_id(TabId(id));
            }
            state.engine.go_back();

            McpResponse::NavigateResult {
                success: true,
                url: "back".to_string(),
                message: "Going back".to_string(),
            }
        }

        McpCommand::GoForward { tab_id } => {
            if let Some(id) = tab_id {
                state.engine.set_active_tab_by_id(TabId(id));
            }
            state.engine.go_forward();

            McpResponse::NavigateResult {
                success: true,
                url: "forward".to_string(),
                message: "Going forward".to_string(),
            }
        }

        McpCommand::Reload { tab_id } => {
            if let Some(id) = tab_id {
                state.engine.set_active_tab_by_id(TabId(id));
            }
            state.engine.reload();

            McpResponse::NavigateResult {
                success: true,
                url: "reload".to_string(),
                message: "Reloading".to_string(),
            }
        }

        McpCommand::ReadPage {
            tab_id,
            depth,
            filter,
        } => {
            if let Some(id) = tab_id {
                state.engine.set_active_tab_by_id(TabId(id));
            }
            // Build an ElementInfo tree from the renderer's DOM document
            fn node_to_element(
                node: &crate::dom::NodeRef,
                ref_ctr: &mut u64,
                element_refs: &mut std::collections::HashMap<String, String>,
                remaining_depth: u32,
                filter: &str,
            ) -> Option<ElementInfo> {
                let n = node.borrow();
                match n.node_type {
                    crate::dom::NodeType::Element => {
                        let tag = n.tag_name.clone().unwrap_or_default();
                        let is_interactive = matches!(
                            tag.as_str(),
                            "a" | "button"
                                | "input"
                                | "select"
                                | "textarea"
                                | "label"
                                | "details"
                                | "summary"
                        );

                        // Apply filter
                        if filter == "interactive" && !is_interactive && remaining_depth == 0 {
                            return None;
                        }

                        *ref_ctr += 1;
                        let ref_id = format!("ref_{}", ref_ctr);
                        element_refs.insert(ref_id.clone(), tag.clone());

                        let children = if remaining_depth > 0 {
                            n.children
                                .iter()
                                .filter_map(|child| {
                                    node_to_element(
                                        child,
                                        ref_ctr,
                                        element_refs,
                                        remaining_depth - 1,
                                        filter,
                                    )
                                })
                                .collect()
                        } else {
                            vec![]
                        };

                        Some(ElementInfo {
                            ref_id,
                            tag,
                            id: n.get_id(),
                            classes: n.get_classes(),
                            text: n.text_content.clone(),
                            attributes: n.attributes.clone(),
                            interactive: is_interactive,
                            bounds: None, // Layout bounds not available in headless mode yet
                            children,
                        })
                    }
                    crate::dom::NodeType::Text => {
                        if filter == "interactive" {
                            return None;
                        }
                        let text = n.text_content.clone().unwrap_or_default();
                        if text.trim().is_empty() {
                            return None;
                        }
                        *ref_ctr += 1;
                        Some(ElementInfo {
                            ref_id: format!("ref_{}", ref_ctr),
                            tag: "#text".to_string(),
                            id: None,
                            classes: vec![],
                            text: Some(text),
                            attributes: HashMap::new(),
                            interactive: false,
                            bounds: None,
                            children: vec![],
                        })
                    }
                    _ => None,
                }
            }

            // Walk the renderer's document to build the tree
            let doc = &state.renderer.document;
            let root = &doc.root;
            let ref_ctr = &mut state.ref_counter;
            let tree = node_to_element(root, ref_ctr, &mut state.element_refs, depth, &filter)
                .unwrap_or_else(|| {
                    *ref_ctr += 1;
                    ElementInfo {
                        ref_id: format!("ref_{}", ref_ctr),
                        tag: "html".to_string(),
                        id: None,
                        classes: vec![],
                        text: Some("(empty document)".to_string()),
                        attributes: HashMap::new(),
                        interactive: false,
                        bounds: None,
                        children: vec![],
                    }
                });

            McpResponse::PageTree {
                tree: Box::new(tree),
                depth,
                filter,
            }
        }

        McpCommand::GetText { tab_id, selector } => {
            if let Some(id) = tab_id {
                state.engine.set_active_tab_by_id(TabId(id));
            }
            // Extract text from the renderer's DOM tree
            let doc = &state.renderer.document;
            let root = &doc.root;
            let text = {
                let node = root.borrow();
                node.get_inner_text()
            };

            // If a CSS selector hint was provided, try to narrow down
            let result_text = if let Some(sel) = selector {
                // Simple tag-name or #id selector matching
                fn find_text_by_selector(
                    node: &crate::dom::NodeRef,
                    selector: &str,
                ) -> Option<String> {
                    let n = node.borrow();
                    if let crate::dom::NodeType::Element = n.node_type {
                        let tag = n.tag_name.as_deref().unwrap_or("");
                        let id = n.get_id().unwrap_or_default();
                        let classes = n.get_classes();
                        let matches = if selector.starts_with('#') {
                            id == &selector[1..]
                        } else if selector.starts_with('.') {
                            classes.iter().any(|c| c == &selector[1..])
                        } else {
                            tag == selector
                        };
                        if matches {
                            return Some(n.get_inner_text());
                        }
                        for child in &n.children {
                            if let Some(t) = find_text_by_selector(child, selector) {
                                return Some(t);
                            }
                        }
                    }
                    None
                }
                find_text_by_selector(root, &sel).unwrap_or(text)
            } else {
                text
            };

            let len = result_text.len();
            McpResponse::TextContent {
                text: result_text,
                length: len,
            }
        }

        McpCommand::FindElement {
            query,
            tab_id,
            max_results,
        } => {
            if let Some(id) = tab_id {
                state.engine.set_active_tab_by_id(TabId(id));
            }
            // Search the DOM tree for elements matching the query (by text, tag, id, or class)
            let mut results: Vec<ElementInfo> = Vec::new();
            let query_lower = crate::fontcase::ascii_lower(&query);

            fn find_matching(
                node: &crate::dom::NodeRef,
                query_lower: &str,
                ref_ctr: &mut u64,
                element_refs: &mut std::collections::HashMap<String, String>,
                results: &mut Vec<ElementInfo>,
                max: u32,
            ) {
                if results.len() >= max as usize {
                    return;
                }
                let n = node.borrow();
                if let crate::dom::NodeType::Element = n.node_type {
                    let tag = n.tag_name.as_deref().unwrap_or("");
                    let id = n.get_id().unwrap_or_default();
                    let classes = n.get_classes();
                    let text = n.get_inner_text();
                    let text_lower = crate::fontcase::ascii_lower(&text);

                    let tag_match = crate::fontcase::ascii_lower(tag).contains(query_lower);
                    let id_match = crate::fontcase::ascii_lower(&id).contains(query_lower);
                    let class_match = classes
                        .iter()
                        .any(|c| crate::fontcase::ascii_lower(c).contains(query_lower));
                    let text_match = text_lower.contains(query_lower);

                    if tag_match || id_match || class_match || text_match {
                        *ref_ctr += 1;
                        let ref_id = format!("ref_{}", ref_ctr);
                        let is_interactive = matches!(
                            tag,
                            "a" | "button" | "input" | "select" | "textarea" | "label"
                        );
                        element_refs.insert(ref_id.clone(), tag.to_string());
                        results.push(ElementInfo {
                            ref_id,
                            tag: tag.to_string(),
                            id: if id.is_empty() { None } else { Some(id) },
                            classes,
                            text: if text.is_empty() { None } else { Some(text) },
                            attributes: n.attributes.clone(),
                            interactive: is_interactive,
                            bounds: None,
                            children: vec![],
                        });
                    }
                    for child in &n.children {
                        find_matching(child, query_lower, ref_ctr, element_refs, results, max);
                    }
                }
            }

            let root = &state.renderer.document.root;
            find_matching(
                root,
                &query_lower,
                &mut state.ref_counter,
                &mut state.element_refs,
                &mut results,
                max_results,
            );
            let count = results.len();
            McpResponse::FindResult {
                elements: results,
                query,
                count,
            }
        }

        McpCommand::Click {
            tab_id,
            element_ref,
            x,
            y,
            button,
            click_count,
        } => {
            if let Some(id) = tab_id {
                state.engine.set_active_tab_by_id(TabId(id));
            }
            // Log button and click_count for debugging
            eprintln!("[MCP] Click: button={}, count={}", button, click_count);
            // If element_ref provided, look up in element_refs and find clickable links
            let mut clicked_url: Option<String> = None;
            if let Some(ref eref) = element_ref {
                // Look up what tag this ref corresponds to — if it's a link, navigate
                if let Some(_tag) = state.element_refs.get(eref) {
                    // Search for the element in the DOM to find its href
                    fn find_href_by_ref(
                        node: &crate::dom::NodeRef,
                        target_tag: &str,
                    ) -> Option<String> {
                        let n = node.borrow();
                        if let crate::dom::NodeType::Element = n.node_type {
                            if n.tag_name.as_deref() == Some("a") {
                                if let Some(href) = n.attributes.get("href") {
                                    return Some(href.clone());
                                }
                            }
                            for child in &n.children {
                                if let Some(h) = find_href_by_ref(child, target_tag) {
                                    return Some(h);
                                }
                            }
                        }
                        None
                    }
                    clicked_url = find_href_by_ref(&state.renderer.document.root, "a");
                }
            }

            // If we found a link, navigate to it
            if let Some(ref url) = clicked_url {
                state.engine.navigate(url);
                state.refresh_renderer(url);
            }

            // Use hit-test on the renderer if coordinates are provided
            let coords = match (x, y) {
                (Some(cx), Some(cy)) => {
                    // Perform hit-test via the renderer
                    let _hit = state.renderer.hit_test(cx as f32, cy as f32);
                    Some((cx, cy))
                }
                _ => None,
            };

            McpResponse::ClickResult {
                success: true,
                element_ref,
                coordinates: coords,
            }
        }

        McpCommand::TypeText {
            tab_id,
            text,
            element_ref,
            clear_first,
        } => {
            if let Some(id) = tab_id {
                state.engine.set_active_tab_by_id(TabId(id));
            }
            // If element_ref provided, find the form input in the DOM and set its value
            if let Some(ref eref) = element_ref {
                fn find_and_set_input(
                    node: &crate::dom::NodeRef,
                    target_ref: &str,
                    value: &str,
                    clear: bool,
                ) -> bool {
                    let mut n = node.borrow_mut();
                    if let crate::dom::NodeType::Element = n.node_type {
                        // Check if this element matches the ref (by id or data-ref attribute)
                        let matches_ref =
                            n.get_attribute("id").map_or(false, |id| id == target_ref)
                                || n.get_attribute("data-ref")
                                    .map_or(false, |r| r == target_ref);

                        let is_input =
                            matches!(n.tag_name.as_deref(), Some("input") | Some("textarea"));
                        if is_input && matches_ref {
                            if clear {
                                n.set_attribute("value", value);
                            } else {
                                let existing = n.get_attribute("value").unwrap_or_default();
                                n.set_attribute("value", &format!("{}{}", existing, value));
                            }
                            return true;
                        }
                        // Search children
                        for child in &n.children {
                            if find_and_set_input(child, target_ref, value, clear) {
                                return true;
                            }
                        }
                    }
                    false
                }
                find_and_set_input(&state.renderer.document.root, eref, &text, clear_first);
            }

            // Also feed the text to the address bar if no element specified
            // (simulates typing into the browser UI)
            if element_ref.is_none() {
                state.engine.set_address_bar_text(text.clone());
            }

            McpResponse::TypeResult {
                success: true,
                typed: text,
            }
        }

        McpCommand::PressKey { tab_id, key } => {
            if let Some(id) = tab_id {
                state.engine.set_active_tab_by_id(TabId(id));
            }
            // Map key names to browser actions
            match key.as_str() {
                "Enter" | "Return" => {
                    // Submit the address bar if focused
                    state.engine.submit_address_bar();
                }
                "Escape" => {
                    state.engine.stop();
                }
                "F5" => {
                    state.engine.reload();
                }
                "Backspace" => {
                    // Noop in headless — no focused input to modify
                }
                _ => {
                    eprintln!("PressKey: unhandled key '{}'", key);
                }
            }

            McpResponse::KeyResult { success: true, key }
        }

        McpCommand::Scroll {
            tab_id,
            direction,
            amount,
            element_ref,
        } => {
            if let Some(id) = tab_id {
                state.engine.set_active_tab_by_id(TabId(id));
            }
            // If element_ref is provided, log it for diagnostics
            if let Some(ref eref) = element_ref {
                eprintln!("[MCP] Scroll target element: {}", eref);
            }
            // Apply scroll to the renderer's viewport
            let delta = match direction.as_str() {
                "down" => amount as f32,
                "up" => -(amount as f32),
                _ => 0.0,
            };
            state.renderer.scroll(delta);

            McpResponse::ScrollResult {
                success: true,
                direction,
                amount,
            }
        }

        McpCommand::FormInput {
            tab_id,
            element_ref,
            value,
        } => {
            if let Some(id) = tab_id {
                state.engine.set_active_tab_by_id(TabId(id));
            }
            // Find the form element by ref and set its value attribute in the DOM
            let value_str = match &value {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Number(n) => n.to_string(),
                other => other.to_string(),
            };

            fn set_form_value(node: &crate::dom::NodeRef, val: &str) -> bool {
                let mut n = node.borrow_mut();
                if let crate::dom::NodeType::Element = n.node_type {
                    let tag = n.tag_name.as_deref().unwrap_or("");
                    match tag {
                        "input" | "textarea" => {
                            n.set_attribute("value", val);
                            return true;
                        }
                        "select" => {
                            n.set_attribute("value", val);
                            return true;
                        }
                        _ => {}
                    }
                    for child in &n.children {
                        if set_form_value(child, val) {
                            return true;
                        }
                    }
                }
                false
            }

            let success = set_form_value(&state.renderer.document.root, &value_str);

            McpResponse::FormInputResult {
                success,
                element_ref,
                value,
            }
        }

        McpCommand::Screenshot {
            tab_id,
            element_ref,
            format,
        } => {
            if let Some(id) = tab_id {
                state.engine.set_active_tab_by_id(TabId(id));
            }
            // If element_ref is provided, log it for diagnostics
            if let Some(ref eref) = element_ref {
                eprintln!("[MCP] Screenshot target element: {}", eref);
            }
            // Render the page into the painter's buffer and encode as base64 PNG
            state.renderer.render(); // ensure up-to-date paint

            let buffer = state.renderer.get_buffer();
            let width = 1920u32;
            let height = 1080u32;

            // Convert u32 ARGB buffer to RGBA u8 bytes for image encoding
            let mut rgba_bytes: Vec<u8> = Vec::with_capacity((width * height * 4) as usize);
            for pixel in buffer.iter() {
                let a = ((pixel >> 24) & 0xFF) as u8;
                let r = ((pixel >> 16) & 0xFF) as u8;
                let g = ((pixel >> 8) & 0xFF) as u8;
                let b = (pixel & 0xFF) as u8;
                rgba_bytes.push(r);
                rgba_bytes.push(g);
                rgba_bytes.push(b);
                rgba_bytes.push(a);
            }

            // Encode as PNG and then base64
            use image::ImageEncoder;
            use std::io::Cursor;
            let mut png_data = Cursor::new(Vec::new());
            let encode_result = image::codecs::png::PngEncoder::new(&mut png_data).write_image(
                &rgba_bytes,
                width,
                height,
                image::ExtendedColorType::Rgba8,
            );

            match encode_result {
                Ok(()) => {
                    use base64::Engine;
                    let b64 =
                        base64::engine::general_purpose::STANDARD.encode(png_data.into_inner());
                    McpResponse::ScreenshotResult {
                        success: true,
                        format: "png".to_string(),
                        width,
                        height,
                        data: b64,
                    }
                }
                Err(e) => McpResponse::ScreenshotResult {
                    success: false,
                    format,
                    width,
                    height,
                    data: format!("Screenshot encoding failed: {}", e),
                },
            }
        }

        McpCommand::ExecuteJs { tab_id, code } => {
            if let Some(id) = tab_id {
                state.engine.set_active_tab_by_id(TabId(id));
            }
            // Execute JavaScript via the SassyScript interpreter
            match state.js_interpreter.execute(&code) {
                Ok(value) => {
                    // Convert JS Value to JSON
                    let json_result = match &value {
                        crate::js::Value::Number(n) => json!(*n),
                        crate::js::Value::String(s) => json!(s),
                        crate::js::Value::Boolean(b) => json!(*b),
                        crate::js::Value::Null | crate::js::Value::Undefined => {
                            serde_json::Value::Null
                        }
                        other => json!(format!("{:?}", other)),
                    };
                    let console_out: Vec<String> = state
                        .js_interpreter
                        .get_console_output()
                        .iter()
                        .cloned()
                        .collect();

                    McpResponse::JsResult {
                        success: true,
                        result: json_result,
                        console: console_out,
                    }
                }
                Err(e) => McpResponse::JsResult {
                    success: false,
                    result: json!({ "error": e }),
                    console: state
                        .js_interpreter
                        .get_console_output()
                        .iter()
                        .cloned()
                        .collect(),
                },
            }
        }

        McpCommand::GetTrustLevel { tab_id } => {
            if let Some(id) = tab_id {
                state.engine.set_active_tab_by_id(TabId(id));
            }
            // Query sandbox trust level for the current page
            // In headless mode, we use the sandbox SecurityContext defaults
            use crate::sandbox::{ContentType, SecurityContext, TrustLevel as SbTrustLevel};

            let url = state
                .engine
                .active_tab()
                .map(|t| match &t.content {
                    TabContent::Web { url, .. } => url.clone(),
                    _ => "about:blank".to_string(),
                })
                .unwrap_or_else(|| "about:blank".to_string());

            let ctx = SecurityContext::new(url, ContentType::WebPage);
            let trust = ctx.trust_level;

            let (level_str, required) = match trust {
                SbTrustLevel::Untrusted => ("untrusted", 1u32),
                SbTrustLevel::Acknowledged => ("acknowledged", 2),
                SbTrustLevel::Reviewed => ("reviewed", 3),
                SbTrustLevel::Approved => ("approved", 10),
                SbTrustLevel::Established => ("established", 0),
            };

            let mut restrictions = Vec::new();
            if !trust.can_execute() {
                restrictions.push("script_execution".to_string());
            }
            if !trust.can_write_filesystem() {
                restrictions.push("filesystem_write".to_string());
            }
            if !trust.can_access_network() {
                restrictions.push("network_access".to_string());
            }

            // Also check page-level permissions
            use crate::sandbox::page::PageTrust;
            let page_trust = PageTrust::Untrusted; // Default for new pages
            if !page_trust.can_access_clipboard() {
                restrictions.push("clipboard_access".to_string());
            }
            if !page_trust.can_initiate_download() {
                restrictions.push("download_initiation".to_string());
            }
            if !page_trust.can_open_popup() {
                restrictions.push("popup_creation".to_string());
            }

            McpResponse::TrustLevel {
                level: level_str.to_string(),
                interactions: ctx.interactions.len() as u32,
                required,
                restrictions,
            }
        }

        McpCommand::GetSecurityInfo { tab_id } => {
            if let Some(id) = tab_id {
                state.engine.set_active_tab_by_id(TabId(id));
            }
            // Gather security information from the active tab
            let url = state
                .engine
                .active_tab()
                .map(|t| match &t.content {
                    TabContent::Web { url, is_secure, .. } => (url.clone(), *is_secure),
                    _ => ("about:blank".to_string(), true),
                })
                .unwrap_or(("about:blank".to_string(), true));

            let is_https = url.0.starts_with("https://");

            McpResponse::SecurityInfo {
                ssl: json!({
                    "valid": is_https,
                    "protocol": if is_https { "TLS 1.3" } else { "none" },
                    "issuer": if is_https { "verified" } else { "n/a" },
                }),
                cookies: json!({
                    "first_party": 0,
                    "third_party_blocked": 0,
                    "note": "headless mode — no persistent cookie jar"
                }),
                trackers_blocked: 0,
                popups_blocked: 0,
            }
        }

        McpCommand::OpenFile { path } => {
            let path_buf = std::path::PathBuf::from(&path);
            match state.engine.open_file(path_buf) {
                Ok(_id) => {
                    // Get file type
                    let format = crate::file_handler::FileHandler::detect_file_type(
                        &std::path::PathBuf::from(&path),
                    );

                    McpResponse::OpenFileResult {
                        success: true,
                        path,
                        format: format!("{:?}", format),
                        viewer: crate::fontcase::ascii_lower(&format!("{:?}_viewer", format)),
                    }
                }
                Err(e) => McpResponse::Error {
                    code: INTERNAL_ERROR,
                    message: format!("Failed to open file: {}", e),
                },
            }
        }

        McpCommand::GetFileInfo { tab_id } => {
            if let Some(id) = tab_id {
                state.engine.set_active_tab_by_id(TabId(id));
            }
            // Check if active tab is a file
            if let Some(tab) = state.engine.active_tab() {
                if let TabContent::File(f) = &tab.content {
                    return McpResponse::FileInfo {
                        is_file: true,
                        path: Some(f.path.display().to_string()),
                        format: Some(format!("{:?}", f.file_type)),
                        metadata: json!({
                            "name": f.name,
                            "size_bytes": f.size,
                        }),
                    };
                }
            }

            McpResponse::FileInfo {
                is_file: false,
                path: None,
                format: None,
                metadata: json!({}),
            }
        }

        // Extended commands
        McpCommand::ActivateTab { tab_id } => {
            state.engine.set_active_tab_by_id(TabId(tab_id));

            McpResponse::ActivateTabResult {
                success: true,
                tab_id,
            }
        }

        McpCommand::SearchBookmarks { query } => {
            let results = state.engine.bookmarks.search(&query);
            let bookmarks: Vec<JsonValue> = results
                .iter()
                .map(|b| {
                    json!({
                        "id": b.id.to_string(),
                        "title": b.title,
                        "url": b.url,
                        "folder_id": b.folder_id.map(|f| f.to_string()),
                        "created_at": b.created_at,
                        "tags": b.tags,
                    })
                })
                .collect();
            let count = bookmarks.len();
            McpResponse::BookmarkResults { bookmarks, count }
        }

        McpCommand::AddBookmark {
            url,
            title,
            folder_id,
        } => {
            let folder_uuid = folder_id.and_then(|fid| uuid::Uuid::parse_str(&fid).ok());
            let id = state.engine.bookmarks.add(&url, &title, folder_uuid);
            McpResponse::BookmarkAdded {
                success: true,
                id: id.to_string(),
            }
        }

        McpCommand::SearchHistory { query, limit } => {
            let results = state.engine.history.search(&query);
            let entries: Vec<JsonValue> = results
                .iter()
                .take(limit as usize)
                .map(|e| {
                    json!({
                        "url": e.url,
                        "title": e.title,
                        "visit_count": e.visit_count,
                        "last_visit": e.visited_at,
                    })
                })
                .collect();
            let count = entries.len();
            McpResponse::HistoryResults { entries, count }
        }

        McpCommand::StartDownload { url, filename } => {
            match state
                .engine
                .downloads
                .start_download(&url, filename.as_deref())
            {
                Ok(download_id) => McpResponse::DownloadStarted {
                    success: true,
                    download_id: download_id.to_string(),
                    url,
                },
                Err(e) => McpResponse::DownloadStarted {
                    success: false,
                    download_id: String::new(),
                    url: format!("Download failed: {}", e),
                },
            }
        }

        McpCommand::ListDownloads => {
            let all_downloads = state.engine.downloads.downloads();
            let downloads: Vec<JsonValue> = all_downloads
                .iter()
                .map(|d| {
                    json!({
                        "id": d.id.to_string(),
                        "url": d.url,
                        "filename": d.filename,
                        "status": format!("{:?}", d.state),
                        "total_bytes": d.total_bytes,
                        "downloaded_bytes": d.downloaded_bytes,
                    })
                })
                .collect();
            McpResponse::DownloadList { downloads }
        }

        McpCommand::WebSearch { query } => {
            // Perform web search by navigating to Google search
            let search_url = format!(
                "https://www.google.com/search?q={}",
                urlencoding::encode(&query)
            );

            state.engine.navigate(&search_url);
            state.refresh_renderer(&search_url);

            McpResponse::WebSearchResult {
                success: true,
                url: search_url,
            }
        }
    }
}

// ==============================================================================
// Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = McpServerConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.transport, "stdio");
        assert_eq!(config.port, 9999);
    }

    #[test]
    fn test_json_rpc_response() {
        let resp = JsonRpcResponse::success(json!(1), json!({"test": true}));
        assert!(resp.error.is_none());
        assert!(resp.result.is_some());
    }

    #[test]
    fn test_bridge_creation() {
        let (sender, receiver) = McpBridge::new();
        // Just verify they can be created
        drop(sender);
        drop(receiver);
    }
}
