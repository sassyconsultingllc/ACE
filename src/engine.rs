//! Browser engine - integrates all components with new UI system
//! v1.0.1 - Production ready with input, network bar, sandbox, link clicking

#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(deprecated)]

use crate::dom::{Document, NodeType, NodeRef};
use crate::style::{StyleEngine, ComputedStyle};
use crate::layout::{LayoutEngine, LayoutBox};
use crate::paint::Painter;
use crate::renderer::Renderer;
use crate::script_engine::ScriptEngine;
use crate::extensions::ExtensionManager;
use crate::protocol::HttpClient;
use crate::ui::{
    UI, Theme, Edge, MouseButton, Modifiers, 
    InputManager, InputAction, Focus, UiBounds, InputRect,
    NetworkBar, RequestState, SidebarState,
    PopupManager, InteractionType,
    MeaningfulInteraction, TrustLevel, SandboxAction,
};
use crate::ai::{AiRuntime, HelpQuery, load_runtime, run_help_query};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use crate::ui::popup::PopupReason;
use crate::ui::input::Key;
use crate::ui::render::UIRenderer;
use crate::sync::{SyncEvent, SyncCommand};
use crate::sync::{SecureSyncServer, SyncConfig, FamilyConfig, UserManager};
use crate::update::{UpdateChecker, UpdateStatus};
use crate::sandbox::{
    Quarantine, QuarantinedFile, ReleaseStatus, SandboxManager, Interaction, PageTrust,
    PopupHandler, PopupRequest, PopupDecision,
};

use winit::event::{ElementState, WindowEvent, MouseButton as WinitMouseButton};
use winit::event_loop::EventLoop;
use winit::keyboard::{Key as WinitKey, NamedKey};
use winit::window::{CursorIcon, WindowAttributes};
use softbuffer::Surface;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

const DEFAULT_WIDTH: u32 = 1280;
const DEFAULT_HEIGHT: u32 = 800;
const SYNC_PORT: u16 = 8765;
const NETWORK_BAR_HEIGHT: u32 = 20;

/// Timer for setTimeout/setInterval
#[derive(Debug, Clone)]
pub struct Timer {
    pub id: u32,
    pub callback: String,
    pub delay_ms: u64,
    pub created_at: std::time::Instant,
    pub repeat: bool,
}

/// Hit test result for click handling
#[derive(Debug, Clone)]
pub enum HitTestResult {
    Nothing,
    Link { url: String, text: String },
    FormInput { element_id: u64 },
    FormButton { element_id: u64 },
    Image { src: String, alt: String },
    Text { content: String },
}

// Registry for tracking clickable element nodes
thread_local! {
    static CLICKABLE_NODES: std::cell::RefCell<std::collections::HashMap<u64, NodeRef>> = 
        std::cell::RefCell::new(std::collections::HashMap::new());
    static NEXT_CLICKABLE_ID: std::cell::Cell<u64> = const { std::cell::Cell::new(1) };
}

/// Register a node for clickable element tracking, returns unique ID
fn register_clickable_node(node: &NodeRef) -> u64 {
    let id = NEXT_CLICKABLE_ID.with(|c| {
        let id = c.get();
        c.set(id + 1);
        id
    });
    CLICKABLE_NODES.with(|m| {
        m.borrow_mut().insert(id, node.clone());
    });
    id
}

/// Get a registered clickable node by ID
fn get_clickable_node(id: u64) -> Option<NodeRef> {
    CLICKABLE_NODES.with(|m| m.borrow().get(&id).cloned())
}

/// Clear all clickable node registrations
fn clear_clickable_nodes() {
    CLICKABLE_NODES.with(|m| m.borrow_mut().clear());
}

/// Link extracted from layout for click detection
#[derive(Debug, Clone)]
struct ClickableRegion {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub target: HitTestResult,
}

pub struct BrowserState {
    // Core components
    document: Option<Document>,
    renderer: Renderer,
    layout: Option<LayoutBox>,
    computed_styles: std::collections::HashMap<*const std::cell::RefCell<crate::dom::Node>, ComputedStyle>,
    
    // Engines
    style_engine: StyleEngine,
    layout_engine: LayoutEngine,
    painter: Painter,
    script_engine: ScriptEngine,
    extension_manager: ExtensionManager,
    http_client: HttpClient,
    
    // UI system
    ui: UI,
    ui_renderer: UIRenderer,
    input: InputManager,
    network_bar: NetworkBar,
    popup_manager: PopupManager,
    ai_runtime: AiRuntime,
    help_rx: Option<Receiver<Result<String, String>>>,
    
    // Navigation
    history: Vec<String>,
    history_index: usize,
    current_url: String,
    
    // Viewport
    scroll_x: i32,
    scroll_y: i32,
    max_scroll_y: i32,
    
    // Hit testing
    clickable_regions: Vec<ClickableRegion>,
    hovered_link: Option<String>,
    
    // UI bounds for hit testing
    ui_bounds: UiBounds,
    
    // State
    loading: bool,
    needs_repaint: bool,
    last_frame: Instant,
    request_id_counter: u64,
    
    // Cursor
    current_cursor: CursorIcon,
    
    // Update checker
    update_checker: UpdateChecker,
    
    // Sandbox and quarantine
    quarantine: Quarantine,
    sandbox_manager: SandboxManager,
    popup_handler: PopupHandler,
    
    // Family sync
    family_config: FamilyConfig,
    user_manager: UserManager,
    secure_sync: Option<SecureSyncServer>,
}

impl BrowserState {
    pub fn new() -> Self {
        #[allow(unused_imports)]
        use crate::data::{SessionRestore, TabState};
        
        let mut ui = UI::new(DEFAULT_WIDTH, DEFAULT_HEIGHT);
        let ai_runtime = load_runtime();
        ui.ai = ai_runtime.config.clone();
        
        // Try to start phone sync server
        if let Err(e) = ui.start_sync(SYNC_PORT) {
            eprintln!("Warning: Could not start phone sync: {}", e);
        } else {
            println!("Phone sync started on port {}", SYNC_PORT);
            if let Some(ref server) = ui.sync_server {
                if let Some(qr) = server.qr_data() {
                    println!("   Connect URL: {}", qr);
                }
            }
        }
        
        // Restore previous session or create initial tab
        let session = SessionRestore::load("default");
        if let Some(ref s) = session {
            for tab_state in &s.tabs {
                ui.tab_manager.create_tab(tab_state.url.clone());
            }
            if let Some(active) = s.active_tab {
                if let Some(tab) = ui.tab_manager.tabs().get(active) {
                    ui.tab_manager.activate_tab(tab.id);
                }
            }
            println!("Session restored: {} tabs", s.tabs.len());
        }
        if ui.tab_manager.tabs().is_empty() {
            ui.tab_manager.create_tab("about:blank".into());
        }
        
        Self {
            document: None,
            renderer: Renderer::new(DEFAULT_WIDTH, DEFAULT_HEIGHT),
            layout: None,
            computed_styles: std::collections::HashMap::new(),
            style_engine: StyleEngine::new(),
            layout_engine: LayoutEngine::new(DEFAULT_WIDTH as f32, DEFAULT_HEIGHT as f32),
            painter: Painter::new(DEFAULT_WIDTH, DEFAULT_HEIGHT),
            script_engine: ScriptEngine::new(),
            extension_manager: ExtensionManager::new(),
            http_client: HttpClient::new(),
            ui,
            ui_renderer: UIRenderer::new(DEFAULT_WIDTH, DEFAULT_HEIGHT),
            input: InputManager::new(),
            network_bar: NetworkBar::new(),
            popup_manager: PopupManager::new(),
            ai_runtime,
            help_rx: None,
            history: Vec::new(),
            history_index: 0,
            current_url: String::new(),
            scroll_x: 0,
            scroll_y: 0,
            max_scroll_y: 0,
            clickable_regions: Vec::new(),
            hovered_link: None,
            ui_bounds: UiBounds::default(),
            loading: false,
            needs_repaint: true,
            last_frame: Instant::now(),
            request_id_counter: 0,
            current_cursor: CursorIcon::Default,
            update_checker: UpdateChecker::new(),
            quarantine: Quarantine::new(),
            sandbox_manager: SandboxManager::new(),
            popup_handler: PopupHandler::new(),
            family_config: FamilyConfig::default(),
            user_manager: UserManager::new(),
            secure_sync: None,
        }
    }
    
    /// Generate unique request ID for network tracking
    fn next_request_id(&mut self) -> u64 {
        self.request_id_counter += 1;
        self.request_id_counter
    }
    
