#![allow(deprecated)]
//! Image Editor - Full editing capabilities for all image formats
//!
//! Features:
//! - View: Zoom, pan, rotate, fit
//! - Edit: Crop, resize, flip, adjustments, filters
//! - Save: Export to PNG, JPG, WebP, BMP, TIFF
//! - Print: System print dialog

use eframe::egui::{self, Color32, ColorImage, Pos2, Rect, Sense, Stroke, TextureHandle, Vec2};
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Editing tool modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditTool {
    Select,
    Crop,
    Resize,
    Draw,
    Text,
    Eyedropper,
}

/// Image adjustments
#[derive(Debug, Clone)]
pub struct ImageAdjustments {
    pub brightness: f32,  // -100 to 100
    pub contrast: f32,    // -100 to 100
    pub saturation: f32,  // -100 to 100
    pub hue: f32,         // -180 to 180
    pub gamma: f32,       // 0.1 to 3.0
    pub exposure: f32,    // -3 to 3
    pub highlights: f32,  // -100 to 100
    pub shadows: f32,     // -100 to 100
    pub temperature: f32, // -100 (cool) to 100 (warm)
    pub tint: f32,        // -100 (green) to 100 (magenta)
}

impl Default for ImageAdjustments {
    fn default() -> Self {
        Self {
            brightness: 0.0,
            contrast: 0.0,
            saturation: 0.0,
            hue: 0.0,
            gamma: 1.0,
            exposure: 0.0,
            highlights: 0.0,
            shadows: 0.0,
            temperature: 0.0,
            tint: 0.0,
        }
    }
}

/// Filter presets
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImageFilter {
    None,
    Grayscale,
    Sepia,
    Invert,
    Blur,
    Sharpen,
    EdgeDetect,
    Emboss,
    Vintage,
    Dramatic,
    Cool,
    Warm,
}

/// Crop selection
#[derive(Debug, Clone)]
pub struct CropSelection {
    pub start: Pos2,
    pub end: Pos2,
    pub aspect_ratio: Option<f32>, // None = free, Some(1.0) = square, Some(16.0/9.0) = 16:9
}

/// Undo/Redo history entry
#[derive(Clone)]
pub struct HistoryEntry {
    pub image: DynamicImage,
    pub description: String,
}

pub struct ImageViewer {
    // Display state
    texture_cache: HashMap<PathBuf, TextureHandle>,
    pan_offset: Vec2,
    rotation: i32, // 0, 90, 180, 270
    fit_to_window: bool,
    show_info: bool,
    zoom_override: Option<f32>,
    current_path: Option<PathBuf>,

    // Editing state
    current_tool: EditTool,
    original_image: Option<DynamicImage>,
    working_image: Option<DynamicImage>,
    working_texture: Option<TextureHandle>,

    // Adjustments
    adjustments: ImageAdjustments,
    current_filter: ImageFilter,

    // Crop
    crop_selection: Option<CropSelection>,
    crop_aspect: Option<f32>,

    // Resize
    resize_width: u32,
    resize_height: u32,
    resize_maintain_aspect: bool,

    // History (undo/redo)
    history: Vec<HistoryEntry>,
    history_index: usize,
    max_history: usize,

    // UI state
    show_adjustments: bool,
    show_filters: bool,
    show_resize: bool,
    show_export: bool,
    has_unsaved_changes: bool,

    // Export options
    export_format: ExportFormat,
    export_quality: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportFormat {
    Png,
    Jpeg,
    WebP,
    Bmp,
    Tiff,
    Gif,
}

impl ImageViewer {
    pub fn new() -> Self {
        Self {
            texture_cache: HashMap::new(),
            pan_offset: Vec2::ZERO,
            rotation: 0,
            fit_to_window: true,
            show_info: false,
            zoom_override: None,
            current_path: None,

            current_tool: EditTool::Select,
            original_image: None,
            working_image: None,
            working_texture: None,

            adjustments: ImageAdjustments::default(),
            current_filter: ImageFilter::None,

            crop_selection: None,
            crop_aspect: None,

            resize_width: 0,
            resize_height: 0,
            resize_maintain_aspect: true,

            history: Vec::new(),
            history_index: 0,
            max_history: 50,

            show_adjustments: false,
            show_filters: false,
            show_resize: false,
            show_export: false,
            has_unsaved_changes: false,

            export_format: ExportFormat::Png,
            export_quality: 90,
        }
    }

