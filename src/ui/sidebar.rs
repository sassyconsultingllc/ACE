//! Four-edge sidebar system
//! Sidebars can be placed on any edge: top, right, bottom, left
//! Each can be hidden, collapsed, or expanded


use super::theme::{SidebarState, Theme};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Edge {
    Top,
    Right,
    Bottom,
    Left,
}

impl Edge {
    pub fn all() -> [Edge; 4] {
        [Edge::Top, Edge::Right, Edge::Bottom, Edge::Left]
    }
    
    pub fn is_horizontal(&self) -> bool {
        matches!(self, Edge::Top | Edge::Bottom)
    }
    
    pub fn is_vertical(&self) -> bool {
        matches!(self, Edge::Left | Edge::Right)
    }
}

#[derive(Debug, Clone)]
pub struct SidebarContent {
    pub id: String,
    pub title: String,
    pub icon: Option<String>,
    pub widget: SidebarWidget,
}

#[derive(Debug, Clone)]
pub enum SidebarWidget {
    TabList,
    TabTiles,
    Bookmarks,
    History,
    Downloads,
    Extensions,
    DevTools,
    Search,
    Navigation,
    AddressBar,
    StatusBar,
    Custom(String),
}

impl SidebarWidget {
    /// Human-readable label for this widget type
    pub fn label(&self) -> &'static str {
        match self {
            SidebarWidget::TabList => "Tab List",
            SidebarWidget::TabTiles => "Tab Tiles",
            SidebarWidget::Bookmarks => "Bookmarks",
            SidebarWidget::History => "History",
            SidebarWidget::Downloads => "Downloads",
            SidebarWidget::Extensions => "Extensions",
            SidebarWidget::DevTools => "DevTools",
            SidebarWidget::Search => "Search",
            SidebarWidget::Navigation => "Navigation",
            SidebarWidget::AddressBar => "Address Bar",
            SidebarWidget::StatusBar => "Status Bar",
            SidebarWidget::Custom(_) => "Custom",
        }
    }
}

impl SidebarContent {
    /// Describe this content entry for accessibility / debugging
    pub fn describe(&self) -> String {
        let icon_str = self.icon.as_deref().unwrap_or("none");
        format!("[{}] {} (icon={}, widget={})", self.id, self.title, icon_str, self.widget.label())
    }
}

#[derive(Debug, Clone)]
pub struct Sidebar {
    pub edge: Edge,
    pub state: SidebarState,
    pub size: u32,           // width for left/right, height for top/bottom
    pub collapsed_size: u32,
    pub contents: Vec<SidebarContent>,
    pub resizable: bool,
    pub dragging: bool,
    pub hover: bool,
    pub min_size: u32,
    pub max_size: u32,
}

impl Sidebar {
    pub fn new(edge: Edge) -> Self {
        let (size, min, max) = match edge {
            Edge::Top => (48, 36, 200),
            Edge::Bottom => (32, 24, 150),
            Edge::Left => (280, 180, 500),
            Edge::Right => (320, 200, 600),
        };
        
        Self {
            edge,
            state: SidebarState::Expanded,
            size,
            collapsed_size: 48,
            contents: Vec::new(),
            resizable: true,
            dragging: false,
            hover: false,
            min_size: min,
            max_size: max,
        }
    }
    
    pub fn with_state(mut self, state: SidebarState) -> Self {
        self.state = state;
        self
    }
    
    pub fn with_size(mut self, size: u32) -> Self {
        self.size = size.clamp(self.min_size, self.max_size);
        self
    }
    
    pub fn add_content(&mut self, content: SidebarContent) {
        self.contents.push(content);
    }
    
    pub fn remove_content(&mut self, id: &str) {
        self.contents.retain(|c| c.id != id);
    }
    
    pub fn toggle(&mut self) {
        self.state = match self.state {
            SidebarState::Hidden => SidebarState::Collapsed,
            SidebarState::Collapsed => SidebarState::Expanded,
            SidebarState::Expanded => SidebarState::Collapsed,
        };
    }
    
