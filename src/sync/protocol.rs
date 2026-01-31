//! Sync protocol definitions
//! JSON-based message format for browser <-> phone communication

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Message wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMessage {
    pub id: u64,
    pub timestamp: u64,
    #[serde(flatten)]
    pub payload: MessagePayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessagePayload {
    #[serde(rename = "command")]
    Command(SyncCommand),
    #[serde(rename = "event")]
    Event(SyncEvent),
    #[serde(rename = "state")]
    State(BrowserState),
    #[serde(rename = "ack")]
    Ack { message_id: u64, success: bool, error: Option<String> },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "pong")]
    Pong,
}

/// Commands from phone to browser
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum SyncCommand {
    // Tab management
    #[serde(rename = "tab.create")]
    TabCreate { url: String },
    #[serde(rename = "tab.close")]
    TabClose { tab_id: u64 },
    #[serde(rename = "tab.activate")]
    TabActivate { tab_id: u64 },
    #[serde(rename = "tab.reload")]
    TabReload { tab_id: u64 },
    #[serde(rename = "tab.move")]
    TabMove { tab_id: u64, to_index: usize },
    #[serde(rename = "tab.pin")]
    TabPin { tab_id: u64, pinned: bool },
    #[serde(rename = "tab.mute")]
    TabMute { tab_id: u64, muted: bool },
    #[serde(rename = "tab.duplicate")]
    TabDuplicate { tab_id: u64 },
    #[serde(rename = "tab.close_others")]
    TabCloseOthers { tab_id: u64 },
    #[serde(rename = "tab.close_right")]
    TabCloseRight { tab_id: u64 },
    
    // Navigation
    #[serde(rename = "nav.back")]
    NavBack,
    #[serde(rename = "nav.forward")]
    NavForward,
    #[serde(rename = "nav.refresh")]
    NavRefresh,
    #[serde(rename = "nav.stop")]
    NavStop,
    #[serde(rename = "nav.go")]
    NavGo { url: String },
    
    // Scroll control
    #[serde(rename = "scroll.to")]
    ScrollTo { x: i32, y: i32 },
    #[serde(rename = "scroll.by")]
    ScrollBy { dx: i32, dy: i32 },
    #[serde(rename = "scroll.top")]
    ScrollTop,
    #[serde(rename = "scroll.bottom")]
    ScrollBottom,
    
    // Page interaction
    #[serde(rename = "page.click")]
    PageClick { x: i32, y: i32 },
    #[serde(rename = "page.type")]
    PageType { text: String },
    #[serde(rename = "page.key")]
    PageKey { key: String, modifiers: Vec<String> },
    #[serde(rename = "page.find")]
    PageFind { query: String },
    #[serde(rename = "page.find_next")]
    PageFindNext,
    #[serde(rename = "page.find_prev")]
    PageFindPrev,
    #[serde(rename = "page.zoom")]
    PageZoom { level: f32 },
    
    // Bookmarks
    #[serde(rename = "bookmark.add")]
    BookmarkAdd { url: String, title: String, folder: Option<String> },
    #[serde(rename = "bookmark.remove")]
    BookmarkRemove { url: String },
    #[serde(rename = "bookmark.list")]
    BookmarkList { folder: Option<String> },
    
    // History
    #[serde(rename = "history.list")]
    HistoryList { limit: usize, offset: usize },
    #[serde(rename = "history.search")]
    HistorySearch { query: String, limit: usize },
    #[serde(rename = "history.clear")]
    HistoryClear { from: Option<u64>, to: Option<u64> },
    
    // Downloads
    #[serde(rename = "download.list")]
    DownloadList,
    #[serde(rename = "download.pause")]
    DownloadPause { download_id: u64 },
    #[serde(rename = "download.resume")]
    DownloadResume { download_id: u64 },
    #[serde(rename = "download.cancel")]
    DownloadCancel { download_id: u64 },
    
    // Settings
    #[serde(rename = "settings.get")]
    SettingsGet { key: String },
    #[serde(rename = "settings.set")]
    SettingsSet { key: String, value: serde_json::Value },
    
    // HUD configuration
    #[serde(rename = "hud.configure")]
    HudConfigure { widgets: Vec<HudWidget> },
    #[serde(rename = "hud.show")]
    HudShow { widget_id: String },
    #[serde(rename = "hud.hide")]
    HudHide { widget_id: String },
    
    // State requests
    #[serde(rename = "state.request")]
    StateRequest { include: Vec<String> },
    #[serde(rename = "state.subscribe")]
    StateSubscribe { events: Vec<String> },
    #[serde(rename = "state.unsubscribe")]
    StateUnsubscribe { events: Vec<String> },
}

