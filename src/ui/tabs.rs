//! Tab tile system - visual grid of tab previews
//! Like Windows alt+tab but for browser tabs

#![allow(dead_code)]
#![allow(unused_variables)]

use std::time::{Duration, Instant};
use std::collections::VecDeque;

/// Tab content type
#[derive(Debug, Clone, PartialEq)]
pub enum TabContent {
    /// Standard web page
    WebPage,
    /// Embedded terminal
    Terminal,
    /// PDF viewer
    Pdf,
    /// Settings page
    Settings,
}

impl Default for TabContent {
    fn default() -> Self {
        TabContent::WebPage
    }
}

/// Embedded terminal state
#[derive(Debug, Clone)]
pub struct TerminalState {
    /// Terminal output buffer (lines)
    pub output: VecDeque<TerminalLine>,
    /// Maximum lines to keep
    pub max_lines: usize,
    /// Current input line
    pub input_buffer: String,
    /// Cursor position in input
    pub cursor_pos: usize,
    /// Command history
    pub history: Vec<String>,
    /// History navigation index
    pub history_index: Option<usize>,
    /// Current working directory
    pub cwd: String,
    /// Is a command currently running?
    pub running: bool,
    /// Process exit code of last command
    pub last_exit_code: Option<i32>,
    /// Shell environment
    pub env: std::collections::HashMap<String, String>,
}

/// A line in the terminal output
#[derive(Debug, Clone)]
pub struct TerminalLine {
    pub text: String,
    pub style: TerminalStyle,
}

/// Terminal text styling
#[derive(Debug, Clone, Copy, Default)]
pub struct TerminalStyle {
    pub fg_color: TerminalColor,
    pub bg_color: TerminalColor,
    pub bold: bool,
    pub dim: bool,
    pub underline: bool,
}

/// Terminal colors (ANSI 16 color palette)
#[derive(Debug, Clone, Copy, Default)]
pub enum TerminalColor {
    #[default]
    Default,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

impl Default for TerminalState {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalState {
    pub fn new() -> Self {
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "~".to_string());
        
        let mut output = VecDeque::new();
        output.push_back(TerminalLine {
            text: format!("Sassy Terminal - {}", cwd),
            style: TerminalStyle {
                fg_color: TerminalColor::Cyan,
                bold: true,
                ..Default::default()
            },
        });
        output.push_back(TerminalLine {
            text: String::new(),
            style: TerminalStyle::default(),
        });
        
        Self {
            output,
            max_lines: 10000,
            input_buffer: String::new(),
            cursor_pos: 0,
            history: Vec::new(),
            history_index: None,
            cwd,
            running: false,
            last_exit_code: None,
            env: std::env::vars().collect(),
        }
    }
    
    /// Get the shell prompt
    pub fn prompt(&self) -> String {
        let exit_indicator = match self.last_exit_code {
            Some(0) | None => "❯",
            Some(_) => "✗",
        };
        format!("{} {} ", self.short_cwd(), exit_indicator)
    }
    
    fn short_cwd(&self) -> String {
        // Shorten the path like fish shell
        if let Some(home) = self.env.get("HOME").or_else(|| self.env.get("USERPROFILE")) {
            if self.cwd.starts_with(home) {
                return format!("~{}", &self.cwd[home.len()..]);
            }
        }
        self.cwd.clone()
    }
    
    /// Add a line to output
    pub fn print(&mut self, text: &str, style: TerminalStyle) {
        self.output.push_back(TerminalLine {
            text: text.to_string(),
            style,
        });
        while self.output.len() > self.max_lines {
            self.output.pop_front();
        }
    }
    
    /// Add output with default style
    pub fn println(&mut self, text: &str) {
        self.print(text, TerminalStyle::default());
    }
    
    /// Print error output
    pub fn print_error(&mut self, text: &str) {
        self.print(text, TerminalStyle {
            fg_color: TerminalColor::Red,
            ..Default::default()
        });
    }
    
