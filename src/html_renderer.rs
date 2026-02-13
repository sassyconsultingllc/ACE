#![allow(dead_code, unused_variables, unused_imports)]
//! HTML Renderer - Pure Rust HTML/CSS/JS rendering
//!
//! Renders web pages using:
//! - html5ever for HTML parsing
//! - Our JS interpreter for JavaScript
//! - taffy for CSS layout (flexbox/grid)
//! - egui for rendering
//!
//! This is the fallback renderer when system webview isn't available,
//! or for rendering HTML in file viewers.

use crate::js::{JsInterpreter, DomBridge};
use crate::layout_engine::{LayoutTree, ComputedStyle};
use crate::style::{StyleEngine, Stylesheet};
use eframe::egui::{self, Color32, RichText, Ui, Vec2};
use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;
use crate::fontcase;

/// Parsed HTML document
#[derive(Debug, Clone)]
pub struct HtmlDocument {
    pub title: String,
    pub nodes: Vec<HtmlNode>,
    pub styles: Vec<CssRule>,
}

/// HTML node in the DOM tree
#[derive(Debug, Clone)]
pub enum HtmlNode {
    Element {
        tag: String,
        id: Option<String>,
        class: Vec<String>,
        style: HashMap<String, String>,
        attrs: HashMap<String, String>,
        children: Vec<HtmlNode>,
    },
    Text(String),
    Script(String),
}

/// CSS rule
#[derive(Debug, Clone)]
pub struct CssRule {
    pub selector: String,
    pub properties: HashMap<String, String>,
}

/// HTML/CSS renderer with JavaScript support
pub struct HtmlRenderer {
    js: JsInterpreter,
    dom: DomBridge,
    style_engine: StyleEngine,
    scroll_offset: f32,
    hover_link: Option<String>,
    font_size_base: f32,
    warn_color: Color32,
    pub cached_doc: Option<HtmlDocument>,
    image_update_registered: bool,
}

impl HtmlRenderer {
    pub fn new() -> Self {
        let dom = DomBridge::new();
        let js = JsInterpreter::new().with_dom(dom.clone());

        Self {
            js,
            dom,
            style_engine: StyleEngine::new(),
            scroll_offset: 0.0,
            hover_link: None,
            font_size_base: 16.0,
            warn_color: Color32::from_rgb(0xf7, 0x8c, 0x1f),
            cached_doc: None,
            image_update_registered: false,
        }
    }

    /// Add a CSS stylesheet to the style engine
    pub fn add_stylesheet(&mut self, css: &str) {
        self.style_engine.add_stylesheet(css);
    }

    /// Parse CSS using the style engine (for compatibility)
    pub fn parse_stylesheet(&self, css: &str) -> Stylesheet {
        Stylesheet::parse(css)
    }

    /// Set the warning accent color used for highlighted links
    pub fn set_warn_color(&mut self, c: Color32) {
        self.warn_color = c;
    }

    /// Build a taffy layout tree from the cached HTML document
    /// This enables CSS flexbox/grid layout calculation
    pub fn build_layout_tree(&self, available_width: f32, available_height: f32) -> Option<LayoutTree> {
        let doc = self.cached_doc.as_ref()?;
        let mut tree = LayoutTree::new();

        // Build root node with flex column layout
        let mut root_style = ComputedStyle::default();
        root_style.display = taffy::prelude::Display::Flex;
        root_style.flex_direction = taffy::prelude::FlexDirection::Column;
        root_style.width = taffy::prelude::Dimension::Length(available_width);
        root_style.height = taffy::prelude::Dimension::Length(available_height);

        let (root_idx, _) = tree.add_node("root", None, root_style);

        // Recursively build layout nodes from HTML
        let child_indices = self.build_layout_nodes(&mut tree, &doc.nodes, &doc.styles);
        tree.set_children(root_idx, &child_indices);
        tree.set_root(root_idx);

        // Compute layout
        tree.compute(available_width, available_height);

        // Log layout info for debugging (uses get_layout and flatten_layouts)
        if let Some(root_layout) = tree.get_layout(root_idx) {
            tracing::debug!(
                "Layout computed: root size {}x{}",
                root_layout.size.width,
                root_layout.size.height
            );
        }

        // Get all layouts for potential rendering use
        let layouts = tree.flatten_layouts();
        tracing::debug!("Layout tree has {} nodes", layouts.len());

        Some(tree)
    }

