//! Main application - Browser with egui chrome and wry webview
//! 
//! Architecture:
//! - tao provides the main window
//! - egui renders browser chrome (tabs, address bar, bookmarks bar, status bar)
//! - wry webview handles web content in the content area
//! - egui viewers handle files (PDF, images, documents, etc.) in the content area

use crate::auth::{AuthManager, FirstRunState, FirstRunStep, DeviceType, TailscaleManager, PhoneSync, SyncType};
use crate::browser::{BrowserEngine, DownloadState, Tab, TabContent, TabId, HistoryManager};
use crate::file_handler::{FileType, OpenFile};
use crate::html_renderer::HtmlRenderer;
use crate::extensions::ExtensionManager;
use crate::input::FocusManager;
use crate::network_monitor::{NetworkMonitor, ActivityIndicatorState, ConnectionType, ConnectionState, ConnectionFilter, ConnectionSort, format_bytes, format_speed, format_duration};
use crate::password_vault::{PasswordVault, Credential, PasswordGeneratorOptions, generate_password};
use crate::smart_history::SmartHistory;
use crate::family_profiles::{ProfileManager, ProfileType, Profile, Action};
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
use eframe::egui::{self, Color32, FontId, Key, RichText, Vec2};
use eframe::egui::{ColorImage, TextureHandle};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;
use url::Url;
use urlencoding::decode;

struct TrackedDownload {
    conn_id: u64,
    last_bytes: u64,
    completed: bool,
}

/// Browser application state
pub struct BrowserApp {
    engine: BrowserEngine,
    extension_manager: ExtensionManager,
    
    // Authentication & licensing
    auth: AuthManager,
    tailscale: TailscaleManager,
    first_run: FirstRunState,
    
    // =========================================================================
    // DISRUPTOR FEATURES - Kills paid software & Chrome bloat
    // =========================================================================
    
    // ðŸ“¡ Network Activity Monitor - NO HIDDEN TRAFFIC
    network_monitor: NetworkMonitor,
    activity_indicator: ActivityIndicatorState,
    download_connections: HashMap<Uuid, TrackedDownload>,
    auth_panel_visible: bool,
    auth_status: String,
    tailscale_auth_key_input: String,
    tailscale_file_target: String,
    phone_sync: PhoneSync,
    network_active_connection: Option<u64>,
    network_last_net_sample: Option<Instant>,
    
    // ðŸ” Password Vault - Replaces LastPass, 1Password, Chrome passwords
    password_vault: PasswordVault,
    vault_search_query: String,
    vault_panel_visible: bool,
    vault_needs_reauth: bool,
    vault_pin_input: String,
    vault_status: String,
    vault_new_title: String,
    vault_new_username: String,
    vault_new_password: String,
    vault_new_url: String,
    vault_new_notes: String,
    vault_new_folder: String,
    vault_folder_filter: String,
    vault_editing_id: Option<String>,
    vault_edit_title: String,
    vault_edit_username: String,
    vault_edit_password: String,
    vault_edit_url: String,
    vault_edit_notes: String,
    vault_edit_folder: String,
    vault_autofill_enabled: bool,
    vault_import_buffer: String,
    vault_export_buffer: String,
    password_generator_opts: PasswordGeneratorOptions,
    generated_password: String,
    
    // â±ï¸ Smart History - 14.7s delay, NSFW detection
    history_manager: HistoryManager,
    smart_history: SmartHistory,
    smart_history_active_url: Option<String>,
    history_panel_visible: bool,
    history_status: String,
    history_search_query: String,
    history_domain_filter: String,
    history_date_start: String,
    history_date_end: String,
    history_day_query: String,
    history_auto_exclude: bool,
    history_nsfw_sensitivity: f32,
    history_last_url: Option<String>,
    history_last_title: Option<String>,
    
    // ðŸ‘¨â€ðŸ‘©â€ðŸ‘§â€ðŸ‘¦ Family Profiles - Parental controls that work
    profile_manager: ProfileManager,
    profiles_panel_visible: bool,
    extensions_panel_visible: bool,
    profile_status: String,
    profile_new_name: String,
    profile_new_pin: String,
    profile_switch_pin: String,
    profile_new_type: ProfileType,
    profile_parent_choice: Option<String>,
    profile_allow_input: String,
    profile_block_input: String,
    profile_search_log_input: String,
    profile_report_output: String,
    profile_request_reason: String,
    
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
    #[allow(dead_code)]
    context_menu_pos: Option<egui::Pos2>,
    #[allow(dead_code)]
    context_menu_link: Option<String>,
    
    // Status
    status_message: String,

    // Input focus + text state
    focus_manager: FocusManager,

    // Import/export buffers
    bookmark_import_buffer: String,
    bookmark_export_buffer: String,
    download_url_input: String,
    extension_load_path: String,
}

impl BrowserApp {
    fn guarded_navigate(&mut self, url: &str) {
        if url.starts_with("sassy://") {
            self.engine.navigate(url);
            return;
        }

        if self.network_monitor.is_blocked(url) {
            self.status_message = "Navigation blocked by tracker filter".into();
            return;
        }

        match self.profile_manager.can_access_url(url) {
            Ok(()) => {
                self.profile_manager.record_activity();
                self.profile_manager.record_site_visit(url);
                let conn_id = self.network_monitor.start_connection(url, ConnectionType::Document);
                self.network_active_connection = Some(conn_id);
                self.network_last_net_sample = Some(Instant::now());
                self.smart_history.visit(url, url, None);
                self.engine.navigate(url);
            }
            Err(reason) => {
                let reason_text = reason.description();
                let mut msg = format!("Navigation blocked: {}", reason_text);
                if let Some(active) = self.profile_manager.active_profile() {
                    if active.is_restricted() {
                        let req_id = self.profile_manager.request_approval(Action::AccessBlockedSite(url.to_string()));
                        msg = format!("{} (approval requested: {})", msg, req_id);
                    }
                }
                self.status_message = msg;
            }
        }
    }

