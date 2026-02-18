//! Browser module - Pure Rust web browsing, NO CHROME/WEBKIT

pub mod bookmarks;
pub mod download;
pub mod engine;
pub mod history;
pub mod runner;
pub mod tab;
pub mod webview;

pub use bookmarks::BookmarkManager;
pub use download::{DownloadManager, DownloadState};
pub use engine::BrowserEngine;
pub use history::HistoryManager;
pub use tab::{Tab, TabContent, TabId};