/// Events from browser to phone
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum SyncEvent {
    // Connection
    #[serde(rename = "connected")]
    Connected { browser_id: String, version: String },
    #[serde(rename = "disconnected")]
    Disconnected { reason: String },
    
    // Tab events
    #[serde(rename = "tab.created")]
    TabCreated { tab: TabInfo },
    #[serde(rename = "tab.closed")]
    TabClosed { tab_id: u64 },
    #[serde(rename = "tab.updated")]
    TabUpdated { tab: TabInfo },
    #[serde(rename = "tab.activated")]
    TabActivated { tab_id: u64 },
    #[serde(rename = "tab.moved")]
    TabMoved { tab_id: u64, from_index: usize, to_index: usize },
    
    // Navigation events
    #[serde(rename = "nav.started")]
    NavStarted { tab_id: u64, url: String },
    #[serde(rename = "nav.committed")]
    NavCommitted { tab_id: u64, url: String },
    #[serde(rename = "nav.completed")]
    NavCompleted { tab_id: u64, url: String },
    #[serde(rename = "nav.failed")]
    NavFailed { tab_id: u64, url: String, error: String },
    
    // Page events
    #[serde(rename = "page.title_changed")]
    PageTitleChanged { tab_id: u64, title: String },
    #[serde(rename = "page.favicon_changed")]
    PageFaviconChanged { tab_id: u64, favicon_url: Option<String> },
    #[serde(rename = "page.scroll")]
    PageScroll { tab_id: u64, x: i32, y: i32, max_x: i32, max_y: i32 },
    #[serde(rename = "page.load_progress")]
    PageLoadProgress { tab_id: u64, progress: f32 },
    
    // Download events
    #[serde(rename = "download.started")]
    DownloadStarted { download: DownloadInfo },
    #[serde(rename = "download.progress")]
    DownloadProgress { download_id: u64, received: u64, total: Option<u64> },
    #[serde(rename = "download.completed")]
    DownloadCompleted { download_id: u64, path: String },
    #[serde(rename = "download.failed")]
    DownloadFailed { download_id: u64, error: String },
    
    // Find events
    #[serde(rename = "find.results")]
    FindResults { query: String, count: usize, active_index: usize },
    
    // Notification
    #[serde(rename = "notification")]
    Notification { title: String, message: String, icon: Option<String> },
}

/// Full browser state for initial sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserState {
    pub browser_id: String,
    pub version: String,
    pub tabs: Vec<TabInfo>,
    pub active_tab_id: Option<u64>,
    pub bookmarks: Vec<BookmarkInfo>,
    pub downloads: Vec<DownloadInfo>,
    pub settings: HashMap<String, serde_json::Value>,
}

/// Tab information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabInfo {
    pub id: u64,
    pub index: usize,
    pub title: String,
    pub url: String,
    pub favicon_url: Option<String>,
    pub loading: bool,
    pub pinned: bool,
    pub muted: bool,
    pub audible: bool,
    pub group_id: Option<u64>,
    pub preview_available: bool,
}

/// Bookmark information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkInfo {
    pub id: String,
    pub title: String,
    pub url: Option<String>,
    pub folder: Option<String>,
    pub children: Option<Vec<BookmarkInfo>>,
}

/// Download information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadInfo {
    pub id: u64,
    pub url: String,
    pub filename: String,
    pub mime_type: String,
    pub received: u64,
    pub total: Option<u64>,
    pub state: DownloadState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadState {
    Pending,
    InProgress,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// HUD widget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HudWidget {
    pub id: String,
    pub widget_type: HudWidgetType,
    pub position: HudPosition,
    pub size: Option<(u32, u32)>,
    pub visible: bool,
    pub data_source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HudWidgetType {
    TabList,
    TabTiles,
    CurrentTab,
    AddressBar,
    Navigation,
    Scroll,
    FindInPage,
    Downloads,
    Bookmarks,
    History,
    QuickActions,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HudPosition {
    Top,
    Bottom,
    Left,
    Right,
    Center,
    FullScreen,
}

impl SyncMessage {
    pub fn new(payload: MessagePayload) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        
        Self {
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            payload,
        }
    }
    
    pub fn command(cmd: SyncCommand) -> Self {
        Self::new(MessagePayload::Command(cmd))
    }
    
    pub fn event(evt: SyncEvent) -> Self {
        Self::new(MessagePayload::Event(evt))
    }
    
    pub fn state(state: BrowserState) -> Self {
        Self::new(MessagePayload::State(state))
    }
    
    pub fn ack(message_id: u64, success: bool, error: Option<String>) -> Self {
        Self::new(MessagePayload::Ack { message_id, success, error })
    }
    
    #[allow(dead_code)]
    pub fn ping() -> Self {
        Self::new(MessagePayload::Ping)
    }
    
    pub fn pong() -> Self {
        Self::new(MessagePayload::Pong)
    }
    
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
    
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

// === User Authentication Commands ===

/// Commands for user management
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum UserCommand {
    /// List available users for login
    #[serde(rename = "users.list")]
    ListUsers,
    
    /// Authenticate as a user
    #[serde(rename = "auth")]
    Authenticate { 
        user_id: String, 
        pin: Option<String> 
    },
    
    /// Logout current user
    #[serde(rename = "logout")]
    Logout,
    
    /// Add a new user (admin only)
    #[serde(rename = "users.add")]
    AddUser { 
        name: String,
        #[serde(default)]
        pin: Option<String>,
    },
    
    /// Remove a user (admin only)  
    #[serde(rename = "users.remove")]
    RemoveUser { user_id: String },
    
    /// Set user's PIN
    #[serde(rename = "users.set_pin")]
    SetPin { 
        user_id: String, 
        pin: Option<String> 
    },
    
    /// Make user an admin
    #[serde(rename = "users.make_admin")]
    MakeAdmin { user_id: String },
}

/// User-related events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UserEvent {
    /// List of users for login screen
    #[serde(rename = "users")]
    UserList { 
        users: Vec<crate::sync::UserLoginInfo> 
    },
    
    /// Authentication successful
    #[serde(rename = "auth_ok")]
    AuthOk { 
        user_id: String,
        user_name: String,
        is_admin: bool,
    },
    
    /// Authentication failed
    #[serde(rename = "auth_fail")]
    AuthFail { 
        reason: String 
    },
    
    /// User added
    #[serde(rename = "user_added")]
    UserAdded { 
        user: crate::sync::UserLoginInfo 
    },
    
    /// User removed
    #[serde(rename = "user_removed")]
    UserRemoved { 
        user_id: String 
    },
}
