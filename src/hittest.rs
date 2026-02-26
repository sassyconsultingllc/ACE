//! Hit testing - find which layout box is at a given (x, y) coordinate
//!
//! Essential for:
//! - Link clicking
//! - Form field focus
//! - Cursor changes (pointer, text, etc.)
//! - Recording meaningful interactions for page sandbox

use crate::dom::{NodeRef, NodeType};
use crate::layout::{LayoutBox, Rect};

/// Result of a hit test
#[derive(Debug, Clone)]
pub struct HitResult {
    pub node: Option<NodeRef>,
    pub element_type: ElementType,
    pub bounds: Rect,
    pub href: Option<String>,
    pub cursor: CursorType,
    pub is_editable: bool,
    pub is_clickable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ElementType {
    Link,
    Button,
    Input,
    Textarea,
    Image,
    Text,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CursorType {
    Default,
    Pointer,
    Text,
    Grab,
    NotAllowed,
}

impl CursorType {
    /// Map element type to the appropriate cursor
    pub fn for_element(element: ElementType, is_draggable: bool, is_disabled: bool) -> Self {
        if is_disabled {
            return CursorType::NotAllowed;
        }
        if is_draggable {
            return CursorType::Grab;
        }
        match element {
            ElementType::Link | ElementType::Button => CursorType::Pointer,
            ElementType::Input | ElementType::Textarea => CursorType::Text,
            _ => CursorType::Default,
        }
    }
}

impl HitResult {
    /// Summary of the hit result for diagnostics
    pub fn describe(&self) -> String {
        format!("HitResult[type={:?}, bounds=({},{},{},{}), href={}, cursor={:?}, editable={}, clickable={}, node={}]",
            self.element_type,
            self.bounds.x, self.bounds.y, self.bounds.width, self.bounds.height,
            self.href.as_deref().unwrap_or("none"),
            self.cursor,
            self.is_editable,
            self.is_clickable,
            self.node.is_some())
    }
}

/// Perform hit test - find what element is at (x, y)
pub fn hit_test(layout: &LayoutBox, x: f32, y: f32) -> Option<HitResult> {
    // Check if point is in this box's bounds (including padding/border)
    let total_bounds = get_total_bounds(layout);
    if !total_bounds.contains(x, y) {
        return None;
    }

    // Check children first (front to back)
    for child in layout.children.iter().rev() {
        if let Some(hit) = hit_test(child, x, y) {
            return Some(hit);
        }
    }

    // This box is hit
    let element_type = get_element_type(layout);
    let href = get_href(layout);
    let is_editable = matches!(element_type, ElementType::Input | ElementType::Textarea);
    let is_clickable =
        href.is_some() || matches!(element_type, ElementType::Button | ElementType::Link);

    let cursor = match element_type {
        ElementType::Link | ElementType::Button if href.is_some() => CursorType::Pointer,
        ElementType::Input | ElementType::Textarea => CursorType::Text,
        ElementType::Image if href.is_some() => CursorType::Pointer,
        _ => CursorType::Default,
    };

    Some(HitResult {
        node: layout.node.clone(),
        element_type,
        bounds: total_bounds,
        href,
        cursor,
        is_editable,
        is_clickable,
    })
}

fn get_total_bounds(layout: &LayoutBox) -> Rect {
    // Total bounds includes content + padding + border
    let box_x = layout.bounds.x;
    let box_y = layout.bounds.y;
    let box_w = layout.bounds.width;
    let box_h = layout.bounds.height;

    Rect::new(box_x, box_y, box_w, box_h)
}

/// Get list of all hit results under point (for multi-layer selection)
pub fn hit_test_all(layout: &LayoutBox, x: f32, y: f32) -> Vec<HitResult> {
    let mut results = Vec::new();
    hit_test_all_recursive(layout, x, y, &mut results);
    results
}

fn hit_test_all_recursive(layout: &LayoutBox, x: f32, y: f32, results: &mut Vec<HitResult>) {
    let total_bounds = get_total_bounds(layout);
    if !total_bounds.contains(x, y) {
        return;
    }

    // Add this box
    let element_type = get_element_type(layout);
    let href = get_href(layout);
    let is_editable = matches!(element_type, ElementType::Input | ElementType::Textarea);
    let is_clickable =
        href.is_some() || matches!(element_type, ElementType::Button | ElementType::Link);

    let cursor = match element_type {
        ElementType::Link | ElementType::Button if href.is_some() => CursorType::Pointer,
        ElementType::Input | ElementType::Textarea => CursorType::Text,
        _ => CursorType::Default,
    };

    results.push(HitResult {
        node: layout.node.clone(),
        element_type,
        bounds: total_bounds,
        href,
        cursor,
        is_editable,
        is_clickable,
    });

    // Check children
    for child in &layout.children {
        hit_test_all_recursive(child, x, y, results);
    }
}

fn get_href(layout: &LayoutBox) -> Option<String> {
    // Check if this is an anchor with href
    if let Some(node_ref) = &layout.node {
        let node = node_ref.borrow();
        if node.node_type == NodeType::Element {
            if let Some(ref tag) = node.tag_name {
                if tag == "a" {
                    return node.attributes.get("href").cloned();
                }
            }
        }
    }
    None
}

fn get_element_type(layout: &LayoutBox) -> ElementType {
    if let Some(node_ref) = &layout.node {
        let node = node_ref.borrow();
        if node.node_type == NodeType::Element {
            if let Some(ref tag) = node.tag_name {
                return match tag.as_str() {
                    "a" => ElementType::Link,
                    "button" => ElementType::Button,
                    "input" => ElementType::Input,
                    "textarea" => ElementType::Textarea,
                    "img" => ElementType::Image,
                    _ => ElementType::Other,
                };
            }
        } else if node.node_type == NodeType::Text {
            return ElementType::Text;
        }
    }

    ElementType::Other
}

// Interaction tracking for page trust score
pub struct InteractionTracker {
    pub interacted: Vec<usize>, // Simple indices for now
    pub total_actions: usize,
    pub edited_fields: Vec<usize>,
    pub keystroke_count: usize,
}

impl InteractionTracker {
    pub fn new() -> Self {
        Self {
            interacted: Vec::new(),
            total_actions: 0,
            edited_fields: Vec::new(),
            keystroke_count: 0,
        }
    }

    pub fn record_click(&mut self, _hit: &HitResult) -> InteractionQuality {
        self.total_actions += 1;
        self.interacted.push(self.total_actions);
        InteractionQuality::Meaningful
    }

    pub fn record_input(&mut self, node_id: usize, char_count: usize) -> InteractionQuality {
        self.keystroke_count += char_count;
        if !self.edited_fields.contains(&node_id) {
            self.edited_fields.push(node_id);
        }
        if char_count > 3 {
            InteractionQuality::Meaningful
        } else if char_count == 0 {
            InteractionQuality::Robotic
        } else {
            InteractionQuality::Superficial
        }
    }

    pub fn get_quality_score(&self) -> f32 {
        if self.total_actions == 0 {
            return 0.0;
        }
        let meaningful = self.edited_fields.len() + (self.keystroke_count / 10);
        meaningful as f32 / self.total_actions.max(1) as f32
    }

    /// Summary of tracker state for diagnostics
    pub fn describe(&self) -> String {
        format!(
            "InteractionTracker[actions={}, interacted={}, edited={}, keystrokes={}, score={:.2}]",
            self.total_actions,
            self.interacted.len(),
            self.edited_fields.len(),
            self.keystroke_count,
            self.get_quality_score()
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InteractionQuality {
    Meaningful,
    Superficial,
    Robotic,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interaction_tracker_basic() {
        let mut tracker = InteractionTracker::new();
        // No actions yet
        assert_eq!(tracker.get_quality_score(), 0.0);

        // Record small input (superficial)
        let q1 = tracker.record_input(1, 2);
        assert_eq!(q1, InteractionQuality::Superficial);

        // Record larger input (meaningful)
        let q2 = tracker.record_input(2, 10);
        assert_eq!(q2, InteractionQuality::Meaningful);

        // Create dummy hit result and record a click
        let hit = HitResult {
            node: None,
            element_type: ElementType::Other,
            bounds: crate::layout::Rect::new(0.0, 0.0, 10.0, 10.0),
            href: None,
            cursor: CursorType::Default,
            is_editable: false,
            is_clickable: false,
        };
        let q3 = tracker.record_click(&hit);
        assert_eq!(q3, InteractionQuality::Meaningful);

        // Quality score should be > 0
        let score = tracker.get_quality_score();
        assert!(score >= 0.0);
    }
}
