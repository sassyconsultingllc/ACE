//! CSS Layout Engine - Bridges cssparser -> taffy -> egui
//! 
//! This module integrates:
//! - html5ever for DOM parsing (already have)
//! - cssparser for CSS parsing (already have)
//! - taffy for layout computation (Block, Flexbox, Grid)
//! - egui for rendering
//!
//! The flow:
//! HTML -> DOM Tree -> Style Resolution -> Layout Tree (taffy) -> Paint -> egui

use taffy::prelude::*;
use taffy::{Overflow, Point};

// ==============================================================================
// STYLE RESOLUTION
// ==============================================================================

/// Computed styles for a DOM node
#[derive(Debug, Clone)]
pub struct ComputedStyle {
    // Display
    pub display: Display,
    
    // Flexbox
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub justify_content: Option<JustifyContent>,
    pub align_items: Option<AlignItems>,
    pub align_content: Option<AlignContent>,
    pub align_self: Option<AlignSelf>,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Dimension,
    
    // Grid
    pub grid_template_columns: Vec<TrackSizingFunction>,
    pub grid_template_rows: Vec<TrackSizingFunction>,
    pub grid_column: Line<GridPlacement>,
    pub grid_row: Line<GridPlacement>,
    pub gap: Size<LengthPercentage>,
    
    // Sizing
    pub width: Dimension,
    pub height: Dimension,
    pub min_width: Dimension,
    pub min_height: Dimension,
    pub max_width: Dimension,
    pub max_height: Dimension,
    
    // Spacing
    pub margin: Rect<LengthPercentageAuto>,
    pub padding: Rect<LengthPercentage>,
    pub border: Rect<LengthPercentage>,
    
    // Position
    pub position: Position,
    pub inset: Rect<LengthPercentageAuto>,
    
    // Text
    pub color: [u8; 4],
    pub background_color: [u8; 4],
    pub font_size: f32,
    pub font_weight: u16,
    pub text_align: TextAlign,
    pub line_height: f32,
    