    /// Load image from bytes
    pub fn load_image(&mut self, data: &[u8], path: &std::path::Path) {
        if let Ok(img) = image::load_from_memory(data) {
            self.current_path = Some(path.to_path_buf());
            self.resize_width = img.width();
            self.resize_height = img.height();
            self.original_image = Some(img.clone());
            self.working_image = Some(img.clone());
            self.working_texture = None;

            // Reset editing state
            self.adjustments = ImageAdjustments::default();
            self.current_filter = ImageFilter::None;
            self.crop_selection = None;
            self.rotation = 0;
            self.has_unsaved_changes = false;

            // Clear history and add initial state
            self.history.clear();
            self.history.push(HistoryEntry {
                image: img,
                description: "Original".to_string(),
            });
            self.history_index = 0;
        }
    }

    /// Add to history for undo
    fn push_history(&mut self, description: &str) {
        if let Some(img) = &self.working_image {
            // Remove any history after current index (if we undid some actions)
            self.history.truncate(self.history_index + 1);

            self.history.push(HistoryEntry {
                image: img.clone(),
                description: description.to_string(),
            });

            // Limit history size
            if self.history.len() > self.max_history {
                self.history.remove(0);
            } else {
                self.history_index = self.history.len() - 1;
            }

            self.has_unsaved_changes = true;
        }
    }

    /// Undo last action
    pub fn undo(&mut self) {
        if self.history_index > 0 {
            self.history_index -= 1;
            self.working_image = Some(self.history[self.history_index].image.clone());
            self.working_texture = None;
        }
    }

    /// Redo action
    pub fn redo(&mut self) {
        if self.history_index < self.history.len() - 1 {
            self.history_index += 1;
            self.working_image = Some(self.history[self.history_index].image.clone());
            self.working_texture = None;
        }
    }

    /// Reset to original
    pub fn reset(&mut self) {
        if let Some(original) = &self.original_image {
            self.working_image = Some(original.clone());
            self.working_texture = None;
            self.adjustments = ImageAdjustments::default();
            self.current_filter = ImageFilter::None;
            self.rotation = 0;
            self.crop_selection = None;
            self.push_history("Reset");
        }
    }

    // ---------------------------------------------------------------------------
    // TRANSFORMATIONS
    // ---------------------------------------------------------------------------

    /// Rotate 90 degrees clockwise
    pub fn rotate_cw(&mut self) {
        if let Some(img) = &self.working_image {
            self.working_image = Some(img.rotate90());
            self.working_texture = None;
            self.push_history("Rotate 90° CW");
        }
    }

    /// Rotate 90 degrees counter-clockwise  
    pub fn rotate_ccw(&mut self) {
        if let Some(img) = &self.working_image {
            self.working_image = Some(img.rotate270());
            self.working_texture = None;
            self.push_history("Rotate 90° CCW");
        }
    }

    /// Rotate 180 degrees
    pub fn rotate_180(&mut self) {
        if let Some(img) = &self.working_image {
            self.working_image = Some(img.rotate180());
            self.working_texture = None;
            self.push_history("Rotate 180°");
        }
    }

    /// Flip horizontal
    pub fn flip_horizontal(&mut self) {
        if let Some(img) = &self.working_image {
            self.working_image = Some(img.fliph());
            self.working_texture = None;
            self.push_history("Flip Horizontal");
        }
    }

    /// Flip vertical
    pub fn flip_vertical(&mut self) {
        if let Some(img) = &self.working_image {
            self.working_image = Some(img.flipv());
            self.working_texture = None;
            self.push_history("Flip Vertical");
        }
    }

    /// Crop image
    pub fn apply_crop(&mut self) {
        if let (Some(img), Some(crop)) = (&self.working_image, &self.crop_selection) {
            let x = crop.start.x.min(crop.end.x) as u32;
            let y = crop.start.y.min(crop.end.y) as u32;
            let w = (crop.end.x - crop.start.x).abs() as u32;
            let h = (crop.end.y - crop.start.y).abs() as u32;

            if w > 0 && h > 0 && x + w <= img.width() && y + h <= img.height() {
                self.working_image = Some(img.crop_imm(x, y, w, h));
                self.working_texture = None;
                self.crop_selection = None;
                self.push_history("Crop");
            }
        }
    }

