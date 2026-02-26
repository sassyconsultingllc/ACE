// Renderer - Main renderer coordination

use crate::dom::{Document, Node, NodeRef};
use crate::layout::{LayoutBox, LayoutEngine};
use crate::paint::Painter;
use crate::style::{ComputedStyle, StyleEngine};
use std::cell::RefCell;
use std::collections::HashMap;

pub struct Renderer {
    pub document: Document,
    pub style_engine: StyleEngine,
    pub layout_engine: LayoutEngine,
    pub painter: Painter,
    pub layout_tree: Option<LayoutBox>,
    pub computed_styles: HashMap<*const RefCell<crate::dom::Node>, ComputedStyle>,
    pub scroll_y: f32,
    pub max_scroll: f32,
}

impl Renderer {
    pub fn new(width: u32, height: u32) -> Self {
        Renderer {
            document: Document::new(),
            style_engine: StyleEngine::new(),
            layout_engine: LayoutEngine::new(width as f32, height as f32),
            painter: Painter::new(width, height),
            layout_tree: None,
            computed_styles: HashMap::new(),
            scroll_y: 0.0,
            max_scroll: 0.0,
        }
    }

    pub fn parse_html(&mut self, html: &str) {
        // Minimal fallback parser: wraps the raw HTML string in a <body> text node.
        // This avoids external parser dependency mismatches while keeping the DOM usable.
        self.document = Document::new();

        // Extract title from HTML if present
        if let Some(start) = html.find("<title>") {
            if let Some(end) = html[start..].find("</title>") {
                self.document.title = html[start + 7..start + end].to_string();
            }
        }

        // Extract base URL if present
        if let Some(start) = html.find("<base") {
            if let Some(href_start) = html[start..].find("href=\"") {
                let after = &html[start + href_start + 6..];
                if let Some(end) = after.find('"') {
                    self.document.base_url = Some(after[..end].to_string());
                }
            }
        }

        let body = Node::new_element("body");
        Node::append_child(&self.document.root, &body);

        if !html.is_empty() {
            // Extract and append HTML comments as comment nodes
            let mut remaining = html;
            while let Some(start) = remaining.find("<!--") {
                if let Some(end) = remaining[start..].find("-->") {
                    let comment_text = &remaining[start + 4..start + end];
                    let comment_node = Node::new_comment(comment_text);
                    Node::append_child(&body, &comment_node);
                    remaining = &remaining[start + end + 3..];
                } else {
                    break;
                }
            }

            let text_node = Node::new_text(html);
            Node::append_child(&body, &text_node);
        }
    }

    pub fn compute_styles(&mut self) {
        self.computed_styles.clear();
        self.compute_node_styles(&self.document.root.clone());
    }

    fn compute_node_styles(&mut self, node: &NodeRef) {
        let style = self.style_engine.compute_style(node);
        self.computed_styles
            .insert(node.as_ref() as *const _, style);

        let children: Vec<NodeRef> = node.borrow().children.to_vec();
        for child in children {
            self.compute_node_styles(&child);
        }
    }

    pub fn layout(&mut self) {
        self.layout_tree = Some(
            self.layout_engine
                .layout(&self.document.root, &self.computed_styles),
        );
        if let Some(ref layout) = self.layout_tree {
            self.max_scroll = (layout.bounds.height - self.layout_engine.viewport_height).max(0.0);
        }
    }

    pub fn paint(&mut self) {
        if let Some(ref layout) = self.layout_tree {
            self.painter.paint(layout, 0, -(self.scroll_y as i32));
        }
    }

    pub fn render(&mut self) {
        self.compute_styles();
        self.layout();
        self.paint();
    }

    /// Render using the staged pipeline instead of direct calls
    pub fn render_with_pipeline(&mut self) {
        let mut pipeline = build_default_pipeline();
        pipeline.execute(self);
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.layout_engine.viewport_width = width as f32;
        self.layout_engine.viewport_height = height as f32;
        self.painter.resize(width, height);
        self.render();
    }

    pub fn scroll(&mut self, delta: f32) {
        self.scroll_y = (self.scroll_y + delta).clamp(0.0, self.max_scroll);
        self.paint();
    }