    /// Execute a command and capture output
    pub fn execute(&mut self, command: &str) {
        // Add to history
        if !command.trim().is_empty() {
            self.history.push(command.to_string());
        }
        self.history_index = None;
        
        // Print the command
        self.print(&format!("{}{}", self.prompt(), command), TerminalStyle {
            fg_color: TerminalColor::Green,
            ..Default::default()
        });
        
        // Handle built-in commands
        let parts: Vec<&str> = command.trim().split_whitespace().collect();
        if parts.is_empty() {
            return;
        }
        
        match parts[0] {
            "cd" => {
                let target = parts.get(1).map(|s| *s).unwrap_or("~");
                let target = if target == "~" {
                    self.env.get("HOME").or_else(|| self.env.get("USERPROFILE"))
                        .map(|s| s.as_str())
                        .unwrap_or(".")
                } else {
                    target
                };
                
                match std::env::set_current_dir(target) {
                    Ok(_) => {
                        self.cwd = std::env::current_dir()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or(target.to_string());
                        self.last_exit_code = Some(0);
                    }
                    Err(e) => {
                        self.print_error(&format!("cd: {}: {}", target, e));
                        self.last_exit_code = Some(1);
                    }
                }
            }
            "clear" => {
                self.output.clear();
                self.last_exit_code = Some(0);
            }
            "exit" => {
                // Would close the terminal tab
                self.println("Use Ctrl+W to close the terminal tab");
                self.last_exit_code = Some(0);
            }
            "pwd" => {
                self.println(&self.cwd.clone());
                self.last_exit_code = Some(0);
            }
            "env" | "export" if parts.len() == 1 => {
                let env_lines: Vec<String> = self.env.iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect();
                for line in env_lines {
                    self.println(&line);
                }
                self.last_exit_code = Some(0);
            }
            "help" => {
                self.println("Sassy Terminal - Built-in commands:");
                self.println("  cd <dir>    - Change directory");
                self.println("  clear       - Clear screen");
                self.println("  pwd         - Print working directory");
                self.println("  env         - Show environment variables");
                self.println("  help        - Show this help");
                self.println("");
                self.println("External commands are executed via the system shell.");
                self.last_exit_code = Some(0);
            }
            _ => {
                // Execute external command
                self.run_external_command(command);
            }
        }
    }
    
    fn run_external_command(&mut self, command: &str) {
        #[cfg(target_os = "windows")]
        let shell = ("cmd", "/C");
        
        #[cfg(not(target_os = "windows"))]
        let shell = ("sh", "-c");
        
        self.running = true;
        
        match std::process::Command::new(shell.0)
            .arg(shell.1)
            .arg(command)
            .current_dir(&self.cwd)
            .envs(&self.env)
            .output()
        {
            Ok(output) => {
                // Print stdout
                if !output.stdout.is_empty() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    for line in stdout.lines() {
                        self.println(line);
                    }
                }
                
                // Print stderr
                if !output.stderr.is_empty() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    for line in stderr.lines() {
                        self.print_error(line);
                    }
                }
                
                self.last_exit_code = output.status.code();
            }
            Err(e) => {
                self.print_error(&format!("Failed to execute: {}", e));
                self.last_exit_code = Some(127);
            }
        }
        
