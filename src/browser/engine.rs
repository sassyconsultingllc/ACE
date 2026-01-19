#![allow(dead_code, unused_variables, unused_imports)]
//! Browser Engine - WebView management and coordination

use crate::browser::{
    Tab, TabId, TabContent,
    DownloadManager,
    HistoryManager,
    BookmarkManager,
};
use crate::file_handler::{FileHandler, FileType};
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Messages sent from WebView to the browser
#[derive(Debug, Clone)]
pub enum WebViewMessage {
    TitleChanged { tab_id: TabId, title: String },
    UrlChanged { tab_id: TabId, url: String },
    LoadStarted { tab_id: TabId },
    LoadFinished { tab_id: TabId },
    NavigationStateChanged { tab_id: TabId, can_go_back: bool, can_go_forward: bool },
    DownloadStarted { url: String, suggested_filename: Option<String> },
    NewWindowRequested { url: String },
    CloseRequested { tab_id: TabId },
    ContextMenu { tab_id: TabId, x: i32, y: i32, link_url: Option<String>, image_url: Option<String> },
}

/// Commands sent to WebView
#[derive(Debug, Clone)]
pub enum WebViewCommand {
    Navigate(String),
    GoBack,
    GoForward,
    Reload,
    Stop,
    ExecuteScript(String),
    SetZoom(f64),
    Find(String),
    Print,
}

/// Pending navigation for a tab
#[derive(Debug, Clone)]
pub struct PendingNavigation {
    pub url: String,
}

/// The main browser engine
pub struct BrowserEngine {
    // Tab management
    tabs: Vec<Tab>,
    active_tab_index: usize,
    
    // Managers
    pub downloads: DownloadManager,
    pub history: HistoryManager,
    pub bookmarks: BookmarkManager,
    pub file_handler: FileHandler,
    
    // Messages from webviews
    messages: Arc<Mutex<Vec<WebViewMessage>>>,
    
    // Pending navigations (for new tabs that need webview setup)
    pending_navigations: HashMap<TabId, PendingNavigation>,
    pending_downloads: Vec<(String, Option<String>)>,
    
    // Settings
    pub home_url: String,
    pub search_engine: String,
    pub default_zoom: f64,
    
    // State
    address_bar_text: String,
    address_bar_focused: bool,
    show_bookmarks_bar: bool,
    show_downloads_panel: bool,
    incognito_mode: bool,
}