    pub fn show(&mut self) {
        if self.state == SidebarState::Hidden {
            self.state = SidebarState::Collapsed;
        }
    }
    
    pub fn hide(&mut self) {
        self.state = SidebarState::Hidden;
    }
    
    pub fn expand(&mut self) {
        self.state = SidebarState::Expanded;
    }
    
    pub fn collapse(&mut self) {
        self.state = SidebarState::Collapsed;
    }
    
    pub fn is_visible(&self) -> bool {
        self.state != SidebarState::Hidden
    }
    
    pub fn is_expanded(&self) -> bool {
        self.state == SidebarState::Expanded
    }
    
    pub fn current_size(&self) -> u32 {
        match self.state {
            SidebarState::Hidden => 0,
            SidebarState::Collapsed => self.collapsed_size,
            SidebarState::Expanded => self.size,
        }
    }
    
    /// Get the rectangle bounds for this sidebar given viewport dimensions
    pub fn bounds(&self, viewport_width: u32, viewport_height: u32, other_sidebars: &SidebarLayout) -> Rect {
        let size = self.current_size();
        
        match self.edge {
            Edge::Top => Rect {
                x: other_sidebars.left_size(),
                y: 0,
                width: viewport_width.saturating_sub(other_sidebars.left_size() + other_sidebars.right_size()),
                height: size,
            },
            Edge::Bottom => Rect {
                x: other_sidebars.left_size(),
                y: viewport_height.saturating_sub(size),
                width: viewport_width.saturating_sub(other_sidebars.left_size() + other_sidebars.right_size()),
                height: size,
            },
            Edge::Left => Rect {
                x: 0,
                y: other_sidebars.top_size(),
                width: size,
                height: viewport_height.saturating_sub(other_sidebars.top_size() + other_sidebars.bottom_size()),
            },
            Edge::Right => Rect {
                x: viewport_width.saturating_sub(size),
                y: other_sidebars.top_size(),
                width: size,
                height: viewport_height.saturating_sub(other_sidebars.top_size() + other_sidebars.bottom_size()),
            },
        }
    }
    
    /// Check if a point is on the resize handle
    pub fn hit_test_resize(&self, x: u32, y: u32, bounds: &Rect) -> bool {
        if !self.resizable || self.state != SidebarState::Expanded {
            return false;
        }
        
        let handle_size = 6;
        
        match self.edge {
            Edge::Top => y >= bounds.y + bounds.height - handle_size && y < bounds.y + bounds.height,
            Edge::Bottom => y >= bounds.y && y < bounds.y + handle_size,
            Edge::Left => x >= bounds.x + bounds.width - handle_size && x < bounds.x + bounds.width,
            Edge::Right => x >= bounds.x && x < bounds.x + handle_size,
        }
    }
    