    fn render_extensions_panel(&mut self, ctx: &egui::Context) {
        if !self.extensions_panel_visible {
            return;
        }

        let extensions_snapshot = self.extension_manager.list_extensions();

        egui::Window::new("ðŸ§© Extensions")
            .open(&mut self.extensions_panel_visible)
            .resizable(true)
            .default_size(Vec2::new(640.0, 420.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Extensions");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let _ = ui.small_button("Close");
                    });
                });

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Load extension from path:");
                    ui.text_edit_singleline(&mut self.extension_load_path);
                    if ui.button("Load").clicked() {
                        let path = self.extension_load_path.trim();
                        if !path.is_empty() {
                            match self.extension_manager.load_extension(path) {
                                Ok(()) => {
                                    self.status_message = format!("Loaded extension: {}", path);
                                    self.extension_load_path.clear();
                                }
                                Err(e) => {
                                    self.status_message = format!("Failed to load extension: {}", e);
                                }
                            }
                        }
                    }
                });

                ui.separator();

                if extensions_snapshot.is_empty() {
                    ui.label("No extensions installed.");
                } else {
                    for ext in extensions_snapshot {
                        ui.horizontal(|ui| {
                            let mut enabled = ext.enabled;
                            if ui.checkbox(&mut enabled, &ext.name).changed() {
                                if enabled {
                                    self.extension_manager.enable_extension(&ext.id);
                                } else {
                                    self.extension_manager.disable_extension(&ext.id);
                                }
                            }
                            ui.label(format!("v{}", ext.version));
                            if ui.small_button("Unload").clicked() {
                                self.extension_manager.unload_extension(&ext.id);
                            }
                        });

                        if !ext.description.is_empty() {
                            ui.label(RichText::new(ext.description).small());
                        }

                        ui.separator();
                    }
                }
            });
    }

    fn submit_address_bar(&mut self) {
        let input = self.engine.address_bar_text().to_string();
        let looks_like_url = input.contains("://") || (input.contains('.') && !input.contains(' '));
        let mut target = input.clone();

        if looks_like_url {
            // As-is navigate
        } else {
            // Treat as search query and enforce safe search flag by using engine search provider
            let encoded = input.replace(' ', "+");
            target = format!("{}{}", self.engine.search_engine.clone(), encoded);
            self.profile_manager.record_search(&input);
        }

        self.guarded_navigate(&target);
        self.engine.set_address_bar_focused(false);
    }

    fn guard_download_request(&mut self, url: &str, suggested_filename: Option<&str>) -> bool {
        let filename = self.pick_download_filename(url, suggested_filename);
        let size_hint = 0u64;
        let action = Action::Download {
            filename: filename.clone(),
            url: url.to_string(),
            size: size_hint,
        };

        let needs_approval = self.profile_manager
            .active_profile()
            .map(|p| p.requires_approval(&action) || p.restrictions.downloads_need_approval)
            .unwrap_or(false);

        match self.profile_manager.can_download(&filename, size_hint) {
            Ok(_) => {
                let engine_filename = suggested_filename.unwrap_or(&filename);
                self.engine.start_download(url, Some(engine_filename));
                self.profile_manager.record_download(&filename, url, size_hint, true, None);
                self.status_message = format!("Download started: {}", filename);
                true
            }
            Err(reason) => {
                let mut msg = format!("Download blocked: {}", reason);
                if needs_approval {
                    let req_id = self.profile_manager.request_approval(action);
                    msg = format!("{} (approval requested: {})", msg, req_id);
                    self.profile_manager.record_download(&filename, url, size_hint, false, None);
                }
                self.status_message = msg;
                false
            }
        }
    }

    fn pick_download_filename(&self, url: &str, suggested: Option<&str>) -> String {
        if let Some(name) = suggested {
            let trimmed = name.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }

        Url::parse(url)
            .ok()
            .and_then(|u| {
                u.path_segments()
                    .and_then(|segments| segments.filter(|s| !s.is_empty()).next_back())
                    .map(|s| decode(s).unwrap_or_else(|_| s.into()).to_string())
            })
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "download".to_string())
    }

    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_fonts(&cc.egui_ctx);
        configure_style(&cc.egui_ctx, true);
        
        let auth = AuthManager::new();
        let mut tailscale = TailscaleManager::new();
        tailscale.check_installation();
        
        let first_run = if auth.is_first_run {
            FirstRunState::default()
        } else {
            FirstRunState { step: FirstRunStep::Complete, ..Default::default() }
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
                // Load raster formats (ICO, PNG, JPEG, etc.)
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

        let mut smart_history = SmartHistory::new();
        smart_history.set_intent_delay(1.5);
        smart_history.set_auto_exclude_nsfw(true);
        smart_history.set_incognito(false);

        Self {
            engine: BrowserEngine::new(),
            extension_manager: ExtensionManager::new(),
            auth,
            tailscale,
            first_run,
            
            // Disruptor features
            network_monitor: NetworkMonitor::new(),
            activity_indicator: ActivityIndicatorState::default(),
            download_connections: HashMap::new(),
            auth_panel_visible: false,
            auth_status: String::new(),
            tailscale_auth_key_input: String::new(),
            tailscale_file_target: String::new(),
            phone_sync: PhoneSync::new(),
            network_active_connection: None,
            network_last_net_sample: None,
            password_vault: PasswordVault::new(config_dir.clone()),
            vault_search_query: String::new(),
            vault_panel_visible: false,
            vault_needs_reauth: false,
            vault_pin_input: String::new(),
            vault_status: String::new(),
            vault_new_title: String::new(),
            vault_new_username: String::new(),
            vault_new_password: String::new(),
            vault_new_url: String::new(),
            vault_new_notes: String::new(),
            vault_new_folder: String::new(),
            vault_folder_filter: String::new(),
            vault_editing_id: None,
            vault_edit_title: String::new(),
            vault_edit_username: String::new(),
            vault_edit_password: String::new(),
            vault_edit_url: String::new(),
            vault_edit_notes: String::new(),
            vault_edit_folder: String::new(),
            vault_autofill_enabled: true,
            vault_import_buffer: String::new(),
            vault_export_buffer: String::new(),
            password_generator_opts: PasswordGeneratorOptions::default(),
            generated_password: String::new(),
            history_manager: HistoryManager::new(),
            smart_history,
            smart_history_active_url: None,
            history_panel_visible: false,
            history_status: String::new(),
            history_search_query: String::new(),
            history_domain_filter: String::new(),
            history_date_start: String::new(),
            history_date_end: String::new(),
            history_day_query: String::new(),
            history_auto_exclude: true,
            history_nsfw_sensitivity: 0.5,
            history_last_url: None,
            history_last_title: None,
            profile_manager: ProfileManager::new(),
            profiles_panel_visible: false,
            extensions_panel_visible: false,
            profile_status: String::new(),
            profile_new_name: String::new(),
            profile_new_pin: String::new(),
            profile_switch_pin: String::new(),
            profile_new_type: ProfileType::Kid,
            profile_parent_choice: None,
            profile_allow_input: String::new(),
            profile_block_input: String::new(),
            profile_search_log_input: String::new(),
            profile_report_output: String::new(),
            profile_request_reason: String::new(),
            
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
            focus_manager: FocusManager::new(),
            bookmark_import_buffer: String::new(),
            bookmark_export_buffer: String::new(),
            download_url_input: String::new(),
            extension_load_path: String::new(),
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
            } else if ui.add_enabled(can_back, egui::Button::new("â—€").min_size(Vec2::new(28.0, 24.0)))
                .on_hover_text("Back (Alt+Left)")
                .clicked() {
                self.engine.go_back();
            }

            if let Some(tex) = self.icons.get("forward") {
                if ui.add_enabled(can_forward, egui::ImageButton::new((tex.id(), Vec2::new(28.0, 24.0))))
                    .on_hover_text("Forward (Alt+Right)")
                    .clicked() {
                    self.engine.go_forward();
                }
            } else if ui.add_enabled(can_forward, egui::Button::new("â–¶").min_size(Vec2::new(28.0, 24.0)))
                .on_hover_text("Forward (Alt+Right)")
                .clicked() {
                self.engine.go_forward();
            }
            
            if is_loading {
                if ui.button("âœ•").on_hover_text("Stop").clicked() {
                    self.engine.stop();
                }
            } else if ui.button("â†»").on_hover_text("Reload (F5)").clicked() {
                self.engine.reload();
            }
            
            if ui.button("Home").on_hover_text("Home").clicked() {
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
                    ui.colored_label(Color32::from_rgb(100, 200, 100), "ðŸ”’");
                }

                let engine_text = self.engine.address_bar_text().to_string();
                if !self.focus_manager.is_address_bar_focused() && self.focus_manager.address_bar.text != engine_text {
                    self.focus_manager.address_bar.set_text(engine_text);
                }

                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.focus_manager.address_bar.text)
                        .desired_width(address_width - 40.0)
                        .font(FontId::proportional(14.0))
                        .hint_text("Search or enter URL")
                );

                if response.changed() {
                    self.engine.set_address_bar_text(self.focus_manager.address_bar.text.clone());
                }

                if response.gained_focus() {
                    self.focus_manager.focus_address_bar(self.engine.address_bar_text());
                    self.engine.set_address_bar_focused(true);
                }

                let enter_pressed = ctx.input(|i| i.key_pressed(Key::Enter));
                if response.lost_focus() {
                    self.focus_manager.blur();
                    self.engine.set_address_bar_focused(false);
                    if enter_pressed {
                        self.submit_address_bar();
                    }
                } else if enter_pressed && response.has_focus() {
                    self.submit_address_bar();
                }
            });
            
            // Bookmark button (adds/removes on the bookmarks bar)
            let current_url = self.engine.address_bar_text().to_string();
            let is_bookmarked = self.engine.bookmarks.get_by_url(&current_url).is_some();
            let bookmark_label = if is_bookmarked { "Bookmarked" } else { "Add to Bar" };
            if ui.button(bookmark_label).on_hover_text("Toggle bookmark on Bookmarks Bar").clicked() {
                if is_bookmarked {
                    self.engine.bookmarks.remove_by_url(&current_url);
                    let _ = self.engine.bookmarks.save();
                    self.status_message = "Removed from Bookmarks Bar".into();
                } else {
                    let title = self.engine.active_tab().map(|t| t.title()).unwrap_or_else(|| current_url.clone());
                    self.engine.bookmarks.add_to_bar(&current_url, &title);
                    let _ = self.engine.bookmarks.save();
                    self.status_message = "Saved to Bookmarks Bar".into();
                }
            }
            
            // Menu button
            ui.menu_button("Menu", |ui| {
                if ui.button("Open File...").clicked() {
                    self.open_file_dialog();
                    ui.close_menu();
                }
                ui.separator();
                // Bookmarks menu button: prefer image icon if available
                if let Some(tex) = self.icons.get("bookmarks") {
                    if ui.add(egui::ImageButton::new((tex.id(), Vec2::new(18.0,18.0))))
                        .on_hover_text("Bookmarks")
                        .clicked() {
                        self.guarded_navigate("sassy://bookmarks");
                        ui.close_menu();
                    }
                } else if ui.button("Bookmarks").clicked() {
                    self.guarded_navigate("sassy://bookmarks");
                    ui.close_menu();
                }
                if ui.button("History").clicked() {
                    self.history_panel_visible = true;
                    ui.close_menu();
                }
                if ui.button("Password Vault").clicked() {
                    self.vault_panel_visible = true;
                    self.vault_needs_reauth = true;
                    self.password_vault.lock();
                    self.vault_status = "Vault locked".into();
                    ui.close_menu();
                }
                if ui.button("Profiles").clicked() {
                    self.profiles_panel_visible = true;
                    ui.close_menu();
                }
                if ui.button("Downloads").clicked() {
                    self.engine.set_show_downloads_panel(!self.engine.show_downloads_panel());
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Find in Page (Ctrl+F)").clicked() {
                    self.find_bar_visible = true;
                    ui.close_menu();
                }
                if ui.button("Print (Ctrl+P)").clicked() {
                    self.print_current();
                    ui.close_menu();
                }
                ui.separator();
                if ui.checkbox(self.engine.show_bookmarks_bar_mut(), "Show Bookmarks Bar").changed() {
                    ui.close_menu();
                }
                if ui.checkbox(&mut self.dark_mode, "Dark Mode").clicked() {
                    configure_style(ctx, self.dark_mode);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Account & Sync").clicked() {
                    self.auth_panel_visible = true;
                    ui.close_menu();
                }
                if ui.button("Settings").clicked() {
                    self.guarded_navigate("sassy://settings");
                    ui.close_menu();
                }
                if ui.button("Extensions").clicked() {
                    self.extensions_panel_visible = true;
                    ui.close_menu();
                }
            });

            ui.separator();
            self.render_network_indicator(ui);
            self.render_vault_autofill(ui);
        });
    }

    fn render_network_indicator(&mut self, ui: &mut egui::Ui) {
        let (down, up) = self.network_monitor.current_speed();
        let (total_down, total_up) = self.network_monitor.total_transferred();
        let blocked = self.network_monitor.blocked_count();
        let active = self.network_monitor.active_count();

        ui.horizontal(|ui| {
            let label = format!("Net {} â†“{} â†‘{}", active, format_speed(down), format_speed(up));
            if ui.selectable_label(self.activity_indicator.expanded, label).clicked() {
                self.activity_indicator.expanded = !self.activity_indicator.expanded;
            }
            ui.label(format!("Total â†“{} â†‘{}", format_bytes(total_down), format_bytes(total_up)));
            if blocked > 0 {
                ui.label(RichText::new(format!("Blocked {}", blocked)).color(Color32::RED));
            }
        });

        if !self.activity_indicator.expanded {
            return;
        }

        ui.horizontal(|ui| {
            ui.label("Filter");
            egui::ComboBox::from_id_salt("net_filter")
                .selected_text(format!("{:?}", self.activity_indicator.filter))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.activity_indicator.filter, ConnectionFilter::All, "All");
                    ui.selectable_value(&mut self.activity_indicator.filter, ConnectionFilter::Active, "Active");
                    ui.selectable_value(&mut self.activity_indicator.filter, ConnectionFilter::Documents, "Docs");
                    ui.selectable_value(&mut self.activity_indicator.filter, ConnectionFilter::Downloads, "Downloads");
                    ui.selectable_value(&mut self.activity_indicator.filter, ConnectionFilter::Scripts, "Scripts");
                    ui.selectable_value(&mut self.activity_indicator.filter, ConnectionFilter::Images, "Images");
                    ui.selectable_value(&mut self.activity_indicator.filter, ConnectionFilter::Xhr, "XHR");
                    ui.selectable_value(&mut self.activity_indicator.filter, ConnectionFilter::Blocked, "Blocked");
                });

            ui.label("Sort");
            egui::ComboBox::from_id_salt("net_sort")
                .selected_text(format!("{:?}", self.activity_indicator.sort_by))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.activity_indicator.sort_by, ConnectionSort::Newest, "Newest");
                    ui.selectable_value(&mut self.activity_indicator.sort_by, ConnectionSort::Oldest, "Oldest");
                    ui.selectable_value(&mut self.activity_indicator.sort_by, ConnectionSort::Largest, "Largest");
                    ui.selectable_value(&mut self.activity_indicator.sort_by, ConnectionSort::Slowest, "Slowest");
                    ui.selectable_value(&mut self.activity_indicator.sort_by, ConnectionSort::Domain, "Domain");
                });
        });

        let mut conns = self.network_monitor.all_connections();
        conns.retain(|c| match self.activity_indicator.filter {
            ConnectionFilter::All => true,
            ConnectionFilter::Active => matches!(c.state, ConnectionState::Connecting | ConnectionState::Uploading | ConnectionState::Downloading),
            ConnectionFilter::Documents => c.connection_type == ConnectionType::Document,
            ConnectionFilter::Downloads => c.connection_type == ConnectionType::Download,
            ConnectionFilter::Scripts => c.connection_type == ConnectionType::Script,
            ConnectionFilter::Images => c.connection_type == ConnectionType::Image,
            ConnectionFilter::Xhr => c.connection_type == ConnectionType::Xhr,
            ConnectionFilter::Blocked => blocked > 0,
        });

        match self.activity_indicator.sort_by {
            ConnectionSort::Newest => conns.sort_by(|a, b| b.started_at.cmp(&a.started_at)),
            ConnectionSort::Oldest => conns.sort_by(|a, b| a.started_at.cmp(&b.started_at)),
            ConnectionSort::Largest => conns.sort_by(|a, b| (b.bytes_received + b.bytes_sent).cmp(&(a.bytes_received + a.bytes_sent))),
            ConnectionSort::Slowest => conns.sort_by(|a, b| a.bytes_received.cmp(&b.bytes_received)),
            ConnectionSort::Domain => conns.sort_by(|a, b| a.domain.cmp(&b.domain)),
        }

        egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
            for conn in conns {
                let elapsed = conn.started_at.elapsed();
                let total = conn.bytes_received + conn.bytes_sent;
                let icon = conn.connection_type.icon();
                let state_color = conn.state.color();
                let color32 = Color32::from_rgb(state_color[0], state_color[1], state_color[2]);
                ui.horizontal(|ui| {
                    ui.label(icon);
                    ui.label(RichText::new(&conn.domain).color(color32));
                    ui.label(format_bytes(total).to_string());
                    ui.label(format_duration(elapsed));
                    if let Some(code) = conn.status_code {
                        ui.label(format!("{}", code));
                    }
                });
            }
        });
    }

    fn render_vault_autofill(&mut self, ui: &mut egui::Ui) {
        if !self.vault_autofill_enabled || !self.password_vault.is_unlocked() {
            return;
        }

        let active_url = if let Some(Tab { content: TabContent::Web { url, .. }, .. }) = self.engine.active_tab() {
            url.clone()
        } else {
            return;
        };

        let matches: Vec<Credential> = self.password_vault.find_for_url(&active_url)
            .into_iter()
            .cloned()
            .collect();

        if matches.is_empty() {
            // Offer quick-add for this site
            ui.horizontal(|ui| {
                ui.label("Vault: no saved login for this site");
                if ui.small_button("Save one").clicked() {
                    self.vault_panel_visible = true;
                    self.vault_needs_reauth = true;
                    self.password_vault.lock();
                    self.vault_new_url = active_url.clone();
                    self.vault_status = "Add a credential for this site".into();
                }
            });
            return;
        }

        ui.horizontal(|ui| {
            ui.label("Vault matches");
            for cred in matches.iter().take(3) {
                if ui.small_button(format!("{} / {}", cred.title, cred.username)).clicked() {
                    ui.output_mut(|o| o.copied_text = cred.password.clone());
                    self.vault_status = format!("Copied password for {}", cred.title);
                }
            }
            if matches.len() > 3 {
                ui.label(format!("+{} more", matches.len() - 3));
            }
            if ui.small_button("Open Vault").clicked() {
                self.vault_panel_visible = true;
                self.vault_needs_reauth = true;
                self.password_vault.lock();
            }
        });
    }

    fn sync_downloads_into_network_monitor(&mut self) {
        let downloads = self.engine.downloads.downloads();
        let mut active_ids = Vec::with_capacity(downloads.len());

        for download in &downloads {
            active_ids.push(download.id);

            let entry = self.download_connections.entry(download.id).or_insert_with(|| TrackedDownload {
                conn_id: self.network_monitor.start_connection(&download.url, ConnectionType::Download),
                last_bytes: 0,
                completed: false,
            });

            if download.downloaded_bytes > entry.last_bytes {
                let delta = download.downloaded_bytes - entry.last_bytes;
                self.network_monitor.update_connection(entry.conn_id, delta, 0);
                entry.last_bytes = download.downloaded_bytes;
            }

            if !entry.completed {
                match download.state {
                    DownloadState::Completed => {
                        self.network_monitor.complete_connection(entry.conn_id, 200, download.mime_type.clone());
                        entry.completed = true;
                    }
                    DownloadState::Failed => {
                        let reason = download.error.clone().unwrap_or_else(|| "Download failed".to_string());
                        self.network_monitor.fail_connection(entry.conn_id, &reason);
                        entry.completed = true;
                    }
                    DownloadState::Cancelled => {
                        self.network_monitor.fail_connection(entry.conn_id, "Cancelled");
                        entry.completed = true;
                    }
                    DownloadState::Paused | DownloadState::Pending | DownloadState::Downloading => {}
                }
            }
        }

        self.download_connections.retain(|id, _| active_ids.contains(id));
    }

    fn render_auth_panel(&mut self, ctx: &egui::Context) {
        if !self.auth_panel_visible {
            return;
        }

        let license = self.auth.license.clone();
        let features = license.features();
        let max_devices = license.max_devices();
        let paired = self.auth.paired_devices.clone();
        let tailscale_status = self.tailscale.get_status();

        egui::Window::new("ðŸ”‘ Account & Sync")
            .open(&mut self.auth_panel_visible)
            .default_size(Vec2::new(720.0, 520.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("License & Devices");
                    if !self.auth_status.is_empty() {
                        ui.label(RichText::new(&self.auth_status).color(Color32::from_rgb(120, 200, 255)));
                    }
                });

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label(format!("Tier: {:?}", license));
                    ui.label(format!("Device limit: {}", max_devices));
                    if ui.small_button("Reload config").clicked() {
                        // Reload stored auth state
                        self.auth = AuthManager::new();
                        self.auth_status = "Auth config reloaded".into();
                    }
                });

                ui.collapsing("Included features", |ui| {
                    for feat in features {
                        ui.label(feat);
                    }
                });

                ui.collapsing("Paired devices", |ui| {
                    if paired.is_empty() {
                        ui.label("No devices paired yet");
                    } else {
                        for dev in &paired {
                            ui.horizontal(|ui| {
                                ui.label(DeviceType::icon(&dev.device_type).to_string());
                                ui.label(format!("{} ({})", dev.device_name, dev.device_id));
                                ui.label(format!("Last seen: {}", dev.last_seen));
                                if let Some(ip) = &dev.tailscale_ip {
                                    ui.label(format!("IP {}", ip));
                                }
                            });
                        }
                    }
                });

                ui.separator();
                ui.heading("Tailscale");
                ui.horizontal(|ui| {
                    ui.label(format!("Status: {:?}", tailscale_status));
                    if ui.small_button("Check").clicked() {
                        let _ = self.tailscale.get_status();
                    }
                    if ui.small_button("Start").clicked() {
                        match self.tailscale.start() {
                            Ok(()) => self.auth_status = "Tailscale started".into(),
                            Err(e) => self.auth_status = format!("Tailscale error: {}", e),
                        }
                    }
                    if ui.small_button("Stop").clicked() {
                        match self.tailscale.stop() {
                            Ok(()) => self.auth_status = "Tailscale stopped".into(),
                            Err(e) => self.auth_status = format!("Stop failed: {}", e),
                        }
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Auth key");
                    ui.text_edit_singleline(&mut self.tailscale_auth_key_input);
                    if ui.small_button("Start with key").clicked() {
                        match self.tailscale.start_with_auth_key(&self.tailscale_auth_key_input) {
                            Ok(()) => self.auth_status = "Tailscale started with key".into(),
                            Err(e) => self.auth_status = format!("Auth key failed: {}", e),
                        }
                    }
                });

                ui.horizontal(|ui| {
                    if ui.small_button("Receive files").clicked() {
                        match self.tailscale.receive_files() {
                            Ok(files) => {
                                if files.is_empty() {
                                    self.auth_status = "No incoming files".into();
                                } else {
                                    self.auth_status = format!("Received {} files", files.len());
                                }
                            }
                            Err(e) => self.auth_status = format!("Receive failed: {}", e),
                        }
                    }
                    ui.add(egui::TextEdit::singleline(&mut self.tailscale_file_target).hint_text("peer_ip:/path/to/file"));
                    if ui.small_button("Send test file").clicked() {
                        let parts: Vec<_> = self.tailscale_file_target.splitn(2, ':').collect();
                        if parts.len() == 2 {
                            let (peer, file) = (parts[0], parts[1]);
                            match self.tailscale.send_file(peer, file) {
                                Ok(()) => self.auth_status = "File sent".into(),
                                Err(e) => self.auth_status = format!("Send failed: {}", e),
                            }
                        } else {
                            self.auth_status = "Enter peer_ip:/path".into();
                        }
                    }
                });

                ui.collapsing("Peers", |ui| {
                    let peers = self.tailscale.get_peers();
                    if peers.is_empty() {
                        ui.label("No peers detected");
                    } else {
                        for peer in peers {
                            ui.horizontal(|ui| {
                                ui.label(format!("{} ({})", peer.hostname, peer.ip_address));
                                ui.label(format!("OS: {}", peer.os));
                                ui.label(if peer.online { "Online" } else { "Offline" });
                            });
                        }
                    }
                });

                ui.separator();
                ui.heading("Phone Sync");
                ui.horizontal(|ui| {
                    if ui.small_button("Connect via Tailscale").clicked() {
                        match self.phone_sync.connect_via_tailscale(&self.tailscale) {
                            Ok(()) => self.auth_status = "Phone connected".into(),
                            Err(e) => self.auth_status = format!("Phone connect failed: {}", e),
                        }
                    }
                    if ui.small_button("Queue bookmark sync").clicked() {
                        self.phone_sync.queue_sync(SyncType::Bookmark, b"sample-bookmark".to_vec());
                        self.auth_status = "Queued sync".into();
                    }
                    if ui.small_button("Sync all").clicked() {
                        match self.phone_sync.sync_all(&self.tailscale) {
                            Ok(count) => self.auth_status = format!("Synced {} items", count),
                            Err(e) => self.auth_status = format!("Sync failed: {}", e),
                        }
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
                            if !*pinned
                                && ui.small_button("Ã—").clicked() {
                                    close_tab = Some(idx);
                                }
                        });
                    });
                
                ui.add_space(2.0);
            }
            
            // New tab button
            if ui.button("ï¼‹").on_hover_text("New Tab (Ctrl+T)").clicked() {
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
                    self.guarded_navigate(url);
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
            
            if ui.button("Ã—").clicked() {
                self.find_bar_visible = false;
            }
        });
    }
    
    fn render_content(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        // Extract content info first to avoid borrow conflicts
        let content_type = self.engine.active_tab().map(|t| match &t.content {
            TabContent::NewTab => ("New Tab", String::new()),
            TabContent::Settings => ("Settings", String::new()),
            TabContent::History => ("History", String::new()),
            TabContent::Bookmarks => ("Bookmarks", String::new()),
            TabContent::Downloads => ("Downloads", String::new()),
            TabContent::Web { url, .. } => ("Web", url.clone()),
            TabContent::File(_) => ("File", String::new()),
        });
        
        match content_type {
            Some(("New Tab", _)) => self.render_new_tab_page(ui),
            Some(("Settings", _)) => self.render_settings_page(ui),
            Some(("History", _)) => self.render_history_page(ui),
            Some(("Bookmarks", _)) => self.render_bookmarks_page(ui),
            Some(("Downloads", _)) => self.render_downloads_page(ui),
            Some(("Web", url)) => {
                // Check if navigation is blocked (status_message starts with "Navigation blocked:")
                if self.status_message.starts_with("Navigation blocked:") {
                    let reason = self.status_message.clone();
                    let domain = crate::family_profiles::extract_domain(&url);
                    let approval_requests: Vec<_> = self.profile_manager.all_approval_requests()
                        .iter()
                        .filter(|r| matches!(&r.action, crate::family_profiles::Action::AccessBlockedSite(u) if crate::family_profiles::extract_domain(u) == domain))
                        .cloned()
                        .collect();
                    ui.vertical_centered(|ui| {
                        ui.add_space(60.0);
                        ui.heading(RichText::new("ðŸš« Site Blocked").size(36.0).color(Color32::RED));
                        ui.add_space(10.0);
                        ui.label(RichText::new(reason).size(20.0).color(Color32::RED));
                        ui.add_space(10.0);
                        if let Some(profile) = self.profile_manager.active_profile() {
                            if profile.is_restricted()
                                && approval_requests.iter().all(|r| r.status != crate::family_profiles::ApprovalStatus::Pending)
                                    && ui.button("Request Parent Approval").clicked() {
                                        let req_id = self.profile_manager.request_approval(crate::family_profiles::Action::AccessBlockedSite(url.clone()));
                                        self.status_message = format!("Approval requested: {}", req_id);
                                    }
                        }
                        if !approval_requests.is_empty() {
                            ui.add_space(10.0);
                            ui.heading("Approval Requests for this Site:");
                            for req in &approval_requests {
                                let (status_text, color, tooltip) = match req.status {
                                    crate::family_profiles::ApprovalStatus::Pending => ("Pending", Color32::YELLOW, "Waiting for parent approval"),
                                    crate::family_profiles::ApprovalStatus::Approved => ("Approved", Color32::from_rgb(0x16, 0xf2, 0xd6), "Approved by parent"),
                                    crate::family_profiles::ApprovalStatus::Denied => ("Denied", Color32::RED, req.parent_response.as_deref().unwrap_or("Denied by parent")),
                                    crate::family_profiles::ApprovalStatus::Expired => ("Expired", Color32::GRAY, "Approval request expired"),
                                };
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(status_text).color(color)).on_hover_text(tooltip);
                                    if (req.status == crate::family_profiles::ApprovalStatus::Denied || req.status == crate::family_profiles::ApprovalStatus::Expired)
                                        && ui.button("Resubmit").on_hover_text("Request approval again").clicked() {
                                            let new_id = self.profile_manager.request_approval(req.action.clone());
                                            self.status_message = format!("Resubmitted approval request: {}", new_id);
                                        }
                                    if let Some(resp) = &req.parent_response {
                                        if req.status == crate::family_profiles::ApprovalStatus::Denied {
                                            ui.label(RichText::new(format!("Reason: {}", resp)).color(Color32::RED)).on_hover_text("Parent's reason for denial");
                                        }
                                    }
                                });
                            }
                        }
                        ui.add_space(20.0);
                        ui.label(RichText::new("If you believe this site should be allowed, ask your parent or guardian for approval.").italics().color(Color32::GRAY));
                        ui.label(RichText::new("Parents: Review requests in the Profiles panel.").italics().color(Color32::GRAY));
                    });
                } else {
                    self.render_web_content(ctx, ui, &url);
                    self.network_monitor.cleanup_old(Duration::from_secs(30));
                }
            }
            Some(("file", _)) => {
                // Get file reference carefully
                if let Some(tab) = self.engine.active_tab() {
                    if let TabContent::File(file) = &tab.content {
                        let file_type = file.file_type;
                        let zoom = self.zoom_level;
                        // Clone necessary data to avoid borrow issues
                        let file_clone = file.clone();
                        let _ = tab;
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
                ui.heading(RichText::new("Sassy Browser").size(48.0));
                ui.add_space(10.0);
                ui.label(RichText::new("Fast â€¢ Free â€¢ Handles Everything").size(16.0).color(Color32::GRAY));
                
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
                            self.guarded_navigate(&url);
                        }
                    }
                });
                
                ui.add_space(30.0);
                
                // Supported formats info
                ui.heading("Native File Support - 100+ Formats");
                ui.add_space(10.0);
                
                ui.horizontal_wrapped(|ui| {
                    ui.label("Images (PNG, JPG, RAW, PSD, SVG)");
                    ui.label("PDF");
                    ui.label("Office Docs (DOCX, ODT, RTF)");
                    ui.label("Spreadsheets (XLSX, CSV)");
                });
                
                ui.add_space(8.0);
                
                ui.horizontal_wrapped(|ui| {
                    ui.label("Scientific (PDB, MOL, XYZ, CIF)");
                    ui.label("Archive Files (ZIP, 7z, RAR, TAR)");
                    ui.label("3D-Rendered Models (OBJ, STL, GLTF)");
                    ui.label("Fonts (TTF, OTF, WOFF)");
                });
                
                ui.add_space(8.0);
                
                ui.horizontal_wrapped(|ui| {
                    ui.label("Audio (MP3, FLAC, WAV, OGG)");
                    ui.label("Video (MP4, MKV, WebM)");
                    ui.label("eBooks (EPUB, MOBI)");
                    ui.label("Code (100+ languages)");
                });
                
                ui.add_space(20.0);
                ui.label(RichText::new("Drag & drop any file or use File â†’ Open").italics().color(Color32::GRAY));
            });
        });
    }
    
    fn render_web_content(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, url: &str) {
        let url = url.to_string();
        let available = ui.available_size();
        let accent_warn = Color32::from_rgb(0xf7, 0x8c, 0x1f);
        // Dark/light mode background
        let bg = if self.dark_mode { Color32::from_gray(25) } else { Color32::WHITE };
        egui::Frame::none()
            .fill(bg)
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.set_min_size(available);
                // Render HTML with flagged link highlighting
                let smart_history = &self.smart_history;
                let profile_manager = &self.profile_manager;
                let blocklist = profile_manager.active_profile().map(|p| p.restrictions.blocklist.clone()).unwrap_or_default();
                let link_check = |href: &str| {
                    let domain = crate::family_profiles::extract_domain(href);
                    // Blocklist
                    if blocklist.contains(&domain) {
                        return Some("Blocklist");
                    }
                    // NSFW
                    let (is_nsfw, _score) = smart_history.analyze(href, "");
                    if is_nsfw {
                        return Some("NSFW");
                    }
                    // Ad/sponsored/spam/unsafe (simple heuristics)
                    let href_lc = crate::fontcase::ascii_lower(href);
                    if href_lc.contains("ad") || href_lc.contains("sponsored") {
                        return Some("Ad/Sponsored");
                    }
                    if href_lc.contains("spam") {
                        return Some("Spam");
                    }
                    if href_lc.contains("unsafe") {
                        return Some("Unsafe");
                    }
                    None
                };
                // Render with link check
                // Ensure renderer uses the same warning accent color
                self.html_renderer.set_warn_color(accent_warn);
                // Run extension content scripts/styles for this URL
                let scripts = self.extension_manager.get_content_scripts(&url);
                if !scripts.is_empty() {
                    self.html_renderer.run_content_scripts(&scripts);
                }
                let styles = self.extension_manager.get_content_styles(&url);
                if !styles.is_empty() {
                    self.html_renderer.apply_content_styles(&styles);
                }
                if let Some(doc) = self.html_renderer.cached_doc.clone() {
                    for node in &doc.nodes {
                        self.html_renderer.render_node_with_link_check(ui, node, &doc.styles, Some(&link_check), accent_warn);
                    }
                } else {
                    self.html_renderer.render(ui, &url);
                }
                // Handle clicked links
                if let Some(href) = self.html_renderer.take_clicked_link() {
                    self.guarded_navigate(&href);
                    self.html_renderer.clear_cache();
                }
                // Show console output if dev tools enabled
                if self.show_dev_tools {
                    ui.separator();
                    ui.collapsing("ðŸ“ Console", |ui| {
                        for line in self.html_renderer.console_output() {
                            ui.label(RichText::new(line).monospace().size(12.0));
                        }
                    });
                }
                // Show option to open in system browser
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("ðŸ’¡ Tip:").small().color(Color32::GRAY));
                    if ui.small_button("Open in system browser").clicked() {
                        let _ = open::that(&url);
                    }
                    ui.label(RichText::new("for full web experience").small().color(Color32::GRAY));
                });
            });
    }
    
    #[allow(dead_code)]
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
            ui.heading("âš™ï¸ Settings");
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
                if ui.button("Reapply").clicked() {
                    let current = self.engine.downloads.download_dir_buf();
                    self.engine.downloads.set_download_dir(current);
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
                ui.heading("ðŸ• History");
                
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
                        self.guarded_navigate(url);
                    }
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("{} visits", visits)).small().color(Color32::GRAY));
                    });
                });
            }
            
            ui.add_space(16.0);
            ui.heading("Smart History (last 20)");
            let smart_recent: Vec<_> = self.smart_history.recent(20).into_iter().cloned().collect();
            for entry in smart_recent {
                ui.horizontal(|ui| {
                    if ui.link(&entry.title).on_hover_text(&entry.url).clicked() {
                        self.guarded_navigate(&entry.url);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(entry.domain.clone()).small().color(Color32::GRAY));
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
            ui.heading("â\u{AD} Bookmarks");
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
                        self.guarded_navigate(url);
                    }
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Ã—").clicked() {
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

            ui.separator();
            ui.heading("Import / Export");
            ui.label("Import HTML (paste Netscape format)");
            ui.text_edit_multiline(&mut self.bookmark_import_buffer);
            if ui.small_button("Import").clicked() {
                match self.engine.bookmarks.import_html(&self.bookmark_import_buffer) {
                    Ok(count) => {
                        let _ = self.engine.bookmarks.save();
                        self.status_message = format!("Imported {} bookmarks", count);
                    }
                    Err(e) => {
                        self.status_message = format!("Import failed: {}", e);
                    }
                }
            }

            ui.label("Export HTML");
            if ui.small_button("Generate Export").clicked() {
                self.bookmark_export_buffer = self.engine.bookmarks.export_html();
                self.status_message = "Export buffer ready".into();
            }
            ui.text_edit_multiline(&mut self.bookmark_export_buffer);
            if ui.small_button("Save Export to File").clicked() {
                let export_path = dirs::data_local_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join("sassy-browser")
                    .join("bookmarks-export.html");
                if let Some(parent) = export_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                match std::fs::write(&export_path, &self.bookmark_export_buffer) {
                    Ok(_) => self.status_message = format!("Saved export to {}", export_path.display()),
                    Err(e) => self.status_message = format!("Save failed: {}", e),
                }
            }
        });
    }
    
    fn render_downloads_page(&mut self, ui: &mut egui::Ui) {
        use crate::family_profiles::{ApprovalStatus, Action};
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Downloads");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let active = self.engine.downloads.has_active_downloads();
                    if active {
                        ui.label(RichText::new("Active").small().color(Color32::from_rgb(0x16, 0xf2, 0xd6)));
                    }
                    if ui.button("Clear Completed").on_hover_text("Remove finished downloads from the list").clicked() {
                        self.engine.downloads.clear_finished();
                    }
                });
            });

            ui.horizontal(|ui| {
                ui.label("URL");
                ui.text_edit_singleline(&mut self.download_url_input).on_hover_text("Paste a direct download link here");
                if ui.button("Start").on_hover_text("Request download (may require approval)").clicked() && !self.download_url_input.is_empty() {
                    let url = self.download_url_input.clone();
                    self.guard_download_request(&url, None);
                    self.download_url_input.clear();
                }
            });
            ui.separator();

            // Show pending/denied/expired approval requests for downloads
            let approval_requests: Vec<_> = self.profile_manager.all_approval_requests()
                .iter()
                .filter(|r| matches!(&r.action, Action::Download { .. }))
                .cloned()
                .collect();

            if !approval_requests.is_empty() {
                ui.heading("Download Approvals");
                for req in &approval_requests {
                    let (status_text, color, tooltip) = match req.status {
                        ApprovalStatus::Pending => ("Pending", Color32::YELLOW, "Waiting for parent approval"),
                        ApprovalStatus::Approved => ("Approved", Color32::from_rgb(0x16, 0xf2, 0xd6), "Approved by parent"),
                        ApprovalStatus::Denied => ("Denied", Color32::RED, req.parent_response.as_deref().unwrap_or("Denied by parent")),
                        ApprovalStatus::Expired => ("Expired", Color32::GRAY, "Approval request expired"),
                    };
                    ui.horizontal(|ui| {
                        if let Action::Download { filename, size, .. } = &req.action {
                            ui.label(RichText::new(filename).strong());
                            ui.label(format!("{:.1} MB", *size as f32 / 1_048_576.0));
                            ui.label(RichText::new(status_text).color(color)).on_hover_text(tooltip);
                            if (req.status == ApprovalStatus::Denied || req.status == ApprovalStatus::Expired)
                                && ui.button("Resubmit").on_hover_text("Request approval again").clicked() {
                                    let new_id = self.profile_manager.request_approval(req.action.clone());
                                    self.status_message = format!("Resubmitted approval request: {}", new_id);
                                }
                            if let Some(resp) = &req.parent_response {
                                if req.status == ApprovalStatus::Denied {
                                    ui.label(RichText::new(format!("Reason: {}", resp)).color(Color32::RED)).on_hover_text("Parent's reason for denial");
                                }
                            }
                        }
                    });
                }
                ui.separator();
            }

            let downloads = self.engine.downloads.downloads();
            for download in &downloads {
                ui.horizontal(|ui| {
                    ui.label(&download.filename);
                    if let Some(total) = download.total_bytes {
                        ui.label(format!("{:.1} MB", total as f32 / 1_048_576.0));
                    }
                    if let Some(ct) = &download.mime_type {
                        ui.label(RichText::new(ct).color(Color32::GRAY));
                    }
                    match download.state {
                        crate::browser::DownloadState::Downloading => {
                            ui.add(egui::ProgressBar::new(download.progress()).show_percentage());
                            ui.label(format!("{:.1} KB/s", download.speed_bps() / 1024.0));
                            if ui.small_button("Cancel").on_hover_text("Cancel this download").clicked() {
                                self.engine.downloads.cancel_download(download.id);
                            }
                        }
                        crate::browser::DownloadState::Completed => {
                            ui.label(RichText::new("Complete").color(Color32::from_rgb(0x16, 0xf2, 0xd6)));
                            if ui.small_button("Open").on_hover_text("Open the downloaded file").clicked() {
                                let _ = open::that(&download.save_path);
                            }
                            if ui.small_button("Show in Folder").on_hover_text("Show file in folder").clicked() {
                                let _ = open::that(download.save_path.parent().unwrap_or(&download.save_path));
                            }
                        }
                        crate::browser::DownloadState::Failed => {
                            ui.label(RichText::new("Failed").color(Color32::RED)).on_hover_text("Download failed");
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
                        if ui.button("Ã—").clicked() {
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
                            if let Some(total) = download.total_bytes {
                                ui.label(format!("{:.1} MB", total as f32 / 1_048_576.0));
                            }
                            match download.state {
                                crate::browser::DownloadState::Downloading => {
                                    ui.add(egui::ProgressBar::new(download.progress())
                                        .desired_width(200.0)
                                        .show_percentage());
                                    ui.label(format!("{:.1} KB/s", download.speed_bps() / 1024.0));
                                    if ui.small_button("Cancel").clicked() {
                                        self.engine.downloads.cancel_download(download.id);
                                    }
                                }
                                crate::browser::DownloadState::Completed => {
                                    ui.label(RichText::new("Complete").color(Color32::from_rgb(0x16, 0xf2, 0xd6)));
                                    if ui.small_button("Open").clicked() {
                                        let _ = open::that(&download.save_path);
                                    }
                                    if ui.small_button("Show in Folder").clicked() {
                                        let _ = open::that(download.save_path.parent().unwrap_or(&download.save_path));
                                    }
                                }
                                crate::browser::DownloadState::Failed => {
                                    ui.label(RichText::new("Failed").color(Color32::RED));
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
                    // Show time left for restricted profiles
                    if let Some(profile) = self.profile_manager.active_profile() {
                        if profile.is_restricted() {
                            if let Some(mins) = self.profile_manager.remaining_time_minutes() {
                                ui.label(RichText::new(format!("Time left: {} min", mins)).color(Color32::YELLOW));
                            }
                        }
                    }
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
                    let addr = self.engine.address_bar_text().to_string();
                    self.focus_manager.focus_address_bar(&addr);
                    self.engine.set_address_bar_text(addr);
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
                    self.guarded_navigate("sassy://history");
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

    fn render_vault_panel(&mut self, ctx: &egui::Context) {
        if !self.vault_panel_visible {
            return;
        }

        if self.vault_needs_reauth {
            self.password_vault.lock();
            self.vault_needs_reauth = false;
            self.vault_status = "Vault locked - reauthenticate".into();
        }

        egui::Window::new("ðŸ” Password Vault")
            .open(&mut self.vault_panel_visible)
            .resizable(true)
            .default_size(Vec2::new(520.0, 520.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Vault");
                    if !self.vault_status.is_empty() {
                        ui.label(RichText::new(&self.vault_status).color(Color32::from_rgb(120, 200, 255)));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Lock").clicked() {
                            self.password_vault.lock();
                            self.vault_status = "Locked".into();
                        }
                    });
                });

                ui.separator();

                if !self.password_vault.is_setup() {
                    ui.label("Set a PIN to initialize the vault");
                    ui.horizontal(|ui| {
                        ui.label("PIN:");
                        ui.add(egui::TextEdit::singleline(&mut self.vault_pin_input).password(true));
                        if ui.button("Set PIN").clicked() {
                            match self.password_vault.setup(&self.vault_pin_input) {
                                Ok(_) => {
                                    self.vault_status = "Vault set up and unlocked".into();
                                    self.vault_pin_input.clear();
                                }
                                Err(e) => self.vault_status = format!("Setup failed: {}", e),
                            }
                        }
                    });
                    return;
                }

                if !self.password_vault.is_unlocked() {
                    ui.label("Vault locked. Enter PIN to unlock.");
                    ui.horizontal(|ui| {
                        ui.label("PIN:");
                        ui.add(egui::TextEdit::singleline(&mut self.vault_pin_input).password(true));
                        if ui.button("Unlock").clicked() {
                            match self.password_vault.unlock(&self.vault_pin_input) {
                                Ok(_) => {
                                    self.vault_status = "Unlocked".into();
                                    self.vault_pin_input.clear();
                                }
                                Err(e) => self.vault_status = format!("Unlock failed: {}", e),
                            }
                        }
                    });
                    return;
                }

                // Unlocked: show actions
                self.password_vault.touch();

                ui.horizontal(|ui| {
                    if ui.button("Generate Password").clicked() {
                        let pw = generate_password(&self.password_generator_opts);
                        self.vault_new_password = pw.clone();
                        self.generated_password = pw;
                        self.vault_status = "Password generated".into();
                    }
                    let mut breach_enabled = self.password_vault.breach_check_enabled();
                    if ui.checkbox(&mut breach_enabled, "Breach check").changed() {
                        self.password_vault.set_breach_check_enabled(breach_enabled);
                        let _ = self.password_vault.save();
                    }
                    if ui.button("Export CSV").clicked() {
                        match self.password_vault.export_csv() {
                            Ok(csv) => {
                                self.vault_export_buffer = csv;
                                self.vault_status = "Exported".into();
                            }
                            Err(e) => self.vault_status = format!("Export failed: {}", e),
                        }
                    }
                    if ui.button("Import CSV").clicked() {
                        match self.password_vault.import_csv(&self.vault_import_buffer) {
                            Ok(count) => self.vault_status = format!("Imported {} entries", count),
                            Err(e) => self.vault_status = format!("Import failed: {}", e),
                        }
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Auto-lock (secs)");
                    let mut auto_lock = self.password_vault.auto_lock_seconds() as f32;
                    if ui.add(egui::Slider::new(&mut auto_lock, 30.0..=86400.0).logarithmic(true)).changed() {
                        let secs = auto_lock.round() as u64;
                        match self.password_vault.set_auto_lock_seconds(secs) {
                            Ok(_) => self.vault_status = format!("Auto-lock set to {}s", secs),
                            Err(e) => self.vault_status = format!("Auto-lock update failed: {}", e),
                        }
                    }
                });

                ui.collapsing("Add Credential", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Title");
                        ui.text_edit_singleline(&mut self.vault_new_title);
                        ui.label("User");
                        ui.text_edit_singleline(&mut self.vault_new_username);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Password");
                        ui.text_edit_singleline(&mut self.vault_new_password);
                        if ui.small_button("Generate").clicked() {
                            let pw = generate_password(&self.password_generator_opts);
                            self.vault_new_password = pw.clone();
                            self.generated_password = pw;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("URL");
                        ui.text_edit_singleline(&mut self.vault_new_url);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Folder");
                        ui.text_edit_singleline(&mut self.vault_new_folder);
                    });
                    ui.label("Notes");
                    ui.text_edit_multiline(&mut self.vault_new_notes);

                    if ui.button("Add").clicked() {
                        let mut cred = Credential::new(
                            &self.vault_new_title,
                            &self.vault_new_username,
                            &self.vault_new_password,
                            &self.vault_new_url,
                        );
                        cred.notes = self.vault_new_notes.clone();
                        if !self.vault_new_folder.trim().is_empty() {
                            cred.folder = Some(self.vault_new_folder.clone());
                        }
                        if let Err(e) = self.password_vault.add(cred) {
                            self.vault_status = format!("Add failed: {}", e);
                        } else {
                            self.vault_status = "Added".into();
                            self.vault_new_title.clear();
                            self.vault_new_username.clear();
                            self.vault_new_password.clear();
                            self.vault_new_url.clear();
                            self.vault_new_notes.clear();
                        }
                    }
                });

                if let Some(edit_id) = self.vault_editing_id.clone() {
                    ui.separator();
                    ui.collapsing("Edit Credential", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Title");
                            ui.text_edit_singleline(&mut self.vault_edit_title);
                            ui.label("User");
                            ui.text_edit_singleline(&mut self.vault_edit_username);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Password");
                            ui.text_edit_singleline(&mut self.vault_edit_password);
                            if ui.small_button("Copy").clicked() {
                                ui.output_mut(|o| o.copied_text = self.vault_edit_password.clone());
                                self.vault_status = "Password copied".into();
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("URL");
                            ui.text_edit_singleline(&mut self.vault_edit_url);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Folder");
                            ui.text_edit_singleline(&mut self.vault_edit_folder);
                        });
                        ui.label("Notes");
                        ui.text_edit_multiline(&mut self.vault_edit_notes);

                        ui.horizontal(|ui| {
                            if ui.button("Save").clicked() {
                                if let Some(existing) = self.password_vault.get(&edit_id) {
                                    let mut updated = existing.clone();
                                    updated.title = self.vault_edit_title.clone();
                                    updated.username = self.vault_edit_username.clone();
                                    updated.password = self.vault_edit_password.clone();
                                    updated.url = self.vault_edit_url.clone();
                                    updated.notes = self.vault_edit_notes.clone();
                                    updated.folder = if self.vault_edit_folder.trim().is_empty() {
                                        None
                                    } else {
                                        Some(self.vault_edit_folder.clone())
                                    };

                                    match self.password_vault.update(&edit_id, updated) {
                                        Ok(_) => {
                                            self.vault_status = "Updated".into();
                                            self.vault_editing_id = None;
                                        }
                                        Err(e) => self.vault_status = format!("Update failed: {}", e),
                                    }
                                } else {
                                    self.vault_status = "Credential not found".into();
                                }
                            }
                            if ui.button("Cancel").clicked() {
                                self.vault_editing_id = None;
                            }
                        });
                    });
                }

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.text_edit_singleline(&mut self.vault_search_query);
                    if ui.button("Clear").clicked() {
                        self.vault_search_query.clear();
                    }
                    if ui.checkbox(&mut self.vault_autofill_enabled, "Inline autofill suggestions").clicked() {
                        self.vault_status = if self.vault_autofill_enabled { "Autofill suggestions on".into() } else { "Autofill suggestions off".into() };
                    }
                    ui.label("Folder");
                    egui::ComboBox::from_label("")
                        .selected_text(if self.vault_folder_filter.is_empty() { "All" } else { self.vault_folder_filter.as_str() })
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(self.vault_folder_filter.is_empty(), "All").clicked() {
                                self.vault_folder_filter.clear();
                            }
                            for folder in self.password_vault.folders() {
                                let selected = self.vault_folder_filter == *folder;
                                if ui.selectable_label(selected, folder).clicked() {
                                    self.vault_folder_filter = folder.clone();
                                }
                            }
                        });
                });

                // Gather data snapshots to avoid borrow conflicts
                let all_creds: Vec<Credential> = self.password_vault.all().to_vec();
                let folder_filter = self.vault_folder_filter.clone();
                let mut search_results: Vec<Credential> = if self.vault_search_query.is_empty() {
                    all_creds.clone()
                } else {
                    self.password_vault.search(&self.vault_search_query).into_iter().cloned().collect()
                };
                if !folder_filter.is_empty() {
                    search_results.retain(|c| c.folder.as_deref() == Some(folder_filter.as_str()));
                }
                let favorites: Vec<Credential> = self.password_vault.favorites().into_iter().cloned().collect();
                let weak: Vec<Credential> = self.password_vault.weak_passwords().into_iter().cloned().collect();
                let recent: Vec<Credential> = self.password_vault.recently_used(6).into_iter().cloned().collect();
                let by_url_matches: Vec<Credential> = if let Some(tab) = self.engine.active_tab() {
                    if let TabContent::Web { url, .. } = &tab.content {
                        self.password_vault.find_for_url(url).into_iter().cloned().collect()
                    } else {
                        Vec::new()
                    }
                } else { Vec::new() };

                ui.collapsing("Matches Current Site", |ui| {
                    if by_url_matches.is_empty() {
                        ui.label("None");
                    } else {
                        for cred in &by_url_matches {
                            ui.horizontal(|ui| {
                                ui.label(&cred.title);
                                ui.label(RichText::new(&cred.username).color(Color32::from_rgb(150, 200, 255)));
                                if ui.small_button("Copy User").clicked() {
                                    ui.output_mut(|o| o.copied_text = cred.username.clone());
                                    let _ = self.password_vault.mark_used(&cred.id);
                                    self.vault_status = "Username copied".into();
                                }
                                if ui.small_button("Copy Pass").clicked() {
                                    ui.output_mut(|o| o.copied_text = cred.password.clone());
                                    let _ = self.password_vault.mark_used(&cred.id);
                                    self.vault_status = "Password copied".into();
                                }
                            });
                        }
                    }
                });

                ui.collapsing("Search Results", |ui| {
                    if search_results.is_empty() {
                        ui.label("No results");
                    }
                    for cred in &search_results {
                        ui.horizontal(|ui| {
                            ui.label(&cred.title);
                            ui.label(RichText::new(&cred.username).color(Color32::GRAY));
                            if ui.small_button("Copy User").clicked() {
                                ui.output_mut(|o| o.copied_text = cred.username.clone());
                                let _ = self.password_vault.mark_used(&cred.id);
                                self.vault_status = "Username copied".into();
                            }
                            if ui.small_button("Copy Pass").clicked() {
                                ui.output_mut(|o| o.copied_text = cred.password.clone());
                                let _ = self.password_vault.mark_used(&cred.id);
                                self.vault_status = "Password copied".into();
                            }
                            if ui.small_button("Edit").clicked() {
                                self.vault_editing_id = Some(cred.id.clone());
                                self.vault_edit_title = cred.title.clone();
                                self.vault_edit_username = cred.username.clone();
                                self.vault_edit_password = cred.password.clone();
                                self.vault_edit_url = cred.url.clone();
                                self.vault_edit_notes = cred.notes.clone();
                                self.vault_edit_folder = cred.folder.clone().unwrap_or_default();
                            }
                            if ui.small_button("Delete").clicked() {
                                match self.password_vault.delete(&cred.id) {
                                    Ok(_) => self.vault_status = "Deleted".into(),
                                    Err(e) => self.vault_status = format!("Delete failed: {}", e),
                                }
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(cred.domain());
                            });
                        });
                    }
                });

                ui.collapsing("Recently Used", |ui| {
                    if recent.is_empty() {
                        ui.label("No recent activity");
                    }
                    for cred in &recent {
                        ui.horizontal(|ui| {
                            ui.label(&cred.title);
                            if let Some(ts) = cred.last_used {
                                ui.label(RichText::new(format!("Last used: {}", ts)).color(Color32::GRAY));
                            }
                        });
                    }
                });

                ui.collapsing("By Folder", |ui| {
                    for folder in self.password_vault.folders() {
                        let creds = self.password_vault.by_folder(Some(folder));
                        ui.collapsing(folder.clone(), |ui| {
                            if creds.is_empty() {
                                ui.label("(empty)");
                            }
                            for cred in creds {
                                ui.horizontal(|ui| {
                                    ui.label(&cred.title);
                                    ui.label(RichText::new(&cred.username).color(Color32::GRAY));
                                });
                            }
                        });
                    }
                });

                ui.collapsing("Favorites", |ui| {
                    if favorites.is_empty() {
                        ui.label("No favorites");
                    }
                    for cred in &favorites {
                        ui.label(&cred.title);
                    }
                });

                ui.collapsing("Weak Passwords", |ui| {
                    if weak.is_empty() {
                        ui.label("None");
                    }
                    for cred in &weak {
                        ui.label(&cred.title);
                    }
                });

                let reused_owned: std::collections::HashMap<String, Vec<Credential>> =
                    self.password_vault
                        .reused_passwords()
                        .into_iter()
                        .map(|(hash, creds)| (hash, creds.into_iter().cloned().collect()))
                        .collect();

                ui.collapsing("Reused Passwords", |ui| {
                    if reused_owned.is_empty() {
                        ui.label("None");
                    }
                    for creds in reused_owned.values() {
                        if creds.len() > 1 {
                            ui.label(format!("{} entries share a password", creds.len()));
                            for cred in creds {
                                ui.label(format!("- {} ({})", cred.title, cred.username));
                            }
                        }
                    }
                });

                ui.collapsing("CSV Export", |ui| {
                    ui.add(egui::TextEdit::multiline(&mut self.vault_export_buffer).desired_rows(4));
                });

                ui.collapsing("CSV Import", |ui| {
                    ui.add(egui::TextEdit::multiline(&mut self.vault_import_buffer).desired_rows(4));
                });
            });
    }

    fn render_profiles_panel(&mut self, ctx: &egui::Context) {
        if !self.profiles_panel_visible {
            return;
        }

        // Clone snapshots to avoid borrow conflicts while we render
        let profiles_snapshot: Vec<Profile> = self.profile_manager.profiles().to_vec();
        let active_profile_id = self.profile_manager.active_profile().map(|p| p.id.clone());

        egui::Window::new("ðŸ‘¨â€ðŸ‘©â€ðŸ‘§ Family Profiles")
            .open(&mut self.profiles_panel_visible)
            .resizable(true)
            .default_size(Vec2::new(640.0, 520.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Family Profiles");
                    if !self.profile_status.is_empty() {
                        ui.label(RichText::new(&self.profile_status).color(Color32::from_rgb(150, 200, 255)));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Reset Daily Stats").clicked() {
                            self.profile_manager.reset_daily_stats();
                            self.profile_status = "Daily stats reset".into();
                        }
                    });
                });

                ui.separator();

                // Bootstrap: require an admin first
                if profiles_snapshot.is_empty() {
                    ui.label("Create an admin profile to enable restrictions.");
                    ui.horizontal(|ui| {
                        ui.label("Name");
                        ui.text_edit_singleline(&mut self.profile_new_name);
                        ui.label("PIN");
                        ui.add(egui::TextEdit::singleline(&mut self.profile_new_pin).password(true));
                        if ui.button("Create Admin").clicked() {
                            let id = self.profile_manager.create_admin(&self.profile_new_name, &self.profile_new_pin);
                            let _ = self.profile_manager.switch_profile(&id, Some(&self.profile_new_pin));
                            self.profile_status = "Admin created and active".into();
                            self.profile_new_name.clear();
                            self.profile_new_pin.clear();
                        }
                    });
                    return;
                }

                // Active profile summary
                let active_profile_info = self.profile_manager.active_profile().cloned();
                ui.horizontal(|ui| {
                    if let Some(profile) = active_profile_info.as_ref() {
                        let color = profile.profile_type.color();
                        ui.colored_label(Color32::from_rgb(color[0], color[1], color[2]), profile.profile_type.icon());
                        ui.label(format!("Active: {} ({:?})", profile.name, profile.profile_type))
                            .on_hover_text("Currently active profile. Click 'Switch' below to change.");
                        if let Some(rem) = self.profile_manager.remaining_time_minutes() {
                            let warn = rem <= 10;
                            ui.label(RichText::new(format!("Time left today: {} min", rem))
                                .color(if warn { Color32::YELLOW } else { Color32::GREEN }))
                                .on_hover_text("Daily time limit. When time runs out, access will be blocked until tomorrow or a parent extends time.");
                        }
                        if let Some(r) = &profile.restrictions.bedtime_start {
                            if let Some(e) = &profile.restrictions.bedtime_end {
                                ui.label(RichText::new(format!("Bedtime: {:02}:{:02}â€“{:02}:{:02}", r.0, r.1, e.0, e.1)).color(Color32::LIGHT_RED))
                                    .on_hover_text("No access allowed during bedtime hours. Ask a parent to adjust bedtime if needed.");
                            }
                        }
                        if profile.is_restricted() && profile.requires_approval(&Action::InstallExtension) {
                            ui.label(RichText::new("Approvals required for extensions").color(Color32::YELLOW))
                                .on_hover_text("This profile needs parent approval for installing extensions.");
                        }
                    } else {
                        ui.label("No active profile");
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(id) = active_profile_id.clone() {
                            if ui.small_button("Weekly Report").on_hover_text("Show usage and blocked activity for this profile").clicked() {
                                if let Some(report) = self.profile_manager.generate_weekly_report(&id) {
                                    self.profile_report_output = format!(
                                        "{}: {} mins, blocked {} sites, {} downloads",
                                        report.profile_name,
                                        report.total_time_minutes,
                                        report.blocked_attempts,
                                        report.downloads
                                    );
                                } else {
                                    self.profile_report_output.clear();
                                }
                            }
                        }
                    });
                });

                if !self.profile_report_output.is_empty() {
                    ui.label(RichText::new(&self.profile_report_output).color(Color32::from_rgb(180, 220, 180)));
                }

                ui.separator();

                ui.collapsing("Create Profile", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Name");
                        ui.text_edit_singleline(&mut self.profile_new_name);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Type");
                        for variant in [ProfileType::Admin, ProfileType::Adult, ProfileType::Teen, ProfileType::Kid] {
                            let label = format!("{} {:?}", variant.icon(), variant);
                            if ui.selectable_label(self.profile_new_type == variant, label).clicked() {
                                self.profile_new_type = variant;
                            }
                        }
                    });

                    // Parent selection for Teen/Kid
                    if matches!(self.profile_new_type, ProfileType::Teen | ProfileType::Kid) {
                        let parents: Vec<_> = profiles_snapshot.iter()
                            .filter(|p| matches!(p.profile_type, ProfileType::Admin | ProfileType::Adult))
                            .collect();
                        ui.horizontal(|ui| {
                            ui.label("Parent");
                            egui::ComboBox::from_id_salt("parent_choice")
                                .selected_text(
                                    self.profile_parent_choice
                                        .as_ref()
                                        .and_then(|id| parents.iter().find(|p| p.id == *id))
                                        .map(|p| p.name.clone())
                                        .unwrap_or_else(|| "Select".into())
                                )
                                .show_ui(ui, |ui| {
                                    for p in parents {
                                        ui.selectable_value(&mut self.profile_parent_choice, Some(p.id.clone()), p.name.clone());
                                    }
                                });
                        });
                    }

                    if matches!(self.profile_new_type, ProfileType::Admin) {
                        ui.horizontal(|ui| {
                            ui.label("PIN");
                            ui.add(egui::TextEdit::singleline(&mut self.profile_new_pin).password(true));
                        });
                    }

                    if ui.button("Create").clicked() {
                        match self.profile_new_type.clone() {
                            ProfileType::Admin => {
                                let id = self.profile_manager.create_admin(&self.profile_new_name, &self.profile_new_pin);
                                self.profile_status = format!("Admin {} created", self.profile_new_name);
                                let _ = self.profile_manager.switch_profile(&id, Some(&self.profile_new_pin));
                            }
                            other => {
                                match self.profile_manager.create_profile(
                                    &self.profile_new_name,
                                    other,
                                    self.profile_parent_choice.as_deref(),
                                ) {
                                    Ok(id) => {
                                        self.profile_status = format!("Profile {} created", self.profile_new_name);
                                        // Auto-switch if unrestricted
                                        let _ = self.profile_manager.switch_profile(&id, None);
                                    }
                                    Err(e) => self.profile_status = format!("Create failed: {}", e),
                                }
                            }
                        }

                        self.profile_new_name.clear();
                        self.profile_new_pin.clear();
                        self.profile_parent_choice = None;
                    }
                });

                ui.separator();

                ui.collapsing("Active Restrictions", |ui| {
                    if let Some(id) = active_profile_id.clone() {
                        let mut log_search = false;
                        let mut test_download = false;

                        if let Some(profile) = self.profile_manager.get_profile_mut(&id) {
                            let r = &mut profile.restrictions;
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut r.block_nsfw, "Block NSFW");
                                ui.checkbox(&mut r.block_gambling, "Block gambling");
                                ui.checkbox(&mut r.block_social_media, "Block social");
                            });
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut r.use_allowlist_only, "Allowlist only");
                                ui.checkbox(&mut r.block_executable_downloads, "Block executables");
                                ui.checkbox(&mut r.downloads_need_approval, "Downloads need approval");
                            });
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut r.safe_search_enforced, "SafeSearch");
                                ui.checkbox(&mut r.block_extension_install, "Block extensions");
                                ui.checkbox(&mut r.block_settings_access, "Block settings");
                            });

                            ui.horizontal(|ui| {
                                ui.label("Add to allowlist");
                                ui.text_edit_singleline(&mut self.profile_allow_input);
                                if ui.small_button("Add").clicked() && !self.profile_allow_input.is_empty() {
                                    r.allowlist.insert(self.profile_allow_input.clone());
                                    self.profile_allow_input.clear();
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label("Add to blocklist");
                                ui.text_edit_singleline(&mut self.profile_block_input);
                                if ui.small_button("Add").clicked() && !self.profile_block_input.is_empty() {
                                    r.blocklist.insert(self.profile_block_input.clone());
                                    self.profile_block_input.clear();
                                }
                            });

                            ui.label(format!("Allowlist entries: {} | Blocklist entries: {}", r.allowlist.len(), r.blocklist.len()));

                            ui.horizontal(|ui| {
                                ui.label("Log search query");
                                ui.text_edit_singleline(&mut self.profile_search_log_input);
                                if ui.small_button("Log").clicked() && !self.profile_search_log_input.is_empty() {
                                    log_search = true;
                                }
                            });

                            if ui.small_button("Use Teen Defaults").clicked() {
                                *r = ProfileType::Teen.default_restrictions();
                            }
                            if ui.small_button("Use Kid Defaults").clicked() {
                                *r = ProfileType::Kid.default_restrictions();
                            }

                            if ui.small_button("Test download guard (setup.exe 50MB)").clicked() {
                                test_download = true;
                            }
                        }

                        if log_search {
                            self.profile_manager.record_search(&self.profile_search_log_input);
                            self.profile_search_log_input.clear();
                            self.profile_status = "Search recorded".into();
                        }

                        if test_download {
                            match self.profile_manager.can_download("setup.exe", 50 * 1024 * 1024) {
                                Ok(_) => self.profile_status = "Download allowed".into(),
                                Err(e) => self.profile_status = format!("Download blocked: {}", e),
                            }
                        }
                    }
                });

                ui.collapsing("Approvals", |ui| {
                    if let Some(active) = active_profile_info.as_ref() {
                        if active.is_restricted() {
                            ui.horizontal(|ui| {
                                ui.label("Request reason/site");
                                ui.text_edit_singleline(&mut self.profile_request_reason);
                                if ui.small_button("Request Access").clicked() {
                                    let action = Action::AccessBlockedSite(self.profile_request_reason.clone());
                                    let id = self.profile_manager.request_approval(action);
                                    self.profile_status = format!("Requested approval ({})", id);
                                }
                            });
                        }
                    }

                    // Parent actions: approve/deny pending
                    let pending: Vec<_> = self.profile_manager.pending_requests()
                        .into_iter()
                        .cloned()
                        .collect();
                    if pending.is_empty() {
                        ui.label("No pending requests");
                    } else {
                        let parent_id = profiles_snapshot.iter()
                            .find(|p| matches!(p.profile_type, ProfileType::Admin | ProfileType::Adult))
                            .map(|p| p.id.clone());
                        for req in &pending {
                            ui.horizontal(|ui| {
                                ui.label(format!("{} from {}", match &req.action {
                                    Action::AccessBlockedSite(url) => format!("Access {}", url),
                                    Action::Download { filename, .. } => format!("Download {}", filename),
                                    Action::InstallExtension => "Install extension".into(),
                                    Action::ChangeSettings => "Change settings".into(),
                                    Action::ExtendTimeLimit { minutes } => format!("Extend by {} min", minutes),
                                    Action::AddToAllowlist(site) => format!("Allow {}", site),
                                    Action::RemoveFromBlocklist(site) => format!("Unblock {}", site),
                                }, req.profile_id));
                                if let Some(parent) = parent_id.as_ref() {
                                    if ui.small_button("Approve").clicked() {
                                        let _ = self.profile_manager.approve_request(&req.id, parent);
                                        self.profile_status = "Request approved".into();
                                    }
                                    if ui.small_button("Deny").clicked() {
                                        let _ = self.profile_manager.deny_request(&req.id, parent, Some("Denied"));
                                        self.profile_status = "Request denied".into();
                                    }
                                }
                            });
                        }
                    }
                });

                ui.separator();

                ui.collapsing("Profiles", |ui| {
                    for profile in &profiles_snapshot {
                        ui.separator();
                        let color = profile.profile_type.color();
                        ui.horizontal(|ui| {
                            ui.colored_label(Color32::from_rgb(color[0], color[1], color[2]), profile.profile_type.icon());
                            ui.label(format!("{} ({:?})", profile.name, profile.profile_type))
                                .on_hover_text("Profile type determines restrictions and required approvals.");
                            ui.label(format!("Visits tracked: {} | Blocked: {}", profile.usage_stats.sites_visited.len(), profile.usage_stats.blocked_attempts.len()))
                                .on_hover_text("Number of sites visited and blocked for this profile.");
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("Switch").on_hover_text("Switch to this profile").clicked() {
                                    let pin_opt = if profile.pin.is_some() { Some(self.profile_switch_pin.as_str()) } else { None };
                                    if let Err(e) = self.profile_manager.switch_profile(&profile.id, pin_opt) {
                                        self.profile_status = format!("Switch failed: {}", e);
                                    } else {
                                        self.profile_status = format!("Switched to {}", profile.name);
                                    }
                                }
                            });
                        });

                        ui.horizontal(|ui| {
                            ui.label("PIN (if set)").on_hover_text("Enter PIN if this profile is protected");
                            ui.add(egui::TextEdit::singleline(&mut self.profile_switch_pin).password(true));
                        });

                        ui.horizontal(|ui| {
                            ui.label(format!("Total minutes: {}", profile.usage_stats.total_time_minutes))
                                .on_hover_text("Total usage time for this profile.");
                            ui.label(format!("Today: {}", profile.usage_stats.today_time_minutes))
                                .on_hover_text("Usage time today. Subject to daily limits.");
                        });

                        if ui.small_button("Record sample search").on_hover_text("Simulate a search for testing").clicked() {
                            self.profile_manager.record_search("sample query");
                            self.profile_status = "Sample search recorded".into();
                        }

                        if ui.small_button("Delete").on_hover_text("Delete this profile").clicked() {
                            if let Err(e) = self.profile_manager.delete_profile(&profile.id) {
                                self.profile_status = format!("Delete failed: {}", e);
                            } else {
                                self.profile_status = format!("Deleted {}", profile.name);
                            }
                        }
                    }
                });
            });
    }

    fn render_history_panel(&mut self, ctx: &egui::Context) {
        if !self.history_panel_visible {
            return;
        }

        // Snapshot immutable views before mutating
        let stats = self.smart_history.stats();
        let pending_count = self.smart_history.pending_count();
        let total_count = self.smart_history.total_count();
        let recent_entries: Vec<_> = self.smart_history.recent(10).into_iter().cloned().collect();
        let most_visited: Vec<_> = self.smart_history.most_visited(6).into_iter().cloned().collect();
        let search_results: Vec<_> = if self.history_search_query.is_empty() {
            Vec::new()
        } else {
            self.smart_history.search(&self.history_search_query).into_iter().cloned().collect()
        };
        let nsfw_entries: Vec<_> = self.smart_history.nsfw_entries().into_iter().cloned().collect();
        let syncable_count = self.smart_history.syncable().len();
        let abandoned: Vec<String> = self.smart_history.recent_abandoned(60).into_iter().map(|s| s.to_string()).collect();

        let hm_recent: Vec<_> = self.history_manager.recent(10).into_iter().cloned().collect();
        let hm_search: Vec<_> = if self.history_search_query.is_empty() {
            Vec::new()
        } else {
            self.history_manager.search(&self.history_search_query).into_iter().cloned().collect()
        };
        let hm_most: Vec<_> = self.history_manager.most_visited(6).into_iter().cloned().collect();
        let hm_day_results: Vec<_> = if !self.history_day_query.is_empty() {
            parse_ymd(&self.history_day_query)
                .map(|(y, m, d)| self.history_manager.for_date(y, m, d).into_iter().cloned().collect())
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        egui::Window::new("ðŸ• History")
            .open(&mut self.history_panel_visible)
            .resizable(true)
            .default_size(Vec2::new(760.0, 560.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("History & Activity");
                    if !self.history_status.is_empty() {
                        ui.label(RichText::new(&self.history_status).color(Color32::from_rgb(150, 200, 255)));
                    }
                });

                ui.separator();

                // Controls
                ui.horizontal(|ui| {
                    let mut incognito = self.smart_history.is_incognito();
                    if ui.checkbox(&mut incognito, "Incognito (don't track)").changed() {
                        self.smart_history.set_incognito(incognito);
                    }

                    let mut intent_delay = self.smart_history.intent_delay_secs();
                    if ui.add(egui::Slider::new(&mut intent_delay, 0.0..=30.0).text("Intent delay (s)")).changed() {
                        self.smart_history.set_intent_delay(intent_delay);
                    }

                    if ui.checkbox(&mut self.history_auto_exclude, "Auto-exclude NSFW").changed() {
                        self.smart_history.set_auto_exclude_nsfw(self.history_auto_exclude);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("NSFW sensitivity");
                    let mut sensitivity = self.history_nsfw_sensitivity;
                    if ui.add(egui::Slider::new(&mut sensitivity, 0.1..=1.0)).changed() {
                        self.history_nsfw_sensitivity = sensitivity;
                        self.smart_history.nsfw_detector().set_sensitivity(sensitivity);
                    }

                    ui.label("Block domain");
                    ui.text_edit_singleline(&mut self.history_domain_filter);
                    if ui.small_button("Delete domain").clicked() && !self.history_domain_filter.is_empty() {
                        self.smart_history.delete_for_domain(&self.history_domain_filter);
                        self.history_status = "Domain removed from history".into();
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Range start (epoch secs)");
                    ui.text_edit_singleline(&mut self.history_date_start);
                    ui.label("End");
                    ui.text_edit_singleline(&mut self.history_date_end);
                    if ui.small_button("Delete range").clicked() {
                        if let (Ok(start), Ok(end)) = (self.history_date_start.parse::<u64>(), self.history_date_end.parse::<u64>()) {
                            self.smart_history.delete_range(start, end);
                            self.history_status = "Range deleted".into();
                        }
                    }
                    if ui.small_button("Clear NSFW").clicked() {
                        self.smart_history.clear_nsfw();
                        self.history_status = "NSFW cleared".into();
                    }
                    if ui.small_button("Clear All").clicked() {
                        self.smart_history.clear();
                        self.history_manager.clear();
                        self.history_status = "History cleared".into();
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Search");
                    ui.text_edit_singleline(&mut self.history_search_query);
                    if ui.button("Clear Search").clicked() {
                        self.history_search_query.clear();
                    }
                    ui.label("Day (YYYY-MM-DD)");
                    ui.text_edit_singleline(&mut self.history_day_query);
                });

                ui.horizontal(|ui| {
                    ui.label(format!("Smart: total {} | pending {} | syncable {} | NSFW {}", total_count, pending_count, syncable_count, nsfw_entries.len()));
                    ui.label(format!("Intent delay {:.1}s", self.smart_history.intent_delay_secs()));
                    ui.label(format!("Visits tracked: {}", stats.total_visits));
                });

                if let Some(tab) = self.engine.active_tab() {
                    if let TabContent::Web { url, title, .. } = &tab.content {
                        let url_clone = url.clone();
                        let raw_title = title.clone();
                        let history_title = if raw_title.is_empty() { tab.title() } else { raw_title.clone() };

                        ui.horizontal(|ui| {
                            ui.label("Active page controls");
                            if ui.small_button("Commit now").clicked() {
                                self.smart_history.force_commit(&url_clone);
                                self.history_manager.add(&url_clone, &history_title);
                                self.history_last_url = Some(url_clone.clone());
                                self.history_last_title = Some(raw_title.clone());
                                self.history_status = "Committed active page".into();
                            }
                            if ui.small_button("Mark left").clicked() {
                                self.smart_history.leave(&url_clone);
                                self.history_status = "Left active page".into();
                            }
                            if self.smart_history.was_recently_left(&url_clone) {
                                ui.label(RichText::new("Recently left").color(Color32::YELLOW));
                            }
                        });
                    }
                }

                ui.separator();

                ui.columns(2, |cols| {
                    let (left_slice, right_slice) = cols.split_at_mut(1);
                    let left = &mut left_slice[0];
                    let right = &mut right_slice[0];

                    left.heading("Smart History");
                    left.label(format!("Unique domains: {} | Pruned: {}", stats.unique_domains, stats.entries_pruned));

                    left.collapsing("Recent (intent-committed)", |ui| {
                        egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                            for entry in &recent_entries {
                                ui.horizontal(|ui| {
                                    ui.label(entry.title.clone());
                                    ui.label(RichText::new(&entry.domain).color(Color32::from_rgb(150, 150, 180)));
                                    ui.label(format!("{}s", entry.duration_secs.unwrap_or(0)));
                                });
                            }
                            if recent_entries.is_empty() {
                                ui.label("No committed entries yet");
                            }
                        });
                    });

                    left.collapsing("Most visited", |ui| {
                        for entry in &most_visited {
                            ui.label(format!("{} ({})", entry.title, entry.visit_count));
                        }
                        if most_visited.is_empty() {
                            ui.label("No data");
                        }
                    });

                    left.collapsing("Search", |ui| {
                        for entry in &search_results {
                            ui.label(format!("{} - {}", entry.title, entry.url));
                        }
                        if search_results.is_empty() {
                            ui.label("No matches");
                        }
                    });

                    left.collapsing("Abandoned (last 60s)", |ui| {
                        for url in &abandoned {
                            ui.label(url);
                        }
                        if abandoned.is_empty() {
                            ui.label("None");
                        }
                    });

                    left.collapsing("NSFW flagged", |ui| {
                        for entry in &nsfw_entries {
                            ui.horizontal(|ui| {
                                ui.label(format!("{} ({:.2})", entry.domain, entry.nsfw_confidence));
                                if ui.small_button("Exclude").clicked() {
                                    self.smart_history.exclude_domain(&entry.domain);
                                    self.history_status = format!("Excluded {}", entry.domain);
                                }
                                if ui.small_button("Include").clicked() {
                                    self.smart_history.include_domain(&entry.domain);
                                    self.history_status = format!("Included {}", entry.domain);
                                }
                            });
                        }
                        if nsfw_entries.is_empty() {
                            ui.label("None");
                        }
                    });

                    let right_stats_label = format!("HistoryManager entries: {}", self.history_manager.all().len());
                    right.heading("Classic History");
                    right.label(right_stats_label);

                    right.collapsing("Recent", |ui| {
                        for entry in &hm_recent {
                            ui.label(format!("{} - {}", entry.title, entry.url));
                        }
                        if hm_recent.is_empty() {
                            ui.label("No recent entries");
                        }
                    });

                    right.collapsing("Most visited", |ui| {
                        for entry in &hm_most {
                            ui.label(format!("{} ({})", entry.title, entry.visit_count));
                        }
                        if hm_most.is_empty() {
                            ui.label("No data");
                        }
                    });

                    right.collapsing("Search", |ui| {
                        for entry in &hm_search {
                            ui.label(format!("{} - {}", entry.title, entry.url));
                        }
                        if hm_search.is_empty() {
                            ui.label("No matches");
                        }
                    });

                    right.collapsing("Day query", |ui| {
                        for entry in &hm_day_results {
                            ui.label(format!("{} - {}", entry.title, entry.url));
                        }
                        if hm_day_results.is_empty() {
                            ui.label("No entries for day");
                        }
                    });
                });
            });
    }
    
    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            for file in &i.raw.dropped_files {
                if let Some(path) = &file.path {
                    if self.engine.open_file(path.clone()).is_ok() {
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
            if self.engine.open_file(path.clone()).is_ok() {
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
                ui.heading(RichText::new("ðŸ¦Š Sassy Browser").size(48.0).color(Color32::from_rgb(255, 140, 0)));
                ui.add_space(10.0);
                ui.label(RichText::new("Pure Rust â€¢ No Chrome â€¢ No Google â€¢ No Tracking").size(16.0).color(Color32::GRAY));
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
                            ui.label(RichText::new(" â†’ ").color(Color32::DARK_GRAY));
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
                    ("ðŸ”’", "100% Pure Rust", "No Chrome, no WebKit, no Google telemetry"),
                    ("ðŸ“", "200+ File Formats", "PDF, PDB, RAW photos, CAD files - all built-in"),
                    ("ðŸ’°", "Kills Paid Software", "Adobe Suite ($504/yr), AutoCAD ($2K/yr) - FREE"),
                    ("ðŸ”—", "Tailscale Mesh", "Sync across all your devices securely"),
                    ("ðŸ“±", "Phone App", "Pair your phone for seamless sync"),
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
        
        if ui.button(RichText::new("Get Started â†’").size(18.0)).clicked() {
            self.first_run.next_step();
        }
    }
    
    fn render_wizard_entropy(&mut self, ui: &mut egui::Ui) {
        if !self.first_run.entropy_seeded {
            self.auth.seed_entropy(&self.first_run.entropy_seed_label);
            self.first_run.entropy_seeded = true;
        }
        if self.first_run.entropy_started_at.is_none() {
            self.first_run.entropy_started_at = Some(Instant::now());
        }

        let elapsed = self.first_run
            .entropy_started_at
            .map(|t| t.elapsed())
            .unwrap_or_default();
        let remaining = self.first_run
            .entropy_min_seconds
            .saturating_sub(elapsed.as_secs());
        let timer_done = elapsed.as_secs_f32() >= self.first_run.entropy_min_seconds as f32;

        ui.heading("ðŸ” Creating Your Security Key");
        ui.add_space(20.0);
        
        ui.label("Move your mouse around to generate cryptographic entropy.");
        ui.label("This creates a unique 256-bit key that stays on YOUR device.");
        ui.label(format!(
            "Seeded with {} to start (keeps key strong even if idle)",
            self.first_run.entropy_seed_label
        ));
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

        ui.add_space(8.0);
        if timer_done {
            ui.label(RichText::new("Timer: done (30s minimum reached)").color(Color32::from_rgb(0, 200, 100)));
        } else {
            ui.label(RichText::new(format!("Timer: {}s remaining to harden the key", remaining)).color(Color32::YELLOW));
        }
        
        ui.add_space(20.0);
        
        // Manual entropy input
        ui.add_space(10.0);
        ui.label("Optional: Type random characters to add more entropy (keyboard mashing helps)");
        let mut manual_entropy = String::new();
        let response = ui.text_edit_singleline(&mut manual_entropy);
        if response.changed() && !manual_entropy.is_empty() {
            for c in manual_entropy.chars() {
                if !c.is_whitespace() {
                    self.auth.add_entropy_key();
                }
            }
        }

        // Visual entropy display
        egui::Frame::dark_canvas(ui.style()).show(ui, |ui| {
            ui.set_min_size(Vec2::new(400.0, 100.0));
                    // Show seed and backup options if ready
                    let ready = self.auth.is_entropy_ready();
                    let can_continue = ready && timer_done;
                    if can_continue {
                        ui.separator();
                        ui.label(RichText::new("Backup your key seed!").strong().color(Color32::YELLOW));
                        if let Some(seed) = self.auth.get_master_key() {
                            let seed_hex = hex::encode(seed);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&seed_hex).monospace());
                                if ui.small_button("Copy").clicked() {
                                    ui.output_mut(|o| o.copied_text = seed_hex.clone());
                                }
                                if ui.small_button("Show QR").clicked() {
                                    self.first_run.error_message = Some(seed_hex.clone()); // Use error_message as temp QR trigger
                                }
                            });
                            // Show QR code if requested
                            if let Some(ref qr) = self.first_run.error_message {
                                if qr == &seed_hex {
                                    if let Ok(code) = qrcode::QrCode::new(seed) {
                                        let image = code.render::<qrcode::render::svg::Color>()
                                            .min_dimensions(200, 200)
                                            .build();
                                        // Render SVG as text (egui can't natively render SVG, so show as text for now)
                                        ui.label(RichText::new(image).monospace().size(8.0));
                                    }
                                }
                            }
                        }
                    }
            
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
            if ui.button("â† Back").clicked() {
                self.first_run.prev_step();
            }
            
            ui.add_space(20.0);
            
            let ready = self.auth.is_entropy_ready();
            let can_continue = ready && timer_done;
            if ui.add_enabled(can_continue, egui::Button::new(RichText::new("Continue â†’").size(18.0))).clicked() {
                self.first_run.next_step();
            }
            
            if !ready {
                ui.label(RichText::new("Keep moving your mouse!").color(Color32::YELLOW));
            } else if !timer_done {
                ui.label(RichText::new("Timer still running for stronger key").color(Color32::YELLOW));
            }
        });
    }
    
    fn render_wizard_device(&mut self, ui: &mut egui::Ui) {
        ui.heading("ðŸ–¥ï¸ Name This Device");
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
                (DeviceType::Desktop, "ðŸ–¥ï¸ Desktop"),
                (DeviceType::Laptop, "ðŸ’» Laptop"),
                (DeviceType::Server, "ðŸ–§ Server"),
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
            if ui.button("â† Back").clicked() {
                self.first_run.prev_step();
            }
            
            ui.add_space(20.0);
            
            if ui.button(RichText::new("Create Device Key â†’").size(18.0)).clicked() {
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
        ui.heading("ðŸ”— Tailscale Setup");
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
                ui.label(RichText::new("âœ… Tailscale is connected!").color(Color32::from_rgb(0, 200, 100)));
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
                        let status = if peer.online { "ðŸŸ¢" } else { "âšª" };
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
            if ui.button("â† Back").clicked() {
                self.first_run.prev_step();
            }
            
            ui.add_space(20.0);
            
            let label = if self.first_run.enable_phone_sync {
                "Continue to Phone Setup â†’"
            } else {
                "Finish Setup â†’"
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
        ui.heading("ðŸ“± Phone App Pairing");
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
            if ui.button("â† Back").clicked() {
                self.first_run.prev_step();
            }
            
            ui.add_space(20.0);
            
            if ui.button(RichText::new("Finish Setup â†’").size(18.0)).clicked() {
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

        // Periodic bookkeeping for profiles and smart history
        self.profile_manager.record_activity();
        self.smart_history.tick();
        
        // Sync smart history lifecycle with the active tab
        let (active_url, active_title) = if let Some(Tab { content: TabContent::Web { url, title, .. }, .. }) = self.engine.active_tab() {
            (Some(url.clone()), Some(title.clone()))
        } else {
            (None, None)
        };

        if let Some(prev_url) = self.smart_history_active_url.take() {
            if Some(prev_url.clone()) != active_url {
                self.smart_history.leave(&prev_url);
            } else {
                self.smart_history_active_url = Some(prev_url);
            }
        }

        if let Some(url) = &active_url {
            if self.smart_history_active_url.as_ref() != Some(url) {
                let title = active_title.clone().unwrap_or_default();
                self.smart_history.visit(url, &title, None);
                self.smart_history_active_url = Some(url.clone());
            }
        }

        if let Some(url) = self.smart_history_active_url.as_deref() {
            self.smart_history.update_scroll(url, 0.9);
        }
        
        // Process any pending messages from webviews
        self.engine.process_messages();

        // Handle download requests after policy checks
        let pending_downloads = self.engine.take_pending_downloads();
        for (url, suggested) in pending_downloads {
            self.guard_download_request(&url, suggested.as_deref());
        }

        // Feed network monitor with active tab loading state
        if let Some(Tab { content: TabContent::Web { url, loading, .. }, .. }) = self.engine.active_tab() {
            if *loading {
                if self.network_active_connection.is_none() && !self.network_monitor.is_blocked(url) {
                    let conn_id = self.network_monitor.start_connection(url, ConnectionType::Document);
                    self.network_active_connection = Some(conn_id);
                    self.network_last_net_sample = Some(Instant::now());
                }

                if let Some(conn_id) = self.network_active_connection {
                    let now = Instant::now();
                    let last = self.network_last_net_sample.unwrap_or(now);
                    let secs = (now - last).as_secs_f64().max(0.016);
                    let bytes_down = (80_000.0 * secs) as u64;
                    let bytes_up = (2_000.0 * secs) as u64;
                    self.network_monitor.update_connection(conn_id, bytes_down, bytes_up);
                    self.network_last_net_sample = Some(now);
                }
            } else if let Some(conn_id) = self.network_active_connection.take() {
                self.network_monitor.complete_connection(conn_id, 200, Some("text/html".to_string()));
                self.network_last_net_sample = None;
            }
        } else {
            self.network_active_connection = None;
            self.network_last_net_sample = None;
        }

        self.sync_downloads_into_network_monitor();
        self.network_monitor.cleanup_old(Duration::from_secs(30));

        // Track last seen URL/title so HistoryManager gets real titles once available
        if let Some(tab) = self.engine.active_tab() {
            if let TabContent::Web { url, title, .. } = &tab.content {
                let url_clone = url.clone();
                let raw_title = title.clone();
                let history_title = if raw_title.is_empty() { tab.title() } else { raw_title.clone() };
                let changed = self.history_last_url.as_deref() != Some(url_clone.as_str())
                    || self.history_last_title.as_deref() != Some(raw_title.as_str());
                if changed {
                    self.history_manager.add(&url_clone, &history_title);
                    self.history_last_url = Some(url_clone);
                    self.history_last_title = Some(raw_title);
                }
            } else {
                self.history_last_url = None;
                self.history_last_title = None;
            }
        } else {
            self.history_last_url = None;
            self.history_last_title = None;
        }

        // Auto-lock vault if idle
        if self.password_vault.check_auto_lock() {
            self.vault_status = "Vault auto-locked".into();
        }
        
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

        // Auth / licensing panel
        self.render_auth_panel(ctx);

        // Profiles/parental controls panel
        self.render_profiles_panel(ctx);
        // Extensions manager panel
        self.render_extensions_panel(ctx);

        // History / activity panel
        self.render_history_panel(ctx);

        // Password vault panel
        self.render_vault_panel(ctx);
        
        // Status bar
        self.render_status_bar(ctx);
        
        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_content(ctx, ui);
        });
    }
}

