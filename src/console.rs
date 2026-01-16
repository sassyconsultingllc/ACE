//! Developer Console
//!
//! Interactive JavaScript console, network inspector, and DOM explorer.
//! The core feature that makes Sassy the developer's browser.

#![allow(dead_code)]

use crate::style::Color;
use crate::syntax::{SyntaxHighlighter, Language};
use std::collections::VecDeque;

/// Console message level
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogLevel {
    Log,
    Info,
    Warn,
    Error,
    Debug,
}

impl LogLevel {
    pub fn color(&self) -> Color {
        match self {
            LogLevel::Log => Color::new(220, 220, 220, 255),
            LogLevel::Info => Color::new(100, 180, 255, 255),
            LogLevel::Warn => Color::new(255, 200, 100, 255),
            LogLevel::Error => Color::new(255, 100, 100, 255),
            LogLevel::Debug => Color::new(180, 180, 180, 255),
        }
    }
    
    pub fn prefix(&self) -> &'static str {
        match self {
            LogLevel::Log => "",
            LogLevel::Info => "ℹ ",
            LogLevel::Warn => "⚠ ",
            LogLevel::Error => "✗ ",
            LogLevel::Debug => "🔍 ",
        }
    }
}

/// A console log entry
#[derive(Debug, Clone)]
pub struct ConsoleEntry {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub source: Option<String>,  // File:line
    pub stack_trace: Option<String>,
}

impl ConsoleEntry {
    pub fn new(level: LogLevel, message: String) -> Self {
        ConsoleEntry {
            level,
            message,
            timestamp: chrono::Local::now(),
            source: None,
            stack_trace: None,
        }
    }
    
    pub fn with_source(mut self, source: String) -> Self {
        self.source = Some(source);
        self
    }
}

/// Network request entry
#[derive(Debug, Clone)]
pub struct NetworkEntry {
    pub id: u64,
    pub method: String,
    pub url: String,
    pub status: Option<u16>,
    pub status_text: Option<String>,
    pub request_headers: Vec<(String, String)>,
    pub response_headers: Vec<(String, String)>,
    pub request_body: Option<String>,
    pub response_body: Option<String>,
    pub content_type: Option<String>,
    pub content_length: Option<usize>,
    pub start_time: chrono::DateTime<chrono::Local>,
    pub end_time: Option<chrono::DateTime<chrono::Local>>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
    // Waterfall timing (milliseconds from start)
    pub waterfall: WaterfallTiming,
}

/// Waterfall timing data for network request visualization
#[derive(Debug, Clone, Default)]
pub struct WaterfallTiming {
    /// Queued/Blocked time (ms)
    pub queued_ms: f64,
    /// DNS lookup time (ms)
    pub dns_ms: f64,
    /// TCP connect time (ms) 
    pub connect_ms: f64,
    /// TLS/SSL handshake time (ms)
    pub ssl_ms: f64,
    /// Time to first byte / server waiting (ms)
    pub ttfb_ms: f64,
    /// Content download time (ms)
    pub download_ms: f64,
}

impl WaterfallTiming {
    pub fn total_ms(&self) -> f64 {
        self.queued_ms + self.dns_ms + self.connect_ms + self.ssl_ms + self.ttfb_ms + self.download_ms
    }
    
    /// Get waterfall bar segments for rendering
    /// Returns: (phase_name, start_ms, duration_ms, color)
    pub fn segments(&self) -> Vec<(&'static str, f64, f64, Color)> {
        let mut segments = Vec::new();
        let mut offset = 0.0;
        
        if self.queued_ms > 0.0 {
            segments.push(("Queued", offset, self.queued_ms, Color::new(128, 128, 128, 255)));
            offset += self.queued_ms;
        }
        if self.dns_ms > 0.0 {
            segments.push(("DNS", offset, self.dns_ms, Color::new(100, 200, 100, 255)));
            offset += self.dns_ms;
        }
        if self.connect_ms > 0.0 {
            segments.push(("Connect", offset, self.connect_ms, Color::new(255, 165, 0, 255)));
            offset += self.connect_ms;
        }
        if self.ssl_ms > 0.0 {
            segments.push(("SSL", offset, self.ssl_ms, Color::new(200, 100, 200, 255)));
            offset += self.ssl_ms;
        }
        if self.ttfb_ms > 0.0 {
            segments.push(("TTFB", offset, self.ttfb_ms, Color::new(100, 180, 255, 255)));
            offset += self.ttfb_ms;
        }
        if self.download_ms > 0.0 {
            segments.push(("Download", offset, self.download_ms, Color::new(100, 100, 255, 255)));
        }
        
        segments
    }
}

