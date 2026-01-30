#![allow(dead_code, unused_imports, unused_variables, deprecated)]
//! Document Editor - Full editing for DOCX, ODT, RTF
//!
//! Features:
//! - View: Zoom, page layout, formatting marks
//! - Edit: Rich text editing, styles, fonts, colors
//! - Format: Bold, italic, underline, alignment, lists
//! - Save: Export to DOCX, ODT, RTF, PDF, TXT, HTML
//! - Print: System print dialog

use crate::file_handler::{DocumentContent, FileContent, OpenFile};
use eframe::egui::{self, Color32, RichText, TextEdit, Vec2, Rect, Pos2, Sense, Stroke};
use std::path::{Path, PathBuf};
use crate::fontcase;

/// Text formatting
#[derive(Debug, Clone, Default)]
pub struct TextFormat {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub font_family: String,
    pub font_size: f32,
    pub color: Color32,
    pub background: Option<Color32>,
    pub alignment: TextAlignment,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TextAlignment {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

/// Paragraph style
#[derive(Debug, Clone)]
pub struct ParagraphStyle {
    pub name: String,
    pub format: TextFormat,
    pub indent_left: f32,
    pub indent_right: f32,
    pub indent_first_line: f32,
    pub spacing_before: f32,
    pub spacing_after: f32,
    pub line_spacing: f32,
    pub list_style: Option<ListStyle>,
}

#[derive(Debug, Clone)]
pub enum ListStyle {
    Bullet,
    Numbered(usize),
    Lettered,
    Roman,
}

/// Document paragraph
#[derive(Debug, Clone)]
pub struct Paragraph {
    pub text: String,
    pub style: String,
    pub format: TextFormat,
    pub list_item: Option<ListStyle>,
}

/// Find/Replace state
#[derive(Default)]
pub struct FindReplace {
    pub find_text: String,
    pub replace_text: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub results: Vec<(usize, usize)>, // (paragraph, char_offset)
    pub current_result: usize,
}

pub struct DocumentViewer {
    // Content
    paragraphs: Vec<Paragraph>,
    styles: Vec<ParagraphStyle>,
    
    // View state
    edit_mode: bool,
    current_paragraph: usize,
    selection_start: Option<(usize, usize)>,
    selection_end: Option<(usize, usize)>,
    scroll_offset: f32,
    
    // Formatting
    current_format: TextFormat,
    font_size: f32,
    show_formatting: bool,
    page_layout: PageLayout,
    
    // Stats
    word_count: usize,
    char_count: usize,
    page_count: usize,
    
    // Find/Replace
    find_replace: FindReplace,
    show_find_replace: bool,
    
    // Clipboard
    clipboard: Option<Vec<Paragraph>>,
    
    // History (undo/redo)
    history: Vec<Vec<Paragraph>>,
    history_index: usize,
    
    // State
    has_unsaved_changes: bool,
    original_path: Option<PathBuf>,
    
    // UI
    show_styles: bool,
    show_export: bool,
    export_format: DocExportFormat,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PageLayout {
    Continuous,
    PageView,
    TwoPage,
    WebView,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DocExportFormat {
    Docx,
    Odt,
    Rtf,
    Pdf,
    Html,
    Txt,
    Markdown,
}

impl DocumentViewer {
    pub fn new() -> Self {
        Self {
            paragraphs: Vec::new(),
            styles: Self::default_styles(),
            
            edit_mode: true,
            current_paragraph: 0,
            selection_start: None,
            selection_end: None,
            scroll_offset: 0.0,
            
            current_format: TextFormat::default(),
            font_size: 12.0,
            show_formatting: false,
            page_layout: PageLayout::Continuous,
            
            word_count: 0,
            char_count: 0,
            page_count: 1,
            
            find_replace: FindReplace::default(),
            show_find_replace: false,
            
            clipboard: None,
            
            history: Vec::new(),
            history_index: 0,
            
            has_unsaved_changes: false,
            original_path: None,
            
            show_styles: false,
            show_export: false,
            export_format: DocExportFormat::Docx,
        }
    }
    
    fn default_styles() -> Vec<ParagraphStyle> {
        vec![
            ParagraphStyle {
                name: "Normal".to_string(),
                format: TextFormat {
                    font_size: 12.0,
                    font_family: "Arial".to_string(),
                    color: Color32::BLACK,
                    ..Default::default()
                },
                indent_left: 0.0,
                indent_right: 0.0,
                indent_first_line: 0.0,
                spacing_before: 0.0,
                spacing_after: 8.0,
                line_spacing: 1.15,
                list_style: None,
            },
            ParagraphStyle {
                name: "Heading 1".to_string(),
                format: TextFormat {
                    font_size: 24.0,
                    font_family: "Arial".to_string(),
                    bold: true,
                    color: Color32::from_rgb(0, 51, 102),
                    ..Default::default()
                },
                indent_left: 0.0,
                indent_right: 0.0,
                indent_first_line: 0.0,
                spacing_before: 24.0,
                spacing_after: 12.0,
                line_spacing: 1.0,
                list_style: None,
            },
            ParagraphStyle {
                name: "Heading 2".to_string(),
                format: TextFormat {
                    font_size: 18.0,
                    font_family: "Arial".to_string(),
                    bold: true,
                    color: Color32::from_rgb(0, 51, 102),
                    ..Default::default()
                },
                indent_left: 0.0,
                indent_right: 0.0,
                indent_first_line: 0.0,
                spacing_before: 18.0,
                spacing_after: 8.0,
                line_spacing: 1.0,
                list_style: None,
            },
            ParagraphStyle {
                name: "Quote".to_string(),
                format: TextFormat {
                    font_size: 12.0,
                    font_family: "Georgia".to_string(),
                    italic: true,
                    color: Color32::from_rgb(80, 80, 80),
                    ..Default::default()
                },
                indent_left: 40.0,
                indent_right: 40.0,
                indent_first_line: 0.0,
                spacing_before: 12.0,
                spacing_after: 12.0,
                line_spacing: 1.15,
                list_style: None,
            },
        ]
    }
    
    /// Load document from content
    pub fn load(&mut self, content: &DocumentContent, path: &std::path::Path) {
        self.paragraphs.clear();
        
        for para in &content.paragraphs {
            self.paragraphs.push(Paragraph {
                text: para.text.clone(),
                style: "Normal".to_string(),
                format: TextFormat::default(),
                list_item: None,
            });
        }
        
        self.original_path = Some(path.to_path_buf());
        self.update_stats();
        self.push_history();
        self.has_unsaved_changes = false;
    }
    
    /// Update word/char counts
    fn update_stats(&mut self) {
        self.char_count = self.paragraphs.iter().map(|p| p.text.len()).sum();
        self.word_count = self.paragraphs.iter()
            .flat_map(|p| p.text.split_whitespace())
            .count();
    }
    
    /// Push current state to history
    fn push_history(&mut self) {
        self.history.truncate(self.history_index + 1);
        self.history.push(self.paragraphs.clone());
        if self.history.len() > 100 {
            self.history.remove(0);
        } else {
            self.history_index = self.history.len() - 1;
        }
    }
    
    /// Undo
    pub fn undo(&mut self) {
        if self.history_index > 0 {
            self.history_index -= 1;
            self.paragraphs = self.history[self.history_index].clone();
            self.update_stats();
        }
    }
    
    /// Redo
    pub fn redo(&mut self) {
        if self.history_index < self.history.len() - 1 {
            self.history_index += 1;
            self.paragraphs = self.history[self.history_index].clone();
            self.update_stats();
        }
    }

    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
    // FORMATTING
    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
    
    /// Toggle bold on selection
    pub fn toggle_bold(&mut self) {
        self.current_format.bold = !self.current_format.bold;
        self.apply_format_to_selection();
    }
    
    /// Toggle italic
    pub fn toggle_italic(&mut self) {
        self.current_format.italic = !self.current_format.italic;
        self.apply_format_to_selection();
    }
    
    /// Toggle underline
    pub fn toggle_underline(&mut self) {
        self.current_format.underline = !self.current_format.underline;
        self.apply_format_to_selection();
    }
    
    /// Set alignment
    pub fn set_alignment(&mut self, align: TextAlignment) {
        self.current_format.alignment = align;
        if self.current_paragraph < self.paragraphs.len() {
            self.paragraphs[self.current_paragraph].format.alignment = align;
            self.has_unsaved_changes = true;
        }
    }
    
    /// Apply current format to selection (simplified - applies to current paragraph)
    fn apply_format_to_selection(&mut self) {
        if self.current_paragraph < self.paragraphs.len() {
            self.paragraphs[self.current_paragraph].format = self.current_format.clone();
            self.has_unsaved_changes = true;
        }
    }
    
    /// Apply style to paragraph
    pub fn apply_style(&mut self, style_name: &str) {
        if let Some(style) = self.styles.iter().find(|s| s.name == style_name) {
            if self.current_paragraph < self.paragraphs.len() {
                self.paragraphs[self.current_paragraph].style = style_name.to_string();
                self.paragraphs[self.current_paragraph].format = style.format.clone();
                self.has_unsaved_changes = true;
            }
        }
    }
    
    /// Insert new paragraph
    pub fn insert_paragraph(&mut self, after: usize, text: String) {
        let para = Paragraph {
            text,
            style: "Normal".to_string(),
            format: self.current_format.clone(),
            list_item: None,
        };
        if after < self.paragraphs.len() {
            self.paragraphs.insert(after + 1, para);
        } else {
            self.paragraphs.push(para);
        }
        self.push_history();
        self.update_stats();
        self.has_unsaved_changes = true;
    }
    
    /// Delete paragraph
    pub fn delete_paragraph(&mut self, index: usize) {
        if index < self.paragraphs.len() && self.paragraphs.len() > 1 {
            self.paragraphs.remove(index);
            if self.current_paragraph >= self.paragraphs.len() {
                self.current_paragraph = self.paragraphs.len() - 1;
            }
            self.push_history();
            self.update_stats();
            self.has_unsaved_changes = true;
        }
    }
    
    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
    // FIND / REPLACE
    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
    
    /// Find text in document
    pub fn find(&mut self, text: &str) {
        self.find_replace.find_text = text.to_string();
        self.find_replace.results.clear();
        
        let search = if self.find_replace.case_sensitive {
            text.to_string()
        } else {
            crate::fontcase::ascii_lower(text)
        };
        
        for (para_idx, para) in self.paragraphs.iter().enumerate() {
            let para_text = if self.find_replace.case_sensitive {
                para.text.clone()
            } else {
                crate::fontcase::ascii_lower(&para.text)
            };
            
            let mut start = 0;
            while let Some(pos) = para_text[start..].find(&search) {
                self.find_replace.results.push((para_idx, start + pos));
                start += pos + 1;
            }
        }
        
        self.find_replace.current_result = 0;
    }
    
    /// Replace current occurrence
    pub fn replace_current(&mut self) {
        if let Some(&(para_idx, char_idx)) = self.find_replace.results.get(self.find_replace.current_result) {
            let find_len = self.find_replace.find_text.len();
            if para_idx < self.paragraphs.len() {
                let text = &mut self.paragraphs[para_idx].text;
                text.replace_range(char_idx..char_idx + find_len, &self.find_replace.replace_text);
                self.push_history();
                self.has_unsaved_changes = true;
                // Re-run find to update results
                self.find(&self.find_replace.find_text.clone());
            }
        }
    }
    
    /// Replace all occurrences
    pub fn replace_all(&mut self) {
        let find = self.find_replace.find_text.clone();
        let replace = self.find_replace.replace_text.clone();
        
        for para in &mut self.paragraphs {
            if self.find_replace.case_sensitive {
                para.text = para.text.replace(&find, &replace);
            } else {
                // Case-insensitive replace is more complex
                let lower_find = crate::fontcase::ascii_lower(&find);
                let mut result = String::new();
                let mut last_end = 0;
                for (start, _) in crate::fontcase::ascii_lower(&para.text).match_indices(&lower_find) {
                    result.push_str(&para.text[last_end..start]);
                    result.push_str(&replace);
                    last_end = start + find.len();
                }
                result.push_str(&para.text[last_end..]);
                para.text = result;
            }
        }
        
        self.push_history();
        self.update_stats();
        self.has_unsaved_changes = true;
        self.find_replace.results.clear();
    }
    
    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
    // SAVE / EXPORT
    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
    
    /// Save to original format
    pub fn save(&self) -> Result<(), String> {
        if let Some(path) = &self.original_path {
            self.save_as(path, self.detect_format(path))
        } else {
            Err("No original path".to_string())
        }
    }
    
    /// Save as specific format
    pub fn save_as(&self, path: &Path, format: DocExportFormat) -> Result<(), String> {
        match format {
            DocExportFormat::Docx => self.save_docx(path),
            DocExportFormat::Odt => self.save_odt(path),
            DocExportFormat::Rtf => self.save_rtf(path),
            DocExportFormat::Html => self.save_html(path),
            DocExportFormat::Txt => self.save_txt(path),
            DocExportFormat::Markdown => self.save_markdown(path),
            DocExportFormat::Pdf => self.save_pdf(path),
        }
    }
    
    fn detect_format(&self, path: &std::path::Path) -> DocExportFormat {
        match path.extension().and_then(|e| e.to_str()) {
            Some("docx") => DocExportFormat::Docx,
            Some("odt") => DocExportFormat::Odt,
            Some("rtf") => DocExportFormat::Rtf,
            Some("html") | Some("htm") => DocExportFormat::Html,
            Some("md") => DocExportFormat::Markdown,
            Some("pdf") => DocExportFormat::Pdf,
            _ => DocExportFormat::Txt,
        }
    }
    
    fn save_docx(&self, path: &Path) -> Result<(), String> {
        use std::io::Write;
        use zip::ZipWriter;
        use zip::write::SimpleFileOptions;
        
        let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        
        // [Content_Types].xml
        zip.start_file("[Content_Types].xml", options).map_err(|e| e.to_string())?;
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#).map_err(|e| e.to_string())?;
        
        // _rels/.rels
        zip.start_file("_rels/.rels", options).map_err(|e| e.to_string())?;
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#).map_err(|e| e.to_string())?;
        
        // word/document.xml
        zip.start_file("word/document.xml", options).map_err(|e| e.to_string())?;
        let mut doc_xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>"#);
        
        for para in &self.paragraphs {
            doc_xml.push_str("<w:p>");
            doc_xml.push_str("<w:pPr>");
            match para.format.alignment {
                TextAlignment::Center => doc_xml.push_str("<w:jc w:val=\"center\"/>"),
                TextAlignment::Right => doc_xml.push_str("<w:jc w:val=\"right\"/>"),
                TextAlignment::Justify => doc_xml.push_str("<w:jc w:val=\"both\"/>"),
                _ => {}
            }
            doc_xml.push_str("</w:pPr>");
            doc_xml.push_str("<w:r>");
            doc_xml.push_str("<w:rPr>");
            if para.format.bold { doc_xml.push_str("<w:b/>"); }
            if para.format.italic { doc_xml.push_str("<w:i/>"); }
            if para.format.underline { doc_xml.push_str("<w:u w:val=\"single\"/>"); }
            doc_xml.push_str("</w:rPr>");
            doc_xml.push_str("<w:t>");
            doc_xml.push_str(&para.text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;"));
            doc_xml.push_str("</w:t>");
            doc_xml.push_str("</w:r>");
            doc_xml.push_str("</w:p>");
        }
        
        doc_xml.push_str("</w:body></w:document>");
        zip.write_all(doc_xml.as_bytes()).map_err(|e| e.to_string())?;
        
        zip.finish().map_err(|e| e.to_string())?;
        Ok(())
    }
    
    fn save_odt(&self, path: &Path) -> Result<(), String> {
        // Similar structure to DOCX but ODF format
        // Simplified: just save as plain text for now
        self.save_txt(path)
    }
    
    fn save_rtf(&self, path: &Path) -> Result<(), String> {
        let mut rtf = String::from("{\\rtf1\\ansi\\deff0\n");
        
        for para in &self.paragraphs {
            if para.format.bold { rtf.push_str("\\b "); }
            if para.format.italic { rtf.push_str("\\i "); }
            if para.format.underline { rtf.push_str("\\ul "); }
            
            rtf.push_str(&para.text);
            
            if para.format.underline { rtf.push_str("\\ul0 "); }
            if para.format.italic { rtf.push_str("\\i0 "); }
            if para.format.bold { rtf.push_str("\\b0 "); }
            
            rtf.push_str("\\par\n");
        }
        
        rtf.push('}');
        std::fs::write(path, rtf).map_err(|e| e.to_string())
    }
    
    fn save_html(&self, path: &Path) -> Result<(), String> {
        let mut html = String::from("<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\"><title>Document</title></head><body>\n");
        
        for para in &self.paragraphs {
            let tag = if para.style == "Heading 1" { "h1" } 
                     else if para.style == "Heading 2" { "h2" }
                     else { "p" };
            
            html.push_str(&format!("<{}>", tag));
            
            let mut text = para.text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
            if para.format.bold { text = format!("<strong>{}</strong>", text); }
            if para.format.italic { text = format!("<em>{}</em>", text); }
            if para.format.underline { text = format!("<u>{}</u>", text); }
            
            html.push_str(&text);
            html.push_str(&format!("</{}>\n", tag));
        }
        
        html.push_str("</body></html>");
        std::fs::write(path, html).map_err(|e| e.to_string())
    }
    
    fn save_txt(&self, path: &Path) -> Result<(), String> {
        let text: String = self.paragraphs.iter()
            .map(|p| p.text.clone())
            .collect::<Vec<_>>()
            .join("\n\n");
        std::fs::write(path, text).map_err(|e| e.to_string())
    }
    
    fn save_markdown(&self, path: &Path) -> Result<(), String> {
        let mut md = String::new();
        
        for para in &self.paragraphs {
            if para.style == "Heading 1" {
                md.push_str(&format!("# {}\n\n", para.text));
            } else if para.style == "Heading 2" {
                md.push_str(&format!("## {}\n\n", para.text));
            } else if para.style == "Quote" {
                md.push_str(&format!("> {}\n\n", para.text));
            } else {
                let mut text = para.text.clone();
                if para.format.bold { text = format!("**{}**", text); }
                if para.format.italic { text = format!("*{}*", text); }
                md.push_str(&format!("{}\n\n", text));
            }
        }
        
        std::fs::write(path, md).map_err(|e| e.to_string())
    }
    
    fn save_pdf(&self, _path: &Path) -> Result<(), String> {
        // Would use printpdf crate
        Err("PDF export not yet implemented".to_string())
    }

    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
    // UI RENDERING
    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
    
    pub fn render(&mut self, ui: &mut egui::Ui, file: &OpenFile, zoom: f32) {
        // Load document if empty
        if self.paragraphs.is_empty() {
            if let FileContent::Document(content) = &file.content {
                self.load(content, &file.path);
            }
        }
        
        // Toolbar
        self.render_toolbar(ui);
        ui.separator();
        
        // Main area
        ui.horizontal(|ui| {
            // Styles sidebar
            if self.show_styles {
                egui::SidePanel::left("doc_styles")
                    .resizable(true)
                    .min_width(150.0)
                    .max_width(250.0)
                    .show_inside(ui, |ui| {
                        self.render_styles_panel(ui);
                    });
            }
            
            // Document content
            self.render_document(ui, zoom);
        });
        
        // Find/Replace dialog
        if self.show_find_replace {
            self.render_find_replace(ui);
        }
        
        // Export dialog
        if self.show_export {
            self.render_export_dialog(ui);
        }
    }
    
    fn render_toolbar(&mut self, ui: &mut egui::Ui) {
        // Row 1: File operations
        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                let _ = self.save();
            }
            if ui.button("Export").clicked() {
                self.show_export = true;
            }
            if ui.button("Print").clicked() {
                // TODO: Print
            }
            
            ui.separator();
            
            // Undo/Redo
            ui.add_enabled_ui(self.history_index > 0, |ui| {
                if ui.button("â†©").on_hover_text("Undo").clicked() {
                    self.undo();
                }
            });
            ui.add_enabled_ui(self.history_index < self.history.len().saturating_sub(1), |ui| {
                if ui.button("â†ª").on_hover_text("Redo").clicked() {
                    self.redo();
                }
            });
            
            ui.separator();
            
            // Find/Replace
            if ui.button("").on_hover_text("Find & Replace").clicked() {
                self.show_find_replace = !self.show_find_replace;
            }
            
            ui.separator();
            
            // Styles toggle
            ui.toggle_value(&mut self.show_styles, "Â¶ Styles");
            ui.checkbox(&mut self.show_formatting, "Show Â¶");
            
            // Stats
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("Words: {} | Chars: {}", self.word_count, self.char_count));
                if self.has_unsaved_changes {
                    ui.label("*").on_hover_text("Unsaved changes");
                }
            });
        });
        
