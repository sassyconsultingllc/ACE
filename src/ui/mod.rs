//! UI system for Sassy Browser
//! Modular, themeable, four-edge sidebar layout

pub mod theme;
pub mod sidebar;
pub mod tabs;
pub mod input;
pub mod network_bar;
pub mod popup;
pub mod render;
pub mod app;

pub use theme::{Theme, ThemeManager, SidebarState};
#[allow(unused_imports)] // Public API re-exports
pub use sidebar::{Sidebar, SidebarLayout, Edge, Rect};
#[allow(unused_imports)]
pub use tabs::{Tab, TabManager, TabPreview, TileLayout, TabSandbox, TrustLevel, MeaningfulInteraction, SandboxAction};
#[allow(unused_imports)]
pub use input::{InputManager, InputAction, Focus, Key, TextInput, UiBounds, Rect as InputRect};
#[allow(unused_imports)]
pub use network_bar::{NetworkBar, NetworkRequest, RequestState};
#[allow(unused_imports)]
pub use popup::{PopupManager, PopupDecision, PopupClassification, InteractionType};
#[allow(unused_imports)]
pub use render::UIRenderer;

use crate::sync::{SyncServer, SyncEvent};
use crate::ai::AiConfig;

/// Main UI state
pub struct UI {
    pub theme_manager: ThemeManager,
    pub sidebar_layout: SidebarLayout,
    pub tab_manager: TabManager,
    pub sync_server: Option<SyncServer>,
    pub ai: AiConfig,
    pub help_pane_open: bool,
    pub help_sidebar_previous: Option<SidebarState>,
    pub help_response: Option<String>,
    pub help_error: Option<String>,
    pub help_inflight: bool,
    
    // Window state
    pub width: u32,
    pub height: u32,
    pub focused: bool,
    pub fullscreen: bool,
    
    // Interaction state
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub dragging: Option<DragState>,
    pub hover_element: Option<HoverElement>,
}

