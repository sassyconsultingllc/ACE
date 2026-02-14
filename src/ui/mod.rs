//! UI system for Sassy Browser
//! Modular, themeable, four-edge sidebar layout

#![allow(dead_code)]

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

impl DragState {
    /// Label for the drag operation kind
    pub fn label(&self) -> &'static str {
        match self {
            DragState::SidebarResize { .. } => "sidebar-resize",
            DragState::TabMove { .. } => "tab-move",
            DragState::PanelMove { .. } => "panel-move",
        }
    }
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

impl HoverElement {
    /// Label for the hover element kind
    pub fn label(&self) -> &'static str {
        match self {
            HoverElement::Tab(_) => "tab",
            HoverElement::TabClose(_) => "tab-close",
            HoverElement::SidebarToggle(_) => "sidebar-toggle",
            HoverElement::SidebarResize(_) => "sidebar-resize",
            HoverElement::NavigationBack => "nav-back",
            HoverElement::NavigationForward => "nav-forward",
            HoverElement::NavigationRefresh => "nav-refresh",
            HoverElement::AddressBar => "address-bar",
            HoverElement::None => "none",
        }
    }
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

        // Check tabs in tile view using TileLayout::hit_test and total_height
        if self.tab_manager.tile_view_active {
            let content = self.content_rect();
            let theme = self.theme_manager.current();
            let tabs = self.tab_manager.filtered_tabs();
            let tile_layout = TileLayout::calculate(
                content.width, content.height, tabs.len(),
                theme.layout.tab_tile_min_width,
                theme.layout.tab_tile_max_width,
                theme.layout.tab_tile_aspect_ratio,
                theme.layout.tab_tile_gap,
            );
            // Use total_height to check if mouse is within tile area
            let th = tile_layout.total_height(tabs.len());
            if y >= content.y && y < content.y + th {
                let rel_x = x.saturating_sub(content.x);
                let rel_y = y.saturating_sub(content.y);
                if let Some(idx) = tile_layout.hit_test(rel_x, rel_y, tabs.len()) {
                    if let Some(tab) = tabs.get(idx) {
                        self.hover_element = Some(HoverElement::Tab(tab.id));
                        return;
                    }
                }
            }
        }

        self.hover_element = None;
    }
    
    /// Handle mouse button down
    pub fn mouse_down(&mut self, x: i32, y: i32, button: MouseButton) {
        // Record context menu interaction on right-click
        if button == MouseButton::Right {
            if let Some(tab) = self.tab_manager.active_tab_mut() {
                tab.sandbox.record_interaction(MeaningfulInteraction::ContextMenu);
            }
            return;
        }

        if button != MouseButton::Left {
            return;
        }

        // Record link click interaction for the active tab
        if let Some(tab) = self.tab_manager.active_tab_mut() {
            tab.sandbox.record_interaction(MeaningfulInteraction::LinkClick);
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
        
        // Record keyboard shortcut interaction on active tab
        if modifiers.ctrl || modifiers.alt {
            if let Some(tab) = self.tab_manager.active_tab_mut() {
                tab.sandbox.record_interaction(MeaningfulInteraction::KeyboardShortcut);
            }
        }

        // Global shortcuts
        match key {
            Key::Tab if modifiers.alt => {
                self.tab_manager.toggle_tile_view();
            }
            Key::T if modifiers.ctrl => {
                self.tab_manager.create_tab("about:blank".into());
            }
            // Ctrl+Shift+T opens a terminal tab (exercises new_terminal, TabContent::Terminal)
            Key::T if modifiers.ctrl && modifiers.shift => {
                self.tab_manager.create_terminal_tab();
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
    
    /// Describe the current UI state for debugging
    pub fn status(&self) -> String {
        let cursor = self.cursor();
        let drag_label = self.dragging.as_ref().map(|d| d.label()).unwrap_or("none");
        let hover_label = self.hover_element.as_ref().map(|h| h.label()).unwrap_or("none");
        format!(
            "{}x{} focused={} fullscreen={} tabs={} cursor={} drag={} hover={}",
            self.width, self.height,
            self.focused, self.fullscreen,
            self.tab_manager.tab_count(),
            cursor.css_name(),
            drag_label,
            hover_label,
        )
    }

    pub fn cursor(&self) -> CursorStyle {
        match &self.hover_element {
            Some(HoverElement::SidebarResize(Edge::Left | Edge::Right)) => CursorStyle::EwResize,
            Some(HoverElement::SidebarResize(Edge::Top | Edge::Bottom)) => CursorStyle::NsResize,
            Some(HoverElement::TabClose(_)) => CursorStyle::Pointer,
            _ => CursorStyle::Default,
        }
    }

    /// Handle scroll events - records scroll interaction on active tab
    pub fn mouse_scroll(&mut self, delta: i32) {
        if let Some(tab) = self.tab_manager.active_tab_mut() {
            tab.sandbox.record_interaction(MeaningfulInteraction::Scroll { distance: delta });
        }
        // Update scroll_offset for tab list scrolling
        if delta > 0 {
            self.tab_manager.scroll_offset = self.tab_manager.scroll_offset.saturating_add(delta as u32);
        } else {
            self.tab_manager.scroll_offset = self.tab_manager.scroll_offset.saturating_sub((-delta) as u32);
        }
    }

    /// Record a form input interaction on the active tab
    pub fn record_form_input(&mut self) {
        if let Some(tab) = self.tab_manager.active_tab_mut() {
            tab.sandbox.record_interaction(MeaningfulInteraction::FormInput);
        }
    }

    /// Record a form submit interaction on the active tab
    pub fn record_form_submit(&mut self) {
        if let Some(tab) = self.tab_manager.active_tab_mut() {
            tab.sandbox.record_interaction(MeaningfulInteraction::FormSubmit);
        }
    }

    /// Record a text selection interaction on the active tab
    pub fn record_text_selection(&mut self) {
        if let Some(tab) = self.tab_manager.active_tab_mut() {
            tab.sandbox.record_interaction(MeaningfulInteraction::TextSelection);
        }
    }

    /// Record a media play interaction on the active tab
    pub fn record_media_play(&mut self) {
        if let Some(tab) = self.tab_manager.active_tab_mut() {
            tab.sandbox.record_interaction(MeaningfulInteraction::MediaPlay);
        }
    }

    /// Record a video interaction on the active tab
    pub fn record_video_interaction(&mut self) {
        if let Some(tab) = self.tab_manager.active_tab_mut() {
            tab.sandbox.record_interaction(MeaningfulInteraction::VideoInteraction);
        }
    }

    /// Open a PDF in a new tab (exercises Tab::new_pdf, TabContent::Pdf)
    pub fn open_pdf(&mut self, url: String) -> u64 {
        let id = self.tab_manager.next_id();
        let tab = crate::ui::tabs::Tab::new_pdf(id, url);
        self.tab_manager.push_tab(tab);
        id
    }

    /// Open a settings tab (exercises TabContent::Settings)
    pub fn open_settings(&mut self) -> u64 {
        let id = self.tab_manager.next_id();
        let now = std::time::Instant::now();
        let tab = crate::ui::tabs::Tab {
            id,
            title: "Settings".to_string(),
            url: "sassy://settings".to_string(),
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
            content_type: crate::ui::tabs::TabContent::Settings,
            terminal: None,
        };
        self.tab_manager.push_tab(tab);
        id
    }

    /// Mark active tab's sandbox warning as shown
    pub fn mark_warning_shown(&mut self) {
        if let Some(tab) = self.tab_manager.active_tab_mut() {
            tab.sandbox.warning_shown = true;
        }
    }

    /// Check if active tab can perform a sandboxed action
    pub fn can_perform(&self, action: SandboxAction) -> bool {
        self.tab_manager.active_tab()
            .map(|t| t.sandbox.can_perform(action))
            .unwrap_or(false)
    }

    /// Create a tab group
    pub fn create_tab_group(&mut self, name: String, color: String) -> u64 {
        self.tab_manager.create_group(name, color)
    }

    /// Toggle collapse state of a tab group
    pub fn toggle_group_collapse(&mut self, group_id: u64) {
        self.tab_manager.toggle_group_collapse(group_id);
    }

    /// Get active tab status string (exercises Tab::status)
    pub fn active_tab_status(&self) -> String {
        self.tab_manager.active_tab()
            .map(|t| t.status())
            .unwrap_or_default()
    }

    /// Focus the active tab's sandbox (exercises focus_gained)
    pub fn focus_active_tab(&mut self) {
        if let Some(tab) = self.tab_manager.active_tab_mut() {
            tab.sandbox.focus_gained();
        }
    }

    /// Unfocus the active tab's sandbox (exercises focus_lost)
    pub fn unfocus_active_tab(&mut self) {
        if let Some(tab) = self.tab_manager.active_tab_mut() {
            tab.sandbox.focus_lost();
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

impl CursorStyle {
    /// CSS cursor name for this style
    pub fn css_name(&self) -> &'static str {
        match self {
            CursorStyle::Default => "default",
            CursorStyle::Pointer => "pointer",
            CursorStyle::Text => "text",
            CursorStyle::Move => "move",
            CursorStyle::EwResize => "ew-resize",
            CursorStyle::NsResize => "ns-resize",
            CursorStyle::NeswResize => "nesw-resize",
            CursorStyle::NwseResize => "nwse-resize",
            CursorStyle::NotAllowed => "not-allowed",
            CursorStyle::Wait => "wait",
        }
    }
}

impl Modifiers {
    /// Describe active modifier keys
    pub fn describe(&self) -> String {
        let mut parts = Vec::new();
        if self.ctrl { parts.push("Ctrl"); }
        if self.alt { parts.push("Alt"); }
        if self.shift { parts.push("Shift"); }
        if self.meta { parts.push("Meta"); }
        if parts.is_empty() {
            "none".to_string()
        } else {
            parts.join("+")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_status_and_variants() {
        let ui = UI::new(1024, 768);
        let status = ui.status();
        assert!(status.contains("1024x768"));
        assert!(status.contains("fullscreen=false"));
    }

    #[test]
    fn test_drag_state_variants() {
        let drags = vec![
            DragState::SidebarResize { edge: Edge::Left, start_size: 200 },
            DragState::TabMove { tab_id: 1, start_index: 0 },
            DragState::PanelMove { panel_id: "devtools".into() },
        ];
        for d in &drags {
            assert!(!d.label().is_empty());
        }
    }

    #[test]
    fn test_hover_element_variants() {
        let hovers = vec![
            HoverElement::Tab(1),
            HoverElement::TabClose(1),
            HoverElement::SidebarToggle(Edge::Left),
            HoverElement::SidebarResize(Edge::Right),
            HoverElement::NavigationBack,
            HoverElement::NavigationForward,
            HoverElement::NavigationRefresh,
            HoverElement::AddressBar,
            HoverElement::None,
        ];
        for h in &hovers {
            assert!(!h.label().is_empty());
        }
    }

    #[test]
    fn test_cursor_style_variants() {
        let cursors = vec![
            CursorStyle::Default,
            CursorStyle::Pointer,
            CursorStyle::Text,
            CursorStyle::Move,
            CursorStyle::EwResize,
            CursorStyle::NsResize,
            CursorStyle::NeswResize,
            CursorStyle::NwseResize,
            CursorStyle::NotAllowed,
            CursorStyle::Wait,
        ];
        for c in &cursors {
            assert!(!c.css_name().is_empty());
        }
    }

    #[test]
    fn test_modifiers_describe() {
        let mods = Modifiers { ctrl: true, alt: false, shift: true, meta: true };
        let desc = mods.describe();
        assert!(desc.contains("Ctrl"));
        assert!(desc.contains("Shift"));
        assert!(desc.contains("Meta"));
        assert!(!desc.contains("Alt"));

        let empty = Modifiers::default();
        assert_eq!(empty.describe(), "none");
    }

    #[test]
    fn test_mouse_button_variants() {
        // Exercise all MouseButton variants
        let buttons = [MouseButton::Left, MouseButton::Right, MouseButton::Middle];
        assert_eq!(buttons.len(), 3);
    }
}
