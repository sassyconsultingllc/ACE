//! Hit testing - find which layout box is at a given (x, y) coordinate
//!
//! Essential for:
//! - Link clicking
//! - Form field focus
//! - Cursor changes (pointer, text, etc.)
//! - Recording meaningful interactions for page sandbox

#![allow(dead_code)]

use crate::layout::{LayoutBox, Rect};
use crate::dom::{NodeRef, NodeType};

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
    let is_clickable = href.is_some() || matches!(element_type, ElementType::Button | ElementType::Link);
    
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
    let is_clickable = href.is_some() || matches!(element_type, ElementType::Button | ElementType::Link);
    
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
    pub interacted: Vec<usize>,  // Simple indices for now
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
        InteractionQuality::Meaningful
    }
    
    pub fn record_input(&mut self, _node_id: usize, char_count: usize) -> InteractionQuality {
        self.keystroke_count += char_count;
        if char_count > 3 {
            InteractionQuality::Meaningful
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
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InteractionQuality {
    Meaningful,
    Superficial,
    Robotic,
}
