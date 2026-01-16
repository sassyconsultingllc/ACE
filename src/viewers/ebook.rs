//! eBook Viewer - EPUB, MOBI reader with chapter navigation
//! 
//! Features:
//! - Table of contents navigation
//! - Chapter reading with HTML rendering
//! - Cover art display
//! - Reading progress tracking
//! - Font size adjustment
//! - Night mode / sepia mode support

use crate::file_handler::{OpenFile, EbookContent, EbookChapter};
use eframe::egui::{self, Color32, FontId, RichText, Stroke, Vec2};

/// Reading theme
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadingTheme {
    Light,
    Dark,
    Sepia,
}

/// eBook viewer state
pub struct EbookViewer {
    current_chapter: usize,
    scroll_position: f32,
    font_size: f32,
    theme: ReadingTheme,
    show_toc: bool,
    search_query: String,
    bookmarks: Vec<(usize, String)>, // (chapter, note)
}

impl EbookViewer {
    pub fn new() -> Self {
        Self {
            current_chapter: 0,
            scroll_position: 0.0,
            font_size: 16.0,
            theme: ReadingTheme::Dark,
            show_toc: true,
            search_query: String::new(),
            bookmarks: Vec::new(),
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui, file: &OpenFile, zoom: f32) {
        let available = ui.available_size();
        
        // Theme colors
        let (bg_color, text_color, accent_color) = match self.theme {
            ReadingTheme::Light => (
                Color32::from_rgb(255, 255, 255),
                Color32::from_rgb(30, 30, 30),
                Color32::from_rgb(0, 100, 200),
            ),
            ReadingTheme::Dark => (
                Color32::from_rgb(25, 28, 35),
                Color32::from_rgb(220, 220, 220),
                Color32::from_rgb(100, 150, 255),
            ),
            ReadingTheme::Sepia => (
                Color32::from_rgb(250, 240, 220),
                Color32::from_rgb(60, 50, 40),
                Color32::from_rgb(150, 100, 50),
            ),
        };
        
        ui.horizontal(|ui| {
            // TOC sidebar
            if self.show_toc {
                egui::Frame::none()
                    .fill(Color32::from_rgb(35, 38, 45))
                    .show(ui, |ui| {
                        ui.set_min_width(250.0);
                        ui.set_max_width(250.0);
                        ui.set_min_height(available.y);
                        
                        self.render_sidebar(ui, file, accent_color);
                    });
            }
            
            // Main reading area
            egui::Frame::none()
                .fill(bg_color)
                .inner_margin(24.0)
                .show(ui, |ui| {
                    ui.set_min_width(if self.show_toc { available.x - 270.0 } else { available.x });
                    
                    self.render_content(ui, file, text_color, accent_color);
                });
        });
    }
    
    fn render_sidebar(&mut self, ui: &mut egui::Ui, file: &OpenFile, accent: Color32) {
        ui.vertical(|ui| {
            ui.add_space(12.0);
            
            // Header with toggle
            ui.horizontal(|ui| {
                ui.heading(RichText::new("📚 Contents").size(16.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("◀").on_hover_text("Hide sidebar").clicked() {
                        self.show_toc = false;
                    }
                });
            });
            
            ui.add_space(8.0);
            ui.separator();
            
            // Cover image (if available)
            if let Some(ref ebook) = file.ebook {
                if ebook.cover_image.is_some() {
                    ui.add_space(8.0);
                    ui.centered_and_justified(|ui| {
                        egui::Frame::none()
                            .fill(Color32::from_gray(50))
                            .rounding(4.0)
                            .show(ui, |ui| {
                                ui.set_min_size(Vec2::new(150.0, 200.0));
                                ui.centered_and_justified(|ui| {
                                    ui.label(RichText::new("📖").size(60.0));
                                });
                            });
                    });
                    ui.add_space(8.0);
                }
                
                // Book title and author
                if let Some(ref title) = ebook.title {
                    ui.add_space(8.0);
                    ui.label(RichText::new(title).size(14.0).strong());
                }
                if let Some(ref author) = ebook.author {
                    ui.label(RichText::new(author).size(12.0).color(Color32::GRAY));
                }
                
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
            }
            
            // Table of Contents
            egui::ScrollArea::vertical()
                .id_source("toc_scroll")
                .show(ui, |ui| {
                    if let Some(ref ebook) = file.ebook {
                        // Progress indicator
                        let progress = if !ebook.chapters.is_empty() {
                            (self.current_chapter + 1) as f32 / ebook.chapters.len() as f32
                        } else {
                            0.0
                        };
                        
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Progress:").size(11.0).color(Color32::GRAY));
                            ui.add(egui::ProgressBar::new(progress)
                                .desired_width(120.0)
                                .fill(accent));
                            ui.label(RichText::new(format!("{:.0}%", progress * 100.0))
                                .size(11.0).color(Color32::GRAY));
                        });
                        
                        ui.add_space(12.0);
                        
                        // TOC entries
                        for (idx, toc_entry) in ebook.toc.iter().enumerate() {
                            let is_current = idx == self.current_chapter;
                            let label_text = if toc_entry.title.len() > 30 {
                                format!("{}...", &toc_entry.title[..30])
                            } else {
                                toc_entry.title.clone()
                            };
                            
                            let response = ui.selectable_label(
                                is_current,
                                RichText::new(&label_text)
                                    .size(13.0)
                                    .color(if is_current { accent } else { Color32::from_gray(180) })
                            );
                            
                            if response.clicked() {
                                self.current_chapter = idx.min(ebook.chapters.len().saturating_sub(1));
                                self.scroll_position = 0.0;
                            }
                        }
                        
                        // If no TOC, show chapter numbers
                        if ebook.toc.is_empty() {
                            for idx in 0..ebook.chapters.len() {
                                let is_current = idx == self.current_chapter;
                                let label = format!("Chapter {}", idx + 1);
                                
                                if ui.selectable_label(is_current, &label).clicked() {
                                    self.current_chapter = idx;
                                    self.scroll_position = 0.0;
                                }
                            }
                        }
                    }
                });
            
            // Bottom controls
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.add_space(8.0);
                
                // Theme selector
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Theme:").size(11.0).color(Color32::GRAY));
                    if ui.selectable_label(self.theme == ReadingTheme::Light, "☀️").clicked() {
                        self.theme = ReadingTheme::Light;
                    }
                    if ui.selectable_label(self.theme == ReadingTheme::Sepia, "📜").clicked() {
                        self.theme = ReadingTheme::Sepia;
                    }
                    if ui.selectable_label(self.theme == ReadingTheme::Dark, "🌙").clicked() {
                        self.theme = ReadingTheme::Dark;
                    }
                });
                
