//! WebView stub - Pure Rust rendering, no external browser engine
//! NO CHROME. NO GOOGLE. NO WEBKIT.

use crate::browser::tab::TabId;
use anyhow::Result;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub enum WebViewMessage {
    TitleChanged { tab_id: TabId, title: String },
    UrlChanged { tab_id: TabId, url: String },
    LoadStarted { tab_id: TabId },
    LoadFinished { tab_id: TabId },
}

/// Stub - actual rendering done by pure Rust engine
pub struct WebViewManager {
    messages: Arc<Mutex<Vec<WebViewMessage>>>,
}

impl WebViewManager {
    pub fn new(messages: Arc<Mutex<Vec<WebViewMessage>>>) -> Self {
        Self { messages }
    }
    
    pub fn navigate(&mut self, _tab_id: TabId, _url: &str) -> Result<()> { Ok(()) }
    pub fn go_back(&mut self, _tab_id: TabId) -> Result<()> { Ok(()) }
    pub fn go_forward(&mut self, _tab_id: TabId) -> Result<()> { Ok(()) }
    pub fn reload(&mut self, _tab_id: TabId) -> Result<()> { Ok(()) }
}
