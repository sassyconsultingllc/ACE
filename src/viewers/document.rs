//! Document Viewer - DOCX, ODT, RTF viewer and editor

use crate::file_handler::{DocumentContent, FileContent, OpenFile};
use eframe::egui::{self, Color32, RichText, TextEdit, Vec2};

pub struct DocumentViewer {
    edit_mode: bool,
    current_paragraph: usize,
    font_size: f32,
    show_formatting: bool,
    word_count: usize,
    char_count: usize,
}

impl DocumentViewer {
    pub fn new() -> Self {
        Self {
            edit_mode: false,
            current_paragraph: 0,
            font_size: 14.0,
            show_formatting: false,
            word_count: 0,
            char_count: 0,
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui, file: &OpenFile, zoom: f32) {
        // Toolbar
        ui.horizontal(|ui| {
            if ui.selectable_label(!self.edit_mode, "👁 View").clicked() {
                self.edit_mode = false;
            }
            if ui.selectable_label(self.edit_mode, "✏️ Edit").clicked() {
                self.edit_mode = true;
            }
            
            ui.separator();
            
            // Font size
            ui.label("Font:");
            if ui.button("−").clicked() {
                self.font_size = (self.font_size - 1.0).max(8.0);
            }
            ui.label(format!("{:.0}", self.font_size));
            if ui.button("+").clicked() {
                self.font_size = (self.font_size + 1.0).min(48.0);
            }
            
            ui.separator();
            
            ui.checkbox(&mut self.show_formatting, "¶ Show Formatting");
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("Words: {} | Chars: {}", self.word_count, self.char_count));
            });
        });
        
        ui.separator();
        
        // Document content
        if let FileContent::Document(doc) = &file.content {
            self.update_stats(doc);
            
            let scaled_font_size = self.font_size * zoom;
            
            // Create a scrollable document view
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // Document container with margins
                    let available_width = ui.available_width();
                    let doc_width = (available_width * 0.8).min(800.0);
                    let margin = (available_width - doc_width) / 2.0;
                    
                    ui.add_space(20.0);
                    
                    ui.horizontal(|ui| {
                        ui.add_space(margin);
                        
                        ui.vertical(|ui| {
                            ui.set_width(doc_width);
                            
                            // Document background
                            let (rect, _) = ui.allocate_space(Vec2::new(doc_width, 0.0));
                            
                            // Render paragraphs
                            for (idx, paragraph) in doc.paragraphs.iter().enumerate() {
                                if self.edit_mode {
                                    // TODO: Make this editable
                                    ui.label(
                                        RichText::new(paragraph)
                                            .size(scaled_font_size)
                                    );
                                } else {
                                    // View mode
                                    let text = if self.show_formatting {
                                        format!("{}¶", paragraph)
                                    } else {
                                        paragraph.clone()
                                    };
                                    
                                    ui.label(
                                        RichText::new(&text)
                                            .size(scaled_font_size)
                                    );
                                }
                                
                                ui.add_space(scaled_font_size * 0.5);
                            }
                            
                            // Empty document message
                            if doc.paragraphs.is_empty() {
                                ui.label(
                                    RichText::new("(Empty document)")
                                        .size(scaled_font_size)
                                        .italics()
                                        .color(Color32::GRAY)
                                );
                            }
                        });
                        
                        ui.add_space(margin);
                    });
                    
                    ui.add_space(20.0);
                });
        } else {
            // Not a document file - show raw content
            ui.centered_and_justified(|ui| {
                ui.label("Unable to parse document format");
            });
        }
    }
    
    fn update_stats(&mut self, doc: &DocumentContent) {
        self.word_count = doc.paragraphs.iter()
            .flat_map(|p| p.split_whitespace())
            .count();
        
        self.char_count = doc.paragraphs.iter()
            .map(|p| p.chars().count())
            .sum();
    }
}