impl NetworkEntry {
    pub fn new(id: u64, method: &str, url: &str) -> Self {
        NetworkEntry {
            id,
            method: method.to_string(),
            url: url.to_string(),
            status: None,
            status_text: None,
            request_headers: Vec::new(),
            response_headers: Vec::new(),
            request_body: None,
            response_body: None,
            content_type: None,
            content_length: None,
            start_time: chrono::Local::now(),
            end_time: None,
            duration_ms: None,
            error: None,
            waterfall: WaterfallTiming::default(),
        }
    }
    
    pub fn complete(&mut self, status: u16, status_text: &str) {
        self.status = Some(status);
        self.status_text = Some(status_text.to_string());
        self.end_time = Some(chrono::Local::now());
        self.duration_ms = Some(
            self.end_time.unwrap()
                .signed_duration_since(self.start_time)
                .num_milliseconds() as u64
        );
    }
    
    pub fn status_color(&self) -> Color {
        match self.status {
            Some(s) if s >= 200 && s < 300 => Color::new(100, 200, 100, 255),
            Some(s) if s >= 300 && s < 400 => Color::new(100, 180, 255, 255),
            Some(s) if s >= 400 && s < 500 => Color::new(255, 200, 100, 255),
            Some(s) if s >= 500 => Color::new(255, 100, 100, 255),
            None => Color::new(180, 180, 180, 255),
            _ => Color::new(220, 220, 220, 255),
        }
    }
    
    /// Update waterfall timing from network bar request timing
    pub fn update_waterfall(&mut self, timing: &crate::ui::network_bar::RequestTiming, _started: std::time::Instant) {
        // Calculate each phase duration
        if let Some(dns) = timing.dns_duration_ms() {
            self.waterfall.dns_ms = dns;
        }
        if let Some(connect) = timing.connect_duration_ms() {
            self.waterfall.connect_ms = connect;
        }
        if let Some(wait) = timing.wait_duration_ms() {
            self.waterfall.ttfb_ms = wait;
        }
        if let Some(download) = timing.receive_duration_ms() {
            self.waterfall.download_ms = download;
        }
    }
}

/// Console panel type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConsolePanel {
    Console,
    Network,
    Elements,
    Sources,
    Application,
}

/// Developer console state
pub struct DevConsole {
    pub visible: bool,
    pub height: u32,  // Height when open
    pub active_panel: ConsolePanel,
    
    // Console log
    pub console_entries: VecDeque<ConsoleEntry>,
    pub max_console_entries: usize,
    
    // Network log
    pub network_entries: VecDeque<NetworkEntry>,
    pub max_network_entries: usize,
    pub next_request_id: u64,
    pub selected_network_entry: Option<u64>,
    
    // Command input
    pub input_buffer: String,
    pub input_cursor: usize,
    pub command_history: Vec<String>,
    pub history_index: Option<usize>,
    
    // Filters
    pub console_filter: String,
    pub network_filter: String,
    pub show_log: bool,
    pub show_info: bool,
    pub show_warn: bool,
    pub show_error: bool,
    
    // Elements inspector
    pub inspector: ElementInspector,
    
    // Syntax highlighter for code display
    highlighter: SyntaxHighlighter,
}

/// Element inspector for CSS debugging
#[derive(Debug, Clone, Default)]
pub struct ElementInspector {
    /// Currently selected element (by path from root)
    pub selected_path: Vec<usize>,
    /// Computed styles of selected element
    pub computed_styles: Vec<(String, String)>,
    /// Matched CSS rules for selected element
    pub matched_rules: Vec<CssRuleMatch>,
    /// Box model dimensions
    pub box_model: BoxModel,
    /// Is inspector mode active (click to select)
    pub pick_mode: bool,
    /// Hovered element path
    pub hovered_path: Vec<usize>,
}

