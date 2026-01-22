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

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::sync::mpsc::{channel, Receiver, Sender};

// ============================================================================
// Configuration
// ============================================================================

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

// ============================================================================
// MCP Protocol Types
// ============================================================================

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

// ============================================================================
// Command/Response Types for Browser Communication
// ============================================================================

/// Commands sent from MCP server to browser engine
#[derive(Debug, Clone)]
pub enum McpCommand {
    // Navigation
    Navigate { url: String, tab_id: Option<u64> },
    NewTab { url: Option<String> },
    CloseTab { tab_id: u64 },
    ListTabs,
    GoBack { tab_id: Option<u64> },
    GoForward { tab_id: Option<u64> },
    Reload { tab_id: Option<u64> },

    // Reading
    ReadPage { tab_id: Option<u64>, depth: u32, filter: String },
    GetText { tab_id: Option<u64>, selector: Option<String> },
    FindElement { query: String, tab_id: Option<u64>, max_results: u32 },

    // Interaction
    Click { tab_id: Option<u64>, element_ref: Option<String>, x: Option<i32>, y: Option<i32>, button: String, click_count: u32 },
    TypeText { tab_id: Option<u64>, text: String, element_ref: Option<String>, clear_first: bool },
    PressKey { tab_id: Option<u64>, key: String },
    Scroll { tab_id: Option<u64>, direction: String, amount: i32, element_ref: Option<String> },
    FormInput { tab_id: Option<u64>, element_ref: String, value: JsonValue },

    // Screenshot
    Screenshot { tab_id: Option<u64>, element_ref: Option<String>, format: String },

    // JavaScript
    ExecuteJs { tab_id: Option<u64>, code: String },

    // Security
    GetTrustLevel { tab_id: Option<u64> },
    GetSecurityInfo { tab_id: Option<u64> },

    // File Viewer
    OpenFile { path: String },
    GetFileInfo { tab_id: Option<u64> },
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
    NavigateResult { success: bool, url: String, message: String },
    NewTabResult { success: bool, tab_id: u64, url: String },
    CloseTabResult { success: bool, closed_tab_id: u64 },
    TabList { tabs: Vec<TabInfo>, active_tab: usize },

    // Reading
    PageTree { tree: Box<ElementInfo>, depth: u32, filter: String },
    TextContent { text: String, length: usize },
    FindResult { elements: Vec<ElementInfo>, query: String, count: usize },

    // Interaction
    ClickResult { success: bool, element_ref: Option<String>, coordinates: Option<(i32, i32)> },
    TypeResult { success: bool, typed: String },
    KeyResult { success: bool, key: String },
    ScrollResult { success: bool, direction: String, amount: i32 },
    FormInputResult { success: bool, element_ref: String, value: JsonValue },

    // Screenshot
    ScreenshotResult { success: bool, format: String, width: u32, height: u32, data: String },

    // JavaScript
    JsResult { success: bool, result: JsonValue, console: Vec<String> },

    // Security
    TrustLevel { level: String, interactions: u32, required: u32, restrictions: Vec<String> },
    SecurityInfo { ssl: JsonValue, cookies: JsonValue, trackers_blocked: u32, popups_blocked: u32 },

    // File
    OpenFileResult { success: bool, path: String, format: String, viewer: String },
    FileInfo { is_file: bool, path: Option<String>, format: Option<String>, metadata: JsonValue },

    // Error
    Error { code: i32, message: String },
}

// ============================================================================
// Browser Bridge - Communication channel between MCP server and browser
// ============================================================================

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

        let sender = McpBridgeSender {
            command_tx: cmd_tx,
            response_rx: resp_rx,
        };

        let receiver = McpBridgeReceiver {
            command_rx: cmd_rx,
            response_tx: resp_tx,
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
        self.command_tx.send(cmd).map_err(|e| format!("Failed to send command: {}", e))?;
        self.response_rx.recv().map_err(|e| format!("Failed to receive response: {}", e))
    }

    /// Send a command without waiting (for notifications)
    pub fn send_command_async(&self, cmd: McpCommand) -> Result<(), String> {
        self.command_tx.send(cmd).map_err(|e| format!("Failed to send command: {}", e))
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
        self.response_tx.send(response).map_err(|e| format!("Failed to send response: {}", e))
    }
}

// ============================================================================
// MCP Tool Definitions
// ============================================================================

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
            description: "Get the accessibility tree / DOM structure of the current page".to_string(),
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
    ]
}

