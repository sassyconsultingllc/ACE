//! Main application - Browser with egui chrome and wry webview
//! 
//! Architecture:
//! - tao provides the main window
//! - egui renders browser chrome (tabs, address bar, bookmarks bar, status bar)
//! - wry webview handles web content in the content area
//! - egui viewers handle files (PDF, images, documents, etc.) in the content area

use crate::auth::{AuthManager, FirstRunState, FirstRunStep, DeviceType, TailscaleManager};
use crate::browser::{BrowserEngine, Tab, TabContent, TabId};
use crate::file_handler::{FileType, OpenFile};
use crate::html_renderer::HtmlRenderer;
use crate::network_monitor::{NetworkMonitor, ActivityIndicatorState, format_bytes, format_speed};
use crate::password_vault::{PasswordVault, Credential, PasswordGeneratorOptions, generate_password};
use crate::smart_history::{SmartHistory, HistoryEntry};
use crate::family_profiles::{ProfileManager, ProfileType, Profile};
use crate::viewers::{
    archive::ArchiveViewer,
    audio::AudioViewer,
    chemical::ChemicalViewer,
    document::DocumentViewer,
    ebook::EbookViewer,
    font::FontViewer,
    image::ImageViewer,
    model3d::Model3DViewer,
    pdf::PdfViewer,
    spreadsheet::SpreadsheetViewer,
    text::TextViewer,
    video::VideoViewer,
};
use anyhow::Result;
use eframe::egui::{self, Color32, FontId, Key, Modifiers, RichText, Stroke, Vec2};
use eframe::egui::{ColorImage, TextureHandle};
use std::sync::Arc;

/// Browser application state
pub struct BrowserApp {
    engine: BrowserEngine,
    
    // Authentication & licensing
    auth: AuthManager,
    tailscale: TailscaleManager,
    first_run: FirstRunState,
    
    // =========================================================================
    // DISRUPTOR FEATURES - Kills paid software & Chrome bloat
    // =========================================================================
    
    // 📡 Network Activity Monitor - NO HIDDEN TRAFFIC
    network_monitor: NetworkMonitor,
    activity_indicator: ActivityIndicatorState,
    
    // 🔐 Password Vault - Replaces LastPass, 1Password, Chrome passwords
    password_vault: PasswordVault,
    vault_search_query: String,
    vault_panel_visible: bool,
    password_generator_opts: PasswordGeneratorOptions,
    generated_password: String,
    
    // ⏱️ Smart History - 14.7s delay, NSFW detection
    smart_history: SmartHistory,
    history_panel_visible: bool,
    
    // 👨‍👩‍👧‍👦 Family Profiles - Parental controls that work
    profile_manager: ProfileManager,
    profiles_panel_visible: bool,
    
    // Viewers for file types
    image_viewer: ImageViewer,
    pdf_viewer: PdfViewer,
    document_viewer: DocumentViewer,
    text_viewer: TextViewer,
    spreadsheet_viewer: SpreadsheetViewer,
    chemical_viewer: ChemicalViewer,
    archive_viewer: ArchiveViewer,
    model3d_viewer: Model3DViewer,
    font_viewer: FontViewer,
    audio_viewer: AudioViewer,
    video_viewer: VideoViewer,
    ebook_viewer: EbookViewer,
    
    // HTML/JS renderer for web content
    html_renderer: HtmlRenderer,
    // UI icon textures (loaded at startup)
    icons: std::collections::HashMap<String, TextureHandle>,
    
    // UI state
    dark_mode: bool,
    zoom_level: f32,
    show_dev_tools: bool,
    find_bar_visible: bool,
    find_query: String,
    
    // Context menu state
    context_menu_pos: Option<egui::Pos2>,
    context_menu_link: Option<String>,
    
    // Status
    status_message: String,
}