    // Overflow
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self {
            display: Display::Block,
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::NoWrap,
            justify_content: None,
            align_items: None,
            align_content: None,
            align_self: None,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Dimension::Auto,
            grid_template_columns: Vec::new(),
            grid_template_rows: Vec::new(),
            grid_column: Line { start: GridPlacement::Auto, end: GridPlacement::Auto },
            grid_row: Line { start: GridPlacement::Auto, end: GridPlacement::Auto },
            gap: Size { width: LengthPercentage::Length(0.0), height: LengthPercentage::Length(0.0) },
            width: Dimension::Auto,
            height: Dimension::Auto,
            min_width: Dimension::Auto,
            min_height: Dimension::Auto,
            max_width: Dimension::Auto,
            max_height: Dimension::Auto,
            margin: Rect {
                top: LengthPercentageAuto::Length(0.0),
                right: LengthPercentageAuto::Length(0.0),
                bottom: LengthPercentageAuto::Length(0.0),
                left: LengthPercentageAuto::Length(0.0),
            },
            padding: Rect {
                top: LengthPercentage::Length(0.0),
                right: LengthPercentage::Length(0.0),
                bottom: LengthPercentage::Length(0.0),
                left: LengthPercentage::Length(0.0),
            },
            border: Rect {
                top: LengthPercentage::Length(0.0),
                right: LengthPercentage::Length(0.0),
                bottom: LengthPercentage::Length(0.0),
                left: LengthPercentage::Length(0.0),
            },
            position: Position::Relative,
            inset: Rect {
                top: LengthPercentageAuto::Auto,
                right: LengthPercentageAuto::Auto,
                bottom: LengthPercentageAuto::Auto,
                left: LengthPercentageAuto::Auto,
            },
            color: [0, 0, 0, 255],
            background_color: [255, 255, 255, 255],
            font_size: 16.0,
            font_weight: 400,
            text_align: TextAlign::Left,
            line_height: 1.2,
            overflow_x: Overflow::Visible,
            overflow_y: Overflow::Visible,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

impl ComputedStyle {
    /// Describe visual style properties for diagnostics
    pub fn describe_visual(&self) -> String {
        format!(
            "Visual[color=({},{},{},{}), bg=({},{},{},{}), font_size={}, weight={}, align={:?}, line_height={}, overflow=({:?},{:?})]",
            self.color[0], self.color[1], self.color[2], self.color[3],
            self.background_color[0], self.background_color[1], self.background_color[2], self.background_color[3],
            self.font_size, self.font_weight, self.text_align, self.line_height,
            self.overflow_x, self.overflow_y,
        )
    }

    /// Convert to taffy Style
    pub fn to_taffy_style(&self) -> Style {
        Style {
            display: self.display,
            position: self.position,
            flex_direction: self.flex_direction,
            flex_wrap: self.flex_wrap,
            justify_content: self.justify_content,
            align_items: self.align_items,
            align_content: self.align_content,
            align_self: self.align_self,
            flex_grow: self.flex_grow,
            flex_shrink: self.flex_shrink,
            flex_basis: self.flex_basis,
            size: Size { width: self.width, height: self.height },
            min_size: Size { width: self.min_width, height: self.min_height },
            max_size: Size { width: self.max_width, height: self.max_height },
            margin: self.margin,
            padding: self.padding,
            border: self.border,
            inset: self.inset,
            gap: self.gap,
            overflow: Point { x: self.overflow_x, y: self.overflow_y },
            grid_template_columns: self.grid_template_columns.clone(),
            grid_template_rows: self.grid_template_rows.clone(),
            grid_column: self.grid_column,
            grid_row: self.grid_row,
            ..Default::default()
        }
    }
}

// ==============================================================================
// CSS PARSER INTEGRATION
// ==============================================================================

/// Parse CSS value to taffy Dimension
pub fn parse_dimension(value: &str) -> Dimension {
    let value = value.trim();
    
    if value == "auto" {
        return Dimension::Auto;
    }
    
    if let Some(stripped) = value.strip_suffix("px") {
        if let Ok(v) = stripped.trim().parse::<f32>() {
            return Dimension::Length(v);
        }
    }
    
    if let Some(stripped) = value.strip_suffix('%') {
        if let Ok(v) = stripped.trim().parse::<f32>() {
            return Dimension::Percent(v / 100.0);
        }
    }
    
    if let Some(stripped) = value.strip_suffix("em") {
        if let Ok(v) = stripped.trim().parse::<f32>() {
            // Convert em to px (assume 16px base)
            return Dimension::Length(v * 16.0);
        }
    }
    
    if let Some(stripped) = value.strip_suffix("rem") {
        if let Ok(v) = stripped.trim().parse::<f32>() {
            return Dimension::Length(v * 16.0);
        }
    }
    
    if let Some(stripped) = value.strip_suffix("vw") {
        if let Ok(v) = stripped.trim().parse::<f32>() {
            // Viewport width - approximate
            return Dimension::Percent(v / 100.0);
        }
    }
    
    if let Some(stripped) = value.strip_suffix("vh") {
        if let Ok(v) = stripped.trim().parse::<f32>() {
            return Dimension::Percent(v / 100.0);
        }
    }
    
    // Try parsing as raw number (treat as px)
    if let Ok(v) = value.parse::<f32>() {
        return Dimension::Length(v);
    }
    
    Dimension::Auto
}

/// Parse CSS length/percentage value
pub fn parse_length_percentage(value: &str) -> LengthPercentage {
    let value = value.trim();
    
    if let Some(stripped) = value.strip_suffix("px") {
        if let Ok(v) = stripped.trim().parse::<f32>() {
            return LengthPercentage::Length(v);
        }
    }
    
    if let Some(stripped) = value.strip_suffix('%') {
        if let Ok(v) = stripped.trim().parse::<f32>() {
            return LengthPercentage::Percent(v / 100.0);
        }
    }
    
    if let Ok(v) = value.parse::<f32>() {
        return LengthPercentage::Length(v);
    }
    
    LengthPercentage::Length(0.0)
}

/// Parse CSS length/percentage/auto value
pub fn parse_length_percentage_auto(value: &str) -> LengthPercentageAuto {
    let value = value.trim();
    
    if value == "auto" {
        return LengthPercentageAuto::Auto;
    }
    
    if let Some(stripped) = value.strip_suffix("px") {
        if let Ok(v) = stripped.trim().parse::<f32>() {
            return LengthPercentageAuto::Length(v);
        }
    }
    
    if let Some(stripped) = value.strip_suffix('%') {
        if let Ok(v) = stripped.trim().parse::<f32>() {
            return LengthPercentageAuto::Percent(v / 100.0);
        }
    }
    
    if let Ok(v) = value.parse::<f32>() {
        return LengthPercentageAuto::Length(v);
    }
    
    LengthPercentageAuto::Auto
}

/// Parse display property
pub fn parse_display(value: &str) -> Display {
    match value.trim().to_lowercase().as_str() {
        "none" => Display::None,
        "block" => Display::Block,
        "flex" => Display::Flex,
        "grid" => Display::Grid,
        "inline" | "inline-block" => Display::Block, // Simplified
        _ => Display::Block,
    }
}

/// Parse flex-direction
pub fn parse_flex_direction(value: &str) -> FlexDirection {
    match value.trim().to_lowercase().as_str() {
        "row" => FlexDirection::Row,
        "row-reverse" => FlexDirection::RowReverse,
        "column" => FlexDirection::Column,
        "column-reverse" => FlexDirection::ColumnReverse,
        _ => FlexDirection::Row,
    }
}

/// Parse flex-wrap
pub fn parse_flex_wrap(value: &str) -> FlexWrap {
    match value.trim().to_lowercase().as_str() {
        "nowrap" => FlexWrap::NoWrap,
        "wrap" => FlexWrap::Wrap,
        "wrap-reverse" => FlexWrap::WrapReverse,
        _ => FlexWrap::NoWrap,
    }
}

/// Parse justify-content
pub fn parse_justify_content(value: &str) -> Option<JustifyContent> {
    Some(match value.trim().to_lowercase().as_str() {
        "flex-start" | "start" => JustifyContent::Start,
        "flex-end" | "end" => JustifyContent::End,
        "center" => JustifyContent::Center,
        "space-between" => JustifyContent::SpaceBetween,
        "space-around" => JustifyContent::SpaceAround,
        "space-evenly" => JustifyContent::SpaceEvenly,
        "stretch" => JustifyContent::Stretch,
        _ => return None,
    })
}

/// Parse align-items
pub fn parse_align_items(value: &str) -> Option<AlignItems> {
    Some(match value.trim().to_lowercase().as_str() {
        "flex-start" | "start" => AlignItems::Start,
        "flex-end" | "end" => AlignItems::End,
        "center" => AlignItems::Center,
        "baseline" => AlignItems::Baseline,
        "stretch" => AlignItems::Stretch,
        _ => return None,
    })
}

/// Parse position
pub fn parse_position(value: &str) -> Position {
    match value.trim().to_lowercase().as_str() {
        "relative" => Position::Relative,
        "absolute" => Position::Absolute,
        _ => Position::Relative,
    }
}

/// Parse overflow
pub fn parse_overflow(value: &str) -> Overflow {
    match value.trim().to_lowercase().as_str() {
        "visible" => Overflow::Visible,
        "hidden" => Overflow::Hidden,
        "clip" => Overflow::Clip,
        "scroll" | "auto" => Overflow::Scroll,
        _ => Overflow::Visible,
    }
}

/// Parse color (returns RGBA)
pub fn parse_color(value: &str) -> [u8; 4] {
    let value = value.trim().to_lowercase();
    
    // Named colors
    let named = match value.as_str() {
        "black" => Some([0, 0, 0, 255]),
        "white" => Some([255, 255, 255, 255]),
        "red" => Some([255, 0, 0, 255]),
        "green" => Some([0, 128, 0, 255]),
        "blue" => Some([0, 0, 255, 255]),
        "yellow" => Some([255, 255, 0, 255]),
        "cyan" | "aqua" => Some([0, 255, 255, 255]),
        "magenta" | "fuchsia" => Some([255, 0, 255, 255]),
        "gray" | "grey" => Some([128, 128, 128, 255]),
        "silver" => Some([192, 192, 192, 255]),
        "maroon" => Some([128, 0, 0, 255]),
        "olive" => Some([128, 128, 0, 255]),
        "navy" => Some([0, 0, 128, 255]),
        "purple" => Some([128, 0, 128, 255]),
        "teal" => Some([0, 128, 128, 255]),
        "orange" => Some([255, 165, 0, 255]),
        "pink" => Some([255, 192, 203, 255]),
        "transparent" => Some([0, 0, 0, 0]),
        "inherit" | "initial" | "unset" => None,
        _ => None,
    };
    
    if let Some(c) = named {
        return c;
    }
    
    // Hex colors
    if let Some(hex) = value.strip_prefix('#') {
        return parse_hex_color(hex);
    }
    
    // rgb() / rgba()
    if value.starts_with("rgb") {
        return parse_rgb_color(&value);
    }
    
    // Default: black
    [0, 0, 0, 255]
}

fn parse_hex_color(hex: &str) -> [u8; 4] {
    let hex = hex.trim();
    
    match hex.len() {
        3 => {
            // #RGB -> #RRGGBB
            let r = u8::from_str_radix(&hex[0..1], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[1..2], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[2..3], 16).unwrap_or(0);
            [r * 17, g * 17, b * 17, 255]
        }
        4 => {
            // #RGBA
            let r = u8::from_str_radix(&hex[0..1], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[1..2], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[2..3], 16).unwrap_or(0);
            let a = u8::from_str_radix(&hex[3..4], 16).unwrap_or(15);
            [r * 17, g * 17, b * 17, a * 17]
        }
        6 => {
            // #RRGGBB
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
            [r, g, b, 255]
        }
        8 => {
            // #RRGGBBAA
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
            let a = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255);
            [r, g, b, a]
        }
        _ => [0, 0, 0, 255],
    }
}

fn parse_rgb_color(value: &str) -> [u8; 4] {
    // Extract numbers from rgb(r, g, b) or rgba(r, g, b, a)
    let start = value.find('(').unwrap_or(0) + 1;
    let end = value.find(')').unwrap_or(value.len());
    let inner = &value[start..end];
    
    let parts: Vec<&str> = inner.split(',').collect();
    
    let r = parts.get(0).and_then(|s| s.trim().parse::<u8>().ok()).unwrap_or(0);
    let g = parts.get(1).and_then(|s| s.trim().parse::<u8>().ok()).unwrap_or(0);
    let b = parts.get(2).and_then(|s| s.trim().parse::<u8>().ok()).unwrap_or(0);
    let a = parts.get(3).and_then(|s| {
        let s = s.trim();
        if s.contains('.') {
            s.parse::<f32>().ok().map(|f| (f * 255.0) as u8)
        } else {
            s.parse::<u8>().ok()
        }
    }).unwrap_or(255);
    
    [r, g, b, a]
}

// ==============================================================================
// STYLE RESOLUTION FROM CSS DECLARATIONS
// ==============================================================================

impl ComputedStyle {
    /// Apply a CSS property-value pair
    pub fn apply_property(&mut self, property: &str, value: &str) {
        let prop = property.trim().to_lowercase();
        let val = value.trim();
        
        match prop.as_str() {
            // Display
            "display" => self.display = parse_display(val),
            
            // Flexbox
            "flex-direction" => self.flex_direction = parse_flex_direction(val),
            "flex-wrap" => self.flex_wrap = parse_flex_wrap(val),
            "justify-content" => self.justify_content = parse_justify_content(val),
            "align-items" => self.align_items = parse_align_items(val),
            "align-content" => {
                self.align_content = match val.to_lowercase().as_str() {
                    "flex-start" | "start" => Some(AlignContent::Start),
                    "flex-end" | "end" => Some(AlignContent::End),
                    "center" => Some(AlignContent::Center),
                    "space-between" => Some(AlignContent::SpaceBetween),
                    "space-around" => Some(AlignContent::SpaceAround),
                    "space-evenly" => Some(AlignContent::SpaceEvenly),
                    "stretch" => Some(AlignContent::Stretch),
                    _ => None,
                };
            }
            "align-self" => {
                self.align_self = match val.to_lowercase().as_str() {
                    "auto" => None,
                    "flex-start" | "start" => Some(AlignSelf::Start),
                    "flex-end" | "end" => Some(AlignSelf::End),
                    "center" => Some(AlignSelf::Center),
                    "baseline" => Some(AlignSelf::Baseline),
                    "stretch" => Some(AlignSelf::Stretch),
                    _ => None,
                };
            }
            "flex-grow" => self.flex_grow = val.parse().unwrap_or(0.0),
            "flex-shrink" => self.flex_shrink = val.parse().unwrap_or(1.0),
            "flex-basis" => self.flex_basis = parse_dimension(val),
            "flex" => {
                // Shorthand: flex: grow shrink basis
                let parts: Vec<&str> = val.split_whitespace().collect();
                if parts.len() >= 1 {
                    self.flex_grow = parts[0].parse().unwrap_or(0.0);
                }
                if parts.len() >= 2 {
                    self.flex_shrink = parts[1].parse().unwrap_or(1.0);
                }
                if parts.len() >= 3 {
                    self.flex_basis = parse_dimension(parts[2]);
                }
            }
            
            // Gap
            "gap" | "grid-gap" => {
                let lp = parse_length_percentage(val);
                self.gap = Size { width: lp, height: lp };
            }
            "row-gap" => self.gap.height = parse_length_percentage(val),
            "column-gap" => self.gap.width = parse_length_percentage(val),
            
            // Sizing
            "width" => self.width = parse_dimension(val),
            "height" => self.height = parse_dimension(val),
            "min-width" => self.min_width = parse_dimension(val),
            "min-height" => self.min_height = parse_dimension(val),
            "max-width" => self.max_width = parse_dimension(val),
            "max-height" => self.max_height = parse_dimension(val),
            
            // Margin
            "margin" => {
                let parts: Vec<&str> = val.split_whitespace().collect();
                let (top, right, bottom, left) = parse_box_shorthand(&parts);
                self.margin = Rect { top, right, bottom, left };
            }
            "margin-top" => self.margin.top = parse_length_percentage_auto(val),
            "margin-right" => self.margin.right = parse_length_percentage_auto(val),
            "margin-bottom" => self.margin.bottom = parse_length_percentage_auto(val),
            "margin-left" => self.margin.left = parse_length_percentage_auto(val),
            
            // Padding
            "padding" => {
                let parts: Vec<&str> = val.split_whitespace().collect();
                let lp = |v: LengthPercentageAuto| match v {
                    LengthPercentageAuto::Length(l) => LengthPercentage::Length(l),
                    LengthPercentageAuto::Percent(p) => LengthPercentage::Percent(p),
                    LengthPercentageAuto::Auto => LengthPercentage::Length(0.0),
                };
                let (top, right, bottom, left) = parse_box_shorthand(&parts);
                self.padding = Rect { 
                    top: lp(top), 
                    right: lp(right), 
                    bottom: lp(bottom), 
                    left: lp(left) 
                };
            }
            "padding-top" => self.padding.top = parse_length_percentage(val),
            "padding-right" => self.padding.right = parse_length_percentage(val),
            "padding-bottom" => self.padding.bottom = parse_length_percentage(val),
            "padding-left" => self.padding.left = parse_length_percentage(val),
            
            // Border (width only for layout)
            "border-width" => {
                let lp = parse_length_percentage(val);
                self.border = Rect { top: lp, right: lp, bottom: lp, left: lp };
            }
            "border" => {
                // Extract width from shorthand (e.g., "1px solid black")
                let parts: Vec<&str> = val.split_whitespace().collect();
                for part in parts {
                    if part.ends_with("px") || part.parse::<f32>().is_ok() {
                        let lp = parse_length_percentage(part);
                        self.border = Rect { top: lp, right: lp, bottom: lp, left: lp };
                        break;
                    }
                }
            }
            
            // Position
            "position" => self.position = parse_position(val),
            "top" => self.inset.top = parse_length_percentage_auto(val),
            "right" => self.inset.right = parse_length_percentage_auto(val),
            "bottom" => self.inset.bottom = parse_length_percentage_auto(val),
            "left" => self.inset.left = parse_length_percentage_auto(val),
            
            // Overflow
            "overflow" => {
                let o = parse_overflow(val);
                self.overflow_x = o;
                self.overflow_y = o;
            }
            "overflow-x" => self.overflow_x = parse_overflow(val),
            "overflow-y" => self.overflow_y = parse_overflow(val),
            
            // Text/Visual (not for layout, but for rendering)
            "color" => self.color = parse_color(val),
            "background-color" | "background" => self.background_color = parse_color(val),
            "font-size" => {
                self.font_size = if let Some(stripped) = val.strip_suffix("px") {
                    stripped.trim().parse().unwrap_or(16.0)
                } else if let Some(stripped) = val.strip_suffix("em") {
                    stripped.trim().parse::<f32>().unwrap_or(1.0) * 16.0
                } else if let Some(stripped) = val.strip_suffix("rem") {
                    stripped.trim().parse::<f32>().unwrap_or(1.0) * 16.0
                } else {
                    match val {
                        "xx-small" => 10.0,
                        "x-small" => 12.0,
                        "small" => 13.0,
                        "medium" => 16.0,
                        "large" => 18.0,
                        "x-large" => 24.0,
                        "xx-large" => 32.0,
                        _ => val.parse().unwrap_or(16.0),
                    }
                };
            }
            "font-weight" => {
                self.font_weight = match val {
                    "normal" => 400,
                    "bold" => 700,
                    "lighter" => 300,
                    "bolder" => 800,
                    _ => val.parse().unwrap_or(400),
                };
            }
            "text-align" => {
                self.text_align = match val.to_lowercase().as_str() {
                    "left" => TextAlign::Left,
                    "center" => TextAlign::Center,
                    "right" => TextAlign::Right,
                    "justify" => TextAlign::Justify,
                    _ => TextAlign::Left,
                };
            }
            "line-height" => {
                self.line_height = if let Some(stripped) = val.strip_suffix("px") {
                    stripped.trim().parse().unwrap_or(1.2)
                } else if val == "normal" {
                    1.2
                } else {
                    val.parse().unwrap_or(1.2)
                };
            }
            
            _ => {} // Ignore unknown properties
        }
    }
}

/// Parse box model shorthand (margin, padding, etc.)
fn parse_box_shorthand(parts: &[&str]) -> (LengthPercentageAuto, LengthPercentageAuto, LengthPercentageAuto, LengthPercentageAuto) {
    match parts.len() {
        1 => {
            let v = parse_length_percentage_auto(parts[0]);
            (v, v, v, v)
        }
        2 => {
            let tb = parse_length_percentage_auto(parts[0]);
            let lr = parse_length_percentage_auto(parts[1]);
            (tb, lr, tb, lr)
        }
        3 => {
            let t = parse_length_percentage_auto(parts[0]);
            let lr = parse_length_percentage_auto(parts[1]);
            let b = parse_length_percentage_auto(parts[2]);
            (t, lr, b, lr)
        }
        4 => {
            let t = parse_length_percentage_auto(parts[0]);
            let r = parse_length_percentage_auto(parts[1]);
            let b = parse_length_percentage_auto(parts[2]);
            let l = parse_length_percentage_auto(parts[3]);
            (t, r, b, l)
        }
        _ => {
            let zero = LengthPercentageAuto::Length(0.0);
            (zero, zero, zero, zero)
        }
    }
}

// ==============================================================================
// LAYOUT TREE
// ==============================================================================

/// A node in the layout tree with computed layout
#[derive(Debug)]
pub struct LayoutNode {
    pub node_id: NodeId,
    pub tag: String,
    pub text: Option<String>,
    pub style: ComputedStyle,
    pub children: Vec<usize>, // Indices into LayoutTree.nodes
}

impl LayoutNode {
    /// Summary for diagnostics - reads tag, text, and style fields
    pub fn describe(&self) -> String {
        format!("LayoutNode[tag={}, text={}, font_size={}, display={:?}]",
            self.tag,
            self.text.as_deref().unwrap_or("(none)"),
            self.style.font_size,
            self.style.display,
        )
    }
}

/// The full layout tree
pub struct LayoutTree {
    pub taffy: TaffyTree<()>,
    pub nodes: Vec<LayoutNode>,
    pub root: Option<NodeId>,
}

impl LayoutTree {
    pub fn new() -> Self {
        Self {
            taffy: TaffyTree::new(),
            nodes: Vec::new(),
            root: None,
        }
    }
    