        self.running = false;
    }
    
    /// Handle input character
    pub fn input_char(&mut self, c: char) {
        self.input_buffer.insert(self.cursor_pos, c);
        self.cursor_pos += 1;
    }
    
    /// Handle backspace
    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.input_buffer.remove(self.cursor_pos);
        }
    }
    
    /// Handle delete
    pub fn delete(&mut self) {
        if self.cursor_pos < self.input_buffer.len() {
            self.input_buffer.remove(self.cursor_pos);
        }
    }
    
    /// Submit current input
    pub fn submit(&mut self) {
        let command = std::mem::take(&mut self.input_buffer);
        self.cursor_pos = 0;
        self.execute(&command);
    }
    
    /// Navigate history up
    pub fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        
        self.history_index = Some(match self.history_index {
            None => self.history.len() - 1,
            Some(i) if i > 0 => i - 1,
            Some(i) => i,
        });
        
        if let Some(idx) = self.history_index {
            if let Some(cmd) = self.history.get(idx) {
                self.input_buffer = cmd.clone();
                self.cursor_pos = self.input_buffer.len();
            }
        }
    }
    
    /// Navigate history down
    pub fn history_down(&mut self) {
        if let Some(idx) = self.history_index {
            if idx + 1 < self.history.len() {
                self.history_index = Some(idx + 1);
                self.input_buffer = self.history[idx + 1].clone();
                self.cursor_pos = self.input_buffer.len();
            } else {
                self.history_index = None;
                self.input_buffer.clear();
                self.cursor_pos = 0;
            }
        }
    }
    
    /// Move cursor left
    pub fn cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }
    
    /// Move cursor right
    pub fn cursor_right(&mut self) {
        if self.cursor_pos < self.input_buffer.len() {
            self.cursor_pos += 1;
        }
    }
    
    /// Move cursor to start
    pub fn cursor_home(&mut self) {
        self.cursor_pos = 0;
    }
    
    /// Move cursor to end
    pub fn cursor_end(&mut self) {
        self.cursor_pos = self.input_buffer.len();
    }
}

/// Meaningful interactions that build trust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MeaningfulInteraction {
    /// User scrolled significantly (not just a pixel)
    Scroll { distance: i32 },
    /// User typed into a form field
    FormInput,
    /// User clicked a link and navigated
    LinkClick,
    /// User submitted a form
    FormSubmit,
    /// User spent significant time (30+ seconds focused)
    TimeSpent,
    /// User right-clicked (context menu)
    ContextMenu,
    /// User used keyboard shortcut
    KeyboardShortcut,
    /// User played media
    MediaPlay,
    /// User interacted with video controls
    VideoInteraction,
    /// User selected text
    TextSelection,
}

/// Sandbox trust level for a tab
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustLevel {
    /// Brand new page, fully sandboxed
    Untrusted,
    /// Some interactions, partial trust
    Building,
    /// 3+ meaningful interactions, trusted
    Trusted,
    /// Explicitly trusted by user (whitelisted domain)
    Whitelisted,
}

/// Per-tab sandbox state
#[derive(Debug, Clone)]
pub struct TabSandbox {
    /// Current trust level
    pub trust_level: TrustLevel,
    
    /// Interactions recorded for this page
    pub interactions: Vec<(MeaningfulInteraction, Instant)>,
    
    /// When the page was loaded (for time-based trust)
    pub page_loaded: Instant,
    
    /// Cumulative scroll distance (must be 500+ px to count)
    pub scroll_accumulated: i32,
    
    /// Time spent with tab focused
    pub focus_time: Duration,
    
    /// Last focus start
    pub focus_start: Option<Instant>,
    
    /// Has the user been warned about this page?
    pub warning_shown: bool,
    
    /// Restrictions currently in effect
    pub restrictions: SandboxRestrictions,
}

/// What the sandbox prevents
#[derive(Debug, Clone, Copy)]
pub struct SandboxRestrictions {
    /// Block clipboard access
    pub block_clipboard: bool,
    /// Block download initiation
    pub block_downloads: bool,
    /// Block popups
    pub block_popups: bool,
    /// Block form auto-submit
    pub block_auto_submit: bool,
    /// Block notifications
    pub block_notifications: bool,
    /// Block camera/microphone
    pub block_media_devices: bool,
    /// Block geolocation
    pub block_geolocation: bool,
    /// Block fullscreen requests
    pub block_fullscreen: bool,
    /// Limit JS execution time
    pub limit_js_time: bool,
    /// Block external protocol handlers
    pub block_protocol_handlers: bool,
}

impl TabSandbox {
    pub fn new() -> Self {
        Self {
            trust_level: TrustLevel::Untrusted,
            interactions: Vec::new(),
            page_loaded: Instant::now(),
            scroll_accumulated: 0,
            focus_time: Duration::ZERO,
            focus_start: None,
            warning_shown: false,
            restrictions: SandboxRestrictions::strict(),
        }
    }
    