impl BrowserApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_fonts(&cc.egui_ctx);
        configure_style(&cc.egui_ctx, true);
        
        let auth = AuthManager::new();
        let mut tailscale = TailscaleManager::new();
        tailscale.check_installation();
        
        let first_run = if auth.is_first_run {
            FirstRunState::default()
        } else {
            let mut state = FirstRunState::default();
            state.step = FirstRunStep::Complete;
            state
        };
        
        // Get config directory for vault
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("SassyBrowser");
        
        // Load icon textures from assets/icons
        let mut icons: std::collections::HashMap<String, TextureHandle> = std::collections::HashMap::new();
        let ctx = &cc.egui_ctx;
        let icon_files = [
            ("back", "assets/icons/backward.ico"),
            ("forward", "assets/icons/forward.ico"),
            ("bookmarks", "assets/icons/bookmarks.ico"),
            ("app", "assets/icons/icon.svg"),
        ];

        for (key, path) in &icon_files {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(img) = image::load_from_memory(&bytes) {
                    let img = img.to_rgba8();
                    let size = [img.width() as usize, img.height() as usize];
                    let pixels: Vec<u8> = img.into_raw();
                    let color_image = ColorImage::from_rgba_unmultiplied(size, &pixels);
                    let handle = ctx.load_texture(*path, color_image, egui::TextureOptions::default());
                    icons.insert((*key).to_string(), handle);
                }
            }
        }

        Self {
            engine: BrowserEngine::new(),
            auth,
            tailscale,
            first_run,
            
            // Disruptor features
            network_monitor: NetworkMonitor::new(),
            activity_indicator: ActivityIndicatorState::default(),
            password_vault: PasswordVault::new(config_dir.clone()),
            vault_search_query: String::new(),
            vault_panel_visible: false,
            password_generator_opts: PasswordGeneratorOptions::default(),
            generated_password: String::new(),
            smart_history: SmartHistory::new(),
            history_panel_visible: false,
            profile_manager: ProfileManager::new(),
            profiles_panel_visible: false,
            
            // Viewers
            image_viewer: ImageViewer::new(),
            pdf_viewer: PdfViewer::new(),
            document_viewer: DocumentViewer::new(),
            text_viewer: TextViewer::new(),
            spreadsheet_viewer: SpreadsheetViewer::new(),
            chemical_viewer: ChemicalViewer::new(),
            archive_viewer: ArchiveViewer::new(),
            model3d_viewer: Model3DViewer::new(),
            font_viewer: FontViewer::new(),
            audio_viewer: AudioViewer::new(),
            video_viewer: VideoViewer::new(),
            ebook_viewer: EbookViewer::new(),
            html_renderer: HtmlRenderer::new(),
            icons,
            dark_mode: true,
            zoom_level: 1.0,
            show_dev_tools: false,
            find_bar_visible: false,
            find_query: String::new(),
            context_menu_pos: None,
            context_menu_link: None,
            status_message: "Ready".into(),
        }
    }
    
    fn render_toolbar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            
            let tab = self.engine.active_tab();
            let (can_back, can_forward, is_loading) = match tab {
                Some(Tab { content: TabContent::Web { can_go_back, can_go_forward, loading, .. }, .. }) => {
                    (*can_go_back, *can_go_forward, *loading)
                }
                _ => (false, false, false),
            };
            
            // Navigation buttons (use loaded icons when available)
            if let Some(tex) = self.icons.get("back") {
                if ui.add_enabled(can_back, egui::ImageButton::new((tex.id(), Vec2::new(28.0, 24.0))))
                    .on_hover_text("Back (Alt+Left)")
                    .clicked() {
                    self.engine.go_back();
                }
            } else {
                if ui.add_enabled(can_back, egui::Button::new("◀").min_size(Vec2::new(28.0, 24.0)))
                    .on_hover_text("Back (Alt+Left)")
                    .clicked() {
                    self.engine.go_back();
                }
            }

            if let Some(tex) = self.icons.get("forward") {
                if ui.add_enabled(can_forward, egui::ImageButton::new((tex.id(), Vec2::new(28.0, 24.0))))
                    .on_hover_text("Forward (Alt+Right)")
                    .clicked() {
                    self.engine.go_forward();
                }
            } else {
                if ui.add_enabled(can_forward, egui::Button::new("▶").min_size(Vec2::new(28.0, 24.0)))
                    .on_hover_text("Forward (Alt+Right)")
                    .clicked() {
                    self.engine.go_forward();
                }
            }
            
            if is_loading {
                if ui.button("✕").on_hover_text("Stop").clicked() {
                    self.engine.stop();
                }
            } else {
                if ui.button("↻").on_hover_text("Reload (F5)").clicked() {
                    self.engine.reload();
                }
            }
            
            if ui.button("🏠").on_hover_text("Home").clicked() {
                self.engine.go_home();
            }
            
            // Address bar
            let address_width = ui.available_width() - 150.0;
            
            ui.scope(|ui| {
                ui.set_min_width(address_width);
                
                let is_secure = match self.engine.active_tab() {
                    Some(Tab { content: TabContent::Web { is_secure, .. }, .. }) => *is_secure,
                    _ => false,
                };
                
                // Security indicator
                if is_secure {
                    ui.colored_label(Color32::from_rgb(100, 200, 100), "🔒");
                }
                
                let mut text = self.engine.address_bar_text().to_string();
                let response = ui.add(
                    egui::TextEdit::singleline(&mut text)
                        .desired_width(address_width - 40.0)
                        .font(FontId::proportional(14.0))
                        .hint_text("Search or enter URL")
                );
                
                if response.changed() {
                    self.engine.set_address_bar_text(text);
                }
                
                if response.gained_focus() {
                    self.engine.set_address_bar_focused(true);
                }
                
                if response.lost_focus() {
                    self.engine.set_address_bar_focused(false);
                    if ctx.input(|i| i.key_pressed(Key::Enter)) {
                        self.engine.submit_address_bar();
                    }
                }
            });
            
            // Bookmark button
            let is_bookmarked = self.engine.is_current_page_bookmarked();
            let bookmark_icon = if is_bookmarked { "★" } else { "☆" };
            if ui.button(bookmark_icon).on_hover_text("Bookmark this page").clicked() {
                self.engine.toggle_bookmark();
            }
            
            // Menu button
            ui.menu_button("☰", |ui| {
                if ui.button("📂 Open File...").clicked() {
                    self.open_file_dialog();
                    ui.close_menu();
                }
                ui.separator();
                // Bookmarks menu button: prefer image icon if available
                if let Some(tex) = self.icons.get("bookmarks") {
                    if ui.add(egui::ImageButton::new((tex.id(), Vec2::new(18.0,18.0))))
                        .on_hover_text("Bookmarks")
                        .clicked() {
                        self.engine.navigate("sassy://bookmarks");
                        ui.close_menu();
                    }
                } else {
                    if ui.button("⭐ Bookmarks").clicked() {
                        self.engine.navigate("sassy://bookmarks");
                        ui.close_menu();
                    }
                }
                if ui.button("🕐 History").clicked() {
                    self.engine.navigate("sassy://history");
                    ui.close_menu();
                }
                if ui.button("⬇️ Downloads").clicked() {
                    self.engine.set_show_downloads_panel(!self.engine.show_downloads_panel());
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("🔍 Find in Page (Ctrl+F)").clicked() {
                    self.find_bar_visible = true;
                    ui.close_menu();
                }
                if ui.button("🖨️ Print (Ctrl+P)").clicked() {
                    self.print_current();
                    ui.close_menu();
                }
                ui.separator();
                if ui.checkbox(self.engine.show_bookmarks_bar_mut(), "Show Bookmarks Bar").changed() {
                    ui.close_menu();
                }
                if ui.checkbox(&mut self.dark_mode, "🌙 Dark Mode").clicked() {
                    configure_style(ctx, self.dark_mode);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("⚙️ Settings").clicked() {
                    self.engine.navigate("sassy://settings");
                    ui.close_menu();
                }
            });
        });
    }
    
    fn render_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let tabs: Vec<(TabId, String, String, bool, bool)> = self.engine.tabs()
                .iter()
                .map(|t| (t.id, t.icon().to_string(), t.title(), t.is_loading(), t.pinned))
                .collect();
            
            let active_idx = self.engine.active_tab_index();
            let mut close_tab: Option<usize> = None;
            
            for (idx, (id, icon, title, loading, pinned)) in tabs.iter().enumerate() {
                let is_active = idx == active_idx;
                
                // Tab styling: use the current visuals/panel fills instead of hardcoded grays
                let visuals = ui.ctx().style().visuals.clone();
                let bg_color = if is_active {
                    visuals.widgets.active.bg_fill
                } else {
                    visuals.widgets.inactive.bg_fill
                };
                
                egui::Frame::none()
                    .fill(bg_color)
                    .rounding(egui::Rounding { nw: 4.0, ne: 4.0, sw: 0.0, se: 0.0 })
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Loading indicator or icon
                            if *loading {
                                ui.spinner();
                            } else {
                                ui.label(icon);
                            }
                            
                            // Title (truncated)
                            let max_title_len = if *pinned { 0 } else { 20 };
                            let display_title = if title.len() > max_title_len && max_title_len > 0 {
                                format!("{}...", &title[..max_title_len.min(title.len())])
                            } else if max_title_len == 0 {
                                String::new()
                            } else {
                                title.clone()
                            };
                            
                            let tab_response = ui.selectable_label(is_active, &display_title);
                            
                            if tab_response.clicked() {
                                self.engine.set_active_tab(idx);
                            }
                            
                            // Middle click to close
                            if tab_response.middle_clicked() {
                                close_tab = Some(idx);
                            }
                            
                            // Context menu
                            tab_response.context_menu(|ui| {
                                if ui.button("New Tab").clicked() {
                                    self.engine.new_tab();
                                    ui.close_menu();
                                }
                                if ui.button("Duplicate Tab").clicked() {
                                    self.engine.duplicate_tab(idx);
                                    ui.close_menu();
                                }
                                ui.separator();
                                let pin_text = if *pinned { "Unpin Tab" } else { "Pin Tab" };
                                if ui.button(pin_text).clicked() {
                                    self.engine.toggle_pin(idx);
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button("Close Tab").clicked() {
                                    close_tab = Some(idx);
                                    ui.close_menu();
                                }
                                if ui.button("Close Other Tabs").clicked() {
                                    // Close all tabs except this one
                                    let current_id = *id;
                                    for i in (0..self.engine.tab_count()).rev() {
                                        if self.engine.tabs()[i].id != current_id {
                                            self.engine.close_tab(i);
                                        }
                                    }
                                    ui.close_menu();
                                }
                            });
                            
                            // Close button (not for pinned tabs)
                            if !*pinned {
                                if ui.small_button("×").clicked() {
                                    close_tab = Some(idx);
                                }
                            }
                        });
                    });
                
                ui.add_space(2.0);
            }
            
            // New tab button
            if ui.button("＋").on_hover_text("New Tab (Ctrl+T)").clicked() {
                self.engine.new_tab();
            }
            
            // Handle tab close
            if let Some(idx) = close_tab {
                self.engine.close_tab(idx);
            }
        });
    }
    
    fn render_bookmarks_bar(&mut self, ui: &mut egui::Ui) {
        if !self.engine.show_bookmarks_bar() {
            return;
        }
        
        ui.horizontal(|ui| {
            let bookmarks: Vec<_> = self.engine.bookmarks.bookmarks_bar()
                .into_iter()
                .map(|b| (b.url.clone(), b.title.clone()))
                .collect();
            
            let is_empty = bookmarks.is_empty();
            
            for (url, title) in &bookmarks {
                let display = if title.len() > 15 {
                    format!("{}...", &title[..15])
                } else {
                    title.clone()
                };
                
                if ui.button(&display).on_hover_text(url).clicked() {
                    self.engine.navigate(url);
                }
            }
            
            if is_empty {
                ui.label(RichText::new("Bookmarks bar is empty").italics().color(Color32::from_rgb(160,160,160)));
            }
        });
    }
    
    fn render_find_bar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        if !self.find_bar_visible {
            return;
        }
        
        ui.horizontal(|ui| {
            ui.label("Find:");
            
            let response = ui.text_edit_singleline(&mut self.find_query);
            
            if response.lost_focus() && ctx.input(|i| i.key_pressed(Key::Escape)) {
                self.find_bar_visible = false;
            }
            
            if ui.button("Find").clicked() || 
               (response.lost_focus() && ctx.input(|i| i.key_pressed(Key::Enter))) {
                // TODO: Implement find in webview
            }
            
            if ui.button("×").clicked() {
                self.find_bar_visible = false;
            }
        });
    }
    
    fn render_content(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        // Extract content info first to avoid borrow conflicts
        let content_type = self.engine.active_tab().map(|t| match &t.content {
            TabContent::NewTab => ("newtab", String::new()),
            TabContent::Settings => ("settings", String::new()),
            TabContent::History => ("history", String::new()),
            TabContent::Bookmarks => ("bookmarks", String::new()),
            TabContent::Downloads => ("downloads", String::new()),
            TabContent::Web { url, .. } => ("web", url.clone()),
            TabContent::File(_) => ("file", String::new()),
        });
        
        match content_type {
            Some(("newtab", _)) => self.render_new_tab_page(ui),
            Some(("settings", _)) => self.render_settings_page(ui),
            Some(("history", _)) => self.render_history_page(ui),
            Some(("bookmarks", _)) => self.render_bookmarks_page(ui),
            Some(("downloads", _)) => self.render_downloads_page(ui),
            Some(("web", url)) => self.render_web_content(ctx, ui, &url),
            Some(("file", _)) => {
                // Get file reference carefully
                if let Some(tab) = self.engine.active_tab() {
                    if let TabContent::File(file) = &tab.content {
                        let file_type = file.file_type;
                        let zoom = self.zoom_level;
                        // Clone necessary data to avoid borrow issues
                        let file_clone = file.clone();
                        drop(tab);
                        match file_type {
                            FileType::Image | FileType::ImageRaw | FileType::ImagePsd => {
                                self.image_viewer.render(ui, &file_clone, zoom)
                            }
                            FileType::Pdf => self.pdf_viewer.render(ui, &file_clone, zoom),
                            FileType::Document => self.document_viewer.render(ui, &file_clone, zoom),
                            FileType::Spreadsheet => self.spreadsheet_viewer.render(ui, &file_clone, zoom),
                            FileType::Chemical => self.chemical_viewer.render(ui, &file_clone, zoom),
                            FileType::Archive => self.archive_viewer.render(ui, &file_clone, zoom),
                            FileType::Model3D => self.model3d_viewer.render(ui, &file_clone, zoom),
                            FileType::Font => self.font_viewer.render(ui, &file_clone, zoom),
                            FileType::Audio => self.audio_viewer.render(ui, &file_clone, zoom),
                            FileType::Video => self.video_viewer.render(ui, &file_clone, zoom),
                            FileType::Ebook => self.ebook_viewer.render(ui, &file_clone, zoom),
                            FileType::Markdown => self.text_viewer.render(ui, &file_clone, zoom),
                            FileType::Text | FileType::Unknown => self.text_viewer.render(ui, &file_clone, zoom),
                        }
                    }
                }
            }
            _ => {
                ui.centered_and_justified(|ui| {
                    ui.label("No tab selected");
                });
            }
        }
    }
    
    fn render_new_tab_page(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                
                // Logo
                ui.heading(RichText::new("🌐 Sassy Browser").size(48.0));
                ui.add_space(10.0);
                ui.label(RichText::new("Fast • Free • Handles Everything").size(16.0).color(Color32::GRAY));
                
                ui.add_space(40.0);
                
                // Quick access - Most visited
                ui.heading("Most Visited");
                ui.add_space(10.0);
                
                ui.horizontal_wrapped(|ui| {
                    let most_visited: Vec<_> = self.engine.history.most_visited(8)
                        .into_iter()
                        .map(|h| (h.url.clone(), h.title.clone()))
                        .collect();
                    
                    for (url, title) in most_visited {
                        let display = if title.is_empty() {
                            url::Url::parse(&url)
                                .ok()
                                .and_then(|u| u.host_str().map(String::from))
                                .unwrap_or_else(|| "Site".into())
                        } else if title.len() > 20 {
                            format!("{}...", &title[..20])
                        } else {
                            title
                        };
                        
                        if ui.button(RichText::new(&display).size(14.0)).clicked() {
                            self.engine.navigate(&url);
                        }
                    }
                });
                
                ui.add_space(30.0);
                
                // Supported formats info
                ui.heading("Native File Support - 100+ Formats");
                ui.add_space(10.0);
                
                ui.horizontal_wrapped(|ui| {
                    ui.label("🖼️ Images (PNG, JPG, RAW, PSD, SVG)");
                    ui.label("📄 PDF");
                    ui.label("📝 Office Docs (DOCX, ODT, RTF)");
                    ui.label("📊 Spreadsheets (XLSX, CSV)");
                });
                
                ui.add_space(8.0);
                
                ui.horizontal_wrapped(|ui| {
                    ui.label("🧬 Chemical (PDB, MOL, XYZ, CIF)");
                    ui.label("📦 Archives (ZIP, 7z, RAR, TAR)");
                    ui.label("🎲 3D Models (OBJ, STL, GLTF)");
                    ui.label("🔤 Fonts (TTF, OTF, WOFF)");
                });
                
                ui.add_space(8.0);
                
                ui.horizontal_wrapped(|ui| {
                    ui.label("🎵 Audio (MP3, FLAC, WAV, OGG)");
                    ui.label("🎬 Video (MP4, MKV, WebM)");
                    ui.label("📚 eBooks (EPUB, MOBI)");
                    ui.label("💻 Code (100+ languages)");
                });
                
                ui.add_space(20.0);
                ui.label(RichText::new("Drag & drop any file or use File → Open").italics().color(Color32::GRAY));
            });
        });
    }
    
    fn render_web_content(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, url: &str) {
        let url = url.to_string();
        let available = ui.available_size();
        
        // Dark/light mode background
        let bg = if self.dark_mode { Color32::from_gray(25) } else { Color32::WHITE };
        let text_color = if self.dark_mode { Color32::WHITE } else { Color32::BLACK };
        
        egui::Frame::none()
            .fill(bg)
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.set_min_size(available);
                
                // Render HTML with our JS interpreter
                self.html_renderer.render(ui, &url);
                
                // Handle clicked links
                if let Some(href) = self.html_renderer.take_clicked_link() {
                    self.engine.navigate(&href);
                    self.html_renderer.clear_cache();
                }
                
                // Show console output if dev tools enabled
                if self.show_dev_tools {
                    ui.separator();
                    ui.collapsing("📝 Console", |ui| {
                        for line in self.html_renderer.console_output() {
                            ui.label(RichText::new(line).monospace().size(12.0));
                        }
                    });
                }
                
                // Show option to open in system browser
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("💡 Tip:").small().color(Color32::GRAY));
                    if ui.small_button("Open in system browser").clicked() {
                        let _ = open::that(&url);
                    }
                    ui.label(RichText::new("for full web experience").small().color(Color32::GRAY));
                });
            });
    }
    
    fn render_file_content(&mut self, ui: &mut egui::Ui, file: &OpenFile) {
        match file.file_type {
            FileType::Image | FileType::ImageRaw | FileType::ImagePsd => {
                self.image_viewer.render(ui, file, self.zoom_level)
            }
            FileType::Pdf => self.pdf_viewer.render(ui, file, self.zoom_level),
            FileType::Document => self.document_viewer.render(ui, file, self.zoom_level),
            FileType::Spreadsheet => self.spreadsheet_viewer.render(ui, file, self.zoom_level),
            FileType::Chemical => self.chemical_viewer.render(ui, file, self.zoom_level),
            FileType::Archive => self.archive_viewer.render(ui, file, self.zoom_level),
            FileType::Model3D => self.model3d_viewer.render(ui, file, self.zoom_level),
            FileType::Font => self.font_viewer.render(ui, file, self.zoom_level),
            FileType::Audio => self.audio_viewer.render(ui, file, self.zoom_level),
            FileType::Video => self.video_viewer.render(ui, file, self.zoom_level),
            FileType::Ebook => self.ebook_viewer.render(ui, file, self.zoom_level),
            FileType::Markdown => self.text_viewer.render(ui, file, self.zoom_level),
            FileType::Text | FileType::Unknown => self.text_viewer.render(ui, file, self.zoom_level),
        }
    }
    
    fn render_settings_page(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("⚙️ Settings");
            ui.separator();
            
            ui.add_space(20.0);
            
            // Appearance
            ui.heading("Appearance");
            ui.checkbox(&mut self.dark_mode, "Dark Mode");
            
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label("Zoom:");
                if ui.button("-").clicked() {
                    self.zoom_level = (self.zoom_level - 0.1).max(0.5);
                }
                ui.label(format!("{:.0}%", self.zoom_level * 100.0));
                if ui.button("+").clicked() {
                    self.zoom_level = (self.zoom_level + 0.1).min(2.0);
                }
                if ui.button("Reset").clicked() {
                    self.zoom_level = 1.0;
                }
            });
            
            ui.add_space(20.0);
            
            // Search
            ui.heading("Search");
            ui.horizontal(|ui| {
                ui.label("Search Engine:");
                // TODO: Dropdown with search engine options
                ui.label(&self.engine.search_engine);
            });
            
            ui.add_space(20.0);
            
            // Downloads
            ui.heading("Downloads");
            ui.horizontal(|ui| {
                ui.label("Download Location:");
                ui.label(self.engine.downloads.download_dir().display().to_string());
                if ui.button("Change...").clicked() {
                    // TODO: Folder picker
                }
            });
            
            ui.add_space(20.0);
            
            // Privacy
            ui.heading("Privacy");
            if ui.button("Clear Browsing Data...").clicked() {
                // TODO: Clear data dialog
            }
            
            ui.add_space(20.0);
            
            // About
            ui.heading("About");
            ui.label("Sassy Browser v2.0.0");
            ui.label("The ultimate free web browser & universal file viewer");
            ui.label("Supports 100+ file formats with zero paid dependencies");
            ui.hyperlink_to("GitHub", "https://github.com/yourusername/sassy-browser");
        });
    }
    
    fn render_history_page(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🕐 History");
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Clear All History").clicked() {
                        self.engine.history.clear();
                    }
                });
            });
            
            ui.separator();
            
            let history: Vec<_> = self.engine.history.recent(100)
                .into_iter()
                .map(|h| (h.url.clone(), h.title.clone(), h.visit_count))
                .collect();
            
            let is_empty = history.is_empty();
            
            for (url, title, visits) in &history {
                ui.horizontal(|ui| {
                    let display_title = if title.is_empty() { url } else { title };
                    
                    if ui.link(display_title).clicked() {
                        self.engine.navigate(url);
                    }
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("{} visits", visits)).small().color(Color32::GRAY));
                    });
                });
            }
            
            if is_empty {
                ui.label(RichText::new("No history yet").italics().color(Color32::GRAY));
            }
        });
    }
    
    fn render_bookmarks_page(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("⭐ Bookmarks");
            ui.separator();
            
            let bookmarks: Vec<_> = self.engine.bookmarks.all()
                .iter()
                .map(|b| (b.id, b.url.clone(), b.title.clone()))
                .collect();
            
            let is_empty = bookmarks.is_empty();
            let mut remove_id = None;
            
            for (id, url, title) in &bookmarks {
                ui.horizontal(|ui| {
                    if ui.link(title).clicked() {
                        self.engine.navigate(url);
                    }
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("×").clicked() {
                            remove_id = Some(*id);
                        }
                        ui.label(RichText::new(url).color(Color32::from_rgb(150,150,150)));
                    });
                });
            }
            
            // Remove bookmark after iteration
            if let Some(id) = remove_id {
                self.engine.bookmarks.remove(id);
            }
            
            if is_empty {
                ui.label(RichText::new("No bookmarks yet").italics().color(Color32::from_rgb(160,160,160)));
            }
        });
    }
    
    fn render_downloads_page(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("⬇️ Downloads");
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Clear Completed").clicked() {
                        self.engine.downloads.clear_finished();
                    }
                });
            });
            
            ui.separator();
            
            let downloads = self.engine.downloads.downloads();
            
            for download in &downloads {
                ui.horizontal(|ui| {
                    ui.label(&download.filename);
                    
                    match download.state {
                        crate::browser::DownloadState::Downloading => {
                            ui.add(egui::ProgressBar::new(download.progress()).show_percentage());
                            if ui.small_button("Cancel").clicked() {
                                self.engine.downloads.cancel_download(download.id);
                            }
                        }
                        crate::browser::DownloadState::Completed => {
                            ui.label(RichText::new("✓ Complete").color(Color32::GREEN));
                            if ui.small_button("Open").clicked() {
                                let _ = open::that(&download.save_path);
                            }
                        }
                        crate::browser::DownloadState::Failed => {
                            ui.label(RichText::new("✗ Failed").color(Color32::RED));
                        }
                        _ => {
                            ui.label(format!("{:?}", download.state));
                        }
                    }
                });
            }
            
            if downloads.is_empty() {
                ui.label(RichText::new("No downloads").italics().color(Color32::GRAY));
            }
        });
    }
    
    fn render_downloads_panel(&mut self, ctx: &egui::Context) {
        if !self.engine.show_downloads_panel() {
            return;
        }
        
        egui::TopBottomPanel::bottom("downloads_panel")
            .resizable(true)
            .default_height(150.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Downloads");
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("×").clicked() {
                            self.engine.set_show_downloads_panel(false);
                        }
                    });
                });
                
                ui.separator();
                
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let downloads = self.engine.downloads.downloads();
                    
                    for download in &downloads {
                        ui.horizontal(|ui| {
                            ui.label(&download.filename);
                            
                            match download.state {
                                crate::browser::DownloadState::Downloading => {
                                    ui.add(egui::ProgressBar::new(download.progress())
                                        .desired_width(200.0)
                                        .show_percentage());
                                }
                                crate::browser::DownloadState::Completed => {
                                    ui.label(RichText::new("✓").color(Color32::GREEN));
                                    if ui.small_button("Open").clicked() {
                                        let _ = open::that(&download.save_path);
                                    }
                                }
                                _ => {}
                            }
                        });
                    }
                });
            });
    }
    
    fn render_status_bar(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(22.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(&self.status_message);
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("Zoom: {:.0}%", self.zoom_level * 100.0));
                        ui.separator();
                        ui.label(format!("{} tabs", self.engine.tab_count()));
                    });
                });
            });
    }
    
    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            // Ctrl shortcuts
            if i.modifiers.ctrl {
                if i.key_pressed(Key::T) {
                    self.engine.new_tab();
                }
                if i.key_pressed(Key::W) {
                    self.engine.close_tab(self.engine.active_tab_index());
                }
                if i.key_pressed(Key::L) || i.key_pressed(Key::K) {
                    self.engine.set_address_bar_focused(true);
                }
                if i.key_pressed(Key::R) {
                    self.engine.reload();
                }
                if i.key_pressed(Key::F) {
                    self.find_bar_visible = true;
                }
                if i.key_pressed(Key::D) {
                    self.engine.toggle_bookmark();
                }
                if i.key_pressed(Key::H) {
                    self.engine.navigate("sassy://history");
                }
                if i.key_pressed(Key::J) {
                    self.engine.set_show_downloads_panel(!self.engine.show_downloads_panel());
                }
                if i.key_pressed(Key::O) {
                    self.open_file_dialog();
                }
                if i.key_pressed(Key::P) {
                    self.print_current();
                }
                if i.key_pressed(Key::Equals) || i.key_pressed(Key::Plus) {
                    self.zoom_level = (self.zoom_level + 0.1).min(3.0);
                }
                if i.key_pressed(Key::Minus) {
                    self.zoom_level = (self.zoom_level - 0.1).max(0.25);
                }
                if i.key_pressed(Key::Num0) {
                    self.zoom_level = 1.0;
                }
                
                // Tab switching with Ctrl+1-9
                for (idx, key) in [Key::Num1, Key::Num2, Key::Num3, Key::Num4, 
                                   Key::Num5, Key::Num6, Key::Num7, Key::Num8, Key::Num9].iter().enumerate() {
                    if i.key_pressed(*key) {
                        if idx == 8 {
                            // Ctrl+9 goes to last tab
                            self.engine.set_active_tab(self.engine.tab_count() - 1);
                        } else if idx < self.engine.tab_count() {
                            self.engine.set_active_tab(idx);
                        }
                    }
                }
                
                // Ctrl+Tab / Ctrl+Shift+Tab for tab navigation
                if i.key_pressed(Key::Tab) {
                    let current = self.engine.active_tab_index();
                    let count = self.engine.tab_count();
                    if i.modifiers.shift {
                        self.engine.set_active_tab(if current == 0 { count - 1 } else { current - 1 });
                    } else {
                        self.engine.set_active_tab((current + 1) % count);
                    }
                }
            }
            
            // Alt shortcuts
            if i.modifiers.alt {
                if i.key_pressed(Key::ArrowLeft) {
                    self.engine.go_back();
                }
                if i.key_pressed(Key::ArrowRight) {
                    self.engine.go_forward();
                }
                if i.key_pressed(Key::Home) {
                    self.engine.go_home();
                }
            }
            
            // Function keys
            if i.key_pressed(Key::F5) {
                self.engine.reload();
            }
            if i.key_pressed(Key::F11) {
                // TODO: Toggle fullscreen
            }
            if i.key_pressed(Key::F12) {
                self.show_dev_tools = !self.show_dev_tools;
            }
            
            // Escape
            if i.key_pressed(Key::Escape) {
                self.find_bar_visible = false;
                self.engine.stop();
            }
        });
    }
    
    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            for file in &i.raw.dropped_files {
                if let Some(path) = &file.path {
                    if let Ok(id) = self.engine.open_file(path.clone()) {
                        self.status_message = format!("Opened: {}", path.display());
                    }
                }
            }
        });
    }
    
    fn open_file_dialog(&mut self) {
        if let Some(path) = native_dialog::FileDialog::new()
            .set_title("Open File")
            .add_filter("All Supported", &[
                // Images
                "png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "avif", "ico", "tiff", "tif",
                "tga", "hdr", "exr", "pnm", "qoi", "dds", "psd", "xcf",
                // RAW camera
                "cr2", "cr3", "nef", "arw", "dng", "raf", "orf", "rw2", "pef", "srw", "raw",
                // Documents
                "pdf",
                "docx", "doc", "odt", "rtf", "wpd",
                "xlsx", "xls", "ods", "csv", "tsv",
                // Chemical/Scientific
                "pdb", "mol", "sdf", "xyz", "cif", "mol2", "mmcif",
                // Archives
                "zip", "tar", "gz", "tgz", "bz2", "xz", "7z", "rar", "zst",
                // 3D Models
                "obj", "stl", "gltf", "glb", "ply", "fbx", "dae", "3ds",
                // Fonts
                "ttf", "otf", "woff", "woff2", "eot", "fon",
                // Audio
                "mp3", "flac", "wav", "ogg", "m4a", "aac", "wma", "opus", "aiff",
                // Video
                "mp4", "mkv", "webm", "avi", "mov", "wmv", "flv", "m4v", "ogv",
                // eBooks
                "epub", "mobi", "azw", "azw3", "fb2",
                // Code/Text
                "txt", "md", "rs", "py", "js", "ts", "html", "css", "json", "xml", "yaml", "yml",
                "c", "cpp", "h", "hpp", "java", "go", "rb", "php", "swift", "kt", "lua", "sh",
                "bat", "ps1", "sql", "toml", "ini", "cfg", "log", "tex", "bib",
            ])
            .add_filter("Images", &[
                "png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "avif", "ico", "tiff", 
                "psd", "cr2", "nef", "arw", "dng",
            ])
            .add_filter("Documents", &["pdf", "docx", "doc", "odt", "rtf", "xlsx", "xls", "csv"])
            .add_filter("Archives", &["zip", "tar", "gz", "7z", "rar"])
            .add_filter("3D Models", &["obj", "stl", "gltf", "glb", "ply"])
            .add_filter("Audio", &["mp3", "flac", "wav", "ogg", "m4a", "aac"])
            .add_filter("Video", &["mp4", "mkv", "webm", "avi", "mov"])
            .add_filter("eBooks", &["epub", "mobi", "azw3"])
            .add_filter("Scientific", &["pdb", "mol", "sdf", "xyz", "cif"])
            .add_filter("Code", &["rs", "py", "js", "ts", "c", "cpp", "java", "go"])
            .show_open_single_file()
            .ok()
            .flatten()
        {
            if let Ok(_) = self.engine.open_file(path.clone()) {
                self.status_message = format!("Opened: {}", path.display());
            }
        }
    }
    
    fn print_current(&mut self) {
        // TODO: Implement printing
        self.status_message = "Print functionality coming soon".into();
    }
    
    // =========================================================================
    // FIRST RUN WIZARD
    // =========================================================================
    
    fn render_first_run_wizard(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                
                // Header
                ui.heading(RichText::new("🦊 Sassy Browser").size(48.0).color(Color32::from_rgb(255, 140, 0)));
                ui.add_space(10.0);
                ui.label(RichText::new("Pure Rust • No Chrome • No Google • No Tracking").size(16.0).color(Color32::GRAY));
                ui.add_space(40.0);
                
                // Progress indicator
                ui.horizontal(|ui| {
                    let steps = ["Welcome", "Security", "Device", "Tailscale", "Phone", "Done"];
                    let current = match self.first_run.step {
                        FirstRunStep::Welcome => 0,
                        FirstRunStep::EntropyCollection => 1,
                        FirstRunStep::DeviceSetup => 2,
                        FirstRunStep::TailscaleSetup => 3,
                        FirstRunStep::PhonePairing => 4,
                        FirstRunStep::Complete => 5,
                    };
                    
                    for (i, step) in steps.iter().enumerate() {
                        let color = if i < current {
                            Color32::from_rgb(0, 200, 100) // Completed
                        } else if i == current {
                            Color32::from_rgb(255, 140, 0)  // Current
                        } else {
                            Color32::GRAY                   // Future
                        };
                        
                        ui.label(RichText::new(format!("{}. {}", i + 1, step)).color(color));
                        if i < steps.len() - 1 {
                            ui.label(RichText::new(" → ").color(Color32::DARK_GRAY));
                        }
                    }
                });
                
                ui.add_space(40.0);
                ui.separator();
                ui.add_space(30.0);
                
                // Step content
                match self.first_run.step {
                    FirstRunStep::Welcome => self.render_wizard_welcome(ui),
                    FirstRunStep::EntropyCollection => self.render_wizard_entropy(ui),
                    FirstRunStep::DeviceSetup => self.render_wizard_device(ui),
                    FirstRunStep::TailscaleSetup => self.render_wizard_tailscale(ui),
                    FirstRunStep::PhonePairing => self.render_wizard_phone(ui),
                    FirstRunStep::Complete => {}
                }
            });
        });
    }
    
    fn render_wizard_welcome(&mut self, ui: &mut egui::Ui) {
        ui.heading("Welcome to Sassy Browser");
        ui.add_space(20.0);
        
        ui.label("This browser is different. Here's why:");
        ui.add_space(10.0);
        
        egui::Frame::dark_canvas(ui.style()).show(ui, |ui| {
            ui.set_min_width(600.0);
            ui.vertical(|ui| {
                ui.add_space(10.0);
                let features = [
                    ("🔒", "100% Pure Rust", "No Chrome, no WebKit, no Google telemetry"),
                    ("📁", "200+ File Formats", "PDF, PDB, RAW photos, CAD files - all built-in"),
                    ("💰", "Kills Paid Software", "Adobe Suite ($504/yr), AutoCAD ($2K/yr) - FREE"),
                    ("🔗", "Tailscale Mesh", "Sync across all your devices securely"),
                    ("📱", "Phone App", "Pair your phone for seamless sync"),
                ];
                
                for (icon, title, desc) in features {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(icon).size(24.0));
                        ui.add_space(10.0);
                        ui.vertical(|ui| {
                            ui.label(RichText::new(title).strong().size(16.0));
                            ui.label(RichText::new(desc).color(Color32::GRAY));
                        });
                    });
                    ui.add_space(8.0);
                }
                ui.add_space(10.0);
            });
        });
        
        ui.add_space(30.0);
        
        if ui.button(RichText::new("Get Started →").size(18.0)).clicked() {
            self.first_run.next_step();
        }
    }
    
    fn render_wizard_entropy(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔐 Creating Your Security Key");
        ui.add_space(20.0);
        
        ui.label("Move your mouse around to generate cryptographic entropy.");
        ui.label("This creates a unique 256-bit key that stays on YOUR device.");
        ui.add_space(20.0);
        
        // Progress bar
        let progress = self.auth.entropy_progress();
        let progress_bar = egui::ProgressBar::new(progress)
            .desired_width(400.0)
            .show_percentage()
            .animate(true);
        ui.add(progress_bar);
        
        ui.add_space(10.0);
        
        let bits = (progress * 256.0) as u32;
        let color = if bits >= 256 {
            Color32::from_rgb(0, 200, 100)
        } else if bits >= 128 {
            Color32::YELLOW
        } else {
            Color32::from_rgb(255, 140, 0)
        };
        
        ui.label(RichText::new(format!("Entropy: {} / 256 bits", bits)).color(color));
        
        ui.add_space(20.0);
        
        // Visual entropy display
        egui::Frame::dark_canvas(ui.style()).show(ui, |ui| {
            ui.set_min_size(Vec2::new(400.0, 100.0));
            
            // Show some "randomness" visualization
            ui.horizontal_wrapped(|ui| {
                let hash_preview = format!("{:016x}", (progress * 1e16) as u64);
                for c in hash_preview.chars() {
                    let char_color = if c.is_ascii_digit() {
                        Color32::from_rgb(100, 200, 255)
                    } else {
                        Color32::from_rgb(255, 200, 100)
                    };
                    ui.label(RichText::new(c.to_string()).monospace().color(char_color).size(20.0));
                }
            });
        });
        
        ui.add_space(30.0);
        
        ui.horizontal(|ui| {
            if ui.button("← Back").clicked() {
                self.first_run.prev_step();
            }
            
            ui.add_space(20.0);
            
            let ready = self.auth.is_entropy_ready();
            if ui.add_enabled(ready, egui::Button::new(RichText::new("Continue →").size(18.0))).clicked() {
                self.first_run.next_step();
            }
            
            if !ready {
                ui.label(RichText::new("Keep moving your mouse!").color(Color32::YELLOW));
            }
        });
    }
    
    fn render_wizard_device(&mut self, ui: &mut egui::Ui) {
        ui.heading("🖥️ Name This Device");
        ui.add_space(20.0);
        
        ui.label("Give this device a name so you can identify it in your network.");
        ui.add_space(20.0);
        
        ui.horizontal(|ui| {
            ui.label("Device Name:");
            ui.text_edit_singleline(&mut self.first_run.device_name);
        });
        
        ui.add_space(20.0);
        
        ui.label("Device Type:");
        ui.horizontal(|ui| {
            let types = [
                (DeviceType::Desktop, "🖥️ Desktop"),
                (DeviceType::Laptop, "💻 Laptop"),
                (DeviceType::Server, "🖧 Server"),
            ];
            
            for (dtype, label) in types {
                if ui.selectable_label(self.first_run.device_type == dtype, label).clicked() {
                    self.first_run.device_type = dtype;
                }
            }
        });
        
        ui.add_space(20.0);
        
        ui.checkbox(&mut self.first_run.enable_tailscale, "Enable Tailscale mesh networking");
        ui.checkbox(&mut self.first_run.enable_phone_sync, "Set up phone sync");
        
        if let Some(ref err) = self.first_run.error_message {
            ui.add_space(10.0);
            ui.label(RichText::new(err).color(Color32::RED));
        }
        
        ui.add_space(30.0);
        
        ui.horizontal(|ui| {
            if ui.button("← Back").clicked() {
                self.first_run.prev_step();
            }
            
            ui.add_space(20.0);
            
            if ui.button(RichText::new("Create Device Key →").size(18.0)).clicked() {
                match self.auth.complete_first_run(
                    &self.first_run.device_name,
                    self.first_run.device_type.clone()
                ) {
                    Ok(device_id) => {
                        self.first_run.error_message = None;
                        self.status_message = format!("Device registered: {}", &device_id[..8]);
                        self.first_run.next_step();
                    }
                    Err(e) => {
                        self.first_run.error_message = Some(e);
                    }
                }
            }
        });
    }
    
    fn render_wizard_tailscale(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔗 Tailscale Setup");
        ui.add_space(20.0);
        
        match self.tailscale.status {
            crate::auth::TailscaleStatus::NotInstalled => {
                ui.label("Tailscale is not installed on this system.");
                ui.add_space(10.0);
                ui.label("Install Tailscale to sync across your devices:");
                ui.add_space(10.0);
                
                if ui.link("https://tailscale.com/download").clicked() {
                    let _ = open::that("https://tailscale.com/download");
                }
                
                ui.add_space(20.0);
                
                if ui.button("Check Again").clicked() {
                    self.tailscale.check_installation();
                }
            }
            crate::auth::TailscaleStatus::Stopped => {
                ui.label("Tailscale is installed but not running.");
                ui.add_space(20.0);
                
                if ui.button("Start Tailscale").clicked() {
                    if let Err(e) = self.tailscale.start() {
                        self.first_run.error_message = Some(e);
                    }
                }
            }
            crate::auth::TailscaleStatus::NeedsLogin => {
                ui.label("Tailscale needs authentication.");
                ui.add_space(10.0);
                ui.label("Run this command in your terminal:");
                ui.add_space(10.0);
                
                egui::Frame::dark_canvas(ui.style()).show(ui, |ui| {
                    ui.label(RichText::new("tailscale login").monospace().size(16.0));
                });
                
                ui.add_space(20.0);
                
                if ui.button("Check Status").clicked() {
                    self.tailscale.get_status();
                }
            }
            crate::auth::TailscaleStatus::Running => {
                ui.label(RichText::new("✅ Tailscale is connected!").color(Color32::from_rgb(0, 200, 100)));
                ui.add_space(10.0);
                
                if let Some(ref ip) = self.tailscale.ip_address {
                    ui.label(format!("Your Tailscale IP: {}", ip));
                }
                if let Some(ref hostname) = self.tailscale.hostname {
                    ui.label(format!("Hostname: {}", hostname));
                }
                
                ui.add_space(20.0);
                
                // Show peers
                let peers = self.tailscale.get_peers();
                if !peers.is_empty() {
                    ui.label(RichText::new("Devices on your network:").strong());
                    for peer in peers {
                        let status = if peer.online { "🟢" } else { "⚪" };
                        ui.label(format!("{} {} ({})", status, peer.hostname, peer.ip_address));
                    }
                }
            }
            crate::auth::TailscaleStatus::Error(ref e) => {
                ui.label(RichText::new(format!("Error: {}", e)).color(Color32::RED));
                
                if ui.button("Retry").clicked() {
                    self.tailscale.get_status();
                }
            }
        }
        
        if let Some(ref err) = self.first_run.error_message {
            ui.add_space(10.0);
            ui.label(RichText::new(err).color(Color32::RED));
        }
        
        ui.add_space(30.0);
        
        ui.horizontal(|ui| {
            if ui.button("← Back").clicked() {
                self.first_run.prev_step();
            }
            
            ui.add_space(20.0);
            
            let label = if self.first_run.enable_phone_sync {
                "Continue to Phone Setup →"
            } else {
                "Finish Setup →"
            };
            
            if ui.button(RichText::new(label).size(18.0)).clicked() {
                self.first_run.next_step();
            }
            
            ui.add_space(20.0);
            
            if ui.small_button("Skip").clicked() {
                self.first_run.enable_tailscale = false;
                self.first_run.next_step();
            }
        });
    }
    
    fn render_wizard_phone(&mut self, ui: &mut egui::Ui) {
        ui.heading("📱 Phone App Pairing");
        ui.add_space(20.0);
        
        // Generate pairing code if not already done
        if self.first_run.pairing_code.is_none() {
            self.first_run.pairing_code = Some(self.auth.generate_pairing_code());
        }
        
        ui.label("Scan this QR code with the Sassy Browser phone app:");
        ui.add_space(20.0);
        
        // QR code placeholder (actual QR rendering would need qrcode crate integration)
        egui::Frame::dark_canvas(ui.style()).show(ui, |ui| {
            ui.set_min_size(Vec2::new(200.0, 200.0));
            ui.centered_and_justified(|ui| {
                let code = self.first_run.pairing_code.as_deref().unwrap_or("000000");
                ui.label(RichText::new(format!("Pairing Code:\n\n{}", code)).size(32.0).monospace());
            });
        });
        
        ui.add_space(20.0);
        
        ui.label("Or enter this code manually in the phone app:");
        ui.add_space(10.0);
        
        if let Some(ref code) = self.first_run.pairing_code {
            ui.label(RichText::new(code).size(48.0).monospace().color(Color32::from_rgb(255, 140, 0)));
        }
        
        ui.add_space(10.0);
        
        ui.label(RichText::new("Download the app:").color(Color32::GRAY));
        ui.horizontal(|ui| {
            if ui.link("iOS App Store").clicked() {
                let _ = open::that("https://apps.apple.com/app/sassy-browser");
            }
            ui.label(" | ");
            if ui.link("Google Play").clicked() {
                let _ = open::that("https://play.google.com/store/apps/details?id=com.sassybrowser");
            }
            ui.label(" | ");
            if ui.link("F-Droid").clicked() {
                let _ = open::that("https://f-droid.org/packages/com.sassybrowser");
            }
        });
        
        ui.add_space(30.0);
        
        ui.horizontal(|ui| {
            if ui.button("← Back").clicked() {
                self.first_run.prev_step();
            }
            
            ui.add_space(20.0);
            
            if ui.button(RichText::new("Finish Setup →").size(18.0)).clicked() {
                self.first_run.next_step();
            }
            
            ui.add_space(20.0);
            
            if ui.small_button("Skip for now").clicked() {
                self.first_run.enable_phone_sync = false;
                self.first_run.next_step();
            }
        });
    }
}