fn parse_ymd(input: &str) -> Option<(i32, u32, u32)> {
    let parts: Vec<_> = input.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let year = parts[0].parse().ok()?;
    let month = parts[1].parse().ok()?;
    let day = parts[2].parse().ok()?;
    Some((year, month, day))
}

fn configure_fonts(ctx: &egui::Context) {
    // Embed Space Grotesk from the repo and make it the preferred
    // proportional font to avoid runtime lookups failing.
    let mut fonts = egui::FontDefinitions::default();
    let space_bytes = include_bytes!("../Space_Grotesk/static/SpaceGrotesk-Regular.ttf");
    // Register several common lookup keys that other code or libraries
    // might use when requesting the font.
    let fb = egui::FontData::from_static(space_bytes);
    let keys = [
        "Space Grotesk",
        "SpaceGrotesk",
        "Space Grotesk Regular",
        "SpaceGrotesk-Regular",
    ];
    for &k in &keys {
        fonts.font_data.insert(k.into(), fb.clone());
    }
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "Space Grotesk".into());

    // Debug: list registered font keys and proportional family order.
    tracing::info!("Registered font keys: {:?}", fonts.font_data.keys().cloned().collect::<Vec<_>>());
    if let Some(fam) = fonts.families.get(&egui::FontFamily::Proportional) {
        tracing::info!("Proportional family: {:?}", fam);
    }

    ctx.set_fonts(fonts);
}