    /// Resize image
    pub fn apply_resize(&mut self, width: u32, height: u32) {
        if let Some(img) = &self.working_image {
            if width > 0 && height > 0 {
                self.working_image =
                    Some(img.resize_exact(width, height, image::imageops::FilterType::Lanczos3));
                self.working_texture = None;
                self.resize_width = width;
                self.resize_height = height;
                self.push_history(&format!("Resize to {}x{}", width, height));
            }
        }
    }

    // ---------------------------------------------------------------------------
    // ADJUSTMENTS
    // ---------------------------------------------------------------------------

    /// Apply brightness adjustment (-100 to 100)
    pub fn apply_adjustments(&mut self) {
        if let Some(original) = &self.original_image {
            let mut img = original.clone();
            let adj = &self.adjustments;

            // Brightness
            if adj.brightness != 0.0 {
                img = img.brighten((adj.brightness * 2.55) as i32);
            }

            // Contrast
            if adj.contrast != 0.0 {
                img = img.adjust_contrast(adj.contrast);
            }

            // Hue rotation
            if adj.hue != 0.0 {
                img = img.huerotate(adj.hue as i32);
            }

            // Apply filter on top
            img = self.apply_filter_to_image(img);

            self.working_image = Some(img);
            self.working_texture = None;
        }
    }

    /// Apply filter to image
    fn apply_filter_to_image(&self, img: DynamicImage) -> DynamicImage {
        match self.current_filter {
            ImageFilter::None => img,
            ImageFilter::Grayscale => img.grayscale(),
            ImageFilter::Sepia => {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                let mut output = ImageBuffer::new(w, h);
                for (x, y, pixel) in rgba.enumerate_pixels() {
                    let r = pixel[0] as f32;
                    let g = pixel[1] as f32;
                    let b = pixel[2] as f32;
                    let new_r = (0.393 * r + 0.769 * g + 0.189 * b).min(255.0) as u8;
                    let new_g = (0.349 * r + 0.686 * g + 0.168 * b).min(255.0) as u8;
                    let new_b = (0.272 * r + 0.534 * g + 0.131 * b).min(255.0) as u8;
                    output.put_pixel(x, y, Rgba([new_r, new_g, new_b, pixel[3]]));
                }
                DynamicImage::ImageRgba8(output)
            }
            ImageFilter::Invert => {
                let mut inverted = img.clone();
                inverted.invert();
                inverted
            }
            ImageFilter::Blur => img.blur(2.0),
            ImageFilter::Sharpen => img.unsharpen(1.5, 5),
            _ => img, // Other filters to be implemented
        }
    }

    // ---------------------------------------------------------------------------
    // EXPORT / SAVE
    // ---------------------------------------------------------------------------

    /// Save image to file
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(img) = &self.working_image {
            img.save(path).map_err(|e| e.to_string())
        } else {
            Err("No image loaded".to_string())
        }
    }

    /// Export to specific format
    pub fn export(&self, path: &Path, format: ExportFormat, quality: u8) -> Result<(), String> {
        use image::codecs::jpeg::JpegEncoder;
        use std::fs::File;
        use std::io::BufWriter;

        if let Some(img) = &self.working_image {
            let file = File::create(path).map_err(|e| e.to_string())?;
            let mut writer = BufWriter::new(file);

            match format {
                ExportFormat::Png => img.save(path).map_err(|e| e.to_string()),
                ExportFormat::Jpeg => {
                    let rgb = img.to_rgb8();
                    let encoder = JpegEncoder::new_with_quality(&mut writer, quality);
                    rgb.write_with_encoder(encoder).map_err(|e| e.to_string())
                }
                ExportFormat::WebP => {
                    // WebP requires special handling
                    let new_path = path.with_extension("webp");
                    img.save(new_path).map_err(|e| e.to_string())
                }
                ExportFormat::Bmp => img.save(path).map_err(|e| e.to_string()),
                ExportFormat::Tiff => img.save(path).map_err(|e| e.to_string()),
                ExportFormat::Gif => img.save(path).map_err(|e| e.to_string()),
            }
        } else {
            Err("No image loaded".to_string())
        }
    }

    /// Get image dimensions
    pub fn dimensions(&self) -> Option<(u32, u32)> {
        self.working_image.as_ref().map(|img| img.dimensions())
    }