    /// Recursively build layout nodes from HTML nodes
    fn build_layout_nodes(&self, tree: &mut LayoutTree, nodes: &[HtmlNode], styles: &[CssRule]) -> Vec<usize> {
        let mut indices = Vec::new();

        for node in nodes {
            match node {
                HtmlNode::Text(text) => {
                    if !text.trim().is_empty() {
                        let style = ComputedStyle::default();
                        let (idx, _) = tree.add_node("text", Some(text.clone()), style);
                        indices.push(idx);
                    }
                }
                HtmlNode::Element { tag, id, class, style: inline_style, children, .. } => {
                    let mut computed = ComputedStyle::default();

                    // Apply inline styles
                    for (prop, val) in inline_style {
                        computed.apply_property(prop, val);
                    }

                    // Apply matching CSS rules
                    for rule in styles {
                        if self.selector_matches(&rule.selector, tag, id.as_deref(), class) {
                            for (prop, val) in &rule.properties {
                                computed.apply_property(prop, val);
                            }
                        }
                    }

                    // Set display based on tag for common block/inline elements
                    match tag.to_lowercase().as_str() {
                        "div" | "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "section" | "article" | "header" | "footer" | "main" | "nav" => {
                            if computed.display == taffy::prelude::Display::Block {
                                computed.display = taffy::prelude::Display::Block;
                            }
                        }
                        "span" | "a" | "strong" | "em" | "b" | "i" | "code" => {
                            // Inline elements - keep as block for now (taffy doesn't do inline)
                        }
                        _ => {}
                    }

                    let (idx, _) = tree.add_node(tag, None, computed);

                    // Build children
                    let child_indices = self.build_layout_nodes(tree, children, styles);
                    tree.set_children(idx, &child_indices);

                    indices.push(idx);
                }
                HtmlNode::Script(_) => {
                    // Skip script nodes in layout
                }
            }
        }

