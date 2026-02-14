//! Input Handling - Address bar, keyboard, text cursor
//!
//! All input routing and text editing in one place.


use std::time::Instant;

/// Focus state - what element has keyboard input
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    None,
    AddressBar,
    SearchBar,  // Tab tile search
    PageContent,
    FormField(u64),  // Element ID
}

/// Text input state for editable fields
#[derive(Debug, Clone)]
pub struct TextInput {
    pub text: String,
    pub cursor: usize,        // Cursor position (byte offset)
    pub selection_start: Option<usize>,
    pub last_edit: Instant,
}

impl TextInput {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
            selection_start: None,
            last_edit: Instant::now(),
        }
    }
    
    pub fn with_text(text: &str) -> Self {
        let len = text.len();
        Self {
            text: text.to_string(),
            cursor: len,
            selection_start: None,
            last_edit: Instant::now(),
        }
    }
    
    /// Insert character at cursor
    pub fn insert(&mut self, c: char) {
        self.delete_selection();
        self.text.insert(self.cursor, c);
        self.cursor += c.len_utf8();
        self.last_edit = Instant::now();
    }
    
    /// Insert string at cursor
    pub fn insert_str(&mut self, s: &str) {
        self.delete_selection();
        self.text.insert_str(self.cursor, s);
        self.cursor += s.len();
        self.last_edit = Instant::now();
    }
    
    /// Backspace - delete char before cursor
    pub fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor > 0 {
            // Find previous char boundary
            let prev = self.text[..self.cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.text.remove(prev);
            self.cursor = prev;
            self.last_edit = Instant::now();
        }
    }
    
    /// Delete - delete char after cursor
    pub fn delete(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor < self.text.len() {
            self.text.remove(self.cursor);
            self.last_edit = Instant::now();
        }
    }
    
    /// Move cursor left
    pub fn move_left(&mut self, select: bool) {
        if select && self.selection_start.is_none() {
            self.selection_start = Some(self.cursor);
        } else if !select {
            self.selection_start = None;
        }
        
        if self.cursor > 0 {
            self.cursor = self.text[..self.cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }
    
    /// Move cursor right
    pub fn move_right(&mut self, select: bool) {
        if select && self.selection_start.is_none() {
            self.selection_start = Some(self.cursor);
        } else if !select {
            self.selection_start = None;
        }
        
        if self.cursor < self.text.len() {
            self.cursor = self.text[self.cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor + i)
                .unwrap_or(self.text.len());
        }
    }
    
    /// Move to start
    pub fn home(&mut self, select: bool) {
        if select && self.selection_start.is_none() {
            self.selection_start = Some(self.cursor);
        } else if !select {
            self.selection_start = None;
        }
        self.cursor = 0;
    }
    
    /// Move to end
    pub fn end(&mut self, select: bool) {
        if select && self.selection_start.is_none() {
            self.selection_start = Some(self.cursor);
        } else if !select {
            self.selection_start = None;
        }
        self.cursor = self.text.len();
    }
    
    /// Select all
    pub fn select_all(&mut self) {
        self.selection_start = Some(0);
        self.cursor = self.text.len();
    }
    
    /// Get selected range
    pub fn selection(&self) -> Option<(usize, usize)> {
        self.selection_start.map(|start| {
            if start < self.cursor {
                (start, self.cursor)
            } else {
                (self.cursor, start)
            }
        })
    }
    
    /// Get selected text
    pub fn selected_text(&self) -> Option<&str> {
        self.selection().map(|(start, end)| &self.text[start..end])
    }
    
    /// Delete selection, return true if deleted
    fn delete_selection(&mut self) -> bool {
        if let Some((start, end)) = self.selection() {
            self.text.replace_range(start..end, "");
            self.cursor = start;
            self.selection_start = None;
            self.last_edit = Instant::now();
            true
        } else {
            false
        }
    }
    
    /// Cut selected text
    pub fn cut(&mut self) -> Option<String> {
        if let Some((start, end)) = self.selection() {
            let text = self.text[start..end].to_string();
            self.delete_selection();
            Some(text)
        } else {
            None
        }
    }
    
    /// Copy selected text
    pub fn copy(&self) -> Option<String> {
        self.selected_text().map(|s| s.to_string())
    }
    
    /// Paste text
    pub fn paste(&mut self, text: &str) {
        self.insert_str(text);
    }
    
    /// Clear all text
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
        self.selection_start = None;
        self.last_edit = Instant::now();
    }
    
    /// Set text (replacing all)
    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.cursor = self.text.len();
        self.selection_start = None;
        self.last_edit = Instant::now();
    }
}

