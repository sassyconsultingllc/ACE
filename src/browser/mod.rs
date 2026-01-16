//! Browser module - Pure Rust web browsing, NO CHROME/WEBKIT

pub mod bookmarks;
pub mod download;
pub mod engine;
pub mod history;
pub mod runner;
pub mod tab;
pub mod webview;

pub use bookmarks::{Bookmark, BookmarkManager};
pub use download::{Download, DownloadManager, DownloadState};
pub use engine::BrowserEngine;
pub use history::{HistoryEntry, HistoryManager};
pub use runner::{run_webview_browser, BrowserEvent};
pub use tab::{Tab, TabContent, TabId};
pub use webview::{WebViewManager, WebViewMessage};
