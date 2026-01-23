// Layout - Box layout engine with Flexbox support
#![allow(unused_variables)]

use crate::dom::{NodeRef, NodeType};
#[allow(unused_imports)]
use crate::style::{ComputedStyle, Display, Dimension, Position, FlexDirection, JustifyContent, AlignItems, FlexWrap};

#[derive(Debug, Clone, Default)]
pub struct LayoutBox {
    pub node: Option<NodeRef>,
    pub style: ComputedStyle,
    pub bounds: Rect,
    pub content: Rect,
    pub padding: Rect,
    pub border: Rect,
    pub margin: Rect,
    pub children: Vec<LayoutBox>,
    pub box_type: BoxType,
    pub text: Option<String>,
    pub is_anonymous: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Rect { x, y, width, height }
    }
    pub fn zero() -> Self { Rect::default() }
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }
    pub fn right(&self) -> f32 { self.x + self.width }
    pub fn bottom(&self) -> f32 { self.y + self.height }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum BoxType { #[default] Block, Inline, InlineBlock, Flex, Anonymous, Text }

/// Flex item for layout calculation
#[derive(Debug, Clone)]
struct FlexItem {
    main_size: f32,
    cross_size: f32,
    flex_grow: f32,
    flex_shrink: f32,
    frozen: bool,
}

/// Flex line for wrapping
#[derive(Debug, Clone)]
struct FlexLine {
    items: Vec<usize>,
    total_main: f32,
    max_cross: f32,
}

impl FlexLine {
    fn new() -> Self {
        FlexLine { items: Vec::new(), total_main: 0.0, max_cross: 0.0 }
    }
}

pub struct LayoutEngine {
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub font_size: f32,
}

impl LayoutEngine {
    pub fn new(width: f32, height: f32) -> Self {
        LayoutEngine { viewport_width: width, viewport_height: height, font_size: 16.0 }
    }

    pub fn layout(&mut self, root: &NodeRef, styles: &std::collections::HashMap<*const std::cell::RefCell<crate::dom::Node>, ComputedStyle>) -> LayoutBox {
        let mut root_box = self.build_layout_tree(root, styles);
        root_box.bounds.width = self.viewport_width;
        root_box.bounds.x = 0.0;
        root_box.bounds.y = 0.0;
        self.calculate_layout(&mut root_box);
        root_box
    }

    #[allow(clippy::only_used_in_recursion)]
    fn build_layout_tree(&self, node: &NodeRef, styles: &std::collections::HashMap<*const std::cell::RefCell<crate::dom::Node>, ComputedStyle>) -> LayoutBox {
        let n = node.borrow();
        let style = styles.get(&(node.as_ref() as *const _)).cloned().unwrap_or_default();
        
        if style.display == Display::None { return LayoutBox::default(); }
        
        let box_type = match n.node_type {
            NodeType::Text => BoxType::Text,
            NodeType::Element => match style.display {
                Display::Block => BoxType::Block,
                Display::Inline => BoxType::Inline,
                Display::InlineBlock => BoxType::InlineBlock,
                Display::Flex => BoxType::Flex,
                Display::Grid => BoxType::Block, // Grid falls back to block for now
                Display::None => return LayoutBox::default(),
            },
            _ => BoxType::Anonymous,
        };
        
        let mut layout_box = LayoutBox {
            node: Some(node.clone()),
            style,
            box_type,
            text: n.text_content.clone(),
            ..Default::default()
        };
        
        for child in &n.children {
            let child_box = self.build_layout_tree(child, styles);
            if child_box.box_type != BoxType::Anonymous || !child_box.children.is_empty() {
                layout_box.children.push(child_box);
            }
        }
        
        layout_box
    }

    fn calculate_layout(&mut self, layout_box: &mut LayoutBox) {
        match layout_box.box_type {
            BoxType::Block => self.layout_block(layout_box),
            BoxType::Flex => self.layout_flex(layout_box),
            BoxType::Inline | BoxType::InlineBlock => self.layout_inline(layout_box),
            BoxType::Text => self.layout_text(layout_box),
            BoxType::Anonymous => self.layout_block(layout_box),
        }
    }

    fn layout_block(&mut self, layout_box: &mut LayoutBox) {
        self.calculate_block_width(layout_box);
        self.calculate_block_position(layout_box);
        self.layout_block_children(layout_box);
        self.calculate_block_height(layout_box);
    }
    
    /// Flexbox layout - the heart of modern CSS
    fn layout_flex(&mut self, layout_box: &mut LayoutBox) {
        self.calculate_block_width(layout_box);
        self.calculate_block_position(layout_box);
        
        let style = &layout_box.style;
        let is_row = matches!(style.flex_direction, FlexDirection::Row | FlexDirection::RowReverse);
        let is_reverse = matches!(style.flex_direction, FlexDirection::RowReverse | FlexDirection::ColumnReverse);
        let wrap = style.flex_wrap;
        let justify = style.justify_content;
        let align = style.align_items;
        
        let container_main = if is_row { layout_box.content.width } else { layout_box.content.height };
        let container_cross = if is_row { layout_box.content.height } else { layout_box.content.width };
        let container_x = layout_box.content.x;
        let container_y = layout_box.content.y;
        
        // First pass: calculate natural sizes of children
        let mut flex_items: Vec<FlexItem> = Vec::new();
        for child in &mut layout_box.children {
            // Give child initial bounds for measurement
            child.bounds.width = if is_row { 0.0 } else { layout_box.content.width };
            child.bounds.height = if !is_row { 0.0 } else { layout_box.content.height };
            child.bounds.x = container_x;
            child.bounds.y = container_y;
            
            // Calculate child's natural size
            self.calculate_layout(child);
            
            let main_size = if is_row { child.margin.width } else { child.margin.height };
            let cross_size = if is_row { child.margin.height } else { child.margin.width };
            
            flex_items.push(FlexItem {
                main_size,
                cross_size,
                flex_grow: child.style.flex_grow,
                flex_shrink: child.style.flex_shrink,
                frozen: false,
            });
        }
        
        // Build flex lines (handle wrapping)
        let mut lines: Vec<FlexLine> = Vec::new();
        let mut current_line = FlexLine::new();
        
        for (i, item) in flex_items.iter().enumerate() {
            if wrap != FlexWrap::NoWrap && 
               current_line.total_main + item.main_size > container_main && 
               !current_line.items.is_empty() {
                lines.push(current_line);
                current_line = FlexLine::new();
            }
            current_line.items.push(i);
            current_line.total_main += item.main_size;
            current_line.max_cross = current_line.max_cross.max(item.cross_size);
        }
        if !current_line.items.is_empty() {
            lines.push(current_line);
        }
        
        // Resolve flexible lengths and position items
        let mut cross_pos = if is_row { container_y } else { container_x };
        
        for line in &lines {
            let free_space = container_main - line.total_main;
            let item_count = line.items.len();
            
            // Calculate positions based on justify-content
            let (mut main_pos, gap, extra_start) = match justify {
                JustifyContent::FlexStart => (0.0, 0.0, 0.0),
                JustifyContent::FlexEnd => (free_space, 0.0, 0.0),
                JustifyContent::Center => (free_space / 2.0, 0.0, 0.0),
                JustifyContent::SpaceBetween => {
                    let g = if item_count > 1 { free_space / (item_count - 1) as f32 } else { 0.0 };
                    (0.0, g, 0.0)
                }
                JustifyContent::SpaceAround => {
                    let g = free_space / item_count as f32;
                    (g / 2.0, g, 0.0)
                }
                JustifyContent::SpaceEvenly => {
                    let g = free_space / (item_count + 1) as f32;
                    (g, g, 0.0)
                }
            };
            
            main_pos += if is_row { container_x } else { container_y };
            
            // Handle reverse order
            let indices: Vec<usize> = if is_reverse {
                line.items.iter().rev().copied().collect()
            } else {
                line.items.clone()
            };
            
            // Position each item in the line
            for &idx in &indices {
                let child = &mut layout_box.children[idx];
                let item = &flex_items[idx];
                
                // Set main axis position
                if is_row {
                    child.bounds.x = main_pos + child.style.margin.left;
                    child.margin.x = main_pos;
                    child.border.x = main_pos + child.style.margin.left;
                    child.padding.x = child.border.x + child.style.border.left;
                    child.content.x = child.padding.x + child.style.padding.left;
                } else {
                    child.bounds.y = main_pos + child.style.margin.top;
                    child.margin.y = main_pos;
                    child.border.y = main_pos + child.style.margin.top;
                    child.padding.y = child.border.y + child.style.border.top;
                    child.content.y = child.padding.y + child.style.padding.top;
                }
                
                // Set cross axis position based on align-items
                let cross_offset = match align {
                    AlignItems::FlexStart => 0.0,
                    AlignItems::FlexEnd => line.max_cross - item.cross_size,
                    AlignItems::Center => (line.max_cross - item.cross_size) / 2.0,
                    AlignItems::Stretch => 0.0, // TODO: stretch the item
                    AlignItems::Baseline => 0.0, // TODO: baseline alignment
                };
                
                if is_row {
                    child.bounds.y = cross_pos + cross_offset + child.style.margin.top;
                    child.margin.y = cross_pos + cross_offset;
                    child.border.y = child.margin.y + child.style.margin.top;
                    child.padding.y = child.border.y + child.style.border.top;
                    child.content.y = child.padding.y + child.style.padding.top;
                } else {
                    child.bounds.x = cross_pos + cross_offset + child.style.margin.left;
                    child.margin.x = cross_pos + cross_offset;
                    child.border.x = child.margin.x + child.style.margin.left;
                    child.padding.x = child.border.x + child.style.border.left;
                    child.content.x = child.padding.x + child.style.padding.left;
                }
                
                main_pos += item.main_size + gap;
            }
            
            cross_pos += line.max_cross;
        }
        
        // Set container height based on content
        let total_cross: f32 = lines.iter().map(|l| l.max_cross).sum();
        if is_row {
            layout_box.content.height = total_cross;
        } else {
            layout_box.content.width = total_cross;
        }
        
        self.calculate_block_height(layout_box);
    }

    fn calculate_block_width(&mut self, layout_box: &mut LayoutBox) {
        let style = &layout_box.style;
        let container_width = layout_box.bounds.width;
        
        let width = match style.width {
            Dimension::Auto => container_width - style.margin.left - style.margin.right
                - style.padding.left - style.padding.right - style.border.left - style.border.right,
            Dimension::Px(px) => px,
            Dimension::Percent(pct) => container_width * pct / 100.0,
            Dimension::Em(em) => em * self.font_size,
            Dimension::Rem(rem) => rem * self.font_size,
            Dimension::Vw(vw) => self.viewport_width * vw / 100.0,
            Dimension::Vh(vh) => self.viewport_height * vh / 100.0,
        };
        
        layout_box.content.width = width.max(0.0);
        layout_box.padding.width = layout_box.content.width + style.padding.left + style.padding.right;
        layout_box.border.width = layout_box.padding.width + style.border.left + style.border.right;
        layout_box.margin.width = layout_box.border.width + style.margin.left + style.margin.right;
    }

    fn calculate_block_position(&mut self, layout_box: &mut LayoutBox) {
        let style = &layout_box.style;
        layout_box.margin.x = layout_box.bounds.x;
        layout_box.border.x = layout_box.margin.x + style.margin.left;
        layout_box.padding.x = layout_box.border.x + style.border.left;
        layout_box.content.x = layout_box.padding.x + style.padding.left;
        layout_box.margin.y = layout_box.bounds.y;
        layout_box.border.y = layout_box.margin.y + style.margin.top;
        layout_box.padding.y = layout_box.border.y + style.border.top;
        layout_box.content.y = layout_box.padding.y + style.padding.top;
    }

    fn layout_block_children(&mut self, layout_box: &mut LayoutBox) {
        let mut y = layout_box.content.y;
        let x = layout_box.content.x;
        let width = layout_box.content.width;
        
        let has_inline = layout_box.children.iter().any(|c| matches!(c.box_type, BoxType::Inline | BoxType::Text));
        
        if has_inline {
            let mut line_x = x;
            let mut line_height: f32 = 0.0;
            
            for child in &mut layout_box.children {
                child.bounds.x = line_x;
                child.bounds.y = y;
                child.bounds.width = width;
                self.calculate_layout(child);
                
                let child_width = child.margin.width;
                let child_height = child.margin.height;
                
                if line_x + child_width > x + width && line_x > x {
                    y += line_height;
                    line_x = x;
                    line_height = 0.0;
                    child.bounds.x = line_x;
                    child.bounds.y = y;
                }
                
                line_x += child_width;
                line_height = line_height.max(child_height);
            }
            y += line_height;
        } else {
            for child in &mut layout_box.children {
                child.bounds.x = x;
                child.bounds.y = y;
                child.bounds.width = width;
                self.calculate_layout(child);
                y += child.margin.height;
            }
        }
        
        layout_box.content.height = y - layout_box.content.y;
    }

    fn calculate_block_height(&mut self, layout_box: &mut LayoutBox) {
        let style = &layout_box.style;
        
        if let Dimension::Px(h) = style.height {
            layout_box.content.height = h;
        } else if let Dimension::Percent(pct) = style.height {
            layout_box.content.height = self.viewport_height * pct / 100.0;
        }
        
        layout_box.padding.height = layout_box.content.height + style.padding.top + style.padding.bottom;
        layout_box.border.height = layout_box.padding.height + style.border.top + style.border.bottom;
        layout_box.margin.height = layout_box.border.height + style.margin.top + style.margin.bottom;
        layout_box.bounds.height = layout_box.margin.height;
    }

    fn layout_inline(&mut self, layout_box: &mut LayoutBox) {
        let style = &layout_box.style;
        let font_size = if style.font_size > 0.0 { style.font_size } else { self.font_size };
        
        let text_width = layout_box.text.as_ref().map(|t| t.len() as f32 * font_size * 0.6).unwrap_or(0.0);
        let width = match style.width {
            Dimension::Px(px) => px,
            _ => text_width,
        };
        
        layout_box.content.width = width;
        layout_box.content.height = font_size * layout_box.style.line_height;
        layout_box.content.x = layout_box.bounds.x + style.margin.left + style.border.left + style.padding.left;
        layout_box.content.y = layout_box.bounds.y + style.margin.top + style.border.top + style.padding.top;
        
        layout_box.padding.x = layout_box.content.x - style.padding.left;
        layout_box.padding.y = layout_box.content.y - style.padding.top;
        layout_box.padding.width = layout_box.content.width + style.padding.left + style.padding.right;
        layout_box.padding.height = layout_box.content.height + style.padding.top + style.padding.bottom;
        
        layout_box.border.x = layout_box.padding.x - style.border.left;
        layout_box.border.y = layout_box.padding.y - style.border.top;
        layout_box.border.width = layout_box.padding.width + style.border.left + style.border.right;
        layout_box.border.height = layout_box.padding.height + style.border.top + style.border.bottom;
        
        layout_box.margin.x = layout_box.border.x - style.margin.left;
        layout_box.margin.y = layout_box.border.y - style.margin.top;
        layout_box.margin.width = layout_box.border.width + style.margin.left + style.margin.right;
        layout_box.margin.height = layout_box.border.height + style.margin.top + style.margin.bottom;
        layout_box.bounds.height = layout_box.margin.height;
    }

    fn layout_text(&mut self, layout_box: &mut LayoutBox) {
        let font_size = if layout_box.style.font_size > 0.0 { layout_box.style.font_size } else { self.font_size };
        let text = layout_box.text.as_deref().unwrap_or("");
        
        layout_box.content.width = text.len() as f32 * font_size * 0.6;
        layout_box.content.height = font_size * layout_box.style.line_height;
        layout_box.content.x = layout_box.bounds.x;
        layout_box.content.y = layout_box.bounds.y;
        
        layout_box.padding = layout_box.content;
        layout_box.border = layout_box.content;
        layout_box.margin = layout_box.content;
        layout_box.bounds.height = layout_box.content.height;
    }

    #[allow(clippy::only_used_in_recursion)]
    pub fn hit_test(&self, layout: &LayoutBox, x: f32, y: f32) -> Option<NodeRef> {
        for child in layout.children.iter().rev() {
            if let Some(node) = self.hit_test(child, x, y) { return Some(node); }
        }
        if layout.border.contains(x, y) {
            if let Some(ref node) = layout.node {
                let n = node.borrow();
                if n.node_type == NodeType::Element { return Some(node.clone()); }
            }
        }
        None
    }

    #[allow(clippy::only_used_in_recursion)]
    pub fn find_layout_for_node<'a>(&self, layout: &'a LayoutBox, target: &NodeRef) -> Option<&'a LayoutBox> {
        if let Some(ref node) = layout.node {
            if std::rc::Rc::ptr_eq(node, target) { return Some(layout); }
        }
        for child in &layout.children {
            if let Some(found) = self.find_layout_for_node(child, target) { return Some(found); }
        }
        None
    }
}