    /// Add a node to the tree
    pub fn add_node(&mut self, tag: &str, text: Option<String>, style: ComputedStyle) -> (usize, NodeId) {
        let taffy_style = style.to_taffy_style();
        let node_id = self.taffy.new_leaf(taffy_style).expect("create taffy node");
        
        let idx = self.nodes.len();
        self.nodes.push(LayoutNode {
            node_id,
            tag: tag.to_string(),
            text,
            style,
            children: Vec::new(),
        });
        
        (idx, node_id)
    }
    
    /// Set children of a node
    pub fn set_children(&mut self, parent_idx: usize, child_indices: &[usize]) {
        let child_ids: Vec<NodeId> = child_indices.iter()
            .map(|&i| self.nodes[i].node_id)
            .collect();
        
        let parent_id = self.nodes[parent_idx].node_id;
        self.taffy.set_children(parent_id, &child_ids).expect("set children");
        self.nodes[parent_idx].children = child_indices.to_vec();
    }
    
    /// Set root node
    pub fn set_root(&mut self, idx: usize) {
        self.root = Some(self.nodes[idx].node_id);
    }
    
    /// Compute layout
    pub fn compute(&mut self, available_width: f32, available_height: f32) {
        if let Some(root) = self.root {
            let size = Size {
                width: AvailableSpace::Definite(available_width),
                height: AvailableSpace::Definite(available_height),
            };
            self.taffy.compute_layout(root, size).expect("compute layout");
        }
    }
    