    pub fn navigate(&mut self, url: &str) {
        // Normalize URL
        let url = self.normalize_url(url);
        
        self.loading = true;
        self.current_url = url.clone();
        
        // Notify popup handler page is loading
        self.popup_handler.page_loading();
        
        // Update input manager address bar
        self.input.set_address(&url);
        
        // Reset sandbox for new navigation
        if let Some(tab) = self.ui.tab_manager.active_tab_mut() {
            tab.navigate(&url);
            // Create page sandbox for this tab
            self.sandbox_manager.create(tab.id, url.clone());
        }
        
        // Start network tracking
        let req_id = self.next_request_id();
        self.network_bar.start_request(req_id, &url, "GET");
        
        // Broadcast navigation start
        if let Some(tab) = self.ui.tab_manager.active_tab() {
            self.ui.broadcast(SyncEvent::NavStarted {
                tab_id: tab.id,
                url: url.clone(),
            });
        }
        
        // Add to history
        if self.history_index < self.history.len() {
            self.history.truncate(self.history_index);
        }
        self.history.push(url.clone());
        self.history_index = self.history.len();
        
        // Fetch and parse
        self.network_bar.update_request(req_id, RequestState::Waiting);
        
        match self.http_client.fetch(&url) {
            Ok(response) => {
                self.network_bar.update_request(req_id, RequestState::Receiving);
                self.network_bar.add_bytes(req_id, response.body.len() as u64);
                
                let html = response.body.clone();
                let base_url = url::Url::parse(&url).ok();
                
                // Parse HTML using renderer
                self.renderer.parse_html(&html);
                self.document = Some(self.renderer.document.clone());
                
                // Extract title
                if let Some(ref doc) = self.document {
                    if let Some(title) = Self::extract_title(doc) {
                        if let Some(tab) = self.ui.tab_manager.active_tab_mut() {
                            tab.title = title.clone();
                        }
                        
                        // Broadcast title change
                        if let Some(tab) = self.ui.tab_manager.active_tab() {
                            self.ui.broadcast(SyncEvent::PageTitleChanged {
                                tab_id: tab.id,
                                title,
                            });
                        }
                    }
                }
                
                // Run content scripts
                if let Some(ref doc) = self.document {
                    let scripts = self.extension_manager.get_content_scripts(&url);
                    for script in scripts {
                        if let Err(err) = self.script_engine.execute_with_dom(&script, doc) {
                            eprintln!("Content script error: {}", err);
                        }
                    }

                    // Process any popups requested by scripts
                    let popup_requests = self.script_engine.take_popup_requests();
                    if !popup_requests.is_empty() {
                        self.handle_popup_requests(popup_requests, &url);
                    }
                }
                
                // Style and layout
                self.restyle();
                self.relayout();
                
                // Build clickable regions for hit testing
                self.build_clickable_regions();
                
                // Reset scroll
                self.scroll_x = 0;
                self.scroll_y = 0;
                
                // Navigation complete
                if let Some(tab) = self.ui.tab_manager.active_tab_mut() {
                    tab.loading = false;
                }
                
                self.network_bar.complete_request(req_id);
                
                self.ui.broadcast(SyncEvent::NavCompleted {
                    tab_id: self.ui.tab_manager.active_tab().map(|t| t.id).unwrap_or(0),
                    url: url.clone(),
                });
            }
            Err(e) => {
                eprintln!("Navigation error: {}", e);
                self.network_bar.fail_request(req_id);
                self.show_error(&format!("Failed to load: {}\n\n{}", url, e));
                
                if let Some(tab) = self.ui.tab_manager.active_tab_mut() {
                    tab.loading = false;
                }
                
                self.ui.broadcast(SyncEvent::NavFailed {
                    tab_id: self.ui.tab_manager.active_tab().map(|t| t.id).unwrap_or(0),
                    url: url.clone(),
                    error: e.to_string(),
                });
            }
        }
        
        self.loading = false;
        self.popup_handler.page_loaded();
        self.needs_repaint = true;
    }
    
    /// Submit a form via POST
    fn submit_form_post(&mut self, url: &str, body: &str, content_type: &str) {
        self.loading = true;
        self.current_url = url.to_string();
        
        // Notify popup handler page is loading
        self.popup_handler.page_loading();
        
        // Update input manager address bar
        self.input.set_address(url);
        
        // Reset sandbox for new navigation
        if let Some(tab) = self.ui.tab_manager.active_tab_mut() {
            tab.navigate(url);
            self.sandbox_manager.create(tab.id, url.to_string());
        }
        
        // Start network tracking
        let req_id = self.next_request_id();
        self.network_bar.start_request(req_id, url, "POST");
        
        // Add to history
        if self.history_index < self.history.len() {
            self.history.truncate(self.history_index);
        }
        self.history.push(url.to_string());
        self.history_index = self.history.len();
        
        // Perform POST request
        self.network_bar.update_request(req_id, RequestState::Waiting);
        
        let parsed_url = match url::Url::parse(url) {
            Ok(u) => u,
            Err(e) => {
                self.show_error(&format!("Invalid URL: {}", e));
                self.loading = false;
                return;
            }
        };
        
        let options = crate::protocol::FetchOptions::post(body)
            .with_header("Content-Type", content_type);
        
        match self.http_client.fetch_with_options(&parsed_url, &options) {
            Ok(response) => {
                self.network_bar.update_request(req_id, RequestState::Receiving);
                self.network_bar.add_bytes(req_id, response.body.len() as u64);
                
                let html = response.body.clone();
                
                // Parse HTML using renderer
                self.renderer.parse_html(&html);
                self.document = Some(self.renderer.document.clone());
                
                // Extract title
                if let Some(ref doc) = self.document {
                    if let Some(title) = Self::extract_title(doc) {
                        if let Some(tab) = self.ui.tab_manager.active_tab_mut() {
                            tab.title = title.clone();
                        }
                    }
                }
                
                // Style and layout
                self.restyle();
                self.relayout();
                self.build_clickable_regions();
                
                // Reset scroll
                self.scroll_x = 0;
                self.scroll_y = 0;
                
                if let Some(tab) = self.ui.tab_manager.active_tab_mut() {
                    tab.loading = false;
                }
                
                self.network_bar.complete_request(req_id);
            }
            Err(e) => {
                eprintln!("Form submission error: {}", e);
                self.network_bar.fail_request(req_id);
                self.show_error(&format!("Failed to submit form: {}\n\n{}", url, e));
                
                if let Some(tab) = self.ui.tab_manager.active_tab_mut() {
                    tab.loading = false;
                }
            }
        }
        
        self.loading = false;
        self.popup_handler.page_loaded();
        self.needs_repaint = true;
    }
    
    /// Normalize URL input
    fn normalize_url(&self, input: &str) -> String {
        let input = input.trim();
        
        // Already a URL?
        if input.starts_with("http://") || input.starts_with("https://") 
           || input.starts_with("about:") || input.starts_with("file://") {
            return input.to_string();
        }
        
        // Looks like a domain?
        if input.contains('.') && !input.contains(' ') {
            return format!("https://{}", input);
        }
        
        // Search query
        format!("https://duckduckgo.com/?q={}", urlencoding::encode(input))
    }
    
    fn extract_title(doc: &Document) -> Option<String> {
        fn find_title(node: &crate::dom::Node) -> Option<String> {
            if node.node_type == NodeType::Element
                && node.tag_name.as_deref() == Some("title") {
                    // Get text content
                    for child_ref in &node.children {
                        let child = child_ref.borrow();
                        if child.node_type == NodeType::Text {
                            if let Some(ref text) = child.text_content {
                                return Some(text.trim().to_string());
                            }
                        }
                    }
                }
            for child_ref in &node.children {
                let child = child_ref.borrow();
                if let Some(title) = find_title(&child) {
                    return Some(title);
                }
            }
            None
        }
        find_title(&doc.root.borrow())
    }
    
