//! Browser runner - Pure Rust, NO CHROME/WEBKIT
use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum BrowserEvent {
    Navigate(String),
    NewTab,
    CloseTab(usize),
    GoBack,
    GoForward,
    Reload,
    OpenFile(PathBuf),
    Quit,
}

/// Run browser with pure Rust rendering
pub fn run_webview_browser() -> Result<()> {
    tracing::info!("Pure Rust browser (no Chrome/WebKit)");
    crate::app::run_browser()
}