    /// Get computed layout for a node
    pub fn get_layout(&self, idx: usize) -> Option<&Layout> {
        let node_id = self.nodes.get(idx)?.node_id;
        self.taffy.layout(node_id).ok()
    }
    
    /// Get all layouts as flat list (for rendering)
    pub fn flatten_layouts(&self) -> Vec<(usize, Layout, &LayoutNode)> {
        let mut result = Vec::new();

        fn walk<'a>(tree: &'a LayoutTree, idx: usize, result: &mut Vec<(usize, Layout, &'a LayoutNode)>) {
            if let Some(node) = tree.nodes.get(idx) {
                if let Some(layout) = tree.get_layout(idx) {
                    result.push((idx, *layout, node));
                }
                for &child_idx in &node.children {
                    walk(tree, child_idx, result);
                }
            }
        }

        if !self.nodes.is_empty() {
            walk(self, 0, &mut result);
        }

        result
    }

    /// Summary for diagnostics - reads tag, text, and style from LayoutNode
    pub fn describe(&self) -> String {
        let flat = self.flatten_layouts();
        let descs: Vec<String> = flat.iter().map(|(_, _, node)| node.describe()).collect();
        format!("LayoutTree[nodes={}, root={:?}, items=[{}]]",
            self.nodes.len(),
            self.root,
            descs.join(", "),
        )
    }
}

impl Default for LayoutTree {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// TESTS
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_dimension() {
        assert!(matches!(parse_dimension("100px"), Dimension::Length(v) if (v - 100.0).abs() < 0.01));
        assert!(matches!(parse_dimension("50%"), Dimension::Percent(v) if (v - 0.5).abs() < 0.01));
        assert!(matches!(parse_dimension("auto"), Dimension::Auto));
        assert!(matches!(parse_dimension("2em"), Dimension::Length(v) if (v - 32.0).abs() < 0.01));
    }
    
