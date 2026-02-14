#![allow(deprecated)]
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
    
    pub fn render(&mut self, ui: &mut egui::Ui, file: &OpenFile, zoom: f32, icons: &crate::icons::Icons) {
        if let FileContent::Spreadsheet(spreadsheet) = &file.content {
            // Toolbar
            self.render_toolbar(ui, spreadsheet, zoom, icons);
            
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
    
    fn render_toolbar(&mut self, ui: &mut egui::Ui, _spreadsheet: &SpreadsheetContent, zoom: f32, icons: &crate::icons::Icons) {
        ui.horizontal(|ui| {
            if ui.selectable_label(!self.edit_mode, "View").clicked() {
                self.edit_mode = false;
            }
            if ui.selectable_label(self.edit_mode, " Edit").clicked() {
                self.edit_mode = true;
            }
            
            ui.separator();
            
            ui.checkbox(&mut self.show_gridlines, "# Grid");
            
            ui.separator();
            
            // Column width
            ui.label("Col Width:");
            if icons.button(ui, "minus", "Decrease column width").clicked() {
                self.default_column_width = (self.default_column_width - 10.0).max(40.0);
            }
            ui.label(format!("{:.0}", self.default_column_width));
            if icons.button(ui, "plus", "Increase column width").clicked() {
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
                // Apply formula to cell
                if let Some((row, col)) = self.selected_cell {
                    if let Some(sheet) = spreadsheet.sheets.get(self.active_sheet) {
                        let value = if self.formula_bar_text.starts_with('=') {
                            // Evaluate formula
                            let formula = &self.formula_bar_text[1..];
                            match Self::evaluate_formula(formula, sheet) {
                                Ok(result) => CellValue::Number(result),
                                Err(e) => CellValue::Error(e),
                            }
                        } else if let Ok(num) = self.formula_bar_text.parse::<f64>() {
                            CellValue::Number(num)
                        } else if self.formula_bar_text.to_uppercase() == "TRUE" {
                            CellValue::Boolean(true)
                        } else if self.formula_bar_text.to_uppercase() == "FALSE" {
                            CellValue::Boolean(false)
                        } else {
                            CellValue::Text(self.formula_bar_text.clone())
                        };

                        // Note: In a mutable context, we would update the cell here
                        // For now, this is just showing the evaluation logic
                    }
                }
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

    /// Basic formula evaluation
    fn evaluate_formula(formula: &str, sheet: &Sheet) -> Result<f64, String> {
        let formula = formula.trim();

        // Handle function calls
        if formula.contains('(') {
            return Self::evaluate_function(formula, sheet);
        }

        // Handle cell references (e.g., A1, B2)
        if let Some(value) = Self::parse_cell_reference(formula, sheet) {
            return Ok(value);
        }

        // Handle simple arithmetic
        if formula.contains('+') || formula.contains('-') || formula.contains('*') || formula.contains('/') {
            return Self::evaluate_arithmetic(formula, sheet);
        }

        // Try to parse as number
        formula.parse::<f64>().map_err(|_| format!("Invalid formula: {}", formula))
    }

    fn evaluate_function(formula: &str, sheet: &Sheet) -> Result<f64, String> {
        let (func_name, args_str) = if let Some(paren_pos) = formula.find('(') {
            let name = formula[..paren_pos].trim().to_uppercase();
            let args = formula[paren_pos + 1..formula.len() - 1].trim();
            (name, args)
        } else {
            return Err("Invalid function syntax".to_string());
        };

        let args: Vec<&str> = args_str.split(',').map(|s| s.trim()).collect();

        match func_name.as_str() {
            "SUM" => {
                let mut sum = 0.0;
                for arg in args {
                    if let Some(range) = Self::parse_range(arg, sheet) {
                        sum += range.iter().sum::<f64>();
                    } else if let Some(val) = Self::parse_cell_reference(arg, sheet) {
                        sum += val;
                    } else if let Ok(val) = arg.parse::<f64>() {
                        sum += val;
                    }
                }
                Ok(sum)
            }
            "AVERAGE" => {
                let mut sum = 0.0;
                let mut count = 0;
                for arg in args {
                    if let Some(range) = Self::parse_range(arg, sheet) {
                        sum += range.iter().sum::<f64>();
                        count += range.len();
                    } else if let Some(val) = Self::parse_cell_reference(arg, sheet) {
                        sum += val;
                        count += 1;
                    } else if let Ok(val) = arg.parse::<f64>() {
                        sum += val;
                        count += 1;
                    }
                }
                if count > 0 {
                    Ok(sum / count as f64)
                } else {
                    Err("No values to average".to_string())
                }
            }
            "COUNT" => {
                let mut count = 0;
                for arg in args {
                    if let Some(range) = Self::parse_range(arg, sheet) {
                        count += range.len();
                    } else {
                        count += 1;
                    }
                }
                Ok(count as f64)
            }
            "MIN" => {
                let mut min = f64::INFINITY;
                for arg in args {
                    if let Some(range) = Self::parse_range(arg, sheet) {
                        if let Some(&val) = range.iter().min_by(|a, b| a.partial_cmp(b).unwrap()) {
                            min = min.min(val);
                        }
                    } else if let Some(val) = Self::parse_cell_reference(arg, sheet) {
                        min = min.min(val);
                    } else if let Ok(val) = arg.parse::<f64>() {
                        min = min.min(val);
                    }
                }
                if min.is_finite() {
                    Ok(min)
                } else {
                    Err("No values for MIN".to_string())
                }
            }
            "MAX" => {
                let mut max = f64::NEG_INFINITY;
                for arg in args {
                    if let Some(range) = Self::parse_range(arg, sheet) {
                        if let Some(&val) = range.iter().max_by(|a, b| a.partial_cmp(b).unwrap()) {
                            max = max.max(val);
                        }
                    } else if let Some(val) = Self::parse_cell_reference(arg, sheet) {
                        max = max.max(val);
                    } else if let Ok(val) = arg.parse::<f64>() {
                        max = max.max(val);
                    }
                }
                if max.is_finite() {
                    Ok(max)
                } else {
                    Err("No values for MAX".to_string())
                }
            }
            "IF" => {
                if args.len() != 3 {
                    return Err("IF requires 3 arguments: condition, true_value, false_value".to_string());
                }
                let condition = Self::evaluate_formula(args[0], sheet)?;
                if condition != 0.0 {
                    Self::evaluate_formula(args[1], sheet)
                } else {
                    Self::evaluate_formula(args[2], sheet)
                }
            }
            _ => Err(format!("Unknown function: {}", func_name)),
        }
    }

    fn parse_cell_reference(ref_str: &str, sheet: &Sheet) -> Option<f64> {
        let ref_str = ref_str.trim();

        // Parse column letters
        let mut col = 0usize;
        let mut row_start = 0;
        for (i, ch) in ref_str.chars().enumerate() {
            if ch.is_ascii_alphabetic() {
                col = col * 26 + (ch.to_ascii_uppercase() as usize - 'A' as usize);
                row_start = i + 1;
            } else {
                break;
            }
        }

        // Parse row number
        let row_str = &ref_str[row_start..];
        let row = row_str.parse::<usize>().ok()?.saturating_sub(1);

        // Get cell value
        sheet.cells.get(row)?.get(col).and_then(|cell| match cell {
            CellValue::Number(n) => Some(*n),
            CellValue::Currency(n, _) => Some(*n),
            _ => None,
        })
    }

    fn parse_range(range_str: &str, sheet: &Sheet) -> Option<Vec<f64>> {
        let range_str = range_str.trim();
        if !range_str.contains(':') {
            return None;
        }

        let parts: Vec<&str> = range_str.split(':').collect();
        if parts.len() != 2 {
            return None;
        }

        let start_ref = parts[0].trim();
        let end_ref = parts[1].trim();

        // Parse start cell
        let (start_col, start_row) = Self::parse_cell_coords(start_ref)?;
        let (end_col, end_row) = Self::parse_cell_coords(end_ref)?;

        let mut values = Vec::new();
        for row in start_row..=end_row {
            if let Some(row_cells) = sheet.cells.get(row) {
                for col in start_col..=end_col {
                    if let Some(cell) = row_cells.get(col) {
                        match cell {
                            CellValue::Number(n) => values.push(*n),
                            CellValue::Currency(n, _) => values.push(*n),
                            _ => {}
                        }
                    }
                }
            }
        }

        Some(values)
    }

    fn parse_cell_coords(ref_str: &str) -> Option<(usize, usize)> {
        let ref_str = ref_str.trim();

        let mut col = 0usize;
        let mut row_start = 0;
        for (i, ch) in ref_str.chars().enumerate() {
            if ch.is_ascii_alphabetic() {
                col = col * 26 + (ch.to_ascii_uppercase() as usize - 'A' as usize);
                row_start = i + 1;
            } else {
                break;
            }
        }

        let row_str = &ref_str[row_start..];
        let row = row_str.parse::<usize>().ok()?.saturating_sub(1);

        Some((col, row))
    }

    fn evaluate_arithmetic(expr: &str, sheet: &Sheet) -> Result<f64, String> {
        // Simple arithmetic evaluation (handles +, -, *, /)
        let expr = expr.replace(" ", "");

        // Handle multiplication and division first
        let mut parts = Vec::new();
        let mut current = String::new();

        for ch in expr.chars() {
            if ch == '+' || ch == '-' {
                if !current.is_empty() {
                    parts.push((current.clone(), ch));
                    current.clear();
                } else if ch == '-' {
                    current.push(ch);
                }
            } else {
                current.push(ch);
            }
        }
        if !current.is_empty() {
            parts.push((current, '+'));
        }

        let mut result = 0.0;
        for (part, op) in parts {
            let value = if part.contains('*') || part.contains('/') {
                Self::evaluate_mul_div(&part, sheet)?
            } else if let Some(val) = Self::parse_cell_reference(&part, sheet) {
                val
            } else {
                part.parse::<f64>().map_err(|_| format!("Invalid number: {}", part))?
            };

            if op == '+' {
                result += value;
            } else {
                result -= value;
            }
        }

        Ok(result)
    }

    fn evaluate_mul_div(expr: &str, sheet: &Sheet) -> Result<f64, String> {
        let mut result = None;
        let mut current_num = String::new();
        let mut current_op = '*';

        for ch in expr.chars() {
            if ch == '*' || ch == '/' {
                let value = if let Some(val) = Self::parse_cell_reference(&current_num, sheet) {
                    val
                } else {
                    current_num.parse::<f64>().map_err(|_| format!("Invalid number: {}", current_num))?
                };

                result = Some(if let Some(r) = result {
                    if current_op == '*' {
                        r * value
                    } else {
                        r / value
                    }
                } else {
                    value
                });

                current_num.clear();
                current_op = ch;
            } else {
                current_num.push(ch);
            }
        }

        // Process last number
        if !current_num.is_empty() {
            let value = if let Some(val) = Self::parse_cell_reference(&current_num, sheet) {
                val
            } else {
                current_num.parse::<f64>().map_err(|_| format!("Invalid number: {}", current_num))?
            };

            result = Some(if let Some(r) = result {
                if current_op == '*' {
                    r * value
                } else {
                    r / value
                }
            } else {
                value
            });
        }

        result.ok_or_else(|| "Empty expression".to_string())
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