/// A matched CSS rule
#[derive(Debug, Clone)]
pub struct CssRuleMatch {
    pub selector: String,
    pub source: String,  // e.g. "style.css:42"
    pub properties: Vec<(String, String, bool)>, // name, value, is_overridden
}

/// Box model for element inspector
#[derive(Debug, Clone, Default)]
pub struct BoxModel {
    pub margin: EdgeBox,
    pub border: EdgeBox,
    pub padding: EdgeBox,
    pub content: ContentBox,
}

#[derive(Debug, Clone, Default)]
pub struct EdgeBox {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

#[derive(Debug, Clone, Default)]
pub struct ContentBox {
    pub width: f32,
    pub height: f32,
}

impl ElementInspector {
    /// Toggle element picker mode
    pub fn toggle_pick_mode(&mut self) {
        self.pick_mode = !self.pick_mode;
        if !self.pick_mode {
            self.hovered_path.clear();
        }
    }
    
    /// Select an element by path
    pub fn select_element(&mut self, path: Vec<usize>) {
        self.selected_path = path;
        self.pick_mode = false;
    }
    
    /// Update computed styles from a ComputedStyle
    pub fn update_from_computed(&mut self, style: &crate::style::ComputedStyle) {
        self.computed_styles.clear();
        
        // Extract key style properties
        self.computed_styles.push(("display".to_string(), format!("{:?}", style.display)));
        self.computed_styles.push(("position".to_string(), format!("{:?}", style.position)));
        self.computed_styles.push(("width".to_string(), format!("{:?}", style.width)));
        self.computed_styles.push(("height".to_string(), format!("{:?}", style.height)));
        self.computed_styles.push(("color".to_string(), 
            format!("rgba({}, {}, {}, {})", style.color.r, style.color.g, style.color.b, style.color.a)));
        self.computed_styles.push(("background-color".to_string(),
            format!("rgba({}, {}, {}, {})", style.background_color.r, style.background_color.g, 
                    style.background_color.b, style.background_color.a)));
        self.computed_styles.push(("font-size".to_string(), format!("{}px", style.font_size)));
        self.computed_styles.push(("font-weight".to_string(), format!("{}", style.font_weight)));
        self.computed_styles.push(("font-family".to_string(), style.font_family.clone()));
        self.computed_styles.push(("margin".to_string(), 
            format!("{} {} {} {}", style.margin.top, style.margin.right, style.margin.bottom, style.margin.left)));
        self.computed_styles.push(("padding".to_string(),
            format!("{} {} {} {}", style.padding.top, style.padding.right, style.padding.bottom, style.padding.left)));
        self.computed_styles.push(("border-width".to_string(),
            format!("{} {} {} {}", style.border.top, style.border.right, style.border.bottom, style.border.left)));
        self.computed_styles.push(("flex-direction".to_string(), format!("{:?}", style.flex_direction)));
        self.computed_styles.push(("opacity".to_string(), format!("{}", style.opacity)));
        
        // Update box model
        self.box_model.margin = EdgeBox {
            top: style.margin.top,
            right: style.margin.right,
            bottom: style.margin.bottom,
            left: style.margin.left,
        };
        self.box_model.padding = EdgeBox {
            top: style.padding.top,
            right: style.padding.right,
            bottom: style.padding.bottom,
            left: style.padding.left,
        };
        self.box_model.border = EdgeBox {
            top: style.border.top,
            right: style.border.right,
            bottom: style.border.bottom,
            left: style.border.left,
        };
    }
    
    /// Update content box dimensions from layout
    pub fn set_content_size(&mut self, width: f32, height: f32) {
        self.box_model.content.width = width;
        self.box_model.content.height = height;
    }
    