    pub fn whitelisted() -> Self {
        Self {
            trust_level: TrustLevel::Whitelisted,
            interactions: Vec::new(),
            page_loaded: Instant::now(),
            scroll_accumulated: 0,
            focus_time: Duration::ZERO,
            focus_start: None,
            warning_shown: false,
            restrictions: SandboxRestrictions::none(),
        }
    }
    
    /// Record an interaction and update trust level
    pub fn record_interaction(&mut self, interaction: MeaningfulInteraction) {
        // Don't record if already trusted
        if matches!(self.trust_level, TrustLevel::Trusted | TrustLevel::Whitelisted) {
            return;
        }
        
        // Check if this is a meaningful new interaction
        let dominated = self.interactions.iter().any(|(prev, _)| {
            std::mem::discriminant(prev) == std::mem::discriminant(&interaction)
        });
        
        // For scroll, accumulate distance
        if let MeaningfulInteraction::Scroll { distance } = interaction {
            self.scroll_accumulated += distance.abs();
            
            // Only count as interaction if scrolled 500+ pixels total
            if self.scroll_accumulated >= 500 && !dominated {
                self.interactions.push((interaction, Instant::now()));
            }
        } else if !dominated {
            self.interactions.push((interaction, Instant::now()));
        }
        
        // Update trust level
        self.update_trust_level();
    }
    
    /// Call when tab gains focus
    pub fn focus_gained(&mut self) {
        self.focus_start = Some(Instant::now());
    }
    
    /// Call when tab loses focus
    pub fn focus_lost(&mut self) {
        if let Some(start) = self.focus_start.take() {
            self.focus_time += start.elapsed();
            
            // Time spent can count as interaction
            if self.focus_time >= Duration::from_secs(30) {
                self.record_interaction(MeaningfulInteraction::TimeSpent);
            }
        }
    }
    
    /// Update trust level based on interactions
    fn update_trust_level(&mut self) {
        let unique_interactions = self.interactions.len();
        
        self.trust_level = if unique_interactions >= 3 {
            TrustLevel::Trusted
        } else if unique_interactions >= 1 {
            TrustLevel::Building
        } else {
            TrustLevel::Untrusted
        };
        
        // Update restrictions based on trust
        self.restrictions = match self.trust_level {
            TrustLevel::Untrusted => SandboxRestrictions::strict(),
            TrustLevel::Building => SandboxRestrictions::moderate(),
            TrustLevel::Trusted => SandboxRestrictions::relaxed(),
            TrustLevel::Whitelisted => SandboxRestrictions::none(),
        };
    }
    
    /// Reset sandbox (for navigation)
    pub fn reset(&mut self) {
        *self = Self::new();
    }
    
    /// Get interaction count
    pub fn interaction_count(&self) -> usize {
        self.interactions.len()
    }
    
    /// Interactions needed for trust
    pub fn interactions_needed(&self) -> usize {
        3usize.saturating_sub(self.interactions.len())
    }
    
    /// Is page trusted enough for action?
    pub fn can_perform(&self, action: SandboxAction) -> bool {
        match action {
            SandboxAction::Clipboard => !self.restrictions.block_clipboard,
            SandboxAction::Download => !self.restrictions.block_downloads,
            SandboxAction::Popup => !self.restrictions.block_popups,
            SandboxAction::Notification => !self.restrictions.block_notifications,
            SandboxAction::MediaDevice => !self.restrictions.block_media_devices,
            SandboxAction::Geolocation => !self.restrictions.block_geolocation,
            SandboxAction::Fullscreen => !self.restrictions.block_fullscreen,
            SandboxAction::ProtocolHandler => !self.restrictions.block_protocol_handlers,
        }
    }
}

impl Default for TabSandbox {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SandboxAction {
    Clipboard,
    Download,
    Popup,
    Notification,
    MediaDevice,
    Geolocation,
    Fullscreen,
    ProtocolHandler,
}

impl SandboxRestrictions {
    /// All restrictions enabled (untrusted)
    pub fn strict() -> Self {
        Self {
            block_clipboard: true,
            block_downloads: true,
            block_popups: true,
            block_auto_submit: true,
            block_notifications: true,
            block_media_devices: true,
            block_geolocation: true,
            block_fullscreen: true,
            limit_js_time: true,
            block_protocol_handlers: true,
        }
    }
    