impl Default for TextInput {
    fn default() -> Self {
        Self::new()
    }
}

/// Input manager for the browser
#[derive(Debug)]
pub struct InputManager {
    pub focus: Focus,
    pub address_bar: TextInput,
    pub search_bar: TextInput,
    
    // Modifier keys
    pub ctrl_held: bool,
    pub shift_held: bool,
    pub alt_held: bool,
    
    // Mouse state
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub mouse_pressed: bool,
    pub last_click: Option<Instant>,
    pub click_count: u32,  // For double/triple click
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            focus: Focus::None,
            address_bar: TextInput::new(),
            search_bar: TextInput::new(),
            ctrl_held: false,
            shift_held: false,
            alt_held: false,
            mouse_x: 0,
            mouse_y: 0,
            mouse_pressed: false,
            last_click: None,
            click_count: 0,
        }
    }
    
    /// Handle character input
    pub fn char_input(&mut self, c: char) -> InputAction {
        match self.focus {
            Focus::AddressBar => {
                self.address_bar.insert(c);
                InputAction::TextChanged
            }
            Focus::SearchBar => {
                self.search_bar.insert(c);
                InputAction::SearchChanged
            }
            Focus::PageContent | Focus::FormField(_) => {
                InputAction::ForwardToPage(c)
            }
            Focus::None => InputAction::None,
        }
    }
    
    /// Handle key press
    pub fn key_press(&mut self, key: Key) -> InputAction {
        // Global shortcuts first
        if self.ctrl_held {
            match key {
                Key::L => {
                    self.focus = Focus::AddressBar;
                    self.address_bar.select_all();
                    return InputAction::FocusAddressBar;
                }
                Key::T => return InputAction::NewTab,
                Key::W => return InputAction::CloseTab,
                Key::Tab if self.shift_held => return InputAction::PrevTab,
                Key::Tab => return InputAction::NextTab,
                Key::R => return InputAction::Reload,
                Key::F => {
                    self.focus = Focus::SearchBar;
                    self.search_bar.clear();
                    return InputAction::OpenFind;
                }
                _ => {}
            }
        }
        
        // Alt+Tab for tile view
        if self.alt_held && key == Key::Tab {
            return InputAction::ToggleTileView;
        }
        
        // Focus-specific handling
        match self.focus {
            Focus::AddressBar => self.handle_address_bar_key(key),
            Focus::SearchBar => self.handle_search_bar_key(key),
            Focus::PageContent => self.handle_page_key(key),
            Focus::FormField(_) => InputAction::ForwardKeyToPage(key),
            Focus::None => self.handle_unfocused_key(key),
        }
    }
    
    fn handle_address_bar_key(&mut self, key: Key) -> InputAction {
        match key {
            Key::Enter => {
                let url = self.address_bar.text.clone();
                self.focus = Focus::PageContent;
                InputAction::Navigate(url)
            }
            Key::Escape => {
                self.focus = Focus::PageContent;
                InputAction::CancelInput
            }
            Key::Backspace => {
                self.address_bar.backspace();
                InputAction::TextChanged
            }
            Key::Delete => {
                self.address_bar.delete();
                InputAction::TextChanged
            }
            Key::Left => {
                self.address_bar.move_left(self.shift_held);
                InputAction::CursorMoved
            }
            Key::Right => {
                self.address_bar.move_right(self.shift_held);
                InputAction::CursorMoved
            }
            Key::Home => {
                self.address_bar.home(self.shift_held);
                InputAction::CursorMoved
            }
            Key::End => {
                self.address_bar.end(self.shift_held);
                InputAction::CursorMoved
            }
            Key::A if self.ctrl_held => {
                self.address_bar.select_all();
                InputAction::SelectionChanged
            }
            Key::C if self.ctrl_held => {
                if let Some(text) = self.address_bar.copy() {
                    InputAction::Copy(text)
                } else {
                    InputAction::None
                }
            }
            Key::V if self.ctrl_held => {
                InputAction::RequestPaste
            }
            Key::X if self.ctrl_held => {
                if let Some(text) = self.address_bar.cut() {
                    InputAction::Cut(text)
                } else {
                    InputAction::None
                }
            }
            _ => InputAction::None,
        }
    }
    
    fn handle_search_bar_key(&mut self, key: Key) -> InputAction {
        match key {
            Key::Enter => {
                let query = self.search_bar.text.clone();
                InputAction::FindNext(query)
            }
            Key::Escape => {
                self.focus = Focus::PageContent;
                self.search_bar.clear();
                InputAction::CloseFind
            }
            Key::Backspace => {
                self.search_bar.backspace();
                InputAction::SearchChanged
            }
            _ => InputAction::None,
        }
    }
    
    fn handle_page_key(&mut self, key: Key) -> InputAction {
        match key {
            Key::Space if !self.shift_held => InputAction::ScrollDown(300),
            Key::Space if self.shift_held => InputAction::ScrollUp(300),
            Key::PageDown => InputAction::ScrollDown(600),
            Key::PageUp => InputAction::ScrollUp(600),
            Key::Down => InputAction::ScrollDown(50),
            Key::Up => InputAction::ScrollUp(50),
            Key::Home if self.ctrl_held => InputAction::ScrollToTop,
            Key::End if self.ctrl_held => InputAction::ScrollToBottom,
            Key::Tab => InputAction::FocusNextElement,
            Key::F5 => InputAction::Reload,
            _ => InputAction::None,
        }
    }
    
    fn handle_unfocused_key(&mut self, key: Key) -> InputAction {
        match key {
            Key::Tab => {
                self.focus = Focus::PageContent;
                InputAction::FocusNextElement
            }
            _ => InputAction::None,
        }
    }
    
    /// Handle mouse click
    pub fn mouse_click(&mut self, x: i32, y: i32, bounds: &UiBounds) -> InputAction {
        self.mouse_x = x;
        self.mouse_y = y;
        
        // Check for double/triple click
        let now = Instant::now();
        if let Some(last) = self.last_click {
            if now.duration_since(last).as_millis() < 500 {
                self.click_count += 1;
            } else {
                self.click_count = 1;
            }
        } else {
            self.click_count = 1;
        }
        self.last_click = Some(now);
        
        // Determine what was clicked
        if bounds.address_bar.contains(x, y) {
            self.focus = Focus::AddressBar;
            if self.click_count >= 2 {
                self.address_bar.select_all();
            }
            return InputAction::FocusAddressBar;
        }
        
        if bounds.back_button.contains(x, y) {
            return InputAction::GoBack;
        }
        
        if bounds.forward_button.contains(x, y) {
            return InputAction::GoForward;
        }
        
        if bounds.refresh_button.contains(x, y) {
            return InputAction::Reload;
        }

        if bounds.help_button.contains(x, y) {
            return InputAction::ToggleHelpPane;
        }
        
        if bounds.content_area.contains(x, y) {
            self.focus = Focus::PageContent;
            return InputAction::PageClick(
                x - bounds.content_area.x,
                y - bounds.content_area.y,
            );
        }
        
        if bounds.tab_list.contains(x, y) {
            return InputAction::TabListClick(x, y);
        }
        
        InputAction::None
    }
    
    /// Handle mouse scroll
    pub fn mouse_scroll(&mut self, delta_y: f32) -> InputAction {
        if delta_y > 0.0 {
            InputAction::ScrollUp((delta_y * 50.0) as i32)
        } else {
            InputAction::ScrollDown((-delta_y * 50.0) as i32)
        }
    }
    
    /// Handle paste from clipboard
    pub fn paste(&mut self, text: &str) {
        match self.focus {
            Focus::AddressBar => self.address_bar.paste(text),
            Focus::SearchBar => self.search_bar.paste(text),
            _ => {}
        }
    }
    
    /// Set address bar text (e.g., when navigating)
    pub fn set_address(&mut self, url: &str) {
        self.address_bar.set_text(url);
    }
}

