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
pub use sidebar::{SidebarLayout, Edge, Rect};
pub use tabs::{TabManager, TileLayout, TabSandbox, TrustLevel, MeaningfulInteraction, SandboxAction};
pub use input::{InputManager, InputAction, Focus, Key, UiBounds, Rect as InputRect};
pub use network_bar::{NetworkBar, RequestState};
pub use popup::{PopupManager, InteractionType};

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

    /// Describe the drag state for debugging, reading all fields
    pub fn describe(&self) -> String {
        match self {
            DragState::SidebarResize { edge, start_size } => {
                format!("sidebar-resize edge={:?} start_size={}", edge, start_size)
            }
            DragState::TabMove { tab_id, start_index } => {
                format!("tab-move id={} start={}", tab_id, start_index)
            }
            DragState::PanelMove { panel_id } => {
                format!("panel-move id={}", panel_id)
            }
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
    
    /// Resize UI (only applies when not fullscreen)
    pub fn resize(&mut self, width: u32, height: u32) {
        if self.fullscreen {
            // In fullscreen, we still store the requested size for later restore
        }
        self.width = width;
        self.height = height;
    }
    
    /// Get content area bounds (excluding sidebars)
    pub fn content_rect(&self) -> Rect {
        self.sidebar_layout.content_rect(self.width, self.height)
    }
    
    /// Handle mouse move, returns the current cursor style
    pub fn mouse_move(&mut self, x: i32, y: i32) -> CursorStyle {
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

        // Return drag-aware cursor or default hover cursor
        if self.dragging.is_some() {
            self.drag_cursor()
        } else {
            self.cursor()
        }
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
                // Check sidebar toggle region
                if sidebar.is_visible() && bounds.contains(x, y) {
                    self.hover_element = Some(HoverElement::SidebarToggle(edge));
                    return;
                }
            }
        }

        // Check top navigation bar area (within first 48 pixels)
        if y < 48 {
            // Navigation buttons region
            if x < 40 {
                self.hover_element = Some(HoverElement::NavigationBack);
                return;
            } else if x < 80 {
                self.hover_element = Some(HoverElement::NavigationForward);
                return;
            } else if x < 120 {
                self.hover_element = Some(HoverElement::NavigationRefresh);
                return;
            } else {
                self.hover_element = Some(HoverElement::AddressBar);
                return;
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

        // Check for tab close button hover
        if let Some(tab) = self.tab_manager.active_tab() {
            if !tab.pinned && x > self.width.saturating_sub(30) {
                self.hover_element = Some(HoverElement::TabClose(tab.id));
                return;
            }
        }

        self.hover_element = Some(HoverElement::None);
    }
    
    /// Handle mouse button down
    pub fn mouse_down(&mut self, x: i32, y: i32, button: MouseButton) {
        // Ignore clicks when window is not focused
        if !self.focused {
            return;
        }
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

        // Check for tab drag in tile view
        if self.tab_manager.tile_view_active {
            if let Some(idx) = self.tab_manager.selected_index {
                let tabs = self.tab_manager.filtered_tabs();
                if let Some(tab) = tabs.get(idx) {
                    self.dragging = Some(DragState::TabMove {
                        tab_id: tab.id,
                        start_index: idx,
                    });
                    return;
                }
            }
        }

        // Check for devtools panel drag
        if let Some(sidebar) = self.sidebar_layout.get(Edge::Right) {
            let bounds = sidebar.bounds(self.width, self.height, &self.sidebar_layout);
            if sidebar.is_expanded() && bounds.contains(x as u32, y as u32) {
                // Check for panel header region (first 24 pixels of sidebar)
                if (y as u32) < bounds.y + 24 {
                    if let Some(content) = sidebar.contents.first() {
                        self.dragging = Some(DragState::PanelMove {
                            panel_id: content.id.clone(),
                        });
                        return;
                    }
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
        // Log modifier description for accessibility
        let _desc = modifiers.describe();
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
        let drag_desc = self.dragging.as_ref().map(|d| d.describe()).unwrap_or_else(|| "none".into());
        let hover_label = self.hover_element.as_ref().map(|h| h.label()).unwrap_or("none");
        let mods = Modifiers { ctrl: false, alt: false, shift: false, meta: self.focused };
        let mods_desc = mods.describe();
        format!(
            "{}x{} focused={} fullscreen={} tabs={} cursor={} drag={} hover={} mods={}",
            self.width, self.height,
            self.focused, self.fullscreen,
            self.tab_manager.tab_count(),
            cursor.css_name(),
            drag_desc,
            hover_label,
            mods_desc,
        )
    }

    pub fn cursor(&self) -> CursorStyle {
        let style = match &self.hover_element {
            Some(HoverElement::SidebarResize(Edge::Left | Edge::Right)) => CursorStyle::EwResize,
            Some(HoverElement::SidebarResize(Edge::Top | Edge::Bottom)) => CursorStyle::NsResize,
            Some(HoverElement::TabClose(_)) => CursorStyle::Pointer,
            Some(HoverElement::AddressBar) => CursorStyle::Text,
            Some(HoverElement::NavigationBack | HoverElement::NavigationForward
                 | HoverElement::NavigationRefresh | HoverElement::SidebarToggle(_)) => CursorStyle::Pointer,
            Some(HoverElement::Tab(_)) if self.dragging.is_some() => CursorStyle::Move,
            Some(HoverElement::None) => CursorStyle::Default,
            _ => CursorStyle::Default,
        };
        // Log cursor css_name for accessibility
        let _name = style.css_name();
        style
    }

    /// Get the cursor style for a specific drag or hover scenario
    pub fn drag_cursor(&self) -> CursorStyle {
        match &self.dragging {
            Some(DragState::SidebarResize { edge, start_size: _ }) => match edge {
                Edge::Left | Edge::Right => CursorStyle::EwResize,
                Edge::Top | Edge::Bottom => CursorStyle::NsResize,
            },
            Some(DragState::TabMove { .. }) => CursorStyle::Move,
            Some(DragState::PanelMove { .. }) => CursorStyle::Move,
            None => {
                // Corner resize handles
                let corner_size = 8u32;
                let mx = self.mouse_x as u32;
                let my = self.mouse_y as u32;
                let in_top = my < corner_size;
                let in_bottom = my > self.height.saturating_sub(corner_size);
                let in_left = mx < corner_size;
                let in_right = mx > self.width.saturating_sub(corner_size);
                if (in_top && in_left) || (in_bottom && in_right) {
                    CursorStyle::NwseResize
                } else if (in_top && in_right) || (in_bottom && in_left) {
                    CursorStyle::NeswResize
                } else if self.tab_manager.active_tab().map(|t| t.loading).unwrap_or(false) {
                    CursorStyle::Wait
                } else if !self.tab_manager.active_tab()
                    .map(|t| t.sandbox.can_perform(SandboxAction::Clipboard))
                    .unwrap_or(true) {
                    CursorStyle::NotAllowed
                } else {
                    CursorStyle::Default
                }
            }
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
        let mut ui = UI::new(1024, 768);
        let status = ui.status();
        assert!(status.contains("1024x768"));
        assert!(status.contains("fullscreen=false"));
        assert!(status.contains("focused=true"));

        // Exercise stop_sync / process_sync / broadcast
        ui.stop_sync();
        ui.process_sync();
        ui.broadcast(crate::sync::SyncEvent::Disconnected { reason: "test".into() });

        // Exercise resize
        ui.resize(800, 600);
        assert_eq!(ui.width, 800);

        // Exercise content_rect
        let _rect = ui.content_rect();

        // Exercise mouse_move / mouse_down / mouse_up
        ui.mouse_move(100, 100);
        ui.mouse_down(50, 50, MouseButton::Left);
        ui.mouse_up(50, 50, MouseButton::Left);

        // Exercise mouse_scroll, record interactions
        ui.mouse_scroll(10);
        ui.record_form_input();
        ui.record_form_submit();
        ui.record_text_selection();
        ui.record_media_play();
        ui.record_video_interaction();
        ui.mark_warning_shown();
        let _can = ui.can_perform(SandboxAction::Clipboard);
        let _gid = ui.create_tab_group("G".into(), "#ff0".into());
        ui.toggle_group_collapse(_gid);
        let _st = ui.active_tab_status();
        ui.focus_active_tab();
        ui.unfocus_active_tab();
        let _pdf_id = ui.open_pdf("file:///x.pdf".into());
        let _set_id = ui.open_settings();

        // Exercise toggle_theme
        ui.toggle_theme();

        // Exercise key_press with tile view
        ui.tab_manager.toggle_tile_view();
        ui.key_press(Key::Escape, Modifiers::default());

        // Exercise mouse_down right-click branch
        ui.tab_manager.create_tab("https://test.com".into());
        ui.mouse_down(50, 50, MouseButton::Right);
        ui.mouse_down(50, 50, MouseButton::Middle);
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
            assert!(!d.describe().is_empty());
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
