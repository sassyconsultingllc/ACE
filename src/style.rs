// Style - CSS parsing and style computation

use std::collections::HashMap;
use crate::dom::{NodeRef, NodeType};

#[derive(Debug, Clone, Default)]
pub struct ComputedStyle {
    pub display: Display,
    pub position: Position,
    pub width: Dimension,
    pub height: Dimension,
    pub min_width: Dimension,
    pub min_height: Dimension,
    pub max_width: Dimension,
    pub max_height: Dimension,
    pub margin: EdgeSizes,
    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    pub border_color: Color,
    pub color: Color,
    pub background_color: Color,
    pub font_size: f32,
    pub font_weight: u16,
    pub font_family: String,
    pub text_align: TextAlign,
    pub text_decoration: TextDecoration,
    pub line_height: f32,
    pub overflow: Overflow,
    pub visibility: Visibility,
    pub opacity: f32,
    pub z_index: i32,
    pub cursor: Cursor,
    pub flex_direction: FlexDirection,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub flex_wrap: FlexWrap,
    pub flex_grow: f32,
    pub flex_shrink: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Display { #[default] Block, Inline, InlineBlock, Flex, Grid, None }

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Position { #[default] Static, Relative, Absolute, Fixed, Sticky }

#[derive(Debug, Clone, Copy, Default)]
pub enum Dimension { #[default] Auto, Px(f32), Percent(f32), Em(f32), Rem(f32), Vw(f32), Vh(f32) }

#[derive(Debug, Clone, Copy, Default)]
pub struct EdgeSizes { pub top: f32, pub right: f32, pub bottom: f32, pub left: f32 }

#[derive(Debug, Clone, Copy, Default)]
pub struct Color { pub r: u8, pub g: u8, pub b: u8, pub a: u8 }

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TextAlign { #[default] Left, Center, Right, Justify }

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TextDecoration { #[default] None, Underline, LineThrough, Overline }

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Overflow { #[default] Visible, Hidden, Scroll, Auto }

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Visibility { #[default] Visible, Hidden, Collapse }

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Cursor { #[default] Default, Pointer, Text, Move, NotAllowed, Crosshair, Wait }

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FlexDirection { #[default] Row, RowReverse, Column, ColumnReverse }

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum JustifyContent { #[default] FlexStart, FlexEnd, Center, SpaceBetween, SpaceAround, SpaceEvenly }

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum AlignItems { #[default] Stretch, FlexStart, FlexEnd, Center, Baseline }

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FlexWrap { #[default] NoWrap, Wrap, WrapReverse }

impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self { Color { r, g, b, a } }
    pub fn black() -> Self { Color::new(0, 0, 0, 255) }
    pub fn white() -> Self { Color::new(255, 255, 255, 255) }
    pub fn transparent() -> Self { Color::new(0, 0, 0, 0) }
    pub fn to_u32(self) -> u32 { ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32) }
}

impl ComputedStyle {
    pub fn new() -> Self {
        ComputedStyle {
            color: Color::black(),
            background_color: Color::transparent(),
            font_size: 16.0,
            font_weight: 400,
            font_family: "sans-serif".to_string(),
            line_height: 1.2,
            opacity: 1.0,
            flex_shrink: 1.0,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct StyleRule {
    pub selector: String,
    pub specificity: (u32, u32, u32),
    pub properties: HashMap<String, String>,
}

pub struct Stylesheet {
    pub rules: Vec<StyleRule>,
}

impl Stylesheet {
    pub fn new() -> Self { Stylesheet { rules: Vec::new() } }

    pub fn parse(css: &str) -> Self {
        let mut stylesheet = Stylesheet::new();
        let css = Self::remove_comments(css);
        let mut chars = css.chars().peekable();
        
        while chars.peek().is_some() {
            Self::skip_whitespace(&mut chars);
            if chars.peek().is_none() { break; }
            
            if chars.peek() == Some(&'@') {
                Self::skip_at_rule(&mut chars);
                continue;
            }
            
            let selector = Self::parse_selector(&mut chars);
            if selector.is_empty() { break; }
            
            Self::skip_whitespace(&mut chars);
            if chars.next() != Some('{') { continue; }
            
            let properties = Self::parse_declarations(&mut chars);
            let specificity = Self::calculate_specificity(&selector);
            
            stylesheet.rules.push(StyleRule { selector, specificity, properties });
        }
        stylesheet
    }

    fn remove_comments(css: &str) -> String {
        let mut result = String::new();
        let mut chars = css.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '/' && chars.peek() == Some(&'*') {
                chars.next();
                while let Some(c2) = chars.next() {
                    if c2 == '*' && chars.peek() == Some(&'/') { chars.next(); break; }
                }
            } else { result.push(c); }
        }
        result
    }

    fn skip_whitespace(chars: &mut std::iter::Peekable<std::str::Chars>) {
        while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) { chars.next(); }
    }

    fn skip_at_rule(chars: &mut std::iter::Peekable<std::str::Chars>) {
        let mut brace_count = 0;
        for c in chars.by_ref() {
            match c {
                '{' => brace_count += 1,
                '}' => { brace_count -= 1; if brace_count <= 0 { break; } }
                ';' if brace_count == 0 => break,
                _ => {}
            }
        }
    }

    fn parse_selector(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
        let mut selector = String::new();
        while let Some(&c) = chars.peek() {
            if c == '{' { break; }
            selector.push(chars.next().unwrap());
        }
        selector.trim().to_string()
    }

    fn parse_declarations(chars: &mut std::iter::Peekable<std::str::Chars>) -> HashMap<String, String> {
        let mut props = HashMap::new();
        let mut current = String::new();
        let mut brace_count = 0;
        
        for c in chars.by_ref() {
            match c {
                '{' => { brace_count += 1; current.push(c); }
                '}' => { if brace_count == 0 { break; } brace_count -= 1; current.push(c); }
                ';' if brace_count == 0 => {
                    if let Some((name, value)) = current.split_once(':') {
                        props.insert(crate::fontcase::ascii_lower(name.trim()), value.trim().to_string());
                    }
                    current.clear();
                }
                _ => current.push(c),
            }
        }
        if !current.trim().is_empty() {
            if let Some((name, value)) = current.split_once(':') {
                props.insert(crate::fontcase::ascii_lower(name.trim()), value.trim().to_string());
            }
        }
        props
    }

    fn calculate_specificity(selector: &str) -> (u32, u32, u32) {
        let mut ids = 0;
        let mut classes = 0;
        let mut elements = 0;
        
        for part in selector.split(|c: char| c.is_whitespace() || c == '>' || c == '+' || c == '~') {
            let part = part.trim();
            if part.is_empty() { continue; }
            for segment in part.split('.') {
                if segment.contains('#') {
                    ids += segment.matches('#').count() as u32;
                    if !segment.starts_with('#') { elements += 1; }
                } else if segment.starts_with(':') { classes += 1; }
                else if !segment.is_empty() {
                    if part.starts_with('.') || segment != part.split('.').next().unwrap_or("") { classes += 1; }
                    else { elements += 1; }
                }
            }
        }
        (ids, classes, elements)
    }

    pub fn matches_selector(node: &NodeRef, selector: &str) -> bool {
        let n = node.borrow();
        if n.node_type != NodeType::Element { return false; }
        
        let selector = selector.trim();
        let tag = n.tag_name.as_deref().unwrap_or("");
        
        if selector == "*" { return true; }
        if let Some(stripped) = selector.strip_prefix('#') { return n.get_id().as_deref() == Some(stripped); }
        if let Some(stripped) = selector.strip_prefix('.') { return n.has_class(stripped); }
        
        let parts: Vec<&str> = selector.split(['.', '#']).collect();
        if parts.is_empty() { return false; }
        
        if !parts[0].is_empty() && parts[0] != tag { return false; }
        
        for (_i, part) in parts.iter().enumerate().skip(1) {
            let is_id = selector[..selector.find(part).unwrap_or(0)].ends_with('#');
            if is_id { if n.get_id().as_deref() != Some(*part) { return false; } }
            else if !n.has_class(part) { return false; }
        }
        true
    }
}

impl Default for Stylesheet {
    fn default() -> Self { Self::new() }
}

pub struct StyleEngine {
    pub stylesheets: Vec<Stylesheet>,
    pub user_agent_styles: Stylesheet,
}

impl StyleEngine {
    pub fn new() -> Self {
        StyleEngine {
            stylesheets: Vec::new(),
            user_agent_styles: Self::default_styles(),
        }
    }

    fn default_styles() -> Stylesheet {
        Stylesheet::parse(r#"
            html, body { display: block; margin: 0; padding: 0; }
            body { margin: 8px; }
            h1 { font-size: 2em; font-weight: bold; margin: 0.67em 0; display: block; }
            h2 { font-size: 1.5em; font-weight: bold; margin: 0.83em 0; display: block; }
            h3 { font-size: 1.17em; font-weight: bold; margin: 1em 0; display: block; }
            h4 { font-weight: bold; margin: 1.33em 0; display: block; }
            h5 { font-size: 0.83em; font-weight: bold; margin: 1.67em 0; display: block; }
            h6 { font-size: 0.67em; font-weight: bold; margin: 2.33em 0; display: block; }
            p { margin: 1em 0; display: block; }
            div { display: block; }
            span { display: inline; }
            a { color: #0000EE; text-decoration: underline; cursor: pointer; display: inline; }
            a:visited { color: #551A8B; }
            strong, b { font-weight: bold; display: inline; }
            em, i { font-style: italic; display: inline; }
            ul, ol { margin: 1em 0; padding-left: 40px; display: block; }
            li { display: list-item; }
            img { display: inline-block; }
            table { display: table; border-collapse: separate; border-spacing: 2px; }
            tr { display: table-row; }
            td, th { display: table-cell; padding: 1px; }
            th { font-weight: bold; text-align: center; }
            form { display: block; margin-top: 0em; }
            input { display: inline-block; }
            button { display: inline-block; cursor: pointer; }
            textarea { display: inline-block; }
            select { display: inline-block; }
            pre, code { font-family: monospace; }
            pre { display: block; margin: 1em 0; white-space: pre; }
            code { display: inline; }
            blockquote { margin: 1em 40px; display: block; }
            hr { display: block; margin: 0.5em auto; border: 1px inset; }
            br { display: inline; }
            script, style, head, title, meta, link { display: none; }
        "#)
    }

    pub fn add_stylesheet(&mut self, css: &str) {
        self.stylesheets.push(Stylesheet::parse(css));
    }

    pub fn compute_style(&self, node: &NodeRef) -> ComputedStyle {
        let mut style = ComputedStyle::new();
        let n = node.borrow();
        if n.node_type != NodeType::Element { return style; }

        let mut matched: Vec<(&StyleRule, usize)> = Vec::new();
        
        for rule in &self.user_agent_styles.rules {
            if Stylesheet::matches_selector(node, &rule.selector) {
                matched.push((rule, 0));
            }
        }
        
        for (sheet_idx, sheet) in self.stylesheets.iter().enumerate() {
            for rule in &sheet.rules {
                if Stylesheet::matches_selector(node, &rule.selector) {
                    matched.push((rule, sheet_idx + 1));
                }
            }
        }
        
        matched.sort_by(|a, b| {
            a.0.specificity.cmp(&b.0.specificity).then(a.1.cmp(&b.1))
        });
        
        for (rule, _) in matched {
            self.apply_properties(&mut style, &rule.properties);
        }
        
        if let Some(inline) = n.get_attribute("style") {
            let props = Self::parse_inline_style(&inline);
            self.apply_properties(&mut style, &props);
        }

        // Apply programmatically-set styles from the node's styles map
        // (e.g. set by JavaScript via element.style.xxx)
        if !n.styles.is_empty() {
            self.apply_properties(&mut style, &n.styles);
        }

        style
    }

    fn parse_inline_style(style: &str) -> HashMap<String, String> {
        let mut props = HashMap::new();
        for decl in style.split(';') {
            if let Some((name, value)) = decl.split_once(':') {
                props.insert(crate::fontcase::ascii_lower(name.trim()), value.trim().to_string());
            }
        }
        props
    }

    fn apply_properties(&self, style: &mut ComputedStyle, props: &HashMap<String, String>) {
        for (name, value) in props {
            match name.as_str() {
                "display" => style.display = Self::parse_display(value),
                "position" => style.position = Self::parse_position(value),
                "width" => style.width = Self::parse_dimension(value),
                "height" => style.height = Self::parse_dimension(value),
                "min-width" => style.min_width = Self::parse_dimension(value),
                "min-height" => style.min_height = Self::parse_dimension(value),
                "max-width" => style.max_width = Self::parse_dimension(value),
                "max-height" => style.max_height = Self::parse_dimension(value),
                "margin" => style.margin = Self::parse_edge_sizes(value),
                "margin-top" => style.margin.top = Self::parse_length(value),
                "margin-right" => style.margin.right = Self::parse_length(value),
                "margin-bottom" => style.margin.bottom = Self::parse_length(value),
                "margin-left" => style.margin.left = Self::parse_length(value),
                "padding" => style.padding = Self::parse_edge_sizes(value),
                "padding-top" => style.padding.top = Self::parse_length(value),
                "padding-right" => style.padding.right = Self::parse_length(value),
                "padding-bottom" => style.padding.bottom = Self::parse_length(value),
                "padding-left" => style.padding.left = Self::parse_length(value),
                "border-width" => style.border = Self::parse_edge_sizes(value),
                "border-color" => style.border_color = Self::parse_color(value),
                "border" => { style.border = EdgeSizes { top: 1.0, right: 1.0, bottom: 1.0, left: 1.0 }; style.border_color = Color::black(); },
                "color" => style.color = Self::parse_color(value),
                "background-color" | "background" => style.background_color = Self::parse_color(value),
                "font-size" => style.font_size = Self::parse_length(value),
                "font-weight" => style.font_weight = Self::parse_font_weight(value),
                "font-family" => style.font_family = value.trim_matches('"').trim_matches('\'').to_string(),
                "text-align" => style.text_align = Self::parse_text_align(value),
                "text-decoration" => style.text_decoration = Self::parse_text_decoration(value),
                "line-height" => style.line_height = Self::parse_line_height(value),
                "overflow" => style.overflow = Self::parse_overflow(value),
                "visibility" => style.visibility = Self::parse_visibility(value),
                "opacity" => style.opacity = value.parse().unwrap_or(1.0),
                "z-index" => style.z_index = value.parse().unwrap_or(0),
                "cursor" => style.cursor = Self::parse_cursor(value),
                "flex-direction" => style.flex_direction = Self::parse_flex_direction(value),
                "justify-content" => style.justify_content = Self::parse_justify_content(value),
                "align-items" => style.align_items = Self::parse_align_items(value),
                "flex-wrap" => style.flex_wrap = Self::parse_flex_wrap(value),
                "flex-grow" => style.flex_grow = value.parse().unwrap_or(0.0),
                "flex-shrink" => style.flex_shrink = value.parse().unwrap_or(1.0),
                _ => {}
            }
        }
    }

    fn parse_display(v: &str) -> Display {
        match crate::fontcase::ascii_lower(v.trim()).as_str() {
            "block" => Display::Block, "inline" => Display::Inline,
            "inline-block" => Display::InlineBlock, "flex" => Display::Flex,
            "grid" => Display::Grid, "none" => Display::None, _ => Display::Block
        }
    }

    fn parse_position(v: &str) -> Position {
        match crate::fontcase::ascii_lower(v.trim()).as_str() {
            "static" => Position::Static, "relative" => Position::Relative,
            "absolute" => Position::Absolute, "fixed" => Position::Fixed,
            "sticky" => Position::Sticky, _ => Position::Static
        }
    }

    fn parse_dimension(v: &str) -> Dimension {
        let v = crate::fontcase::ascii_lower(v.trim());
        if v == "auto" { return Dimension::Auto; }
        if v.ends_with('%') { return Dimension::Percent(v[..v.len()-1].parse().unwrap_or(0.0)); }
        if v.ends_with("px") { return Dimension::Px(v[..v.len()-2].parse().unwrap_or(0.0)); }
        if v.ends_with("em") { return Dimension::Em(v[..v.len()-2].parse().unwrap_or(0.0)); }
        if v.ends_with("rem") { return Dimension::Rem(v[..v.len()-3].parse().unwrap_or(0.0)); }
        if v.ends_with("vw") { return Dimension::Vw(v[..v.len()-2].parse().unwrap_or(0.0)); }
        if v.ends_with("vh") { return Dimension::Vh(v[..v.len()-2].parse().unwrap_or(0.0)); }
        Dimension::Px(v.parse().unwrap_or(0.0))
    }

    fn parse_length(v: &str) -> f32 {
        let v = crate::fontcase::ascii_lower(v.trim());
        if v.ends_with("px") { v[..v.len()-2].parse().unwrap_or(0.0) }
        else if v.ends_with("em") { v[..v.len()-2].parse::<f32>().unwrap_or(0.0) * 16.0 }
        else if v.ends_with("rem") { v[..v.len()-3].parse::<f32>().unwrap_or(0.0) * 16.0 }
        else if v.ends_with('%') { v[..v.len()-1].parse::<f32>().unwrap_or(0.0) }
        else { v.parse().unwrap_or(0.0) }
    }

    fn parse_edge_sizes(v: &str) -> EdgeSizes {
        let parts: Vec<f32> = v.split_whitespace().map(Self::parse_length).collect();
        match parts.len() {
            1 => EdgeSizes { top: parts[0], right: parts[0], bottom: parts[0], left: parts[0] },
            2 => EdgeSizes { top: parts[0], right: parts[1], bottom: parts[0], left: parts[1] },
            3 => EdgeSizes { top: parts[0], right: parts[1], bottom: parts[2], left: parts[1] },
            4 => EdgeSizes { top: parts[0], right: parts[1], bottom: parts[2], left: parts[3] },
            _ => EdgeSizes::default()
        }
    }

    fn parse_color(v: &str) -> Color {
        let v = crate::fontcase::ascii_lower(v.trim());
        match v.as_str() {
            "black" => Color::black(), "white" => Color::white(),
            "red" => Color::new(255, 0, 0, 255), "green" => Color::new(0, 128, 0, 255),
            "blue" => Color::new(0, 0, 255, 255), "yellow" => Color::new(255, 255, 0, 255),
            "transparent" => Color::transparent(),
            _ if v.starts_with('#') => Self::parse_hex_color(&v),
            _ if v.starts_with("rgb") => Self::parse_rgb_color(&v),
            _ => Color::black()
        }
    }

    fn parse_hex_color(v: &str) -> Color {
        let hex = &v[1..];
        let (r, g, b) = match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).unwrap_or(0);
                (r, g, b)
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                (r, g, b)
            }
            _ => (0, 0, 0)
        };
        Color::new(r, g, b, 255)
    }

    fn parse_rgb_color(v: &str) -> Color {
        let start = v.find('(').unwrap_or(0) + 1;
        let end = v.find(')').unwrap_or(v.len());
        let parts: Vec<u8> = v[start..end].split(',').filter_map(|p| p.trim().parse().ok()).collect();
        if parts.len() >= 3 { Color::new(parts[0], parts[1], parts[2], 255) }
        else { Color::black() }
    }

    fn parse_font_weight(v: &str) -> u16 {
        match crate::fontcase::ascii_lower(v.trim()).as_str() {
            "normal" => 400, "bold" => 700, "lighter" => 300, "bolder" => 700,
            _ => v.parse().unwrap_or(400)
        }
    }

    fn parse_text_align(v: &str) -> TextAlign {
        match crate::fontcase::ascii_lower(v.trim()).as_str() {
            "left" => TextAlign::Left, "center" => TextAlign::Center,
            "right" => TextAlign::Right, "justify" => TextAlign::Justify, _ => TextAlign::Left
        }
    }

    fn parse_text_decoration(v: &str) -> TextDecoration {
        match crate::fontcase::ascii_lower(v.trim()).as_str() {
            "underline" => TextDecoration::Underline, "line-through" => TextDecoration::LineThrough,
            "overline" => TextDecoration::Overline, _ => TextDecoration::None
        }
    }

    fn parse_line_height(v: &str) -> f32 {
        let v = v.trim();
        if v == "normal" { 1.2 } else { v.parse().unwrap_or(1.2) }
    }

    fn parse_overflow(v: &str) -> Overflow {
        match crate::fontcase::ascii_lower(v.trim()).as_str() {
            "visible" => Overflow::Visible, "hidden" => Overflow::Hidden,
            "scroll" => Overflow::Scroll, "auto" => Overflow::Auto, _ => Overflow::Visible
        }
    }

    fn parse_visibility(v: &str) -> Visibility {
        match crate::fontcase::ascii_lower(v.trim()).as_str() {
            "visible" => Visibility::Visible, "hidden" => Visibility::Hidden,
            "collapse" => Visibility::Collapse, _ => Visibility::Visible
        }
    }

    fn parse_cursor(v: &str) -> Cursor {
        match crate::fontcase::ascii_lower(v.trim()).as_str() {
            "pointer" => Cursor::Pointer, "text" => Cursor::Text, "move" => Cursor::Move,
            "not-allowed" => Cursor::NotAllowed, "crosshair" => Cursor::Crosshair,
            "wait" => Cursor::Wait, _ => Cursor::Default
        }
    }

    fn parse_flex_direction(v: &str) -> FlexDirection {
        match crate::fontcase::ascii_lower(v.trim()).as_str() {
            "row" => FlexDirection::Row, "row-reverse" => FlexDirection::RowReverse,
            "column" => FlexDirection::Column, "column-reverse" => FlexDirection::ColumnReverse,
            _ => FlexDirection::Row
        }
    }

    fn parse_justify_content(v: &str) -> JustifyContent {
        match crate::fontcase::ascii_lower(v.trim()).as_str() {
            "flex-start" => JustifyContent::FlexStart, "flex-end" => JustifyContent::FlexEnd,
            "center" => JustifyContent::Center, "space-between" => JustifyContent::SpaceBetween,
            "space-around" => JustifyContent::SpaceAround, "space-evenly" => JustifyContent::SpaceEvenly,
            _ => JustifyContent::FlexStart
        }
    }

    fn parse_align_items(v: &str) -> AlignItems {
        match crate::fontcase::ascii_lower(v.trim()).as_str() {
            "stretch" => AlignItems::Stretch, "flex-start" => AlignItems::FlexStart,
            "flex-end" => AlignItems::FlexEnd, "center" => AlignItems::Center,
            "baseline" => AlignItems::Baseline, _ => AlignItems::Stretch
        }
    }

    fn parse_flex_wrap(v: &str) -> FlexWrap {
        match crate::fontcase::ascii_lower(v.trim()).as_str() {
            "nowrap" => FlexWrap::NoWrap, "wrap" => FlexWrap::Wrap,
            "wrap-reverse" => FlexWrap::WrapReverse, _ => FlexWrap::NoWrap
        }
    }
}

impl Default for StyleEngine {
    fn default() -> Self { Self::new() }
}