        indices
    }

    /// Check if a CSS selector matches an element
    fn selector_matches(&self, selector: &str, tag: &str, id: Option<&str>, classes: &[String]) -> bool {
        let selector = selector.trim();

        // Tag selector
        if selector.eq_ignore_ascii_case(tag) {
            return true;
        }

        // ID selector
        if let Some(id_val) = id {
            if selector == format!("#{}", id_val) {
                return true;
            }
        }

        // Class selector
        for class in classes {
            if selector == format!(".{}", class) {
                return true;
            }
        }

        // Universal selector
        if selector == "*" {
            return true;
        }

        false
    }

    /// Execute extension/content scripts in the renderer's JS context
    pub fn run_content_scripts(&mut self, scripts: &[String]) {
        for s in scripts {
            let _ = self.js.execute(s);
        }
    }

    /// Apply extension/content styles by parsing and appending to styles
    pub fn apply_content_styles(&mut self, styles: &[String]) {
        // Parse all styles first to avoid borrowing self while mutably borrowing cached_doc
        let mut all_parsed: Vec<CssRule> = Vec::new();
        for css in styles {
            let parsed = self.parse_css(css);
            all_parsed.extend(parsed);
        }

        if let Some(doc) = &mut self.cached_doc {
            doc.styles.extend(all_parsed);
        }
    }

    /// Render a single node with link analysis (for flagged links)
    pub fn render_node_with_link_check<F: Fn(&str) -> Option<&'static str>>(
        &mut self,
        ui: &mut Ui,
        node: &HtmlNode,
        styles: &[CssRule],
        link_check: Option<&F>,
        accent_warn: Color32,
    ) {
        match node {
            HtmlNode::Text(text) => {
                if !text.trim().is_empty() {
                    ui.label(text);
                }
            }
            HtmlNode::Script(_) => {}
            HtmlNode::Element { tag, id, class, style, attrs, children } => {
                let computed = self.compute_styles(tag, id.as_deref(), class, style, styles);
                match tag.as_str() {
                    "a" => {
                        let href = attrs.get("href").cloned().unwrap_or_default();
                        let text = self.text_content(children);
                        let display = if text.is_empty() { &href } else { &text };
                        let flagged_reason = link_check.and_then(|f| f(&href));
                        let link = ui.link(
                            if let Some(reason) = flagged_reason {
                                RichText::new(display).color(accent_warn).underline().strong()
                            } else {
                                RichText::new(display)
                            }
                        ).on_hover_text(
                            if let Some(reason) = flagged_reason {
                                format!("{}\n[Flagged: {}]", href, reason)
                            } else {
                                href.clone()
                            }
                        );
                        if link.clicked() {
                            self.hover_link = Some(href.clone());
                        }
                        // Optionally: prefetch/analyze flagged links here
                    }
                    _ => self.render_node(ui, node, styles),
                }
            }
        }
    }
    
    /// Parse CSS into rules
    fn parse_css(&self, css: &str) -> Vec<CssRule> {
        let mut rules = Vec::new();
        let css = css.trim();
        
        // Simple CSS parser
        let mut pos = 0;
        while pos < css.len() {
            // Find selector
            if let Some(brace_start) = css[pos..].find('{') {
                let selector = css[pos..pos + brace_start].trim().to_string();
                let props_start = pos + brace_start + 1;
                
                if let Some(brace_end) = css[props_start..].find('}') {
                    let props_str = &css[props_start..props_start + brace_end];
                    let mut properties = HashMap::new();
                    
                    for prop in props_str.split(';') {
                        let prop = prop.trim();
                        if let Some(colon) = prop.find(':') {
                            let key = prop[..colon].trim().to_string();
                            let value = prop[colon + 1..].trim().to_string();
                            properties.insert(key, value);
                        }
                    }
                    
                    if !selector.is_empty() {
                        rules.push(CssRule { selector, properties });
                    }
                    
                    pos = props_start + brace_end + 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        
        rules
    }
    
    /// Parse full HTML and populate cached_doc
    pub fn parse_html(&mut self, html: &str) {
        let html = html.to_string();

        // Extract title (case-insensitive)
        let mut title = String::new();
        let lhtml = crate::fontcase::ascii_lower(&html);
        if let Some(start) = lhtml.find("<title>") {
            if let Some(end) = lhtml[start..].find("</title>") {
                let t = &html[start + 7..start + end];
                title = decode_html_entities(t).trim().to_string();
            }
        }

        // Collect style blocks
        let mut styles: Vec<CssRule> = Vec::new();
        let mut pos = 0;
        while let Some(s) = lhtml[pos..].find("<style") {
            let open = pos + s;
            if let Some(start_tag_end) = html[open..].find('>') {
                let content_start = open + start_tag_end + 1;
                if let Some(close_rel) = lhtml[content_start..].find("</style>") {
                    let content_end = content_start + close_rel;
                    let css = &html[content_start..content_end];
                    let mut parsed = self.parse_css(css);
                    styles.append(&mut parsed);
                    pos = content_end + 8; // len("</style>")
                    continue;
                }
            }
            break;
        }

        // Try to find body content, else parse whole HTML
        let body_html = if let Some(bstart) = lhtml.find("<body") {
            if let Some(bopen_end) = html[bstart..].find('>') {
                let content_start = bstart + bopen_end + 1;
                if let Some(bclose) = lhtml[content_start..].find("</body>") {
                    let content_end = content_start + bclose;
                    html[content_start..content_end].to_string()
                } else {
                    html.clone()
                }
            } else {
                html.clone()
            }
        } else {
            html.clone()
        };

        let nodes = self.parse_nodes(&body_html);

        // Execute inline <script> blocks found in the parsed nodes so that
        // the JS interpreter is kept in sync with the rendered DOM.
        for script in self.collect_inline_scripts(&nodes) {
            if let Err(e) = self.js.execute(&script) {
                tracing::warn!("script execution error: {}", e);
            }
        }

        let doc = HtmlDocument { title, nodes, styles };
        self.cached_doc = Some(doc);
    }

    /// Recursively collect inline script contents from parsed nodes.
    fn collect_inline_scripts(&self, nodes: &[HtmlNode]) -> Vec<String> {
        let mut out = Vec::new();
        for node in nodes {
            if let HtmlNode::Element { tag, children, .. } = node {
                if tag.eq_ignore_ascii_case("script") {
                    let mut s = String::new();
                    for c in children {
                        if let HtmlNode::Text(t) = c {
                            s.push_str(t);
                        }
                    }
                    out.push(s);
                } else {
                    out.extend(self.collect_inline_scripts(children));
                }
            }
        }
        out
    }

    /// Very small fallback HTML -> nodes parser (returns a single text node for now)
    fn parse_nodes(&self, html: &str) -> Vec<HtmlNode> {
        let mut nodes = Vec::new();
        let mut pos = 0;

        while pos < html.len() {
            // Find next tag
            if let Some(tag_start_rel) = html[pos..].find('<') {
                let tag_start = pos + tag_start_rel;

                // Text before tag
                let text = html[pos..tag_start].trim();
                if !text.is_empty() {
                    nodes.push(HtmlNode::Text(decode_html_entities(text)));
                }

                // Parse tag
                if let Some(tag_end_rel) = html[tag_start..].find('>') {
                    let tag_end = tag_start + tag_end_rel;
                    let tag_content = &html[tag_start + 1..tag_end];

                    // Skip comments and doctype
                    if tag_content.starts_with('!') || tag_content.starts_with('?') {
                        pos = tag_end + 1;
                        continue;
                    }

                    // Closing tag - skip
                    if tag_content.starts_with('/') {
                        pos = tag_end + 1;
                        continue;
                    }

                    let is_self_closing = tag_content.ends_with('/') ||
                        tag_content.split_whitespace().next()
                            .map(|t| matches!(crate::fontcase::ascii_lower(t).as_str(), 
                                "br" | "hr" | "img" | "input" | "meta" | "link" | "area" | "base" | "col" | "embed" | "source" | "track" | "wbr"))
                            .unwrap_or(false);

                    let (tag_name, attrs) = self.parse_tag(tag_content);

                    if is_self_closing {
                        nodes.push(self.create_element(&tag_name, attrs, Vec::new()));
                        pos = tag_end + 1;
                    } else {
                        // Find closing tag
                        let close_tag = format!("</{}", tag_name);
                        if let Some(close_rel) = crate::fontcase::ascii_lower(&html[tag_end + 1..]).find(&crate::fontcase::ascii_lower(&close_tag)) {
                            let close_start = tag_end + 1 + close_rel;
                            let inner_html = &html[tag_end + 1..close_start];

                            let children = if tag_name.eq_ignore_ascii_case("script") || tag_name.eq_ignore_ascii_case("style") {
                                vec![HtmlNode::Text(inner_html.to_string())]
                            } else {
                                self.parse_nodes(inner_html)
                            };

                            nodes.push(self.create_element(&tag_name, attrs, children));

                            if let Some(end_rel2) = html[close_start..].find('>') {
                                pos = close_start + end_rel2 + 1;
                            } else {
                                pos = close_start;
                            }
                        } else {
                            // No closing tag found
                            nodes.push(self.create_element(&tag_name, attrs, Vec::new()));
                            pos = tag_end + 1;
                        }
                    }
                } else {
                    break;
                }
            } else {
                // Remaining text
                let text = html[pos..].trim();
                if !text.is_empty() {
                    nodes.push(HtmlNode::Text(decode_html_entities(text)));
                }
                break;
            }
        }

        nodes
    }
    
    
    /// Parse tag name and attributes
    fn parse_tag(&self, content: &str) -> (String, HashMap<String, String>) {
        let s = content.trim_end_matches('/').trim();
        let mut i = 0;
        let bytes = s.as_bytes();
        let len = bytes.len();

        // Helper to skip ASCII whitespace
        let skip_ws = |i: &mut usize| {
            while *i < len && (bytes[*i] == b' ' || bytes[*i] == b'\n' || bytes[*i] == b'\t' || bytes[*i] == b'\r') {
                *i += 1;
            }
        };

        skip_ws(&mut i);
        // read tag name
        let name_start = i;
        while i < len && !bytes[i].is_ascii_whitespace() && bytes[i] != b'/' && bytes[i] != b'>' {
            i += 1;
        }
        let tag_name = if name_start < i { crate::fontcase::ascii_lower(&s[name_start..i]) } else { "div".into() };

        let mut attrs = HashMap::new();

        loop {
            skip_ws(&mut i);
            if i >= len { break; }
            if bytes[i] == b'/' || bytes[i] == b'>' { break; }

            // attr name
            let an_start = i;
            while i < len && bytes[i] != b'=' && !bytes[i].is_ascii_whitespace() && bytes[i] != b'/' && bytes[i] != b'>' {
                i += 1;
            }
            let an_end = i;
            let mut attr_name = crate::fontcase::ascii_lower(s[an_start..an_end].trim());

            skip_ws(&mut i);
            if i < len && bytes[i] == b'=' {
                i += 1; // skip '='
                skip_ws(&mut i);

                // parse value
                if i < len && (bytes[i] == b'"' || bytes[i] == b'\'') {
                    let quote = bytes[i];
                    i += 1;
                    let val_start = i;
                    while i < len && bytes[i] != quote {
                        i += 1;
                    }
                    let val_end = i;
                    if i < len { i += 1; }
                    let raw = &s[val_start..val_end];
                    let decoded = decode_html_entities(raw).trim().to_string();
                    if attr_name.is_empty() { attr_name = "".into(); }
                    attrs.insert(attr_name, decoded);
                } else {
                    // unquoted value
                    let val_start = i;
                    while i < len && !bytes[i].is_ascii_whitespace() && bytes[i] != b'/' && bytes[i] != b'>' {
                        i += 1;
                    }
                    let val_end = i;
                    let raw = &s[val_start..val_end];
                    let decoded = decode_html_entities(raw).trim().to_string();
                    attrs.insert(attr_name, decoded);
                }
            } else {
                // boolean attribute
                if !attr_name.is_empty() {
                    attrs.insert(attr_name, "true".into());
                }
            }
        }

        (tag_name, attrs)
    }
    
    /// Create an element node
    fn create_element(&self, tag: &str, attrs: HashMap<String, String>, children: Vec<HtmlNode>) -> HtmlNode {
        let id = attrs.get("id").cloned();
        let class = attrs.get("class")
            .map(|c| c.split_whitespace().map(String::from).collect())
            .unwrap_or_default();
        let style = attrs.get("style")
            .map(|s| self.parse_inline_style(s))
            .unwrap_or_default();
        
        HtmlNode::Element {
            tag: crate::fontcase::ascii_lower(tag),
            id,
            class,
            style,
            attrs,
            children,
        }
    }
    
    /// Parse inline style attribute
    fn parse_inline_style(&self, style: &str) -> HashMap<String, String> {
        let mut props = HashMap::new();
        for prop in style.split(';') {
            let prop = prop.trim();
            if let Some(colon) = prop.find(':') {
                let key = prop[..colon].trim().to_string();
                let value = prop[colon + 1..].trim().to_string();
                props.insert(key, value);
            }
        }
        props
    }
    
    /// Render HTML document to egui
    pub fn render(&mut self, ui: &mut Ui, url: &str) {
        // Register for image-update notifications on first render so
        // background imaging workers can trigger UI repaints.
        if !self.image_update_registered {
            let (tx, rx) = mpsc::channel::<String>();
            // Register sender with imaging module
            crate::imaging::register_image_update_sender(tx);
            let ctx = ui.ctx().clone();
            // Spawn a listener thread that requests repaint when images update
            // and enqueues the url into the imaging update queue for selective reload.
            thread::spawn(move || {
                while let Ok(url) = rx.recv() {
                    // push into imaging queue so render() can selectively reload textures
                    crate::imaging::push_image_update_to_queue(&url);
                    ctx.request_repaint();
                }
            });
            self.image_update_registered = true;
        }
        // Try to fetch and parse if we don't have cached content
        if self.cached_doc.is_none() {
            self.load_url(url);
        }
        
        // Drain any image updates and reload textures for those specific URLs
        let updates = crate::imaging::drain_image_update_queue();
        for u in updates {
            if let Some(crate::imaging::ImageState::Loaded(img)) = crate::imaging::cache_get_global(&u) {
                let size = [img.width as usize, img.height as usize];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &img.pixels);
                let key = format!("sassy_image:{}", u);
                // Replace or insert texture so subsequent draw uses fresh data
                let _ = ui.ctx().load_texture(&key, color_image, egui::TextureOptions::LINEAR);
            }
        }

        if let Some(doc) = &self.cached_doc.clone() {
            // Build layout tree using taffy for CSS flexbox/grid computation
            let available = ui.available_size();
            let _layout_tree = self.build_layout_tree(available.x, available.y);

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    self.render_nodes(ui, &doc.nodes, &doc.styles);
                });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new("Loading...").size(20.0));
            });
        }
    }
    
    /// Load URL content
    fn load_url(&mut self, url: &str) {
        // For local files
        if url.starts_with("file://") {
            let path = url.trim_start_matches("file://");
            if let Ok(content) = std::fs::read_to_string(path) {
                self.parse_html(&content);
                return;
            }
        }
        
        // For sassy:// internal pages
        if url.starts_with("sassy://") {
            let page = url.trim_start_matches("sassy://");
            let html = self.generate_internal_page(page);
            self.parse_html(&html);
            return;
        }
        
        // Fetch remote page using pure Rust HTTP client (ureq)
        match crate::http_client::fetch_text(url) {
            Ok(body) => {
                self.parse_html(&body);
                return;
            }
            Err(e) => {
                tracing::warn!("Failed to fetch {}: {}", url, e);
                // Show error page with details
                let html = format!(r#"
                    <!DOCTYPE html>
                    <html>
                    <head><title>Page Load Error</title></head>
                    <body style="font-family: sans-serif; padding: 40px; text-align: center;">
                        <h1 style="color: #e74c3c;">! Failed to Load Page</h1>
                        <p><strong>URL:</strong> {}</p>
                        <p><strong>Error:</strong> {}</p>
                        <hr style="margin: 30px 0;">
                        <p style="color: #666;">The Sassy Browser pure Rust renderer attempted to fetch this page but encountered an error.</p>
                        <p style="color: #666;">Common causes: network issues, invalid URL, site blocking non-browser requests</p>
                    </body>
                    </html>
                "#, url, e);
                self.parse_html(&html);
            }
        }
    }
    
    /// Generate internal pages
    fn generate_internal_page(&self, page: &str) -> String {
        match page {
            "newtab" => r#"
                <!DOCTYPE html>
                <html>
                <head><title>New Tab</title></head>
                <body style="font-family: sans-serif; text-align: center; padding: 60px;">
                    <h1>Sassy Browser</h1>
                    <p>Fast - Free - Handles Everything</p>
                </body>
                </html>
            "#.into(),
            
            "settings" => r#"
                <!DOCTYPE html>
                <html>
                <head><title>Settings</title></head>
                <body style="font-family: sans-serif; padding: 20px;">
                    <h1>(settings) Settings</h1>
                    <p>Settings page content here</p>
                </body>
                </html>
            "#.into(),
            
            _ => format!(r#"
                <!DOCTYPE html>
                <html>
                <head><title>Sassy Browser</title></head>
                <body style="font-family: sans-serif; padding: 20px;">
                    <h1>Page: {}</h1>
                </body>
                </html>
            "#, page),
        }
    }
    
    /// Render nodes recursively
    fn render_nodes(&mut self, ui: &mut Ui, nodes: &[HtmlNode], styles: &[CssRule]) {
        for node in nodes {
            self.render_node(ui, node, styles);
        }
    }
    
    /// Render a single node
    fn render_node(&mut self, ui: &mut Ui, node: &HtmlNode, styles: &[CssRule]) {
        match node {
            HtmlNode::Text(text) => {
                if !text.trim().is_empty() {
                    ui.label(text);
                }
            }
            
            HtmlNode::Script(_) => {
                // Scripts already executed during parse
            }
            
            HtmlNode::Element { tag, id, class, style, attrs, children } => {
                // Get computed styles
                let computed = self.compute_styles(tag, id.as_deref(), class, style, styles);
                
                match tag.as_str() {
                    // Block elements
                    "h1" => {
                        let size = computed.get("font-size").and_then(|s| parse_size(s)).unwrap_or(32.0);
                        ui.heading(RichText::new(self.text_content(children)).size(size).strong());
                    }
                    "h2" => {
                        let size = computed.get("font-size").and_then(|s| parse_size(s)).unwrap_or(28.0);
                        ui.heading(RichText::new(self.text_content(children)).size(size).strong());
                    }
                    "h3" => {
                        let size = computed.get("font-size").and_then(|s| parse_size(s)).unwrap_or(24.0);
                        ui.heading(RichText::new(self.text_content(children)).size(size).strong());
                    }
                    "h4" | "h5" | "h6" => {
                        let size = computed.get("font-size").and_then(|s| parse_size(s)).unwrap_or(20.0);
                        ui.heading(RichText::new(self.text_content(children)).size(size).strong());
                    }
                    
                    "p" => {
                        let text = self.text_content(children);
                        if !text.is_empty() {
                            ui.label(text);
                        } else {
                            self.render_nodes(ui, children, styles);
                        }
                        ui.add_space(8.0);
                    }
                    
                    "div" | "section" | "article" | "main" | "header" | "footer" | "nav" | "aside" => {
                        self.render_nodes(ui, children, styles);
                    }
                    
                    "span" => {
                        ui.horizontal(|ui| {
                            self.render_nodes(ui, children, styles);
                        });
                    }
                    
                    "a" => {
                        let href = attrs.get("href").cloned().unwrap_or_default();
                        let text = self.text_content(children);
                        let display = if text.is_empty() { &href } else { &text };
                        if ui.link(display).on_hover_text(&href).clicked() {
                            self.hover_link = Some(href);
                        }
                    }
                    
                    "br" => {
                        ui.add_space(self.font_size_base);
                    }
                    
                    "hr" => {
                        ui.separator();
                    }
                    
                    "strong" | "b" => {
                        ui.label(RichText::new(self.text_content(children)).strong());
                    }
                    
                    "em" | "i" => {
                        ui.label(RichText::new(self.text_content(children)).italics());
                    }
                    
                    "code" => {
                        ui.label(RichText::new(self.text_content(children)).monospace().background_color(Color32::from_gray(40)));
                    }
                    
                    "pre" => {
                        egui::Frame::none()
                            .fill(Color32::from_gray(30))
                            .inner_margin(8.0)
                            .rounding(4.0)
                            .show(ui, |ui| {
                                ui.label(RichText::new(self.text_content(children)).monospace());
                            });
                    }
                    
                    "ul" => {
                        for child in children {
                            if let HtmlNode::Element { tag, children: li_children, .. } = child {
                                if tag == "li" {
                                    ui.horizontal(|ui| {
                                        ui.label("-");
                                        self.render_nodes(ui, li_children, styles);
                                    });
                                }
                            }
                        }
                    }
                    
                    "ol" => {
                        for (i, child) in children.iter().enumerate() {
                            if let HtmlNode::Element { tag, children: li_children, .. } = child {
                                if tag == "li" {
                                    ui.horizontal(|ui| {
                                        ui.label(format!("{}.", i + 1));
                                        self.render_nodes(ui, li_children, styles);
                                    });
                                }
                            }
                        }
                    }
                    
                    "img" => {
                        let src = attrs.get("src").cloned().unwrap_or_default();
                        let alt = attrs.get("alt").cloned().unwrap_or_else(|| "Image".into());

                        // If it's a data: URL, try to decode immediately
                        let maybe_img = if src.starts_with("data:") {
                            crate::imaging::load_data_url(&src).ok()
                        } else {
                            // Trigger background load and then check cache for a loaded image
                            crate::imaging::load_image_background(&src);
                            match crate::imaging::cache_get_global(&src) {
                                Some(crate::imaging::ImageState::Loaded(img)) => Some(img),
                                _ => None,
                            }
                        };

                        if let Some(img) = maybe_img {
                            // Convert to egui ColorImage and display
                            let size = [img.width as usize, img.height as usize];
                            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &img.pixels);
                            let key = format!("sassy_image:{}", src);
                            let texture = ui.ctx().load_texture(&key, color_image, egui::TextureOptions::LINEAR);
                            ui.image((texture.id(), Vec2::new(img.width as f32, img.height as f32)));
                        } else {
                            // Show placeholder text or small placeholder image
                            let ph = crate::imaging::placeholder_image(64, 48);
                            let size = [ph.width as usize, ph.height as usize];
                            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &ph.pixels);
                            let key = format!("sassy_image_placeholder:{}", src);
                            let texture = ui.ctx().load_texture(&key, color_image, egui::TextureOptions::LINEAR);
                            ui.image((texture.id(), Vec2::new(ph.width as f32, ph.height as f32)));
                            ui.label(RichText::new(alt.to_string()).italics().color(Color32::GRAY));
                        }
                    }
                    
                    "table" => {
                        egui::Grid::new("html_table").striped(true).show(ui, |ui| {
                            self.render_table_contents(ui, children, styles);
                        });
                    }
                    
                    "button" => {
                        let text = self.text_content(children);
                        if ui.button(&text).clicked() {
                            // Handle click event
                            if let Some(onclick) = attrs.get("onclick") {
                                let _ = self.js.execute(onclick);
                            }
                        }
                    }
                    
                    "input" => {
                        let input_type = attrs.get("type").map(|s| s.as_str()).unwrap_or("text");
                        let placeholder = attrs.get("placeholder").cloned().unwrap_or_default();
                        
                        match input_type {
                            "text" | "email" | "password" | "search" => {
                                let mut value = attrs.get("value").cloned().unwrap_or_default();
                                ui.add(egui::TextEdit::singleline(&mut value).hint_text(placeholder));
                            }
                            "checkbox" => {
                                let mut checked = attrs.get("checked").is_some();
                                ui.checkbox(&mut checked, "");
                            }
                            "submit" | "button" => {
                                let value = attrs.get("value").cloned().unwrap_or_else(|| "Submit".into());
                                let _ = ui.button(&value);
                            }
                            _ => {}
                        }
                    }
                    
                    "textarea" => {
                        let mut value = self.text_content(children);
                        ui.text_edit_multiline(&mut value);
                    }
                    
                    "form" => {
                        egui::Frame::none()
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                self.render_nodes(ui, children, styles);
                            });
                    }
                    
                    "canvas" => {
                        let width = attrs.get("width").and_then(|s| s.parse().ok()).unwrap_or(300.0);
                        let height = attrs.get("height").and_then(|s| s.parse().ok()).unwrap_or(150.0);
                        
                        let (rect, _response) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 0.0, Color32::from_gray(50));
                    }
                    
                    "script" | "style" | "head" | "meta" | "link" | "title" => {
                        // Ignore these in rendering
                    }
                    
                    _ => {
                        // Generic element - just render children
                        self.render_nodes(ui, children, styles);
                    }
                }
            }
        }
    }
    
    /// Render table contents
    #[allow(clippy::only_used_in_recursion)]
    fn render_table_contents(&mut self, ui: &mut Ui, nodes: &[HtmlNode], styles: &[CssRule]) {
        for node in nodes {
            if let HtmlNode::Element { tag, children, .. } = node {
                match tag.as_str() {
                    "thead" | "tbody" | "tfoot" => {
                        self.render_table_contents(ui, children, styles);
                    }
                    "tr" => {
                        for cell in children {
                            if let HtmlNode::Element { tag, children: cell_children, .. } = cell {
                                if tag == "th" || tag == "td" {
                                    let text = self.text_content(cell_children);
                                    if tag == "th" {
                                        ui.strong(text);
                                    } else {
                                        ui.label(text);
                                    }
                                }
                            }
                        }
                        ui.end_row();
                    }
                    _ => {}
                }
            }
        }
    }
    
    /// Get text content from nodes
    #[allow(clippy::only_used_in_recursion)]
    fn text_content(&self, nodes: &[HtmlNode]) -> String {
        let mut text = String::new();
        for node in nodes {
            match node {
                HtmlNode::Text(t) => text.push_str(t),
                HtmlNode::Element { children, .. } => {
                    text.push_str(&self.text_content(children));
                }
                _ => {}
            }
        }
        text
    }
    
    /// Compute styles for an element
    fn compute_styles(
        &self,
        tag: &str,
        id: Option<&str>,
        classes: &[String],
        inline: &HashMap<String, String>,
        rules: &[CssRule],
    ) -> HashMap<String, String> {
        let mut result = HashMap::new();
        
        // Apply rules in order of specificity
        for rule in rules {
            let matches = if rule.selector.starts_with('#') {
                id == Some(&rule.selector[1..])
            } else if rule.selector.starts_with('.') {
                classes.contains(&rule.selector[1..].to_string())
            } else {
                rule.selector == tag || rule.selector == "*"
            };
            
            if matches {
                result.extend(rule.properties.clone());
            }
        }
        
        // Inline styles override
        result.extend(inline.clone());
        
        result
    }
    
    /// Get JS console output
    pub fn console_output(&self) -> &[String] {
        self.js.get_console_output()
    }
    
    /// Get clicked link
    pub fn take_clicked_link(&mut self) -> Option<String> {
        self.hover_link.take()
    }
    
    /// Clear cached document
    pub fn clear_cache(&mut self) {
        self.cached_doc = None;
    }

    /// Search for text in cached document, return match count
    pub fn find_text(&self, query: &str) -> usize {
        if query.is_empty() {
            return 0;
        }
        let Some(doc) = &self.cached_doc else { return 0 };
        let query_lower = query.to_lowercase();
        let mut count = 0;
        fn count_in_nodes(nodes: &[HtmlNode], query: &str, count: &mut usize) {
            for node in nodes {
                match node {
                    HtmlNode::Text(text) => {
                        let text_lower = text.to_lowercase();
                        *count += text_lower.matches(query).count();
                    }
                    HtmlNode::Element { children, .. } => {
                        count_in_nodes(children, query, count);
                    }
                    HtmlNode::Script(_) => {}
                }
            }
        }
        count_in_nodes(&doc.nodes, &query_lower, &mut count);
        count
    }
}

