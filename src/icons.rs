// icons.rs — Centralized SVG icon system
//
// Loads SVG files from assets/icons/svg/ at startup, rasterizes them
// via resvg into egui textures, and provides helper methods for
// rendering icon buttons and labels throughout the UI.
//
// All UI glyphs go through this module — no inline Unicode emoji anywhere.

use eframe::egui::{self, ColorImage, TextureHandle, Vec2};
use std::collections::HashMap;

/// Default icon render size (logical pixels).
const DEFAULT_ICON_SIZE: f32 = 16.0;

/// All loaded icon textures, keyed by icon name (e.g. "play", "arrow-left").
pub struct Icons {
    textures: HashMap<String, TextureHandle>,
}

impl Icons {
    /// Load all SVG icons from `assets/icons/svg/` and rasterize to egui textures.
    pub fn load(ctx: &egui::Context) -> Self {
        let mut textures = HashMap::new();
        let svg_dir = std::path::Path::new("assets/icons/svg");

        if let Ok(entries) = std::fs::read_dir(svg_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "svg") {
                    let name = path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();

                    if let Some(handle) = Self::load_svg(ctx, &path, &name) {
                        textures.insert(name, handle);
                    }
                }
            }
        }

        tracing::info!("Loaded {} SVG icons", textures.len());
        Icons { textures }
    }

    /// Rasterize a single SVG file to an egui texture at a fixed pixel size.
    fn load_svg(
        ctx: &egui::Context,
        path: &std::path::Path,
        name: &str,
    ) -> Option<TextureHandle> {
        let svg_data = std::fs::read(path).ok()?;
        let opt = usvg::Options::default();
        let tree = usvg::Tree::from_data(&svg_data, &opt).ok()?;

        // Rasterize at 2x for crispness on HiDPI
        let render_size = 48u32; // 24 logical * 2x
        let mut pixmap = tiny_skia::Pixmap::new(render_size, render_size)?;

        let sx = render_size as f32 / tree.size().width();
        let sy = render_size as f32 / tree.size().height();
        let scale = sx.min(sy);

        let transform = tiny_skia::Transform::from_scale(scale, scale);
        resvg::render(&tree, transform, &mut pixmap.as_mut());

        let pixels = pixmap.data().to_vec();
        let size = [render_size as usize, render_size as usize];
        let color_image = ColorImage::from_rgba_unmultiplied(size, &pixels);
        let handle = ctx.load_texture(
            format!("icon/{}", name),
            color_image,
            egui::TextureOptions::LINEAR,
        );
        Some(handle)
    }

    /// Check if an icon with the given name is loaded.
    pub fn has(&self, name: &str) -> bool {
        self.textures.contains_key(name)
    }

    /// Get a texture handle by icon name.
    pub fn get(&self, name: &str) -> Option<&TextureHandle> {
        self.textures.get(name)
    }

    // ── Rendering helpers ────────────────────────────────────────────

    /// Render an icon image at the default size.
    pub fn image(&self, name: &str) -> Option<egui::Image<'static>> {
        self.image_sized(name, DEFAULT_ICON_SIZE)
    }

    /// Render an icon image at a custom size (logical pixels).
    /// Returns None if the icon is not loaded.
    pub fn image_sized(&self, name: &str, size: f32) -> Option<egui::Image<'static>> {
        let tex = self.textures.get(name)?;
        Some(egui::Image::new((tex.id(), Vec2::splat(size))))
    }

    /// Render a clickable icon button. Returns the Response.
    pub fn button(
        &self,
        ui: &mut egui::Ui,
        name: &str,
        tooltip: &str,
    ) -> egui::Response {
        self.button_sized(ui, name, tooltip, DEFAULT_ICON_SIZE)
    }

    /// Render a clickable icon button at a custom size.
    pub fn button_sized(
        &self,
        ui: &mut egui::Ui,
        name: &str,
        tooltip: &str,
        size: f32,
    ) -> egui::Response {
        if let Some(tex) = self.textures.get(name) {
            ui.add(egui::ImageButton::new((tex.id(), Vec2::splat(size))))
                .on_hover_text(tooltip)
        } else {
            // Fallback: plain text button with the icon name
            ui.button(name).on_hover_text(tooltip)
        }
    }

    /// Render an icon followed by a text label, side by side.
    /// Useful for "Extract All", "Add Files", etc.
    pub fn label_with_icon(
        &self,
        ui: &mut egui::Ui,
        icon_name: &str,
        text: &str,
    ) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            if let Some(tex) = self.textures.get(icon_name) {
                ui.image((tex.id(), Vec2::splat(DEFAULT_ICON_SIZE)));
            }
            ui.label(text);
        });
    }

    /// Create a button that has an icon + text label.
    /// Returns the Response so caller can check `.clicked()`.
    pub fn text_button(
        &self,
        ui: &mut egui::Ui,
        icon_name: &str,
        text: &str,
        tooltip: &str,
    ) -> egui::Response {
        // We use a horizontal layout inside a button-like frame
        let btn = egui::Button::new({
            let mut job = egui::text::LayoutJob::default();
            // We can't easily embed an image in a Button label in egui,
            // so we use the text-only path and prepend a space for the icon.
            // The icon will be rendered separately via ui.horizontal.
            job.append(text, 0.0, egui::TextFormat::default());
            job
        });
        // For now, just return text button — icon rendered adjacently by caller
        ui.button(text).on_hover_text(tooltip)
    }

    /// Convenience: render an icon inline at text-height scale,
    /// matching the surrounding text baseline.
    pub fn inline(&self, ui: &mut egui::Ui, name: &str) {
        if let Some(tex) = self.textures.get(name) {
            let text_height = ui.text_style_height(&egui::TextStyle::Body);
            ui.image((tex.id(), Vec2::splat(text_height)));
        }
    }
}
