//! PDF Viewer - View, navigate, and print PDFs

use crate::file_handler::{FileContent, OpenFile};
use eframe::egui::{self, Color32, ColorImage, TextureHandle, Vec2, Rect, Pos2, RichText};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct PdfViewer {
    current_page: usize,
    total_pages: usize,
    page_cache: HashMap<(PathBuf, usize), TextureHandle>,
    fit_mode: FitMode,
    show_thumbnails: bool,
    search_query: String,
    search_results: Vec<SearchResult>,
}

#[derive(Clone, Copy, PartialEq)]
enum FitMode {
    FitWidth,
    FitPage,
    Actual,
}

struct SearchResult {
    page: usize,
    text: String,
}

impl PdfViewer {
    pub fn new() -> Self {
        Self {
            current_page: 0,
            total_pages: 0,
            page_cache: HashMap::new(),
            fit_mode: FitMode::FitPage,
            show_thumbnails: false,
            search_query: String::new(),
            search_results: Vec::new(),
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui, file: &OpenFile, zoom: f32) {
        // Try to get page count from PDF
        if let FileContent::Binary(data) = &file.content {
            self.total_pages = self.get_page_count(data);
        }
        
        // Toolbar
        ui.horizontal(|ui| {
            // Navigation
            if ui.button("⏮").on_hover_text("First page").clicked() {
                self.current_page = 0;
            }
            if ui.button("◀").on_hover_text("Previous page").clicked() {
                self.current_page = self.current_page.saturating_sub(1);
            }
            
            ui.label(format!("{} / {}", self.current_page + 1, self.total_pages.max(1)));
            
            if ui.button("▶").on_hover_text("Next page").clicked() {
                if self.current_page + 1 < self.total_pages {
                    self.current_page += 1;
                }
            }
            if ui.button("⏭").on_hover_text("Last page").clicked() {
                self.current_page = self.total_pages.saturating_sub(1);
            }
            
            ui.separator();
            
            // Fit modes
            if ui.selectable_label(self.fit_mode == FitMode::FitPage, "📄 Fit Page").clicked() {
                self.fit_mode = FitMode::FitPage;
            }
            if ui.selectable_label(self.fit_mode == FitMode::FitWidth, "↔ Fit Width").clicked() {
                self.fit_mode = FitMode::FitWidth;
            }
            if ui.selectable_label(self.fit_mode == FitMode::Actual, "🔍 100%").clicked() {
                self.fit_mode = FitMode::Actual;
            }
            
            ui.separator();
            
            ui.checkbox(&mut self.show_thumbnails, "📑 Thumbnails");
            
            ui.separator();
            
            // Search
            ui.label("🔍");
            let search_response = ui.text_edit_singleline(&mut self.search_query);
            if search_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.search_pdf(file);
            }
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("Zoom: {:.0}%", zoom * 100.0));
            });
        });
        
        ui.separator();
        
        // Main content area with optional thumbnails
        ui.horizontal(|ui| {
            // Thumbnail panel
            if self.show_thumbnails {
                egui::SidePanel::left("pdf_thumbnails")
                    .resizable(true)
                    .default_width(150.0)
                    .show_inside(ui, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for page in 0..self.total_pages {
                                let selected = page == self.current_page;
                                let response = ui.selectable_label(
                                    selected,
                                    format!("Page {}", page + 1),
                                );
                                if response.clicked() {
                                    self.current_page = page;
                                }
                            }
                        });
                    });
            }
            
            // Main page view
            egui::CentralPanel::default().show_inside(ui, |ui| {
                self.render_page(ui, file, zoom);
            });
        });
        
        // Search results panel
        if !self.search_results.is_empty() {
            egui::TopBottomPanel::bottom("search_results")
                .resizable(true)
                .default_height(100.0)
                .show_inside(ui, |ui| {
                    ui.heading(format!("🔍 Search Results: {}", self.search_results.len()));
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for result in &self.search_results {
                            if ui.button(format!("Page {}: {}", result.page + 1, &result.text)).clicked() {
                                self.current_page = result.page;
                            }
                        }
                    });
                });
        }
    }
    
    fn render_page(&mut self, ui: &mut egui::Ui, file: &OpenFile, zoom: f32) {
        let available = ui.available_size();
        
        if let FileContent::Binary(data) = &file.content {
            // Try to render with pdfium or fallback to text extraction
            if let Some(texture) = self.render_page_texture(ui.ctx(), &file.path, data, self.current_page) {
                let img_size = texture.size_vec2();
                
                let display_size = match self.fit_mode {
                    FitMode::FitPage => {
                        let scale_x = available.x / img_size.x;
                        let scale_y = available.y / img_size.y;
                        img_size * scale_x.min(scale_y) * zoom
                    }
                    FitMode::FitWidth => {
                        let scale = available.x / img_size.x;
                        img_size * scale * zoom
                    }
                    FitMode::Actual => img_size * zoom,
                };
                
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let (rect, _response) = ui.allocate_exact_size(
                            display_size.max(available),
                            egui::Sense::click_and_drag(),
                        );
                        
                        // Center the page
                        let offset = (available - display_size).max(Vec2::ZERO) / 2.0;
                        let page_rect = Rect::from_min_size(
                            rect.min + offset,
                            display_size,
                        );
                        
                        ui.painter().image(
                            texture.id(),
                            page_rect,
                            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    });
            } else {
                // Fallback: Show extracted text
                ui.centered_and_justified(|ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.heading(format!("Page {}", self.current_page + 1));
                        ui.separator();
                        
                        if let Some(text) = self.extract_page_text(data, self.current_page) {
                            ui.label(&text);
                        } else {
                            ui.label("Unable to render PDF page. PDF rendering requires pdfium library.");
                            ui.label("");
                            ui.label("To enable full PDF rendering:");
                            ui.label("1. Download pdfium from: https://pdfium.googlesource.com/pdfium/");
                            ui.label("2. Place pdfium.dll in the application directory");
                        }
                    });
                });
            }
        }
    }
    
    fn render_page_texture(
        &mut self,
        ctx: &egui::Context,
        path: &PathBuf,
        data: &[u8],
        page: usize,
    ) -> Option<TextureHandle> {
        let cache_key = (path.clone(), page);
        
        if let Some(texture) = self.page_cache.get(&cache_key) {
            return Some(texture.clone());
        }
        
        // Try to render with pdfium
        #[cfg(feature = "pdfium")]
        {
            use pdfium_render::prelude::*;
            
            if let Ok(pdfium) = Pdfium::default() {
                if let Ok(document) = pdfium.load_pdf_from_byte_slice(data, None) {
                    if let Ok(page_obj) = document.pages().get(page as u16) {
                        let render_config = PdfRenderConfig::new()
                            .set_target_width(1200)
                            .set_maximum_height(1600);
                        
                        if let Ok(bitmap) = page_obj.render_with_config(&render_config) {
                            let width = bitmap.width() as usize;
                            let height = bitmap.height() as usize;
                            
                            let color_image = ColorImage::from_rgba_unmultiplied(
                                [width, height],
                                bitmap.as_bytes(),
                            );
                            
                            let texture = ctx.load_texture(
                                format!("pdf_page_{}_{}", path.display(), page),
                                color_image,
                                egui::TextureOptions::LINEAR,
                            );
                            
                            self.page_cache.insert(cache_key, texture.clone());
                            return Some(texture);
                        }
                    }
                }
            }
        }
        
        None
    }
    
    fn get_page_count(&self, data: &[u8]) -> usize {
        // Try to parse page count from PDF
        if let Ok(doc) = lopdf::Document::load_mem(data) {
            return doc.get_pages().len();
        }
        1
    }
    
    fn extract_page_text(&self, data: &[u8], page: usize) -> Option<String> {
        // Basic text extraction from PDF
        if let Ok(doc) = lopdf::Document::load_mem(data) {
            let pages = doc.get_pages();
            if let Some((&page_id, _)) = pages.iter().nth(page) {
                if let Ok(text) = doc.extract_text(&[page_id as u32]) {
                    return Some(text);
                }
            }
        }
        None
    }
    
    fn search_pdf(&mut self, file: &OpenFile) {
        self.search_results.clear();
        
        if self.search_query.is_empty() {
            return;
        }
        
        if let FileContent::Binary(data) = &file.content {
            if let Ok(doc) = lopdf::Document::load_mem(data) {
                let pages = doc.get_pages();
                for (idx, (&page_id, _)) in pages.iter().enumerate() {
                    if let Ok(text) = doc.extract_text(&[page_id as u32]) {
                        let query_lower = self.search_query.to_lowercase();
                        let text_lower = text.to_lowercase();
                        
                        if text_lower.contains(&query_lower) {
                            // Find context around match
                            if let Some(pos) = text_lower.find(&query_lower) {
                                let start = pos.saturating_sub(30);
                                let end = (pos + self.search_query.len() + 30).min(text.len());
                                let context = &text[start..end];
                                
                                self.search_results.push(SearchResult {
                                    page: idx,
                                    text: format!("...{}...", context.trim()),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
}