    fn show_error(&mut self, message: &str) {
        let html = format!(r#"
            <!DOCTYPE html>
            <html>
            <head><title>Error</title></head>
            <body style="font-family: system-ui; padding: 40px; background: #0d1117; color: #e6edf3;">
                <h1 style="color: #f85149;">âš ï¸ Error</h1>
                <pre style="background: #161b22; padding: 20px; border-radius: 8px; overflow: auto;">{}</pre>
                <p><a href="about:blank" style="color: #58a6ff;">Go to blank page</a></p>
            </body>
            </html>
        "#, message);
        
        // Parse HTML using renderer
        self.renderer.parse_html(&html);
        self.document = Some(self.renderer.document.clone());
        self.restyle();
        self.relayout();
        self.build_clickable_regions();
    }
    
    fn restyle(&mut self) {
        // Compute styles for all nodes
        self.computed_styles.clear();
        if let Some(ref doc) = self.document {
            self.compute_styles_recursive(&doc.root.clone());
        }
    }
    
    fn compute_styles_recursive(&mut self, node: &NodeRef) {
        let key: *const std::cell::RefCell<crate::dom::Node> = std::rc::Rc::as_ptr(node);
        let style = self.style_engine.compute_style(node);
        self.computed_styles.insert(key, style);
        
        for child in &node.borrow().children {
            self.compute_styles_recursive(child);
        }
    }
    
    fn relayout(&mut self) {
        if let Some(ref doc) = self.document {
            let content_rect = self.ui.content_rect();
            self.layout_engine.viewport_width = content_rect.width as f32;
            self.layout_engine.viewport_height = content_rect.height as f32;
            self.layout = Some(self.layout_engine.layout(&doc.root, &self.computed_styles));
            
            // Calculate max scroll
            if let Some(ref layout) = self.layout {
                let content_height = layout.bounds.height;
                self.max_scroll_y = (content_height - content_rect.height as f32).max(0.0) as i32;
            }
        }
    }
    
    /// Build list of clickable regions from layout tree
    fn build_clickable_regions(&mut self) {
        self.clickable_regions.clear();
        clear_clickable_nodes();
        
        if let Some(layout) = self.layout.as_ref() {
            let ptr: *const LayoutBox = layout;
            unsafe {
                self.collect_clickable_regions(&*ptr, 0.0, 0.0);
            }
        }
    }
    
    fn collect_clickable_regions(&mut self, layout: &LayoutBox, offset_x: f32, offset_y: f32) {
        let x = offset_x + layout.content.x;
        let y = offset_y + layout.content.y;
        let w = layout.content.width;
        let h = layout.content.height;
        
        // Check if this is a link
        if let Some(ref node_ref) = layout.node {
            // Extract info while borrowing, then act
            let (tag, href, input_type, src, alt) = {
                let node = node_ref.borrow();
                if node.node_type != NodeType::Element {
                    (None, None, None, None, None)
                } else {
                    let tag = node.tag_name.clone();
                    let href = node.attributes.get("href").cloned();
                    let input_type = node.attributes.get("type").cloned();
                    let src = node.attributes.get("src").cloned();
                    let alt = node.attributes.get("alt").cloned();
                    (tag, href, input_type, src, alt)
                }
            };
            
            if let Some(ref tag) = tag {
                match tag.as_str() {
                    "a" => {
                        if let Some(href) = href {
                            let text = Self::get_text_content_from_ref(node_ref);
                            self.clickable_regions.push(ClickableRegion {
                                x, y, width: w, height: h,
                                target: HitTestResult::Link { url: href, text },
                            });
                        }
                    }
                    "input" => {
                        let element_id = register_clickable_node(node_ref);
                        let itype = input_type.as_deref().unwrap_or("text");
                        if itype == "submit" || itype == "button" {
                            self.clickable_regions.push(ClickableRegion {
                                x, y, width: w, height: h,
                                target: HitTestResult::FormButton { element_id },
                            });
                        } else {
                            self.clickable_regions.push(ClickableRegion {
                                x, y, width: w, height: h,
                                target: HitTestResult::FormInput { element_id },
                            });
                        }
                    }
                    "button" => {
                        let element_id = register_clickable_node(node_ref);
                        self.clickable_regions.push(ClickableRegion {
                            x, y, width: w, height: h,
                            target: HitTestResult::FormButton { element_id },
                        });
                    }
                    "img" => {
                        self.clickable_regions.push(ClickableRegion {
                            x, y, width: w, height: h,
                            target: HitTestResult::Image { 
                                src: src.unwrap_or_default(), 
                                alt: alt.unwrap_or_default(),
                            },
                        });
                    }
                    _ => {}
                }
            }
        }
        
        // Recurse into children
        for child in &layout.children {
            self.collect_clickable_regions(child, x, y);
        }
    }
    
    fn get_text_content_from_ref(node_ref: &NodeRef) -> String {
        let mut text = String::new();
        Self::collect_text_from_ref(node_ref, &mut text);
        text.trim().to_string()
    }
    
    fn collect_text_from_ref(node_ref: &NodeRef, out: &mut String) {
        let node = node_ref.borrow();
        if node.node_type == NodeType::Text {
            if let Some(ref t) = node.text_content {
                out.push_str(t);
            }
        } else {
            for child in &node.children {
                Self::collect_text_from_ref(child, out);
            }
        }
    }
    
    /// Hit test at content coordinates
    fn hit_test(&self, content_x: f32, content_y: f32) -> HitTestResult {
        // Add scroll offset
        let x = content_x + self.scroll_x as f32;
        let y = content_y + self.scroll_y as f32;
        
        for region in &self.clickable_regions {
            if x >= region.x && x < region.x + region.width &&
               y >= region.y && y < region.y + region.height {
                return region.target.clone();
            }
        }
        
        HitTestResult::Nothing
    }

    /// Handle popup requests (e.g., window.open) with sandbox gating
    fn handle_popup_requests(&mut self, requests: Vec<String>, opener_url: &str) {
        let sandbox_allowed = self.ui.tab_manager.active_tab()
            .map(|t| t.sandbox.can_perform(SandboxAction::Popup))
            .unwrap_or(true);
        
        for popup_url in requests {
            let decision = self.popup_manager.evaluate(
                &popup_url,
                opener_url,
                PopupReason::WindowOpen,
                sandbox_allowed,
            );
            
            if decision.allow {
                let new_tab_id = self.ui.tab_manager.create_tab(popup_url.clone());
                self.ui.tab_manager.activate_tab(new_tab_id);
                self.navigate(&popup_url);
            } else {
                // Keep blocked popups pending for potential future UI surfacing
                println!("Popup blocked: {} ({})", popup_url, decision.reason);
            }
        }
    }
    
    fn go_back(&mut self) {
        if self.history_index > 1 {
            self.history_index -= 1;
            let url = self.history[self.history_index - 1].clone();
            self.navigate(&url);
        }
    }
    
    fn go_forward(&mut self) {
        if self.history_index < self.history.len() {
            self.history_index += 1;
            let url = self.history[self.history_index - 1].clone();
            self.navigate(&url);
        }
    }
    
    fn can_go_back(&self) -> bool {
        self.history_index > 1
    }
    
    fn can_go_forward(&self) -> bool {
        self.history_index < self.history.len()
    }
    
    fn refresh(&mut self) {
        let url = self.current_url.clone();
        if !url.is_empty() {
            self.navigate(&url);
        }
    }
    
    fn scroll(&mut self, delta: i32) {
        let old_scroll = self.scroll_y;
        self.scroll_y = (self.scroll_y + delta).clamp(0, self.max_scroll_y);
        
        // Record scroll interaction for sandbox
        if (self.scroll_y - old_scroll).abs() > 0 {
            if let Some(tab) = self.ui.tab_manager.active_tab_mut() {
                tab.sandbox.record_interaction(MeaningfulInteraction::Scroll {
                    distance: delta.abs(),
                });
            }
        }
        
        self.needs_repaint = true;
        
        // Broadcast scroll position
        if let Some(tab) = self.ui.tab_manager.active_tab() {
            let content = self.ui.content_rect();
            self.ui.broadcast(SyncEvent::PageScroll {
                tab_id: tab.id,
                x: self.scroll_x,
                y: self.scroll_y,
                max_x: 0,
                max_y: self.max_scroll_y,
            });
        }
    }
    
    fn resize(&mut self, width: u32, height: u32) {
        self.ui.resize(width, height);
        self.ui_renderer.resize(width, height);
        self.painter = Painter::new(width, height);
        self.relayout();
        self.update_ui_bounds();
        self.needs_repaint = true;
    }
    
    /// Update UI bounds for hit testing
    fn update_ui_bounds(&mut self) {
        let content = self.ui.content_rect();
        let left_width = self.ui.sidebar_layout.left_size();
        let right_width = self.ui.sidebar_layout.right_size();
        let top_height = self.ui.sidebar_layout.top_size();
        let btn_size = 32;
        let btn_y = ((top_height as i32 - btn_size).max(0)) / 2;
        let mut x = left_width as i32 + 8;
        let nav_width = (self.ui.width.saturating_sub(left_width + right_width)) as i32;
        let nav_x = left_width as i32;
        
        self.ui_bounds.back_button = InputRect::new(x, btn_y, btn_size, btn_size);
        x += btn_size + 4;
        
        self.ui_bounds.forward_button = InputRect::new(x, btn_y, btn_size, btn_size);
        x += btn_size + 4;
        
        self.ui_bounds.refresh_button = InputRect::new(x, btn_y, btn_size, btn_size);
        x += btn_size + 12;
        
        // Right-aligned controls
        let network_width = 100;
        let network_x = nav_x + nav_width - (network_width + 10);
        let mut address_right = network_x - 8;
        if self.ui.ai.show_help_button {
            let help_x = network_x - btn_size - 8;
            self.ui_bounds.help_button = InputRect::new(help_x, btn_y, btn_size, btn_size);
            address_right = help_x - 8;
        } else {
            self.ui_bounds.help_button = InputRect::new(0, 0, 0, 0);
        }
        
        // Address bar takes remaining width up to help/network area
        let addr_width = (address_right - x).max(200);
        self.ui_bounds.address_bar = InputRect::new(x, btn_y, addr_width, btn_size);
        
        // Network bar on right of top bar
        self.ui_bounds.network_bar = InputRect::new(
            network_x,
            btn_y,
            network_width,
            btn_size,
        );
        
        // Content area
        self.ui_bounds.content_area = InputRect::new(
            content.x as i32,
            content.y as i32,
            content.width as i32,
            content.height as i32,
        );
        
        // Tab list
        self.ui_bounds.tab_list = InputRect::new(
            0,
            top_height as i32,
            left_width as i32,
            (self.ui.height - top_height) as i32,
        );
    }
    
    fn render(&mut self, buffer: &mut [u32]) {
        let content_rect = self.ui.content_rect();
        let theme = self.ui.theme_manager.current();
        
        // Clear with background color
        let bg = crate::ui::render::hex_to_u32(&theme.colors.background);
        buffer.fill(bg);
        
        // Render page content in content area
        if let Some(ref layout) = self.layout {
            let offset_x = content_rect.x as i32 - self.scroll_x;
            let offset_y = content_rect.y as i32 - self.scroll_y;
            self.painter.paint_into(layout, buffer, self.ui.width, offset_x, offset_y);
        }
        
        // Draw sidebars (backgrounds)
        for edge in Edge::all() {
            self.ui_renderer.draw_sidebar(buffer, edge, &self.ui.sidebar_layout, theme);
        }
        
        // Draw navigation bar in top sidebar
        if let Some(top) = self.ui.sidebar_layout.get(Edge::Top) {
            if top.is_visible() {
                let bounds = top.bounds(self.ui.width, self.ui.height, &self.ui.sidebar_layout);
                let loading = self.loading;
                let can_back = self.can_go_back();
                let can_forward = self.can_go_forward();
                
                // Draw nav bar with address bar text
                let display_url = if self.input.focus == Focus::AddressBar {
                    &self.input.address_bar.text
                } else {
                    &self.current_url
                };
                
                self.ui_renderer.draw_nav_bar(
                    buffer,
                    bounds,
                    theme,
                    display_url,
                    crate::ui::render::NavBarState {
                        can_back,
                        can_forward,
                        loading,
                        show_help_button: self.ui.ai.show_help_button,
                        help_enabled: self.ui.ai.enabled,
                        help_open: self.ui.help_pane_open,
                    },
                );
                
                // Draw cursor in address bar if focused
                if self.input.focus == Focus::AddressBar {
                    self.draw_address_cursor(buffer, theme);
                }
                
                // Draw network activity indicator with detailed status
                let net_bounds = crate::ui::Rect {
                    x: self.ui_bounds.network_bar.x as u32,
                    y: self.ui_bounds.network_bar.y as u32,
                    width: self.ui_bounds.network_bar.width as u32,
                    height: self.ui_bounds.network_bar.height as u32,
                };
                let is_dark = theme.meta.name.contains("Dark");
                self.ui_renderer.draw_network_bar_detailed(buffer, net_bounds, &self.network_bar, is_dark);
            }
        }
        
        // Draw trust indicator for current tab
        self.draw_trust_indicator(buffer, theme);
        
        // Draw tab list in left sidebar
        if let Some(left) = self.ui.sidebar_layout.get(Edge::Left) {
            if left.is_expanded() {
                let bounds = left.bounds(self.ui.width, self.ui.height, &self.ui.sidebar_layout);
                self.ui_renderer.draw_tab_list(buffer, bounds, &self.ui.tab_manager, theme);
            }
        }

        // Draw help pane content in right sidebar when enabled
        if self.ui.help_pane_open && self.ui.ai.enabled {
            if let Some(right) = self.ui.sidebar_layout.get(Edge::Right) {
                if right.is_visible() {
                    let bounds = right.bounds(self.ui.width, self.ui.height, &self.ui.sidebar_layout);
                    self.ui_renderer.draw_help_pane(
                        buffer,
                        bounds,
                        theme,
                        &self.ui.ai,
                        self.ui.help_response.as_deref(),
                        self.ui.help_error.as_deref(),
                    );
                }
            }
        }
        
        // Draw tab tile overlay if active
        self.ui_renderer.draw_tab_tiles(buffer, &self.ui.tab_manager, theme, content_rect);
        
        // Draw sync status
        if let Some(ref server) = self.ui.sync_server {
            if server.is_running() {
                let x = (self.ui.width - 140) as i32;
                self.ui_renderer.draw_sync_status(buffer, x, 10, server.client_count(), theme);
            }
        }
        
        // Capture preview periodically
        if self.last_frame.elapsed() > Duration::from_secs(5) {
            if let Some(tab) = self.ui.tab_manager.active_tab() {
                let tab_id = tab.id;
                self.ui.tab_manager.capture_preview(tab_id, buffer, self.ui.width, self.ui.height);
            }
            self.last_frame = Instant::now();
        }
    }
    
    /// Draw text cursor in address bar
    fn draw_address_cursor(&self, buffer: &mut [u32], theme: &Theme) {
        let bounds = &self.ui_bounds.address_bar;
        let cursor_pos = self.input.address_bar.cursor;
        
        // Approximate cursor x position (assuming ~8px per char)
        let char_width = 8;
        let cursor_x = bounds.x + 8 + (cursor_pos as i32 * char_width);
        let cursor_y = bounds.y + 6;
        let cursor_height = bounds.height - 12;
        
        // Draw cursor line
        let cursor_color = 0xffffffff;
        for y in cursor_y..(cursor_y + cursor_height) {
            if y >= 0 && (y as u32) < self.ui.height {
                let idx = (y as u32 * self.ui.width + cursor_x as u32) as usize;
                if idx < buffer.len() {
                    buffer[idx] = cursor_color;
                }
            }
        }
    }
    
    /// Draw network activity bar
    fn draw_network_bar(&self, buffer: &mut [u32], theme: &Theme) {
        let bounds = &self.ui_bounds.network_bar;
        let activity = self.network_bar.activity_level();
        
        // Background
        let bg_color = if theme.meta.name.contains("Dark") { 0xff2a2a2a } else { 0xffe0e0e0 };
        for y in bounds.y..(bounds.y + bounds.height) {
            for x in bounds.x..(bounds.x + bounds.width) {
                if x >= 0 && y >= 0 && (x as u32) < self.ui.width && (y as u32) < self.ui.height {
                    let idx = (y as u32 * self.ui.width + x as u32) as usize;
                    if idx < buffer.len() {
                        buffer[idx] = bg_color;
                    }
                }
            }
        }
        
        // Activity bar
        if activity > 0.0 {
            let bar_width = ((bounds.width - 4) as f32 * activity) as i32;
            let bar_color = if self.network_bar.is_active { 0xff4a9eff } else { 0xff44ff44 };
            
            for y in (bounds.y + 2)..(bounds.y + bounds.height - 2) {
                for x in (bounds.x + 2)..(bounds.x + 2 + bar_width) {
                    if x >= 0 && y >= 0 && (x as u32) < self.ui.width && (y as u32) < self.ui.height {
                        let idx = (y as u32 * self.ui.width + x as u32) as usize;
                        if idx < buffer.len() {
                            buffer[idx] = bar_color;
                        }
                    }
                }
            }
        }
    }
    
    /// Draw sandbox trust indicator
    fn draw_trust_indicator(&self, buffer: &mut [u32], theme: &Theme) {
        if let Some(tab) = self.ui.tab_manager.active_tab() {
            let trust_color = tab.trust_color();
            let trust_text = tab.trust_text();
            
            // Draw small indicator dot in top-right of content area
            let content = self.ui.content_rect();
            let dot_x = content.x as i32 + content.width as i32 - 80;
            let dot_y = content.y as i32 + 5;
            
            // Draw dot
            let radius = 5;
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx*dx + dy*dy <= radius*radius {
                        let x = dot_x + dx;
                        let y = dot_y + dy;
                        if x >= 0 && y >= 0 && (x as u32) < self.ui.width && (y as u32) < self.ui.height {
                            let idx = (y as u32 * self.ui.width + x as u32) as usize;
                            if idx < buffer.len() {
                                buffer[idx] = trust_color;
                            }
                        }
                    }
                }
            }
            
            // Draw short label with trust status next to the dot
            self.ui_renderer.draw_text(buffer, trust_text, dot_x + radius + 6, dot_y - 2, 12.0, trust_color);
            
            // Show interactions needed if not trusted
            if tab.sandbox.trust_level != TrustLevel::Trusted && 
               tab.sandbox.trust_level != TrustLevel::Whitelisted {
                let needed = tab.sandbox.interactions_needed();
                let needed_text = format!("{}", needed.max(0));
                self.ui_renderer.draw_text(buffer, &needed_text, dot_x - 10, dot_y - 10, 11.0, 0xffffffff);
            }
        }
    }
    
    fn process_sync_commands(&mut self) {
        if let Some(ref server) = self.ui.sync_server {
            let mut pending = Vec::new();
            while let Some((client_id, cmd)) = server.poll_command() {
                pending.push((client_id, cmd));
            }
            for (client_id, cmd) in pending {
                self.handle_sync_command(client_id, cmd);
            }
        }
    }
    
    fn handle_sync_command(&mut self, client_id: u64, cmd: SyncCommand) {
        use crate::sync::protocol::{SyncMessage, BrowserState as SyncBrowserState, TabInfo};
        
        let success = match cmd {
            SyncCommand::TabCreate { url } => {
                self.ui.tab_manager.create_tab(url.clone());
                self.navigate(&url);
                true
            }
            SyncCommand::TabClose { tab_id } => {
                self.ui.tab_manager.close_tab(tab_id);
                true
            }
            SyncCommand::TabActivate { tab_id } => {
                self.ui.tab_manager.activate_tab(tab_id);
                if let Some(tab) = self.ui.tab_manager.get_tab(tab_id) {
                    let url = tab.url.clone();
                    self.navigate(&url);
                }
                true
            }
            SyncCommand::TabReload { .. } => {
                self.refresh();
                true
            }
            SyncCommand::NavBack => {
                self.go_back();
                true
            }
            SyncCommand::NavForward => {
                self.go_forward();
                true
            }
            SyncCommand::NavRefresh => {
                self.refresh();
                true
            }
            SyncCommand::NavGo { url } => {
                self.navigate(&url);
                true
            }
            SyncCommand::ScrollBy { dy, .. } => {
                self.scroll(dy);
                true
            }
            SyncCommand::ScrollTop => {
                self.scroll_y = 0;
                self.needs_repaint = true;
                true
            }
            SyncCommand::ScrollBottom => {
                self.scroll_y = self.max_scroll_y;
                self.needs_repaint = true;
                true
            }
            SyncCommand::StateRequest { .. } => {
                // Build and send current browser state
                let active_id = self.ui.tab_manager.active_tab().map(|a| a.id);
                let tabs: Vec<_> = self.ui.tab_manager.tabs().iter().enumerate().map(|(idx, t)| {
                    TabInfo {
                        id: t.id,
                        index: idx,
                        title: t.title.clone(),
                        url: t.url.clone(),
                        favicon_url: None,
                        loading: t.loading,
                        pinned: t.pinned,
                        muted: false,
                        audible: false,
                        group_id: None,
                        preview_available: false,
                    }
                }).collect();
                let state = SyncBrowserState {
                    browser_id: "sassy-browser".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    tabs,
                    active_tab_id: active_id,
                    bookmarks: Vec::new(),
                    downloads: Vec::new(),
                    settings: std::collections::HashMap::new(),
                };
                if let Some(ref server) = self.ui.sync_server {
                    server.send_state(client_id, state);
                }
                true
            }
            _ => { false }
        };
        
        // Send acknowledgment
        if let Some(ref server) = self.ui.sync_server {
            let ack = SyncMessage::ack(0, success, None);
            server.send_to_client(client_id, ack);
        }
    }
    
    /// Handle input action from InputManager
    fn handle_input_action(&mut self, action: InputAction) {
        match action {
            InputAction::Navigate(url) => {
                self.navigate(&url);
            }
            InputAction::GoBack => {
                self.go_back();
            }
            InputAction::GoForward => {
                self.go_forward();
            }
            InputAction::Reload => {
                self.refresh();
            }
            InputAction::ToggleHelpPane => {
                if self.ui.ai.enabled && self.ui.ai.show_help_button {
                    let was_open = self.ui.help_pane_open;
                    if was_open {
                        if let Some(prev) = self.ui.help_sidebar_previous.take() {
                            if let Some(right) = self.ui.sidebar_layout.get_mut(Edge::Right) {
                                right.state = prev;
                            }
                        }
                        self.ui.help_pane_open = false;
                        self.help_rx = None;
                        self.ui.help_inflight = false;
                    } else {
                        let current_state = self.ui.sidebar_layout.get(Edge::Right)
                            .map(|s| s.state.clone())
                            .unwrap_or(SidebarState::Collapsed);
                        self.ui.help_sidebar_previous = Some(current_state);
                        if let Some(right) = self.ui.sidebar_layout.get_mut(Edge::Right) {
                            right.expand();
                        }
                        self.ui.help_pane_open = true;
                        self.ui.help_response = None;
                        self.ui.help_error = None;
                        self.ui.help_inflight = true;

                        let (tx, rx) = mpsc::channel();
                        let runtime = self.ai_runtime.clone();
                        let url = self.current_url.clone();
                        std::thread::spawn(move || {
                            let result = run_help_query(&runtime, HelpQuery::ExplainPage { url });
                            let _ = tx.send(result);
                        });
                        self.help_rx = Some(rx);
                    }
                    self.update_ui_bounds();
                }
            }
            InputAction::ScrollUp(delta) => {
                self.scroll(-delta);
            }
            InputAction::ScrollDown(delta) => {
                self.scroll(delta);
            }
            InputAction::ScrollToTop => {
                self.scroll_y = 0;
                self.needs_repaint = true;
            }
            InputAction::ScrollToBottom => {
                self.scroll_y = self.max_scroll_y;
                self.needs_repaint = true;
            }
            InputAction::NewTab => {
                self.ui.tab_manager.create_tab("about:blank".into());
            }
            InputAction::CloseTab => {
                if let Some(tab) = self.ui.tab_manager.active_tab() {
                    let id = tab.id;
                    self.ui.tab_manager.close_tab(id);
                }
            }
            InputAction::NextTab => {
                self.ui.tab_manager.select_next();
            }
            InputAction::PrevTab => {
                self.ui.tab_manager.select_prev();
            }
            InputAction::ToggleTileView => {
                self.ui.tab_manager.toggle_tile_view();
            }
            InputAction::PageClick(x, y) => {
                self.handle_page_click(x, y);
            }
            InputAction::FocusAddressBar => {
                self.input.address_bar.set_text(&self.current_url);
                self.input.address_bar.select_all();
            }
            InputAction::AllowPendingPopup => {
                if let Some(url) = self.popup_manager.allow_pending(0) {
                    let new_tab_id = self.ui.tab_manager.create_tab(url.clone());
                    self.ui.tab_manager.activate_tab(new_tab_id);
                    self.navigate(&url);
                }
            }
            InputAction::BlockPendingPopup => {
                self.popup_manager.block_pending(0);
            }
            InputAction::AllowDomainPopups(domain) => {
                self.popup_manager.allow_domain(&domain);
            }
            InputAction::BlockDomainPopups(domain) => {
                self.popup_manager.block_domain(&domain);
            }
            _ => {}
        }
        
        self.needs_repaint = true;
    }
    
    /// Handle click in page content
    fn handle_page_click(&mut self, x: i32, y: i32) {
        let hit = self.hit_test(x as f32, y as f32);
        
        match hit {
            HitTestResult::Link { url, .. } => {
                // Record link click as meaningful interaction
                if let Some(tab) = self.ui.tab_manager.active_tab_mut() {
                    tab.sandbox.record_interaction(MeaningfulInteraction::LinkClick);
                    // Also record in page sandbox manager
                    self.sandbox_manager.record(tab.id, Interaction::Click {
                        x,
                        y,
                        element_width: 100,  // Estimate for link
                        element_height: 20,
                        element_type: "a".to_string(),
                        timestamp: Instant::now(),
                    });
                }
                
                // Resolve relative URLs
                let full_url = if url.starts_with("http") || url.starts_with("about:") {
                    url
                } else if url.starts_with("/") {
                    // Absolute path
                    if let Ok(base) = url::Url::parse(&self.current_url) {
                        format!("{}://{}{}", base.scheme(), base.host_str().unwrap_or(""), url)
                    } else {
                        url
                    }
                } else {
                    // Relative path
                    if let Ok(base) = url::Url::parse(&self.current_url) {
                        base.join(&url).map(|u| u.to_string()).unwrap_or(url)
                    } else {
                        url
                    }
                };
                
                self.navigate(&full_url);
            }
            HitTestResult::FormInput { .. } => {
                // Would focus the input
                if let Some(tab) = self.ui.tab_manager.active_tab_mut() {
                    tab.sandbox.record_interaction(MeaningfulInteraction::FormInput);
                }
            }
            HitTestResult::FormButton { element_id } => {
                // Submit form
                if let Some(tab) = self.ui.tab_manager.active_tab_mut() {
                    tab.sandbox.record_interaction(MeaningfulInteraction::FormSubmit);
                }
                
                // Find the button node and its parent form
                if let Some(button_node) = get_clickable_node(element_id) {
                    if let Some(form_node) = crate::dom::Node::find_parent_form(&button_node) {
                        let form_data = crate::dom::FormData::from_form(&form_node);
                        let base_url = &self.current_url;
                        
                        if form_data.method == "GET" {
                            // GET submission - navigate to URL with query params
                            let url = form_data.get_url(base_url);
                            self.navigate(&url);
                        } else {
                            // POST submission - send form data
                            let url = form_data.get_url(base_url);
                            let body = form_data.to_urlencoded();
                            self.submit_form_post(&url, &body, &form_data.enctype);
                        }
                    }
                }
            }
            _ => {}
        }
        
        // Record popup interaction timing
        self.popup_manager.record_interaction(InteractionType::Click);
    }
    
    /// Update cursor based on hover position
    fn update_cursor(&mut self, x: i32, y: i32) -> CursorIcon {
        // Check UI elements first
        if self.ui_bounds.address_bar.contains(x, y) {
            return CursorIcon::Text;
        }
        if self.ui_bounds.back_button.contains(x, y) && self.can_go_back() {
            return CursorIcon::Pointer;
        }
        if self.ui_bounds.forward_button.contains(x, y) && self.can_go_forward() {
            return CursorIcon::Pointer;
        }
        if self.ui_bounds.refresh_button.contains(x, y) {
            return CursorIcon::Pointer;
        }
        if self.ui.ai.show_help_button && self.ui.ai.enabled && self.ui_bounds.help_button.contains(x, y) {
            return CursorIcon::Pointer;
        }
        
        // Check content area
        if self.ui_bounds.content_area.contains(x, y) {
            let content_x = x - self.ui_bounds.content_area.x;
            let content_y = y - self.ui_bounds.content_area.y;
            
            let hit = self.hit_test(content_x as f32, content_y as f32);
            match hit {
                HitTestResult::Link { url, .. } => {
                    self.hovered_link = Some(url);
                    return CursorIcon::Pointer;
                }
                HitTestResult::FormInput { .. } => {
                    return CursorIcon::Text;
                }
                _ => {}
            }
        }
        
        self.hovered_link = None;
        CursorIcon::Default
    }
    
    fn handle_char_input(&mut self, c: char) {
        let action = self.input.char_input(c);
        self.handle_input_action(action);
    }
    
    fn handle_key(&mut self, key: crate::ui::input::Key, pressed: bool) {
        if !pressed {
            return;
        }
        
        let action = self.input.key_press(key);
        self.handle_input_action(action);
        
        // Also let UI handle for tile view etc
        let modifiers = Modifiers {
            ctrl: self.input.ctrl_held,
            alt: self.input.alt_held,
            shift: self.input.shift_held,
            meta: false,
        };
        
        let ui_key = match key {
            crate::ui::input::Key::Escape => Key::Escape,
            crate::ui::input::Key::Enter => Key::Enter,
            crate::ui::input::Key::Tab => Key::Tab,
            crate::ui::input::Key::Left => Key::Left,
            crate::ui::input::Key::Right => Key::Right,
            crate::ui::input::Key::Up => Key::Up,
            crate::ui::input::Key::Down => Key::Down,
            crate::ui::input::Key::Delete => Key::Delete,
            crate::ui::input::Key::W => Key::W,
            crate::ui::input::Key::T => Key::T,
            _ => Key::Unknown,
        };
        
        if !self.ui.tab_manager.tile_view_active {
            self.ui.key_press(ui_key, modifiers);
        }
    }
    
    fn handle_mouse_click(&mut self, x: i32, y: i32, button: MouseButton) {
        if button != MouseButton::Left {
            return;
        }
        
        let action = self.input.mouse_click(x, y, &self.ui_bounds);
        self.handle_input_action(action);
        
        // Also handle UI clicks
        self.ui.mouse_down(x, y, button);
    }
    
    fn handle_mouse_move(&mut self, x: i32, y: i32) {
        self.input.mouse_x = x;
        self.input.mouse_y = y;
        self.ui.mouse_move(x, y);
        
        // Update cursor
        self.current_cursor = self.update_cursor(x, y);
    }
    
    fn handle_mouse_scroll(&mut self, delta: f32) {
        let action = self.input.mouse_scroll(delta);
        self.handle_input_action(action);
    }
    
    /// Check for completed async help queries without blocking the UI
    fn poll_help_channel(&mut self) {
        if let Some(rx) = self.help_rx.as_ref() {
            match rx.try_recv() {
                Ok(result) => {
                    self.help_rx = None;
                    self.ui.help_inflight = false;
                    match result {
                        Ok(text) => {
                            self.ui.help_response = Some(text);
                            self.ui.help_error = None;
                        }
                        Err(e) => {
                            self.ui.help_response = None;
                            self.ui.help_error = Some(e);
                        }
                    }
                    self.needs_repaint = true;
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    self.help_rx = None;
                    self.ui.help_inflight = false;
                    if self.ui.help_error.is_none() {
                        self.ui.help_error = Some("Help request ended unexpectedly".to_string());
                    }
                    self.needs_repaint = true;
                }
            }
        }
    }
    
    /// Update per frame
    fn update(&mut self) {
        self.poll_help_channel();
        self.check_easter_eggs();
        self.check_for_updates();
        // Update network bar animation
        self.network_bar.update();
        
        // Poll timers and execute ready callbacks
        self.poll_timers();
        
        // Check for DOM mutations and re-layout if needed
        self.check_dom_mutations();
        
        // Update focus time tracking
        if let Some(tab) = self.ui.tab_manager.active_tab_mut() {
            if tab.sandbox.focus_start.is_none() {
                tab.sandbox.focus_gained();
            }
            // If sandbox now allows popups, release any previously queued popups
            if tab.sandbox.can_perform(SandboxAction::Popup) {
                let pending = self.popup_manager.allow_all_pending();
                if !pending.is_empty() {
                    for popup_url in pending {
                        let new_tab_id = self.ui.tab_manager.create_tab(popup_url.clone());
                        self.ui.tab_manager.activate_tab(new_tab_id);
                        self.navigate(&popup_url);
                    }
                }
            }
        }
    }
    
    /// Poll timers and execute any that are ready
    fn poll_timers(&mut self) {
        // Get ready timers from script engine
        let ready_timers = self.script_engine.get_ready_timers();
        
        for (id, callback, repeat) in ready_timers {
            // Execute the callback
            if let Err(e) = self.script_engine.execute_callback(callback.clone(), vec![]) {
                eprintln!("Timer callback error: {}", e);
            }
            
            if repeat {
                // Reset the timer for next interval
                self.script_engine.reset_timer(id);
            } else {
                // Remove one-shot timer
                self.script_engine.remove_timer(id);
            }
        }
    }
    
    /// Check for DOM mutations and trigger re-layout
    fn check_dom_mutations(&mut self) {
        if self.script_engine.check_dom_mutation() {
            // DOM was mutated, need to re-layout and repaint
            self.relayout();
            self.needs_repaint = true;
        }
    }
    
    /// Check and award easter eggs based on browser state
    fn check_easter_eggs(&mut self) {
        // Night owl: browsing between 2am and 5am (UTC approximation)
        if let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            let secs = now.as_secs();
            let hour = ((secs % 86400) / 3600) as u32;
            if (2..5).contains(&hour) {
                if let Some(reward) = self.ui.ai.discover_easter_egg("night_owl") {
                    println!("ðŸ¦‰ Easter Egg: {}", reward.message);
                    println!("   Redeem at: {}", reward.redeem_url);
                }
            }
        }
        
        // First popup blocked
        if self.popup_manager.total_blocked == 1 {
            if let Some(reward) = self.ui.ai.discover_easter_egg("first_block") {
                println!("ðŸ›¡ï¸ Easter Egg: {}", reward.message);
                println!("   Redeem at: {}", reward.redeem_url);
            }
        }
        
        // Trust watcher: if user reached Trusted level
        if let Some(tab) = self.ui.tab_manager.active_tab() {
            if tab.sandbox.trust_level == TrustLevel::Trusted {
                if let Some(reward) = self.ui.ai.discover_easter_egg("trust_watcher") {
                    println!("ðŸ”’ Easter Egg: {}", reward.message);
                    println!("   Redeem at: {}", reward.redeem_url);
                }
            }
        }
    }
    
    /// Check for browser updates periodically
    fn check_for_updates(&mut self) {
        if self.update_checker.should_check() {
            let status = self.update_checker.check();
            match status {
                UpdateStatus::Available(ref info) => {
                    println!("ðŸ“¦ Update available: v{}", info.version);
                    println!("   Changelog: {}", info.changelog);
                    if info.required {
                        println!("   âš ï¸ This is a security update - please install soon!");
                    }
                }
                UpdateStatus::UpToDate => {
                    println!("âœ“ Sassy Browser is up to date");
                }
                UpdateStatus::Error(ref e) => {
                    eprintln!("Update check failed: {}", e);
                }
                UpdateStatus::Checking => {}
            }
        }
    }
    
    /// Get current update status
    pub fn get_update_status(&self) -> &UpdateStatus {
        self.update_checker.status()
    }
    
    /// Download available update
    pub fn download_update(&self) -> Result<std::path::PathBuf, String> {
        match self.update_checker.status() {
            UpdateStatus::Available(info) => {
                println!("â¬‡ï¸ Downloading update v{}...", info.version);
                let path = self.update_checker.download(info)?;
                println!("âœ… Downloaded to: {:?}", path);
                Ok(path)
            }
            UpdateStatus::UpToDate => Err("Already up to date".to_string()),
            UpdateStatus::Checking => Err("Update check in progress".to_string()),
            UpdateStatus::Error(e) => Err(format!("Update error: {}", e)),
        }
    }

    /// Handle file download through quarantine system
    pub fn handle_download(&mut self, url: &str, filename: &str, data: Vec<u8>, content_type: &str) {
        // Check if download is allowed by sandbox
        if let Some(tab) = self.ui.tab_manager.active_tab() {
            if !self.sandbox_manager.check(tab.id, "download") {
                println!("â›” Download blocked: page not trusted enough");
                return;
            }
        }
        
        // Create quarantined file
        let file = QuarantinedFile::new(
            filename.to_string(),
            url.to_string(),
            content_type.to_string(),
            data,
        );
        
        let id = self.quarantine.add(file);
        println!("ðŸ“¥ Download quarantined: {} (id: {})", filename, id);
        println!("   Three interactions required to release");
        
        // Display warnings
        if let Some(q_file) = self.quarantine.get(&id) {
            for warning in &q_file.warnings {
                let icon = match warning.level {
                    crate::sandbox::WarningLevel::Info => "â„¹ï¸",
                    crate::sandbox::WarningLevel::Caution => "âš ï¸",
                    crate::sandbox::WarningLevel::Warning => "ðŸ”¶",
                    crate::sandbox::WarningLevel::Danger => "ðŸ”´",
                };
                println!("   {} {}: {}", icon, warning.message, warning.detail);
            }
        }
    }
    
    /// Interact with quarantined file (required to release)
    pub fn quarantine_interact(&mut self, file_id: &str) {
        if let Some(file) = self.quarantine.get_mut(file_id) {
            file.interact(crate::sandbox::InteractionType::Acknowledge);
            match file.can_release() {
                ReleaseStatus::Ready => {
                    println!("âœ… File ready for release: {}", file.filename);
                }
                ReleaseStatus::NeedsInteraction { current, required } => {
                    println!("ðŸ”„ Progress: {}/{} interactions", current, required);
                }
                ReleaseStatus::Waiting { seconds_remaining } => {
                    println!("â³ Wait {} more seconds", seconds_remaining);
                }
                ReleaseStatus::Blocked { reason } => {
                    println!("ðŸš« Cannot release: {}", reason);
                }
            }
        }
    }
    
    /// Release quarantined file to downloads folder
    pub fn quarantine_release(&mut self, file_id: &str) -> Result<std::path::PathBuf, String> {
        let downloads = dirs::download_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        
        if let Some(file) = self.quarantine.get(file_id) {
            let path = file.release(downloads)?;
            self.quarantine.remove(file_id);
            println!("âœ… File released to: {}", path.display());
            Ok(path)
        } else {
            Err("File not found in quarantine".to_string())
        }
    }
    
    /// Save current session for restore on next launch
    fn save_session(&self) {
        use crate::data::{SessionRestore, TabState};
        
        let tabs: Vec<TabState> = self.ui.tab_manager.tabs().iter().map(|t| {
            TabState {
                id: t.id,
                url: t.url.clone(),
                title: t.title.clone(),
                scroll_x: 0,
                scroll_y: 0,
            }
        }).collect();
        
        let active_tab = self.ui.tab_manager.active_tab()
            .and_then(|active| self.ui.tab_manager.tabs().iter().position(|t| t.id == active.id));
        
        let session = SessionRestore {
            user_id: "default".to_string(),
            tabs,
            active_tab,
        };
        
        if let Err(e) = session.save() {
            eprintln!("Warning: Could not save session: {}", e);
        } else {
            println!("Session saved");
        }
    }
    
    /// Start secure sync server with Tailscale detection
    pub fn start_secure_sync(&mut self) {
        let config = SyncConfig::default();
        let mut server = SecureSyncServer::new(config);
        
        println!("ðŸ”’ Secure sync status:");
        println!("   Tailscale: {}", if server.tailscale.available { "detected" } else { "not found" });
        if let Some(ref hostname) = server.tailscale.hostname {
            println!("   Hostname: {}", hostname);
        }
        if let Some(ref magic_dns) = server.tailscale.magic_dns {
            println!("   MagicDNS: {}", magic_dns);
        }
        
        match server.start() {
            Ok(()) => {
                println!("   {}", server.connection_info());
                self.secure_sync = Some(server);
            }
            Err(e) => {
                eprintln!("   Failed to start: {}", e);
            }
        }
    }
    
    /// Register a new device in the family
    pub fn register_family_device(&mut self, device_id: &str, name: &str, owner: &str) {
        let device = self.family_config.request_device(
            device_id.to_string(),
            name.to_string(),
            owner.to_string(),
        );
        println!("ðŸ“± Device registered: {} ({})", device.name, device.trust_level.description());
        
        if self.family_config.devices.len() == 1 {
            // First device becomes admin
            self.family_config.bootstrap_admin(
                device_id.to_string(),
                name.to_string(),
                owner.to_string(),
            );
            println!("   Promoted to Admin (first device)");
        }
    }
    
    /// Approve a pending device
    pub fn approve_device(&mut self, device_id: &str, approver_id: &str) {
        match self.family_config.approve_device(device_id, approver_id, crate::sync::TrustLevel::Trusted) {
            Ok(()) => println!("âœ… Device {} approved", device_id),
            Err(e) => println!("âŒ Approval failed: {}", e),
        }
    }
    
    /// Login a user
    pub fn login_user(&mut self, username: &str, device_id: &str) {
        match self.user_manager.login(username, device_id) {
            Ok(session) => {
                println!("ðŸ‘¤ User logged in: {} (session: {})", username, session.session_id);
                // Touch family device
                self.family_config.touch_device(device_id);
            }
            Err(e) => println!("âŒ Login failed: {}", e),
        }
    }
    
    /// Handle a popup request from JavaScript (smart detection)
    pub fn handle_popup_request(&mut self, target_url: &str, width: Option<u32>, height: Option<u32>, user_gesture: bool) -> bool {
        let request = PopupRequest {
            source_url: self.current_url.clone(),
            target_url: target_url.to_string(),
            width,
            height,
            user_gesture,
            timestamp: Instant::now(),
        };
        
        let decision = self.popup_handler.evaluate(&request);
        let allowed = self.popup_handler.handle(request, &decision);
        
        match &decision {
            PopupDecision::Allow { reason } => {
                println!("âœ… Popup allowed: {}", reason);
            }
            PopupDecision::Block { reason } => {
                println!("ðŸ›¡ï¸ Popup blocked: {}", reason);
            }
            PopupDecision::Prompt { reason } => {
                println!("â“ Popup pending: {}", reason);
            }
        }
        
        allowed
    }
    
    /// Get page trust status for current tab
    pub fn get_page_trust_status(&self) -> Option<(PageTrust, String)> {
        if let Some(tab) = self.ui.tab_manager.active_tab() {
            if let Some(sandbox) = self.sandbox_manager.get(tab.id) {
                let color = sandbox.status_color();
                let text = sandbox.status_text();
                return Some((sandbox.trust, format!("{} ({})", text, color)));
            }
        }
        None
    }
}