impl Default for InputManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Key codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Enter,
    Escape,
    Backspace,
    Delete,
    Tab,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Space,
    F5,
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9,
    Key1, Key2,
    Unknown,
    Other,
}

impl Key {
    /// Map an ASCII character to the corresponding Key variant
    pub fn from_char(c: char) -> Self {
        match c {
            'a' | 'A' => Key::A,
            'b' | 'B' => Key::B,
            'c' | 'C' => Key::C,
            'd' | 'D' => Key::D,
            'e' | 'E' => Key::E,
            'f' | 'F' => Key::F,
            'g' | 'G' => Key::G,
            'h' | 'H' => Key::H,
            'i' | 'I' => Key::I,
            'j' | 'J' => Key::J,
            'k' | 'K' => Key::K,
            'l' | 'L' => Key::L,
            'm' | 'M' => Key::M,
            'n' | 'N' => Key::N,
            'o' | 'O' => Key::O,
            'p' | 'P' => Key::P,
            'q' | 'Q' => Key::Q,
            'r' | 'R' => Key::R,
            's' | 'S' => Key::S,
            't' | 'T' => Key::T,
            'u' | 'U' => Key::U,
            'v' | 'V' => Key::V,
            'w' | 'W' => Key::W,
            'x' | 'X' => Key::X,
            'y' | 'Y' => Key::Y,
            'z' | 'Z' => Key::Z,
            '0' => Key::Num0,
            '1' => Key::Num1,
            '2' => Key::Num2,
            '3' => Key::Num3,
            '4' => Key::Num4,
            '5' => Key::Num5,
            '6' => Key::Num6,
            '7' => Key::Num7,
            '8' => Key::Num8,
            '9' => Key::Num9,
            ' ' => Key::Space,
            '\n' | '\r' => Key::Enter,
            '\t' => Key::Tab,
            '\x08' => Key::Backspace,
            '\x7f' => Key::Delete,
            _ => Key::Unknown,
        }
    }
}