fn configure_style(ctx: &egui::Context, dark_mode: bool) {
    // Palette: brutalist-tech hybrid
    let accent = Color32::from_rgb(0x16, 0xf2, 0xd6); // electric teal
    let accent_warn = Color32::from_rgb(0xf7, 0x8c, 0x1f);

    let mut visuals = if dark_mode { egui::Visuals::dark() } else { egui::Visuals::light() };

    if dark_mode {
        visuals.window_fill = Color32::from_rgb(0x0f, 0x11, 0x15);
        visuals.panel_fill = Color32::from_rgb(0x14, 0x18, 0x1f);
        visuals.extreme_bg_color = Color32::from_rgb(0x0b, 0x0d, 0x10);
        visuals.faint_bg_color = Color32::from_rgb(0x18, 0x1d, 0x24);
        visuals.widgets.noninteractive.bg_fill = visuals.panel_fill;
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(0x1c, 0x20, 0x27);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x22, 0x28, 0x31);
        visuals.widgets.active.bg_fill = Color32::from_rgb(0x27, 0x2e, 0x38);
        visuals.override_text_color = Some(Color32::from_rgb(0xf4, 0xf5, 0xf7));
        visuals.selection.bg_fill = accent;
        visuals.selection.stroke = egui::Stroke::new(1.5, Color32::from_rgb(0x09, 0x9c, 0x86));
    } else {
        visuals.window_fill = Color32::from_rgb(0xf7, 0xf9, 0xfb);
        visuals.panel_fill = Color32::from_rgb(0xf0, 0xf3, 0xf7);
        visuals.extreme_bg_color = Color32::from_rgb(0xeb, 0xee, 0xf2);
        visuals.faint_bg_color = Color32::from_rgb(0xe5, 0xe9, 0xee);
        visuals.widgets.noninteractive.bg_fill = visuals.panel_fill;
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(0xe8, 0xec, 0xf2);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(0xdf, 0xe8, 0xf3);
        visuals.widgets.active.bg_fill = Color32::from_rgb(0xd4, 0xe1, 0xf1);
        visuals.override_text_color = Some(Color32::from_rgb(0x14, 0x1a, 0x22));
        // Use warning accent for hyperlinks in the UI so it matches renderer
        visuals.hyperlink_color = accent_warn;
        visuals.selection.bg_fill = Color32::from_rgb(0x0f, 0xb8, 0x9b);
        visuals.selection.stroke = egui::Stroke::new(1.5, Color32::from_rgb(0x0b, 0x87, 0x72));
    }

    visuals.window_stroke = egui::Stroke::new(2.0, Color32::from_rgba_unmultiplied(0x21, 0x26, 0x30, 190));
    visuals.window_rounding = egui::Rounding::same(16.0);
    visuals.menu_rounding = egui::Rounding::same(12.0);
    visuals.widgets.noninteractive.rounding = egui::Rounding::same(12.0);
    visuals.widgets.inactive.rounding = egui::Rounding::same(12.0);
    visuals.widgets.hovered.rounding = egui::Rounding::same(12.0);
    visuals.widgets.active.rounding = egui::Rounding::same(12.0);
    visuals.widgets.open.rounding = egui::Rounding::same(12.0);

    visuals.window_shadow = egui::Shadow {
        offset: egui::vec2(0.0, 14.0),
        blur: 28.0,
        spread: 0.0,
        color: Color32::from_rgba_unmultiplied(0, 0, 0, 96),
    };

    let rounding = egui::Rounding::same(12.0);
    visuals.widgets.noninteractive.rounding = rounding;
    visuals.widgets.inactive.rounding = rounding;
    visuals.widgets.hovered.rounding = rounding;
    visuals.widgets.active.rounding = rounding;
    visuals.widgets.open.rounding = rounding;

    ctx.set_visuals(visuals);

    // Typographic scale + spacing tweaks
    let mut style = (*ctx.style()).clone();
    style.text_styles.insert(egui::TextStyle::Heading, FontId::proportional(22.0));
    style.text_styles.insert(egui::TextStyle::Body, FontId::proportional(16.0));
    style.text_styles.insert(egui::TextStyle::Monospace, FontId::monospace(15.0));
    style.text_styles.insert(egui::TextStyle::Button, FontId::proportional(15.0));
    style.text_styles.insert(egui::TextStyle::Small, FontId::proportional(13.5));

    style.spacing.item_spacing = egui::vec2(10.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 10.0);
    style.spacing.menu_margin = egui::Margin::symmetric(12.0, 10.0);
    style.spacing.slider_width = 180.0;
    style.spacing.interact_size = egui::vec2(40.0, 28.0);
    style.visuals.popup_shadow = egui::Shadow { offset: egui::vec2(0.0, 10.0), blur: 24.0, spread: 0.0, color: Color32::from_rgba_unmultiplied(0, 0, 0, 90) };

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