    #[test]
    fn test_parse_color() {
        assert_eq!(parse_color("red"), [255, 0, 0, 255]);
        assert_eq!(parse_color("#ff0000"), [255, 0, 0, 255]);
        assert_eq!(parse_color("#f00"), [255, 0, 0, 255]);
        assert_eq!(parse_color("rgb(255, 128, 0)"), [255, 128, 0, 255]);
        assert_eq!(parse_color("rgba(255, 128, 0, 0.5)"), [255, 128, 0, 127]);
    }
    
    #[test]
    fn test_layout_tree() {
        let mut tree = LayoutTree::new();
        
        let mut root_style = ComputedStyle::default();
        root_style.display = Display::Flex;
        root_style.flex_direction = FlexDirection::Column;
        root_style.width = Dimension::Length(800.0);
        root_style.height = Dimension::Length(600.0);
        
        let (root_idx, _) = tree.add_node("div", None, root_style);
        
        let mut child_style = ComputedStyle::default();
        child_style.width = Dimension::Length(200.0);
        child_style.height = Dimension::Length(100.0);
        
        let (child_idx, _) = tree.add_node("div", None, child_style);
        
        tree.set_children(root_idx, &[child_idx]);
        tree.set_root(root_idx);
        tree.compute(800.0, 600.0);
        
        let root_layout = tree.get_layout(root_idx).unwrap();
        assert_eq!(root_layout.size.width, 800.0);
        assert_eq!(root_layout.size.height, 600.0);
        
        let child_layout = tree.get_layout(child_idx).unwrap();
        assert_eq!(child_layout.size.width, 200.0);
        assert_eq!(child_layout.size.height, 100.0);
    }
}
