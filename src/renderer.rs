// Renderer - Main renderer coordination
#![allow(dead_code)]

use crate::dom::{Document, NodeRef, Node};
use crate::style::{StyleEngine, ComputedStyle};
use crate::layout::{LayoutEngine, LayoutBox};
use crate::paint::Painter;
use std::collections::HashMap;
use std::cell::RefCell;

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

        let body = Node::new_element("body");
        Node::append_child(&self.document.root, &body);

        if !html.is_empty() {
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
        self.computed_styles.insert(node.as_ref() as *const _, style);
        
        let children: Vec<NodeRef> = node.borrow().children.to_vec();
        for child in children {
            self.compute_node_styles(&child);
        }
    }

    pub fn layout(&mut self) {
        self.layout_tree = Some(self.layout_engine.layout(&self.document.root, &self.computed_styles));
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
            self.layout_engine.find_layout_for_node(layout, node).map(|l| l.border)
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
        self.document.get_links().iter().filter_map(|node| {
            let n = node.borrow();
            n.get_attribute("href").map(|href| (href, node.clone()))
        }).collect()
    }

    pub fn get_images(&self) -> Vec<(String, NodeRef)> {
        self.document.get_images().iter().filter_map(|node| {
            let n = node.borrow();
            n.get_attribute("src").map(|src| (src, node.clone()))
        }).collect()
    }

    pub fn get_forms(&self) -> Vec<NodeRef> {
        self.document.get_forms()
    }

    pub fn get_scripts(&self) -> Vec<String> {
        self.document.get_scripts().iter().filter_map(|node| {
            let n = node.borrow();
            if n.get_attribute("src").is_some() {
                n.get_attribute("src")
            } else {
                Some(n.get_inner_text())
            }
        }).collect()
    }
}

pub struct RenderPipeline {
    stages: Vec<Box<dyn RenderStage>>,
}

pub trait RenderStage {
    fn execute(&mut self, renderer: &mut Renderer);
}

impl RenderPipeline {
    pub fn new() -> Self { RenderPipeline { stages: Vec::new() } }
    pub fn add_stage(&mut self, stage: Box<dyn RenderStage>) { self.stages.push(stage); }
    pub fn execute(&mut self, renderer: &mut Renderer) {
        for stage in &mut self.stages { stage.execute(renderer); }
    }
}

impl Default for RenderPipeline {
    fn default() -> Self { Self::new() }
}

pub struct StyleStage;
impl RenderStage for StyleStage { fn execute(&mut self, r: &mut Renderer) { r.compute_styles(); } }

pub struct LayoutStage;
impl RenderStage for LayoutStage { fn execute(&mut self, r: &mut Renderer) { r.layout(); } }

pub struct PaintStage;
impl RenderStage for PaintStage { fn execute(&mut self, r: &mut Renderer) { r.paint(); } }