impl BrowserEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            tabs: Vec::new(),
            active_tab_index: 0,
            downloads: DownloadManager::new(),
            history: HistoryManager::new(),
            bookmarks: BookmarkManager::new(),
            file_handler: FileHandler::new(),
            messages: Arc::new(Mutex::new(Vec::new())),
            pending_navigations: HashMap::new(),
            pending_downloads: Vec::new(),
            home_url: "https://duckduckgo.com".into(),
            search_engine: "https://duckduckgo.com/?q=".into(),
            default_zoom: 1.0,
            address_bar_text: String::new(),
            address_bar_focused: false,
            show_bookmarks_bar: true,
            show_downloads_panel: false,
            incognito_mode: false,
        };
        
        // Start with one new tab
        engine.new_tab();
        
        engine
    }
    
    // ========== Tab Management ==========
    
    /// Create a new tab
    pub fn new_tab(&mut self) -> TabId {
        let tab = Tab::new_tab();
        let id = tab.id;
        self.tabs.push(tab);
        self.active_tab_index = self.tabs.len() - 1;
        self.update_address_bar();
        id
    }
    
    /// Create a new tab and navigate to URL
    pub fn new_tab_with_url(&mut self, url: &str) -> TabId {
        let url = self.normalize_url(url);
        let tab = Tab::web(&url);
        let id = tab.id;
        
        self.pending_navigations.insert(id, PendingNavigation { url });
        self.tabs.push(tab);
        self.active_tab_index = self.tabs.len() - 1;
        self.update_address_bar();
        
        id
    }
    
    /// Open a file in a new tab
    pub fn open_file(&mut self, path: PathBuf) -> Result<TabId> {
        let file = self.file_handler.load_file(&path)?;
        let tab = Tab::file(file);
        let id = tab.id;
        self.tabs.push(tab);
        self.active_tab_index = self.tabs.len() - 1;
        self.update_address_bar();
        Ok(id)
    }
    
    /// Close a tab by index
    pub fn close_tab(&mut self, index: usize) {
        if self.tabs.len() <= 1 {
            // Don't close last tab, just reset it
            self.tabs[0] = Tab::new_tab();
            return;
        }
        
        if index < self.tabs.len() {
            self.tabs.remove(index);
            
            if self.active_tab_index >= self.tabs.len() {
                self.active_tab_index = self.tabs.len() - 1;
            } else if self.active_tab_index > index {
                self.active_tab_index -= 1;
            }
            
            self.update_address_bar();
        }
    }
    
    /// Close tab by ID
    pub fn close_tab_by_id(&mut self, id: TabId) {
        if let Some(index) = self.tabs.iter().position(|t| t.id == id) {
            self.close_tab(index);
        }
    }
    
    /// Set active tab by index
    pub fn set_active_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_tab_index = index;
            self.tabs[index].touch();
            self.update_address_bar();
        }
    }
    
    /// Set active tab by ID
    pub fn set_active_tab_by_id(&mut self, id: TabId) {
        if let Some(index) = self.tabs.iter().position(|t| t.id == id) {
            self.set_active_tab(index);
        }
    }
    
    /// Get current active tab
    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active_tab_index)
    }
    
    /// Get current active tab mutably
    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active_tab_index)
    }
    
    /// Get active tab ID
    pub fn active_tab_id(&self) -> Option<TabId> {
        self.active_tab().map(|t| t.id)
    }
    
    /// Get active tab index
    pub fn active_tab_index(&self) -> usize {
        self.active_tab_index
    }
    
    /// Get all tabs
    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }
    
    /// Get tab count
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }
    
    /// Move tab
    pub fn move_tab(&mut self, from: usize, to: usize) {
        if from < self.tabs.len() && to < self.tabs.len() && from != to {
            let tab = self.tabs.remove(from);
            self.tabs.insert(to, tab);
            
            // Update active index
            if self.active_tab_index == from {
                self.active_tab_index = to;
            } else if from < self.active_tab_index && to >= self.active_tab_index {
                self.active_tab_index -= 1;
            } else if from > self.active_tab_index && to <= self.active_tab_index {
                self.active_tab_index += 1;
            }
        }
    }
    
    /// Duplicate a tab
    pub fn duplicate_tab(&mut self, index: usize) -> Option<TabId> {
        if index >= self.tabs.len() {
            return None;
        }
        
        let old_tab_id = self.tabs[index].id;
        let new_tab = self.tabs[index].clone();
        let new_id = TabId::new();
        
        let mut new_tab = Tab {
            id: new_id,
            ..new_tab
        };
        new_tab.touch();
        
        self.tabs.insert(index + 1, new_tab);
        
        // Copy pending navigation if any
        if let Some(pending) = self.pending_navigations.get(&old_tab_id).cloned() {
            self.pending_navigations.insert(new_id, pending);
        }
        
        Some(new_id)
    }
    
    /// Pin/unpin a tab
    pub fn toggle_pin(&mut self, index: usize) {
        if let Some(tab) = self.tabs.get_mut(index) {
            tab.pinned = !tab.pinned;
        }
    }
    
    // ========== Navigation ==========
    
    /// Navigate the active tab to a URL
    pub fn navigate(&mut self, url: &str) {
        let url = self.normalize_url(url);
        
        // Check if it's a file URL or local file
        if url.starts_with("file://") || self.is_local_file(&url) {
            self.navigate_to_file(&url);
            return;
        }
        
        // Check if it's an internal URL
        if url.starts_with("sassy://") {
            self.navigate_to_internal(&url);
            return;
        }
        
        // Get tab ID first
        let tab_id = if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            match &mut tab.content {
                TabContent::Web { url: current_url, loading, .. } => {
                    *current_url = url.clone();
                    *loading = true;
                }
                _ => {
                    // Convert to web tab
                    tab.content = TabContent::web(&url);
                }
            }
            Some(tab.id)
        } else {
            None
        };
        
        if let Some(id) = tab_id {
            self.pending_navigations.insert(id, PendingNavigation { url: url.clone() });
            
            // Add to history
            if !self.incognito_mode {
                self.history.add(&url, "");
            }
        }
        
        self.update_address_bar();
    }
    
    fn navigate_to_file(&mut self, url: &str) {
        let path = if url.starts_with("file://") {
            PathBuf::from(url.strip_prefix("file://").unwrap_or(url))
        } else {
            PathBuf::from(url)
        };
        
        if let Ok(file) = self.file_handler.load_file(&path) {
            if let Some(tab) = self.active_tab_mut() {
                tab.content = TabContent::File(file);
            }
        }
        
        self.update_address_bar();
    }
    
    fn navigate_to_internal(&mut self, url: &str) {
        if let Some(tab) = self.active_tab_mut() {
            tab.content = match url {
                "sassy://newtab" => TabContent::NewTab,
                "sassy://settings" => TabContent::Settings,
                "sassy://history" => TabContent::History,
                "sassy://bookmarks" => TabContent::Bookmarks,
                "sassy://downloads" => TabContent::Downloads,
                _ => TabContent::NewTab,
            };
        }
        
        self.update_address_bar();
    }
    
    fn is_local_file(&self, url: &str) -> bool {
        let path = PathBuf::from(url);
        path.exists() && path.is_file()
    }
    
    /// Normalize URL (add https://, handle search queries)
    pub fn normalize_url(&self, input: &str) -> String {
        let trimmed = input.trim();
        
        // Empty input -> home
        if trimmed.is_empty() {
            return self.home_url.clone();
        }
        
        // Already has scheme
        if trimmed.starts_with("http://") || 
           trimmed.starts_with("https://") ||
           trimmed.starts_with("file://") ||
           trimmed.starts_with("sassy://") {
            return trimmed.to_string();
        }
        
        // Looks like a URL (has dot and no spaces)
        if trimmed.contains('.') && !trimmed.contains(' ') {
            // Check for common TLDs
            let parts: Vec<&str> = trimmed.split('/').collect();
            let domain = parts[0];
            if domain.ends_with(".com") || domain.ends_with(".org") ||
               domain.ends_with(".net") || domain.ends_with(".io") ||
               domain.ends_with(".dev") || domain.ends_with(".gov") ||
               domain.ends_with(".edu") || domain.contains(':') {
                return format!("https://{}", trimmed);
            }
        }
        
        // Localhost
        if trimmed.starts_with("localhost") || trimmed.starts_with("127.0.0.1") {
            return format!("http://{}", trimmed);
        }
        
        // Treat as search query
        format!("{}{}", self.search_engine, urlencoding::encode(trimmed))
    }
    
    /// Go back in history for active tab
    pub fn go_back(&mut self) {
        if let Some(tab) = self.active_tab() {
            if let TabContent::Web { can_go_back: true, .. } = tab.content {
                self.pending_navigations.insert(tab.id, PendingNavigation {
                    url: "__BACK__".into(),
                });
            }
        }
    }
    
    /// Go forward in history for active tab
    pub fn go_forward(&mut self) {
        if let Some(tab) = self.active_tab() {
            if let TabContent::Web { can_go_forward: true, .. } = tab.content {
                self.pending_navigations.insert(tab.id, PendingNavigation {
                    url: "__FORWARD__".into(),
                });
            }
        }
    }
    
    /// Reload active tab
    pub fn reload(&mut self) {
        if let Some(tab) = self.active_tab() {
            if tab.is_web() {
                self.pending_navigations.insert(tab.id, PendingNavigation {
                    url: "__RELOAD__".into(),
                });
            }
        }
    }
    
    /// Stop loading active tab
    pub fn stop(&mut self) {
        if let Some(tab) = self.active_tab() {
            if tab.is_web() {
                self.pending_navigations.insert(tab.id, PendingNavigation {
                    url: "__STOP__".into(),
                });
            }
        }
    }
    
    /// Navigate to home page
    pub fn go_home(&mut self) {
        self.navigate(&self.home_url.clone());
    }
    
    // ========== Address Bar ==========
    
    pub fn address_bar_text(&self) -> &str {
        &self.address_bar_text
    }
    
    pub fn set_address_bar_text(&mut self, text: String) {
        self.address_bar_text = text;
    }
    
    pub fn address_bar_focused(&self) -> bool {
        self.address_bar_focused
    }
    
    pub fn set_address_bar_focused(&mut self, focused: bool) {
        self.address_bar_focused = focused;
        if focused {
            // Select all text when focused
            self.address_bar_text = self.active_tab()
                .map(|t| t.content.get_display_url())
                .unwrap_or_default();
        }
    }
    
    fn update_address_bar(&mut self) {
        if !self.address_bar_focused {
            self.address_bar_text = self.active_tab()
                .map(|t| t.content.get_display_url())
                .unwrap_or_default();
        }
    }
    
    /// Submit address bar (navigate to entered URL)
    pub fn submit_address_bar(&mut self) {
        let url = self.address_bar_text.clone();
        self.address_bar_focused = false;
        self.navigate(&url);
    }
    
    // ========== Bookmarks ==========
    
    pub fn toggle_bookmark(&mut self) {
        let bookmark_info = self.active_tab().and_then(|tab| {
            if let TabContent::Web { url, title, .. } = &tab.content {
                Some((url.clone(), title.clone()))
            } else {
                None
            }
        });
        
        if let Some((url, title)) = bookmark_info {
            if self.bookmarks.is_bookmarked(&url) {
                self.bookmarks.remove_by_url(&url);
            } else {
                self.bookmarks.add(&url, &title, None);
            }
        }
    }
    
    pub fn is_current_page_bookmarked(&self) -> bool {
        if let Some(tab) = self.active_tab() {
            if let TabContent::Web { url, .. } = &tab.content {
                return self.bookmarks.is_bookmarked(url);
            }
        }
        false
    }
    
    pub fn show_bookmarks_bar(&self) -> bool {
        self.show_bookmarks_bar
    }
    
    pub fn show_bookmarks_bar_mut(&mut self) -> &mut bool {
        &mut self.show_bookmarks_bar
    }
    
    pub fn set_show_bookmarks_bar(&mut self, show: bool) {
        self.show_bookmarks_bar = show;
    }
    
    // ========== Downloads ==========
    
    pub fn show_downloads_panel(&self) -> bool {
        self.show_downloads_panel
    }
    
    pub fn set_show_downloads_panel(&mut self, show: bool) {
        self.show_downloads_panel = show;
    }
    
    pub fn start_download(&mut self, url: &str, filename: Option<&str>) {
        if let Ok(_id) = self.downloads.start_download(url, filename) {
            self.show_downloads_panel = true;
        }
    }
    
    // ========== Message Handling ==========
    
    /// Get message channel for WebView callbacks
    pub fn message_channel(&self) -> Arc<Mutex<Vec<WebViewMessage>>> {
        Arc::clone(&self.messages)
    }
    
    /// Process pending messages from WebViews
    pub fn process_messages(&mut self) {
        let messages: Vec<WebViewMessage> = {
            let mut msgs = self.messages.lock().unwrap();
            std::mem::take(&mut *msgs)
        };
        
        for msg in messages {
            self.handle_message(msg);
        }
    }
    
    fn handle_message(&mut self, msg: WebViewMessage) {
        match msg {
            WebViewMessage::TitleChanged { tab_id, title } => {
                if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
                    if let TabContent::Web { title: t, url, .. } = &mut tab.content {
                        *t = title.clone();
                        
                        // Update history with title
                        if !self.incognito_mode {
                            self.history.add(url, &title);
                        }
                    }
                }
            }
            
            WebViewMessage::UrlChanged { tab_id, url } => {
                if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
                    if let TabContent::Web { url: u, is_secure, .. } = &mut tab.content {
                        *u = url.clone();
                        *is_secure = url.starts_with("https://");
                    }
                }
                self.update_address_bar();
            }
            
            WebViewMessage::LoadStarted { tab_id } => {
                if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
                    if let TabContent::Web { loading, .. } = &mut tab.content {
                        *loading = true;
                    }
                }
            }
            
            WebViewMessage::LoadFinished { tab_id } => {
                if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
                    if let TabContent::Web { loading, .. } = &mut tab.content {
                        *loading = false;
                    }
                }
            }
            
            WebViewMessage::NavigationStateChanged { tab_id, can_go_back, can_go_forward } => {
                if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
                    if let TabContent::Web { can_go_back: b, can_go_forward: f, .. } = &mut tab.content {
                        *b = can_go_back;
                        *f = can_go_forward;
                    }
                }
            }
            
            WebViewMessage::DownloadStarted { url, suggested_filename } => {
                // Defer to caller to enforce policies (parental controls, approvals)
                self.pending_downloads.push((url, suggested_filename));
            }
            
            WebViewMessage::NewWindowRequested { url } => {
                self.new_tab_with_url(&url);
            }
            
            WebViewMessage::CloseRequested { tab_id } => {
                self.close_tab_by_id(tab_id);
            }
            
            WebViewMessage::ContextMenu { .. } => {
                // Handle context menu - will be implemented in UI
            }
        }
    }
    
    /// Get pending navigation for a tab
    pub fn take_pending_navigation(&mut self, tab_id: TabId) -> Option<PendingNavigation> {
        self.pending_navigations.remove(&tab_id)
    }
    
    /// Drain any pending download requests emitted from webviews
    pub fn take_pending_downloads(&mut self) -> Vec<(String, Option<String>)> {
        std::mem::take(&mut self.pending_downloads)
    }

    /// Check if there are pending navigations
    pub fn has_pending_navigation(&self, tab_id: TabId) -> bool {
        self.pending_navigations.contains_key(&tab_id)
    }
    
    // ========== Settings ==========
    
    pub fn set_home_url(&mut self, url: String) {
        self.home_url = url;
    }
    
    pub fn set_search_engine(&mut self, url: String) {
        self.search_engine = url;
    }
    
    pub fn incognito_mode(&self) -> bool {
        self.incognito_mode
    }
    
    pub fn set_incognito_mode(&mut self, incognito: bool) {
        self.incognito_mode = incognito;
    }
    
    // ========== File Handling ==========
    
    /// Check if a URL should be opened in native viewer
    pub fn should_open_natively(url: &str) -> bool {
        let lower = url.to_lowercase();
        
        // Check file extension
        let extensions = [
            // Images
            ".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp", ".svg", ".ico", ".tiff", ".avif",
            // Documents
            ".pdf", ".docx", ".doc", ".odt", ".rtf",
            // Spreadsheets
            ".xlsx", ".xls", ".ods", ".csv",
            // Chemical
            ".pdb", ".mol", ".sdf", ".xyz",
        ];
        
        extensions.iter().any(|ext| lower.ends_with(ext))
    }
    
    /// Get file type from URL
    pub fn file_type_from_url(url: &str) -> FileType {
        FileHandler::detect_file_type(&PathBuf::from(url))
    }
}

impl Default for BrowserEngine {
    fn default() -> Self {
        Self::new()
    }
}
