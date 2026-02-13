#![allow(dead_code, unused_imports, unused_variables, deprecated)]
//! Text/Code Viewer - Syntax highlighting, line numbers, editing

use crate::file_handler::{FileContent, OpenFile};
use eframe::egui::{self, Color32, FontId, RichText, TextEdit, Vec2};
use syntect::highlighting::{ThemeSet, Style};
use syntect::parsing::SyntaxSet;
use syntect::easy::HighlightLines;

pub struct TextViewer {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    current_theme: String,
    show_line_numbers: bool,
    word_wrap: bool,
    font_size: f32,
    cursor_line: usize,
    cursor_column: usize,
    edit_buffer: String,
    is_editing: bool,
}

impl TextViewer {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            current_theme: "base16-ocean.dark".into(),
            show_line_numbers: true,
            word_wrap: true,
            font_size: 13.0,
            cursor_line: 1,
            cursor_column: 1,
            edit_buffer: String::new(),
            is_editing: false,
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui, file: &OpenFile, zoom: f32) {
        // Toolbar
        ui.horizontal(|ui| {
            if ui.selectable_label(!self.is_editing, "View").clicked() {
                self.is_editing = false;
            }
            if ui.selectable_label(self.is_editing, " Edit").clicked() {
                self.is_editing = true;
                if let FileContent::Text { content, .. } = &file.content {
                    self.edit_buffer = content.clone();
                }
            }
            
            ui.separator();
            
            ui.checkbox(&mut self.show_line_numbers, "123");
            ui.checkbox(&mut self.word_wrap, "<- Wrap");
            
            ui.separator();
            
            // Theme selector
            ui.label("Theme:");
            egui::ComboBox::from_id_salt("theme_selector")
                .selected_text(&self.current_theme)
                .show_ui(ui, |ui| {
                    for theme_name in self.theme_set.themes.keys() {
                        if ui.selectable_label(
                            self.current_theme == *theme_name,
                            theme_name,
                        ).clicked() {
                            self.current_theme = theme_name.clone();
                        }
                    }
                });
            
            ui.separator();
            
            // Font size
            if ui.button("Sum").clicked() {
                self.font_size = (self.font_size - 1.0).max(8.0);
            }
            ui.label(format!("{:.0}px", self.font_size));
            if ui.button("+").clicked() {
                self.font_size = (self.font_size + 1.0).min(32.0);
            }
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Show syntax type
                if let FileContent::Text { syntax: Some(syn), .. } = &file.content {
                    ui.label(format!("{}", syn));
                }
                
                ui.label(format!("Ln {}, Col {}", self.cursor_line, self.cursor_column));
            });
        });
        
        ui.separator();
        
        // Main content
        let scaled_font = self.font_size * zoom;
        
        if self.is_editing {
            self.render_editor(ui, file, scaled_font);
        } else {
            self.render_highlighted(ui, file, scaled_font);
        }
    }
    
    fn render_editor(&mut self, ui: &mut egui::Ui, file: &OpenFile, font_size: f32) {
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let response = TextEdit::multiline(&mut self.edit_buffer)
                    .font(FontId::monospace(font_size))
                    .desired_width(f32::INFINITY)
                    .code_editor()
                    .show(ui);
                
                // Update cursor position
                if let Some(cursor) = response.cursor_range {
                    let text_before = &self.edit_buffer[..cursor.primary.ccursor.index];
                    self.cursor_line = text_before.lines().count().max(1);
                    self.cursor_column = text_before.lines().last()
                        .map(|l| l.len() + 1)
                        .unwrap_or(1);
                }
            });
    }
    
    fn render_highlighted(&mut self, ui: &mut egui::Ui, file: &OpenFile, font_size: f32) {
        if let FileContent::Text { content, syntax, .. } = &file.content {
            let lines: Vec<&str> = content.lines().collect();
            let line_count = lines.len();
            let line_number_width = format!("{}", line_count).len();
            
            // Get syntax definition
            let syntax_def = syntax.as_ref()
                .and_then(|s| self.syntax_set.find_syntax_by_name(s))
                .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
            
            let theme = self.theme_set.themes.get(&self.current_theme)
                .unwrap_or_else(|| self.theme_set.themes.values().next().unwrap());
            
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Line numbers column
                        if self.show_line_numbers {
                            ui.vertical(|ui| {
                                for line_num in 1..=line_count {
                                    ui.label(
                                        RichText::new(format!("{:>width$}", line_num, width = line_number_width))
                                            .font(FontId::monospace(font_size))
                                            .color(Color32::GRAY)
                                    );
                                }
                            });
                            
                            ui.separator();
                        }
                        
                        // Code content with syntax highlighting
                        ui.vertical(|ui| {
                            let mut highlighter = HighlightLines::new(syntax_def, theme);
                            
                            for line in &lines {
                                if let Ok(highlighted) = highlighter.highlight_line(line, &self.syntax_set) {
                                    self.render_highlighted_line(ui, &highlighted, font_size);
                                } else {
                                    ui.label(
                                        RichText::new(*line)
                                            .font(FontId::monospace(font_size))
                                    );
                                }
                            }
                        });
                    });
                });
        }
    }
    
    fn render_highlighted_line(
        &self,
        ui: &mut egui::Ui,
        highlighted: &[(Style, &str)],
        font_size: f32,
    ) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            
            for (style, text) in highlighted {
                let fg = style.foreground;
                let color = Color32::from_rgb(fg.r, fg.g, fg.b);
                
                ui.label(
                    RichText::new(*text)
                        .font(FontId::monospace(font_size))
                        .color(color)
                );
            }
        });
    }
}