/// Actions that can result from input
#[derive(Debug, Clone)]
pub enum InputAction {
    None,
    TextChanged,
    SearchChanged,
    CursorMoved,
    SelectionChanged,
    FocusAddressBar,
    Navigate(String),
    CancelInput,
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    Reload,
    ToggleHelpPane,
    GoBack,
    GoForward,
    ToggleTileView,
    OpenFind,
    CloseFind,
    FindNext(String),
    ScrollUp(i32),
    ScrollDown(i32),
    ScrollToTop,
    ScrollToBottom,
    FocusNextElement,
    PageClick(i32, i32),  // Relative to content area
    TabListClick(i32, i32),
    ForwardToPage(char),
    ForwardKeyToPage(Key),
    Copy(String),
    Cut(String),
    RequestPaste,
    AllowPendingPopup,
    BlockPendingPopup,
    AllowDomainPopups(String),
    BlockDomainPopups(String),
}

impl InputAction {
    /// Label for the action kind (useful for logging / accessibility)
    pub fn label(&self) -> &'static str {
        match self {
            InputAction::None => "none",
            InputAction::TextChanged => "text-changed",
            InputAction::SearchChanged => "search-changed",
            InputAction::CursorMoved => "cursor-moved",
            InputAction::SelectionChanged => "selection-changed",
            InputAction::FocusAddressBar => "focus-address-bar",
            InputAction::Navigate(_) => "navigate",
            InputAction::CancelInput => "cancel-input",
            InputAction::NewTab => "new-tab",
            InputAction::CloseTab => "close-tab",
            InputAction::NextTab => "next-tab",
            InputAction::PrevTab => "prev-tab",
            InputAction::Reload => "reload",
            InputAction::ToggleHelpPane => "toggle-help",
            InputAction::GoBack => "go-back",
            InputAction::GoForward => "go-forward",
            InputAction::ToggleTileView => "toggle-tile-view",
            InputAction::OpenFind => "open-find",
            InputAction::CloseFind => "close-find",
            InputAction::FindNext(_) => "find-next",
            InputAction::ScrollUp(_) => "scroll-up",
            InputAction::ScrollDown(_) => "scroll-down",
            InputAction::ScrollToTop => "scroll-to-top",
            InputAction::ScrollToBottom => "scroll-to-bottom",
            InputAction::FocusNextElement => "focus-next",
            InputAction::PageClick(_, _) => "page-click",
            InputAction::TabListClick(_, _) => "tab-list-click",
            InputAction::ForwardToPage(_) => "forward-char",
            InputAction::ForwardKeyToPage(_) => "forward-key",
            InputAction::Copy(_) => "copy",
            InputAction::Cut(_) => "cut",
            InputAction::RequestPaste => "request-paste",
            InputAction::AllowPendingPopup => "allow-popup",
            InputAction::BlockPendingPopup => "block-popup",
            InputAction::AllowDomainPopups(_) => "allow-domain-popups",
            InputAction::BlockDomainPopups(_) => "block-domain-popups",
        }
    }
}

/// UI element bounds for hit testing
#[derive(Debug, Clone, Default)]
pub struct UiBounds {
    pub address_bar: Rect,
    pub back_button: Rect,
    pub forward_button: Rect,
    pub refresh_button: Rect,
    pub help_button: Rect,
    pub content_area: Rect,
    pub tab_list: Rect,
    pub network_bar: Rect,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self { x, y, width, height }
    }
    
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.width &&
        py >= self.y && py < self.y + self.height
    }
}

