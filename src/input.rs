//! Input handling - Address bar, text input, focus management
//!
//! Manages:
//! - Which element has keyboard focus
//! - Text cursor position and selection
//! - Input history (for undo)
//! - IME composition
#![allow(dead_code, unused_variables, unused_imports)]

use std::collections::VecDeque;

/// Currently focused input element
#[derive(Debug, Clone, PartialEq)]
pub enum FocusTarget {
    None,
    AddressBar,
    SearchBar,
    PageInput { element_id: String },
    TabSearch,
}

/// Text input state
#[derive(Debug, Clone)]
pub struct TextInput {
    pub text: String,
    pub cursor: usize,
    pub selection_start: Option<usize>,
    pub history: VecDeque<String>,
    pub history_index: usize,
    pub max_history: usize,
}

impl TextInput {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
            selection_start: None,
            history: VecDeque::new(),
            history_index: 0,
            max_history: 50,
        }
    }
    
    pub fn with_text(text: String) -> Self {
        let cursor = text.len();
        Self {
            text,
            cursor,
            selection_start: None,
            history: VecDeque::new(),
            history_index: 0,
            max_history: 50,
        }
    }
    
    /// Get selection range (start, end) normalized
    pub fn selection(&self) -> Option<(usize, usize)> {
        self.selection_start.map(|start| {
            if start < self.cursor {
                (start, self.cursor)
            } else {
                (self.cursor, start)
            }
        })
    }
    
    /// Insert character at cursor
    pub fn insert_char(&mut self, c: char) {
        // Delete selection first if any
        self.delete_selection();
        
        self.text.insert(self.cursor, c);
        self.cursor += c.len_utf8();
        self.save_history();
    }
    
    /// Insert string at cursor
    pub fn insert_str(&mut self, s: &str) {
        self.delete_selection();
        self.text.insert_str(self.cursor, s);
        self.cursor += s.len();
        self.save_history();
    }
    
    /// Delete selection if any, returns true if deleted
    pub fn delete_selection(&mut self) -> bool {
        if let Some((start, end)) = self.selection() {
            self.text.drain(start..end);
            self.cursor = start;
            self.selection_start = None;
            return true;
        }
        false
    }
    
    /// Backspace - delete char before cursor
    pub fn backspace(&mut self) {
        if self.delete_selection() {
            self.save_history();
            return;
        }
        
        if self.cursor > 0 {
            // Find previous char boundary
            let prev = self.text[..self.cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            
            self.text.drain(prev..self.cursor);
            self.cursor = prev;
            self.save_history();
        }
    }
    
    /// Delete - delete char after cursor
    pub fn delete(&mut self) {
        if self.delete_selection() {
            self.save_history();
            return;
        }
        
        if self.cursor < self.text.len() {
            // Find next char boundary
            let next = self.text[self.cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor + i)
                .unwrap_or(self.text.len());
            
            self.text.drain(self.cursor..next);
            self.save_history();
        }
    }
    
    /// Move cursor left
    pub fn move_left(&mut self, select: bool) {
        if !select {
            // If we have selection, move to start of it
            if let Some((start, _)) = self.selection() {
                self.cursor = start;
                self.selection_start = None;
                return;
            }
        }
        
        if self.cursor > 0 {
            // Start selection if shift held
            if select && self.selection_start.is_none() {
                self.selection_start = Some(self.cursor);
            } else if !select {
                self.selection_start = None;
            }
            
            // Find previous char boundary
            self.cursor = self.text[..self.cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }
    
    /// Move cursor right
    pub fn move_right(&mut self, select: bool) {
        if !select {
            // If we have selection, move to end of it
            if let Some((_, end)) = self.selection() {
                self.cursor = end;
                self.selection_start = None;
                return;
            }
        }
        
        if self.cursor < self.text.len() {
            if select && self.selection_start.is_none() {
                self.selection_start = Some(self.cursor);
            } else if !select {
                self.selection_start = None;
            }
            
            // Find next char boundary
            self.cursor = self.text[self.cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor + i)
                .unwrap_or(self.text.len());
        }
    }
    
    /// Move to start
    pub fn move_home(&mut self, select: bool) {
        if select && self.selection_start.is_none() {
            self.selection_start = Some(self.cursor);
        } else if !select {
            self.selection_start = None;
        }
        self.cursor = 0;
    }
    
    /// Move to end
    pub fn move_end(&mut self, select: bool) {
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
    
    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.selection_start = None;
    }
    
    /// Get selected text
    pub fn selected_text(&self) -> Option<&str> {
        self.selection().map(|(start, end)| &self.text[start..end])
    }
    
    /// Cut selected text
    pub fn cut(&mut self) -> Option<String> {
        self.selection().map(|(start, end)| {
            let cut = self.text[start..end].to_string();
            self.delete_selection();
            self.save_history();
            cut
        })
    }
    
    /// Set text (replaces all)
    pub fn set_text(&mut self, text: String) {
        self.text = text;
        self.cursor = self.text.len();
        self.selection_start = None;
        self.save_history();
    }
    
    /// Save current state to history
    fn save_history(&mut self) {
        // Only save if different from last
        if self.history.back() != Some(&self.text) {
            self.history.push_back(self.text.clone());
            if self.history.len() > self.max_history {
                self.history.pop_front();
            }
            self.history_index = self.history.len();
        }
    }
    
    /// Undo
    pub fn undo(&mut self) {
        if self.history_index > 0 {
            self.history_index -= 1;
            if let Some(text) = self.history.get(self.history_index) {
                self.text = text.clone();
                self.cursor = self.text.len();
                self.selection_start = None;
            }
        }
    }
    
    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

impl Default for TextInput {
    fn default() -> Self {
        Self::new()
    }
}

/// Focus manager
#[derive(Debug)]
pub struct FocusManager {
    pub current: FocusTarget,
    pub address_bar: TextInput,
    pub search_bar: TextInput,
    pub tab_search: TextInput,
}

impl FocusManager {
    pub fn new() -> Self {
        Self {
            current: FocusTarget::None,
            address_bar: TextInput::new(),
            search_bar: TextInput::new(),
            tab_search: TextInput::new(),
        }
    }
    
    /// Focus the address bar
    pub fn focus_address_bar(&mut self, url: &str) {
        self.address_bar.set_text(url.to_string());
        self.address_bar.select_all();
        self.current = FocusTarget::AddressBar;
    }
    
    /// Blur (unfocus) everything
    pub fn blur(&mut self) {
        self.current = FocusTarget::None;
    }
    
    /// Check if address bar has focus
    pub fn is_address_bar_focused(&self) -> bool {
        self.current == FocusTarget::AddressBar
    }
    
    /// Get current input target
    pub fn current_input(&mut self) -> Option<&mut TextInput> {
        match self.current {
            FocusTarget::AddressBar => Some(&mut self.address_bar),
            FocusTarget::SearchBar => Some(&mut self.search_bar),
            FocusTarget::TabSearch => Some(&mut self.tab_search),
            _ => None,
        }
    }
    
    /// Handle character input
    pub fn handle_char(&mut self, c: char) {
        if let Some(input) = self.current_input() {
            input.insert_char(c);
        }
    }
}

impl Default for FocusManager {
    fn default() -> Self {
        Self::new()
    }
}
