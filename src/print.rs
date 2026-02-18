//! Print module - Cross-platform print dialog and preview
//!
//! Provides printing support for:
//! - Web pages
//! - Documents
//! - Images
//! - PDFs

use eframe::egui;
use printpdf::*;
use std::io::BufWriter;

#[derive(Clone)]
pub struct PrintSettings {
    pub copies: u32,
    pub color_mode: ColorMode,
    pub page_range: PageRange,
    pub paper_size: PaperSize,
    pub orientation: Orientation,
    pub margins: Margins,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ColorMode {
    Color,
    Grayscale,
    BlackAndWhite,
}

#[derive(Clone)]
pub enum PageRange {
    All,
    Current,
    Range(u32, u32),
    Custom(Vec<u32>),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PaperSize {
    Letter,
    Legal,
    A4,
    A3,
    Custom(f32, f32),
}

#[derive(Clone, Copy, PartialEq)]
pub enum Orientation {
    Portrait,
    Landscape,
}

#[derive(Clone, Copy)]
pub struct Margins {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

impl Default for PrintSettings {
    fn default() -> Self {
        Self {
            copies: 1,
            color_mode: ColorMode::Color,
            page_range: PageRange::All,
            paper_size: PaperSize::Letter,
            orientation: Orientation::Portrait,
            margins: Margins {
                top: 0.5,
                bottom: 0.5,
                left: 0.5,
                right: 0.5,
            },
        }
    }
}

pub struct PrintDialog {
    settings: PrintSettings,
    show_preview: bool,
    page_range_text: String,
    is_open: bool,
}

impl PrintDialog {
    pub fn new() -> Self {
        Self {
            settings: PrintSettings::default(),
            show_preview: false,
            page_range_text: String::from("All"),
            is_open: true,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<PrintSettings> {
        if !self.is_open {
            return None;
        }

        let mut result = None;

        egui::Window::new("Print Settings")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(400.0);

                // Number of copies
                ui.horizontal(|ui| {
                    ui.label("Copies:");
                    ui.add(egui::DragValue::new(&mut self.settings.copies).speed(1).range(1..=99));
                });

                ui.add_space(8.0);

                // Color mode
                ui.label("Color Mode:");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.settings.color_mode, ColorMode::Color, "Color");
                    ui.radio_value(
                        &mut self.settings.color_mode,
                        ColorMode::Grayscale,
                        "Grayscale",
                    );
                    ui.radio_value(
                        &mut self.settings.color_mode,
                        ColorMode::BlackAndWhite,
                        "Black & White",
                    );
                });

                ui.add_space(8.0);

                // Paper size
                ui.horizontal(|ui| {
                    ui.label("Paper Size:");
                    egui::ComboBox::from_id_salt("paper_size")
                        .selected_text(format!("{:?}", self.settings.paper_size))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.settings.paper_size,
                                PaperSize::Letter,
                                "Letter",
                            );
                            ui.selectable_value(
                                &mut self.settings.paper_size,
                                PaperSize::Legal,
                                "Legal",
                            );
                            ui.selectable_value(&mut self.settings.paper_size, PaperSize::A4, "A4");
                            ui.selectable_value(&mut self.settings.paper_size, PaperSize::A3, "A3");
                        });
                });

                ui.add_space(8.0);

                // Orientation
                ui.label("Orientation:");
                ui.horizontal(|ui| {
                    ui.radio_value(
                        &mut self.settings.orientation,
                        Orientation::Portrait,
                        "Portrait",
                    );
                    ui.radio_value(
                        &mut self.settings.orientation,
                        Orientation::Landscape,
                        "Landscape",
                    );
                });

                ui.add_space(8.0);

                // Page range
                ui.horizontal(|ui| {
                    ui.label("Page Range:");
                    if ui.text_edit_singleline(&mut self.page_range_text).changed() {
                        self.settings.page_range = parse_page_range(&self.page_range_text);
                    }
                });
                ui.label(egui::RichText::new("(e.g., 'All', '1-5', '1,3,5')").small().weak());

                ui.add_space(8.0);

                // Margins
                ui.label("Margins (inches):");
                ui.horizontal(|ui| {
                    ui.label("Top:");
                    ui.add(
                        egui::DragValue::new(&mut self.settings.margins.top)
                            .speed(0.1)
                            .range(0.0..=2.0),
                    );
                    ui.label("Bottom:");
                    ui.add(
                        egui::DragValue::new(&mut self.settings.margins.bottom)
                            .speed(0.1)
                            .range(0.0..=2.0),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Left:");
                    ui.add(
                        egui::DragValue::new(&mut self.settings.margins.left)
                            .speed(0.1)
                            .range(0.0..=2.0),
                    );
                    ui.label("Right:");
                    ui.add(
                        egui::DragValue::new(&mut self.settings.margins.right)
                            .speed(0.1)
                            .range(0.0..=2.0),
                    );
                });

                ui.add_space(16.0);

                // Buttons
                ui.horizontal(|ui| {
                    if ui.button("Print").clicked() {
                        result = Some(self.settings.clone());
                        self.is_open = false;
                    }
                    if ui.button("Cancel").clicked() {
                        self.is_open = false;
                    }
                });
            });

        result
    }
}