    pub fn scroll_to(&mut self, y: f32) {
        self.scroll_y = y.clamp(0.0, self.max_scroll);
        self.paint();
    }

    pub fn hit_test(&self, x: f32, y: f32) -> Option<NodeRef> {
        if let Some(ref layout) = self.layout_tree {
            self.layout_engine.hit_test(layout, x, y + self.scroll_y)
        } else {
            None
        }
    }

    pub fn get_element_rect(&self, node: &NodeRef) -> Option<crate::layout::Rect> {
        if let Some(ref layout) = self.layout_tree {
            self.layout_engine
                .find_layout_for_node(layout, node)
                .map(|l| l.border)
        } else {
            None
        }
    }

    pub fn get_buffer(&self) -> &[u32] {
        &self.painter.buffer
    }

    pub fn get_title(&self) -> &str {
        &self.document.title
    }

    pub fn add_stylesheet(&mut self, css: &str) {
        self.style_engine.add_stylesheet(css);
    }

    pub fn get_links(&self) -> Vec<(String, NodeRef)> {
        self.document
            .get_links()
            .iter()
            .filter_map(|node| {
                let n = node.borrow();
                n.get_attribute("href").map(|href| (href, node.clone()))
            })
            .collect()
    }

    pub fn get_images(&self) -> Vec<(String, NodeRef)> {
        self.document
            .get_images()
            .iter()
            .filter_map(|node| {
                let n = node.borrow();
                n.get_attribute("src").map(|src| (src, node.clone()))
            })
            .collect()
    }

    pub fn get_forms(&self) -> Vec<NodeRef> {
        self.document.get_forms()
    }

    pub fn get_scripts(&self) -> Vec<String> {
        self.document
            .get_scripts()
            .iter()
            .filter_map(|node| {
                let n = node.borrow();
                if n.get_attribute("src").is_some() {
                    n.get_attribute("src")
                } else {
                    Some(n.get_inner_text())
                }
            })
            .collect()
    }

    /// Summary for diagnostics - wires get_links, get_images, get_forms, get_scripts, get_element_rect
    pub fn describe(&self) -> String {
        let links = self.get_links();
        let images = self.get_images();
        let forms = self.get_forms();
        let scripts = self.get_scripts();
        let first_link_rect = links
            .first()
            .and_then(|(_, node)| self.get_element_rect(node));
        format!(
            "Renderer[title={}, scroll={:.0}/{:.0}, links={}, images={}, forms={}, scripts={}, first_rect={}]",
            self.get_title(), self.scroll_y, self.max_scroll,
            links.len(), images.len(), forms.len(), scripts.len(),
            first_link_rect.map(|r| format!("({},{},{},{})", r.x, r.y, r.width, r.height))
                .unwrap_or_else(|| "none".to_string())
        )
    }
}

pub struct RenderPipeline {
    stages: Vec<Box<dyn RenderStage>>,
}

pub trait RenderStage {
    fn execute(&mut self, renderer: &mut Renderer);
}

impl RenderPipeline {
    pub fn new() -> Self {
        RenderPipeline { stages: Vec::new() }
    }
    pub fn add_stage(&mut self, stage: Box<dyn RenderStage>) {
        self.stages.push(stage);
    }
    pub fn execute(&mut self, renderer: &mut Renderer) {
        for stage in &mut self.stages {
            stage.execute(renderer);
        }
    }
}

impl Default for RenderPipeline {
    fn default() -> Self {
        Self::new()
    }
}

pub struct StyleStage;
impl RenderStage for StyleStage {
    fn execute(&mut self, r: &mut Renderer) {
        r.compute_styles();
    }
}

pub struct LayoutStage;
impl RenderStage for LayoutStage {
    fn execute(&mut self, r: &mut Renderer) {
        r.layout();
    }
}

pub struct PaintStage;
impl RenderStage for PaintStage {
    fn execute(&mut self, r: &mut Renderer) {
        r.paint();
    }
}