    /// Clear selection
    pub fn clear(&mut self) {
        self.selected_path.clear();
        self.computed_styles.clear();
        self.matched_rules.clear();
        self.box_model = BoxModel::default();
        self.pick_mode = false;
        self.hovered_path.clear();
    }
}

impl DevConsole {
    pub fn new() -> Self {
        DevConsole {
            visible: false,
            height: 300,
            active_panel: ConsolePanel::Console,
            console_entries: VecDeque::new(),
            max_console_entries: 1000,
            network_entries: VecDeque::new(),
            max_network_entries: 500,
            next_request_id: 1,
            selected_network_entry: None,
            input_buffer: String::new(),
            input_cursor: 0,
            command_history: Vec::new(),
            history_index: None,
            console_filter: String::new(),
            network_filter: String::new(),
            show_log: true,
            show_info: true,
            show_warn: true,
            show_error: true,
            inspector: ElementInspector::default(),
            highlighter: SyntaxHighlighter::new(),
        }
    }
    
    /// Toggle visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }
    
    /// Log a message
    pub fn log(&mut self, level: LogLevel, message: String) {
        let entry = ConsoleEntry::new(level, message);
        self.console_entries.push_back(entry);
        
        while self.console_entries.len() > self.max_console_entries {
            self.console_entries.pop_front();
        }
    }
    
    /// Log with source location
    pub fn log_with_source(&mut self, level: LogLevel, message: String, source: String) {
        let entry = ConsoleEntry::new(level, message).with_source(source);
        self.console_entries.push_back(entry);
        
        while self.console_entries.len() > self.max_console_entries {
            self.console_entries.pop_front();
        }
    }
    
    /// Start tracking a network request
    pub fn start_request(&mut self, method: &str, url: &str) -> u64 {
        let id = self.next_request_id;
        self.next_request_id += 1;
        
        let entry = NetworkEntry::new(id, method, url);
        self.network_entries.push_back(entry);
        
        while self.network_entries.len() > self.max_network_entries {
            self.network_entries.pop_front();
        }
        
        id
    }
    
    /// Complete a network request
    pub fn complete_request(&mut self, id: u64, status: u16, status_text: &str) {
        if let Some(entry) = self.network_entries.iter_mut().find(|e| e.id == id) {
            entry.complete(status, status_text);
        }
    }
    
    /// Mark a request as failed
    pub fn fail_request(&mut self, id: u64, error: &str) {
        if let Some(entry) = self.network_entries.iter_mut().find(|e| e.id == id) {
            entry.error = Some(error.to_string());
            entry.end_time = Some(chrono::Local::now());
        }
    }
    
    /// Execute a console command
    pub fn execute_command(&mut self, js_executor: impl FnOnce(&str) -> Result<String, String>) {
        let command = self.input_buffer.clone();
        if command.is_empty() {
            return;
        }
        
        // Add to history
        self.command_history.push(command.clone());
        self.history_index = None;
        
        // Log the command
        self.log(LogLevel::Log, format!("> {}", command));
        
        // Execute
        match js_executor(&command) {
            Ok(result) => {
                if !result.is_empty() {
                    self.log(LogLevel::Log, result);
                }
            }
            Err(err) => {
                self.log(LogLevel::Error, err);
            }
        }
        
        // Clear input
        self.input_buffer.clear();
        self.input_cursor = 0;
    }
    