#[derive(Debug, Clone)]
pub enum DragState {
    SidebarResize { edge: Edge, start_size: u32 },
    TabMove { tab_id: u64, start_index: usize },
    PanelMove { panel_id: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum HoverElement {
    Tab(u64),
    TabClose(u64),
    SidebarToggle(Edge),
    SidebarResize(Edge),
    NavigationBack,
    NavigationForward,
    NavigationRefresh,
    AddressBar,
    None,
}

impl UI {
    pub fn new(width: u32, height: u32) -> Self {
        let theme_manager = ThemeManager::new();
        let theme = theme_manager.current().clone();
        let sidebar_layout = SidebarLayout::from_theme(&theme);
        let tab_manager = TabManager::new();
        let ai = AiConfig::default();
        
        Self {
            theme_manager,
            sidebar_layout,
            tab_manager,
            sync_server: None,
            ai,
            help_pane_open: false,
            help_sidebar_previous: None,
            help_response: None,
            help_error: None,
            help_inflight: false,
            width,
            height,
            focused: true,
            fullscreen: false,
            mouse_x: 0,
            mouse_y: 0,
            dragging: None,
            hover_element: None,
        }
    }
    
    /// Initialize phone sync server
    pub fn start_sync(&mut self, port: u16) -> Result<(), String> {
        let mut server = SyncServer::new(port);
        server.start()?;
        self.sync_server = Some(server);
        Ok(())
    }
    
    pub fn stop_sync(&mut self) {
        if let Some(ref mut server) = self.sync_server {
            server.stop();
        }
        self.sync_server = None;
    }
    
    pub fn process_sync(&mut self) {
        if let Some(ref server) = self.sync_server {
            let mut pending = Vec::new();
            while let Some((client_id, cmd)) = server.poll_command() {
                pending.push((client_id, cmd));
            }
            for (client_id, cmd) in pending {
                self.handle_sync_command(client_id, cmd);
            }
        }
    }
    
    fn handle_sync_command(&mut self, _client_id: u64, cmd: crate::sync::SyncCommand) {
        use crate::sync::SyncCommand::*;
        
        match cmd {
            TabCreate { url } => {
                self.tab_manager.create_tab(url);
            }
            TabClose { tab_id } => {
                self.tab_manager.close_tab(tab_id);
            }
            TabActivate { tab_id } => {
                self.tab_manager.activate_tab(tab_id);
            }
            TabPin { tab_id, pinned } => {
                if let Some(tab) = self.tab_manager.get_tab_mut(tab_id) {
                    tab.pinned = pinned;
                }
            }
            TabMute { tab_id, muted } => {
                if let Some(tab) = self.tab_manager.get_tab_mut(tab_id) {
                    tab.muted = muted;
                }
            }
            TabDuplicate { tab_id } => {
                self.tab_manager.duplicate_tab(tab_id);
            }
            _ => {
                // Other commands handled by engine
            }
        }
    }
    
    /// Broadcast event to connected phones
    pub fn broadcast(&self, event: SyncEvent) {
        if let Some(ref server) = self.sync_server {
            server.broadcast_event(event);
        }
    }
    
    /// Resize UI
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }
    
    /// Get content area bounds (excluding sidebars)
    pub fn content_rect(&self) -> Rect {
        self.sidebar_layout.content_rect(self.width, self.height)
    }
    
    /// Handle mouse move
    pub fn mouse_move(&mut self, x: i32, y: i32) {
        let dx = x - self.mouse_x;
        let dy = y - self.mouse_y;
        self.mouse_x = x;
        self.mouse_y = y;
        
        // Handle dragging
        if let Some(DragState::SidebarResize { edge, .. }) = &self.dragging {
            let delta = match edge {
                Edge::Top | Edge::Bottom => dy,
                Edge::Left | Edge::Right => dx,
            };
            if let Some(sidebar) = self.sidebar_layout.get_mut(*edge) {
                sidebar.handle_resize(delta);
            }
        }
        
        // Update hover state
        self.update_hover(x as u32, y as u32);
    }
    
    fn update_hover(&mut self, x: u32, y: u32) {
        // Check sidebars for resize handles
        for edge in Edge::all() {
            if let Some(sidebar) = self.sidebar_layout.get(edge) {
                let bounds = sidebar.bounds(self.width, self.height, &self.sidebar_layout);
                if sidebar.hit_test_resize(x, y, &bounds) {
                    self.hover_element = Some(HoverElement::SidebarResize(edge));
                    return;
                }
            }
        }
        
        // Check tabs in tile view
        if self.tab_manager.tile_view_active {
            // ... tab tile hit testing
        }
        
        self.hover_element = None;
    }
    
    /// Handle mouse button down
    pub fn mouse_down(&mut self, x: i32, y: i32, button: MouseButton) {
        if button != MouseButton::Left {
            return;
        }
        
        // Check for sidebar resize start
        for edge in Edge::all() {
            if let Some(sidebar) = self.sidebar_layout.get(edge) {
                let bounds = sidebar.bounds(self.width, self.height, &self.sidebar_layout);
                if sidebar.hit_test_resize(x as u32, y as u32, &bounds) {
                    self.dragging = Some(DragState::SidebarResize {
                        edge,
                        start_size: sidebar.size,
                    });
                    return;
                }
            }
        }
    }
    
    /// Handle mouse button up
    pub fn mouse_up(&mut self, _x: i32, _y: i32, _button: MouseButton) {
        self.dragging = None;
    }
    
    /// Handle keyboard
    pub fn key_press(&mut self, key: Key, modifiers: Modifiers) {
        // Tab tile view controls
        if self.tab_manager.tile_view_active {
            match key {
                Key::Escape => {
                    self.tab_manager.tile_view_active = false;
                }
                Key::Enter => {
                    self.tab_manager.activate_selected();
                }
                Key::Tab if modifiers.shift => {
                    self.tab_manager.select_prev();
                }
                Key::Tab => {
                    self.tab_manager.select_next();
                }
                Key::Left => {
                    self.tab_manager.select_prev();
                }
                Key::Right => {
                    self.tab_manager.select_next();
                }
                Key::Up => {
                    let cols = self.calculate_tile_columns();
                    self.tab_manager.select_row_up(cols);
                }
                Key::Down => {
                    let cols = self.calculate_tile_columns();
                    self.tab_manager.select_row_down(cols);
                }
                Key::W if modifiers.ctrl => {
                    self.tab_manager.close_selected();
                }
                Key::Delete => {
                    self.tab_manager.close_selected();
                }
                _ => {}
            }
            return;
        }
        
        // Global shortcuts
        match key {
            Key::Tab if modifiers.alt => {
                self.tab_manager.toggle_tile_view();
            }
            Key::T if modifiers.ctrl => {
                self.tab_manager.create_tab("about:blank".into());
            }
            Key::W if modifiers.ctrl => {
                if let Some(tab) = self.tab_manager.active_tab() {
                    let id = tab.id;
                    self.tab_manager.close_tab(id);
                }
            }
            Key::Key1 if modifiers.ctrl => {
                self.sidebar_layout.toggle(Edge::Left);
            }
            Key::Key2 if modifiers.ctrl => {
                self.sidebar_layout.toggle(Edge::Right);
            }
            _ => {}
        }
    }
    
    fn calculate_tile_columns(&self) -> u32 {
        let content = self.content_rect();
        let theme = self.theme_manager.current();
        
        TileLayout::calculate(
            content.width,
            content.height,
            self.tab_manager.tab_count(),
            theme.layout.tab_tile_min_width,
            theme.layout.tab_tile_max_width,
            theme.layout.tab_tile_aspect_ratio,
            theme.layout.tab_tile_gap,
        ).columns
    }
    
    pub fn toggle_theme(&mut self) {
        let current = self.theme_manager.current().meta.name.clone();
        if current.contains("Dark") {
            let _ = self.theme_manager.switch("light");
        } else {
            let _ = self.theme_manager.switch("dark");
        }
    }
    
    pub fn cursor(&self) -> CursorStyle {
        match &self.hover_element {
            Some(HoverElement::SidebarResize(Edge::Left | Edge::Right)) => CursorStyle::EwResize,
            Some(HoverElement::SidebarResize(Edge::Top | Edge::Bottom)) => CursorStyle::NsResize,
            Some(HoverElement::TabClose(_)) => CursorStyle::Pointer,
            _ => CursorStyle::Default,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CursorStyle {
    Default,
    Pointer,
    Text,
    Move,
    EwResize,
    NsResize,
    NeswResize,
    NwseResize,
    NotAllowed,
    Wait,
}