    // ---------------------------------------------------------------------------
    // UI RENDERING
    // ---------------------------------------------------------------------------

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        file: &crate::file_handler::OpenFile,
        zoom: f32,
        icons: &crate::icons::Icons,
    ) {
        // Load image if not already loaded
        if self.working_image.is_none() {
            if let crate::file_handler::FileContent::Binary(data) = &file.content {
                self.load_image(data, &file.path);
            }
        }

        // Top toolbar
        self.render_toolbar(ui, icons);
        ui.separator();

        // Main content area with side panel
        ui.horizontal(|ui| {
            // Side panel for adjustments/filters
            if self.show_adjustments || self.show_filters || self.show_resize {
                egui::SidePanel::left("image_edit_panel")
                    .resizable(true)
                    .min_width(200.0)
                    .max_width(350.0)
                    .show_inside(ui, |ui| {
                        self.render_side_panel(ui);
                    });
            }

            // Main image area
            self.render_image_area(ui, zoom);
        });

        // Export dialog
        if self.show_export {
            self.render_export_dialog(ui);
        }
    }

    fn render_toolbar(&mut self, ui: &mut egui::Ui, icons: &crate::icons::Icons) {
        ui.horizontal(|ui| {
            // File operations
            if icons
                .text_button(ui, "download", "Save", "Save image")
                .clicked()
            {
                self.show_export = true;
            }
            if icons
                .text_button(ui, "upload", "Export", "Export image")
                .clicked()
            {
                self.show_export = true;
            }
            if icons
                .text_button(ui, "file-pdf", "Print", "Print image")
                .clicked()
            {
                // TODO: System print dialog
            }

            ui.separator();

            // Undo/Redo
            ui.add_enabled_ui(self.history_index > 0, |ui| {
                if icons.button(ui, "nav-back", "Undo").clicked() {
                    self.undo();
                }
            });
            ui.add_enabled_ui(
                self.history_index < self.history.len().saturating_sub(1),
                |ui| {
                    if icons.button(ui, "nav-forward", "Redo").clicked() {
                        self.redo();
                    }
                },
            );
            if icons.button(ui, "reload", "Reset edits").clicked() {
                self.reset();
            }

            ui.separator();

            // Transform tools
            if icons.button(ui, "reload", "Rotate left").clicked() {
                self.rotate_ccw();
            }
            if icons.button(ui, "reload", "Rotate right").clicked() {
                self.rotate_cw();
            }
            if icons.button(ui, "arrow-left", "Flip horizontal").clicked() {
                self.flip_horizontal();
            }
            if icons.button(ui, "arrow-up", "Flip vertical").clicked() {
                self.flip_vertical();
            }

            ui.separator();

            // Tool selection
            icons.inline(ui, "select-cursor");
            ui.selectable_value(&mut self.current_tool, EditTool::Select, "Select");
            icons.inline(ui, "extract");
            ui.selectable_value(&mut self.current_tool, EditTool::Crop, "Crop");

            ui.separator();

            // Panel toggles
            icons.inline(ui, "settings");
            ui.toggle_value(&mut self.show_adjustments, "Adjust");
            icons.inline(ui, "bullet");
            ui.toggle_value(&mut self.show_filters, "Filters");
            icons.inline(ui, "fullscreen");
            ui.toggle_value(&mut self.show_resize, "Resize");

            // Right side - image info
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(path) = &self.current_path {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        ui.label(name);
                    }
                }
                if let Some((w, h)) = self.dimensions() {
                    ui.label(format!("{} x {}", w, h));
                }
                if self.has_unsaved_changes {
                    ui.label("*").on_hover_text("Unsaved changes");
                }
            });
        });
    }

    fn render_side_panel(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Adjustments panel
            if self.show_adjustments {
                ui.collapsing("(style) Adjustments", |ui| {
                    let mut changed = false;

                    ui.label("Brightness");
                    changed |= ui
                        .add(egui::Slider::new(
                            &mut self.adjustments.brightness,
                            -100.0..=100.0,
                        ))
                        .changed();

                    ui.label("Contrast");
                    changed |= ui
                        .add(egui::Slider::new(
                            &mut self.adjustments.contrast,
                            -100.0..=100.0,
                        ))
                        .changed();

                    ui.label("Saturation");
                    changed |= ui
                        .add(egui::Slider::new(
                            &mut self.adjustments.saturation,
                            -100.0..=100.0,
                        ))
                        .changed();

                    ui.label("Hue");
                    changed |= ui
                        .add(egui::Slider::new(&mut self.adjustments.hue, -180.0..=180.0))
                        .changed();

                    ui.label("Gamma");
                    changed |= ui
                        .add(egui::Slider::new(&mut self.adjustments.gamma, 0.1..=3.0))
                        .changed();

                    if changed {
                        self.apply_adjustments();
                    }

                    if ui.button("Reset Adjustments").clicked() {
                        self.adjustments = ImageAdjustments::default();
                        self.apply_adjustments();
                    }
                });
            }

            // Filters panel
            if self.show_filters {
                ui.collapsing("* Filters", |ui| {
                    let filters = [
                        (ImageFilter::None, "None"),
                        (ImageFilter::Grayscale, "Grayscale"),
                        (ImageFilter::Sepia, "Sepia"),
                        (ImageFilter::Invert, "Invert"),
                        (ImageFilter::Blur, "Blur"),
                        (ImageFilter::Sharpen, "Sharpen"),
                        (ImageFilter::Vintage, "Vintage"),
                        (ImageFilter::Dramatic, "Dramatic"),
                        (ImageFilter::Cool, "Cool"),
                        (ImageFilter::Warm, "Warm"),
                    ];

                    for (filter, name) in filters {
                        if ui
                            .selectable_label(self.current_filter == filter, name)
                            .clicked()
                        {
                            self.current_filter = filter;
                            self.apply_adjustments();
                        }
                    }
                });
            }

            // Resize panel
            if self.show_resize {
                ui.collapsing("(resize) Resize", |ui| {
                    if let Some((orig_w, orig_h)) = self.dimensions() {
                        let aspect = orig_w as f32 / orig_h as f32;

                        ui.checkbox(&mut self.resize_maintain_aspect, "Maintain aspect ratio");

                        ui.horizontal(|ui| {
                            ui.label("Width:");
                            if ui
                                .add(egui::DragValue::new(&mut self.resize_width).speed(1))
                                .changed()
                                && self.resize_maintain_aspect
                            {
                                self.resize_height = (self.resize_width as f32 / aspect) as u32;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Height:");
                            if ui
                                .add(egui::DragValue::new(&mut self.resize_height).speed(1))
                                .changed()
                                && self.resize_maintain_aspect
                            {
                                self.resize_width = (self.resize_height as f32 * aspect) as u32;
                            }
                        });

                        // Preset sizes
                        ui.label("Presets:");
                        ui.horizontal_wrapped(|ui| {
                            for (name, w, h) in [
                                ("HD", 1280, 720),
                                ("Full HD", 1920, 1080),
                                ("4K", 3840, 2160),
                                ("50%", orig_w / 2, orig_h / 2),
                                ("25%", orig_w / 4, orig_h / 4),
                            ] {
                                if ui.small_button(name).clicked() {
                                    self.resize_width = w;
                                    self.resize_height = h;
                                }
                            }
                        });

                        if ui.button("Apply Resize").clicked() {
                            self.apply_resize(self.resize_width, self.resize_height);
                        }
                    }
                });
            }

            // Crop panel
            if self.current_tool == EditTool::Crop {
                ui.separator();
                ui.label("Crop Selection");

                ui.horizontal(|ui| {
                    if ui.button("Free").clicked() {
                        self.crop_aspect = None;
                    }
                    if ui.button("1:1").clicked() {
                        self.crop_aspect = Some(1.0);
                    }
                    if ui.button("4:3").clicked() {
                        self.crop_aspect = Some(4.0 / 3.0);
                    }
                    if ui.button("16:9").clicked() {
                        self.crop_aspect = Some(16.0 / 9.0);
                    }
                });

                if ui.button("Apply Crop").clicked() {
                    self.apply_crop();
                }
                if ui.button("Cancel").clicked() {
                    self.crop_selection = None;
                }
            }
        });
    }

    fn render_image_area(&mut self, ui: &mut egui::Ui, zoom: f32) {
        let available = ui.available_size();

        // Create texture from working image if needed
        if self.working_texture.is_none() {
            if let Some(img) = &self.working_image {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let pixels = rgba.into_raw();
                let color_image = ColorImage::from_rgba_unmultiplied(size, &pixels);
                self.working_texture = Some(ui.ctx().load_texture(
                    "working_image",
                    color_image,
                    egui::TextureOptions::LINEAR,
                ));
            }
        }

        // Extract texture info before closure to avoid borrow conflicts
        let texture_info = self
            .working_texture
            .as_ref()
            .map(|t| (t.id(), t.size_vec2()));

        if let Some((texture_id, img_size)) = texture_info {
            // Calculate display size
            let effective_zoom = self.zoom_override.unwrap_or(zoom);
            let display_size = if self.fit_to_window {
                let scale_x = available.x / img_size.x;
                let scale_y = (available.y - 20.0) / img_size.y;
                let scale = scale_x.min(scale_y).min(1.0);
                img_size * scale * effective_zoom
            } else {
                img_size * effective_zoom
            };

            // Scrollable area
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let (response, painter) =
                        ui.allocate_painter(display_size.max(available), Sense::click_and_drag());

                    // Handle pan with middle mouse or when select tool
                    if self.current_tool == EditTool::Select && response.dragged() {
                        self.pan_offset += response.drag_delta();
                    }

                    // Calculate image rect
                    let offset = (available - display_size) / 2.0 + self.pan_offset;
                    let rect = Rect::from_min_size(
                        response.rect.min + offset.max(Vec2::ZERO),
                        display_size,
                    );

                    // Draw checkerboard for transparency
                    let checker_size = 10.0;
                    let cols = (rect.width() / checker_size).ceil() as i32;
                    let rows = (rect.height() / checker_size).ceil() as i32;
                    for row in 0..rows {
                        for col in 0..cols {
                            let x = rect.left() + col as f32 * checker_size;
                            let y = rect.top() + row as f32 * checker_size;
                            let tile_rect = Rect::from_min_max(
                                Pos2::new(x, y),
                                Pos2::new(
                                    (x + checker_size).min(rect.right()),
                                    (y + checker_size).min(rect.bottom()),
                                ),
                            );
                            let color = if (row + col) % 2 == 0 {
                                Color32::from_gray(35)
                            } else {
                                Color32::from_gray(55)
                            };
                            painter.rect_filled(tile_rect, 0.0, color);
                        }
                    }

                    // Draw image
                    painter.image(
                        texture_id,
                        rect,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );

                    // Handle crop tool
                    if self.current_tool == EditTool::Crop {
                        self.handle_crop_interaction(
                            &response,
                            &painter,
                            rect,
                            img_size,
                            display_size,
                        );
                    }
                });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Loading image...");
            });
        }
    }

    fn handle_crop_interaction(
        &mut self,
        response: &egui::Response,
        painter: &egui::Painter,
        image_rect: Rect,
        img_size: Vec2,
        display_size: Vec2,
    ) {
        let scale = img_size / display_size;

        // Start crop selection
        if response.drag_started() {
            if let Some(pos) = response.interact_pointer_pos() {
                let local = (pos - image_rect.min) * scale;
                self.crop_selection = Some(CropSelection {
                    start: Pos2::new(local.x.max(0.0), local.y.max(0.0)),
                    end: Pos2::new(local.x.max(0.0), local.y.max(0.0)),
                    aspect_ratio: self.crop_aspect,
                });
            }
        }

        // Update crop selection
        if response.dragged() {
            if let (Some(pos), Some(crop)) =
                (response.interact_pointer_pos(), &mut self.crop_selection)
            {
                let local = (pos - image_rect.min) * scale;
                crop.end = Pos2::new(
                    local.x.clamp(0.0, img_size.x),
                    local.y.clamp(0.0, img_size.y),
                );

                // Apply aspect ratio constraint
                if let Some(aspect) = crop.aspect_ratio {
                    let w = (crop.end.x - crop.start.x).abs();
                    let h = w / aspect;
                    crop.end.y = crop.start.y + h * (crop.end.y - crop.start.y).signum();
                }
            }
        }

        // Draw crop overlay
        if let Some(crop) = &self.crop_selection {
            let inv_scale = display_size / img_size;
            let min = Pos2::new(
                crop.start.x.min(crop.end.x) * inv_scale.x,
                crop.start.y.min(crop.end.y) * inv_scale.y,
            );
            let max = Pos2::new(
                crop.start.x.max(crop.end.x) * inv_scale.x,
                crop.start.y.max(crop.end.y) * inv_scale.y,
            );
            let crop_rect = Rect::from_min_max(
                image_rect.min + min.to_vec2(),
                image_rect.min + max.to_vec2(),
            );

            // Darken outside crop area
            let dark = Color32::from_rgba_unmultiplied(0, 0, 0, 150);
            painter.rect_filled(
                Rect::from_min_max(image_rect.min, Pos2::new(image_rect.max.x, crop_rect.min.y)),
                0.0,
                dark,
            );
            painter.rect_filled(
                Rect::from_min_max(Pos2::new(image_rect.min.x, crop_rect.max.y), image_rect.max),
                0.0,
                dark,
            );
            painter.rect_filled(
                Rect::from_min_max(
                    Pos2::new(image_rect.min.x, crop_rect.min.y),
                    Pos2::new(crop_rect.min.x, crop_rect.max.y),
                ),
                0.0,
                dark,
            );
            painter.rect_filled(
                Rect::from_min_max(
                    Pos2::new(crop_rect.max.x, crop_rect.min.y),
                    Pos2::new(image_rect.max.x, crop_rect.max.y),
                ),
                0.0,
                dark,
            );

            // Draw crop border
            painter.rect_stroke(crop_rect, 0.0, Stroke::new(2.0, Color32::WHITE));

            // Draw rule of thirds
            let third_w = crop_rect.width() / 3.0;
            let third_h = crop_rect.height() / 3.0;
            let grid_color = Color32::from_rgba_unmultiplied(255, 255, 255, 100);
            for i in 1..3 {
                painter.line_segment(
                    [
                        Pos2::new(crop_rect.min.x + third_w * i as f32, crop_rect.min.y),
                        Pos2::new(crop_rect.min.x + third_w * i as f32, crop_rect.max.y),
                    ],
                    Stroke::new(1.0, grid_color),
                );
                painter.line_segment(
                    [
                        Pos2::new(crop_rect.min.x, crop_rect.min.y + third_h * i as f32),
                        Pos2::new(crop_rect.max.x, crop_rect.min.y + third_h * i as f32),
                    ],
                    Stroke::new(1.0, grid_color),
                );
            }
        }
    }

    fn render_export_dialog(&mut self, ui: &mut egui::Ui) {
        egui::Window::new("Export Image")
            .collapsible(false)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    ui.label("Format:");
                    egui::ComboBox::from_id_salt("export_format")
                        .selected_text(match self.export_format {
                            ExportFormat::Png => "PNG",
                            ExportFormat::Jpeg => "JPEG",
                            ExportFormat::WebP => "WebP",
                            ExportFormat::Bmp => "BMP",
                            ExportFormat::Tiff => "TIFF",
                            ExportFormat::Gif => "GIF",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.export_format,
                                ExportFormat::Png,
                                "PNG (lossless)",
                            );
                            ui.selectable_value(
                                &mut self.export_format,
                                ExportFormat::Jpeg,
                                "JPEG",
                            );
                            ui.selectable_value(
                                &mut self.export_format,
                                ExportFormat::WebP,
                                "WebP",
                            );
                            ui.selectable_value(&mut self.export_format, ExportFormat::Bmp, "BMP");
                            ui.selectable_value(
                                &mut self.export_format,
                                ExportFormat::Tiff,
                                "TIFF",
                            );
                            ui.selectable_value(&mut self.export_format, ExportFormat::Gif, "GIF");
                        });
                });

                if self.export_format == ExportFormat::Jpeg
                    || self.export_format == ExportFormat::WebP
                {
                    ui.horizontal(|ui| {
                        ui.label("Quality:");
                        ui.add(egui::Slider::new(&mut self.export_quality, 1..=100).suffix("%"));
                    });
                }

                if let Some((w, h)) = self.dimensions() {
                    ui.label(format!("Size: {} x {} pixels", w, h));
                }

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Save As...").clicked() {
                        if let Some(path) = native_dialog::FileDialog::new()
                            .add_filter("Image", &["png", "jpg", "webp", "bmp", "tiff"])
                            .show_save_single_file()
                            .ok()
                            .flatten()
                        {
                            if let Err(e) =
                                self.export(&path, self.export_format, self.export_quality)
                            {
                                tracing::error!("Export failed: {}", e);
                            } else {
                                self.has_unsaved_changes = false;
                                self.show_export = false;
                            }
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_export = false;
                    }
                });
            });
    }
}