    /// Some restrictions (building trust)
    pub fn moderate() -> Self {
        Self {
            block_clipboard: true,
            block_downloads: true,
            block_popups: true,
            block_auto_submit: true,
            block_notifications: true,
            block_media_devices: true,
            block_geolocation: true,
            block_fullscreen: false, // Allow fullscreen after 1 interaction
            limit_js_time: false,
            block_protocol_handlers: true,
        }
    }
    
    /// Minimal restrictions (trusted)
    pub fn relaxed() -> Self {
        Self {
            block_clipboard: false,
            block_downloads: false,
            block_popups: false, // Smart popup blocker takes over
            block_auto_submit: false,
            block_notifications: false,
            block_media_devices: false,
            block_geolocation: false, // Still prompts user
            block_fullscreen: false,
            limit_js_time: false,
            block_protocol_handlers: false,
        }
    }
    
    /// No restrictions (whitelisted)
    pub fn none() -> Self {
        Self {
            block_clipboard: false,
            block_downloads: false,
            block_popups: false,
            block_auto_submit: false,
            block_notifications: false,
            block_media_devices: false,
            block_geolocation: false,
            block_fullscreen: false,
            limit_js_time: false,
            block_protocol_handlers: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Tab {
    pub id: u64,
    pub title: String,
    pub url: String,
    pub favicon: Option<Vec<u8>>,
    pub preview: Option<TabPreview>,
    pub loading: bool,
    pub pinned: bool,
    pub muted: bool,
    pub audible: bool,
    pub created_at: Instant,
    pub last_accessed: Instant,
    pub group_id: Option<u64>,
    
    /// Per-page sandbox state
    pub sandbox: TabSandbox,
    
    /// Tab content type
    pub content_type: TabContent,
    
    /// Terminal state (if content_type is Terminal)
    pub terminal: Option<TerminalState>,
}

#[derive(Debug, Clone)]
pub struct TabPreview {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u32>,  // RGBA pixels
    pub captured_at: Instant,
}

impl TabPreview {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: vec![0; (width * height) as usize],
            captured_at: Instant::now(),
        }
    }
    
    pub fn from_buffer(buffer: &[u32], src_width: u32, src_height: u32, target_width: u32, target_height: u32) -> Self {
        // Downsample the buffer to preview size
        let mut preview = Self::new(target_width, target_height);
        
        let scale_x = src_width as f32 / target_width as f32;
        let scale_y = src_height as f32 / target_height as f32;
        
        for y in 0..target_height {
            for x in 0..target_width {
                let src_x = (x as f32 * scale_x) as u32;
                let src_y = (y as f32 * scale_y) as u32;
                let src_idx = (src_y * src_width + src_x) as usize;
                let dst_idx = (y * target_width + x) as usize;
                
                if src_idx < buffer.len() {
                    preview.data[dst_idx] = buffer[src_idx];
                }
            }
        }
        
        preview.captured_at = Instant::now();
        preview
    }
    
    pub fn is_stale(&self, max_age: Duration) -> bool {
        self.captured_at.elapsed() > max_age
    }
}

impl Tab {
    pub fn new(id: u64, url: String) -> Self {
        let now = Instant::now();
        Self {
            id,
            title: url.clone(),
            url,
            favicon: None,
            preview: None,
            loading: false,
            pinned: false,
            muted: false,
            audible: false,
            created_at: now,
            last_accessed: now,
            group_id: None,
            sandbox: TabSandbox::new(),
            content_type: TabContent::WebPage,
            terminal: None,
        }
    }
    
    /// Create a new terminal tab
    pub fn new_terminal(id: u64) -> Self {
        let now = Instant::now();
        Self {
            id,
            title: "Terminal".to_string(),
            url: "sassy://terminal".to_string(),
            favicon: None,
            preview: None,
            loading: false,
            pinned: false,
            muted: false,
            audible: false,
            created_at: now,
            last_accessed: now,
            group_id: None,
            sandbox: TabSandbox::new(),
            content_type: TabContent::Terminal,
            terminal: Some(TerminalState::new()),
        }
    }
    