impl UiBounds {
    /// Describe all element bounds for debugging
    pub fn describe(&self) -> String {
        format!(
            "addr={}x{} back={}x{} fwd={}x{} refresh={}x{} help={}x{} content={}x{} tabs={}x{} net={}x{}",
            self.address_bar.width, self.address_bar.height,
            self.back_button.width, self.back_button.height,
            self.forward_button.width, self.forward_button.height,
            self.refresh_button.width, self.refresh_button.height,
            self.help_button.width, self.help_button.height,
            self.content_area.width, self.content_area.height,
            self.tab_list.width, self.tab_list.height,
            self.network_bar.width, self.network_bar.height,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_input_operations() {
        let mut ti = TextInput::with_text("hello");
        assert_eq!(ti.text, "hello");
        assert_eq!(ti.cursor, 5);

        ti.insert(' ');
        ti.insert_str("world");
        assert!(ti.text.contains("world"));

        ti.select_all();
        assert!(ti.selected_text().is_some());

        let copied = ti.copy();
        assert!(copied.is_some());

        let cut = ti.cut();
        assert!(cut.is_some());
        assert!(ti.text.is_empty());

        ti.paste("new text");
        assert_eq!(ti.text, "new text");

        ti.home(false);
        ti.end(true);
        ti.move_left(false);
        ti.move_right(false);
        ti.backspace();
        ti.delete();
        ti.clear();
        ti.set_text("final");
        assert_eq!(ti.text, "final");
    }

    #[test]
    fn test_key_from_char() {
        assert_eq!(Key::from_char('a'), Key::A);
        assert_eq!(Key::from_char('Z'), Key::Z);
        assert_eq!(Key::from_char('5'), Key::Num5);
        assert_eq!(Key::from_char(' '), Key::Space);
        assert_eq!(Key::from_char('!'), Key::Unknown);

        // Exercise Key1/Key2 in mod.rs key handling
        let k1 = Key::Key1;
        let k2 = Key::Key2;
        let other = Key::Other;
        assert_ne!(k1, k2);
        assert_ne!(k2, other);
    }

    #[test]
    fn test_input_manager_and_actions() {
        let mut im = InputManager::new();
        im.set_address("https://example.com");
        assert_eq!(im.address_bar.text, "https://example.com");

        // Character input in address bar
        im.focus = Focus::AddressBar;
        let action = im.char_input('x');
        assert_eq!(action.label(), "text-changed");

        // Scroll
        let scroll_action = im.mouse_scroll(-3.0);
        assert_eq!(scroll_action.label(), "scroll-down");
        let scroll_up = im.mouse_scroll(3.0);
        assert_eq!(scroll_up.label(), "scroll-up");

        // Paste
        im.focus = Focus::SearchBar;
        im.paste("query");
        assert_eq!(im.search_bar.text, "query");

        // FormField focus
        im.focus = Focus::FormField(42);
        let fwd = im.char_input('z');
        assert_eq!(fwd.label(), "forward-char");

        // UiBounds describe exercises network_bar field
        let bounds = UiBounds::default();
        let desc = bounds.describe();
        assert!(desc.contains("net="));

        // Rect::new
        let r = Rect::new(10, 20, 100, 50);
        assert!(r.contains(15, 25));
        assert!(!r.contains(0, 0));
    }

    #[test]
    fn test_input_action_labels_exhaustive() {
        // Ensure all InputAction variants have labels
        let actions: Vec<InputAction> = vec![
            InputAction::None,
            InputAction::TextChanged,
            InputAction::SearchChanged,
            InputAction::CursorMoved,
            InputAction::SelectionChanged,
            InputAction::FocusAddressBar,
            InputAction::Navigate("url".into()),
            InputAction::CancelInput,
            InputAction::NewTab,
            InputAction::CloseTab,
            InputAction::NextTab,
            InputAction::PrevTab,
            InputAction::Reload,
            InputAction::ToggleHelpPane,
            InputAction::GoBack,
            InputAction::GoForward,
            InputAction::ToggleTileView,
            InputAction::OpenFind,
            InputAction::CloseFind,
            InputAction::FindNext("q".into()),
            InputAction::ScrollUp(10),
            InputAction::ScrollDown(10),
            InputAction::ScrollToTop,
            InputAction::ScrollToBottom,
            InputAction::FocusNextElement,
            InputAction::PageClick(0, 0),
            InputAction::TabListClick(0, 0),
            InputAction::ForwardToPage('a'),
            InputAction::ForwardKeyToPage(Key::A),
            InputAction::Copy("t".into()),
            InputAction::Cut("t".into()),
            InputAction::RequestPaste,
            InputAction::AllowPendingPopup,
            InputAction::BlockPendingPopup,
            InputAction::AllowDomainPopups("d".into()),
            InputAction::BlockDomainPopups("d".into()),
        ];
        for a in &actions {
            assert!(!a.label().is_empty());
        }
    }
}
