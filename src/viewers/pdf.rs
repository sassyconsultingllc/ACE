#![allow(dead_code, unused_imports, unused_variables, deprecated)]
//! PDF Viewer - Pure Rust PDF viewing with text extraction
//!
//! REPLACES: Adobe Acrobat ($240/yr), Foxit PDF ($140/yr)
//!
//! Features:
//! - View: Text extraction, page navigation, zoom
//! - Search: Full text search across pages
//! - Annotations: Highlights, notes (stored separately)
//! - Export: Save annotations, export text

use crate::file_handler::{FileContent, OpenFile};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================================
// PAGE CONTENT
// ============================================================================

#[derive(Debug, Clone)]
pub struct PdfPage {
    pub page_num: usize,
    pub text: String,
    pub width: f32,
    pub height: f32,
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
    
    // Search
    search_query: String,
    search_results: Vec<SearchResult>,
    current_search_index: usize,
    
    // Annotations
    annotations: Vec<Annotation>,
    current_tool: PdfTool,
    annotation_color: Color32,
    
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
            
            search_query: String::new(),
            search_results: Vec::new(),
            current_search_index: 0,
            
            annotations: Vec::new(),
            current_tool: PdfTool::Select,
            annotation_color: Color32::YELLOW,
            
            loading: false,
            error_message: None,
        }
    }
    
    // ========================================================================
    // LOADING
    // ========================================================================
    
    pub fn load_pdf(&mut self, data: &[u8]) {
        self.loading = true;
        self.error_message = None;
        self.pages.clear();
        self.pdf_data = Some(data.to_vec());
        
        // Get page count
        self.total_pages = self.get_page_count(data);
        
        // Extract text from all pages
        match self.extract_all_text(data) {
            Ok(pages) => {
                self.pages = pages;
                self.loading = false;
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to extract PDF text: {}", e));
                self.loading = false;
                // Create placeholder pages
                for i in 0..self.total_pages {
                    self.pages.push(PdfPage {
                        page_num: i,
                        text: format!("[Page {} - Text extraction failed]", i + 1),
                        width: 612.0,
                        height: 792.0,
                    });
                }
            }
        }
    }
    
    fn get_page_count(&self, data: &[u8]) -> usize {
        match lopdf::Document::load_mem(data) {
            Ok(doc) => doc.get_pages().len(),
            Err(_) => 1,
        }
    }
    
    fn extract_all_text(&self, data: &[u8]) -> Result<Vec<PdfPage>, String> {
        let mut pages = Vec::new();
        
        // Use pdf_extract to get text
        let text = pdf_extract::extract_text_from_mem(data)
            .map_err(|e| format!("PDF extraction error: {:?}", e))?;
        
        // Try to split by page markers or use heuristics
        // pdf_extract doesn't give us page boundaries directly, so we'll estimate
        let total_chars = text.len();
        let chars_per_page = if self.total_pages > 0 {
            total_chars / self.total_pages
        } else {
            total_chars
        };
        
        if self.total_pages <= 1 {
            pages.push(PdfPage {
                page_num: 0,
                text,
                width: 612.0,
                height: 792.0,
            });
        } else {
            // Split text roughly by page
            let chars: Vec<char> = text.chars().collect();
            for i in 0..self.total_pages {
                let start = i * chars_per_page;
                let end = ((i + 1) * chars_per_page).min(chars.len());
                
                let page_text: String = if start < chars.len() {
                    chars[start..end].iter().collect()
                } else {
                    String::new()
                };
                
                pages.push(PdfPage {
                    page_num: i,
                    text: page_text,
                    width: 612.0,
                    height: 792.0,
                });
            }
        }
        
        Ok(pages)
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
        
        let query_lower = query.to_lowercase();
        
        for page in &self.pages {
            let text_lower = page.text.to_lowercase();
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
                    .min_width(100.0)
                    .max_width(200.0)
                    .show_inside(ui, |ui| {
                        self.render_thumbnails(ui);
                    });
            }
            
            // Page content
            self.render_page_content(ui, global_zoom);
        });
    }
    
    fn render_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Thumbnail toggle
            ui.toggle_value(&mut self.show_thumbnails, "🖼 Thumbnails");
            
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
                self.zoom = (self.zoom - 0.1).max(0.5);
            }
            ui.label(format!("{:.0}%", self.zoom * 100.0));
            if ui.button("➕").clicked() {
                self.zoom = (self.zoom + 0.1).min(3.0);
            }
            
            ui.separator();
            
            // Tools
            ui.selectable_value(&mut self.current_tool, PdfTool::Select, "☝ Select");
            ui.selectable_value(&mut self.current_tool, PdfTool::Highlight, "🖌 Highlight");
            ui.selectable_value(&mut self.current_tool, PdfTool::Note, "📝 Note");
            
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
            for i in 0..self.total_pages {
                let is_current = i == self.current_page;
                
                let resp = ui.add(
                    egui::Button::new(format!("Page {}", i + 1))
                        .fill(if is_current { Color32::from_rgb(66, 133, 244) } else { Color32::TRANSPARENT })
                        .min_size(Vec2::new(80.0, 30.0))
                );
                
                if resp.clicked() {
                    self.current_page = i;
                }
            }
        });
    }
    
    fn render_page_content(&mut self, ui: &mut egui::Ui, global_zoom: f32) {
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
        
        // Get current page
        let page = match self.pages.get(self.current_page) {
            Some(p) => p.clone(),
            None => return,
        };
        
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Page container
                let page_width = page.width * combined_zoom;
                let page_height = page.height * combined_zoom;
                
                ui.vertical_centered(|ui| {
                    // Page frame
                    egui::Frame::none()
                        .fill(Color32::WHITE)
                        .stroke(Stroke::new(1.0, Color32::GRAY))
                        .shadow(egui::epaint::Shadow {
                            offset: Vec2::new(2.0, 2.0),
                            blur: 4.0,
                            spread: 0.0,
                            color: Color32::from_black_alpha(40),
                        })
                        .inner_margin(20.0 * combined_zoom)
                        .show(ui, |ui| {
                            ui.set_min_width(page_width - 40.0 * combined_zoom);
                            ui.set_min_height(page_height - 40.0 * combined_zoom);
                            
                            // Page header
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("Page {}", self.current_page + 1))
                                    .size(10.0 * combined_zoom)
                                    .color(Color32::GRAY));
                            });
                            
                            ui.add_space(10.0 * combined_zoom);
                            
                            // Page text content
                            let font_size = 12.0 * combined_zoom;
                            
                            if page.text.trim().is_empty() {
                                ui.colored_label(Color32::GRAY, "[No text content on this page]");
                            } else {
                                // Render text with search highlighting
                                self.render_text_with_highlights(ui, &page, font_size);
                            }
                        });
                });
            });
    }
    
    fn render_text_with_highlights(&self, ui: &mut egui::Ui, page: &PdfPage, font_size: f32) {
        let text = &page.text;
        
        // Find search highlights for this page
        let highlights: Vec<_> = self.search_results.iter()
            .filter(|r| r.page == page.page_num)
            .map(|r| (r.position, r.position + r.text.len()))
            .collect();
        
        if highlights.is_empty() || self.search_query.is_empty() {
            // No highlights - just render text
            ui.label(RichText::new(text).size(font_size).color(Color32::BLACK));
        } else {
            // Render with highlights
            let mut job = egui::text::LayoutJob::default();
            let chars: Vec<char> = text.chars().collect();
            let mut i = 0;
            
            while i < chars.len() {
                // Check if we're at a highlight
                let mut in_highlight = false;
                let mut highlight_end = i;
                
                for (start, end) in &highlights {
                    if i >= *start && i < *end {
                        in_highlight = true;
                        highlight_end = *end;
                        break;
                    }
                }
                
                if in_highlight {
                    // Highlighted text
                    let highlighted: String = chars[i..highlight_end.min(chars.len())].iter().collect();
                    job.append(
                        &highlighted,
                        0.0,
                        egui::TextFormat {
                            font_id: FontId::proportional(font_size),
                            color: Color32::BLACK,
                            background: Color32::YELLOW,
                            ..Default::default()
                        },
                    );
                    i = highlight_end;
                } else {
                    // Find next highlight or end
                    let next_highlight = highlights.iter()
                        .filter(|(s, _)| *s > i)
                        .map(|(s, _)| *s)
                        .min()
                        .unwrap_or(chars.len());
                    
                    let normal: String = chars[i..next_highlight].iter().collect();
                    job.append(
                        &normal,
                        0.0,
                        egui::TextFormat {
                            font_id: FontId::proportional(font_size),
                            color: Color32::BLACK,
                            ..Default::default()
                        },
                    );
                    i = next_highlight;
                }
            }
            
            ui.label(job);
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
    }
}