// ============================================================================
// MCP Server Implementation
// ============================================================================

/// MCP Server implementation
pub struct McpServer {
    config: McpServerConfig,
    tools: Vec<McpTool>,
    bridge: McpBridgeSender,
}

impl McpServer {
    pub fn new(config: McpServerConfig, bridge: McpBridgeSender) -> Self {
        let tools = define_tools();
        Self {
            config,
            tools,
            bridge,
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

        // Route to appropriate handler
        let result = match request.method.as_str() {
            // MCP protocol methods
            "initialize" => self.handle_initialize(&request.params),
            "initialized" => return None, // Notification, no response
            "tools/list" => self.handle_tools_list(),
            "tools/call" => self.handle_tool_call(&request.params),
            "ping" => Ok(json!({"pong": true})),

            // Unknown method
            _ => Err((METHOD_NOT_FOUND, format!("Unknown method: {}", request.method))),
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

        // Dispatch to appropriate tool handler
        match name {
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

            _ => Err((METHOD_NOT_FOUND, format!("Unknown tool: {}", name))),
        }
    }

    // =========================================================================
    // Tool Implementations - These send commands to browser and process responses
    // =========================================================================

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
            Ok(McpResponse::NavigateResult { success, url, message }) => {
                Ok(json!({
                    "success": success,
                    "url": url,
                    "message": message
                }))
            }
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_new_tab(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let url = args["url"].as_str().map(String::from);

        match self.bridge.send_command(McpCommand::NewTab { url: url.clone() }) {
            Ok(McpResponse::NewTabResult { success, tab_id, url }) => {
                Ok(json!({
                    "success": success,
                    "tab_id": tab_id,
                    "url": url
                }))
            }
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
            Ok(McpResponse::CloseTabResult { success, closed_tab_id }) => {
                Ok(json!({
                    "success": success,
                    "closed_tab_id": closed_tab_id
                }))
            }
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_list_tabs(&self, _args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        match self.bridge.send_command(McpCommand::ListTabs) {
            Ok(McpResponse::TabList { tabs, active_tab }) => {
                Ok(json!({
                    "tabs": tabs,
                    "active_tab": active_tab
                }))
            }
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_read_page(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let tab_id = args["tab_id"].as_u64();
        let depth = args["depth"].as_u64().unwrap_or(10) as u32;
        let filter = args["filter"].as_str().unwrap_or("all").to_string();

        match self.bridge.send_command(McpCommand::ReadPage { tab_id, depth, filter: filter.clone() }) {
            Ok(McpResponse::PageTree { tree, depth, filter }) => {
                Ok(json!({
                    "tree": tree,
                    "depth": depth,
                    "filter": filter
                }))
            }
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_get_text(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let tab_id = args["tab_id"].as_u64();
        let selector = args["selector"].as_str().map(String::from);

        match self.bridge.send_command(McpCommand::GetText { tab_id, selector }) {
            Ok(McpResponse::TextContent { text, length }) => {
                Ok(json!({
                    "text": text,
                    "length": length
                }))
            }
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

        match self.bridge.send_command(McpCommand::FindElement { query: query.clone(), tab_id, max_results }) {
            Ok(McpResponse::FindResult { elements, query, count }) => {
                Ok(json!({
                    "elements": elements,
                    "query": query,
                    "count": count
                }))
            }
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

        match self.bridge.send_command(McpCommand::Click { tab_id, element_ref: element_ref.clone(), x, y, button: button.clone(), click_count }) {
            Ok(McpResponse::ClickResult { success, element_ref, coordinates }) => {
                Ok(json!({
                    "success": success,
                    "element_ref": element_ref,
                    "coordinates": coordinates,
                    "button": button,
                    "click_count": click_count
                }))
            }
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

        match self.bridge.send_command(McpCommand::TypeText { tab_id, text: text.clone(), element_ref, clear_first }) {
            Ok(McpResponse::TypeResult { success, typed }) => {
                Ok(json!({
                    "success": success,
                    "typed": typed,
                    "clear_first": clear_first
                }))
            }
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

        match self.bridge.send_command(McpCommand::PressKey { tab_id, key: key.clone() }) {
            Ok(McpResponse::KeyResult { success, key }) => {
                Ok(json!({
                    "success": success,
                    "key": key
                }))
            }
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

        match self.bridge.send_command(McpCommand::Scroll { tab_id, direction: direction.clone(), amount, element_ref }) {
            Ok(McpResponse::ScrollResult { success, direction, amount }) => {
                Ok(json!({
                    "success": success,
                    "direction": direction,
                    "amount": amount
                }))
            }
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

        match self.bridge.send_command(McpCommand::FormInput { tab_id, element_ref: element_ref.clone(), value: value.clone() }) {
            Ok(McpResponse::FormInputResult { success, element_ref, value }) => {
                Ok(json!({
                    "success": success,
                    "element_ref": element_ref,
                    "value": value
                }))
            }
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_screenshot(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let tab_id = args["tab_id"].as_u64();
        let element_ref = args["element_ref"].as_str().map(String::from);
        let format = args["format"].as_str().unwrap_or("png").to_string();

        match self.bridge.send_command(McpCommand::Screenshot { tab_id, element_ref, format: format.clone() }) {
            Ok(McpResponse::ScreenshotResult { success, format, width, height, data }) => {
                Ok(json!({
                    "success": success,
                    "format": format,
                    "width": width,
                    "height": height,
                    "data": data
                }))
            }
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

        match self.bridge.send_command(McpCommand::ExecuteJs { tab_id, code }) {
            Ok(McpResponse::JsResult { success, result, console }) => {
                Ok(json!({
                    "success": success,
                    "result": result,
                    "console": console,
                    "note": "Executed via SassyScript (no V8, no JIT exploits)"
                }))
            }
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_get_trust_level(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let tab_id = args["tab_id"].as_u64();

        match self.bridge.send_command(McpCommand::GetTrustLevel { tab_id }) {
            Ok(McpResponse::TrustLevel { level, interactions, required, restrictions }) => {
                Ok(json!({
                    "trust_level": level,
                    "interactions": interactions,
                    "required_for_trusted": required,
                    "restrictions": restrictions
                }))
            }
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_get_security_info(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let tab_id = args["tab_id"].as_u64();

        match self.bridge.send_command(McpCommand::GetSecurityInfo { tab_id }) {
            Ok(McpResponse::SecurityInfo { ssl, cookies, trackers_blocked, popups_blocked }) => {
                Ok(json!({
                    "ssl": ssl,
                    "cookies": cookies,
                    "trackers_blocked": trackers_blocked,
                    "popups_blocked": popups_blocked
                }))
            }
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

        match self.bridge.send_command(McpCommand::OpenFile { path: path.clone() }) {
            Ok(McpResponse::OpenFileResult { success, path, format, viewer }) => {
                Ok(json!({
                    "success": success,
                    "path": path,
                    "detected_format": format,
                    "viewer": viewer
                }))
            }
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }

    fn tool_get_file_info(&self, args: &JsonValue) -> Result<JsonValue, (i32, String)> {
        let tab_id = args["tab_id"].as_u64();

        match self.bridge.send_command(McpCommand::GetFileInfo { tab_id }) {
            Ok(McpResponse::FileInfo { is_file, path, format, metadata }) => {
                Ok(json!({
                    "is_file": is_file,
                    "path": path,
                    "format": format,
                    "metadata": metadata
                }))
            }
            Ok(McpResponse::Error { code, message }) => Err((code, message)),
            Err(e) => Err((INTERNAL_ERROR, e)),
            _ => Err((INTERNAL_ERROR, "Unexpected response".to_string())),
        }
    }
}

// ============================================================================
// Entry Point
// ============================================================================

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
            eprintln!("Socket transport on port {} (not yet implemented)", config.port);
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

/// Run a headless browser that processes MCP commands
fn run_headless_browser(bridge: McpBridgeReceiver) {
    use crate::browser::BrowserEngine;
    
    let mut engine = BrowserEngine::new();
    let mut ref_counter: u64 = 0;
    let mut element_refs: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    
    eprintln!("Headless browser started, processing commands...");
    
    loop {
        // Block waiting for commands
        match bridge.command_rx.recv() {
            Ok(cmd) => {
                let response = process_command(&mut engine, &mut ref_counter, &mut element_refs, cmd);
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
fn process_command(
    engine: &mut crate::browser::BrowserEngine,
    ref_counter: &mut u64,
    _element_refs: &mut std::collections::HashMap<String, String>,
    cmd: McpCommand,
) -> McpResponse {
    use crate::browser::{TabContent, TabId};
    
    match cmd {
        McpCommand::Navigate { url, tab_id } => {
            // If tab_id specified, switch to that tab first
            if let Some(id) = tab_id {
                engine.set_active_tab_by_id(TabId(id));
            }
            
            engine.navigate(&url);
            
            McpResponse::NavigateResult {
                success: true,
                url: url.clone(),
                message: format!("Navigating to {}", url),
            }
        }
        
        McpCommand::NewTab { url } => {
            let id = if let Some(ref u) = url {
                engine.new_tab_with_url(u)
            } else {
                engine.new_tab()
            };
            
            McpResponse::NewTabResult {
                success: true,
                tab_id: id.0,
                url: url.unwrap_or_else(|| "about:blank".to_string()),
            }
        }
        
        McpCommand::CloseTab { tab_id } => {
            engine.close_tab_by_id(TabId(tab_id));
            
            McpResponse::CloseTabResult {
                success: true,
                closed_tab_id: tab_id,
            }
        }
        
        McpCommand::ListTabs => {
            let tabs: Vec<TabInfo> = engine.tabs()
                .iter()
                .enumerate()
                .map(|(idx, tab)| {
                    let (url, title, loading, is_secure, is_file, file_type) = match &tab.content {
                        TabContent::Web { url, title, loading, is_secure, .. } => {
                            (url.clone(), title.clone(), *loading, *is_secure, false, None)
                        }
                        TabContent::File(f) => {
                            (format!("file://{}", f.path.display()), f.name.clone(), false, true, true, Some(format!("{:?}", f.file_type)))
                        }
                        TabContent::NewTab => ("about:blank".to_string(), "New Tab".to_string(), false, true, false, None),
                        TabContent::Settings => ("sassy://settings".to_string(), "Settings".to_string(), false, true, false, None),
                        TabContent::History => ("sassy://history".to_string(), "History".to_string(), false, true, false, None),
                        TabContent::Bookmarks => ("sassy://bookmarks".to_string(), "Bookmarks".to_string(), false, true, false, None),
                        TabContent::Downloads => ("sassy://downloads".to_string(), "Downloads".to_string(), false, true, false, None),
                    };
                    
                    TabInfo {
                        id: tab.id.0,
                        index: idx,
                        url,
                        title,
                        loading,
                        is_secure,
                        trust_level: "untrusted".to_string(), // TODO: Get from sandbox
                        is_file,
                        file_type,
                    }
                })
                .collect();
            
            McpResponse::TabList {
                tabs,
                active_tab: engine.active_tab_index(),
            }
        }
        
        McpCommand::GoBack { tab_id } => {
            if let Some(id) = tab_id {
                engine.set_active_tab_by_id(TabId(id));
            }
            engine.go_back();
            
            McpResponse::NavigateResult {
                success: true,
                url: "back".to_string(),
                message: "Going back".to_string(),
            }
        }
        
        McpCommand::GoForward { tab_id } => {
            if let Some(id) = tab_id {
                engine.set_active_tab_by_id(TabId(id));
            }
            engine.go_forward();
            
            McpResponse::NavigateResult {
                success: true,
                url: "forward".to_string(),
                message: "Going forward".to_string(),
            }
        }
        
        McpCommand::Reload { tab_id } => {
            if let Some(id) = tab_id {
                engine.set_active_tab_by_id(TabId(id));
            }
            engine.reload();
            
            McpResponse::NavigateResult {
                success: true,
                url: "reload".to_string(),
                message: "Reloading".to_string(),
            }
        }
        
        McpCommand::ReadPage { tab_id: _, depth, filter } => {
            // TODO: Wire to actual DOM tree from html_renderer
            // For now, return a placeholder structure
            *ref_counter += 1;
            let root_ref = format!("ref_{}", ref_counter);
            
            McpResponse::PageTree {
                tree: Box::new(ElementInfo {
                    ref_id: root_ref,
                    tag: "html".to_string(),
                    id: None,
                    classes: vec![],
                    text: None,
                    attributes: HashMap::new(),
                    interactive: false,
                    bounds: None,
                    children: vec![
                        ElementInfo {
                            ref_id: { *ref_counter += 1; format!("ref_{}", ref_counter) },
                            tag: "body".to_string(),
                            id: None,
                            classes: vec![],
                            text: Some("Page content".to_string()),
                            attributes: HashMap::new(),
                            interactive: false,
                            bounds: Some(ElementBounds { x: 0.0, y: 0.0, width: 1920.0, height: 1080.0 }),
                            children: vec![],
                        }
                    ],
                }),
                depth,
                filter,
            }
        }
        
        McpCommand::GetText { tab_id: _, selector: _ } => {
            // TODO: Wire to DOM text extraction
            McpResponse::TextContent {
                text: "Page text content".to_string(),
                length: 17,
            }
        }
        
        McpCommand::FindElement { query, tab_id: _, max_results: _ } => {
            // TODO: Wire to DOM query
            *ref_counter += 1;
            
            McpResponse::FindResult {
                elements: vec![
                    ElementInfo {
                        ref_id: format!("ref_{}", ref_counter),
                        tag: "button".to_string(),
                        id: Some("submit".to_string()),
                        classes: vec!["btn".to_string()],
                        text: Some("Submit".to_string()),
                        attributes: HashMap::new(),
                        interactive: true,
                        bounds: Some(ElementBounds { x: 100.0, y: 200.0, width: 80.0, height: 30.0 }),
                        children: vec![],
                    }
                ],
                query,
                count: 1,
            }
        }
        
        McpCommand::Click { tab_id: _, element_ref, x, y, button: _, click_count: _ } => {
            // TODO: Wire to input handling
            McpResponse::ClickResult {
                success: true,
                element_ref,
                coordinates: match (x, y) {
                    (Some(x), Some(y)) => Some((x, y)),
                    _ => None,
                },
            }
        }
        
        McpCommand::TypeText { tab_id: _, text, element_ref: _, clear_first: _ } => {
            // TODO: Wire to input handling
            McpResponse::TypeResult {
                success: true,
                typed: text,
            }
        }
        
        McpCommand::PressKey { tab_id: _, key } => {
            // TODO: Wire to input handling
            McpResponse::KeyResult {
                success: true,
                key,
            }
        }
        
        McpCommand::Scroll { tab_id: _, direction, amount, element_ref: _ } => {
            // TODO: Wire to scroll handling
            McpResponse::ScrollResult {
                success: true,
                direction,
                amount,
            }
        }
        
        McpCommand::FormInput { tab_id: _, element_ref, value } => {
            // TODO: Wire to form handling
            McpResponse::FormInputResult {
                success: true,
                element_ref,
                value,
            }
        }
        
        McpCommand::Screenshot { tab_id: _, element_ref: _, format } => {
            // TODO: Wire to paint.rs screenshot capture
            McpResponse::ScreenshotResult {
                success: true,
                format,
                width: 1920,
                height: 1080,
                data: "base64_placeholder".to_string(),
            }
        }
        
        McpCommand::ExecuteJs { tab_id: _, code: _ } => {
            // TODO: Wire to js/interpreter.rs
            McpResponse::JsResult {
                success: true,
                result: serde_json::Value::Null,
                console: vec![],
            }
        }
        
        McpCommand::GetTrustLevel { tab_id: _ } => {
            // TODO: Wire to sandbox/page.rs
            McpResponse::TrustLevel {
                level: "untrusted".to_string(),
                interactions: 0,
                required: 3,
                restrictions: vec![
                    "clipboard_access".to_string(),
                    "download_initiation".to_string(),
                    "notification_requests".to_string(),
                    "popup_creation".to_string(),
                ],
            }
        }
        
        McpCommand::GetSecurityInfo { tab_id: _ } => {
            // TODO: Wire to security modules
            McpResponse::SecurityInfo {
                ssl: json!({
                    "valid": true,
                    "issuer": "Unknown",
                    "expires": "Unknown"
                }),
                cookies: json!({
                    "first_party": 0,
                    "third_party_blocked": 0
                }),
                trackers_blocked: 0,
                popups_blocked: 0,
            }
        }
        
        McpCommand::OpenFile { path } => {
            let path_buf = std::path::PathBuf::from(&path);
            match engine.open_file(path_buf) {
                Ok(_id) => {
                    // Get file type
                    let format = crate::file_handler::FileHandler::detect_file_type(&std::path::PathBuf::from(&path));
                    
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
        
        McpCommand::GetFileInfo { tab_id: _ } => {
            // Check if active tab is a file
            if let Some(tab) = engine.active_tab() {
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
    }
}

// ============================================================================
// Tests
// ============================================================================

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