    /// Check if this is a terminal tab
    pub fn is_terminal(&self) -> bool {
        self.content_type == TabContent::Terminal
    }
    
    pub fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }
    
    /// Reset sandbox for new navigation
    pub fn navigate(&mut self, url: &str) {
        self.url = url.to_string();
        self.sandbox.reset();
        self.loading = true;
    }
    
    /// Get trust level text for UI
    pub fn trust_text(&self) -> &'static str {
        match self.sandbox.trust_level {
            TrustLevel::Untrusted => "Sandboxed",
            TrustLevel::Building => "Building Trust",
            TrustLevel::Trusted => "Trusted",
            TrustLevel::Whitelisted => "Whitelisted",
        }
    }
    
    /// Get trust indicator color
    pub fn trust_color(&self) -> u32 {
        match self.sandbox.trust_level {
            TrustLevel::Untrusted => 0xFFff4444,    // Red
            TrustLevel::Building => 0xFFffaa44,     // Orange
            TrustLevel::Trusted => 0xFF44ff44,      // Green
            TrustLevel::Whitelisted => 0xFF44aaff,  // Blue
        }
    }
}

#[derive(Debug, Clone)]
pub struct TabGroup {
    pub id: u64,
    pub name: String,
    pub color: String,
    pub collapsed: bool,
}

/// Tab tile layout configuration
#[derive(Debug, Clone)]
pub struct TileLayout {
    pub columns: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub gap: u32,
    pub padding: u32,
}

impl TileLayout {
    pub fn calculate(
        available_width: u32,
        available_height: u32,
        tab_count: usize,
        min_tile_width: u32,
        max_tile_width: u32,
        aspect_ratio: f32,
        gap: u32,
    ) -> Self {
        // Calculate optimal number of columns
        let padding = gap;
        let usable_width = available_width.saturating_sub(padding * 2);
        
        // Start with max tiles that fit
        let mut columns = ((usable_width + gap) / (min_tile_width + gap)).max(1);
        
        // Reduce columns if we have fewer tabs
        let rows_needed = (tab_count as u32 + columns - 1) / columns;
        if rows_needed == 1 && tab_count > 0 {
            columns = tab_count as u32;
        }
        
        // Calculate tile width based on columns
        let tile_width = ((usable_width + gap) / columns - gap)
            .clamp(min_tile_width, max_tile_width);
        let tile_height = (tile_width as f32 * aspect_ratio) as u32;
        
        Self {
            columns,
            tile_width,
            tile_height,
            gap,
            padding,
        }
    }
    
    pub fn tile_rect(&self, index: usize) -> (u32, u32, u32, u32) {
        let col = index as u32 % self.columns;
        let row = index as u32 / self.columns;
        
        let x = self.padding + col * (self.tile_width + self.gap);
        let y = self.padding + row * (self.tile_height + self.gap);
        
        (x, y, self.tile_width, self.tile_height)
    }
    
    pub fn total_height(&self, tab_count: usize) -> u32 {
        if tab_count == 0 {
            return self.padding * 2;
        }
        
        let rows = (tab_count as u32 + self.columns - 1) / self.columns;
        self.padding * 2 + rows * self.tile_height + (rows - 1) * self.gap
    }
    
    pub fn hit_test(&self, x: u32, y: u32, tab_count: usize) -> Option<usize> {
        if x < self.padding || y < self.padding {
            return None;
        }
        
        let rel_x = x - self.padding;
        let rel_y = y - self.padding;
        
        let col = rel_x / (self.tile_width + self.gap);
        let row = rel_y / (self.tile_height + self.gap);
        
        // Check if actually within a tile (not in the gap)
        let tile_start_x = col * (self.tile_width + self.gap);
        let tile_start_y = row * (self.tile_height + self.gap);
        
        if rel_x < tile_start_x + self.tile_width && rel_y < tile_start_y + self.tile_height {
            let index = (row * self.columns + col) as usize;
            if index < tab_count {
                return Some(index);
            }
        }
        
        None
    }
}