fn parse_page_range(text: &str) -> PageRange {
    let text = text.trim().to_lowercase();
    if text == "all" || text.is_empty() {
        return PageRange::All;
    }
    if text == "current" {
        return PageRange::Current;
    }

    // Try to parse range like "1-5"
    if let Some(dash_pos) = text.find('-') {
        let start = text[..dash_pos].trim().parse::<u32>().unwrap_or(1);
        let end = text[dash_pos + 1..].trim().parse::<u32>().unwrap_or(1);
        return PageRange::Range(start, end);
    }

    // Try to parse custom list like "1,3,5"
    if text.contains(',') {
        let pages: Vec<u32> = text
            .split(',')
            .filter_map(|s| s.trim().parse::<u32>().ok())
            .collect();
        if !pages.is_empty() {
            return PageRange::Custom(pages);
        }
    }

    // Single page number
    if let Ok(page) = text.parse::<u32>() {
        return PageRange::Custom(vec![page]);
    }

    PageRange::All
}

/// Print the current page/document
pub fn print_page(content: &[u8], settings: &PrintSettings) -> Result<(), String> {
    // Convert content to text (assume UTF-8)
    let text = String::from_utf8_lossy(content);

    // Get paper dimensions in mm
    let (page_width_mm, page_height_mm) = match settings.paper_size {
        PaperSize::Letter => (215.9, 279.4),
        PaperSize::Legal => (215.9, 355.6),
        PaperSize::A4 => (210.0, 297.0),
        PaperSize::A3 => (297.0, 420.0),
        PaperSize::Custom(w, h) => (w * 25.4, h * 25.4), // inches to mm
    };

    // Apply orientation
    let (width_mm, height_mm) = match settings.orientation {
        Orientation::Portrait => (page_width_mm, page_height_mm),
        Orientation::Landscape => (page_height_mm, page_width_mm),
    };

    // Create PDF document
    let (doc, page1, layer1) =
        PdfDocument::new("Print Output", Mm(width_mm), Mm(height_mm), "Layer 1");
    let current_layer = doc.get_page(page1).get_layer(layer1);

    // Convert margins from inches to mm
    let margin_top = Mm(settings.margins.top * 25.4);
    let margin_bottom = Mm(settings.margins.bottom * 25.4);
    let margin_left = Mm(settings.margins.left * 25.4);
    let margin_right = Mm(settings.margins.right * 25.4);

    // Calculate printable area
    let printable_width = width_mm - (margin_left.0 + margin_right.0);
    let _printable_height = height_mm - (margin_top.0 + margin_bottom.0);

    // Font settings
    let font_size = 12.0;
    let line_height = font_size * 1.2;

    // Calculate starting Y position (from top)
    let mut y_pos = height_mm - margin_top.0 - font_size;

    // Load built-in font
    let font = doc.add_builtin_font(BuiltinFont::Helvetica).map_err(|e| e.to_string())?;

    // Split text into lines and render
    let lines: Vec<&str> = text.lines().collect();
    let max_chars_per_line = (printable_width / (font_size * 0.6)) as usize;

    let mut current_layer_ref = current_layer;

    for line in lines {
        // Wrap long lines
        let wrapped_lines = wrap_text(line, max_chars_per_line);

        for wrapped_line in wrapped_lines {
            // Check if we need a new page
            if y_pos < margin_bottom.0 + font_size {
                // Add new page
                let (new_page, new_layer) =
                    doc.add_page(Mm(width_mm), Mm(height_mm), "Layer 1");
                current_layer_ref = doc.get_page(new_page).get_layer(new_layer);
                y_pos = height_mm - margin_top.0 - font_size;
            }

            // Write text
            current_layer_ref.use_text(
                &wrapped_line,
                font_size,
                Mm(margin_left.0),
                Mm(y_pos),
                &font,
            );

            y_pos -= line_height;
        }
    }

    // Save PDF to buffer
    let mut buffer = Vec::new();
    doc.save(&mut BufWriter::new(&mut buffer))
        .map_err(|e| format!("Failed to save PDF: {}", e))?;

    // Open the PDF with system viewer
    let temp_path = std::env::temp_dir().join("sassy_print_output.pdf");
    std::fs::write(&temp_path, buffer)
        .map_err(|e| format!("Failed to write PDF file: {}", e))?;

    // Try to open with system default
    if let Err(e) = open::that(&temp_path) {
        return Err(format!("Failed to open PDF: {}", e));
    }

    Ok(())
}

fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if text.len() <= max_width {
        return vec![text.to_string()];
    }

    let mut result = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.len() + word.len() + 1 > max_width {
            if !current_line.is_empty() {
                result.push(current_line.clone());
                current_line.clear();
            }
            // If single word is too long, split it
            if word.len() > max_width {
                let mut remaining = word;
                while remaining.len() > max_width {
                    result.push(remaining[..max_width].to_string());
                    remaining = &remaining[max_width..];
                }
                current_line = remaining.to_string();
            } else {
                current_line = word.to_string();
            }
        } else {
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }
    }

    if !current_line.is_empty() {
        result.push(current_line);
    }

    if result.is_empty() {
        result.push(String::new());
    }

    result
}

