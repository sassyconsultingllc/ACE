use crate::ui::render::UIRenderer;
use crate::ui::UI;
use eframe::egui::{self, ColorImage, Vec2};

/// Small egui app that exercises `crate::ui` runtime APIs for manual testing.
pub struct UIMain {
    ui_state: UI,
    renderer: UIRenderer,
    img_texture: Option<egui::TextureHandle>,
}

impl UIMain {
    pub fn new() -> Self {
        let ui_state = UI::new(1200, 800);
        let renderer = UIRenderer::new(800, 600);
        Self {
            ui_state,
            renderer,
            img_texture: None,
        }
    }
}

impl eframe::App for UIMain {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Toggle Theme").clicked() {
                    self.ui_state.toggle_theme();
                }
                if ui.button("New Tab").clicked() {
                    self.ui_state.tab_manager.create_tab("about:blank".into());
                }
                if ui.button("Start Sync").clicked() {
                    let _ = self.ui_state.start_sync(9999);
                }
                if ui.button("Stop Sync").clicked() {
                    self.ui_state.stop_sync();
                }
                ui.label(format!("Tabs: {}", self.ui_state.tab_manager.tab_count()));
            });
        });

        egui::SidePanel::left("left_panel")
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Sidebar (demo)");
                ui.label(format!("Hover: {:?}", self.ui_state.hover_element));
                if ui.button("Toggle Left Sidebar").clicked() {
                    self.ui_state.sidebar_layout.toggle(crate::ui::Edge::Left);
                }
                ui.separator();
                ui.label("Theme:");
                let theme = self.ui_state.theme_manager.current();
                ui.label(&theme.meta.name);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("UI Module Demo");
            ui.label("This simple demo exercises `src/ui` runtime helpers.");

            // Render a small software-drawn preview using UIRenderer into a ColorImage
            let w = 400usize;
            let h = 240usize;
            let mut buffer = vec![0u32; w * h];

            // Use renderer to draw a nav bar and some tab list into buffer
            self.renderer.resize(w as u32, h as u32);
            let theme = self.ui_state.theme_manager.current().clone();
            let state = crate::ui::render::NavBarState {
                can_back: true,
                can_forward: true,
                loading: false,
                show_help_button: true,
                help_enabled: true,
                help_open: false,
            };
            let nav_bounds = crate::ui::Rect {
                x: 0,
                y: 0,
                width: w as u32,
                height: 48,
            };
            self.renderer.draw_nav_bar(
                &mut buffer,
                nav_bounds,
                &theme,
                "https://example.com",
                state,
            );

            // Convert u32 ARGB buffer to RGBA bytes for egui
            let mut pixels: Vec<u8> = Vec::with_capacity(w * h * 4);
            for px in buffer.iter() {
                let v = *px;
                let a = ((v >> 24) & 0xFF) as u8;
                let r = ((v >> 16) & 0xFF) as u8;
                let g = ((v >> 8) & 0xFF) as u8;
                let b = (v & 0xFF) as u8;
                pixels.push(r);
                pixels.push(g);
                pixels.push(b);
                pixels.push(a);
            }

            let image = ColorImage::from_rgba_unmultiplied([w, h], &pixels);
            let tex = ctx.load_texture("ui_preview", image, egui::TextureOptions::default());
            ui.image((tex.id(), Vec2::new(w as f32, h as f32)));

            // Keep texture alive across frames
            self.img_texture = Some(tex);
        });
    }
}

/// Convenience runner for manual testing from other code.
pub fn run_ui() -> Result<(), String> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Sassy UI Demo",
        native_options,
        Box::new(|_cc| Ok(Box::new(UIMain::new()))),
    )
    .map_err(|e| format!("failed to run ui: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construct_ui_main_and_invoke_helpers() {
        let mut app = UIMain::new();

        // Toggle theme via UI state
        let before = app.ui_state.theme_manager.current().meta.name.clone();
        app.ui_state.toggle_theme();
        let after = app.ui_state.theme_manager.current().meta.name.clone();
        assert_ne!(before, after);

        // Create a tab and check count
        let prev = app.ui_state.tab_manager.tab_count();
        app.ui_state
            .tab_manager
            .create_tab("https://example.com".into());
        assert_eq!(app.ui_state.tab_manager.tab_count(), prev + 1);

        // Toggle a sidebar
        app.ui_state.sidebar_layout.toggle(crate::ui::Edge::Left);
    }
}