impl eframe::App for BrowserApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Collect entropy from mouse movements for first-run key generation
        if self.first_run.step == FirstRunStep::EntropyCollection {
            ctx.input(|i| {
                if let Some(pos) = i.pointer.hover_pos() {
                    self.auth.add_entropy_mouse(pos.x as i32, pos.y as i32);
                }
                if i.events.iter().any(|e| matches!(e, egui::Event::Key { .. })) {
                    self.auth.add_entropy_key();
                }
            });
        }
        
        // Show first-run wizard if not complete
        if self.first_run.step != FirstRunStep::Complete {
            self.render_first_run_wizard(ctx);
            return;
        }
        
        // Process any pending messages from webviews
        self.engine.process_messages();
        
        // Handle input
        self.handle_keyboard_shortcuts(ctx);
        self.handle_dropped_files(ctx);
        
        // Main layout
        egui::TopBottomPanel::top("browser_chrome").show(ctx, |ui| {
            // Tab bar
            self.render_tabs(ui);
            
            ui.separator();
            
            // Toolbar (navigation + address bar)
            self.render_toolbar(ctx, ui);
            
            // Bookmarks bar
            if self.engine.show_bookmarks_bar() {
                ui.separator();
                self.render_bookmarks_bar(ui);
            }
            
            // Find bar
            if self.find_bar_visible {
                ui.separator();
                self.render_find_bar(ctx, ui);
            }
        });
        
        // Downloads panel (if visible)
        self.render_downloads_panel(ctx);
        
        // Status bar
        self.render_status_bar(ctx);
        
        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_content(ctx, ui);
        });
    }
}

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    // Add bundled custom font (Metamorphous) if present
    // Embedded at compile time from assets/fonts
    fonts.font_data.insert(
        "metamorphous".into(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Metamorphous-7wZ4.ttf")),
    );
    // Prefer our custom font for proportional text
    fonts.families.get_mut(&egui::FontFamily::Proportional).map(|v| v.insert(0, "metamorphous".into()));
    ctx.set_fonts(fonts);
}

