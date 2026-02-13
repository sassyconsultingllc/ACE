#![allow(dead_code, unused_imports, unused_variables, deprecated)]
//! PDF Viewer - Pure Rust PDF viewing with visual rendering
//!
//! REPLACES: Adobe Acrobat ($240/yr), Foxit PDF ($140/yr)
//!
//! Features:
//! - Visual rendering: Positioned text, embedded images, graphic primitives
//! - Navigation: Page thumbnails, continuous scroll, zoom
//! - Search: Full text search with highlighting
//! - Annotations: Highlights, notes (stored separately)
//! - Export: Save annotations, export text

use crate::file_handler::{FileContent, OpenFile};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use image::GenericImageView;
use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================================
// POSITIONED TEXT ELEMENT — text with coordinates from content stream
// ============================================================================

#[derive(Debug, Clone)]
struct TextElement {
    /// Text content
    text: String,
    /// X position in PDF points (from page left)
    x: f32,
    /// Y position in PDF points (from page bottom — will be flipped for rendering)
    y: f32,
    /// Font size in PDF points
    font_size: f32,
    /// Font name key (e.g. "F1", "F2")
    font_key: String,
    /// Whether text is bold (heuristic from font name)
    bold: bool,
    /// Whether text is italic (heuristic from font name)
    italic: bool,
}

// ============================================================================
// EMBEDDED IMAGE — extracted from XObject resources
// ============================================================================

#[derive(Debug, Clone)]
struct EmbeddedImage {
    /// Image data as RGBA pixels
    rgba_data: Vec<u8>,
    /// Image width in pixels
    width: u32,
    /// Image height in pixels
    height: u32,
    /// Position on page (PDF points, origin bottom-left)
    x: f32,
    y: f32,
    /// Display size on page
    display_width: f32,
    display_height: f32,
}

// ============================================================================
// GRAPHIC PRIMITIVE — lines, rects, fills from content stream
// ============================================================================

#[derive(Debug, Clone)]
enum GraphicPrimitive {
    /// Stroked rectangle
    StrokeRect { rect: [f32; 4], color: [f32; 3], width: f32 },
    /// Filled rectangle
    FillRect { rect: [f32; 4], color: [f32; 3] },
    /// Line segment
    Line { x1: f32, y1: f32, x2: f32, y2: f32, color: [f32; 3], width: f32 },
}

// ============================================================================
// ENHANCED PAGE CONTENT
// ============================================================================

#[derive(Debug, Clone)]
pub struct PdfPage {
    pub page_num: usize,
    /// Raw extracted text (for search)
    pub text: String,
    /// Real page width in PDF points
    pub width: f32,
    /// Real page height in PDF points
    pub height: f32,
    /// Positioned text elements from content stream
    text_elements: Vec<TextElement>,
    /// Embedded images decoded from XObject
    images: Vec<EmbeddedImage>,
    /// Graphic primitives (lines, rects)
    graphics: Vec<GraphicPrimitive>,
}

// ============================================================================
// ANNOTATIONS
// ============================================================================