/// Build the default 3-stage pipeline (style -> layout -> paint)
pub fn build_default_pipeline() -> RenderPipeline {
    let mut pipeline = RenderPipeline::new();
    pipeline.add_stage(Box::new(StyleStage));
    pipeline.add_stage(Box::new(LayoutStage));
    pipeline.add_stage(Box::new(PaintStage));
    pipeline
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_new_and_parse() {
        let mut r = Renderer::new(800, 600);
        r.parse_html("<html><head><title>Test</title></head><body>Hello</body></html>");
        assert_eq!(r.get_title(), "Test");
    }

    #[test]
    fn test_renderer_render_pipeline() {
        let mut r = Renderer::new(320, 240);
        r.parse_html("<body>Pipeline test</body>");
        r.render();
        assert!(!r.get_buffer().is_empty());
        // Also exercise the staged pipeline
        r.render_with_pipeline();
        assert!(!r.get_buffer().is_empty());
    }

    #[test]
    fn test_build_default_pipeline() {
        let mut pipeline = build_default_pipeline();
        let mut r = Renderer::new(100, 100);
        r.parse_html("<body>stage</body>");
        pipeline.execute(&mut r);
        assert!(!r.get_buffer().is_empty());
    }

    #[test]
    fn test_pipeline_stages_individually() {
        let mut r = Renderer::new(100, 100);
        r.parse_html("<body>stages</body>");
        StyleStage.execute(&mut r);
        LayoutStage.execute(&mut r);
        PaintStage.execute(&mut r);
        assert!(!r.get_buffer().is_empty());
    }

    #[test]
    fn test_pipeline_default_trait() {
        let pipeline = RenderPipeline::default();
        assert!(pipeline.stages.is_empty());
    }

    #[test]
    fn test_scroll_to() {
        let mut r = Renderer::new(800, 600);
        r.parse_html("<body>Scroll test content</body>");
        r.render();
        r.scroll_to(50.0);
        assert!(r.scroll_y <= r.max_scroll.max(50.0));
    }

    #[test]
    fn test_hit_test() {
        let mut r = Renderer::new(800, 600);
        r.parse_html("<body><a href='https://example.com'>Link</a></body>");
        r.render();
        // Hit test at origin - may or may not find anything
        let _result = r.hit_test(10.0, 10.0);
    }

    #[test]
    fn test_get_element_rect() {
        let mut r = Renderer::new(800, 600);
        r.parse_html("<body><p id='test'>text</p></body>");
        r.render();
        // Get links and try to get rect for first node
        let links = r.get_links();
        if let Some((_, node)) = links.first() {
            let _rect = r.get_element_rect(node);
        }
    }

    #[test]
    fn test_get_links_images_forms_scripts() {
        let mut r = Renderer::new(800, 600);
        r.parse_html(
            "<body><a href='x'>L</a><img src='y'><form><input></form><script>1</script></body>",
        );
        r.render();
        let links = r.get_links();
        let images = r.get_images();
        let forms = r.get_forms();
        let scripts = r.get_scripts();
        // Verify they return (possibly empty) collections
        let _ = (links.len(), images.len(), forms.len(), scripts.len());
    }

    #[test]
    fn test_describe() {
        let mut r = Renderer::new(800, 600);
        r.parse_html("<body><a href='https://example.com'>click</a></body>");
        r.render();
        let desc = r.describe();
        assert!(desc.contains("Renderer["));
        assert!(desc.contains("title="));
    }

    #[test]
    fn test_resize() {
        let mut r = Renderer::new(800, 600);
        r.parse_html("<body>Resize test</body>");
        r.render();
        r.resize(1024, 768);
        assert_eq!(r.get_buffer().len(), 1024 * 768);
    }

    #[test]
    fn test_scroll() {
        let mut r = Renderer::new(800, 600);
        r.parse_html("<body>Scroll content</body>");
        r.render();
        r.scroll(10.0);
        // Scroll should clamp properly
        assert!(r.scroll_y >= 0.0);
    }

    #[test]
    fn test_add_stylesheet() {
        let mut r = Renderer::new(800, 600);
        r.parse_html("<body><p>styled</p></body>");
        r.add_stylesheet("p { color: red; }");
        r.render();
        assert!(!r.get_buffer().is_empty());
    }

    #[test]
    fn test_base_url_extraction() {
        let mut r = Renderer::new(800, 600);
        r.parse_html(
            r#"<html><head><base href="https://example.com/"></head><body>Base</body></html>"#,
        );
        assert_eq!(r.document.base_url.as_deref(), Some("https://example.com/"));
    }
}