/// Generate print preview
pub fn generate_preview(content: &[u8], settings: &PrintSettings) -> Option<Vec<u8>> {
    use ::image::{ImageBuffer, Rgba, RgbaImage};

    // Convert content to text
    let text = String::from_utf8_lossy(content);

    // Get paper dimensions in pixels at 72 DPI
    let (page_width_in, page_height_in) = match settings.paper_size {
        PaperSize::Letter => (8.5, 11.0),
        PaperSize::Legal => (8.5, 14.0),
        PaperSize::A4 => (8.27, 11.69),
        PaperSize::A3 => (11.69, 16.54),
        PaperSize::Custom(w, h) => (w, h),
    };

    // Apply orientation
    let (width_in, height_in) = match settings.orientation {
        Orientation::Portrait => (page_width_in, page_height_in),
        Orientation::Landscape => (page_height_in, page_width_in),
    };

    // Convert to pixels at 72 DPI
    let width_px = (width_in * 72.0) as u32;
    let height_px = (height_in * 72.0) as u32;

    // Create white background
    let mut img: RgbaImage = ImageBuffer::from_pixel(width_px, height_px, Rgba([255, 255, 255, 255]));

    // Calculate margins in pixels
    let margin_top = (settings.margins.top * 72.0) as i32;
    let margin_left = (settings.margins.left * 72.0) as i32;
    let margin_right = (settings.margins.right * 72.0) as i32;
    let margin_bottom = (settings.margins.bottom * 72.0) as i32;

    // Draw margin lines (light gray)
    for x in margin_left.max(0)..=(width_px as i32 - margin_right).min(width_px as i32 - 1) {
        if margin_top >= 0 && margin_top < height_px as i32 {
            img.put_pixel(x as u32, margin_top as u32, Rgba([200, 200, 200, 255]));
        }
        let bottom_y = height_px as i32 - margin_bottom;
        if bottom_y >= 0 && bottom_y < height_px as i32 {
            img.put_pixel(x as u32, bottom_y as u32, Rgba([200, 200, 200, 255]));
        }
    }
    for y in margin_top.max(0)..=(height_px as i32 - margin_bottom).min(height_px as i32 - 1) {
        if margin_left >= 0 && margin_left < width_px as i32 {
            img.put_pixel(margin_left as u32, y as u32, Rgba([200, 200, 200, 255]));
        }
        let right_x = width_px as i32 - margin_right;
        if right_x >= 0 && right_x < width_px as i32 {
            img.put_pixel(right_x as u32, y as u32, Rgba([200, 200, 200, 255]));
        }
    }

    // Simple text rendering (draw dots as placeholder for characters)
    let font_size = 10;
    let line_height = font_size + 4;
    let mut y = margin_top + font_size;

    let max_chars_per_line = ((width_px as i32 - margin_left - margin_right) / 6).max(1) as usize;

    for line in text.lines().take(50) {
        // Limit preview to 50 lines
        let wrapped = wrap_text(line, max_chars_per_line);
        for wrapped_line in wrapped {
            if y + line_height > height_px as i32 - margin_bottom {
                break;
            }

            // Draw simple character representations
            let mut x = margin_left;
            for ch in wrapped_line.chars().take(max_chars_per_line) {
                if x + 6 > width_px as i32 - margin_right {
                    break;
                }

                // Draw a simple rectangle for each character
                if !ch.is_whitespace() {
                    for dy in 0..font_size.min(8) {
                        for dx in 0..6 {
                            let px = (x + dx) as u32;
                            let py = (y + dy) as u32;
                            if px < width_px && py < height_px {
                                img.put_pixel(px, py, Rgba([0, 0, 0, 255]));
                            }
                        }
                    }
                }
                x += 6;
            }

            y += line_height;
        }
    }

    // Add "Preview" watermark
    let watermark_text = "PREVIEW";
    let watermark_x = width_px / 2 - 50;
    let watermark_y = height_px / 2;
    for (i, _ch) in watermark_text.chars().enumerate() {
        let x_offset = i * 15;
        for dy in 0..20 {
            for dx in 0..12 {
                let px = watermark_x + x_offset as u32 + dx;
                let py = watermark_y + dy;
                if px < width_px && py < height_px {
                    img.put_pixel(px, py, Rgba([220, 220, 220, 128]));
                }
            }
        }
    }

    // Convert to PNG bytes
    let mut buffer = Vec::new();
    let encoder = ::image::codecs::png::PngEncoder::new(&mut buffer);
    use ::image::ImageEncoder;
    match encoder.write_image(
        &img,
        width_px,
        height_px,
        ::image::ExtendedColorType::Rgba8,
    ) {
        Ok(_) => Some(buffer),
        Err(_) => None,
    }
}