#[derive(Debug, Clone)]
pub enum Annotation {
    Highlight { page: usize, start: usize, end: usize, color: Color32 },
    Note { page: usize, position: Pos2, text: String, color: Color32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PdfTool {
    Select,
    Highlight,
    Note,
}

// ============================================================================
// SEARCH
// ============================================================================

#[derive(Clone)]
pub struct SearchResult {
    pub page: usize,
    pub position: usize,
    pub text: String,
}

// ============================================================================
// PDF VIEWER
// ============================================================================

pub struct PdfViewer {
    // Content
    pages: Vec<PdfPage>,
    total_pages: usize,
    pdf_data: Option<Vec<u8>>,
    loaded_path: Option<PathBuf>,

    // Navigation
    current_page: usize,
    zoom: f32,
    scroll_offset: f32,

    // View options
    show_thumbnails: bool,
    continuous_scroll: bool,
    show_images: bool,
    show_graphics: bool,

    // Search
    search_query: String,
    search_results: Vec<SearchResult>,
    current_search_index: usize,

    // Annotations
    annotations: Vec<Annotation>,
    current_tool: PdfTool,
    annotation_color: Color32,

    // Cached textures for embedded images
    image_textures: HashMap<String, egui::TextureHandle>,

    // Status
    loading: bool,
    error_message: Option<String>,
}

impl PdfViewer {
    pub fn new() -> Self {
        Self {
            pages: Vec::new(),
            total_pages: 0,
            pdf_data: None,
            loaded_path: None,

            current_page: 0,
            zoom: 1.0,
            scroll_offset: 0.0,

            show_thumbnails: true,
            continuous_scroll: false,
            show_images: true,
            show_graphics: true,

            search_query: String::new(),
            search_results: Vec::new(),
            current_search_index: 0,

            annotations: Vec::new(),
            current_tool: PdfTool::Select,
            annotation_color: Color32::YELLOW,

            image_textures: HashMap::new(),

            loading: false,
            error_message: None,
        }
    }

    // ========================================================================
    // LOADING — extract positioned content from PDF
    // ========================================================================

    pub fn load_pdf(&mut self, data: &[u8]) {
        self.loading = true;
        self.error_message = None;
        self.pages.clear();
        self.image_textures.clear();
        self.pdf_data = Some(data.to_vec());

        // Parse with lopdf for structure + content streams
        match lopdf::Document::load_mem(data) {
            Ok(doc) => {
                let page_ids: Vec<_> = doc.page_iter().collect();
                self.total_pages = page_ids.len();

                for (idx, page_id) in page_ids.iter().enumerate() {
                    let mut page = PdfPage {
                        page_num: idx,
                        text: String::new(),
                        width: 612.0,
                        height: 792.0,
                        text_elements: Vec::new(),
                        images: Vec::new(),
                        graphics: Vec::new(),
                    };

                    // Extract real page dimensions from MediaBox
                    self.extract_page_dimensions(&doc, *page_id, &mut page);

                    // Extract positioned text from content stream
                    self.extract_positioned_text(&doc, *page_id, &mut page);

                    // Extract embedded images
                    self.extract_page_images(&doc, *page_id, &mut page);

                    // Extract graphic primitives
                    self.extract_graphics(&doc, *page_id, &mut page);

                    self.pages.push(page);
                }

                // Also extract full text via pdf_extract for search
                if let Ok(full_text) = pdf_extract::extract_text_from_mem(data) {
                    self.distribute_search_text(&full_text);
                }

                self.loading = false;
            }
            Err(e) => {
                // Fallback: try text-only extraction
                self.error_message = Some(format!("PDF parse warning: {:?}", e));
                self.fallback_text_extraction(data);
                self.loading = false;
            }
        }
    }

    fn extract_page_dimensions(&self, doc: &lopdf::Document, page_id: lopdf::ObjectId, page: &mut PdfPage) {
        if let Ok(page_dict) = doc.get_dictionary(page_id) {
            // Try CropBox first, then MediaBox
            // use byte-slices so both branches have the same type (&[u8])
            let box_key: &[u8] = if page_dict.has(b"CropBox") { &b"CropBox"[..] } else { &b"MediaBox"[..] };

            if let Ok(obj) = page_dict.get(box_key) {
                if let Ok(arr) = obj.as_array() {
                    if arr.len() >= 4 {
                        let x0 = Self::obj_to_f32(&arr[0]).unwrap_or(0.0);
                        let y0 = Self::obj_to_f32(&arr[1]).unwrap_or(0.0);
                        let x1 = Self::obj_to_f32(&arr[2]).unwrap_or(612.0);
                        let y1 = Self::obj_to_f32(&arr[3]).unwrap_or(792.0);
                        page.width = (x1 - x0).abs();
                        page.height = (y1 - y0).abs();
                    }
                }
            }

            // Check parent for inherited MediaBox if not found
            if page.width == 0.0 || page.height == 0.0 {
                page.width = 612.0;
                page.height = 792.0;
            }
        }
    }

    fn extract_positioned_text(&self, doc: &lopdf::Document, page_id: lopdf::ObjectId, page: &mut PdfPage) {
        let content_data = match doc.get_page_content(page_id) {
            Ok(data) => data,
            Err(_) => return,
        };

        let content = match lopdf::content::Content::decode(&content_data) {
            Ok(c) => c,
            Err(_) => return,
        };

        // Text state machine
        let mut current_x: f32 = 0.0;
        let mut current_y: f32 = 0.0;
        let mut font_size: f32 = 12.0;
        let mut font_key = String::from("F1");
        let mut in_text = false;
        let mut text_matrix = [1.0f32, 0.0, 0.0, 1.0, 0.0, 0.0]; // a,b,c,d,tx,ty
        let mut line_x: f32 = 0.0;
        let mut line_y: f32 = 0.0;

        // Collect font info for bold/italic detection
        let mut font_flags: HashMap<String, (bool, bool)> = HashMap::new();
        if let Ok(fonts) = doc.get_page_fonts(page_id) {
            for (key, dict) in &fonts {
                let key_str = String::from_utf8_lossy(key).to_string();
                let bold = self.font_is_bold(dict);
                let italic = self.font_is_italic(dict);
                font_flags.insert(key_str, (bold, italic));
            }
        }

        for op in content.operations.iter() {
            match op.operator.as_str() {
                "BT" => {
                    in_text = true;
                    text_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                    line_x = 0.0;
                    line_y = 0.0;
                }
                "ET" => {
                    in_text = false;
                }
                "Tf" if in_text => {
                    // Set font: /FontName size Tf
                    if let Some(name) = op.operands.first() {
                        if let Ok(n) = name.as_name() {
                            font_key = String::from_utf8_lossy(n).to_string();
                        }
                    }
                    if let Some(size) = op.operands.get(1) {
                        font_size = Self::obj_to_f32(size).unwrap_or(12.0).abs();
                        if font_size < 1.0 { font_size = 12.0; }
                    }
                }
                "Tm" if in_text => {
                    // Text matrix: a b c d tx ty Tm
                    if op.operands.len() >= 6 {
                        for (i, operand) in op.operands.iter().enumerate().take(6) {
                            text_matrix[i] = Self::obj_to_f32(operand).unwrap_or(text_matrix[i]);
                        }
                        current_x = text_matrix[4];
                        current_y = text_matrix[5];
                        line_x = current_x;
                        line_y = current_y;
                    }
                }
                "Td" | "TD" if in_text => {
                    // Move text position: tx ty Td
                    if op.operands.len() >= 2 {
                        let tx = Self::obj_to_f32(&op.operands[0]).unwrap_or(0.0);
                        let ty = Self::obj_to_f32(&op.operands[1]).unwrap_or(0.0);
                        line_x += tx;
                        line_y += ty;
                        current_x = line_x;
                        current_y = line_y;
                    }
                    if op.operator == "TD" {
                        // TD also sets leading
                    }
                }
                "T*" if in_text => {
                    // Move to next line (uses TL leading)
                    current_y -= font_size * 1.2; // Approximate leading
                    current_x = line_x;
                    line_y = current_y;
                }
                "Tj" if in_text => {
                    // Show text string: (text) Tj
                    if let Some(operand) = op.operands.first() {
                        let text = self.extract_text_from_operand(operand);
                        if !text.is_empty() {
                            let (bold, italic) = font_flags.get(&font_key)
                                .copied()
                                .unwrap_or((false, false));
                            page.text_elements.push(TextElement {
                                text: text.clone(),
                                x: current_x,
                                y: current_y,
                                font_size,
                                font_key: font_key.clone(),
                                bold,
                                italic,
                            });
                            // Approximate advance
                            current_x += text.len() as f32 * font_size * 0.5;
                        }
                    }
                }
                "TJ" if in_text => {
                    // Show text with kerning: [(text) kern (text) ...] TJ
                    if let Some(operand) = op.operands.first() {
                        if let Ok(arr) = operand.as_array() {
                            let mut combined = String::new();
                            for item in arr {
                                match item {
                                    lopdf::Object::String(bytes, _) => {
                                        let decoded = self.decode_pdf_string(bytes);
                                        combined.push_str(&decoded);
                                    }
                                    lopdf::Object::Integer(n) => {
                                        // Negative = move right, positive = move left
                                        // Large values (>100) usually indicate a space
                                        if *n < -100 || *n > 100 {
                                            combined.push(' ');
                                        }
                                    }
                                    lopdf::Object::Real(n) => {
                                        if *n < -100.0 || *n > 100.0 {
                                            combined.push(' ');
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            if !combined.is_empty() {
                                let (bold, italic) = font_flags.get(&font_key)
                                    .copied()
                                    .unwrap_or((false, false));
                                page.text_elements.push(TextElement {
                                    text: combined.clone(),
                                    x: current_x,
                                    y: current_y,
                                    font_size,
                                    font_key: font_key.clone(),
                                    bold,
                                    italic,
                                });
                                current_x += combined.len() as f32 * font_size * 0.5;
                            }
                        }
                    }
                }
                "'" if in_text => {
                    // Move to next line and show text
                    current_y -= font_size * 1.2;
                    current_x = line_x;
                    line_y = current_y;
                    if let Some(operand) = op.operands.first() {
                        let text = self.extract_text_from_operand(operand);
                        if !text.is_empty() {
                            let (bold, italic) = font_flags.get(&font_key)
                                .copied()
                                .unwrap_or((false, false));
                            page.text_elements.push(TextElement {
                                text,
                                x: current_x,
                                y: current_y,
                                font_size,
                                font_key: font_key.clone(),
                                bold,
                                italic,
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        // Build search text from positioned elements
        if page.text.is_empty() {
            let mut elements = page.text_elements.clone();
            // Sort by Y (descending = top first) then X
            elements.sort_by(|a, b| {
                let y_cmp = b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal);
                if y_cmp == std::cmp::Ordering::Equal {
                    a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal)
                } else {
                    y_cmp
                }
            });

            let mut last_y: f32 = f32::MAX;
            for elem in &elements {
                if (last_y - elem.y).abs() > elem.font_size * 0.8 {
                    if !page.text.is_empty() {
                        page.text.push('\n');
                    }
                }
                page.text.push_str(&elem.text);
                last_y = elem.y;
            }
        }
    }

    fn extract_page_images(&self, doc: &lopdf::Document, page_id: lopdf::ObjectId, page: &mut PdfPage) {
        if let Ok(images) = doc.get_page_images(page_id) {
            for pdf_img in &images {
                // Try to decode the image data
                if let Some(embedded) = self.decode_pdf_image(pdf_img, page) {
                    page.images.push(embedded);
                }
            }
        }
    }

    fn decode_pdf_image(&self, img: &lopdf::xobject::PdfImage, page: &PdfPage) -> Option<EmbeddedImage> {
        let width = img.width as u32;
        let height = img.height as u32;
        if width == 0 || height == 0 { return None; }

        let filters: Vec<String> = img.filters.clone().unwrap_or_default();
        let content = img.content;

        // Try to decode based on filter
        let rgba_data = if filters.iter().any(|f| f == "DCTDecode") {
            // JPEG data — decode with image crate
            match image::load_from_memory_with_format(content, image::ImageFormat::Jpeg) {
                Ok(img) => {
                    let rgba = img.to_rgba8();
                    Some(rgba.into_raw())
                }
                Err(_) => None,
            }
        } else if filters.iter().any(|f| f == "FlateDecode") || filters.is_empty() {
            // Raw pixel data (possibly zlib-compressed, lopdf may have already decoded)
            let bpc = img.bits_per_component.unwrap_or(8) as u32;
            let cs = img.color_space.as_deref().unwrap_or("DeviceRGB");
            self.raw_to_rgba(content, width, height, bpc, cs)
        } else {
            None
        };

        rgba_data.map(|data| EmbeddedImage {
            rgba_data: data,
            width,
            height,
            // Default position — actual position comes from content stream 'cm' operator
            x: 0.0,
            y: 0.0,
            display_width: width as f32,
            display_height: height as f32,
        })
    }

    fn raw_to_rgba(&self, data: &[u8], width: u32, height: u32, bpc: u32, color_space: &str) -> Option<Vec<u8>> {
        let pixel_count = (width * height) as usize;

        match color_space {
            "DeviceRGB" => {
                if data.len() >= pixel_count * 3 {
                    let mut rgba = Vec::with_capacity(pixel_count * 4);
                    for pixel in data.chunks_exact(3).take(pixel_count) {
                        rgba.push(pixel[0]);
                        rgba.push(pixel[1]);
                        rgba.push(pixel[2]);
                        rgba.push(255);
                    }
                    Some(rgba)
                } else {
                    None
                }
            }
            "DeviceGray" => {
                if data.len() >= pixel_count {
                    let mut rgba = Vec::with_capacity(pixel_count * 4);
                    for &gray in data.iter().take(pixel_count) {
                        rgba.push(gray);
                        rgba.push(gray);
                        rgba.push(gray);
                        rgba.push(255);
                    }
                    Some(rgba)
                } else {
                    None
                }
            }
            "DeviceCMYK" => {
                if data.len() >= pixel_count * 4 {
                    let mut rgba = Vec::with_capacity(pixel_count * 4);
                    for pixel in data.chunks_exact(4).take(pixel_count) {
                        let c = pixel[0] as f32 / 255.0;
                        let m = pixel[1] as f32 / 255.0;
                        let y = pixel[2] as f32 / 255.0;
                        let k = pixel[3] as f32 / 255.0;
                        let r = ((1.0 - c) * (1.0 - k) * 255.0) as u8;
                        let g = ((1.0 - m) * (1.0 - k) * 255.0) as u8;
                        let b = ((1.0 - y) * (1.0 - k) * 255.0) as u8;
                        rgba.push(r);
                        rgba.push(g);
                        rgba.push(b);
                        rgba.push(255);
                    }
                    Some(rgba)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn extract_graphics(&self, doc: &lopdf::Document, page_id: lopdf::ObjectId, page: &mut PdfPage) {
        let content_data = match doc.get_page_content(page_id) {
            Ok(data) => data,
            Err(_) => return,
        };

        let content = match lopdf::content::Content::decode(&content_data) {
            Ok(c) => c,
            Err(_) => return,
        };

        let mut stroke_color: [f32; 3] = [0.0, 0.0, 0.0];
        let mut fill_color: [f32; 3] = [0.0, 0.0, 0.0];
        let mut line_width: f32 = 1.0;
        let mut current_x: f32 = 0.0;
        let mut current_y: f32 = 0.0;
        let mut path_start_x: f32 = 0.0;
        let mut path_start_y: f32 = 0.0;
        let mut rect_pending: Option<[f32; 4]> = None;

        for op in content.operations.iter() {
            match op.operator.as_str() {
                "w" => {
                    // Line width
                    if let Some(w) = op.operands.first() {
                        line_width = Self::obj_to_f32(w).unwrap_or(1.0);
                    }
                }
                "RG" => {
                    // Stroke color RGB
                    if op.operands.len() >= 3 {
                        stroke_color[0] = Self::obj_to_f32(&op.operands[0]).unwrap_or(0.0);
                        stroke_color[1] = Self::obj_to_f32(&op.operands[1]).unwrap_or(0.0);
                        stroke_color[2] = Self::obj_to_f32(&op.operands[2]).unwrap_or(0.0);
                    }
                }
                "rg" => {
                    // Fill color RGB
                    if op.operands.len() >= 3 {
                        fill_color[0] = Self::obj_to_f32(&op.operands[0]).unwrap_or(0.0);
                        fill_color[1] = Self::obj_to_f32(&op.operands[1]).unwrap_or(0.0);
                        fill_color[2] = Self::obj_to_f32(&op.operands[2]).unwrap_or(0.0);
                    }
                }
                "G" => {
                    // Stroke gray
                    if let Some(g) = op.operands.first() {
                        let v = Self::obj_to_f32(g).unwrap_or(0.0);
                        stroke_color = [v, v, v];
                    }
                }
                "g" => {
                    // Fill gray
                    if let Some(g) = op.operands.first() {
                        let v = Self::obj_to_f32(g).unwrap_or(0.0);
                        fill_color = [v, v, v];
                    }
                }
                "re" => {
                    // Rectangle: x y w h re
                    if op.operands.len() >= 4 {
                        let x = Self::obj_to_f32(&op.operands[0]).unwrap_or(0.0);
                        let y = Self::obj_to_f32(&op.operands[1]).unwrap_or(0.0);
                        let w = Self::obj_to_f32(&op.operands[2]).unwrap_or(0.0);
                        let h = Self::obj_to_f32(&op.operands[3]).unwrap_or(0.0);
                        rect_pending = Some([x, y, w, h]);
                    }
                }
                "m" => {
                    // Move to: x y m
                    if op.operands.len() >= 2 {
                        current_x = Self::obj_to_f32(&op.operands[0]).unwrap_or(0.0);
                        current_y = Self::obj_to_f32(&op.operands[1]).unwrap_or(0.0);
                        path_start_x = current_x;
                        path_start_y = current_y;
                    }
                }
                "l" => {
                    // Line to: x y l
                    if op.operands.len() >= 2 {
                        let x2 = Self::obj_to_f32(&op.operands[0]).unwrap_or(0.0);
                        let y2 = Self::obj_to_f32(&op.operands[1]).unwrap_or(0.0);
                        page.graphics.push(GraphicPrimitive::Line {
                            x1: current_x, y1: current_y,
                            x2, y2,
                            color: stroke_color,
                            width: line_width,
                        });
                        current_x = x2;
                        current_y = y2;
                    }
                }
                "S" | "s" => {
                    // Stroke path
                    if let Some(r) = rect_pending.take() {
                        page.graphics.push(GraphicPrimitive::StrokeRect {
                            rect: r,
                            color: stroke_color,
                            width: line_width,
                        });
                    }
                }
                "f" | "F" | "f*" => {
                    // Fill path
                    if let Some(r) = rect_pending.take() {
                        page.graphics.push(GraphicPrimitive::FillRect {
                            rect: r,
                            color: fill_color,
                        });
                    }
                }
                "B" | "B*" => {
                    // Fill and stroke
                    if let Some(r) = rect_pending.take() {
                        page.graphics.push(GraphicPrimitive::FillRect {
                            rect: r,
                            color: fill_color,
                        });
                        page.graphics.push(GraphicPrimitive::StrokeRect {
                            rect: r,
                            color: stroke_color,
                            width: line_width,
                        });
                    }
                }
                "n" => {
                    // End path without stroke/fill (clipping)
                    rect_pending = None;
                }
                _ => {}
            }
        }
    }

    // ========================================================================
    // HELPER METHODS
    // ========================================================================

    fn obj_to_f32(obj: &lopdf::Object) -> Option<f32> {
        match obj {
            lopdf::Object::Integer(n) => Some(*n as f32),
            lopdf::Object::Real(n) => Some(*n as f32),
            lopdf::Object::Reference(_) => None, // Would need doc to dereference
            _ => None,
        }
    }

    fn extract_text_from_operand(&self, operand: &lopdf::Object) -> String {
        match operand {
            lopdf::Object::String(bytes, _) => self.decode_pdf_string(bytes),
            _ => String::new(),
        }
    }

    fn decode_pdf_string(&self, bytes: &[u8]) -> String {
        // Try UTF-16BE first (PDF text strings starting with BOM)
        if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
            let chars: Vec<u16> = bytes[2..].chunks_exact(2)
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .collect();
            return String::from_utf16_lossy(&chars);
        }

        // PDFDocEncoding (similar to Latin-1 for most chars)
        bytes.iter().map(|&b| b as char).collect()
    }

    fn font_is_bold(&self, font_dict: &lopdf::Dictionary) -> bool {
        if let Ok(name) = font_dict.get(b"BaseFont") {
            let name_str = format!("{:?}", name);
            let lower = name_str.to_ascii_lowercase();
            return lower.contains("bold") || lower.contains("heavy") || lower.contains("black");
        }
        false
    }

    fn font_is_italic(&self, font_dict: &lopdf::Dictionary) -> bool {
        if let Ok(name) = font_dict.get(b"BaseFont") {
            let name_str = format!("{:?}", name);
            let lower = name_str.to_ascii_lowercase();
            return lower.contains("italic") || lower.contains("oblique");
        }
        false
    }

    fn distribute_search_text(&mut self, full_text: &str) {
        // Distribute extracted text across pages for search
        let total_chars = full_text.len();
        if self.total_pages == 0 { return; }
        let chars_per_page = total_chars / self.total_pages.max(1);
        let chars: Vec<char> = full_text.chars().collect();

        for (i, page) in self.pages.iter_mut().enumerate() {
            if page.text.is_empty() {
                let start = i * chars_per_page;
                let end = ((i + 1) * chars_per_page).min(chars.len());
                if start < chars.len() {
                    page.text = chars[start..end].iter().collect();
                }
            }
        }
    }

    fn fallback_text_extraction(&mut self, data: &[u8]) {
        // Fallback path when lopdf fails
        let text = pdf_extract::extract_text_from_mem(data)
            .unwrap_or_else(|_| "[Unable to extract text]".to_string());

        self.total_pages = 1;
        self.pages.push(PdfPage {
            page_num: 0,
            text,
            width: 612.0,
            height: 792.0,
            text_elements: Vec::new(),
            images: Vec::new(),
            graphics: Vec::new(),
        });
    }

    // ========================================================================
    // SEARCH
    // ========================================================================

    fn search(&mut self, query: &str) {
        self.search_results.clear();
        self.current_search_index = 0;

        if query.is_empty() {
            return;
        }

        let query_lower = crate::fontcase::ascii_lower(query);

        for page in &self.pages {
            let text_lower = crate::fontcase::ascii_lower(&page.text);
            let mut start = 0;

            while let Some(pos) = text_lower[start..].find(&query_lower) {
                let abs_pos = start + pos;
                self.search_results.push(SearchResult {
                    page: page.page_num,
                    position: abs_pos,
                    text: query.to_string(),
                });
                start = abs_pos + 1;
            }
        }

        // Jump to first result
        if !self.search_results.is_empty() {
            self.current_page = self.search_results[0].page;
        }
    }

    fn next_search_result(&mut self) {
        if !self.search_results.is_empty() {
            self.current_search_index = (self.current_search_index + 1) % self.search_results.len();
            self.current_page = self.search_results[self.current_search_index].page;
        }
    }

    fn prev_search_result(&mut self) {
        if !self.search_results.is_empty() {
            self.current_search_index = if self.current_search_index == 0 {
                self.search_results.len() - 1
            } else {
                self.current_search_index - 1
            };
            self.current_page = self.search_results[self.current_search_index].page;
        }
    }

    // ========================================================================
    // RENDERING
    // ========================================================================

    pub fn render(&mut self, ui: &mut egui::Ui, file: &OpenFile, global_zoom: f32) {
        // Load PDF if needed
        if self.pdf_data.is_none() || self.pages.is_empty() {
            if let FileContent::Binary(data) = &file.content {
                self.load_pdf(data);
            }
        }

        // Toolbar
        self.render_toolbar(ui);
        ui.separator();

        // Error message
        if let Some(ref err) = self.error_message {
            ui.colored_label(Color32::YELLOW, format!("⚠ {}", err));
            ui.separator();
        }

        // Main content
        ui.horizontal(|ui| {
            // Thumbnails sidebar
            if self.show_thumbnails {
                egui::SidePanel::left("pdf_thumbs")
                    .resizable(true)
                    .min_width(120.0)
                    .max_width(200.0)
                    .show_inside(ui, |ui| {
                        self.render_thumbnails(ui);
                    });
            }

            // Page content
            if self.continuous_scroll {
                self.render_continuous(ui, global_zoom);
            } else {
                self.render_single_page(ui, global_zoom);
            }
        });
    }

    fn render_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Thumbnail toggle
            ui.toggle_value(&mut self.show_thumbnails, "Thumbnails");
            ui.toggle_value(&mut self.continuous_scroll, "Continuous");

            ui.separator();

            // Navigation
            if ui.button("⏮").on_hover_text("First page").clicked() {
                self.current_page = 0;
            }
            if ui.button("◀").on_hover_text("Previous page").clicked() {
                self.current_page = self.current_page.saturating_sub(1);
            }

            // Page number
            let mut page_str = format!("{}", self.current_page + 1);
            let resp = ui.add(egui::TextEdit::singleline(&mut page_str).desired_width(40.0));
            if resp.lost_focus() {
                if let Ok(p) = page_str.parse::<usize>() {
                    self.current_page = (p.saturating_sub(1)).min(self.total_pages.saturating_sub(1));
                }
            }
            ui.label(format!("/ {}", self.total_pages));

            if ui.button("▶").on_hover_text("Next page").clicked()
                && self.current_page + 1 < self.total_pages {
                self.current_page += 1;
            }
            if ui.button("⏭").on_hover_text("Last page").clicked() {
                self.current_page = self.total_pages.saturating_sub(1);
            }

            ui.separator();

            // Zoom
            if ui.button("➖").clicked() {
                self.zoom = (self.zoom - 0.1).max(0.3);
            }
            ui.label(format!("{:.0}%", self.zoom * 100.0));
            if ui.button("➕").clicked() {
                self.zoom = (self.zoom + 0.1).min(4.0);
            }
            if ui.button("Fit").on_hover_text("Fit to width").clicked() {
                self.zoom = 1.0;
            }

            ui.separator();

            // Tools
            ui.selectable_value(&mut self.current_tool, PdfTool::Select, "☛ Select");
            ui.selectable_value(&mut self.current_tool, PdfTool::Highlight, "Highlight");
            ui.selectable_value(&mut self.current_tool, PdfTool::Note, "Note");

            ui.separator();

            // Visual toggles
            ui.toggle_value(&mut self.show_images, "Images");
            ui.toggle_value(&mut self.show_graphics, "Graphics");

            // Search (right aligned)
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔍").clicked() {
                    let q = self.search_query.clone();
                    self.search(&q);
                }

                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("Search...")
                        .desired_width(150.0)
                );
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let q = self.search_query.clone();
                    self.search(&q);
                }

                if !self.search_results.is_empty() {
                    if ui.button("▼").clicked() { self.next_search_result(); }
                    if ui.button("▲").clicked() { self.prev_search_result(); }
                    ui.label(format!("{}/{}", self.current_search_index + 1, self.search_results.len()));
                }
            });
        });
    }

    fn render_thumbnails(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Pages").strong());
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            let thumb_width = ui.available_width() - 10.0;

            for i in 0..self.total_pages {
                let is_current = i == self.current_page;

                let page = self.pages.get(i);
                let aspect = page.map(|p| p.height / p.width.max(1.0)).unwrap_or(1.414);
                let thumb_height = thumb_width * aspect;

                // Thumbnail frame
                let bg = if is_current {
                    Color32::from_rgb(45, 55, 80)
                } else {
                    Color32::from_rgb(35, 38, 45)
                };
                let border = if is_current {
                    Stroke::new(2.0, Color32::from_rgb(100, 180, 255))
                } else {
                    Stroke::new(1.0, Color32::from_rgb(60, 65, 75))
                };

                let (rect, response) = ui.allocate_exact_size(
                    Vec2::new(thumb_width, thumb_height + 20.0),
                    Sense::click(),
                );

                if response.clicked() {
                    self.current_page = i;
                }

                // Draw thumbnail background (white page)
                let page_rect = Rect::from_min_size(
                    rect.min + Vec2::new(4.0, 2.0),
                    Vec2::new(thumb_width - 8.0, thumb_height - 4.0),
                );
                ui.painter().rect_filled(page_rect, 2.0, Color32::WHITE);
                ui.painter().rect_stroke(page_rect, 2.0, border);

                // Render mini text lines
                if let Some(page) = self.pages.get(i) {
                    let scale = (thumb_width - 16.0) / page.width.max(1.0);

                    // Draw mini representations of text elements
                    let max_elements = 40; // Limit for performance
                    for (idx, elem) in page.text_elements.iter().take(max_elements).enumerate() {
                        let tx = page_rect.left() + 4.0 + elem.x * scale;
                        let ty = page_rect.bottom() - 4.0 - elem.y * scale; // Flip Y
                        let text_width = elem.text.len() as f32 * elem.font_size * 0.3 * scale;
                        let text_height = (elem.font_size * scale).max(1.0).min(3.0);

                        if tx >= page_rect.left() && tx < page_rect.right()
                            && ty >= page_rect.top() && ty < page_rect.bottom()
                        {
                            let line_rect = Rect::from_min_size(
                                Pos2::new(tx, ty),
                                Vec2::new(text_width.min(page_rect.right() - tx), text_height),
                            );
                            ui.painter().rect_filled(line_rect, 0.0, Color32::from_rgb(80, 85, 95));
                        }
                    }

                    // If no text elements, show text lines placeholder
                    if page.text_elements.is_empty() && !page.text.is_empty() {
                        let line_count = page.text.lines().count().min(20);
                        for line_idx in 0..line_count {
                            let ly = page_rect.top() + 4.0 + line_idx as f32 * 4.0;
                            if ly + 2.0 > page_rect.bottom() { break; }
                            let lw = (thumb_width - 20.0) * (0.6 + ((line_idx * 7) % 4) as f32 * 0.1);
                            let line_rect = Rect::from_min_size(
                                Pos2::new(page_rect.left() + 4.0, ly),
                                Vec2::new(lw, 2.0),
                            );
                            ui.painter().rect_filled(line_rect, 0.0, Color32::from_rgb(180, 180, 190));
                        }
                    }
                }

                // Page number label
                let label_pos = Pos2::new(rect.center().x, rect.max.y - 8.0);
                ui.painter().text(
                    label_pos,
                    egui::Align2::CENTER_CENTER,
                    format!("{}", i + 1),
                    FontId::proportional(10.0),
                    if is_current { Color32::from_rgb(100, 180, 255) } else { Color32::GRAY },
                );

                ui.add_space(4.0);
            }
        });
    }

    fn render_single_page(&mut self, ui: &mut egui::Ui, global_zoom: f32) {
        let combined_zoom = self.zoom * global_zoom;

        if self.pages.is_empty() {
            ui.centered_and_justified(|ui| {
                if self.loading {
                    ui.spinner();
                    ui.label("Loading PDF...");
                } else {
                    ui.label("No PDF loaded");
                }
            });
            return;
        }

        let page = match self.pages.get(self.current_page) {
            Some(p) => p.clone(),
            None => return,
        };

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.render_page_frame(ui, &page, combined_zoom);
            });
    }

    fn render_continuous(&mut self, ui: &mut egui::Ui, global_zoom: f32) {
        let combined_zoom = self.zoom * global_zoom;

        if self.pages.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("No PDF loaded");
            });
            return;
        }

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let pages_clone: Vec<PdfPage> = self.pages.clone();
                for page in &pages_clone {
                    self.render_page_frame(ui, page, combined_zoom);
                    ui.add_space(20.0); // Gap between pages
                }
            });
    }

    fn render_page_frame(&mut self, ui: &mut egui::Ui, page: &PdfPage, zoom: f32) {
        let page_width = page.width * zoom;
        let page_height = page.height * zoom;

        ui.vertical_centered(|ui| {
            // Page shadow
            egui::Frame::none()
                .fill(Color32::WHITE)
                .stroke(Stroke::new(1.0, Color32::from_rgb(180, 180, 190)))
                .shadow(egui::epaint::Shadow {
                    offset: Vec2::new(3.0, 3.0),
                    blur: 6.0,
                    spread: 0.0,
                    color: Color32::from_black_alpha(50),
                })
                .show(ui, |ui| {
                    // Allocate the full page area
                    let (page_rect, _response) = ui.allocate_exact_size(
                        Vec2::new(page_width, page_height),
                        Sense::click_and_drag(),
                    );

                    let painter = ui.painter_at(page_rect);

                    // White page background
                    painter.rect_filled(page_rect, 0.0, Color32::WHITE);

                    // Render graphic primitives (behind text)
                    if self.show_graphics {
                        self.paint_graphics(&painter, page, page_rect, zoom);
                    }

                    // Render embedded images
                    if self.show_images {
                        self.paint_images(ui, &painter, page, page_rect, zoom);
                    }

                    // Render positioned text
                    if !page.text_elements.is_empty() {
                        self.paint_positioned_text(&painter, page, page_rect, zoom);
                    } else {
                        // Fallback: render raw text
                        self.paint_raw_text(&painter, page, page_rect, zoom);
                    }

                    // Page number footer
                    painter.text(
                        Pos2::new(page_rect.center().x, page_rect.bottom() - 12.0 * zoom),
                        egui::Align2::CENTER_CENTER,
                        format!("— {} —", page.page_num + 1),
                        FontId::proportional(9.0 * zoom),
                        Color32::from_rgb(160, 160, 170),
                    );
                });
        });
    }

    fn paint_graphics(&self, painter: &egui::Painter, page: &PdfPage, page_rect: Rect, zoom: f32) {
        let scale_x = zoom;
        let scale_y = zoom;
        let page_h = page.height;

        for prim in &page.graphics {
            match prim {
                GraphicPrimitive::FillRect { rect, color } => {
                    let c = Color32::from_rgb(
                        (color[0] * 255.0) as u8,
                        (color[1] * 255.0) as u8,
                        (color[2] * 255.0) as u8,
                    );
                    // Skip white/near-white fills that would be invisible
                    if color[0] > 0.98 && color[1] > 0.98 && color[2] > 0.98 { continue; }

                    let r = Rect::from_min_size(
                        Pos2::new(
                            page_rect.left() + rect[0] * scale_x,
                            page_rect.top() + (page_h - rect[1] - rect[3]) * scale_y,
                        ),
                        Vec2::new(rect[2] * scale_x, rect[3] * scale_y),
                    );
                    // Clamp to page
                    let r = r.intersect(page_rect);
                    if r.width() > 0.0 && r.height() > 0.0 {
                        painter.rect_filled(r, 0.0, c);
                    }
                }
                GraphicPrimitive::StrokeRect { rect, color, width } => {
                    let c = Color32::from_rgb(
                        (color[0] * 255.0) as u8,
                        (color[1] * 255.0) as u8,
                        (color[2] * 255.0) as u8,
                    );
                    let r = Rect::from_min_size(
                        Pos2::new(
                            page_rect.left() + rect[0] * scale_x,
                            page_rect.top() + (page_h - rect[1] - rect[3]) * scale_y,
                        ),
                        Vec2::new(rect[2] * scale_x, rect[3] * scale_y),
                    );
                    let r = r.intersect(page_rect);
                    if r.width() > 0.0 && r.height() > 0.0 {
                        painter.rect_stroke(r, 0.0, Stroke::new(*width * zoom, c));
                    }
                }
                GraphicPrimitive::Line { x1, y1, x2, y2, color, width } => {
                    let c = Color32::from_rgb(
                        (color[0] * 255.0) as u8,
                        (color[1] * 255.0) as u8,
                        (color[2] * 255.0) as u8,
                    );
                    let p1 = Pos2::new(
                        page_rect.left() + x1 * scale_x,
                        page_rect.top() + (page_h - y1) * scale_y,
                    );
                    let p2 = Pos2::new(
                        page_rect.left() + x2 * scale_x,
                        page_rect.top() + (page_h - y2) * scale_y,
                    );
                    painter.line_segment([p1, p2], Stroke::new(*width * zoom, c));
                }
            }
        }
    }

    fn paint_images(&mut self, ui: &egui::Ui, painter: &egui::Painter, page: &PdfPage, page_rect: Rect, zoom: f32) {
        for (img_idx, img) in page.images.iter().enumerate() {
            let tex_key = format!("pdf_img_{}_{}", page.page_num, img_idx);

            // Create or reuse texture
            let texture = self.image_textures.entry(tex_key.clone()).or_insert_with(|| {
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    [img.width as usize, img.height as usize],
                    &img.rgba_data,
                );
                ui.ctx().load_texture(&tex_key, color_image, egui::TextureOptions::LINEAR)
            });

            // Position the image on the page
            let img_width = img.display_width * zoom;
            let img_height = img.display_height * zoom;

            // Center images that don't have explicit positioning
            let ix = if img.x > 0.0 {
                page_rect.left() + img.x * zoom
            } else {
                page_rect.center().x - img_width / 2.0
            };
            let iy = if img.y > 0.0 {
                page_rect.top() + (page.height - img.y - img.display_height) * zoom
            } else {
                page_rect.top() + 20.0 * zoom
            };

            let img_rect = Rect::from_min_size(
                Pos2::new(ix, iy),
                Vec2::new(img_width, img_height),
            );

            painter.image(texture.id(), img_rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        }
    }

    fn paint_positioned_text(&self, painter: &egui::Painter, page: &PdfPage, page_rect: Rect, zoom: f32) {
        let page_h = page.height;

        for elem in &page.text_elements {
            if elem.text.trim().is_empty() { continue; }

            let font_size = (elem.font_size * zoom).max(4.0).min(100.0);

            // Convert PDF coordinates (origin bottom-left) to screen (origin top-left)
            let screen_x = page_rect.left() + elem.x * zoom;
            let screen_y = page_rect.top() + (page_h - elem.y) * zoom;

            // Skip if outside page bounds
            if screen_x < page_rect.left() - 50.0 || screen_x > page_rect.right() + 50.0
                || screen_y < page_rect.top() - 20.0 || screen_y > page_rect.bottom() + 20.0
            {
                continue;
            }

            let font_id = if elem.bold {
                FontId::new(font_size, egui::FontFamily::Proportional)
            } else {
                FontId::proportional(font_size)
            };

            let color = Color32::from_rgb(30, 30, 35);

            painter.text(
                Pos2::new(screen_x, screen_y),
                egui::Align2::LEFT_BOTTOM,
                &elem.text,
                font_id,
                color,
            );
        }
    }

    fn paint_raw_text(&self, painter: &egui::Painter, page: &PdfPage, page_rect: Rect, zoom: f32) {
        if page.text.trim().is_empty() {
            painter.text(
                page_rect.center(),
                egui::Align2::CENTER_CENTER,
                "[No text content on this page]",
                FontId::proportional(14.0 * zoom),
                Color32::GRAY,
            );
            return;
        }

        // Render as flowing text with word wrap
        let margin = 36.0 * zoom; // 0.5 inch margins
        let font_size = 11.0 * zoom;
        let line_height = font_size * 1.4;
        let max_width = page_rect.width() - margin * 2.0;

        let mut y = page_rect.top() + margin;

        for line in page.text.lines() {
            if y + line_height > page_rect.bottom() - margin {
                break; // Don't overflow page
            }

            if line.trim().is_empty() {
                y += line_height * 0.5;
                continue;
            }

            // Simple word wrap
            let chars_per_line = (max_width / (font_size * 0.55)) as usize;
            let mut remaining = line;

            while !remaining.is_empty() && y + line_height <= page_rect.bottom() - margin {
                let chunk_end = if remaining.len() <= chars_per_line {
                    remaining.len()
                } else {
                    // Find word break
                    remaining[..chars_per_line.min(remaining.len())]
                        .rfind(' ')
                        .map(|p| p + 1)
                        .unwrap_or(chars_per_line.min(remaining.len()))
                };

                let chunk = &remaining[..chunk_end];
                painter.text(
                    Pos2::new(page_rect.left() + margin, y),
                    egui::Align2::LEFT_TOP,
                    chunk,
                    FontId::proportional(font_size),
                    Color32::from_rgb(30, 30, 35),
                );

                remaining = remaining[chunk_end..].trim_start();
                y += line_height;
            }
        }
    }

    // ========================================================================
    // EXPORT
    // ========================================================================

    pub fn export_text(&self) -> String {
        self.pages.iter()
            .map(|p| format!("=== Page {} ===\n{}\n", p.page_num + 1, p.text))
            .collect()
    }

    pub fn get_page_text(&self, page: usize) -> Option<&str> {
        self.pages.get(page).map(|p| p.text.as_str())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_viewer_new() {
        let viewer = PdfViewer::new();
        assert_eq!(viewer.current_page, 0);
        assert_eq!(viewer.zoom, 1.0);
        assert_eq!(viewer.total_pages, 0);
        assert!(viewer.show_thumbnails);
        assert!(!viewer.continuous_scroll);
    }

    #[test]
    fn test_pdf_decode_string_latin1() {
        let viewer = PdfViewer::new();
        let bytes = b"Hello World";
        assert_eq!(viewer.decode_pdf_string(bytes), "Hello World");
    }

    #[test]
    fn test_obj_to_f32() {
        assert_eq!(PdfViewer::obj_to_f32(&lopdf::Object::Integer(42)), Some(42.0));
        assert_eq!(PdfViewer::obj_to_f32(&lopdf::Object::Real(3.14)), Some(3.14));
        assert_eq!(PdfViewer::obj_to_f32(&lopdf::Object::Boolean(true)), None);
    }
}
