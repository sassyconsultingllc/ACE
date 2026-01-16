//! Spreadsheet Viewer - XLSX, XLS, ODS, CSV viewer and editor

use crate::file_handler::{CellValue, FileContent, OpenFile, Sheet, SpreadsheetContent};
use eframe::egui::{self, Color32, FontId, RichText, Rect, Vec2, Stroke, Sense};

pub struct SpreadsheetViewer {
    active_sheet: usize,
    selected_cell: Option<(usize, usize)>,
    selection_range: Option<((usize, usize), (usize, usize))>,
    column_widths: Vec<f32>,
    row_heights: Vec<f32>,
    default_column_width: f32,
    default_row_height: f32,
    formula_bar_text: String,
    edit_mode: bool,
    show_gridlines: bool,
    freeze_rows: usize,
    freeze_cols: usize,
}

impl SpreadsheetViewer {
    pub fn new() -> Self {
        Self {
            active_sheet: 0,
            selected_cell: None,
            selection_range: None,
            column_widths: Vec::new(),
            row_heights: Vec::new(),
            default_column_width: 100.0,
            default_row_height: 24.0,
            formula_bar_text: String::new(),
            edit_mode: false,
            show_gridlines: true,
            freeze_rows: 0,
            freeze_cols: 0,
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui, file: &OpenFile, zoom: f32) {
        if let FileContent::Spreadsheet(spreadsheet) = &file.content {
            // Toolbar
            self.render_toolbar(ui, spreadsheet, zoom);
            
            ui.separator();
            
            // Formula bar
            self.render_formula_bar(ui, spreadsheet);
            
            ui.separator();
            
            // Main grid
            self.render_grid(ui, spreadsheet, zoom);
            
            // Sheet tabs
            self.render_sheet_tabs(ui, spreadsheet);
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Not a spreadsheet file");
            });
        }
    }
    
    fn render_toolbar(&mut self, ui: &mut egui::Ui, _spreadsheet: &SpreadsheetContent, zoom: f32) {
        ui.horizontal(|ui| {
            if ui.selectable_label(!self.edit_mode, "👁 View").clicked() {
                self.edit_mode = false;
            }
            if ui.selectable_label(self.edit_mode, "✏️ Edit").clicked() {
                self.edit_mode = true;
            }
            
            ui.separator();
            
            ui.checkbox(&mut self.show_gridlines, "⊞ Grid");
            
            ui.separator();
            
            // Column width
            ui.label("Col Width:");
            if ui.button("−").clicked() {
                self.default_column_width = (self.default_column_width - 10.0).max(40.0);
            }
            ui.label(format!("{:.0}", self.default_column_width));
            if ui.button("+").clicked() {
                self.default_column_width = (self.default_column_width + 10.0).min(300.0);
            }
            
            ui.separator();
            
            // Freeze panes
            ui.label("Freeze:");
            if ui.button("Row").clicked() {
                if let Some((row, _)) = self.selected_cell {
                    self.freeze_rows = row;
                }
            }
            if ui.button("Col").clicked() {
                if let Some((_, col)) = self.selected_cell {
                    self.freeze_cols = col;
                }
            }
            if ui.button("Clear").clicked() {
                self.freeze_rows = 0;
                self.freeze_cols = 0;
            }
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some((row, col)) = self.selected_cell {
                    ui.label(format!("Cell: {}{}", column_name(col), row + 1));
                }
                ui.label(format!("Zoom: {:.0}%", zoom * 100.0));
            });
        });
    }
    
    fn render_formula_bar(&mut self, ui: &mut egui::Ui, spreadsheet: &SpreadsheetContent) {
        ui.horizontal(|ui| {
            // Cell reference
            let cell_ref = if let Some((row, col)) = self.selected_cell {
                format!("{}{}", column_name(col), row + 1)
            } else {
                "".into()
            };
            
            ui.label(RichText::new(cell_ref).strong().monospace());
            ui.separator();
            
            // Formula/content input
            ui.label("fx");
            
            let response = ui.text_edit_singleline(&mut self.formula_bar_text);
            
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                // TODO: Apply formula to cell
            }
            
            // Update formula bar when cell selected
            if let Some((row, col)) = self.selected_cell {
                if let Some(sheet) = spreadsheet.sheets.get(self.active_sheet) {
                    if let Some(cell_row) = sheet.cells.get(row) {
                        if let Some(cell) = cell_row.get(col) {
                            let text = match cell {
                                CellValue::Empty => String::new(),
                                CellValue::Text(s) => s.clone(),
                                CellValue::Number(n) => n.to_string(),
                                CellValue::Boolean(b) => b.to_string(),
                                CellValue::Formula(f) => format!("={}", f),
                                CellValue::Error(e) => format!("#ERR: {}", e),
                                CellValue::Date(d) => d.clone(),
                                CellValue::Currency(n, sym) => format!("{}{:.2}", sym, n),
                            };
                            if !response.has_focus() {
                                self.formula_bar_text = text;
                            }
                        }
                    }
                }
            }
        });
    }
    
    fn render_grid(&mut self, ui: &mut egui::Ui, spreadsheet: &SpreadsheetContent, zoom: f32) {
        let Some(sheet) = spreadsheet.sheets.get(self.active_sheet) else {
            ui.label("No sheets available");
            return;
        };
        
        let row_count = sheet.cells.len();
        let col_count = sheet.cells.iter().map(|r| r.len()).max().unwrap_or(0);
        
        if row_count == 0 || col_count == 0 {
            ui.centered_and_justified(|ui| {
                ui.label("Empty spreadsheet");
            });
            return;
        }
        
        let col_width = self.default_column_width * zoom;
        let row_height = self.default_row_height * zoom;
        let header_width = 50.0 * zoom;
        
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let (response, painter) = ui.allocate_painter(
                    Vec2::new(
                        header_width + col_count as f32 * col_width + 20.0,
                        row_height + row_count as f32 * row_height + 20.0,
                    ),
                    Sense::click(),
                );
                
                let origin = response.rect.min;
                
                // Draw column headers
                for col in 0..col_count {
                    let x = origin.x + header_width + col as f32 * col_width;
                    let rect = Rect::from_min_size(
                        egui::pos2(x, origin.y),
                        Vec2::new(col_width, row_height),
                    );
                    
                    painter.rect_filled(rect, 0.0, Color32::from_gray(60));
                    painter.rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::from_gray(40)));
                    
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        column_name(col),
                        FontId::proportional(12.0 * zoom),
                        Color32::WHITE,
                    );
                }
                
                // Draw row headers and cells
                for row in 0..row_count {
                    let y = origin.y + row_height + row as f32 * row_height;
                    
                    // Row header
                    let header_rect = Rect::from_min_size(
                        egui::pos2(origin.x, y),
                        Vec2::new(header_width, row_height),
                    );
                    
                    painter.rect_filled(header_rect, 0.0, Color32::from_gray(60));
                    painter.rect_stroke(header_rect, 0.0, Stroke::new(1.0, Color32::from_gray(40)));
                    
                    painter.text(
                        header_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("{}", row + 1),
                        FontId::proportional(12.0 * zoom),
                        Color32::WHITE,
                    );
                    
                    // Cells
                    let cells = sheet.cells.get(row);
                    
                    for col in 0..col_count {
                        let x = origin.x + header_width + col as f32 * col_width;
                        let cell_rect = Rect::from_min_size(
                            egui::pos2(x, y),
                            Vec2::new(col_width, row_height),
                        );
                        
                        // Selection highlight
                        let is_selected = self.selected_cell == Some((row, col));
                        
                        if is_selected {
                            painter.rect_filled(cell_rect, 0.0, Color32::from_rgb(50, 80, 120));
                            painter.rect_stroke(cell_rect, 0.0, Stroke::new(2.0, Color32::from_rgb(100, 150, 200)));
                        } else if self.show_gridlines {
                            painter.rect_stroke(cell_rect, 0.0, Stroke::new(1.0, Color32::from_gray(50)));
                        }
                        
                        // Cell content
                        if let Some(cells) = cells {
                            if let Some(cell) = cells.get(col) {
                                let (text, color) = match cell {
                                    CellValue::Empty => (String::new(), Color32::WHITE),
                                    CellValue::Text(s) => (s.clone(), Color32::WHITE),
                                    CellValue::Number(n) => (format_number(*n), Color32::from_rgb(150, 200, 255)),
                                    CellValue::Boolean(b) => (b.to_string().to_uppercase(), Color32::from_rgb(255, 200, 100)),
                                    CellValue::Formula(f) => (f.clone(), Color32::from_rgb(100, 255, 150)),
                                    CellValue::Error(e) => (format!("#{}", e), Color32::RED),
                                    CellValue::Date(d) => (d.clone(), Color32::from_rgb(200, 180, 255)),
                                    CellValue::Currency(symbol, amount) => (format!("{}{:.2}", symbol, amount), Color32::from_rgb(100, 255, 200)),
                                };
                                
                                if !text.is_empty() {
                                    // Clip text to cell
                                    let text_pos = egui::pos2(
                                        cell_rect.left() + 4.0,
                                        cell_rect.center().y,
                                    );
                                    
                                    painter.text(
                                        text_pos,
                                        egui::Align2::LEFT_CENTER,
                                        &text,
                                        FontId::proportional(11.0 * zoom),
                                        color,
                                    );
                                }
                            }
                        }
                        
                        // Handle click
                        if response.clicked() {
                            if let Some(pos) = response.interact_pointer_pos() {
                                if cell_rect.contains(pos) {
                                    self.selected_cell = Some((row, col));
                                }
                            }
                        }
                    }
                }
            });
    }
    
    fn render_sheet_tabs(&mut self, ui: &mut egui::Ui, spreadsheet: &SpreadsheetContent) {
        egui::TopBottomPanel::bottom("sheet_tabs")
            .resizable(false)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    for (idx, sheet) in spreadsheet.sheets.iter().enumerate() {
                        let selected = idx == self.active_sheet;
                        if ui.selectable_label(selected, &sheet.name).clicked() {
                            self.active_sheet = idx;
                            self.selected_cell = None;
                        }
                    }
                });
            });
    }
}

fn column_name(col: usize) -> String {
    let mut name = String::new();
    let mut n = col;
    
    loop {
        name.insert(0, (b'A' + (n % 26) as u8) as char);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    
    name
}

fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e10 {
        format!("{:.0}", n)
    } else if n.abs() < 0.0001 || n.abs() >= 1e10 {
        format!("{:.2e}", n)
    } else {
        format!("{:.4}", n).trim_end_matches('0').trim_end_matches('.').to_string()
    }
}