                ui.add_space(4.0);
                
                // Font size
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Size:").size(11.0).color(Color32::GRAY));
                    if ui.small_button("A-").clicked() {
                        self.font_size = (self.font_size - 1.0).max(10.0);
                    }
                    ui.label(format!("{:.0}", self.font_size));
                    if ui.small_button("A+").clicked() {
                        self.font_size = (self.font_size + 1.0).min(32.0);
                    }
                });
                
                ui.separator();
            });
        });
    }
    
    fn render_content(&mut self, ui: &mut egui::Ui, file: &OpenFile, text_color: Color32, accent: Color32) {
        // Show TOC button if hidden
        if !self.show_toc {
            ui.horizontal(|ui| {
                if ui.button("☰ Contents").clicked() {
                    self.show_toc = true;
                }
                ui.separator();
            });
            ui.add_space(8.0);
        }
        
        // Reading toolbar
        ui.horizontal(|ui| {
            // Navigation
            if let Some(ref ebook) = file.ebook {
                let can_prev = self.current_chapter > 0;
                let can_next = self.current_chapter < ebook.chapters.len().saturating_sub(1);
                
                if ui.add_enabled(can_prev, egui::Button::new("◀ Previous")).clicked() {
                    self.current_chapter -= 1;
                    self.scroll_position = 0.0;
                }
                
                ui.label(format!("{} / {}", 
                    self.current_chapter + 1,
                    ebook.chapters.len()
                ));
                
                if ui.add_enabled(can_next, egui::Button::new("Next ▶")).clicked() {
                    self.current_chapter += 1;
                    self.scroll_position = 0.0;
                }
            }
            
            ui.separator();
            
            // Bookmark
            if ui.button("🔖 Bookmark").on_hover_text("Add bookmark").clicked() {
                self.bookmarks.push((self.current_chapter, String::new()));
            }
            
            // Search
            ui.separator();
            ui.label("🔍");
            let search_resp = ui.add(egui::TextEdit::singleline(&mut self.search_query)
                .desired_width(150.0)
                .hint_text("Search..."));
        });
        
        ui.add_space(12.0);
        ui.separator();
        ui.add_space(16.0);
        
        // Chapter content
        egui::ScrollArea::vertical()
            .id_source("chapter_content")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if let Some(ref ebook) = file.ebook {
                    if let Some(chapter) = ebook.chapters.get(self.current_chapter) {
                        // Chapter title
                        if let Some(ref title) = chapter.title {
                            ui.label(RichText::new(title)
                                .size(self.font_size + 8.0)
                                .strong()
                                .color(text_color));
                            ui.add_space(16.0);
                        }
                        
                        // Render chapter content (simplified HTML to text)
                        let content = self.html_to_text(&chapter.content);
                        
                        // Split into paragraphs and render
                        for paragraph in content.split("\n\n") {
                            let trimmed = paragraph.trim();
                            if !trimmed.is_empty() {
                                ui.label(RichText::new(trimmed)
                                    .size(self.font_size)
                                    .color(text_color));
                                ui.add_space(12.0);
                            }
                        }
                        
                        // End of chapter marker
                        ui.add_space(40.0);
                        ui.centered_and_justified(|ui| {
                            ui.label(RichText::new("—  ✦  —")
                                .size(14.0)
                                .color(Color32::GRAY));
                        });
                        
                        // Next chapter prompt
                        if self.current_chapter < ebook.chapters.len() - 1 {
                            ui.add_space(20.0);
                            ui.centered_and_justified(|ui| {
                                if ui.button(RichText::new("Continue to next chapter →")
                                    .color(accent))
                                    .clicked()
                                {
                                    self.current_chapter += 1;
                                    self.scroll_position = 0.0;
                                }
                            });
                        }
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.label(RichText::new("No chapter content available")
                                .italics()
                                .color(Color32::GRAY));
                        });
                    }
                } else {
                    // No ebook data
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(60.0);
                            ui.label(RichText::new("📚").size(64.0));
                            ui.add_space(16.0);
                            ui.label(RichText::new("eBook")
                                .size(24.0)
                                .color(text_color));
                            ui.add_space(8.0);
                            ui.label(RichText::new(&file.name)
                                .monospace()
                                .color(Color32::GRAY));
                            ui.add_space(16.0);
                            ui.label(RichText::new("Unable to parse ebook content")
                                .color(Color32::from_rgb(200, 100, 100)));
                        });
                    });
                }
            });
    }
    
    /// Simple HTML to text converter
    fn html_to_text(&self, html: &str) -> String {
        let mut result = String::new();
        let mut in_tag = false;
        let mut current_tag = String::new();
        
        for ch in html.chars() {
            match ch {
                '<' => {
                    in_tag = true;
                    current_tag.clear();
                }
                '>' => {
                    in_tag = false;
                    let tag_lower = current_tag.to_lowercase();
                    
                    // Handle block-level tags with newlines
                    if tag_lower.starts_with("p") || tag_lower.starts_with("/p") ||
                       tag_lower.starts_with("br") ||
                       tag_lower.starts_with("div") || tag_lower.starts_with("/div") ||
                       tag_lower.starts_with("h") || tag_lower.starts_with("/h")
                    {
                        result.push_str("\n\n");
                    }
                }
                _ => {
                    if in_tag {
                        current_tag.push(ch);
                    } else {
                        // Decode common HTML entities
                        result.push(ch);
                    }
                }
            }
        }
        
        // Decode HTML entities
        result
            .replace("&nbsp;", " ")
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&mdash;", "—")
            .replace("&ndash;", "–")
            .replace("&hellip;", "…")
            .replace("&rsquo;", "'")
            .replace("&lsquo;", "'")
            .replace("&rdquo;", "\u{201D}")
            .replace("&ldquo;", "\u{201C}")
    }
}