fn configure_style(ctx: &egui::Context, dark_mode: bool) {
    // Start from egui defaults then tweak fills so the whole window and panels follow theme
    let mut visuals = if dark_mode { egui::Visuals::dark() } else { egui::Visuals::light() };

    if dark_mode {
        visuals.window_fill = Color32::from_rgb(0x0d, 0x11, 0x17); // #0d1117
        visuals.panel_fill = Color32::from_rgb(0x16, 0x1b, 0x22); // #161b22
        visuals.extreme_bg_color = Color32::from_rgb(0x0a, 0x0c, 0x0e);
        visuals.faint_bg_color = Color32::from_rgb(0x14, 0x18, 0x1e);
        visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(0x0d, 0x11, 0x17);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(0x16, 0x1b, 0x22);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x21, 0x26, 0x2d);
        visuals.widgets.active.bg_fill = Color32::from_rgb(0x21, 0x26, 0x2d);
        visuals.override_text_color = Some(Color32::from_rgb(0xe6, 0xed, 0xf3));
    } else {
        // Light mode: keep defaults but nudge fills to match theme defaults
        visuals.window_fill = Color32::from_rgb(0xff, 0xff, 0xff);
        visuals.panel_fill = Color32::from_rgb(0xf6, 0xf8, 0xfa);
        visuals.extreme_bg_color = Color32::from_rgb(0xf0, 0xf2, 0xf4);
    }

    ctx.set_visuals(visuals);

    // Improve default text sizes for readability across the UI
    let mut style = (*ctx.style()).clone();
    style.text_styles.insert(egui::TextStyle::Heading, FontId::proportional(20.0));
    style.text_styles.insert(egui::TextStyle::Body, FontId::proportional(16.0));
    style.text_styles.insert(egui::TextStyle::Monospace, FontId::monospace(15.0));
    style.text_styles.insert(egui::TextStyle::Button, FontId::proportional(15.0));
    style.text_styles.insert(egui::TextStyle::Small, FontId::proportional(14.0));
    ctx.set_style(style);
}

/// Run the browser application
pub fn run_browser() -> Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("Sassy Browser")
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "Sassy Browser",
        native_options,
        Box::new(|cc| Ok(Box::new(BrowserApp::new(cc)))),
    ).map_err(|e| anyhow::anyhow!("Failed to run browser: {}", e))
}