    /// Handle resize drag
    pub fn handle_resize(&mut self, delta: i32) {
        let new_size = match self.edge {
            Edge::Top | Edge::Left => (self.size as i32 + delta).max(0) as u32,
            Edge::Bottom | Edge::Right => (self.size as i32 - delta).max(0) as u32,
        };
        self.size = new_size.clamp(self.min_size, self.max_size);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn contains(&self, x: u32, y: u32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
    
    pub fn right(&self) -> u32 {
        self.x + self.width
    }
    
    pub fn bottom(&self) -> u32 {
        self.y + self.height
    }
}

/// Manages all four sidebars
pub struct SidebarLayout {
    pub sidebars: HashMap<Edge, Sidebar>,
}

impl SidebarLayout {
    pub fn new() -> Self {
        let mut sidebars = HashMap::new();
        
        // Create default sidebars
        let mut top = Sidebar::new(Edge::Top);
        top.add_content(SidebarContent {
            id: "nav".into(),
            title: "Navigation".into(),
            icon: None,
            widget: SidebarWidget::Navigation,
        });
        top.add_content(SidebarContent {
            id: "address".into(),
            title: "Address Bar".into(),
            icon: None,
            widget: SidebarWidget::AddressBar,
        });
        
        let mut left = Sidebar::new(Edge::Left);
        left.add_content(SidebarContent {
            id: "tabs".into(),
            title: "Tabs".into(),
            icon: Some("".into()),
            widget: SidebarWidget::TabTiles,
        });
        left.add_content(SidebarContent {
            id: "bookmarks".into(),
            title: "Bookmarks".into(),
                icon: Some("".into()),
            widget: SidebarWidget::Bookmarks,
        });
        
        let mut right = Sidebar::new(Edge::Right).with_state(SidebarState::Collapsed);
        right.add_content(SidebarContent {
            id: "devtools".into(),
            title: "DevTools".into(),
            icon: Some("".into()),
            widget: SidebarWidget::DevTools,
        });
        
        let mut bottom = Sidebar::new(Edge::Bottom).with_state(SidebarState::Hidden);
        bottom.add_content(SidebarContent {
            id: "status".into(),
            title: "Status".into(),
            icon: None,
            widget: SidebarWidget::StatusBar,
        });
        
        sidebars.insert(Edge::Top, top);
        sidebars.insert(Edge::Right, right);
        sidebars.insert(Edge::Bottom, bottom);
        sidebars.insert(Edge::Left, left);
        
        Self { sidebars }
    }
    
    pub fn from_theme(theme: &Theme) -> Self {
        let mut layout = Self::new();
        
        // Apply theme layout settings
        if let Some(top) = layout.sidebars.get_mut(&Edge::Top) {
            top.state = theme.layout.sidebar_top.clone();
            top.size = theme.layout.sidebar_top_height;
            top.collapsed_size = theme.layout.sidebar_collapsed_size;
        }
        if let Some(right) = layout.sidebars.get_mut(&Edge::Right) {
            right.state = theme.layout.sidebar_right.clone();
            right.size = theme.layout.sidebar_right_width;
            right.collapsed_size = theme.layout.sidebar_collapsed_size;
        }
        if let Some(bottom) = layout.sidebars.get_mut(&Edge::Bottom) {
            bottom.state = theme.layout.sidebar_bottom.clone();
            bottom.size = theme.layout.sidebar_bottom_height;
            bottom.collapsed_size = theme.layout.sidebar_collapsed_size;
        }
        if let Some(left) = layout.sidebars.get_mut(&Edge::Left) {
            left.state = theme.layout.sidebar_left.clone();
            left.size = theme.layout.sidebar_left_width;
            left.collapsed_size = theme.layout.sidebar_collapsed_size;
        }
        
        layout
    }
    
    pub fn get(&self, edge: Edge) -> Option<&Sidebar> {
        self.sidebars.get(&edge)
    }
    
    pub fn get_mut(&mut self, edge: Edge) -> Option<&mut Sidebar> {
        self.sidebars.get_mut(&edge)
    }
    
    pub fn top_size(&self) -> u32 {
        self.sidebars.get(&Edge::Top).map(|s| s.current_size()).unwrap_or(0)
    }
    
    pub fn bottom_size(&self) -> u32 {
        self.sidebars.get(&Edge::Bottom).map(|s| s.current_size()).unwrap_or(0)
    }
    
    pub fn left_size(&self) -> u32 {
        self.sidebars.get(&Edge::Left).map(|s| s.current_size()).unwrap_or(0)
    }
    
    pub fn right_size(&self) -> u32 {
        self.sidebars.get(&Edge::Right).map(|s| s.current_size()).unwrap_or(0)
    }
    
    /// Get the content area (viewport minus sidebars)
    pub fn content_rect(&self, viewport_width: u32, viewport_height: u32) -> Rect {
        Rect {
            x: self.left_size(),
            y: self.top_size(),
            width: viewport_width.saturating_sub(self.left_size() + self.right_size()),
            height: viewport_height.saturating_sub(self.top_size() + self.bottom_size()),
        }
    }
    
    /// Find which sidebar (if any) contains a point
    pub fn hit_test(&self, x: u32, y: u32, viewport_width: u32, viewport_height: u32) -> Option<Edge> {
        for edge in Edge::all() {
            if let Some(sidebar) = self.sidebars.get(&edge) {
                if sidebar.is_visible() {
                    let bounds = sidebar.bounds(viewport_width, viewport_height, self);
                    if bounds.contains(x, y) {
                        return Some(edge);
                    }
                }
            }
        }
        None
    }
    
    /// Toggle a specific sidebar
    pub fn toggle(&mut self, edge: Edge) {
        if let Some(sidebar) = self.sidebars.get_mut(&edge) {
            sidebar.toggle();
        }
    }
    
    /// Move content between sidebars
    pub fn move_content(&mut self, content_id: &str, from: Edge, to: Edge) {
        let content = self.sidebars.get_mut(&from)
            .and_then(|s| {
                let idx = s.contents.iter().position(|c| c.id == content_id)?;
                Some(s.contents.remove(idx))
            });
        
        if let Some(content) = content {
            if let Some(sidebar) = self.sidebars.get_mut(&to) {
                sidebar.contents.push(content);
            }
        }
    }
}

impl Default for SidebarLayout {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sidebar_bounds_and_toggle() {
        let layout = SidebarLayout::new();
        let left = layout.get(Edge::Left).unwrap();
        assert!(left.is_visible());

        // Toggle left sidebar on a separate layout instance
        let mut layout2 = SidebarLayout::new();
        layout2.toggle(Edge::Left);
        let left2 = layout2.get(Edge::Left).unwrap();
        assert!(!matches!(left2.state, super::SidebarState::Hidden));

        // Content rect should shrink when left sidebar present
        let rect = layout.content_rect(800, 600);
        assert!(rect.width < 800);

        // Exercise Rect::right and Rect::bottom
        assert_eq!(rect.right(), rect.x + rect.width);
        assert_eq!(rect.bottom(), rect.y + rect.height);

        // Exercise Edge::is_horizontal / is_vertical
        assert!(Edge::Top.is_horizontal());
        assert!(Edge::Bottom.is_horizontal());
        assert!(Edge::Left.is_vertical());
        assert!(Edge::Right.is_vertical());
    }

    #[test]
    fn test_sidebar_with_size_and_remove() {
        let mut sidebar = Sidebar::new(Edge::Left).with_size(300);
        assert_eq!(sidebar.size, 300);

        sidebar.add_content(SidebarContent {
            id: "test".into(),
            title: "Test".into(),
            icon: Some("icon".into()),
            widget: SidebarWidget::History,
        });
        assert_eq!(sidebar.contents.len(), 1);

        // Exercise SidebarContent::describe which reads icon
        let desc = sidebar.contents[0].describe();
        assert!(desc.contains("icon=icon"));

        sidebar.remove_content("test");
        assert!(sidebar.contents.is_empty());
    }

    #[test]
    fn test_all_sidebar_widget_variants() {
        // Construct every SidebarWidget variant to wire them up
        let widgets = vec![
            SidebarWidget::TabList,
            SidebarWidget::TabTiles,
            SidebarWidget::Bookmarks,
            SidebarWidget::History,
            SidebarWidget::Downloads,
            SidebarWidget::Extensions,
            SidebarWidget::DevTools,
            SidebarWidget::Search,
            SidebarWidget::Navigation,
            SidebarWidget::AddressBar,
            SidebarWidget::StatusBar,
            SidebarWidget::Custom("my-widget".into()),
        ];
        for w in &widgets {
            assert!(!w.label().is_empty());
        }
    }

    #[test]
    fn test_sidebar_resize_and_move_content() {
        let mut sidebar = Sidebar::new(Edge::Right);
        sidebar.expand();
        assert!(sidebar.is_expanded());
        sidebar.handle_resize(-20);
        assert!(sidebar.size <= sidebar.max_size);

        let mut layout = SidebarLayout::new();

        // Exercise move_content
        layout.move_content("tabs", Edge::Left, Edge::Right);
        let right = layout.get(Edge::Right).unwrap();
        assert!(right.contents.iter().any(|c| c.id == "tabs"));

        // Hit test
        let edge = layout.hit_test(5, 200, 800, 600);
        assert!(edge.is_some());
    }
}