pub fn run_browser(initial_url: Option<String>) {
    use winit::event::{Event, StartCause};
    use winit::event_loop::ControlFlow;
    
    println!();
    println!("  â•”â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•—");
    println!("  â•‘       Sassy Browser v1.0.1            â•‘");
    println!("  â•‘  Pure Rust | SassyScript | Sandboxed  â•‘");
    println!("  â•šâ•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•");
    println!();
    println!("  Features:");
    println!("    â€¢ Click address bar or Ctrl+L to type URL");
    println!("    â€¢ Click links to navigate");
    println!("    â€¢ Scroll with mouse wheel or arrow keys");
    println!("    â€¢ Alt+Tab for tab tile view");
    println!("    â€¢ Network activity indicator (top right)");
    println!("    â€¢ Trust indicator (builds with 3 interactions)");
    println!();
    
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    
    let window_attrs = WindowAttributes::default()
        .with_title("Sassy Browser v1.0.1")
        .with_inner_size(winit::dpi::LogicalSize::new(DEFAULT_WIDTH, DEFAULT_HEIGHT));
    
    let window = Arc::new(
        event_loop.create_window(window_attrs)
            .expect("Failed to create window")
    );
    
    let context = softbuffer::Context::new(window.clone()).expect("Failed to create context");
    let mut surface = Surface::new(&context, window.clone()).expect("Failed to create surface");
    
    let mut state = BrowserState::new();
    state.update_ui_bounds();
    
    // Navigate to initial URL
    if let Some(ref url) = initial_url {
        state.navigate(url);
    } else {
        state.navigate("about:blank");
    }
    
    let _ = event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);
        
        match event {
            Event::NewEvents(StartCause::Poll) => {
                window.request_redraw();
            }
            
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => {
                        // Save session before exit
                        state.save_session();
                        elwt.exit();
                    }
                    
                    WindowEvent::Resized(size) => {
                        let _ = surface.resize(
                            NonZeroU32::new(size.width).unwrap_or(NonZeroU32::new(1).unwrap()),
                            NonZeroU32::new(size.height).unwrap_or(NonZeroU32::new(1).unwrap()),
                        );
                        state.resize(size.width, size.height);
                    }
                    
                    WindowEvent::RedrawRequested => {
                        state.update();
                        state.process_sync_commands();
                        
                        let size = window.inner_size();
                        if size.width > 0 && size.height > 0 {
                            if let Ok(mut buffer) = surface.buffer_mut() {
                                {
                                    let slice = buffer.as_mut();
                                    state.render(slice);
                                }
                                let _ = buffer.present();
                            }
                            window.set_cursor_icon(state.current_cursor);
                        }
                        state.needs_repaint = false;
                    }
                    
                    WindowEvent::CursorMoved { position, .. } => {
                        state.handle_mouse_move(position.x as i32, position.y as i32);
                        window.request_redraw();
                    }
                    
                    WindowEvent::MouseInput { state: btn_state, button, .. } => {
                        let btn = match button {
                            WinitMouseButton::Left => MouseButton::Left,
                            WinitMouseButton::Right => MouseButton::Right,
                            WinitMouseButton::Middle => MouseButton::Middle,
                            _ => return,
                        };
                        
                        match btn_state {
                            ElementState::Pressed => {
                                state.handle_mouse_click(
                                    state.input.mouse_x,
                                    state.input.mouse_y,
                                    btn,
                                );
                            }
                            ElementState::Released => {
                                state.ui.mouse_up(
                                    state.input.mouse_x,
                                    state.input.mouse_y,
                                    btn,
                                );
                            }
                        }
                        window.request_redraw();
                    }
                    
                    WindowEvent::MouseWheel { delta, .. } => {
                        let scroll = match delta {
                            winit::event::MouseScrollDelta::LineDelta(_, y) => y * 40.0,
                            winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                        };
                        state.handle_mouse_scroll(-scroll);
                        window.request_redraw();
                    }
                    
                    WindowEvent::ModifiersChanged(mods) => {
                        state.input.ctrl_held = mods.state().control_key();
                        state.input.shift_held = mods.state().shift_key();
                        state.input.alt_held = mods.state().alt_key();
                    }
                    
                    WindowEvent::KeyboardInput { event, .. } => {
                        if event.state == ElementState::Pressed {
                            // Handle character input
                            if let WinitKey::Character(ref s) = event.logical_key {
                                if !state.input.ctrl_held && !state.input.alt_held {
                                    for c in s.chars() {
                                        state.handle_char_input(c);
                                    }
                                }
                            }
                            
                            // Map key
                            let key = match event.logical_key {
                                WinitKey::Named(NamedKey::Escape) => crate::ui::input::Key::Escape,
                                WinitKey::Named(NamedKey::Enter) => crate::ui::input::Key::Enter,
                                WinitKey::Named(NamedKey::Tab) => crate::ui::input::Key::Tab,
                                WinitKey::Named(NamedKey::Space) => crate::ui::input::Key::Space,
                                WinitKey::Named(NamedKey::Backspace) => crate::ui::input::Key::Backspace,
                                WinitKey::Named(NamedKey::Delete) => crate::ui::input::Key::Delete,
                                WinitKey::Named(NamedKey::ArrowLeft) => crate::ui::input::Key::Left,
                                WinitKey::Named(NamedKey::ArrowRight) => crate::ui::input::Key::Right,
                                WinitKey::Named(NamedKey::ArrowUp) => crate::ui::input::Key::Up,
                                WinitKey::Named(NamedKey::ArrowDown) => crate::ui::input::Key::Down,
                                WinitKey::Named(NamedKey::Home) => crate::ui::input::Key::Home,
                                WinitKey::Named(NamedKey::End) => crate::ui::input::Key::End,
                                WinitKey::Named(NamedKey::PageUp) => crate::ui::input::Key::PageUp,
                                WinitKey::Named(NamedKey::PageDown) => crate::ui::input::Key::PageDown,
                                WinitKey::Named(NamedKey::F5) => crate::ui::input::Key::F5,
                                WinitKey::Character(ref s) => {
                                    match crate::fontcase::ascii_lower(s).as_str() {
                                        "t" => crate::ui::input::Key::T,
                                        "w" => crate::ui::input::Key::W,
                                        "r" => crate::ui::input::Key::R,
                                        "l" => crate::ui::input::Key::L,
                                        "a" => crate::ui::input::Key::A,
                                        "c" => crate::ui::input::Key::C,
                                        "v" => crate::ui::input::Key::V,
                                        "x" => crate::ui::input::Key::X,
                                        "[" if state.input.alt_held => {
                                            state.go_back();
                                            crate::ui::input::Key::Other
                                        }
                                        "]" if state.input.alt_held => {
                                            state.go_forward();
                                            crate::ui::input::Key::Other
                                        }
                                        _ => crate::ui::input::Key::Other,
                                    }
                                }
                                _ => crate::ui::input::Key::Other,
                            };
                            
                            state.handle_key(key, true);
                            window.request_redraw();
                        }
                    }
                    
                    WindowEvent::Focused(focused) => {
                        if focused {
                            if let Some(tab) = state.ui.tab_manager.active_tab_mut() {
                                tab.sandbox.focus_gained();
                            }
                        } else if let Some(tab) = state.ui.tab_manager.active_tab_mut() {
                            tab.sandbox.focus_lost();
                        }
                    }
                    
                    _ => {}
                }
            }
            
            _ => {}
        }
    });
}