impl Default for HtmlRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Decode HTML entities
fn decode_html_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
        .replace("&#39;", "'")
        .replace("&#34;", "\"")
}

/// Parse CSS size value
fn parse_size(s: &str) -> Option<f32> {
    let s = s.trim();
    if let Some(stripped) = s.strip_suffix("px") {
        stripped.parse().ok()
    } else if let Some(stripped) = s.strip_suffix("em") {
        stripped.parse::<f32>().ok().map(|v| v * 16.0)
    } else if let Some(stripped) = s.strip_suffix("rem") {
        stripped.parse::<f32>().ok().map(|v| v * 16.0)
    } else if let Some(stripped) = s.strip_suffix('%') {
        stripped.parse::<f32>().ok().map(|v| v / 100.0 * 16.0)
    } else {
        s.parse().ok()
    }
}

/// Parse CSS color value
fn parse_color(s: &str) -> Option<Color32> {
    let s = crate::fontcase::ascii_lower(s.trim());
    
    // Named colors
    match s.as_str() {
        "black" => return Some(Color32::BLACK),
        "white" => return Some(Color32::WHITE),
        "red" => return Some(Color32::RED),
        "green" => return Some(Color32::GREEN),
        "blue" => return Some(Color32::BLUE),
        "yellow" => return Some(Color32::YELLOW),
        "gray" | "grey" => return Some(Color32::GRAY),
        "transparent" => return Some(Color32::TRANSPARENT),
        _ => {}
    }
    
    // Hex colors
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            return Some(Color32::from_rgb(r, g, b));
        } else if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(Color32::from_rgb(r, g, b));
        }
    }
    
    // RGB/RGBA
    if s.starts_with("rgb") {
        let inner = s.trim_start_matches("rgba(")
            .trim_start_matches("rgb(")
            .trim_end_matches(')');
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() >= 3 {
            let r = parts[0].trim().parse::<u8>().ok()?;
            let g = parts[1].trim().parse::<u8>().ok()?;
            let b = parts[2].trim().parse::<u8>().ok()?;
            let a = parts.get(3)
                .and_then(|s| s.trim().parse::<f32>().ok())
                .map(|v| (v * 255.0) as u8)
                .unwrap_or(255);
            return Some(Color32::from_rgba_unmultiplied(r, g, b, a));
        }
    }
    
    None
}