    /// Handle keyboard input
    pub fn handle_key(&mut self, key: &str, ctrl: bool) {
        match key {
            "Backspace" => {
                if self.input_cursor > 0 {
                    self.input_buffer.remove(self.input_cursor - 1);
                    self.input_cursor -= 1;
                }
            }
            "Delete" => {
                if self.input_cursor < self.input_buffer.len() {
                    self.input_buffer.remove(self.input_cursor);
                }
            }
            "Left" => {
                if self.input_cursor > 0 {
                    self.input_cursor -= 1;
                }
            }
            "Right" => {
                if self.input_cursor < self.input_buffer.len() {
                    self.input_cursor += 1;
                }
            }
            "Home" => {
                self.input_cursor = 0;
            }
            "End" => {
                self.input_cursor = self.input_buffer.len();
            }
            "Up" => {
                // History navigation
                if !self.command_history.is_empty() {
                    match self.history_index {
                        None => {
                            self.history_index = Some(self.command_history.len() - 1);
                        }
                        Some(idx) if idx > 0 => {
                            self.history_index = Some(idx - 1);
                        }
                        _ => {}
                    }
                    if let Some(idx) = self.history_index {
                        self.input_buffer = self.command_history[idx].clone();
                        self.input_cursor = self.input_buffer.len();
                    }
                }
            }
            "Down" => {
                if let Some(idx) = self.history_index {
                    if idx + 1 < self.command_history.len() {
                        self.history_index = Some(idx + 1);
                        self.input_buffer = self.command_history[idx + 1].clone();
                        self.input_cursor = self.input_buffer.len();
                    } else {
                        self.history_index = None;
                        self.input_buffer.clear();
                        self.input_cursor = 0;
                    }
                }
            }
            _ if ctrl && key == "l" => {
                // Clear console
                self.console_entries.clear();
            }
            _ if ctrl && key == "k" => {
                // Clear network
                self.network_entries.clear();
            }
            _ if key.len() == 1 => {
                // Insert character
                self.input_buffer.insert(self.input_cursor, key.chars().next().unwrap());
                self.input_cursor += 1;
            }
            _ => {}
        }
    }
    
    /// Get filtered console entries
    pub fn filtered_console_entries(&self) -> Vec<&ConsoleEntry> {
        self.console_entries.iter().filter(|e| {
            // Filter by level
            match e.level {
                LogLevel::Log => self.show_log,
                LogLevel::Info => self.show_info,
                LogLevel::Warn => self.show_warn,
                LogLevel::Error => self.show_error,
                LogLevel::Debug => self.show_log,
            }
        }).filter(|e| {
            // Filter by text
            if self.console_filter.is_empty() {
                true
            } else {
                e.message.to_lowercase().contains(&self.console_filter.to_lowercase())
            }
        }).collect()
    }
    
    /// Get filtered network entries
    pub fn filtered_network_entries(&self) -> Vec<&NetworkEntry> {
        self.network_entries.iter().filter(|e| {
            if self.network_filter.is_empty() {
                true
            } else {
                e.url.to_lowercase().contains(&self.network_filter.to_lowercase()) ||
                e.method.to_lowercase().contains(&self.network_filter.to_lowercase())
            }
        }).collect()
    }
    
    /// Highlight JavaScript code for display
    pub fn highlight_js(&self, code: &str) -> Vec<Vec<crate::syntax::HighlightToken>> {
        self.highlighter.highlight(code, Language::JavaScript)
    }
    
    /// Clear all entries
    pub fn clear(&mut self) {
        self.console_entries.clear();
        self.network_entries.clear();
    }
}

impl Default for DevConsole {
    fn default() -> Self {
        Self::new()
    }
}

// Global console instance
lazy_static::lazy_static! {
    pub static ref CONSOLE: std::sync::Mutex<DevConsole> = std::sync::Mutex::new(DevConsole::new());
}

/// Convenience functions for logging
pub fn console_log(message: &str) {
    if let Ok(mut console) = CONSOLE.lock() {
        console.log(LogLevel::Log, message.to_string());
    }
}

pub fn console_info(message: &str) {
    if let Ok(mut console) = CONSOLE.lock() {
        console.log(LogLevel::Info, message.to_string());
    }
}

pub fn console_warn(message: &str) {
    if let Ok(mut console) = CONSOLE.lock() {
        console.log(LogLevel::Warn, message.to_string());
    }
}

pub fn console_error(message: &str) {
    if let Ok(mut console) = CONSOLE.lock() {
        console.log(LogLevel::Error, message.to_string());
    }
}

pub fn console_debug(message: &str) {
    if let Ok(mut console) = CONSOLE.lock() {
        console.log(LogLevel::Debug, message.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_console_log() {
        let mut console = DevConsole::new();
        console.log(LogLevel::Log, "test".to_string());
        assert_eq!(console.console_entries.len(), 1);
    }
    
    #[test]
    fn test_network_tracking() {
        let mut console = DevConsole::new();
        let id = console.start_request("GET", "https://example.com");
        console.complete_request(id, 200, "OK");
        
        assert_eq!(console.network_entries.len(), 1);
        assert_eq!(console.network_entries[0].status, Some(200));
    }
}