        // Row 2: Formatting
        ui.horizontal(|ui| {
            // Style dropdown
            let current_style = if self.current_paragraph < self.paragraphs.len() {
                self.paragraphs[self.current_paragraph].style.clone()
            } else {
                "Normal".to_string()
            };
            egui::ComboBox::from_id_salt("style_select")
                .selected_text(&current_style)
                .show_ui(ui, |ui| {
                    for style in &self.styles.clone() {
                        if ui.selectable_label(current_style == style.name, &style.name).clicked() {
                            self.apply_style(&style.name);
                        }
                    }
                });
            
            ui.separator();
            
            // Font size
            ui.label("Size:");
            if ui.add(egui::DragValue::new(&mut self.font_size).speed(0.5).range(8.0..=72.0)).changed() {
                self.current_format.font_size = self.font_size;
            }
            
            ui.separator();
            
            // Bold, Italic, Underline
            let bold = self.current_format.bold;
            let italic = self.current_format.italic;
            let underline = self.current_format.underline;
            
            if ui.selectable_label(bold, RichText::new("B").strong()).clicked() {
                self.toggle_bold();
            }
            if ui.selectable_label(italic, RichText::new("I").italics()).clicked() {
                self.toggle_italic();
            }
            if ui.selectable_label(underline, RichText::new("U").underline()).clicked() {
                self.toggle_underline();
            }
            
            ui.separator();
            
            // Alignment
            let align = self.current_format.alignment;
            if ui.selectable_label(align == TextAlignment::Left, "â¬…").on_hover_text("Align Left").clicked() {
                self.set_alignment(TextAlignment::Left);
            }
            if ui.selectable_label(align == TextAlignment::Center, "â¬Œ").on_hover_text("Center").clicked() {
                self.set_alignment(TextAlignment::Center);
            }
            if ui.selectable_label(align == TextAlignment::Right, "âž¡").on_hover_text("Align Right").clicked() {
                self.set_alignment(TextAlignment::Right);
            }
            if ui.selectable_label(align == TextAlignment::Justify, "â˜°").on_hover_text("Justify").clicked() {
                self.set_alignment(TextAlignment::Justify);
            }
            
            ui.separator();
            
            // Text color
            ui.label("Color:");
            ui.color_edit_button_srgba(&mut self.current_format.color);
        });
    }
    
    fn render_styles_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Styles");
        ui.separator();
        
        for style in &self.styles.clone() {
            let is_current = self.current_paragraph < self.paragraphs.len() 
                && self.paragraphs[self.current_paragraph].style == style.name;
            
            if ui.selectable_label(is_current, &style.name).clicked() {
                self.apply_style(&style.name);
            }
        }
    }
    
    fn render_document(&mut self, ui: &mut egui::Ui, zoom: f32) {
        let page_width = 612.0 * zoom; // Letter width in points
        
        // Track changes to apply after iteration
        let mut needs_stats_update = false;
        let mut new_current_paragraph = self.current_paragraph;
        let mut new_current_format = self.current_format.clone();
        
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    // Page background
                    let available_width = ui.available_width();
                    let margin = ((available_width - page_width) / 2.0).max(20.0);
                    
                    ui.add_space(20.0);
                    
                    egui::Frame::none()
                        .fill(Color32::WHITE)
                        .stroke(Stroke::new(1.0, Color32::GRAY))
                        .inner_margin(egui::Margin::symmetric(margin.min(72.0), 72.0))
                        .show(ui, |ui| {
                            ui.set_min_width(page_width);
                            
                            // Render paragraphs using index-based iteration
                            let para_count = self.paragraphs.len();
                            for idx in 0..para_count {
                                // Get style format
                                let style_format = {
                                    let para_style = &self.paragraphs[idx].style;
                                    self.styles.iter()
                                        .find(|s| &s.name == para_style)
                                        .map(|s| s.format.clone())
                                        .unwrap_or_default()
                                };
                                
                                let para = &self.paragraphs[idx];
                                
                                // Merge paragraph format with style
                                let font_size = if para.format.font_size > 0.0 { 
                                    para.format.font_size 
                                } else { 
                                    style_format.font_size 
                                } * zoom;
                                
                                let mut text = RichText::new(&para.text)
                                    .size(font_size)
                                    .color(para.format.color);
                                
                                if para.format.bold || style_format.bold { text = text.strong(); }
                                if para.format.italic || style_format.italic { text = text.italics(); }
                                if para.format.underline || style_format.underline { text = text.underline(); }
                                if para.format.strikethrough { text = text.strikethrough(); }
                                
                                // Alignment
                                let layout = match para.format.alignment {
                                    TextAlignment::Center => egui::Layout::top_down(egui::Align::Center),
                                    TextAlignment::Right => egui::Layout::top_down(egui::Align::RIGHT),
                                    _ => egui::Layout::top_down(egui::Align::LEFT),
                                };
                                
                                let para_format = para.format.clone();
                                
                                ui.with_layout(layout, |ui| {
                                    if self.edit_mode {
                                        let response = ui.add(
                                            TextEdit::multiline(&mut self.paragraphs[idx].text)
                                                .font(egui::FontId::proportional(font_size))
                                                .desired_width(page_width - 144.0)
                                                .frame(false)
                                        );
                                        
                                        if response.changed() {
                                            self.has_unsaved_changes = true;
                                            needs_stats_update = true;
                                        }
                                        
                                        if response.has_focus() {
                                            new_current_paragraph = idx;
                                            new_current_format = para_format.clone();
                                        }
                                    } else {
                                        let response = ui.add(
                                            egui::Label::new(text)
                                                .wrap()
                                                .sense(Sense::click())
                                        );
                                        
                                        if response.clicked() {
                                            new_current_paragraph = idx;
                                            new_current_format = para_format.clone();
                                        }
                                    }
                                    
                                    // Show formatting marks
                                    if self.show_formatting {
                                        ui.label(RichText::new("Â¶").color(Color32::LIGHT_GRAY).small());
                                    }
                                });
                                
                                ui.add_space(8.0);
                            }
                        });
                    
                    ui.add_space(20.0);
                });
            });
        
        // Apply tracked changes
        self.current_paragraph = new_current_paragraph;
        self.current_format = new_current_format;
        if needs_stats_update {
            self.update_stats();
        }
    }
    
    fn render_find_replace(&mut self, ui: &mut egui::Ui) {
        egui::Window::new("Find & Replace")
            .collapsible(true)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    ui.label("Find:");
                    let response = ui.text_edit_singleline(&mut self.find_replace.find_text);
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.find(&self.find_replace.find_text.clone());
                    }
                });
                
                ui.horizontal(|ui| {
                    ui.label("Replace:");
                    ui.text_edit_singleline(&mut self.find_replace.replace_text);
                });
                
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.find_replace.case_sensitive, "Case sensitive");
                    ui.checkbox(&mut self.find_replace.whole_word, "Whole word");
                });
                
                ui.horizontal(|ui| {
                    if ui.button("Find").clicked() {
                        self.find(&self.find_replace.find_text.clone());
                    }
                    if ui.button("Replace").clicked() {
                        self.replace_current();
                    }
                    if ui.button("Replace All").clicked() {
                        self.replace_all();
                    }
                });
                
                if !self.find_replace.results.is_empty() {
                    ui.label(format!("{} results found", self.find_replace.results.len()));
                }
            });
    }
    
    fn render_export_dialog(&mut self, ui: &mut egui::Ui) {
        egui::Window::new("Export Document")
            .collapsible(false)
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    ui.label("Format:");
                    egui::ComboBox::from_id_salt("doc_export_format")
                        .selected_text(match self.export_format {
                            DocExportFormat::Docx => "Word (.docx)",
                            DocExportFormat::Odt => "OpenDocument (.odt)",
                            DocExportFormat::Rtf => "Rich Text (.rtf)",
                            DocExportFormat::Html => "HTML (.html)",
                            DocExportFormat::Txt => "Plain Text (.txt)",
                            DocExportFormat::Markdown => "Markdown (.md)",
                            DocExportFormat::Pdf => "PDF (.pdf)",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.export_format, DocExportFormat::Docx, "Word (.docx)");
                            ui.selectable_value(&mut self.export_format, DocExportFormat::Odt, "OpenDocument (.odt)");
                            ui.selectable_value(&mut self.export_format, DocExportFormat::Rtf, "Rich Text (.rtf)");
                            ui.selectable_value(&mut self.export_format, DocExportFormat::Html, "HTML (.html)");
                            ui.selectable_value(&mut self.export_format, DocExportFormat::Txt, "Plain Text (.txt)");
                            ui.selectable_value(&mut self.export_format, DocExportFormat::Markdown, "Markdown (.md)");
                            ui.selectable_value(&mut self.export_format, DocExportFormat::Pdf, "PDF (.pdf)");
                        });
                });
                
                ui.separator();
                
                ui.horizontal(|ui| {
                    if ui.button("Export").clicked() {
                        let ext = match self.export_format {
                            DocExportFormat::Docx => "docx",
                            DocExportFormat::Odt => "odt",
                            DocExportFormat::Rtf => "rtf",
                            DocExportFormat::Html => "html",
                            DocExportFormat::Txt => "txt",
                            DocExportFormat::Markdown => "md",
                            DocExportFormat::Pdf => "pdf",
                        };
                        
                        if let Some(path) = native_dialog::FileDialog::new()
                            .add_filter("Document", &[ext])
                            .show_save_single_file()
                            .ok()
                            .flatten()
                        {
                            let _ = self.save_as(&path, self.export_format);
                            self.has_unsaved_changes = false;
                        }
                        self.show_export = false;
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_export = false;
                    }
                });
            });
    }
}
