#![allow(dead_code, unused_imports, unused_variables, deprecated)]
//! Font Viewer - TTF, OTF, WOFF preview with character map

use crate::file_handler::{FileContent, FontContent, OpenFile};
use eframe::egui::{self, Color32, FontId, RichText, Vec2};

pub struct FontViewer {
    preview_text: String,
    preview_sizes: Vec<f32>,
    selected_size: usize,
    show_charset: bool,
    charset_start: u32,
    custom_text_mode: bool,
}

impl FontViewer {
    pub fn new() -> Self {
        Self {
            preview_text: "The quick brown fox jumps over the lazy dog.\nABCDEFGHIJKLMNOPQRSTUVWXYZ\nabcdefghijklmnopqrstuvwxyz\n0123456789 !@#$%^&*()".into(),
            preview_sizes: vec![12.0, 14.0, 16.0, 18.0, 20.0, 24.0, 28.0, 32.0, 36.0, 48.0, 64.0, 72.0, 96.0],
            selected_size: 4,
            show_charset: false,
            charset_start: 0x20,
            custom_text_mode: false,
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui, file: &OpenFile, zoom: f32) {
        if let FileContent::Font(font) = &file.content {
            self.render_toolbar(ui, font);
            ui.separator();
            
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    self.render_font_info(ui, font);
                    ui.separator();
                    self.render_preview(ui, font, zoom);
                    
                    if self.show_charset {
                        ui.separator();
                        self.render_character_map(ui, font, zoom);
                    }
                });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Not a font file");
            });
        }
    }
    
    fn render_toolbar(&mut self, ui: &mut egui::Ui, font: &FontContent) {
        ui.horizontal(|ui| {
            // View mode
            if ui.selectable_label(!self.show_charset, "ðŸ“ Preview").clicked() {
                self.show_charset = false;
            }
            if ui.selectable_label(self.show_charset, "ðŸ”¤ Character Map").clicked() {
                self.show_charset = true;
            }
            
            ui.separator();
            
            // Preview size
            ui.label("Size:");
            egui::ComboBox::from_id_salt("font_size")
                .selected_text(format!("{}pt", self.preview_sizes[self.selected_size] as u32))
                .show_ui(ui, |ui| {
                    for (idx, size) in self.preview_sizes.iter().enumerate() {
                        if ui.selectable_label(idx == self.selected_size, format!("{}pt", *size as u32)).clicked() {
                            self.selected_size = idx;
                        }
                    }
                });
            
            ui.separator();
            
            ui.checkbox(&mut self.custom_text_mode, "Custom text");
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Weight and style badges
                if font.is_italic {
                    ui.label(RichText::new("ð¼").color(Color32::from_rgb(100, 200, 255)));
                }
                if font.weight >= 700 {
                    ui.label(RichText::new("B").strong().color(Color32::from_rgb(255, 200, 100)));
                }
                if font.is_monospace {
                    ui.label(RichText::new("âŒ¨").color(Color32::from_rgb(150, 255, 150)));
                }
                if font.is_variable {
                    ui.label(RichText::new("VAR").small().color(Color32::from_rgb(255, 150, 255)));
                }
            });
        });
    }
    
    fn render_font_info(&mut self, ui: &mut egui::Ui, font: &FontContent) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.heading(RichText::new(&font.family_name).size(24.0));
                ui.label(RichText::new(&font.subfamily).color(Color32::GRAY));
            });
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Full name:");
                        ui.label(&font.full_name);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Version:");
                        ui.label(&font.version);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Glyphs:");
                        ui.label(format!("{}", font.glyph_count));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Weight:");
                        ui.label(format!("{}", font.weight));
                    });
                });
            });
        });
    }
    
    fn render_preview(&mut self, ui: &mut egui::Ui, font: &FontContent, zoom: f32) {
        ui.heading("Preview");
        ui.add_space(10.0);
        
        if self.custom_text_mode {
            let size = self.preview_sizes[self.selected_size] * zoom;
            
            ui.add(
                egui::TextEdit::multiline(&mut self.preview_text)
                    .font(FontId::proportional(size))
                    .desired_width(f32::INFINITY)
                    .desired_rows(5)
            );
        } else {
            // Show at multiple sizes
            let pangram = "The quick brown fox jumps over the lazy dog";
            let uppercase = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
            let lowercase = "abcdefghijklmnopqrstuvwxyz";
            let numbers = "0123456789";
            let symbols = "!@#$%^&*()-=_+[]{}|;':\",./<>?";
            
            // Large preview
            let large_size = 48.0 * zoom;
            ui.label(RichText::new(&font.family_name).size(large_size));
            ui.add_space(10.0);
            
            // Pangram at various sizes
            for size in &[12.0, 16.0, 20.0, 24.0, 32.0] {
                let scaled = size * zoom;
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{}pt", *size as u32)).small().color(Color32::GRAY));
                    ui.label(RichText::new(pangram).size(scaled));
                });
            }
            
            ui.add_space(20.0);
            
            // Character sets
            let display_size = self.preview_sizes[self.selected_size] * zoom;
            
            ui.label(RichText::new("Uppercase").small().color(Color32::GRAY));
            ui.label(RichText::new(uppercase).size(display_size));
            
            ui.add_space(10.0);
            
            ui.label(RichText::new("Lowercase").small().color(Color32::GRAY));
            ui.label(RichText::new(lowercase).size(display_size));
            
            ui.add_space(10.0);
            
            ui.label(RichText::new("Numbers").small().color(Color32::GRAY));
            ui.label(RichText::new(numbers).size(display_size));
            
            ui.add_space(10.0);
            
            ui.label(RichText::new("Symbols").small().color(Color32::GRAY));
            ui.label(RichText::new(symbols).size(display_size));
        }
    }
    
    fn render_character_map(&mut self, ui: &mut egui::Ui, font: &FontContent, zoom: f32) {
        ui.heading("Character Map");
        ui.add_space(10.0);
        
        ui.horizontal(|ui| {
            ui.label("Range:");
            
            if ui.button("Basic Latin").clicked() {
                self.charset_start = 0x20;
            }
            if ui.button("Latin Extended").clicked() {
                self.charset_start = 0x100;
            }
            if ui.button("Greek").clicked() {
                self.charset_start = 0x370;
            }
            if ui.button("Cyrillic").clicked() {
                self.charset_start = 0x400;
            }
            if ui.button("Symbols").clicked() {
                self.charset_start = 0x2000;
            }
            if ui.button("Emoji").clicked() {
                self.charset_start = 0x1F300;
            }
            
            if ui.button("â—€").clicked() {
                self.charset_start = self.charset_start.saturating_sub(256);
            }
            if ui.button("â–¶").clicked() {
                self.charset_start = self.charset_start.saturating_add(256);
            }
            
            ui.label(format!("U+{:04X} - U+{:04X}", self.charset_start, self.charset_start + 255));
        });
        
        ui.add_space(10.0);
        
        let cell_size = 32.0 * zoom;
        let font_size = 18.0 * zoom;
        
        egui::Grid::new("charset_grid")
            .num_columns(16)
            .spacing([2.0, 2.0])
            .show(ui, |ui| {
                for row in 0..16 {
                    for col in 0..16 {
                        let codepoint = self.charset_start + (row * 16 + col);
                        
                        let char_opt = char::from_u32(codepoint);
                        let display = char_opt
                            .filter(|c| !c.is_control())
                            .map(|c| c.to_string())
                            .unwrap_or_else(|| "Â·".into());
                        
                        let response = ui.add_sized(
                            [cell_size, cell_size],
                            egui::Button::new(RichText::new(&display).size(font_size))
                        );
                        
                        response.on_hover_ui(|ui| {
                            ui.label(format!("U+{:04X}", codepoint));
                            if let Some(c) = char_opt {
                                ui.label(format!("'{}'", c));
                            }
                        });
                    }
                    ui.end_row();
                }
            });
    }
}
