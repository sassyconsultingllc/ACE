//! Tab management - Represents browser tabs (web pages or file viewers)

use crate::file_handler::{FileType, OpenFile};
use std::path::PathBuf;
use uuid::Uuid;

/// Unique identifier for a tab
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TabId(pub Uuid);

impl TabId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TabId {
    fn default() -> Self {
        Self::new()
    }
}

/// What type of content a tab holds
#[derive(Debug, Clone)]
pub enum TabContent {
    /// Web page loaded in webview
    Web {
        url: String,
        title: String,
        loading: bool,
        can_go_back: bool,
        can_go_forward: bool,
        is_secure: bool,
        favicon: Option<Vec<u8>>,
    },
    /// Local or downloaded file
    File(OpenFile),
    /// New tab page (home/start page)
    NewTab,
    /// Settings page
    Settings,
    /// History page
    History,
    /// Bookmarks page
    Bookmarks,
    /// Downloads page
    Downloads,
}

impl TabContent {
    pub fn web(url: impl Into<String>) -> Self {
        let url = url.into();
        Self::Web {
            url,
            title: String::new(),
            loading: true,
            can_go_back: false,
            can_go_forward: false,
            is_secure: false,
            favicon: None,
        }
    }
    
    pub fn get_display_url(&self) -> String {
        match self {
            TabContent::Web { url, .. } => url.clone(),
            TabContent::File(f) => format!("file://{}", f.path.display()),
            TabContent::NewTab => "sassy://newtab".into(),
            TabContent::Settings => "sassy://settings".into(),
            TabContent::History => "sassy://history".into(),
            TabContent::Bookmarks => "sassy://bookmarks".into(),
            TabContent::Downloads => "sassy://downloads".into(),
        }
    }
}

/// A browser tab
#[derive(Debug, Clone)]
pub struct Tab {
    pub id: TabId,
    pub content: TabContent,
    pub pinned: bool,
    pub muted: bool,
    pub created_at: std::time::Instant,
    pub last_accessed: std::time::Instant,
}

impl Tab {
    pub fn new(content: TabContent) -> Self {
        let now = std::time::Instant::now();
        Self {
            id: TabId::new(),
            content,
            pinned: false,
            muted: false,
            created_at: now,
            last_accessed: now,
        }
    }
    
    pub fn new_tab() -> Self {
        Self::new(TabContent::NewTab)
    }
    
    pub fn web(url: impl Into<String>) -> Self {
        Self::new(TabContent::web(url))
    }
    
    pub fn file(file: OpenFile) -> Self {
        Self::new(TabContent::File(file))
    }
    
    /// Get the tab's display title
    pub fn title(&self) -> String {
        match &self.content {
            TabContent::Web { title, url, .. } => {
                if title.is_empty() {
                    // Extract domain from URL
                    url::Url::parse(url)
                        .ok()
                        .and_then(|u| u.host_str().map(String::from))
                        .unwrap_or_else(|| url.clone())
                } else {
                    title.clone()
                }
            }
            TabContent::File(f) => {
                f.path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "File".into())
            }
            TabContent::NewTab => "New Tab".into(),
            TabContent::Settings => "Settings".into(),
            TabContent::History => "History".into(),
            TabContent::Bookmarks => "Bookmarks".into(),
            TabContent::Downloads => "Downloads".into(),
        }
    }
    
    /// Get icon for the tab
    pub fn icon(&self) -> &'static str {
        match &self.content {
            TabContent::Web { is_secure, .. } => {
                if *is_secure { "🔒" } else { "🌐" }
            }
            TabContent::File(f) => match f.file_type {
                FileType::Image | FileType::ImageRaw | FileType::ImagePsd => "🖼️",
                FileType::Pdf => "📄",
                FileType::Document => "📝",
                FileType::Spreadsheet => "📊",
                FileType::Chemical => "🧬",
                FileType::Text | FileType::Markdown => "📃",
                FileType::Archive => "📦",
                FileType::Model3D => "🧊",
                FileType::Font => "🔤",
                FileType::Audio => "🎵",
                FileType::Video => "🎬",
                FileType::Ebook => "📚",
                FileType::Unknown => "📁",
            },
            TabContent::NewTab => "➕",
            TabContent::Settings => "⚙️",
            TabContent::History => "🕐",
            TabContent::Bookmarks => "⭐",
            TabContent::Downloads => "⬇️",
        }
    }
    
    /// Check if this is a web tab
    pub fn is_web(&self) -> bool {
        matches!(self.content, TabContent::Web { .. })
    }
    
    /// Check if this is a file viewer tab  
    pub fn is_file(&self) -> bool {
        matches!(self.content, TabContent::File(_))
    }
    
    /// Check if loading
    pub fn is_loading(&self) -> bool {
        match &self.content {
            TabContent::Web { loading, .. } => *loading,
            _ => false,
        }
    }
    
    /// Update last accessed time
    pub fn touch(&mut self) {
        self.last_accessed = std::time::Instant::now();
    }
}