/// Tab manager with tile view support
pub struct TabManager {
    tabs: Vec<Tab>,
    groups: Vec<TabGroup>,
    active_tab: Option<u64>,
    next_id: u64,
    next_group_id: u64,
    
    // View state
    pub tile_view_active: bool,
    pub selected_index: Option<usize>,
    pub search_query: String,
    pub scroll_offset: u32,
    
    // Settings
    pub preview_enabled: bool,
    pub preview_max_age: Duration,
    pub preview_size: (u32, u32),
}

impl TabManager {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            groups: Vec::new(),
            active_tab: None,
            next_id: 1,
            next_group_id: 1,
            tile_view_active: false,
            selected_index: None,
            search_query: String::new(),
            scroll_offset: 0,
            preview_enabled: true,
            preview_max_age: Duration::from_secs(30),
            preview_size: (320, 240),
        }
    }
    
    pub fn create_tab(&mut self, url: String) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        
        let tab = Tab::new(id, url);
        self.tabs.push(tab);
        self.active_tab = Some(id);
        
        id
    }
    
    /// Create a new terminal tab
    pub fn create_terminal_tab(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        
        let tab = Tab::new_terminal(id);
        self.tabs.push(tab);
        self.active_tab = Some(id);
        
        id
    }
    
    pub fn close_tab(&mut self, id: u64) {
        if let Some(idx) = self.tabs.iter().position(|t| t.id == id) {
            self.tabs.remove(idx);
            
            if self.active_tab == Some(id) {
                // Activate nearest tab
                self.active_tab = if idx > 0 {
                    self.tabs.get(idx - 1).map(|t| t.id)
                } else {
                    self.tabs.get(0).map(|t| t.id)
                };
            }
        }
    }
    
    pub fn activate_tab(&mut self, id: u64) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.touch();
            self.active_tab = Some(id);
        }
    }
    
    pub fn get_tab(&self, id: u64) -> Option<&Tab> {
        self.tabs.iter().find(|t| t.id == id)
    }
    
    pub fn get_tab_mut(&mut self, id: u64) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|t| t.id == id)
    }
    
    pub fn active_tab(&self) -> Option<&Tab> {
        self.active_tab.and_then(|id| self.get_tab(id))
    }
    
    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        let id = self.active_tab?;
        self.get_tab_mut(id)
    }
    
    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }
    
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }
    
    /// Get filtered tabs based on search query
    pub fn filtered_tabs(&self) -> Vec<&Tab> {
        if self.search_query.is_empty() {
            self.tabs.iter().collect()
        } else {
            let query = self.search_query.to_lowercase();
            self.tabs.iter()
                .filter(|t| {
                    t.title.to_lowercase().contains(&query) ||
                    t.url.to_lowercase().contains(&query)
                })
                .collect()
        }
    }
    
    /// Update tab preview from current render buffer
    pub fn capture_preview(&mut self, tab_id: u64, buffer: &[u32], width: u32, height: u32) {
        if !self.preview_enabled {
            return;
        }
        let (prev_w, prev_h) = self.preview_size;
        
        if let Some(tab) = self.get_tab_mut(tab_id) {
            tab.preview = Some(TabPreview::from_buffer(
                buffer, width, height,
                prev_w, prev_h
            ));
        }
    }
    
    /// Toggle tile view
    pub fn toggle_tile_view(&mut self) {
        self.tile_view_active = !self.tile_view_active;
        if self.tile_view_active {
            // Select active tab in tile view
            if let Some(active_id) = self.active_tab {
                self.selected_index = self.tabs.iter().position(|t| t.id == active_id);
            }
        } else {
            self.selected_index = None;
            self.search_query.clear();
        }
    }
    
    /// Navigate tile selection
    pub fn select_next(&mut self) {
        let count = self.filtered_tabs().len();
        if count == 0 {
            return;
        }
        
        self.selected_index = Some(match self.selected_index {
            Some(i) => (i + 1) % count,
            None => 0,
        });
    }
    
    pub fn select_prev(&mut self) {
        let count = self.filtered_tabs().len();
        if count == 0 {
            return;
        }
        
        self.selected_index = Some(match self.selected_index {
            Some(0) => count - 1,
            Some(i) => i - 1,
            None => count - 1,
        });
    }
    
    pub fn select_row_down(&mut self, columns: u32) {
        let count = self.filtered_tabs().len();
        if count == 0 {
            return;
        }
        
        self.selected_index = Some(match self.selected_index {
            Some(i) => ((i + columns as usize) % count).min(count - 1),
            None => 0,
        });
    }
    
    pub fn select_row_up(&mut self, columns: u32) {
        let count = self.filtered_tabs().len();
        if count == 0 {
            return;
        }
        
        let cols = columns as usize;
        self.selected_index = Some(match self.selected_index {
            Some(i) if i >= cols => i - cols,
            Some(i) => count - (cols - i).min(count),
            None => count - 1,
        });
    }
    
    /// Activate selected tab and close tile view
    pub fn activate_selected(&mut self) {
        if let Some(idx) = self.selected_index {
            let tabs = self.filtered_tabs();
            if let Some(tab) = tabs.get(idx) {
                let id = tab.id;
                self.activate_tab(id);
            }
        }
        self.tile_view_active = false;
        self.search_query.clear();
    }
    
    /// Close selected tab in tile view
    pub fn close_selected(&mut self) {
        if let Some(idx) = self.selected_index {
            let tabs = self.filtered_tabs();
            if let Some(tab) = tabs.get(idx) {
                let id = tab.id;
                self.close_tab(id);
                
                // Adjust selection
                let new_count = self.filtered_tabs().len();
                if new_count == 0 {
                    self.selected_index = None;
                } else if idx >= new_count {
                    self.selected_index = Some(new_count - 1);
                }
            }
        }
    }
    
    /// Move tab to a new position
    pub fn move_tab(&mut self, from: usize, to: usize) {
        if from < self.tabs.len() && to < self.tabs.len() {
            let tab = self.tabs.remove(from);
            self.tabs.insert(to, tab);
        }
    }
    
    /// Pin/unpin tab
    pub fn toggle_pin(&mut self, id: u64) {
        if let Some(tab) = self.get_tab_mut(id) {
            tab.pinned = !tab.pinned;
        }
        
        // Move pinned tabs to start
        self.tabs.sort_by(|a, b| b.pinned.cmp(&a.pinned));
    }
    
    /// Create a tab group
    pub fn create_group(&mut self, name: String, color: String) -> u64 {
        let id = self.next_group_id;
        self.next_group_id += 1;
        
        self.groups.push(TabGroup {
            id,
            name,
            color,
            collapsed: false,
        });
        
        id
    }
    
    /// Add tab to group
    pub fn add_to_group(&mut self, tab_id: u64, group_id: u64) {
        if let Some(tab) = self.get_tab_mut(tab_id) {
            tab.group_id = Some(group_id);
        }
    }
    
    /// Remove tab from group
    pub fn remove_from_group(&mut self, tab_id: u64) {
        if let Some(tab) = self.get_tab_mut(tab_id) {
            tab.group_id = None;
        }
    }
    
    /// Get tabs by most recently accessed
    pub fn recent_tabs(&self, limit: usize) -> Vec<&Tab> {
        let mut tabs: Vec<_> = self.tabs.iter().collect();
        tabs.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));
        tabs.truncate(limit);
        tabs
    }
    
    /// Duplicate a tab
    pub fn duplicate_tab(&mut self, id: u64) -> Option<u64> {
        let tab = self.get_tab(id)?;
        let url = tab.url.clone();
        let group_id = tab.group_id;
        
        let new_id = self.create_tab(url);
        if let (Some(group_id), Some(tab)) = (group_id, self.get_tab_mut(new_id)) {
            tab.group_id = Some(group_id);
        }
        
        Some(new_id)
    }
}

impl Default for TabManager {
    fn default() -> Self {
        Self::new()
    }
}
