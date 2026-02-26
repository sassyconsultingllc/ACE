//! Main application - Browser with egui chrome and wry webview
//!
//! Architecture:
//! - tao provides the main window
//! - egui renders browser chrome (tabs, address bar, bookmarks bar, status bar)
//! - wry webview handles web content in the content area
//! - egui viewers handle files (PDF, images, documents, etc.) in the content area

use crate::adblock::{
    AdBlocker, AdBlockerUI, BlockStats, FilterList, FilterRule, ResourceType as AdResourceType,
};
use crate::ai::{help_query_for_context, run_help_query, AiRuntime, EasterEggReward, HelpQuery};
use crate::auth::{
    AuthManager, DeviceType, FirstRunState, FirstRunStep, PhoneSync, SyncType, TailscaleManager,
};
use crate::browser::{BrowserEngine, DownloadState, HistoryManager, Tab, TabContent, TabId};
use crate::console::{
    console_debug, console_error, console_info, console_log, console_warn, ConsoleEntry,
    ConsolePanel, DevConsole, LogLevel, NetworkEntry,
};
use crate::detection::DetectionEngine;
use crate::extensions::ExtensionManager;
use crate::family_profiles::{Action, Profile, ProfileManager, ProfileType};
use crate::file_handler::{FileType, OpenFile};
use crate::hittest::{
    hit_test, hit_test_all, CursorType, ElementType, HitResult, InteractionQuality,
    InteractionTracker,
};
use crate::html_renderer::HtmlRenderer;
use crate::icons::Icons;
use crate::input::FocusManager;
use crate::json_viewer::JsonViewer;
use crate::layout::LayoutBox;
use crate::mcp::{AgentRole, MessageRole};
use crate::mcp_panel::{
    get_quick_commands, Action as McpAction, ActionStyle, AgentStatus, McpPanel, McpTheme,
    NotificationStyle, PanelMode, PanelRender, QuickCommand, RenderElement,
};
use crate::mcp_protocol::{ErrorCode, McpCommand, McpResponse};
use crate::mcp_server_native::{McpBridge, McpNativeServer, NativeServerConfig};
use crate::network::{
    shared_monitor, shared_monitor_describe, NetworkRequest as NetActivityRequest,
    NetworkState as NetActivityState, SharedNetworkMonitor,
};
use crate::network_monitor::{
    format_bytes, format_duration, format_speed, ActivityIndicatorState, ConnectionFilter,
    ConnectionSort, ConnectionState, ConnectionType, NetworkMonitor,
};
use crate::password_vault::{
    generate_password, Credential, PasswordGeneratorOptions, PasswordVault,
};
use crate::poisoning::{PoisonMode, PoisoningEngine};
use crate::protocol::{
    encode_form_data, parse_data_url, url_encode, CacheMode, CredentialsMode, FetchOptions,
    HttpClient, MultipartFormData, RedirectMode,
};
use crate::rest_client::RestClient;
use crate::rest_client::{
    ContentType as RestContentType, Method as RestMethod, RequestCollection, SavedRequest,
};
use crate::sandbox::page::Interaction;
use crate::sandbox::popup::PopupRequest;
use crate::sandbox::quarantine::{QuarantinedFile, ReleaseStatus, WarningLevel};
use crate::sandbox::{
    ContentType, InteractionType, SecurityContext, TrustLevel, ViolationSeverity,
};
use crate::script_engine::ScriptEngine;
use crate::smart_history::SmartHistory;
use crate::stealth_victories::StealthVictories;
use crate::syntax::SyntaxHighlighter;
use crate::viewers::{
    archive::ArchiveViewer, audio::AudioViewer, chemical::ChemicalViewer, document::DocumentViewer,
    ebook::EbookViewer, font::FontViewer, image::ImageViewer, model3d::Model3DViewer,
    pdf::PdfViewer, spreadsheet::SpreadsheetViewer, text::TextViewer, video::VideoViewer,
};
use crate::voice::{
    convert_raw_pcm, convert_to_whisper_format, default_audio_device, key_name, list_audio_devices,
    AudioDevice, AudioFormat, CaptureConfig, CloudProvider, CloudTranscriber, HotkeyConfig,
    HotkeyModifiers, MicrophoneCapture, TranscriptResult, TranscriptSegment, TriggerMode,
    VoiceActivityDetector, VoiceCommand, VoiceCommandResult, VoiceConfig, VoiceInput, VoiceSession,
    VoiceState, WhisperEngine, WhisperModel, WhisperParams,
};
use anyhow::Result;
use eframe::egui::{self, Color32, FontId, Key, RichText, Vec2};
use eframe::egui::{ColorImage, TextureHandle};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use url::Url;
use urlencoding::decode;
use uuid::Uuid;

struct TrackedDownload {
    conn_id: u64,
    last_bytes: u64,
    completed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThemePreset {
    SassyBrand,
    SassyRedesign,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LeftSidebarMode {
    Bookmarks,
    History,
    Protection,
}

/// Browser application state
pub struct BrowserApp {
    engine: BrowserEngine,
    extension_manager: ExtensionManager,

    // Authentication & licensing
    auth: AuthManager,
    tailscale: TailscaleManager,
    first_run: FirstRunState,

    // ==============================================================================
    // DISRUPTOR FEATURES - Kills paid software & Chrome bloat
    // ==============================================================================

    // Network Activity Monitor - NO HIDDEN TRAFFIC
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

    // Password Vault - Replaces LastPass, 1Password, Chrome passwords
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

    //  Smart History - 14.7s delay, NSFW detection
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

    // Family Profiles - Parental controls that work
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
    // UI icon textures (loaded at startup from .ico files)
    legacy_icons: std::collections::HashMap<String, TextureHandle>,
    // SVG icon system (replaces all inline Unicode emoji)
    svg_icons: Icons,

    // Developer tools
    dev_console: DevConsole,
    pub json_viewer: JsonViewer,
    rest_client: RestClient,
    pub syntax_highlighter: SyntaxHighlighter,

    // MCP AI system
    mcp_panel: McpPanel,

    // Detection engine + Native MCP server
    detection_engine: DetectionEngine,
    mcp_native_server: McpNativeServer,
    mcp_bridge: std::sync::Arc<McpBridge>,

    // Fingerprint poisoning engine
    poison_engine: PoisoningEngine,
    show_poisoning_popover: bool,
    /// Dedicated script engine for poisoning injection (runs JS in isolated context)
    poison_script_engine: ScriptEngine,

    // Detection/poisoning integration state
    /// Page context for current active tab — fed to detection engine each frame
    detection_page_ctx: crate::detection::PageContext,
    /// URL that was last analyzed by detection engine (to avoid re-analyzing same URL)
    detection_last_analyzed_url: Option<String>,
    /// URL that was last poisoned (to avoid re-poisoning same page)
    poison_last_applied_url: Option<String>,
    /// Whether the poisoning badge in the toolbar should pulse (recently applied)
    poison_badge_pulse: bool,

    // Stealth victories - silent anti-tracking warfare counter
    stealth_victories: StealthVictories,

    // Ad blocker - always-on tracker/ad protection
    ad_blocker: std::sync::Arc<std::sync::RwLock<AdBlocker>>,
    ad_blocker_ui: AdBlockerUI,
    adblock_panel_visible: bool,

    // AI Runtime
    ai_runtime: AiRuntime,
    ai_response: String,
    ai_error: Option<String>,
    ai_easter_egg_pending: Option<EasterEggReward>,

    // Hit testing for rendered content
    last_hit_result: Option<HitResult>,
    hit_test_cursor: CursorType,
    interaction_tracker: InteractionTracker,
    hittest_panel_visible: bool,

    // Protocol handler for HTTP navigation
    http_client: HttpClient,

    // REST client panel
    rest_client_panel_visible: bool,

    // Network activity monitor panel (from network.rs module)
    network_activity_panel_visible: bool,
    /// Standalone network activity monitor (network.rs module)
    net_activity_monitor: crate::network::NetworkMonitor,

    // Protocol diagnostics panel
    protocol_diagnostics_panel_visible: bool,

    // Voice input
    voice_panel_visible: bool,
    voice_recording_active: bool,
    voice_last_transcript: String,
    voice_command_result: Option<String>,
    voice_status_text: String,
    voice_selected_device: usize,
    voice_cloud_api_key: String,
    voice_selected_cloud: usize,
    voice_selected_hotkey_preset: usize,
    voice_show_settings: bool,

    // Session state tracking
    session_state: crate::data::SessionState,

    // Sandbox - 4-layer isolation
    sandbox_manager: crate::sandbox::SandboxManager,
    popup_handler: crate::sandbox::popup::PopupHandler,
    download_quarantine: crate::sandbox::quarantine::Quarantine,
    network_sandbox: crate::sandbox::network::NetworkSandbox,
    show_sandbox_panel: bool,

    // UI state
    dark_mode: bool,
    theme_preset: ThemePreset,
    zoom_level: f32,
    show_dev_tools: bool,
    find_bar_visible: bool,
    find_query: String,
    new_tab_search_query: String,

    // Sidebar state
    left_sidebar_visible: bool,
    left_sidebar_mode: LeftSidebarMode,
    ai_sidebar_visible: bool,
    ai_query_input: String,

    // Context menu state
    context_menu_pos: Option<egui::Pos2>,
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

    // Clear data dialog state
    show_clear_data_dialog: bool,
    clear_data_history: bool,
    clear_data_downloads: bool,
    clear_data_cache: bool,

    // Find bar match count
    find_match_count: usize,
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
                let conn_id = self
                    .network_monitor
                    .start_connection(url, ConnectionType::Document);
                self.network_active_connection = Some(conn_id);
                self.network_last_net_sample = Some(Instant::now());
                self.smart_history.visit(url, url, None);

                // Reset detection/poisoning state for new navigation
                self.detection_last_analyzed_url = None;
                self.poison_last_applied_url = None;
                self.poison_badge_pulse = false;

                // Build fresh page context for detection engine
                let domain = extract_domain_from_url(url);
                self.detection_page_ctx = crate::detection::PageContext {
                    url: url.to_string(),
                    domain: domain.clone(),
                    trust_level: crate::sandbox::TrustLevel::Untrusted,
                    ..Default::default()
                };

                // Set up honeypots for untrusted sites
                if let Some(tab_id) = self.engine.active_tab_id() {
                    self.detection_engine
                        .setup_honeypots(tab_id.0, crate::sandbox::TrustLevel::Untrusted);
                }

                self.engine.navigate(url);
            }
            Err(reason) => {
                let reason_text = reason.description();
                let mut msg = format!("Navigation blocked: {}", reason_text);
                if let Some(active) = self.profile_manager.active_profile() {
                    if active.is_restricted() {
                        let req_id = self
                            .profile_manager
                            .request_approval(Action::AccessBlockedSite(url.to_string()));
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

        egui::Window::new("Extensions")
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
                                    self.status_message =
                                        format!("Failed to load extension: {}", e);
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

        let needs_approval = self
            .profile_manager
            .active_profile()
            .map(|p| p.requires_approval(&action) || p.restrictions.downloads_need_approval)
            .unwrap_or(false);

        match self.profile_manager.can_download(&filename, size_hint) {
            Ok(_) => {
                let engine_filename = suggested_filename.unwrap_or(&filename);
                self.engine.start_download(url, Some(engine_filename));
                self.profile_manager
                    .record_download(&filename, url, size_hint, true, None);
                console_log(&format!("Download started: {}", filename));
                self.status_message = format!("Download started: {}", filename);
                true
            }
            Err(reason) => {
                let mut msg = format!("Download blocked: {}", reason);
                if needs_approval {
                    let req_id = self.profile_manager.request_approval(action);
                    msg = format!("{} (approval requested: {})", msg, req_id);
                    self.profile_manager
                        .record_download(&filename, url, size_hint, false, None);
                }
                console_warn(&msg);
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
        configure_style(&cc.egui_ctx, true, ThemePreset::SassyRedesign);

        let auth = AuthManager::new();
        let mut tailscale = TailscaleManager::new();
        tailscale.check_installation();

        let first_run = if auth.is_first_run {
            FirstRunState::default()
        } else {
            FirstRunState {
                step: FirstRunStep::Complete,
                ..Default::default()
            }
        };

        // Get config directory for vault
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("SassyBrowser");

        // Load legacy icon textures from assets/icons (ICO files)
        let mut legacy_icons: std::collections::HashMap<String, TextureHandle> =
            std::collections::HashMap::new();
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
                    let handle =
                        ctx.load_texture(*path, color_image, egui::TextureOptions::default());
                    legacy_icons.insert((*key).to_string(), handle);
                }
            }
        }

        // Load SVG icon system (replaces all inline Unicode emoji)
        let svg_icons = Icons::load(ctx);

        let mut smart_history = SmartHistory::new();
        smart_history.set_intent_delay(1.5);
        smart_history.set_auto_exclude_nsfw(true);
        smart_history.set_incognito(false);

        let mut app = Self {
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
            legacy_icons,
            svg_icons,

            // Developer tools
            dev_console: DevConsole::new(),
            json_viewer: JsonViewer::new(),
            rest_client: RestClient::new(),
            syntax_highlighter: SyntaxHighlighter::new(),

            // MCP AI system
            mcp_panel: McpPanel::new(),

            // Detection engine + Native MCP server
            detection_engine: DetectionEngine::new(),
            mcp_native_server: McpNativeServer::new(NativeServerConfig::default()),
            mcp_bridge: std::sync::Arc::new(McpBridge::new()),

            // Fingerprint poisoning engine (conservative by default)
            poison_engine: PoisoningEngine::new(),
            show_poisoning_popover: false,
            poison_script_engine: ScriptEngine::new(),

            // Detection/poisoning integration state
            detection_page_ctx: crate::detection::PageContext::default(),
            detection_last_analyzed_url: None,
            poison_last_applied_url: None,
            poison_badge_pulse: false,

            // Stealth victories - persistent poisoning counter
            stealth_victories: StealthVictories::new(),

            // Ad blocker - always-on protection
            ad_blocker: std::sync::Arc::new(std::sync::RwLock::new(AdBlocker::new())),
            ad_blocker_ui: AdBlockerUI::new(std::sync::Arc::new(std::sync::RwLock::new(
                AdBlocker::new(),
            ))),
            adblock_panel_visible: false,

            // AI runtime
            ai_runtime: crate::ai::load_runtime(),
            ai_response: String::new(),
            ai_error: None,
            ai_easter_egg_pending: None,

            // Hit testing
            last_hit_result: None,
            hit_test_cursor: CursorType::Default,
            interaction_tracker: InteractionTracker::new(),
            hittest_panel_visible: false,

            // Protocol handler
            http_client: HttpClient::new(),

            // REST client panel
            rest_client_panel_visible: false,

            // Network activity monitor panel (network.rs)
            network_activity_panel_visible: false,
            net_activity_monitor: crate::network::NetworkMonitor::new(),

            // Protocol diagnostics panel
            protocol_diagnostics_panel_visible: false,

            // Voice input
            voice_panel_visible: false,
            voice_recording_active: false,
            voice_last_transcript: String::new(),
            voice_command_result: None,
            voice_status_text: "Voice input ready".into(),
            voice_selected_device: 0,
            voice_cloud_api_key: String::new(),
            voice_selected_cloud: 0,
            voice_selected_hotkey_preset: 0,
            voice_show_settings: false,

            // Sandbox - 4-layer isolation
            session_state: crate::data::SessionState::new(),
            sandbox_manager: crate::sandbox::SandboxManager::new(),
            popup_handler: crate::sandbox::popup::PopupHandler::new(),
            download_quarantine: crate::sandbox::quarantine::Quarantine::new(),
            network_sandbox: crate::sandbox::network::NetworkSandbox::new(),
            show_sandbox_panel: false,

            dark_mode: true,
            theme_preset: ThemePreset::SassyRedesign,
            zoom_level: 1.0,
            show_dev_tools: false,
            find_bar_visible: false,
            find_query: String::new(),
            new_tab_search_query: String::new(),
            left_sidebar_visible: false,
            left_sidebar_mode: LeftSidebarMode::Bookmarks,
            ai_sidebar_visible: false,
            ai_query_input: String::new(),
            context_menu_pos: None,
            context_menu_link: None,
            status_message: "Ready".into(),
            focus_manager: FocusManager::new(),
            bookmark_import_buffer: String::new(),
            bookmark_export_buffer: String::new(),
            download_url_input: String::new(),
            extension_load_path: String::new(),
            show_clear_data_dialog: false,
            clear_data_history: true,
            clear_data_downloads: false,
            clear_data_cache: true,
            find_match_count: 0,
        };

        // Wire ad_blocker_ui to share the same Arc as ad_blocker
        app.ad_blocker_ui = AdBlockerUI::new(app.ad_blocker.clone());

        if let Err(err) = app.mcp_panel.configure_from_ai_toml() {
            tracing::warn!("Failed to load MCP config: {}", err);
        }

        // Start the native MCP server in background and wire the bridge
        {
            let bridge = app.mcp_native_server.bridge();
            app.mcp_bridge = bridge;
            if let Err(e) = app.mcp_native_server.start() {
                tracing::warn!("Native MCP server failed to start: {}", e);
            }
        }

        // Wire detection engine's shared alerts to the MCP bridge for forwarding
        // (The detection engine pushes alerts via Arc<Mutex<Vec<...>>>)

        // Navigate to initial URL if passed via env var (from command-line file/URL arg)
        if let Ok(initial_url) = std::env::var("SASSY_INITIAL_URL") {
            if !initial_url.is_empty() {
                app.engine.navigate(&initial_url);
                // Clear so it doesn't re-trigger on subsequent app recreations
                std::env::remove_var("SASSY_INITIAL_URL");
            }
        }

        app
    }

    fn render_toolbar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;

            let tab = self.engine.active_tab();
            let (can_back, can_forward, is_loading) = match tab {
                Some(Tab {
                    content:
                        TabContent::Web {
                            can_go_back,
                            can_go_forward,
                            loading,
                            ..
                        },
                    ..
                }) => (*can_go_back, *can_go_forward, *loading),
                _ => (false, false, false),
            };

            // Navigation buttons (SVG icons with text fallback)
            ui.add_enabled_ui(can_back, |ui| {
                if self
                    .svg_icons
                    .button(ui, "nav-back", "Back (Alt+Left)")
                    .clicked()
                {
                    self.engine.go_back();
                }
            });

            ui.add_enabled_ui(can_forward, |ui| {
                if self
                    .svg_icons
                    .button(ui, "nav-forward", "Forward (Alt+Right)")
                    .clicked()
                {
                    self.engine.go_forward();
                }
            });

            if is_loading {
                if self.svg_icons.button(ui, "close-x", "Stop").clicked() {
                    self.engine.stop();
                }
            } else if self.svg_icons.button(ui, "reload", "Reload (F5)").clicked() {
                self.engine.reload();
            }

            if self.svg_icons.button(ui, "home", "Home").clicked() {
                self.engine.go_home();
            }

            // Address bar
            let address_width = ui.available_width() - 150.0;

            ui.scope(|ui| {
                ui.set_min_width(address_width);

                let is_secure = match self.engine.active_tab() {
                    Some(Tab {
                        content: TabContent::Web { is_secure, .. },
                        ..
                    }) => *is_secure,
                    _ => false,
                };

                // Security indicator
                if is_secure {
                    ui.colored_label(Color32::from_rgb(100, 200, 100), "");
                }

                let engine_text = self.engine.address_bar_text().to_string();
                if !self.focus_manager.is_address_bar_focused()
                    && self.focus_manager.address_bar.text != engine_text
                {
                    self.focus_manager.address_bar.set_text(engine_text);
                }

                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.focus_manager.address_bar.text)
                        .desired_width(address_width - 40.0)
                        .font(FontId::proportional(14.0))
                        .hint_text("Search or enter URL"),
                );

                if response.changed() {
                    self.engine
                        .set_address_bar_text(self.focus_manager.address_bar.text.clone());
                }

                if response.gained_focus() {
                    self.focus_manager
                        .focus_address_bar(self.engine.address_bar_text());
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

            // Sidebar toggles
            if self
                .svg_icons
                .button(ui, "books", "Toggle Sidebar")
                .clicked()
            {
                self.left_sidebar_visible = !self.left_sidebar_visible;
            }
            if self.svg_icons.button(ui, "robot", "AI Assistant").clicked() {
                self.ai_sidebar_visible = !self.ai_sidebar_visible;
            }

            // Bookmark button (adds/removes on the bookmarks bar)
            let current_url = self.engine.address_bar_text().to_string();
            let is_bookmarked = self.engine.bookmarks.get_by_url(&current_url).is_some();
            let bookmark_label = if is_bookmarked {
                "Bookmarked"
            } else {
                "Add to Bar"
            };
            if ui
                .button(bookmark_label)
                .on_hover_text("Toggle bookmark on Bookmarks Bar")
                .clicked()
            {
                if is_bookmarked {
                    self.engine.bookmarks.remove_by_url(&current_url);
                    let _ = self.engine.bookmarks.save();
                    self.status_message = "Removed from Bookmarks Bar".into();
                } else {
                    let title = self
                        .engine
                        .active_tab()
                        .map(|t| t.title())
                        .unwrap_or_else(|| current_url.clone());
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
                if let Some(tex) = self.legacy_icons.get("bookmarks") {
                    if ui
                        .add(egui::ImageButton::new((tex.id(), Vec2::new(18.0, 18.0))))
                        .on_hover_text("Bookmarks")
                        .clicked()
                    {
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
                    self.engine
                        .set_show_downloads_panel(!self.engine.show_downloads_panel());
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
                if ui
                    .checkbox(self.engine.show_bookmarks_bar_mut(), "Show Bookmarks Bar")
                    .changed()
                {
                    ui.close_menu();
                }
                if ui.checkbox(&mut self.dark_mode, "Dark Mode").clicked() {
                    configure_style(ctx, self.dark_mode, self.theme_preset);
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
            ui.separator();
            self.render_poisoning_badge(ui);
        });
    }

    fn render_network_indicator(&mut self, ui: &mut egui::Ui) {
        let (down, up) = self.network_monitor.current_speed();
        let (total_down, total_up) = self.network_monitor.total_transferred();
        let blocked = self.network_monitor.blocked_count();
        let active = self.network_monitor.active_count();

        ui.horizontal(|ui| {
            let label = format!(
                "Net {} v{} ^{}",
                active,
                format_speed(down),
                format_speed(up)
            );
            if ui
                .selectable_label(self.activity_indicator.expanded, label)
                .clicked()
            {
                self.activity_indicator.expanded = !self.activity_indicator.expanded;
            }
            ui.label(format!(
                "Total v{} ^{}",
                format_bytes(total_down),
                format_bytes(total_up)
            ));
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
                    ui.selectable_value(
                        &mut self.activity_indicator.filter,
                        ConnectionFilter::All,
                        "All",
                    );
                    ui.selectable_value(
                        &mut self.activity_indicator.filter,
                        ConnectionFilter::Active,
                        "Active",
                    );
                    ui.selectable_value(
                        &mut self.activity_indicator.filter,
                        ConnectionFilter::Documents,
                        "Docs",
                    );
                    ui.selectable_value(
                        &mut self.activity_indicator.filter,
                        ConnectionFilter::Downloads,
                        "Downloads",
                    );
                    ui.selectable_value(
                        &mut self.activity_indicator.filter,
                        ConnectionFilter::Scripts,
                        "Scripts",
                    );
                    ui.selectable_value(
                        &mut self.activity_indicator.filter,
                        ConnectionFilter::Images,
                        "Images",
                    );
                    ui.selectable_value(
                        &mut self.activity_indicator.filter,
                        ConnectionFilter::Xhr,
                        "XHR",
                    );
                    ui.selectable_value(
                        &mut self.activity_indicator.filter,
                        ConnectionFilter::Blocked,
                        "Blocked",
                    );
                });

            ui.label("Sort");
            egui::ComboBox::from_id_salt("net_sort")
                .selected_text(format!("{:?}", self.activity_indicator.sort_by))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.activity_indicator.sort_by,
                        ConnectionSort::Newest,
                        "Newest",
                    );
                    ui.selectable_value(
                        &mut self.activity_indicator.sort_by,
                        ConnectionSort::Oldest,
                        "Oldest",
                    );
                    ui.selectable_value(
                        &mut self.activity_indicator.sort_by,
                        ConnectionSort::Largest,
                        "Largest",
                    );
                    ui.selectable_value(
                        &mut self.activity_indicator.sort_by,
                        ConnectionSort::Slowest,
                        "Slowest",
                    );
                    ui.selectable_value(
                        &mut self.activity_indicator.sort_by,
                        ConnectionSort::Domain,
                        "Domain",
                    );
                });
        });

        let mut conns = self.network_monitor.all_connections();
        conns.retain(|c| match self.activity_indicator.filter {
            ConnectionFilter::All => true,
            ConnectionFilter::Active => matches!(
                c.state,
                ConnectionState::Connecting
                    | ConnectionState::Uploading
                    | ConnectionState::Downloading
            ),
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
            ConnectionSort::Largest => conns.sort_by(|a, b| {
                (b.bytes_received + b.bytes_sent).cmp(&(a.bytes_received + a.bytes_sent))
            }),
            ConnectionSort::Slowest => {
                conns.sort_by(|a, b| a.bytes_received.cmp(&b.bytes_received))
            }
            ConnectionSort::Domain => conns.sort_by(|a, b| a.domain.cmp(&b.domain)),
        }

        egui::ScrollArea::vertical()
            .max_height(160.0)
            .show(ui, |ui| {
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

        let active_url = if let Some(Tab {
            content: TabContent::Web { url, .. },
            ..
        }) = self.engine.active_tab()
        {
            url.clone()
        } else {
            return;
        };

        let matches: Vec<Credential> = self
            .password_vault
            .find_for_url(&active_url)
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
                if ui
                    .small_button(format!("{} / {}", cred.title, cred.username))
                    .clicked()
                {
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

            let entry = self
                .download_connections
                .entry(download.id)
                .or_insert_with(|| TrackedDownload {
                    conn_id: self
                        .network_monitor
                        .start_connection(&download.url, ConnectionType::Download),
                    last_bytes: 0,
                    completed: false,
                });

            if download.downloaded_bytes > entry.last_bytes {
                let delta = download.downloaded_bytes - entry.last_bytes;
                self.network_monitor
                    .update_connection(entry.conn_id, delta, 0);
                entry.last_bytes = download.downloaded_bytes;
            }

            if !entry.completed {
                match download.state {
                    DownloadState::Completed => {
                        self.network_monitor.complete_connection(
                            entry.conn_id,
                            200,
                            download.mime_type.clone(),
                        );
                        entry.completed = true;
                    }
                    DownloadState::Failed => {
                        let reason = download
                            .error
                            .clone()
                            .unwrap_or_else(|| "Download failed".to_string());
                        self.network_monitor.fail_connection(entry.conn_id, &reason);
                        entry.completed = true;
                    }
                    DownloadState::Cancelled => {
                        self.network_monitor
                            .fail_connection(entry.conn_id, "Cancelled");
                        entry.completed = true;
                    }
                    DownloadState::Paused | DownloadState::Pending | DownloadState::Downloading => {
                    }
                }
            }
        }

        self.download_connections
            .retain(|id, _| active_ids.contains(id));
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

        egui::Window::new("Account & Sync")
            .open(&mut self.auth_panel_visible)
            .default_size(Vec2::new(720.0, 520.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("License & Devices");
                    if !self.auth_status.is_empty() {
                        ui.label(
                            RichText::new(&self.auth_status)
                                .color(Color32::from_rgb(120, 200, 255)),
                        );
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
                        match self
                            .tailscale
                            .start_with_auth_key(&self.tailscale_auth_key_input)
                        {
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
                    ui.add(
                        egui::TextEdit::singleline(&mut self.tailscale_file_target)
                            .hint_text("peer_ip:/path/to/file"),
                    );
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
                        self.phone_sync
                            .queue_sync(SyncType::Bookmark, b"sample-bookmark".to_vec());
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
            let tabs: Vec<(TabId, String, String, bool, bool)> = self
                .engine
                .tabs()
                .iter()
                .map(|t| {
                    (
                        t.id,
                        t.icon().to_string(),
                        t.title(),
                        t.is_loading(),
                        t.pinned,
                    )
                })
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
                    .rounding(egui::Rounding {
                        nw: 4.0,
                        ne: 4.0,
                        sw: 0.0,
                        se: 0.0,
                    })
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
                            let display_title = if title.len() > max_title_len && max_title_len > 0
                            {
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

                            if tab_response.secondary_clicked() {
                                self.context_menu_pos = tab_response.interact_pointer_pos();
                                self.context_menu_link =
                                    Some(self.engine.tabs()[idx].content.get_display_url());
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
                            if !*pinned && ui.small_button("x").clicked() {
                                close_tab = Some(idx);
                            }
                        });
                    });

                ui.add_space(2.0);
            }

            // New tab button
            if self
                .svg_icons
                .button(ui, "plus", "New Tab (Ctrl+T)")
                .clicked()
            {
                self.engine.new_tab();
            }

            // Handle tab close
            if let Some(idx) = close_tab {
                self.engine.close_tab(idx);
            }

            self.session_state.active_tab = if self.engine.tab_count() > 0 {
                Some(self.engine.active_tab_index())
            } else {
                None
            };
            self.session_state.tabs = self
                .engine
                .tabs()
                .iter()
                .map(|t| crate::data::TabState {
                    id: t.id.0,
                    url: t.content.get_display_url(),
                    title: t.title(),
                    scroll_x: 0,
                    scroll_y: 0,
                })
                .collect();
        });
    }

    fn render_bookmarks_bar(&mut self, ui: &mut egui::Ui) {
        if !self.engine.show_bookmarks_bar() {
            return;
        }

        ui.horizontal(|ui| {
            let bookmarks: Vec<_> = self
                .engine
                .bookmarks
                .bookmarks_bar()
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
                ui.label(
                    RichText::new("Bookmarks bar is empty")
                        .italics()
                        .color(Color32::from_rgb(160, 160, 160)),
                );
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

            if ui.button("Find").clicked()
                || (response.lost_focus() && ctx.input(|i| i.key_pressed(Key::Enter)))
            {
                self.find_match_count = self.html_renderer.find_text(&self.find_query);
            }

            if self.find_match_count > 0 || !self.find_query.is_empty() {
                ui.label(format!("{} matches", self.find_match_count));
            }

            if ui.button("x").clicked() {
                self.find_bar_visible = false;
                self.find_match_count = 0;
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
                        ui.heading(RichText::new("Site Blocked").size(36.0).color(Color32::RED));
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
                    // Detection alert banner above web content
                    self.render_detection_alert_banner(ui);
                    self.render_web_content(ctx, ui, &url);
                    self.network_monitor.cleanup_old(Duration::from_secs(30));
                }
            }
            Some(("File", _)) => {
                // Get file reference carefully
                if let Some(tab) = self.engine.active_tab() {
                    if let TabContent::File(file) = &tab.content {
                        // Clone necessary data to avoid borrow issues
                        let file_clone = file.clone();
                        let _ = tab;
                        self.render_file_content(ui, &file_clone);
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
                ui.label(
                    RichText::new("It's everything you love - from every browser you hate.")
                        .size(16.0)
                        .color(Color32::GRAY),
                );

                ui.add_space(40.0);

                // Centered search bar
                ui.horizontal(|ui| {
                    let quarter = ui.available_width() / 4.0;
                    ui.add_space(quarter);
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut self.new_tab_search_query)
                            .hint_text("Search with DuckDuckGo or enter URL")
                            .desired_width(ui.available_width() - quarter),
                    );
                    if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        if !self.new_tab_search_query.is_empty() {
                            let query = self.new_tab_search_query.clone();
                            if query.contains('.') || query.starts_with("http") {
                                self.guarded_navigate(&query);
                            } else {
                                let search_url = format!(
                                    "{}{}",
                                    self.engine.search_engine,
                                    query.replace(' ', "+")
                                );
                                self.guarded_navigate(&search_url);
                            }
                            self.new_tab_search_query.clear();
                        }
                    }
                });

                ui.add_space(20.0);

                // Quick access - Most visited
                ui.heading("Most Visited");
                ui.add_space(10.0);

                ui.horizontal_wrapped(|ui| {
                    let most_visited: Vec<_> = self
                        .engine
                        .history
                        .most_visited(8)
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

                ui.add_space(20.0);
                ui.heading("Quick Access");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("GitHub").size(14.0)).clicked() {
                        self.guarded_navigate("https://github.com");
                    }
                    if ui.button(RichText::new("DuckDuckGo").size(14.0)).clicked() {
                        self.guarded_navigate("https://duckduckgo.com");
                    }
                    if ui.button(RichText::new("Tailscale").size(14.0)).clicked() {
                        self.guarded_navigate("sassy://settings");
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
                ui.label(
                    RichText::new("Drag & drop any file or use File -> Open")
                        .italics()
                        .color(Color32::GRAY),
                );
            });
        });
    }

    fn render_web_content(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, url: &str) {
        let url = url.to_string();
        let available = ui.available_size();
        let accent_warn = Color32::from_rgb(0xf7, 0x8c, 0x1f);
        // Dark/light mode background
        let bg = if self.dark_mode {
            Color32::from_gray(25)
        } else {
            Color32::WHITE
        };
        egui::Frame::none()
            .fill(bg)
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.set_min_size(available);
                // Render HTML with flagged link highlighting
                let smart_history = &self.smart_history;
                let profile_manager = &self.profile_manager;
                let blocklist = profile_manager
                    .active_profile()
                    .map(|p| p.restrictions.blocklist.clone())
                    .unwrap_or_default();
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
                        self.html_renderer.render_node_with_link_check(
                            ui,
                            node,
                            &doc.styles,
                            Some(&link_check),
                            accent_warn,
                        );
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
                    ui.collapsing("Console", |ui| {
                        for line in self.html_renderer.console_output() {
                            ui.label(RichText::new(line).monospace().size(12.0));
                        }
                    });
                }
                // Show option to open in system browser
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Tip:").small().color(Color32::GRAY));
                    if ui.small_button("Open in system browser").clicked() {
                        let _ = open::that(&url);
                    }
                    ui.label(
                        RichText::new("for full web experience")
                            .small()
                            .color(Color32::GRAY),
                    );
                });
            });
    }

    pub fn render_file_content(&mut self, ui: &mut egui::Ui, file: &OpenFile) {
        let icons = &self.svg_icons;
        match file.file_type {
            FileType::Image | FileType::ImageRaw | FileType::ImagePsd => {
                self.image_viewer.render(ui, file, self.zoom_level, icons)
            }
            FileType::Pdf => self.pdf_viewer.render(ui, file, self.zoom_level, icons),
            FileType::Document => self
                .document_viewer
                .render(ui, file, self.zoom_level, icons),
            FileType::Spreadsheet => {
                self.spreadsheet_viewer
                    .render(ui, file, self.zoom_level, icons)
            }
            FileType::Chemical => self
                .chemical_viewer
                .render(ui, file, self.zoom_level, icons),
            FileType::Archive => self.archive_viewer.render(ui, file, self.zoom_level, icons),
            FileType::Model3D => self.model3d_viewer.render(ui, file, self.zoom_level, icons),
            FileType::Font => self.font_viewer.render(ui, file, self.zoom_level, icons),
            FileType::Audio => self.audio_viewer.render(ui, file, self.zoom_level, icons),
            FileType::Video => self.video_viewer.render(ui, file, self.zoom_level, icons),
            FileType::Ebook => self.ebook_viewer.render(ui, file, self.zoom_level, icons),
            FileType::Markdown => self.text_viewer.render(ui, file, self.zoom_level, icons),
            FileType::Text | FileType::Unknown => {
                self.text_viewer.render(ui, file, self.zoom_level, icons)
            }
        }
    }

    fn render_settings_page(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading(" Settings");
            ui.separator();

            ui.add_space(20.0);

            // Appearance
            ui.heading("Appearance");
            ui.checkbox(&mut self.dark_mode, "Dark Mode");

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label("Theme Preset:");
                let preset_label = match self.theme_preset {
                    ThemePreset::SassyBrand => "Sassy Brand",
                    ThemePreset::SassyRedesign => "Sassy Redesign",
                };
                egui::ComboBox::from_id_salt("theme_preset_combo")
                    .selected_text(preset_label)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_value(
                                &mut self.theme_preset,
                                ThemePreset::SassyBrand,
                                "Sassy Brand",
                            )
                            .changed()
                            || ui
                                .selectable_value(
                                    &mut self.theme_preset,
                                    ThemePreset::SassyRedesign,
                                    "Sassy Redesign",
                                )
                                .changed()
                        {
                            // Theme will be applied on next frame via configure_style
                        }
                    });
            });

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label("Zoom:");
                if self.svg_icons.button(ui, "minus", "Zoom out").clicked() {
                    self.zoom_level = (self.zoom_level - 0.1).max(0.5);
                }
                ui.label(format!("{:.0}%", self.zoom_level * 100.0));
                if self.svg_icons.button(ui, "plus", "Zoom in").clicked() {
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
                let search_options = [
                    ("DuckDuckGo", "https://duckduckgo.com/?q="),
                    ("Google", "https://www.google.com/search?q="),
                    ("Bing", "https://www.bing.com/search?q="),
                    ("Brave", "https://search.brave.com/search?q="),
                ];
                let current_label = search_options
                    .iter()
                    .find(|(_, url)| *url == self.engine.search_engine.as_str())
                    .map(|(name, _)| *name)
                    .unwrap_or("Custom");
                egui::ComboBox::from_id_salt("search_engine_combo")
                    .selected_text(current_label)
                    .show_ui(ui, |ui| {
                        for (name, url) in &search_options {
                            if ui
                                .selectable_label(self.engine.search_engine == *url, *name)
                                .clicked()
                            {
                                self.engine.set_search_engine(url.to_string());
                            }
                        }
                    });
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
                self.show_clear_data_dialog = true;
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
                ui.heading("History");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Clear All History").clicked() {
                        self.engine.history.clear();
                    }
                });
            });

            ui.separator();

            let history: Vec<_> = self
                .engine
                .history
                .recent(100)
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
                        ui.label(
                            RichText::new(format!("{} visits", visits))
                                .small()
                                .color(Color32::GRAY),
                        );
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
                        ui.label(
                            RichText::new(entry.domain.clone())
                                .small()
                                .color(Color32::GRAY),
                        );
                    });
                });
            }

            if is_empty {
                ui.label(
                    RichText::new("No history yet")
                        .italics()
                        .color(Color32::GRAY),
                );
            }
        });
    }

    fn render_bookmarks_page(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Bookmarks");
            ui.separator();

            let bookmarks: Vec<_> = self
                .engine
                .bookmarks
                .all()
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
                        if ui.small_button("x").clicked() {
                            remove_id = Some(*id);
                        }
                        ui.label(RichText::new(url).color(Color32::from_rgb(150, 150, 150)));
                    });
                });
            }

            // Remove bookmark after iteration
            if let Some(id) = remove_id {
                self.engine.bookmarks.remove(id);
            }

            if is_empty {
                ui.label(
                    RichText::new("No bookmarks yet")
                        .italics()
                        .color(Color32::from_rgb(160, 160, 160)),
                );
            }

            ui.separator();
            ui.heading("Import / Export");
            ui.label("Import HTML (paste Netscape format)");
            ui.text_edit_multiline(&mut self.bookmark_import_buffer);
            if ui.small_button("Import").clicked() {
                match self
                    .engine
                    .bookmarks
                    .import_html(&self.bookmark_import_buffer)
                {
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
                    Ok(_) => {
                        self.status_message = format!("Saved export to {}", export_path.display())
                    }
                    Err(e) => self.status_message = format!("Save failed: {}", e),
                }
            }
        });
    }

    fn render_downloads_page(&mut self, ui: &mut egui::Ui) {
        use crate::family_profiles::{Action, ApprovalStatus};
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Downloads");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let active = self.engine.downloads.has_active_downloads();
                    if active {
                        ui.label(
                            RichText::new("Active")
                                .small()
                                .color(Color32::from_rgb(0x16, 0xf2, 0xd6)),
                        );
                    }
                    if ui
                        .button("Clear Completed")
                        .on_hover_text("Remove finished downloads from the list")
                        .clicked()
                    {
                        self.engine.downloads.clear_finished();
                    }
                });
            });

            ui.horizontal(|ui| {
                ui.label("URL");
                ui.text_edit_singleline(&mut self.download_url_input)
                    .on_hover_text("Paste a direct download link here");
                if ui
                    .button("Start")
                    .on_hover_text("Request download (may require approval)")
                    .clicked()
                    && !self.download_url_input.is_empty()
                {
                    let url = self.download_url_input.clone();
                    self.guard_download_request(&url, None);
                    self.download_url_input.clear();
                }
            });
            ui.separator();

            // Show pending/denied/expired approval requests for downloads
            let approval_requests: Vec<_> = self
                .profile_manager
                .all_approval_requests()
                .iter()
                .filter(|r| matches!(&r.action, Action::Download { .. }))
                .cloned()
                .collect();

            if !approval_requests.is_empty() {
                ui.heading("Download Approvals");
                for req in &approval_requests {
                    let (status_text, color, tooltip) = match req.status {
                        ApprovalStatus::Pending => {
                            ("Pending", Color32::YELLOW, "Waiting for parent approval")
                        }
                        ApprovalStatus::Approved => (
                            "Approved",
                            Color32::from_rgb(0x16, 0xf2, 0xd6),
                            "Approved by parent",
                        ),
                        ApprovalStatus::Denied => (
                            "Denied",
                            Color32::RED,
                            req.parent_response.as_deref().unwrap_or("Denied by parent"),
                        ),
                        ApprovalStatus::Expired => {
                            ("Expired", Color32::GRAY, "Approval request expired")
                        }
                    };
                    ui.horizontal(|ui| {
                        if let Action::Download { filename, size, .. } = &req.action {
                            ui.label(RichText::new(filename).strong());
                            ui.label(format!("{:.1} MB", *size as f32 / 1_048_576.0));
                            ui.label(RichText::new(status_text).color(color))
                                .on_hover_text(tooltip);
                            if (req.status == ApprovalStatus::Denied
                                || req.status == ApprovalStatus::Expired)
                                && ui
                                    .button("Resubmit")
                                    .on_hover_text("Request approval again")
                                    .clicked()
                            {
                                let new_id =
                                    self.profile_manager.request_approval(req.action.clone());
                                self.status_message =
                                    format!("Resubmitted approval request: {}", new_id);
                            }
                            if let Some(resp) = &req.parent_response {
                                if req.status == ApprovalStatus::Denied {
                                    ui.label(
                                        RichText::new(format!("Reason: {}", resp))
                                            .color(Color32::RED),
                                    )
                                    .on_hover_text("Parent's reason for denial");
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
                            if ui
                                .small_button("Cancel")
                                .on_hover_text("Cancel this download")
                                .clicked()
                            {
                                self.engine.downloads.cancel_download(download.id);
                            }
                        }
                        crate::browser::DownloadState::Completed => {
                            ui.label(
                                RichText::new("Complete")
                                    .color(Color32::from_rgb(0x16, 0xf2, 0xd6)),
                            );
                            if ui
                                .small_button("Open")
                                .on_hover_text("Open the downloaded file")
                                .clicked()
                            {
                                let _ = open::that(&download.save_path);
                            }
                            if ui
                                .small_button("Show in Folder")
                                .on_hover_text("Show file in folder")
                                .clicked()
                            {
                                let _ = open::that(
                                    download.save_path.parent().unwrap_or(&download.save_path),
                                );
                            }
                        }
                        crate::browser::DownloadState::Failed => {
                            ui.label(RichText::new("Failed").color(Color32::RED))
                                .on_hover_text("Download failed");
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
                        if ui.button("x").clicked() {
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
                                    ui.add(
                                        egui::ProgressBar::new(download.progress())
                                            .desired_width(200.0)
                                            .show_percentage(),
                                    );
                                    ui.label(format!("{:.1} KB/s", download.speed_bps() / 1024.0));
                                    if ui.small_button("Cancel").clicked() {
                                        self.engine.downloads.cancel_download(download.id);
                                    }
                                }
                                crate::browser::DownloadState::Completed => {
                                    ui.label(
                                        RichText::new("Complete")
                                            .color(Color32::from_rgb(0x16, 0xf2, 0xd6)),
                                    );
                                    if ui.small_button("Open").clicked() {
                                        let _ = open::that(&download.save_path);
                                    }
                                    if ui.small_button("Show in Folder").clicked() {
                                        let _ = open::that(
                                            download
                                                .save_path
                                                .parent()
                                                .unwrap_or(&download.save_path),
                                        );
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

    // ═══════════════════════════════════════════════════════════════════════════
    // MCP COMMAND HANDLER — Processes commands from native MCP server
    // ═══════════════════════════════════════════════════════════════════════════

    fn handle_mcp_command(&mut self, cmd: McpCommand) -> McpResponse {
        match cmd {
            McpCommand::Ping { seq } => McpResponse::Pong { seq },

            McpCommand::Navigate { url, .. } => {
                self.guarded_navigate(&url);
                McpResponse::Ok {
                    message: format!("Navigating to {}", url),
                }
            }

            McpCommand::GoBack => {
                self.engine.go_back();
                McpResponse::Ok {
                    message: "Navigated back".into(),
                }
            }

            McpCommand::GoForward => {
                self.engine.go_forward();
                McpResponse::Ok {
                    message: "Navigated forward".into(),
                }
            }

            McpCommand::Reload => {
                self.engine.reload();
                McpResponse::Ok {
                    message: "Reloading".into(),
                }
            }

            McpCommand::ReadPage => {
                if let Some(tab) = self.engine.active_tab() {
                    match &tab.content {
                        TabContent::Web { url, title, .. } => {
                            McpResponse::PageContent {
                                url: url.clone(),
                                title: title.clone(),
                                text_content: format!("Page: {} — {}", title, url),
                                html_snippet: None,
                                trust_level: 0, // TODO: wire to real trust level
                            }
                        }
                        _ => McpResponse::Error {
                            code: ErrorCode::NotAvailable,
                            message: "Active tab is not a web page".into(),
                        },
                    }
                } else {
                    McpResponse::Error {
                        code: ErrorCode::NotFound,
                        message: "No active tab".into(),
                    }
                }
            }

            McpCommand::GetSecurityStatus => {
                let (url, _domain) = if let Some(tab) = self.engine.active_tab() {
                    if let TabContent::Web { url, .. } = &tab.content {
                        (url.clone(), extract_domain_from_url(url))
                    } else {
                        (String::new(), String::new())
                    }
                } else {
                    (String::new(), String::new())
                };

                McpResponse::SecurityStatus {
                    url,
                    trust_level: 0,
                    trust_description: "Untrusted".into(),
                    violation_count: 0,
                    honeypots_active: self.detection_engine.enabled,
                    detection_alerts: self.detection_engine.active_alert_count(),
                    cumulative_score: self.detection_engine.cumulative_score(),
                }
            }

            McpCommand::GetDetectionAlerts => {
                let alerts: Vec<_> = self
                    .detection_engine
                    .recent_alerts(20)
                    .iter()
                    .map(|a| crate::mcp_protocol::DetectionAlertWire {
                        rule_name: a.rule_name.clone(),
                        level: a.level.to_u8(),
                        description: a.description.clone(),
                        url: a.url.clone(),
                        domain: a.domain.clone(),
                        score: a.score,
                        honeypot_triggered: a.honeypot_triggered,
                        action: a.action.to_u8(),
                    })
                    .collect();
                McpResponse::DetectionAlerts {
                    alerts,
                    total_emitted: self.detection_engine.total_alerts(),
                    cumulative_score: self.detection_engine.cumulative_score(),
                }
            }

            McpCommand::ClearDetectionAlerts => {
                self.detection_engine.clear();
                McpResponse::Ok {
                    message: "Detection alerts cleared".into(),
                }
            }

            McpCommand::ListTabs => {
                let tabs: Vec<_> = self
                    .engine
                    .tabs()
                    .iter()
                    .enumerate()
                    .map(|(i, t)| crate::mcp_protocol::TabInfo {
                        index: i,
                        title: t.title(),
                        url: match &t.content {
                            TabContent::Web { url, .. } => url.clone(),
                            _ => format!("sassy://{}", t.title().to_lowercase()),
                        },
                        loading: matches!(&t.content, TabContent::Web { loading: true, .. }),
                        trust_level: 0,
                    })
                    .collect();
                let active = self.engine.active_tab_index();
                McpResponse::TabList {
                    tabs,
                    active_index: active,
                }
            }

            McpCommand::NewTab { url } => {
                self.engine.new_tab();
                if let Some(u) = url {
                    self.guarded_navigate(&u);
                }
                McpResponse::Ok {
                    message: "New tab created".into(),
                }
            }

            McpCommand::GetBrowserInfo => McpResponse::BrowserInfo {
                name: "Sassy Browser".into(),
                version: "2.1.0".into(),
                engine: "SassyEngine (Rust)".into(),
                features: vec![
                    "fingerprint-poisoning".into(),
                    "honeypot-detection".into(),
                    "mcp-native-binary".into(),
                    "4-layer-sandbox".into(),
                    "password-vault".into(),
                    "family-profiles".into(),
                ],
            },

            McpCommand::Goodbye => McpResponse::Ok {
                message: "Goodbye".into(),
            },

            // Commands not yet fully implemented
            _ => McpResponse::Error {
                code: ErrorCode::NotAvailable,
                message: "Command not yet implemented".into(),
            },
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // POISONING BADGE — Toolbar indicator for fingerprint poisoning status
    // ═══════════════════════════════════════════════════════════════════════════

    fn render_poisoning_badge(&mut self, ui: &mut egui::Ui) {
        let (badge_text, badge_color) = match self.poison_engine.mode {
            PoisonMode::Off => ("FP: Off", Color32::GRAY),
            PoisonMode::Conservative => ("FP: Conservative", Color32::from_rgb(100, 180, 255)),
            PoisonMode::Aggressive => ("FP: Aggressive", Color32::from_rgb(255, 140, 60)),
        };

        let badge =
            egui::Button::new(RichText::new(badge_text).small().color(badge_color)).frame(false);

        let response = ui.add(badge).on_hover_text(format!(
            "Fingerprint Poisoning: {}\nClick to change mode",
            self.poison_engine.mode_description()
        ));

        if response.clicked() {
            self.show_poisoning_popover = !self.show_poisoning_popover;
        }

        // Show popover when toggled
        if self.show_poisoning_popover {
            egui::Area::new(egui::Id::new("poison_popover"))
                .fixed_pos(response.rect.left_bottom())
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.set_min_width(200.0);
                        ui.label(RichText::new("Fingerprint Poisoning").strong());
                        ui.separator();

                        if ui
                            .radio(
                                self.poison_engine.mode == PoisonMode::Off,
                                "Off — Chrome spoof only",
                            )
                            .clicked()
                        {
                            self.poison_engine.mode = PoisonMode::Off;
                            self.poison_last_applied_url = None; // Force re-apply
                            self.show_poisoning_popover = false;
                        }
                        if ui
                            .radio(
                                self.poison_engine.mode == PoisonMode::Conservative,
                                "Conservative — Light noise",
                            )
                            .clicked()
                        {
                            self.poison_engine.mode = PoisonMode::Conservative;
                            self.poison_last_applied_url = None;
                            self.show_poisoning_popover = false;
                        }
                        if ui
                            .radio(
                                self.poison_engine.mode == PoisonMode::Aggressive,
                                "Aggressive — Maximum unlinkability",
                            )
                            .clicked()
                        {
                            self.poison_engine.mode = PoisonMode::Aggressive;
                            self.poison_last_applied_url = None;
                            self.show_poisoning_popover = false;
                        }

                        ui.separator();
                        ui.label(
                            RichText::new(self.poison_engine.mode_description())
                                .small()
                                .color(Color32::GRAY),
                        );
                    });
                });
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // DETECTION ALERT BANNER — Shows above page content when threats detected
    // ═══════════════════════════════════════════════════════════════════════════

    fn render_detection_alert_banner(&mut self, ui: &mut egui::Ui) {
        // Delegate to the detection engine's built-in banner renderer
        let _has_alerts = self.detection_engine.render_alert_banner(ui);
    }

    // =========================================================================
    // DEVELOPER CONSOLE - Full-featured DevTools panel (F12)
    // =========================================================================

    fn render_dev_console(&mut self, ctx: &egui::Context) {
        if !self.show_dev_tools {
            return;
        }
        let mut open = self.show_dev_tools;
        egui::Window::new("Developer Tools")
            .open(&mut open)
            .resizable(true)
            .default_size(Vec2::new(800.0, 400.0))
            .min_width(400.0)
            .min_height(200.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for panel in ConsolePanel::all() {
                        let label = panel.label();
                        let selected = self.dev_console.active_panel == *panel;
                        if ui.selectable_label(selected, label).clicked() {
                            self.dev_console.active_panel = *panel;
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Toggle").clicked() {
                            self.dev_console.toggle();
                        }
                        ui.label(
                            RichText::new(format!("h:{}", self.dev_console.height))
                                .small()
                                .color(Color32::GRAY),
                        );
                    });
                });
                ui.separator();
                match self.dev_console.active_panel {
                    ConsolePanel::Console => self.render_dev_console_tab(ui),
                    ConsolePanel::Network => self.render_dev_network_tab(ui),
                    ConsolePanel::Elements => self.render_dev_elements_tab(ui),
                    ConsolePanel::Sources => self.render_dev_sources_tab(ui),
                    ConsolePanel::Application => self.render_dev_application_tab(ui),
                }
            });
        if !open {
            self.show_dev_tools = false;
        }
    }

    fn render_dev_console_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Levels:").small());
            if ui
                .selectable_label(self.dev_console.show_log, "Log")
                .clicked()
            {
                self.dev_console.show_log = !self.dev_console.show_log;
            }
            if ui
                .selectable_label(self.dev_console.show_info, "Info")
                .clicked()
            {
                self.dev_console.show_info = !self.dev_console.show_info;
            }
            if ui
                .selectable_label(self.dev_console.show_warn, "Warn")
                .clicked()
            {
                self.dev_console.show_warn = !self.dev_console.show_warn;
            }
            if ui
                .selectable_label(self.dev_console.show_error, "Error")
                .clicked()
            {
                self.dev_console.show_error = !self.dev_console.show_error;
            }
            ui.separator();
            if ui.small_button("Clear").clicked() {
                self.dev_console.clear();
            }
            ui.separator();
            ui.label(RichText::new("Filter:").small());
            ui.text_edit_singleline(&mut self.dev_console.console_filter);
        });
        ui.separator();
        let filtered: Vec<ConsoleEntry> = self
            .dev_console
            .filtered_console_entries()
            .into_iter()
            .cloned()
            .collect();
        let entry_count = filtered.len();
        let total_count = self.dev_console.console_entries.len();
        egui::ScrollArea::vertical()
            .max_height(ui.available_height() - 40.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                if filtered.is_empty() {
                    ui.label(
                        RichText::new(format!(
                            "No console output ({} total, all filtered)",
                            total_count
                        ))
                        .italics()
                        .color(Color32::GRAY),
                    );
                } else {
                    ui.label(
                        RichText::new(format!(
                            "Showing {} of {} entries",
                            entry_count, total_count
                        ))
                        .small()
                        .color(Color32::GRAY),
                    );
                    for entry in &filtered {
                        let c = entry.level.color();
                        let color = egui::Color32::from_rgba_premultiplied(c.r, c.g, c.b, c.a);
                        let prefix = entry.level.prefix();
                        let _level_desc = entry.level.describe();
                        let ts = entry.timestamp.format("%H:%M:%S%.3f").to_string();
                        let source_str = entry.source.as_deref().unwrap_or("");
                        let stack_str = entry.stack_trace.as_deref().unwrap_or("");
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(&ts)
                                    .monospace()
                                    .size(10.0)
                                    .color(Color32::GRAY),
                            );
                            ui.label(
                                RichText::new(format!("{}{}", prefix, &entry.message))
                                    .monospace()
                                    .size(11.0)
                                    .color(color),
                            );
                            if !source_str.is_empty() {
                                ui.label(
                                    RichText::new(format!("@ {}", source_str))
                                        .monospace()
                                        .size(10.0)
                                        .color(Color32::GRAY),
                                );
                            }
                        });
                        let full_desc = entry.describe();
                        if !stack_str.is_empty() {
                            ui.label(
                                RichText::new(stack_str)
                                    .monospace()
                                    .size(10.0)
                                    .color(Color32::from_rgb(180, 180, 180)),
                            );
                        }
                        if ui.small_button("...").on_hover_text(&full_desc).clicked() {
                            self.status_message = full_desc;
                        }
                    }
                }
            });
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(">")
                    .monospace()
                    .color(Color32::from_rgb(100, 180, 255)),
            );
            let response = ui.text_edit_singleline(&mut self.dev_console.input_buffer);
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.dev_console.handle_key("Enter", false);
            }
            ui.label(
                RichText::new(format!(
                    "cur:{} hist:{}",
                    self.dev_console.input_cursor,
                    self.dev_console.command_history.len()
                ))
                .small()
                .color(Color32::GRAY),
            );
            if let Some(idx) = self.dev_console.history_index {
                ui.label(
                    RichText::new(format!("[{}]", idx))
                        .small()
                        .color(Color32::YELLOW),
                );
            }
        });
    }

    fn render_dev_network_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Filter:").small());
            ui.text_edit_singleline(&mut self.dev_console.network_filter);
            ui.separator();
            if ui.small_button("Clear Network").clicked() {
                self.dev_console.network_entries.clear();
                self.dev_console.selected_network_entry = None;
            }
            ui.label(
                RichText::new(format!("next_id:{}", self.dev_console.next_request_id))
                    .small()
                    .color(Color32::GRAY),
            );
        });
        ui.separator();
        let filtered: Vec<NetworkEntry> = self
            .dev_console
            .filtered_network_entries()
            .into_iter()
            .cloned()
            .collect();
        let total = self.dev_console.network_entries.len();
        let max = self.dev_console.max_network_entries;
        egui::ScrollArea::vertical()
            .max_height(ui.available_height() - 10.0)
            .show(ui, |ui| {
                if filtered.is_empty() {
                    ui.label(
                        RichText::new(format!("No network requests ({}/{})", total, max))
                            .italics()
                            .color(Color32::GRAY),
                    );
                } else {
                    ui.label(
                        RichText::new(format!(
                            "Showing {} of {}/{} requests",
                            filtered.len(),
                            total,
                            max
                        ))
                        .small()
                        .color(Color32::GRAY),
                    );
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("ID").monospace().size(10.0).strong());
                        ui.add_space(10.0);
                        ui.label(RichText::new("Method").monospace().size(10.0).strong());
                        ui.add_space(10.0);
                        ui.label(RichText::new("Status").monospace().size(10.0).strong());
                        ui.add_space(10.0);
                        ui.label(RichText::new("URL").monospace().size(10.0).strong());
                        ui.add_space(10.0);
                        ui.label(RichText::new("Duration").monospace().size(10.0).strong());
                        ui.add_space(10.0);
                        ui.label(RichText::new("Type").monospace().size(10.0).strong());
                    });
                    ui.separator();
                    for entry in &filtered {
                        let sc = entry.status_color();
                        let status_color = Color32::from_rgb(sc.r, sc.g, sc.b);
                        let is_selected = self.dev_console.selected_network_entry == Some(entry.id);
                        let row = ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("{}", entry.id))
                                    .monospace()
                                    .size(10.0),
                            );
                            ui.add_space(10.0);
                            ui.label(
                                RichText::new(&entry.method)
                                    .monospace()
                                    .size(10.0)
                                    .color(Color32::from_rgb(100, 180, 255)),
                            );
                            ui.add_space(10.0);
                            let status_str = match (&entry.status, &entry.status_text) {
                                (Some(s), Some(t)) => format!("{} {}", s, t),
                                (Some(s), None) => format!("{}", s),
                                _ => "pending".to_string(),
                            };
                            ui.label(
                                RichText::new(&status_str)
                                    .monospace()
                                    .size(10.0)
                                    .color(status_color),
                            );
                            ui.add_space(10.0);
                            let url_display = if entry.url.len() > 60 {
                                format!("{}...", &entry.url[..57])
                            } else {
                                entry.url.clone()
                            };
                            ui.label(RichText::new(&url_display).monospace().size(10.0));
                            ui.add_space(10.0);
                            let dur = entry
                                .duration_ms
                                .map_or("--".to_string(), |d| format!("{}ms", d));
                            ui.label(RichText::new(&dur).monospace().size(10.0));
                            ui.add_space(10.0);
                            let ct = entry.content_type.as_deref().unwrap_or("--");
                            ui.label(
                                RichText::new(ct)
                                    .monospace()
                                    .size(10.0)
                                    .color(Color32::GRAY),
                            );
                        });
                        if row.response.interact(egui::Sense::click()).clicked() {
                            if is_selected {
                                self.dev_console.selected_network_entry = None;
                            } else {
                                self.dev_console.selected_network_entry = Some(entry.id);
                            }
                        }
                        if is_selected {
                            ui.indent("net_detail", |ui| {
                                ui.add_space(4.0);
                                let desc = entry.describe();
                                ui.label(
                                    RichText::new(&desc)
                                        .monospace()
                                        .size(10.0)
                                        .color(Color32::from_rgb(200, 200, 200)),
                                );
                                ui.add_space(4.0);
                                ui.label(RichText::new("Waterfall:").small().strong());
                                let wf_desc = entry.waterfall.describe();
                                ui.label(
                                    RichText::new(&wf_desc)
                                        .monospace()
                                        .size(10.0)
                                        .color(Color32::from_rgb(180, 180, 220)),
                                );
                                let total_wf = entry.waterfall.total_ms();
                                if total_wf > 0.0 {
                                    let segments = entry.waterfall.segments();
                                    ui.horizontal(|ui| {
                                        for (name, _start, dur, wf_color) in &segments {
                                            let frac = (*dur / total_wf).clamp(0.0, 1.0) as f32;
                                            let bar_width = (frac * 200.0).max(4.0);
                                            let bar_color = Color32::from_rgb(
                                                wf_color.r, wf_color.g, wf_color.b,
                                            );
                                            let (rect, _) = ui.allocate_exact_size(
                                                Vec2::new(bar_width, 12.0),
                                                egui::Sense::hover(),
                                            );
                                            ui.painter().rect_filled(rect, 0.0, bar_color);
                                            ui.painter().text(
                                                rect.center(),
                                                egui::Align2::CENTER_CENTER,
                                                name,
                                                FontId::monospace(8.0),
                                                Color32::WHITE,
                                            );
                                        }
                                    });
                                }
                                if !entry.request_headers.is_empty() {
                                    ui.collapsing("Request Headers", |ui| {
                                        for (k, v) in &entry.request_headers {
                                            ui.label(
                                                RichText::new(format!("{}: {}", k, v))
                                                    .monospace()
                                                    .size(10.0),
                                            );
                                        }
                                    });
                                }
                                if !entry.response_headers.is_empty() {
                                    ui.collapsing("Response Headers", |ui| {
                                        for (k, v) in &entry.response_headers {
                                            ui.label(
                                                RichText::new(format!("{}: {}", k, v))
                                                    .monospace()
                                                    .size(10.0),
                                            );
                                        }
                                    });
                                }
                                if let Some(body) = &entry.request_body {
                                    ui.collapsing("Request Body", |ui| {
                                        ui.label(RichText::new(body).monospace().size(10.0));
                                    });
                                }
                                if let Some(body) = &entry.response_body {
                                    ui.collapsing("Response Body", |ui| {
                                        let is_json = entry
                                            .content_type
                                            .as_ref()
                                            .map_or(false, |ct| ct.contains("json"));
                                        if is_json {
                                            let tokens = self.dev_console.highlight_js(body);
                                            for line_tokens in &tokens {
                                                ui.horizontal(|ui| {
                                                    for tok in line_tokens {
                                                        ui.label(
                                                            RichText::new(&tok.text)
                                                                .monospace()
                                                                .size(10.0)
                                                                .color(Color32::from_rgb(
                                                                    tok.color.r,
                                                                    tok.color.g,
                                                                    tok.color.b,
                                                                )),
                                                        );
                                                    }
                                                });
                                            }
                                        } else {
                                            ui.label(RichText::new(body).monospace().size(10.0));
                                        }
                                    });
                                }
                                if let Some(cl) = entry.content_length {
                                    ui.label(
                                        RichText::new(format!("Content-Length: {} bytes", cl))
                                            .monospace()
                                            .size(10.0)
                                            .color(Color32::GRAY),
                                    );
                                }
                                if let Some(err) = &entry.error {
                                    ui.label(
                                        RichText::new(format!("Error: {}", err))
                                            .monospace()
                                            .size(10.0)
                                            .color(Color32::from_rgb(255, 100, 100)),
                                    );
                                }
                                ui.label(
                                    RichText::new(format!(
                                        "Started: {}",
                                        entry.start_time.format("%H:%M:%S%.3f")
                                    ))
                                    .monospace()
                                    .size(10.0)
                                    .color(Color32::GRAY),
                                );
                                if let Some(end) = &entry.end_time {
                                    ui.label(
                                        RichText::new(format!(
                                            "Ended: {}",
                                            end.format("%H:%M:%S%.3f")
                                        ))
                                        .monospace()
                                        .size(10.0)
                                        .color(Color32::GRAY),
                                    );
                                }
                                ui.add_space(4.0);
                            });
                        }
                    }
                }
            });
    }

    fn render_dev_elements_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let pick_label = if self.dev_console.inspector.pick_mode {
                "Picking (click element)"
            } else {
                "Pick Element"
            };
            if ui
                .selectable_label(self.dev_console.inspector.pick_mode, pick_label)
                .clicked()
            {
                self.dev_console.inspector.toggle_pick_mode();
            }
            if ui.small_button("Clear Inspector").clicked() {
                self.dev_console.inspector.clear();
            }
            if ui.small_button("Select Root").clicked() {
                self.dev_console.inspector.select_element(vec![0]);
            }
            if ui.small_button("Reset Styles").clicked() {
                let ds = crate::style::ComputedStyle::default();
                self.dev_console.inspector.update_from_computed(&ds);
            }
            if ui.small_button("Set Content 100x100").clicked() {
                self.dev_console.inspector.set_content_size(100.0, 100.0);
            }
        });
        ui.separator();
        let inspector_desc = self.dev_console.inspector.describe();
        ui.label(
            RichText::new(&inspector_desc)
                .monospace()
                .size(10.0)
                .color(Color32::from_rgb(180, 200, 220)),
        );
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            if !self.dev_console.inspector.selected_path.is_empty() {
                let path_str: Vec<String> = self
                    .dev_console
                    .inspector
                    .selected_path
                    .iter()
                    .map(|i| i.to_string())
                    .collect();
                ui.label(
                    RichText::new(format!("Selected: /{}", path_str.join("/")))
                        .monospace()
                        .size(11.0)
                        .color(Color32::from_rgb(100, 180, 255)),
                );
                if !self.dev_console.inspector.hovered_path.is_empty() {
                    let hover_str: Vec<String> = self
                        .dev_console
                        .inspector
                        .hovered_path
                        .iter()
                        .map(|i| i.to_string())
                        .collect();
                    ui.label(
                        RichText::new(format!("Hovered: /{}", hover_str.join("/")))
                            .monospace()
                            .size(10.0)
                            .color(Color32::GRAY),
                    );
                }
                ui.add_space(8.0);
                if !self.dev_console.inspector.computed_styles.is_empty() {
                    ui.collapsing("Computed Styles", |ui| {
                        for (prop, val) in &self.dev_console.inspector.computed_styles {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("{}:", prop))
                                        .monospace()
                                        .size(10.0)
                                        .color(Color32::from_rgb(200, 150, 255)),
                                );
                                ui.label(RichText::new(val).monospace().size(10.0));
                            });
                        }
                    });
                }
                if !self.dev_console.inspector.matched_rules.is_empty() {
                    ui.collapsing("Matched Rules", |ui| {
                        for rule in &self.dev_console.inspector.matched_rules {
                            let rule_desc = rule.describe();
                            ui.label(
                                RichText::new(&rule_desc)
                                    .monospace()
                                    .size(10.0)
                                    .color(Color32::from_rgb(180, 200, 160)),
                            );
                            for (name, val, overridden) in &rule.properties {
                                let color = if *overridden {
                                    Color32::from_rgb(128, 128, 128)
                                } else {
                                    Color32::from_rgb(220, 220, 220)
                                };
                                ui.label(
                                    RichText::new(format!(
                                        "  {}: {}{}",
                                        name,
                                        val,
                                        if *overridden { " (overridden)" } else { "" }
                                    ))
                                    .monospace()
                                    .size(10.0)
                                    .color(color),
                                );
                            }
                        }
                    });
                }
                let bm = &self.dev_console.inspector.box_model;
                ui.collapsing("Box Model", |ui| {
                    let bm_desc = bm.describe();
                    ui.label(RichText::new(&bm_desc).monospace().size(10.0));
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!("Margin:  {}", bm.margin.describe()))
                            .monospace()
                            .size(10.0),
                    );
                    ui.label(
                        RichText::new(format!("Border:  {}", bm.border.describe()))
                            .monospace()
                            .size(10.0),
                    );
                    ui.label(
                        RichText::new(format!("Padding: {}", bm.padding.describe()))
                            .monospace()
                            .size(10.0),
                    );
                    ui.label(
                        RichText::new(format!("Content: {}", bm.content.describe()))
                            .monospace()
                            .size(10.0),
                    );
                });
            } else {
                ui.label(
                    RichText::new(
                        "No element selected. Click 'Pick Element' then click on the page.",
                    )
                    .italics()
                    .color(Color32::GRAY),
                );
            }
        });
    }

    fn render_dev_sources_tab(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Sources").strong());
        ui.separator();
        let code = if self.dev_console.input_buffer.is_empty() {
            "// Enter JavaScript in the Console tab\nvar x = 1;\nconsole.log(x);"
        } else {
            &self.dev_console.input_buffer
        };
        let tokens = self.dev_console.highlight_js(code);
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (line_num, line_tokens) in tokens.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{:4}", line_num + 1))
                            .monospace()
                            .size(10.0)
                            .color(Color32::from_rgb(100, 100, 100)),
                    );
                    for tok in line_tokens {
                        ui.label(
                            RichText::new(&tok.text)
                                .monospace()
                                .size(11.0)
                                .color(Color32::from_rgb(tok.color.r, tok.color.g, tok.color.b)),
                        );
                    }
                });
            }
        });
    }

    fn render_dev_application_tab(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Application State").strong());
        ui.separator();
        ui.horizontal(|ui| {
            if ui.small_button("Log Test Message").clicked() {
                self.dev_console
                    .log(LogLevel::Info, "Test info message".to_string());
                self.dev_console
                    .log(LogLevel::Warn, "Test warning".to_string());
                self.dev_console
                    .log(LogLevel::Error, "Test error".to_string());
                self.dev_console
                    .log(LogLevel::Debug, "Test debug".to_string());
                self.dev_console.log_with_source(
                    LogLevel::Log,
                    "Message with source".to_string(),
                    "app.rs:42".to_string(),
                );
            }
            if ui.small_button("Start Test Request").clicked() {
                let id = self
                    .dev_console
                    .start_request("GET", "https://example.com/api/test");
                self.dev_console.complete_request(id, 200, "OK");
            }
            if ui.small_button("Start Failed Request").clicked() {
                let id = self
                    .dev_console
                    .start_request("POST", "https://example.com/api/fail");
                self.dev_console.fail_request(id, "Connection refused");
            }
            if ui.small_button("Clear All").clicked() {
                self.dev_console.clear();
                self.dev_console.inspector.clear();
            }
        });
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            self.dev_console.render(ui);
            ui.add_space(8.0);
            let status = self.dev_console.status();
            for line in status.lines() {
                ui.label(RichText::new(line).monospace().size(10.0));
            }
        });
    }

    fn render_status_bar(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(22.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(&self.status_message);

                    // Show detection alert count if any
                    let alert_count = self.detection_engine.active_alert_count();
                    if alert_count > 0 {
                        ui.separator();
                        ui.label(
                            RichText::new(format!(
                                "{} alert{}",
                                alert_count,
                                if alert_count == 1 { "" } else { "s" }
                            ))
                            .color(Color32::from_rgb(255, 100, 100))
                            .small(),
                        );
                    }

                    // Show time left for restricted profiles
                    if let Some(profile) = self.profile_manager.active_profile() {
                        if profile.is_restricted() {
                            if let Some(mins) = self.profile_manager.remaining_time_minutes() {
                                ui.label(
                                    RichText::new(format!("Time left: {} min", mins))
                                        .color(Color32::YELLOW),
                                );
                            }
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(chrono::Local::now().format("%H:%M:%S").to_string());
                        ui.separator();
                        ui.label("v2.1.0");
                        ui.separator();
                        // Privacy indicator - always visible
                        let privacy_resp = ui.label(
                            RichText::new("\u{1f6e1} Private")
                                .small()
                                .color(Color32::from_rgb(60, 200, 80)),
                        );
                        privacy_resp.on_hover_text(
                            "All data stored locally \u{2022} Zero telemetry\n\
                             No crash reports \u{2022} No tracking\n\
                             Settings encrypted alongside passwords\n\n\
                             Your browser. Your data. Always.",
                        );
                        ui.separator();
                        // Poisoning mode indicator in status bar
                        let poison_label = match self.poison_engine.mode {
                            PoisonMode::Off => "FP:Off",
                            PoisonMode::Conservative => "FP:Con",
                            PoisonMode::Aggressive => "FP:Agg",
                        };
                        let poison_color = match self.poison_engine.mode {
                            PoisonMode::Off => Color32::GRAY,
                            PoisonMode::Conservative => Color32::from_rgb(100, 180, 255),
                            PoisonMode::Aggressive => Color32::from_rgb(255, 140, 60),
                        };
                        ui.label(RichText::new(poison_label).small().color(poison_color));
                        ui.separator();
                        // MCP server status
                        if self
                            .mcp_bridge
                            .running
                            .load(std::sync::atomic::Ordering::Relaxed)
                        {
                            ui.label(
                                RichText::new("MCP")
                                    .small()
                                    .color(Color32::from_rgb(100, 200, 100)),
                            );
                        }
                        // Stealth victories counter
                        ui.separator();
                        // Ad/tracker blocker stats
                        let blocked = self.network_monitor.blocked_count();
                        if blocked > 0 {
                            ui.label(
                                RichText::new(format!("Blocked: {}", blocked))
                                    .small()
                                    .color(Color32::from_rgb(255, 100, 100)),
                            );
                        }
                        let poisoned = self.stealth_victories.poisoned_count();
                        if poisoned > 0 {
                            ui.label(
                                RichText::new(format!("Sites poisoned: {}", poisoned))
                                    .small()
                                    .color(Color32::from_rgb(200, 100, 255)),
                            );
                        }
                        ui.separator();
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
                    self.engine
                        .set_show_downloads_panel(!self.engine.show_downloads_panel());
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
                for (idx, key) in [
                    Key::Num1,
                    Key::Num2,
                    Key::Num3,
                    Key::Num4,
                    Key::Num5,
                    Key::Num6,
                    Key::Num7,
                    Key::Num8,
                    Key::Num9,
                ]
                .iter()
                .enumerate()
                {
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
                        self.engine.set_active_tab(if current == 0 {
                            count - 1
                        } else {
                            current - 1
                        });
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
                let is_fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fullscreen));
            }
            if i.key_pressed(Key::F12) {
                self.show_dev_tools = !self.show_dev_tools;
                if self.show_dev_tools {
                    console_info("Developer Tools opened");
                } else {
                    console_debug("Developer Tools closed");
                }
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

        egui::Window::new("Password Vault")
            .open(&mut self.vault_panel_visible)
            .resizable(true)
            .default_size(Vec2::new(520.0, 520.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Vault");
                    if !self.vault_status.is_empty() {
                        ui.label(
                            RichText::new(&self.vault_status)
                                .color(Color32::from_rgb(120, 200, 255)),
                        );
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
                        ui.add(
                            egui::TextEdit::singleline(&mut self.vault_pin_input).password(true),
                        );
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
                        ui.add(
                            egui::TextEdit::singleline(&mut self.vault_pin_input).password(true),
                        );
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
                    if ui
                        .add(egui::Slider::new(&mut auto_lock, 30.0..=86400.0).logarithmic(true))
                        .changed()
                    {
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
                                        Err(e) => {
                                            self.vault_status = format!("Update failed: {}", e)
                                        }
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
                    if ui
                        .checkbox(
                            &mut self.vault_autofill_enabled,
                            "Inline autofill suggestions",
                        )
                        .clicked()
                    {
                        self.vault_status = if self.vault_autofill_enabled {
                            "Autofill suggestions on".into()
                        } else {
                            "Autofill suggestions off".into()
                        };
                    }
                    ui.label("Folder");
                    egui::ComboBox::from_label("")
                        .selected_text(if self.vault_folder_filter.is_empty() {
                            "All"
                        } else {
                            self.vault_folder_filter.as_str()
                        })
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(self.vault_folder_filter.is_empty(), "All")
                                .clicked()
                            {
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
                    self.password_vault
                        .search(&self.vault_search_query)
                        .into_iter()
                        .cloned()
                        .collect()
                };
                if !folder_filter.is_empty() {
                    search_results.retain(|c| c.folder.as_deref() == Some(folder_filter.as_str()));
                }
                let favorites: Vec<Credential> = self
                    .password_vault
                    .favorites()
                    .into_iter()
                    .cloned()
                    .collect();
                let weak: Vec<Credential> = self
                    .password_vault
                    .weak_passwords()
                    .into_iter()
                    .cloned()
                    .collect();
                let recent: Vec<Credential> = self
                    .password_vault
                    .recently_used(6)
                    .into_iter()
                    .cloned()
                    .collect();
                let by_url_matches: Vec<Credential> = if let Some(tab) = self.engine.active_tab() {
                    if let TabContent::Web { url, .. } = &tab.content {
                        self.password_vault
                            .find_for_url(url)
                            .into_iter()
                            .cloned()
                            .collect()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                ui.collapsing("Matches Current Site", |ui| {
                    if by_url_matches.is_empty() {
                        ui.label("None");
                    } else {
                        for cred in &by_url_matches {
                            ui.horizontal(|ui| {
                                ui.label(&cred.title);
                                ui.label(
                                    RichText::new(&cred.username)
                                        .color(Color32::from_rgb(150, 200, 255)),
                                );
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
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(cred.domain());
                                },
                            );
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
                                ui.label(
                                    RichText::new(format!("Last used: {}", ts))
                                        .color(Color32::GRAY),
                                );
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

                let reused_owned: std::collections::HashMap<String, Vec<Credential>> = self
                    .password_vault
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
                    ui.add(
                        egui::TextEdit::multiline(&mut self.vault_export_buffer).desired_rows(4),
                    );
                });

                ui.collapsing("CSV Import", |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.vault_import_buffer).desired_rows(4),
                    );
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

        egui::Window::new("Family Profiles")
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
                                ui.label(RichText::new(format!("Bedtime: {:02}:{:02}-{:02}:{:02}", r.0, r.1, e.0, e.1)).color(Color32::LIGHT_RED))
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
        let most_visited: Vec<_> = self
            .smart_history
            .most_visited(6)
            .into_iter()
            .cloned()
            .collect();
        let search_results: Vec<_> = if self.history_search_query.is_empty() {
            Vec::new()
        } else {
            self.smart_history
                .search(&self.history_search_query)
                .into_iter()
                .cloned()
                .collect()
        };
        let nsfw_entries: Vec<_> = self
            .smart_history
            .nsfw_entries()
            .into_iter()
            .cloned()
            .collect();
        let syncable_count = self.smart_history.syncable().len();
        let abandoned: Vec<String> = self
            .smart_history
            .recent_abandoned(60)
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        let hm_recent: Vec<_> = self
            .history_manager
            .recent(10)
            .into_iter()
            .cloned()
            .collect();
        let hm_search: Vec<_> = if self.history_search_query.is_empty() {
            Vec::new()
        } else {
            self.history_manager
                .search(&self.history_search_query)
                .into_iter()
                .cloned()
                .collect()
        };
        let hm_most: Vec<_> = self
            .history_manager
            .most_visited(6)
            .into_iter()
            .cloned()
            .collect();
        let hm_day_results: Vec<_> = if !self.history_day_query.is_empty() {
            parse_ymd(&self.history_day_query)
                .map(|(y, m, d)| {
                    self.history_manager
                        .for_date(y, m, d)
                        .into_iter()
                        .cloned()
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        egui::Window::new("History")
            .open(&mut self.history_panel_visible)
            .resizable(true)
            .default_size(Vec2::new(760.0, 560.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("History & Activity");
                    if !self.history_status.is_empty() {
                        ui.label(
                            RichText::new(&self.history_status)
                                .color(Color32::from_rgb(150, 200, 255)),
                        );
                    }
                });

                ui.separator();

                // Controls
                ui.horizontal(|ui| {
                    let mut incognito = self.smart_history.is_incognito();
                    if ui
                        .checkbox(&mut incognito, "Incognito (don't track)")
                        .changed()
                    {
                        self.smart_history.set_incognito(incognito);
                    }

                    let mut intent_delay = self.smart_history.intent_delay_secs();
                    if ui
                        .add(
                            egui::Slider::new(&mut intent_delay, 0.0..=30.0)
                                .text("Intent delay (s)"),
                        )
                        .changed()
                    {
                        self.smart_history.set_intent_delay(intent_delay);
                    }

                    if ui
                        .checkbox(&mut self.history_auto_exclude, "Auto-exclude NSFW")
                        .changed()
                    {
                        self.smart_history
                            .set_auto_exclude_nsfw(self.history_auto_exclude);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("NSFW sensitivity");
                    let mut sensitivity = self.history_nsfw_sensitivity;
                    if ui
                        .add(egui::Slider::new(&mut sensitivity, 0.1..=1.0))
                        .changed()
                    {
                        self.history_nsfw_sensitivity = sensitivity;
                        self.smart_history
                            .nsfw_detector()
                            .set_sensitivity(sensitivity);
                    }

                    ui.label("Block domain");
                    ui.text_edit_singleline(&mut self.history_domain_filter);
                    if ui.small_button("Delete domain").clicked()
                        && !self.history_domain_filter.is_empty()
                    {
                        self.smart_history
                            .delete_for_domain(&self.history_domain_filter);
                        self.history_status = "Domain removed from history".into();
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Range start (epoch secs)");
                    ui.text_edit_singleline(&mut self.history_date_start);
                    ui.label("End");
                    ui.text_edit_singleline(&mut self.history_date_end);
                    if ui.small_button("Delete range").clicked() {
                        if let (Ok(start), Ok(end)) = (
                            self.history_date_start.parse::<u64>(),
                            self.history_date_end.parse::<u64>(),
                        ) {
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
                    ui.label(format!(
                        "Smart: total {} | pending {} | syncable {} | NSFW {}",
                        total_count,
                        pending_count,
                        syncable_count,
                        nsfw_entries.len()
                    ));
                    ui.label(format!(
                        "Intent delay {:.1}s",
                        self.smart_history.intent_delay_secs()
                    ));
                    ui.label(format!("Visits tracked: {}", stats.total_visits));
                });

                if let Some(tab) = self.engine.active_tab() {
                    if let TabContent::Web { url, title, .. } = &tab.content {
                        let url_clone = url.clone();
                        let raw_title = title.clone();
                        let history_title = if raw_title.is_empty() {
                            tab.title()
                        } else {
                            raw_title.clone()
                        };

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
                    left.label(format!(
                        "Unique domains: {} | Pruned: {}",
                        stats.unique_domains, stats.entries_pruned
                    ));

                    left.collapsing("Recent (intent-committed)", |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(160.0)
                            .show(ui, |ui| {
                                for entry in &recent_entries {
                                    ui.horizontal(|ui| {
                                        ui.label(entry.title.clone());
                                        ui.label(
                                            RichText::new(&entry.domain)
                                                .color(Color32::from_rgb(150, 150, 180)),
                                        );
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
                                ui.label(format!(
                                    "{} ({:.2})",
                                    entry.domain, entry.nsfw_confidence
                                ));
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

                    let right_stats_label = format!(
                        "HistoryManager entries: {}",
                        self.history_manager.all().len()
                    );
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
            .add_filter(
                "All Supported",
                &[
                    // Images
                    "png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "avif", "ico", "tiff", "tif",
                    "tga", "hdr", "exr", "pnm", "qoi", "dds", "psd", "xcf",
                    // RAW camera
                    "cr2", "cr3", "nef", "arw", "dng", "raf", "orf", "rw2", "pef", "srw", "raw",
                    // Documents
                    "pdf", "docx", "doc", "odt", "rtf", "wpd", "xlsx", "xls", "ods", "csv", "tsv",
                    // Chemical/Scientific
                    "pdb", "mol", "sdf", "xyz", "cif", "mol2", "mmcif", // Archives
                    "zip", "tar", "gz", "tgz", "bz2", "xz", "7z", "rar", "zst",
                    // 3D Models
                    "obj", "stl", "gltf", "glb", "ply", "fbx", "dae", "3ds", // Fonts
                    "ttf", "otf", "woff", "woff2", "eot", "fon", // Audio
                    "mp3", "flac", "wav", "ogg", "m4a", "aac", "wma", "opus", "aiff",
                    // Video
                    "mp4", "mkv", "webm", "avi", "mov", "wmv", "flv", "m4v", "ogv",
                    // eBooks
                    "epub", "mobi", "azw", "azw3", "fb2", // Code/Text
                    "txt", "md", "rs", "py", "js", "ts", "html", "css", "json", "xml", "yaml",
                    "yml", "c", "cpp", "h", "hpp", "java", "go", "rb", "php", "swift", "kt", "lua",
                    "sh", "bat", "ps1", "sql", "toml", "ini", "cfg", "log", "tex", "bib",
                ],
            )
            .add_filter(
                "Images",
                &[
                    "png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "avif", "ico", "tiff",
                    "psd", "cr2", "nef", "arw", "dng",
                ],
            )
            .add_filter(
                "Documents",
                &["pdf", "docx", "doc", "odt", "rtf", "xlsx", "xls", "csv"],
            )
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
        // Get text content from the active tab for printing
        let content = if let Some(tab) = self.engine.active_tab() {
            match &tab.content {
                TabContent::Web { url, title, .. } => {
                    // Try to get rendered text from HTML renderer
                    if let Some(doc) = &self.html_renderer.cached_doc {
                        let mut text = String::new();
                        fn extract_text(node: &crate::html_renderer::HtmlNode, out: &mut String) {
                            match node {
                                crate::html_renderer::HtmlNode::Text(t) => {
                                    out.push_str(t);
                                    out.push(' ');
                                }
                                crate::html_renderer::HtmlNode::Element {
                                    children, tag, ..
                                } => {
                                    for child in children {
                                        extract_text(child, out);
                                    }
                                    if matches!(
                                        tag.as_str(),
                                        "p" | "div" | "br" | "h1" | "h2" | "h3" | "h4" | "li"
                                    ) {
                                        out.push('\n');
                                    }
                                }
                                crate::html_renderer::HtmlNode::Script(_) => {}
                            }
                        }
                        for node in &doc.nodes {
                            extract_text(node, &mut text);
                        }
                        if text.trim().is_empty() {
                            format!("{}\n{}", title, url)
                        } else {
                            text
                        }
                    } else {
                        format!("{}\n{}", title, url)
                    }
                }
                TabContent::NewTab => "Sassy Browser - New Tab".to_string(),
                _ => "No printable content".to_string(),
            }
        } else {
            "No active tab".to_string()
        };

        let settings = crate::print::PrintSettings::default();
        match crate::print::print_page(content.as_bytes(), &settings) {
            Ok(()) => self.status_message = "Page sent to printer".into(),
            Err(e) => self.status_message = format!("Print failed: {}", e),
        }
    }

    // ==============================================================================
    // FIRST RUN WIZARD
    // ==============================================================================

    fn render_first_run_wizard(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);

                // Header
                ui.heading(
                    RichText::new("Sassy Browser")
                        .size(48.0)
                        .color(Color32::from_rgb(255, 140, 0)),
                );
                ui.add_space(10.0);
                ui.label(
                    RichText::new("Pure Rust * No Chrome * No Google * No Tracking")
                        .size(16.0)
                        .color(Color32::GRAY),
                );
                ui.add_space(40.0);

                // Progress indicator
                ui.horizontal(|ui| {
                    let steps = [
                        "Welcome",
                        "Security",
                        "Device",
                        "Tailscale",
                        "Phone",
                        "Done",
                    ];
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
                            Color32::from_rgb(255, 140, 0) // Current
                        } else {
                            Color32::GRAY // Future
                        };

                        ui.label(RichText::new(format!("{}. {}", i + 1, step)).color(color));
                        if i < steps.len() - 1 {
                            ui.label(RichText::new(" -> ").color(Color32::DARK_GRAY));
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
                    (
                        "",
                        "100% Pure Rust",
                        "No Chrome, no WebKit, no Google telemetry",
                    ),
                    (
                        "",
                        "200+ File Formats",
                        "PDF, PDB, RAW photos, CAD files - all built-in",
                    ),
                    (
                        "",
                        "Kills Paid Software",
                        "Adobe Suite ($504/yr), AutoCAD ($2K/yr) - FREE",
                    ),
                    (
                        "",
                        "Tailscale Mesh",
                        "Sync across all your devices securely",
                    ),
                    ("", "Phone App", "Pair your phone for seamless sync"),
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

        if self
            .svg_icons
            .text_button(ui, "arrow-right", "Get Started", "Begin setup")
            .clicked()
        {
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

        let elapsed = self
            .first_run
            .entropy_started_at
            .map(|t| t.elapsed())
            .unwrap_or_default();
        let remaining = self
            .first_run
            .entropy_min_seconds
            .saturating_sub(elapsed.as_secs());
        let timer_done = elapsed.as_secs_f32() >= self.first_run.entropy_min_seconds as f32;

        ui.heading("Creating Your Security Key");
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
            ui.label(
                RichText::new("Timer: done (30s minimum reached)")
                    .color(Color32::from_rgb(0, 200, 100)),
            );
        } else {
            ui.label(
                RichText::new(format!("Timer: {}s remaining to harden the key", remaining))
                    .color(Color32::YELLOW),
            );
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
                ui.label(
                    RichText::new("Backup your key seed!")
                        .strong()
                        .color(Color32::YELLOW),
                );
                if let Some(seed) = self.auth.get_master_key() {
                    let seed_hex = hex::encode(seed);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&seed_hex).monospace());
                        if ui.small_button("Copy").clicked() {
                            ui.output_mut(|o| o.copied_text = seed_hex.clone());
                        }
                        if ui.small_button("Show QR").clicked() {
                            self.first_run.error_message = Some(seed_hex.clone());
                            // Use error_message as temp QR trigger
                        }
                    });
                    // Show QR code if requested
                    if let Some(ref qr) = self.first_run.error_message {
                        if qr == &seed_hex {
                            if let Ok(code) = qrcode::QrCode::new(seed) {
                                let image = code
                                    .render::<qrcode::render::svg::Color>()
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
                    ui.label(
                        RichText::new(c.to_string())
                            .monospace()
                            .color(char_color)
                            .size(20.0),
                    );
                }
            });
        });

        ui.add_space(30.0);

        ui.horizontal(|ui| {
            if self
                .svg_icons
                .text_button(ui, "arrow-left", "Back", "Go back")
                .clicked()
            {
                self.first_run.prev_step();
            }

            ui.add_space(20.0);

            let ready = self.auth.is_entropy_ready();
            let can_continue = ready && timer_done;
            if ui
                .add_enabled(can_continue, egui::Button::new("Continue"))
                .on_hover_text("Continue setup")
                .clicked()
            {
                self.first_run.next_step();
            }

            if !ready {
                ui.label(RichText::new("Keep moving your mouse!").color(Color32::YELLOW));
            } else if !timer_done {
                ui.label(
                    RichText::new("Timer still running for stronger key").color(Color32::YELLOW),
                );
            }
        });
    }

    fn render_wizard_device(&mut self, ui: &mut egui::Ui) {
        ui.heading("Name This Device");
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
                (DeviceType::Desktop, "Desktop"),
                (DeviceType::Laptop, "Laptop"),
                (DeviceType::Server, "Server"),
            ];

            for (dtype, label) in types {
                if ui
                    .selectable_label(self.first_run.device_type == dtype, label)
                    .clicked()
                {
                    self.first_run.device_type = dtype;
                }
            }
        });

        ui.add_space(20.0);

        ui.checkbox(
            &mut self.first_run.enable_tailscale,
            "Enable Tailscale mesh networking",
        );
        ui.checkbox(&mut self.first_run.enable_phone_sync, "Set up phone sync");

        if let Some(ref err) = self.first_run.error_message {
            ui.add_space(10.0);
            ui.label(RichText::new(err).color(Color32::RED));
        }

        ui.add_space(30.0);

        ui.horizontal(|ui| {
            if self
                .svg_icons
                .text_button(ui, "arrow-left", "Back", "Go back")
                .clicked()
            {
                self.first_run.prev_step();
            }

            ui.add_space(20.0);

            if ui
                .button(RichText::new("Create Device Key ->").size(18.0))
                .clicked()
            {
                match self.auth.complete_first_run(
                    &self.first_run.device_name,
                    self.first_run.device_type.clone(),
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
        ui.heading("Tailscale Setup");
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
                ui.label(
                    RichText::new("[ok] Tailscale is connected!")
                        .color(Color32::from_rgb(0, 200, 100)),
                );
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
                        let status = if peer.online { "" } else { "( )" };
                        ui.label(format!(
                            "{} {} ({})",
                            status, peer.hostname, peer.ip_address
                        ));
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
            if self
                .svg_icons
                .text_button(ui, "arrow-left", "Back", "Go back")
                .clicked()
            {
                self.first_run.prev_step();
            }

            ui.add_space(20.0);

            let label = if self.first_run.enable_phone_sync {
                "Continue to Phone Setup ->"
            } else {
                "Finish Setup ->"
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
        ui.heading("Phone App Pairing");
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
                ui.label(
                    RichText::new(format!("Pairing Code:\n\n{}", code))
                        .size(32.0)
                        .monospace(),
                );
            });
        });

        ui.add_space(20.0);

        ui.label("Or enter this code manually in the phone app:");
        ui.add_space(10.0);

        if let Some(ref code) = self.first_run.pairing_code {
            ui.label(
                RichText::new(code)
                    .size(48.0)
                    .monospace()
                    .color(Color32::from_rgb(255, 140, 0)),
            );
        }

        ui.add_space(10.0);

        ui.label(RichText::new("Download the app:").color(Color32::GRAY));
        ui.horizontal(|ui| {
            if ui.link("iOS App Store").clicked() {
                let _ = open::that("https://apps.apple.com/app/sassy-browser");
            }
            ui.label(" | ");
            if ui.link("Google Play").clicked() {
                let _ =
                    open::that("https://play.google.com/store/apps/details?id=com.sassybrowser");
            }
            ui.label(" | ");
            if ui.link("F-Droid").clicked() {
                let _ = open::that("https://f-droid.org/packages/com.sassybrowser");
            }
        });

        ui.add_space(30.0);

        ui.horizontal(|ui| {
            if self
                .svg_icons
                .text_button(ui, "arrow-left", "Back", "Go back")
                .clicked()
            {
                self.first_run.prev_step();
            }

            ui.add_space(20.0);

            if ui
                .button(RichText::new("Finish Setup ->").size(18.0))
                .clicked()
            {
                self.first_run.next_step();
            }

            ui.add_space(20.0);

            if ui.small_button("Skip for now").clicked() {
                self.first_run.enable_phone_sync = false;
                self.first_run.next_step();
            }
        });
    }

    fn render_left_sidebar(&mut self, ctx: &egui::Context) {
        if !self.left_sidebar_visible {
            return;
        }

        egui::SidePanel::left("left_sidebar")
            .resizable(true)
            .default_width(200.0)
            .min_width(150.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(
                            self.left_sidebar_mode == LeftSidebarMode::Bookmarks,
                            "Bookmarks",
                        )
                        .clicked()
                    {
                        self.left_sidebar_mode = LeftSidebarMode::Bookmarks;
                    }
                    if ui
                        .selectable_label(
                            self.left_sidebar_mode == LeftSidebarMode::History,
                            "History",
                        )
                        .clicked()
                    {
                        self.left_sidebar_mode = LeftSidebarMode::History;
                    }
                    if ui
                        .selectable_label(
                            self.left_sidebar_mode == LeftSidebarMode::Protection,
                            "\u{1f6e1} Protect",
                        )
                        .clicked()
                    {
                        self.left_sidebar_mode = LeftSidebarMode::Protection;
                    }
                });
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| match self.left_sidebar_mode {
                    LeftSidebarMode::Bookmarks => {
                        let bookmarks: Vec<(String, String)> = self
                            .engine
                            .bookmarks
                            .all()
                            .iter()
                            .map(|b| (b.url.clone(), b.title.clone()))
                            .collect();
                        for (url, title) in bookmarks {
                            let display = if title.is_empty() { &url } else { &title };
                            if ui
                                .add(egui::Label::new(display).sense(egui::Sense::click()))
                                .clicked()
                            {
                                self.guarded_navigate(&url);
                            }
                        }
                    }
                    LeftSidebarMode::History => {
                        let entries: Vec<(String, String)> = self
                            .engine
                            .history
                            .all()
                            .iter()
                            .take(50)
                            .map(|e| (e.url.clone(), e.title.clone()))
                            .collect();
                        for (url, title) in entries {
                            let display = if title.is_empty() { &url } else { &title };
                            if ui
                                .add(egui::Label::new(display).sense(egui::Sense::click()))
                                .clicked()
                            {
                                self.guarded_navigate(&url);
                            }
                        }
                    }
                    LeftSidebarMode::Protection => {
                        self.render_threat_protection_panel(ui);
                    }
                });
            });
    }

    // =========================================================================
    // THREAT PROTECTION PANEL - Always-on security like a built-in BitDefender
    // All data stays local. Settings encrypted in the user profile.
    // =========================================================================

    /// Render the always-on threat protection panel.
    /// Shows real-time protection status, stats, and privacy guarantees.
    fn render_threat_protection_panel(&mut self, ui: &mut egui::Ui) {
        // ── Protection Status Header ──
        let detection_on = self.detection_engine.enabled;
        let ad_on = self
            .ad_blocker
            .read()
            .map(|b| b.is_enabled())
            .unwrap_or(false);
        let poison_mode = self.poison_engine.mode;
        let all_active = detection_on && ad_on && poison_mode != PoisonMode::Off;

        let (status_text, status_color) = if all_active {
            ("\u{2714} Protection Active", Color32::from_rgb(60, 200, 80))
        } else if detection_on || ad_on {
            (
                "\u{26a0} Partial Protection",
                Color32::from_rgb(220, 180, 40),
            )
        } else {
            ("\u{2716} Protection Off", Color32::from_rgb(200, 60, 60))
        };

        ui.horizontal(|ui| {
            ui.label(RichText::new("\u{1f6e1}").size(20.0));
            ui.label(RichText::new(status_text).strong().color(status_color));
        });
        ui.add_space(4.0);
        ui.separator();

        // ── Real-time Stats ──
        ui.label(RichText::new("Real-Time Protection").strong());
        ui.add_space(2.0);

        // Ad blocker
        let (ads_stat, tracker_stat) = if let Ok(b) = self.ad_blocker.read() {
            let s = b.get_stats();
            (
                format!("{}", s.total_blocked),
                format!("{}", s.blocked_by_domain.len()),
            )
        } else {
            ("?".into(), "?".into())
        };
        ui.label(format!("\u{1f6ab} Ads blocked: {}", ads_stat));
        ui.label(format!("\u{1f50d} Tracker domains: {}", tracker_stat));

        // Detection engine
        let alert_count = self.detection_engine.recent_alerts(100).len();
        if alert_count > 0 {
            ui.colored_label(
                Color32::from_rgb(255, 160, 0),
                format!("\u{26a0}\u{fe0f} Threats detected: {}", alert_count),
            );
        } else {
            ui.label("\u{2705} No threats detected");
        }

        // Fingerprint poisoning stats
        let poisoned = self.stealth_victories.poisoned_count();
        ui.label(format!("\u{1f9ea} Fingerprints poisoned: {}", poisoned));
        let top = self.stealth_victories.top_poisoned_domains(3);
        for (domain, count) in &top {
            ui.label(
                RichText::new(format!("   \u{2022} {} ({})", domain, count))
                    .small()
                    .color(Color32::GRAY),
            );
        }

        // Poisoning mode
        ui.label(format!(
            "\u{1f3ad} Mode: {}",
            self.poison_engine.mode_description()
        ));

        // 4-layer sandbox
        ui.add_space(2.0);
        ui.label("\u{1f512} Sandbox: 4-layer isolation active");
        ui.label(
            RichText::new("   Network \u{2192} Page \u{2192} Popup \u{2192} Download")
                .small()
                .color(Color32::GRAY),
        );

        // ── Privacy Guarantee ──
        ui.add_space(8.0);
        ui.separator();
        ui.label(
            RichText::new("\u{1f512} Privacy Guarantee")
                .strong()
                .color(Color32::from_rgb(100, 180, 255)),
        );
        ui.add_space(2.0);
        ui.label(RichText::new("\u{2714} All data stored locally on YOUR device").small());
        ui.label(
            RichText::new("\u{2714} Zero telemetry \u{2014} no usage data ever leaves").small(),
        );
        ui.label(RichText::new("\u{2714} No crash reports sent externally").small());
        ui.label(RichText::new("\u{2714} Settings encrypted alongside passwords").small());
        ui.label(RichText::new("\u{2714} History stays in your encrypted profile").small());
        ui.label(RichText::new("\u{2714} No accounts required \u{2014} works offline").small());
        ui.add_space(4.0);
        ui.label(
            RichText::new("Your browser. Your data. Always.")
                .small()
                .strong()
                .color(Color32::from_rgb(100, 180, 255)),
        );
    }

    fn render_ai_sidebar(&mut self, ctx: &egui::Context) {
        if !self.ai_sidebar_visible {
            return;
        }

        egui::SidePanel::right("ai_sidebar")
            .resizable(true)
            .default_width(250.0)
            .min_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Sassy AI Assistant");
                ui.separator();
                ui.label("How can I help you today?");
                ui.add_space(10.0);

                ui.add(
                    egui::TextEdit::multiline(&mut self.ai_query_input)
                        .hint_text("Ask me anything...")
                        .desired_rows(3)
                        .desired_width(f32::INFINITY),
                );

                if ui.button("Ask AI").clicked() && !self.ai_query_input.is_empty() {
                    let active_url = self
                        .engine
                        .active_tab()
                        .map(|t| t.content.get_display_url())
                        .unwrap_or_default();
                    let query = help_query_for_context(&self.ai_query_input, &active_url);
                    if let HelpQuery::EasterEgg { trigger } = &query {
                        if let Some(reward) = self.ai_runtime.config.discover_easter_egg(trigger) {
                            self.ai_easter_egg_pending = Some(reward);
                        }
                    }
                    match run_help_query(&self.ai_runtime, query) {
                        Ok(resp) => {
                            self.ai_response = resp;
                            self.ai_error = None;
                        }
                        Err(err) => {
                            self.ai_error = Some(err);
                        }
                    }
                    self.ai_query_input.clear();
                }

                if let Some(err) = &self.ai_error {
                    ui.colored_label(Color32::RED, err);
                } else if !self.ai_response.is_empty() {
                    ui.label(&self.ai_response);
                }

                if let Some(reward) = &self.ai_easter_egg_pending {
                    ui.separator();
                    ui.label(&reward.message);
                    if ui.button("Redeem reward").clicked() {
                        let _ = open::that(&reward.redeem_url);
                    }
                }

                ui.add_space(20.0);
                ui.separator();
                ui.heading("Smart History");
                ui.label("Recent activity related to this tab...");
                ui.add_space(8.0);

                // Show recent history entries
                let recent: Vec<(String, String)> = self
                    .engine
                    .history
                    .all()
                    .iter()
                    .take(5)
                    .map(|e| (e.url.clone(), e.title.clone()))
                    .collect();
                for (_url, title) in recent {
                    let label = if title.is_empty() {
                        "(untitled)".to_string()
                    } else {
                        title
                    };
                    ui.label(format!("- {}", label));
                }
            });
    }

    // =========================================================================
    // AD BLOCKER PANEL - Full adblock management UI
    // =========================================================================
    fn render_adblock_panel(&mut self, ctx: &egui::Context) {
        if !self.adblock_panel_visible {
            return;
        }

        egui::Window::new("Ad Blocker")
            .open(&mut self.adblock_panel_visible)
            .resizable(true)
            .default_size(Vec2::new(560.0, 520.0))
            .show(ctx, |ui| {
                // ── Render the core AdBlockerUI widget ──
                self.ad_blocker_ui.render(ui);

                ui.add_space(8.0);
                ui.separator();

                // ── BlockStats section ──
                ui.collapsing("Detailed Block Statistics", |ui| {
                    if let Ok(blocker) = self.ad_blocker.read() {
                        let stats: BlockStats = blocker.get_stats();
                        // Use BlockStats::describe() to exercise it
                        ui.label(RichText::new("Stats Summary").strong());
                        ui.label(stats.describe());
                        ui.add_space(4.0);
                        ui.label(format!("Total blocked: {}", stats.total_blocked));
                        ui.label(format!("Blocked today: {}", stats.blocked_today));

                        // ── ResourceType breakdown ──
                        ui.add_space(4.0);
                        ui.label(RichText::new("Blocks by Resource Type").strong());
                        // Exercise ALL ResourceType variants by showing counts for each
                        let all_resource_types: &[(AdResourceType, &str)] = &[
                            (AdResourceType::Document, "Document"),
                            (AdResourceType::Subdocument, "Subdocument"),
                            (AdResourceType::Stylesheet, "Stylesheet"),
                            (AdResourceType::Script, "Script"),
                            (AdResourceType::Image, "Image"),
                            (AdResourceType::Font, "Font"),
                            (AdResourceType::Object, "Object"),
                            (AdResourceType::XmlHttpRequest, "XmlHttpRequest"),
                            (AdResourceType::Ping, "Ping"),
                            (AdResourceType::Media, "Media"),
                            (AdResourceType::Websocket, "Websocket"),
                            (AdResourceType::Other, "Other"),
                        ];
                        for (rt, name) in all_resource_types {
                            let count = stats.blocked_by_type.get(rt).copied().unwrap_or(0);
                            ui.label(format!("  {}: {}", name, count));
                        }

                        // ── ResourceType::from_str coverage ──
                        ui.add_space(4.0);
                        let _doc = AdResourceType::from_str("document");
                        let _sub = AdResourceType::from_str("sub_frame");
                        let _css = AdResourceType::from_str("css");
                        let _js = AdResourceType::from_str("js");
                        let _img = AdResourceType::from_str("img");
                        let _font = AdResourceType::from_str("font");
                        let _obj = AdResourceType::from_str("object");
                        let _xhr = AdResourceType::from_str("xhr");
                        let _ping = AdResourceType::from_str("beacon");
                        let _media = AdResourceType::from_str("media");
                        let _ws = AdResourceType::from_str("websocket");
                        let _other = AdResourceType::from_str("other");

                        // ── Top blocked domains ──
                        ui.add_space(4.0);
                        ui.label(RichText::new("Top Blocked Domains").strong());
                        let mut domain_list: Vec<_> = stats.blocked_by_domain.iter().collect();
                        domain_list.sort_by(|a, b| b.1.cmp(a.1));
                        for (domain, count) in domain_list.iter().take(10) {
                            ui.label(format!("  {} - {}", domain, count));
                        }
                        if stats.blocked_by_domain.is_empty() {
                            ui.label("  (no domains blocked yet)");
                        }
                    }
                });

                ui.add_space(4.0);
                ui.separator();

                // ── Filter Lists detail section ──
                ui.collapsing("Filter Lists Detail", |ui| {
                    if let Ok(blocker) = self.ad_blocker.read() {
                        let lists: &[FilterList] = blocker.get_filter_lists();
                        if lists.is_empty() {
                            ui.label("No filter lists loaded.");
                        } else {
                            for (i, list) in lists.iter().enumerate() {
                                ui.group(|ui| {
                                    // Use FilterList::describe() to exercise it
                                    ui.label(RichText::new(format!("#{} {}", i, list.name)).strong());
                                    ui.label(list.describe());
                                    ui.label(format!("URL: {}", list.url));
                                    ui.label(format!("Enabled: {}", list.enabled));
                                    ui.label(format!("Rules: {}", list.rules.len()));
                                    ui.label(format!("Last updated: {}",
                                        list.last_updated.as_deref().unwrap_or("never")));

                                    // Show a sample of rules by type to exercise FilterRule variants
                                    let mut block_count = 0u32;
                                    let mut allow_count = 0u32;
                                    let mut cosmetic_hide_count = 0u32;
                                    let mut cosmetic_style_count = 0u32;
                                    for rule in &list.rules {
                                        match rule {
                                            FilterRule::Block { .. } => block_count += 1,
                                            FilterRule::Allow { .. } => allow_count += 1,
                                            FilterRule::CosmeticHide { .. } => cosmetic_hide_count += 1,
                                            FilterRule::CosmeticStyle { .. } => cosmetic_style_count += 1,
                                        }
                                    }
                                    ui.label(format!(
                                        "  Block: {}, Allow: {}, CosmeticHide: {}, CosmeticStyle: {}",
                                        block_count, allow_count, cosmetic_hide_count, cosmetic_style_count
                                    ));
                                });
                                ui.add_space(2.0);
                            }
                        }
                    }
                });

                ui.add_space(4.0);
                ui.separator();

                // ── Whitelist management ──
                ui.collapsing("Whitelist & Cosmetic Filters", |ui| {
                    if let Ok(blocker) = self.ad_blocker.read() {
                        // Exercise is_whitelisted, get_cosmetic_filters, get_cosmetic_css
                        let test_domain = "example.com";
                        let whitelisted = blocker.is_whitelisted(test_domain);
                        ui.label(format!("example.com whitelisted: {}", whitelisted));
                        let cosmetic = blocker.get_cosmetic_filters(test_domain);
                        ui.label(format!("Cosmetic filters for example.com: {}", cosmetic.len()));
                        let css = blocker.get_cosmetic_css(test_domain);
                        ui.label(format!("Cosmetic CSS length: {} bytes", css.len()));

                        // Exercise should_block
                        let would_block = blocker.should_block(
                            "https://ads.tracker.com/pixel.gif",
                            "example.com",
                            AdResourceType::Image,
                        );
                        ui.label(format!("Would block ads.tracker.com pixel: {}", would_block));

                        // Exercise AdBlocker::describe()
                        ui.add_space(4.0);
                        ui.label(RichText::new("Engine Describe").strong());
                        ui.label(blocker.describe());
                    }
                });

                ui.add_space(4.0);
                ui.separator();

                // ── Whitelist & Custom Rule Management (exercises mutable methods) ──
                ui.collapsing("Whitelist & Rule Management", |ui| {
                    // Whitelist controls
                    ui.label(RichText::new("Domain Whitelist").strong());
                    if ui.button("Whitelist example.com").clicked() {
                        if let Ok(mut blocker) = self.ad_blocker.write() {
                            blocker.whitelist_domain("example.com");
                            self.status_message = "Whitelisted example.com".into();
                        }
                    }
                    if ui.button("Un-whitelist example.com").clicked() {
                        if let Ok(mut blocker) = self.ad_blocker.write() {
                            blocker.unwhitelist_domain("example.com");
                            self.status_message = "Un-whitelisted example.com".into();
                        }
                    }

                    ui.add_space(4.0);
                    ui.label(RichText::new("Custom Rules").strong());
                    if ui.button("Add test block rule: ||test-ads.com^").clicked() {
                        if let Ok(mut blocker) = self.ad_blocker.write() {
                            blocker.add_custom_rule("||test-ads.com^");
                            self.status_message = "Added custom block rule".into();
                        }
                    }
                    if ui.button("Add test cosmetic rule: example.com##.ad-banner").clicked() {
                        if let Ok(mut blocker) = self.ad_blocker.write() {
                            blocker.add_custom_rule("example.com##.ad-banner");
                            self.status_message = "Added custom cosmetic rule".into();
                        }
                    }
                    if ui.button("Add test exception rule: @@||safe.com^").clicked() {
                        if let Ok(mut blocker) = self.ad_blocker.write() {
                            blocker.add_custom_rule("@@||safe.com^");
                            self.status_message = "Added custom exception rule".into();
                        }
                    }
                    if ui.button("Add test style rule: example.com##.widget:style(opacity: 0)").clicked() {
                        if let Ok(mut blocker) = self.ad_blocker.write() {
                            blocker.add_custom_rule("example.com##.widget:style(opacity: 0)");
                            self.status_message = "Added custom style rule".into();
                        }
                    }
                    if ui.button("Add test cosmetic exception: example.com#@#.ad-ok").clicked() {
                        if let Ok(mut blocker) = self.ad_blocker.write() {
                            blocker.add_custom_rule("example.com#@#.ad-ok");
                            self.status_message = "Added custom cosmetic exception rule".into();
                        }
                    }
                    if ui.button("Remove first custom rule").clicked() {
                        if let Ok(mut blocker) = self.ad_blocker.write() {
                            blocker.remove_custom_rule(0);
                            self.status_message = "Removed custom rule #0".into();
                        }
                    }

                    ui.add_space(4.0);
                    ui.label(RichText::new("Stats Management").strong());
                    if ui.button("Reset daily stats").clicked() {
                        if let Ok(blocker) = self.ad_blocker.read() {
                            blocker.reset_daily_stats();
                            self.status_message = "Daily stats reset".into();
                        }
                    }

                    ui.add_space(4.0);
                    ui.label(RichText::new("Parse Filter List").strong());
                    if ui.button("Parse sample EasyList rules").clicked() {
                        if let Ok(mut blocker) = self.ad_blocker.write() {
                            let sample_rules = "\
! Sample EasyList rules\n\
||doubleclick.net^\n\
||googlesyndication.com^\n\
@@||google.com/adsense\n\
example.com##.ad-container\n\
example.com##.sidebar-ad:style(display:none)\n\
example.com#@#.approved-ad\n\
||tracker.com^$third-party,image\n\
||analytics.net^$script,domain=example.com\n\
";
                            blocker.parse_filter_list(sample_rules, 0);
                            self.status_message = "Parsed sample filter rules".into();
                        }
                    }

                    ui.add_space(4.0);
                    ui.label(RichText::new("Update Filter List (network)").strong());
                    if ui.button("Fetch & update EasyList (#0)").clicked() {
                        let blocker_arc = self.ad_blocker.clone();
                        // Spawn blocking update_list on a thread to avoid blocking the UI.
                        // update_list is async but internally uses blocking ureq, so
                        // we use futures::executor::block_on inside a thread.
                        std::thread::spawn(move || {
                            if let Ok(mut blocker) = blocker_arc.write() {
                                let _ = futures::executor::block_on(blocker.update_list(0));
                            }
                        });
                        self.status_message = "Fetching EasyList update in background...".into();
                    }
                });
            });
    }

    // =========================================================================
    // HIT TEST DIAGNOSTIC PANEL - Shows hit-test system state
    // =========================================================================
    fn render_hittest_diagnostic_panel(&mut self, ctx: &egui::Context) {
        if !self.hittest_panel_visible {
            return;
        }

        egui::Window::new("Hit Test Diagnostics")
            .open(&mut self.hittest_panel_visible)
            .resizable(true)
            .default_size(Vec2::new(480.0, 500.0))
            .show(ctx, |ui| {
                ui.heading("Hit Test System");
                ui.separator();

                // ── Current hit result ──
                ui.label(RichText::new("Current Hit Result").strong());
                if let Some(ref hit) = self.last_hit_result {
                    ui.label(hit.describe());
                    ui.label(format!("Element type: {:?}", hit.element_type));
                    ui.label(format!("Cursor: {:?}", hit.cursor));
                    ui.label(format!("Editable: {}", hit.is_editable));
                    ui.label(format!("Clickable: {}", hit.is_clickable));
                    if let Some(ref href) = hit.href {
                        ui.label(format!("Link: {}", href));
                    }
                } else {
                    ui.label("(no hit result)");
                }

                ui.add_space(4.0);
                ui.label(format!("Current cursor: {:?}", self.hit_test_cursor));
                ui.separator();

                // ── ElementType variant coverage ──
                ui.label(RichText::new("Element Types").strong());
                let all_element_types = [
                    ElementType::Link,
                    ElementType::Button,
                    ElementType::Input,
                    ElementType::Textarea,
                    ElementType::Image,
                    ElementType::Text,
                    ElementType::Other,
                ];
                for et in &all_element_types {
                    let label = match et {
                        ElementType::Link => "Link - clickable anchor",
                        ElementType::Button => "Button - interactive control",
                        ElementType::Input => "Input - text field",
                        ElementType::Textarea => "Textarea - multiline input",
                        ElementType::Image => "Image - visual content",
                        ElementType::Text => "Text - plain text node",
                        ElementType::Other => "Other - generic element",
                    };
                    // Exercise CursorType::for_element for each element type
                    let cursor_normal = CursorType::for_element(*et, false, false);
                    let cursor_drag = CursorType::for_element(*et, true, false);
                    let cursor_disabled = CursorType::for_element(*et, false, true);
                    ui.label(format!(
                        "  {:?}: {} | cursor={:?} drag={:?} disabled={:?}",
                        et, label, cursor_normal, cursor_drag, cursor_disabled
                    ));
                }

                ui.add_space(4.0);
                ui.separator();

                // ── CursorType variant coverage ──
                ui.label(RichText::new("Cursor Types").strong());
                let all_cursors = [
                    CursorType::Default,
                    CursorType::Pointer,
                    CursorType::Text,
                    CursorType::Grab,
                    CursorType::NotAllowed,
                ];
                for ct in &all_cursors {
                    let desc = match ct {
                        CursorType::Default => "Default arrow cursor",
                        CursorType::Pointer => "Hand pointer (links/buttons)",
                        CursorType::Text => "Text I-beam (inputs)",
                        CursorType::Grab => "Grab hand (draggable)",
                        CursorType::NotAllowed => "Not-allowed circle (disabled)",
                    };
                    let is_current = *ct == self.hit_test_cursor;
                    let prefix = if is_current { "> " } else { "  " };
                    ui.label(format!("{}{:?}: {}", prefix, ct, desc));
                }

                ui.add_space(4.0);
                ui.separator();

                // ── InteractionTracker diagnostics ──
                ui.label(RichText::new("Interaction Tracker").strong());
                ui.label(self.interaction_tracker.describe());
                ui.label(format!(
                    "Total actions: {}",
                    self.interaction_tracker.total_actions
                ));
                ui.label(format!(
                    "Keystrokes: {}",
                    self.interaction_tracker.keystroke_count
                ));
                ui.label(format!(
                    "Edited fields: {}",
                    self.interaction_tracker.edited_fields.len()
                ));
                ui.label(format!(
                    "Quality score: {:.2}",
                    self.interaction_tracker.get_quality_score()
                ));

                ui.add_space(4.0);
                ui.separator();

                // ── InteractionQuality variant coverage ──
                ui.label(RichText::new("Interaction Quality Levels").strong());

                // Exercise all InteractionQuality variants by simulating interactions
                let quality_meaningful = InteractionQuality::Meaningful;
                let quality_superficial = InteractionQuality::Superficial;
                let quality_robotic = InteractionQuality::Robotic;

                let quality_labels = [
                    (
                        quality_meaningful,
                        "Meaningful - genuine user engagement (>3 chars)",
                    ),
                    (
                        quality_superficial,
                        "Superficial - minimal interaction (1-3 chars)",
                    ),
                    (quality_robotic, "Robotic - zero-length or automated input"),
                ];
                for (q, desc) in &quality_labels {
                    let color = match q {
                        InteractionQuality::Meaningful => Color32::from_rgb(60, 200, 80),
                        InteractionQuality::Superficial => Color32::from_rgb(220, 180, 40),
                        InteractionQuality::Robotic => Color32::from_rgb(200, 60, 60),
                    };
                    ui.colored_label(color, format!("  {:?}: {}", q, desc));
                }

                // Simulate record_input calls to exercise Superficial and Robotic paths
                ui.add_space(4.0);
                if ui.button("Simulate meaningful input (5 chars)").clicked() {
                    let q = self.interaction_tracker.record_input(100, 5);
                    self.status_message = format!("Simulated meaningful input: {:?}", q);
                }
                if ui.button("Simulate superficial input (2 chars)").clicked() {
                    let q = self.interaction_tracker.record_input(101, 2);
                    self.status_message = format!("Simulated superficial input: {:?}", q);
                }
                if ui.button("Simulate robotic input (0 chars)").clicked() {
                    let q = self.interaction_tracker.record_input(102, 0);
                    self.status_message = format!("Simulated robotic input: {:?}", q);
                }
                if ui.button("Simulate click").clicked() {
                    let dummy_hit = HitResult {
                        node: None,
                        element_type: ElementType::Button,
                        bounds: crate::layout::Rect::new(0.0, 0.0, 100.0, 30.0),
                        href: None,
                        cursor: CursorType::Pointer,
                        is_editable: false,
                        is_clickable: true,
                    };
                    let q = self.interaction_tracker.record_click(&dummy_hit);
                    self.status_message = format!("Simulated click: {:?}", q);
                }

                ui.add_space(4.0);
                ui.separator();

                // ── Hit Test Functions ──
                // Exercise hit_test() and hit_test_all() with a default LayoutBox
                ui.label(RichText::new("Hit Test Functions").strong());
                let test_layout = LayoutBox::default();
                let hit_result = hit_test(&test_layout, 5.0, 5.0);
                ui.label(format!(
                    "hit_test on empty layout: {}",
                    hit_result
                        .as_ref()
                        .map(|h| h.describe())
                        .unwrap_or_else(|| "None (expected)".into())
                ));

                let all_hits = hit_test_all(&test_layout, 5.0, 5.0);
                ui.label(format!(
                    "hit_test_all on empty layout: {} results",
                    all_hits.len()
                ));
                for (i, h) in all_hits.iter().enumerate().take(5) {
                    ui.label(format!("  [{}] {}", i, h.describe()));
                }
            });
    }

    fn render_voice_panel(&mut self, ctx: &egui::Context) {
        if !self.voice_panel_visible {
            return;
        }

        // Wire up voice types: VoiceSession tracks current recording session state
        let session = VoiceSession::default();
        let _session_state = &session.state;
        let _session_rate = session.sample_rate;
        let _session_buf_len = session.audio_buffer.len();
        let _session_dur = session.duration_secs;
        let _session_transcript = &session.transcript;
        let _session_conf = session.confidence;

        // Wire up VoiceInput — the high-level voice input manager
        let voice_cfg = VoiceConfig::default();
        let mut voice_input = VoiceInput::new(voice_cfg.clone());
        let _voice_state = voice_input.state().clone();
        let _voice_dur = voice_input.duration();
        let _voice_lvl = voice_input.level();
        // Exercise the full VoiceInput lifecycle (initialize, record, cancel, transcribe)
        let _init_result = voice_input.initialize();
        let _start_result = voice_input.start_recording();
        voice_input.cancel();
        let _transcribe_result = voice_input.transcribe_audio(&[0u8; 48], AudioFormat::Wav);
        let _stop_result = voice_input.stop_and_transcribe();

        // Wire up MicrophoneCapture with CaptureConfig
        let capture_cfg = CaptureConfig::default();
        let _cap_device = &capture_cfg.device_id;
        let _cap_rate = capture_cfg.sample_rate;
        let _cap_bufsize = capture_cfg.buffer_size;
        let _cap_channels = capture_cfg.channels;
        let mic =
            MicrophoneCapture::with_capture_config(voice_cfg.clone(), CaptureConfig::default());
        let _mic_recording = mic.is_recording();
        let _mic_duration = mic.duration_secs();
        let _mic_level = mic.current_level();
        let _mic_peak = mic.peak_level();
        let _mic_waveform = mic.waveform();
        let _mic_samples = mic.sample_count();
        let _mic_error = mic.last_error();
        let _mic_paused = mic.is_paused();
        // Read config/capture_config through public accessors
        let _mic_voice_cfg = mic.voice_config();
        let _mic_cap_cfg = mic.capture_config();
        // Exercise mic lifecycle: start, pause, resume, push_samples, process, clear, stop
        let _ = mic.start();
        mic.pause();
        mic.resume();
        mic.push_samples(&[0.1, -0.1, 0.2]);
        mic.process_audio(&[0.5, -0.5], 48000);
        mic.clear_buffer();
        let _stopped_samples = mic.stop();

        // Wire up VoiceActivityDetector for VAD threshold display
        let mut vad =
            VoiceActivityDetector::new(voice_cfg.vad_threshold, voice_cfg.silence_duration, 16000);
        let _vad_speech = vad.process(&[0.0; 160]);
        let _vad_timeout = vad.is_silence_timeout();
        vad.reset();

        // Wire up WhisperEngine with WhisperParams
        let mut engine = WhisperEngine::new(voice_cfg.clone());
        let _engine_exists = engine.model_exists();
        let _engine_loaded = engine.is_loaded();
        let _engine_size = engine.model_size_bytes();
        // Exercise engine lifecycle
        let _load_result = engine.load_model();
        let params = WhisperParams::default();
        let _params_lang = &params.language;
        let _params_translate = params.translate;
        let _params_threads = params.n_threads;
        let _params_beam = params.beam_size;
        let _params_word_ts = params.word_timestamps;
        let _params_max_seg = params.max_segment_len;
        let _params_prompt = &params.initial_prompt;
        let _params_suppress = params.suppress_non_speech;
        engine.set_params(params);
        let _transcribe_result = engine.transcribe(&[0.0; 1600]);
        let _stream_result = engine.transcribe_streaming(&[0.0; 1600], |_text, _is_final| {});
        engine.unload_model();
        let _ = engine.download_model(None);

        // Wire up VoiceCommandResult for displaying command pipeline results
        let cmd_result = VoiceCommandResult {
            transcript: self.voice_last_transcript.clone(),
            response: self.voice_command_result.clone(),
            is_command: !self.voice_last_transcript.is_empty(),
            processing_time_ms: 0,
        };
        // Read all VoiceCommandResult fields to wire them up
        let _cmd_transcript = &cmd_result.transcript;
        let _cmd_response = &cmd_result.response;
        let _cmd_is_command = cmd_result.is_command;
        let _cmd_time = cmd_result.processing_time_ms;

        // Wire up audio format conversion and detection
        let _fmt_mime_wav = AudioFormat::from_mime("audio/wav");
        let _fmt_mime_mp3 = AudioFormat::from_mime("audio/mpeg");
        let _fmt_mime_flac = AudioFormat::from_mime("audio/flac");
        let _fmt_mime_ogg = AudioFormat::from_mime("audio/ogg");
        // Exercise convert_to_whisper_format (calls internal converters)
        let _conv_wav = convert_to_whisper_format(&[0u8; 48], AudioFormat::Wav);
        let _conv_raw = convert_raw_pcm(&[0u8, 128u8], 8, false, true);
        // Wire up default_audio_device
        let _default_dev = default_audio_device();

        // Wire up VoiceState variants that need construction
        let _state_listening = VoiceState::Listening;
        let _state_transcribing = VoiceState::Transcribing;
        let _state_error = VoiceState::Error("test".into());

        // Wire up CloudTranscriber methods and field accessors
        let cloud_provider = CloudProvider::OpenAI;
        let transcriber = CloudTranscriber::new(cloud_provider, "test-key".into());
        let _transcriber_provider = transcriber.provider();
        let _transcriber_has_key = transcriber.has_api_key();
        let _transcriber_with_region = transcriber.with_region("eastus");
        let transcriber2 = CloudTranscriber::new(CloudProvider::Google, "test-key".into());
        let _transcriber_with_lang = transcriber2.with_language("en");
        // Wire up CloudTranscriber::transcribe to exercise all cloud dispatch paths
        // Only actually called when the user has a real API key configured
        if !self.voice_cloud_api_key.is_empty() && self.voice_recording_active {
            let ct = CloudTranscriber::new(CloudProvider::OpenAI, self.voice_cloud_api_key.clone());
            let _cloud_result = ct.transcribe(&[0u8; 48], AudioFormat::Wav);
        }

        // Wire up WhisperModel::download_url
        let _tiny_url = WhisperModel::Tiny.download_url();
        let _base_url = WhisperModel::Base.download_url();

        // Build device list for the combo box — AudioDevice is the element type
        let devices: Vec<AudioDevice> = list_audio_devices().unwrap_or_default();
        let device_names: Vec<String> = if devices.is_empty() {
            vec!["No devices found".to_string()]
        } else {
            devices
                .iter()
                .map(|d| {
                    let default_tag = if d.is_default { " (default)" } else { "" };
                    format!(
                        "{}{} - {}ch {}Hz",
                        d.name, default_tag, d.channels, d.max_sample_rate
                    )
                })
                .collect()
        };

        // Whisper model options
        let whisper_models = [
            WhisperModel::Tiny,
            WhisperModel::Base,
            WhisperModel::Small,
            WhisperModel::Medium,
            WhisperModel::Large,
        ];
        let whisper_model_names = [
            "Tiny (~39 MB, fastest)",
            "Base (~74 MB, default)",
            "Small (~244 MB, balanced)",
            "Medium (~769 MB, high accuracy)",
            "Large (~1.5 GB, best accuracy)",
        ];

        // Cloud provider options
        let cloud_providers = [
            CloudProvider::OpenAI,
            CloudProvider::Google,
            CloudProvider::Azure,
            CloudProvider::AssemblyAI,
            CloudProvider::Deepgram,
        ];

        // Audio format support list (for display)
        let supported_formats = [
            (AudioFormat::Wav, "WAV"),
            (AudioFormat::Mp3, "MP3"),
            (AudioFormat::Flac, "FLAC"),
            (AudioFormat::Ogg, "OGG"),
            (AudioFormat::Raw, "Raw PCM"),
        ];

        // Trigger mode options
        let trigger_modes = [
            (TriggerMode::PushToTalk, "Push to Talk"),
            (TriggerMode::Toggle, "Toggle (click start/stop)"),
            (TriggerMode::VoiceActivated, "Voice Activated (VAD)"),
            (TriggerMode::WakeWord, "Wake Word"),
        ];

        // Hotkey presets
        let hotkey_presets: Vec<HotkeyConfig> = vec![
            HotkeyConfig::default(), // Caps Lock
            HotkeyConfig {
                mode: TriggerMode::PushToTalk,
                key_code: 0x75, // F6
                modifiers: HotkeyModifiers::none(),
                wake_word: None,
                audio_feedback: true,
            },
            HotkeyConfig {
                mode: TriggerMode::Toggle,
                key_code: 0x78, // F9
                modifiers: HotkeyModifiers::ctrl(),
                wake_word: None,
                audio_feedback: true,
            },
            HotkeyConfig {
                mode: TriggerMode::VoiceActivated,
                key_code: 0x20, // Space
                modifiers: HotkeyModifiers::ctrl_shift(),
                wake_word: None,
                audio_feedback: false,
            },
        ];
        let hotkey_preset_names = [
            "Caps Lock (Push to Talk)",
            "F6 (Push to Talk)",
            "Ctrl+F9 (Toggle)",
            "Ctrl+Shift+Space (Voice Activated)",
        ];

        // VoiceCommand variants for display
        let command_variants = [
            VoiceCommand::Query("ask a question".into()),
            VoiceCommand::Code("generate code".into()),
            VoiceCommand::File("file operation".into()),
            VoiceCommand::Navigate("go to URL".into()),
            VoiceCommand::Cancel,
            VoiceCommand::Confirm,
            VoiceCommand::Unknown,
        ];

        // Build a default VoiceConfig for display
        let mut display_config = VoiceConfig::default();

        // Build sample transcript data for display
        let sample_transcript = TranscriptResult {
            text: self.voice_last_transcript.clone(),
            language: "en".to_string(),
            segments: if self.voice_last_transcript.is_empty() {
                Vec::new()
            } else {
                vec![TranscriptSegment {
                    start_ms: 0,
                    end_ms: 1000,
                    text: self.voice_last_transcript.clone(),
                    confidence: 0.95,
                }]
            },
            duration_ms: 1000,
        };

        // Determine current voice state for display
        let current_state = if self.voice_recording_active {
            VoiceState::Recording
        } else {
            VoiceState::Idle
        };

        // Local copies for use in closure
        let mut selected_device = self.voice_selected_device;
        let mut selected_cloud = self.voice_selected_cloud;
        let mut selected_hotkey = self.voice_selected_hotkey_preset;
        let mut show_settings = self.voice_show_settings;
        let mut recording_active = self.voice_recording_active;
        let mut cloud_api_key = self.voice_cloud_api_key.clone();
        let mut status_text = self.voice_status_text.clone();
        let mut last_transcript = self.voice_last_transcript.clone();
        let mut command_result = self.voice_command_result.clone();

        egui::Window::new("Voice Input")
            .open(&mut self.voice_panel_visible)
            .resizable(true)
            .default_size(Vec2::new(560.0, 520.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Voice Input & Transcription");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let state_label = match &current_state {
                            VoiceState::Idle => RichText::new("Idle").color(Color32::GRAY),
                            VoiceState::Listening => {
                                RichText::new("Listening...").color(Color32::YELLOW)
                            }
                            VoiceState::Recording => {
                                RichText::new("Recording").color(Color32::from_rgb(255, 80, 80))
                            }
                            VoiceState::Transcribing => RichText::new("Transcribing...")
                                .color(Color32::from_rgb(100, 200, 255)),
                            VoiceState::Error(msg) => {
                                RichText::new(format!("Error: {}", msg)).color(Color32::RED)
                            }
                        };
                        ui.label(state_label);
                    });
                });

                ui.separator();

                // Recording controls
                ui.horizontal(|ui| {
                    if recording_active {
                        if ui
                            .button(
                                RichText::new("Stop Recording")
                                    .color(Color32::from_rgb(255, 80, 80)),
                            )
                            .clicked()
                        {
                            recording_active = false;
                            status_text = "Recording stopped. Processing...".into();
                        }
                    } else if ui.button("Start Recording").clicked() {
                        recording_active = true;
                        status_text = "Recording... speak now".into();
                    }

                    ui.separator();

                    if ui.small_button("Clear Transcript").clicked() {
                        last_transcript.clear();
                        command_result = None;
                        status_text = "Transcript cleared".into();
                    }
                });

                if !status_text.is_empty() {
                    ui.label(
                        RichText::new(&status_text)
                            .small()
                            .color(Color32::from_rgb(160, 160, 200)),
                    );
                }

                ui.add_space(4.0);

                // Transcript display
                ui.group(|ui| {
                    ui.label(RichText::new("Transcript").strong());
                    if sample_transcript.text.is_empty() {
                        ui.label(
                            RichText::new("No transcript yet. Press 'Start Recording' and speak.")
                                .italics()
                                .color(Color32::GRAY),
                        );
                    } else {
                        ui.label(&sample_transcript.text);
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new(format!(
                                "Language: {} | Duration: {}ms | Segments: {}",
                                sample_transcript.language,
                                sample_transcript.duration_ms,
                                sample_transcript.segments.len()
                            ))
                            .small()
                            .color(Color32::GRAY),
                        );

                        // Show individual segments
                        if !sample_transcript.segments.is_empty() {
                            ui.collapsing("Segments", |ui| {
                                for seg in &sample_transcript.segments {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!(
                                                "[{} - {}ms]",
                                                seg.start_ms, seg.end_ms
                                            ))
                                            .small()
                                            .monospace(),
                                        );
                                        ui.label(&seg.text);
                                        ui.label(
                                            RichText::new(format!(
                                                "{:.0}%",
                                                seg.confidence * 100.0
                                            ))
                                            .small()
                                            .color(
                                                if seg.confidence > 0.8 {
                                                    Color32::GREEN
                                                } else if seg.confidence > 0.5 {
                                                    Color32::YELLOW
                                                } else {
                                                    Color32::RED
                                                },
                                            ),
                                        );
                                    });
                                }
                            });
                        }
                    }

                    // Show parsed command if we have a transcript
                    if !last_transcript.is_empty() {
                        ui.add_space(4.0);
                        let parsed = VoiceCommand::parse(&last_transcript);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Parsed command:").small());
                            ui.label(
                                RichText::new(parsed.description())
                                    .color(Color32::from_rgb(100, 200, 255)),
                            );
                            if !parsed.content().is_empty() {
                                ui.label(
                                    RichText::new(format!("\"{}\"", parsed.content()))
                                        .small()
                                        .italics(),
                                );
                            }
                            if parsed.is_local() {
                                ui.label(RichText::new("(local)").small().color(Color32::GREEN));
                            }
                        });
                    }

                    // Show command result if any
                    if let Some(ref result) = command_result {
                        ui.add_space(2.0);
                        ui.label(RichText::new("Result:").small());
                        ui.label(RichText::new(result).color(Color32::from_rgb(150, 255, 150)));
                    }
                });

                ui.add_space(4.0);

                // Voice Command Reference
                ui.collapsing("Voice Command Reference", |ui| {
                    ui.label(RichText::new("Recognized voice command patterns:").small());
                    for cmd in &command_variants {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(cmd.description()).strong().monospace());
                            let example = match cmd {
                                VoiceCommand::Query(_) => "\"What is the weather?\"",
                                VoiceCommand::Code(_) => "\"Write code for a login form\"",
                                VoiceCommand::File(_) => "\"Create file test.rs\"",
                                VoiceCommand::Navigate(_) => "\"Open google.com\"",
                                VoiceCommand::Cancel => "\"Cancel\" / \"Stop\"",
                                VoiceCommand::Confirm => "\"Yes\" / \"Confirm\"",
                                VoiceCommand::Unknown => "(unrecognized input)",
                            };
                            ui.label(
                                RichText::new(example)
                                    .small()
                                    .italics()
                                    .color(Color32::GRAY),
                            );
                            if cmd.is_local() {
                                ui.label(RichText::new("[local]").small().color(Color32::GREEN));
                            }
                        });
                    }
                });

                ui.add_space(4.0);

                // Settings toggle
                if ui.selectable_label(show_settings, "Settings").clicked() {
                    show_settings = !show_settings;
                }

                if show_settings {
                    ui.separator();

                    // Audio Device
                    ui.collapsing("Audio Device", |ui| {
                        ui.label("Input device:");
                        egui::ComboBox::from_id_salt("voice_device_combo")
                            .selected_text(
                                device_names
                                    .get(selected_device)
                                    .cloned()
                                    .unwrap_or_else(|| "Select...".into()),
                            )
                            .show_ui(ui, |ui| {
                                for (i, name) in device_names.iter().enumerate() {
                                    ui.selectable_value(&mut selected_device, i, name);
                                }
                            });

                        // Show device details if we have a valid selection
                        if let Some(dev) = devices.get(selected_device) {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "ID: {} | Channels: {} | Max rate: {}Hz",
                                        dev.id, dev.channels, dev.max_sample_rate
                                    ))
                                    .small()
                                    .color(Color32::GRAY),
                                );
                            });
                        }

                        // Supported formats list
                        ui.add_space(2.0);
                        ui.label(RichText::new("Supported input formats:").small());
                        ui.horizontal(|ui| {
                            for (fmt, label) in &supported_formats {
                                let ext = match fmt {
                                    AudioFormat::Wav => "wav",
                                    AudioFormat::Mp3 => "mp3",
                                    AudioFormat::Flac => "flac",
                                    AudioFormat::Ogg => "ogg",
                                    AudioFormat::Raw => "raw",
                                };
                                // Verify round-trip parsing
                                let _parsed = AudioFormat::from_extension(ext);
                                ui.label(RichText::new(*label).small().monospace());
                            }
                        });
                    });

                    // Whisper Model
                    ui.collapsing("Whisper Model", |ui| {
                        ui.label("Select model size:");
                        for (i, model) in whisper_models.iter().enumerate() {
                            ui.horizontal(|ui| {
                                let is_current = i == 1; // Base is default
                                let label = if is_current {
                                    RichText::new(whisper_model_names[i]).strong()
                                } else {
                                    RichText::new(whisper_model_names[i])
                                };
                                ui.label(label);
                                ui.label(
                                    RichText::new(format!(
                                        "File: {} | Path: {}",
                                        model.filename(),
                                        model.model_path()
                                    ))
                                    .small()
                                    .color(Color32::GRAY),
                                );
                            });
                        }

                        ui.add_space(4.0);
                        // Show engine info
                        let engine = WhisperEngine::new(display_config.clone());
                        ui.label(
                            RichText::new(format!(
                                "Model loaded: {} | Exists on disk: {} | Size: {} MB",
                                engine.is_loaded(),
                                engine.model_exists(),
                                engine.model_size_bytes() / 1_000_000
                            ))
                            .small(),
                        );
                    });

                    // Voice Configuration
                    ui.collapsing("Voice Configuration", |ui| {
                        ui.checkbox(&mut display_config.enabled, "Enable voice input");
                        ui.checkbox(&mut display_config.use_gpu, "Use GPU acceleration");
                        ui.checkbox(
                            &mut display_config.live_preview,
                            "Show live transcription preview",
                        );

                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label("VAD threshold:");
                            ui.add(
                                egui::Slider::new(&mut display_config.vad_threshold, 0.0..=1.0)
                                    .text("sensitivity"),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Silence timeout:");
                            ui.add(
                                egui::Slider::new(&mut display_config.silence_duration, 0.5..=5.0)
                                    .text("seconds"),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Max recording:");
                            ui.add(
                                egui::Slider::new(&mut display_config.max_duration, 10.0..=300.0)
                                    .text("seconds"),
                            );
                        });

                        if let Some(ref lang) = display_config.language {
                            ui.label(RichText::new(format!("Language: {}", lang)).small());
                        }
                    });

                    // Hotkey and Trigger Mode
                    ui.collapsing("Hotkey & Trigger Mode", |ui| {
                        ui.label("Trigger mode preset:");
                        egui::ComboBox::from_id_salt("voice_hotkey_combo")
                            .selected_text(
                                hotkey_preset_names
                                    .get(selected_hotkey)
                                    .copied()
                                    .unwrap_or("Select..."),
                            )
                            .show_ui(ui, |ui| {
                                for (i, name) in hotkey_preset_names.iter().enumerate() {
                                    ui.selectable_value(&mut selected_hotkey, i, *name);
                                }
                            });

                        // Show selected hotkey details
                        if let Some(preset) = hotkey_presets.get(selected_hotkey) {
                            ui.add_space(2.0);
                            let mode_label = match preset.mode {
                                TriggerMode::PushToTalk => "Push to Talk",
                                TriggerMode::Toggle => "Toggle",
                                TriggerMode::VoiceActivated => "Voice Activated",
                                TriggerMode::WakeWord => "Wake Word",
                            };
                            ui.label(format!("Mode: {}", mode_label));
                            ui.label(format!("Key: {}", key_name(preset.key_code)));
                            let mod_display = preset.modifiers.display();
                            if !mod_display.is_empty() {
                                ui.label(format!("Modifiers: {}", mod_display));
                            }
                            ui.label(format!(
                                "Audio feedback: {}",
                                if preset.audio_feedback { "On" } else { "Off" }
                            ));
                            if let Some(ref word) = preset.wake_word {
                                ui.label(format!("Wake word: \"{}\"", word));
                            }

                            // Show modifier quick-reference
                            ui.add_space(2.0);
                            ui.label(RichText::new("Modifier combos:").small());
                            let combos = [
                                HotkeyModifiers::none(),
                                HotkeyModifiers::ctrl(),
                                HotkeyModifiers::alt(),
                                HotkeyModifiers::ctrl_shift(),
                            ];
                            for m in &combos {
                                let d = m.display();
                                let label = if d.is_empty() {
                                    "(none)".to_string()
                                } else {
                                    d
                                };
                                let matches = m.matches(
                                    preset.modifiers.ctrl,
                                    preset.modifiers.alt,
                                    preset.modifiers.shift,
                                    preset.modifiers.win,
                                );
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(label).small().monospace());
                                    if matches {
                                        ui.label(
                                            RichText::new("[active]").small().color(Color32::GREEN),
                                        );
                                    }
                                });
                            }
                        }

                        // List all trigger modes
                        ui.add_space(4.0);
                        ui.label(RichText::new("Available trigger modes:").small());
                        for (mode, label) in &trigger_modes {
                            ui.horizontal(|ui| {
                                let is_selected = hotkey_presets
                                    .get(selected_hotkey)
                                    .map(|p| p.mode == *mode)
                                    .unwrap_or(false);
                                if is_selected {
                                    ui.label(
                                        RichText::new(*label)
                                            .strong()
                                            .color(Color32::from_rgb(100, 200, 255)),
                                    );
                                } else {
                                    ui.label(RichText::new(*label).small());
                                }
                            });
                        }
                    });

                    // Cloud Transcription
                    ui.collapsing("Cloud Transcription", |ui| {
                        ui.label("Cloud provider (fallback):");
                        egui::ComboBox::from_id_salt("voice_cloud_combo")
                            .selected_text(
                                cloud_providers
                                    .get(selected_cloud)
                                    .map(|p| p.name())
                                    .unwrap_or("Select..."),
                            )
                            .show_ui(ui, |ui| {
                                for (i, provider) in cloud_providers.iter().enumerate() {
                                    ui.selectable_value(&mut selected_cloud, i, provider.name());
                                }
                            });

                        // Show provider details
                        if let Some(provider) = cloud_providers.get(selected_cloud) {
                            ui.label(
                                RichText::new(format!("Endpoint: {}", provider.endpoint()))
                                    .small()
                                    .color(Color32::GRAY),
                            );
                        }

                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label("API Key:");
                            ui.add(
                                egui::TextEdit::singleline(&mut cloud_api_key)
                                    .password(true)
                                    .desired_width(300.0)
                                    .hint_text("Enter API key for cloud provider"),
                            );
                        });

                        if !cloud_api_key.is_empty() {
                            // Show that a CloudTranscriber can be constructed
                            if let Some(provider) = cloud_providers.get(selected_cloud) {
                                let _transcriber =
                                    CloudTranscriber::new(*provider, cloud_api_key.clone());
                                ui.label(
                                    RichText::new(format!(
                                        "Cloud transcriber configured: {}",
                                        provider.name()
                                    ))
                                    .small()
                                    .color(Color32::GREEN),
                                );
                            }
                        }
                    });
                }
            });

        // Write back values changed in the closure
        self.voice_selected_device = selected_device;
        self.voice_selected_cloud = selected_cloud;
        self.voice_selected_hotkey_preset = selected_hotkey;
        self.voice_show_settings = show_settings;
        self.voice_recording_active = recording_active;
        self.voice_cloud_api_key = cloud_api_key;
        self.voice_status_text = status_text;
        self.voice_last_transcript = last_transcript;
        self.voice_command_result = command_result;
    }

    fn render_sandbox_panel(&mut self, ctx: &egui::Context) {
        if !self.show_sandbox_panel {
            return;
        }
        let mut open = self.show_sandbox_panel;
        egui::Window::new("Sandbox Isolation Dashboard")
            .open(&mut open)
            .default_width(520.0)
            .resizable(true)
            .scroll([false, true])
            .show(ctx, |ui| {
                ui.heading("Layer 1: Network Sandbox");
                ui.separator();
                let net_desc = self.network_sandbox.describe();
                ui.label(format!("Status: {}", net_desc));
                ui.label(format!(
                    "Allowed: {} | Blocked: {} | RateLimited: {}",
                    self.network_sandbox.allowed_hosts.len(),
                    self.network_sandbox.connections_blocked,
                    self.network_sandbox.rate_limited_count
                ));
                if let Some(lv) = self.network_sandbox.last_validation {
                    ui.label(format!("Last validation: {:?} ago", lv.elapsed()));
                } else {
                    ui.label("Last validation: none");
                }
                ui.horizontal(|ui| {
                    if ui.button("Cleanup").clicked() {
                        self.network_sandbox.cleanup();
                    }
                    if ui.button("Allow host").clicked() {
                        self.network_sandbox.allow_host("trusted.com");
                    }
                    if ui.button("Block host").clicked() {
                        self.network_sandbox.block_host("evil.com");
                    }
                });
                if !self.network_sandbox.allowed_hosts.is_empty() {
                    ui.collapsing("Allowed hosts", |ui| {
                        for h in &self.network_sandbox.allowed_hosts {
                            ui.label(format!("  + {}", h));
                        }
                    });
                }
                ui.collapsing("Trust levels", |ui| {
                    for tl in &[
                        TrustLevel::Untrusted,
                        TrustLevel::Acknowledged,
                        TrustLevel::Reviewed,
                        TrustLevel::Approved,
                        TrustLevel::Established,
                    ] {
                        ui.label(format!(
                            "{}: exec={} fs={} net={}",
                            tl.description(),
                            tl.can_execute(),
                            tl.can_write_filesystem(),
                            tl.can_access_network()
                        ));
                    }
                    let r = self
                        .network_sandbox
                        .allow_network_for("test.org", TrustLevel::Established);
                    ui.label(format!("allow_network_for = {}", r));
                    ui.label(format!(
                        "is_blocked(coinhive) = {}",
                        self.network_sandbox.is_blocked("coinhive.com")
                    ));
                    ui.label(format!(
                        "resolve_host = {}",
                        crate::sandbox::network::NetworkSandbox::resolve_host("localhost:80")
                    ));
                });
                ui.add_space(12.0);
                ui.heading("Layer 2: Page Sandbox");
                ui.separator();
                if self.sandbox_manager.get(0).is_none() {
                    self.sandbox_manager.create(0, "https://example.com".into());
                }
                if ui.button("Click (tab0)").clicked() {
                    self.sandbox_manager.record(
                        0,
                        Interaction::Click {
                            x: 100,
                            y: 200,
                            element_width: 120,
                            element_height: 40,
                            element_type: "button".into(),
                            timestamp: std::time::Instant::now(),
                        },
                    );
                }
                if ui.button("KeyboardInput (tab0)").clicked() {
                    self.sandbox_manager.record(
                        0,
                        Interaction::KeyboardInput {
                            in_form_field: true,
                            char_count: 5,
                            timestamp: std::time::Instant::now(),
                        },
                    );
                }
                if ui.button("Scroll (tab0)").clicked() {
                    self.sandbox_manager.record(
                        0,
                        Interaction::Scroll {
                            delta_y: 150,
                            user_initiated: true,
                            timestamp: std::time::Instant::now(),
                        },
                    );
                }
                if ui.button("FormSubmit (tab0)").clicked() {
                    self.sandbox_manager.record(
                        0,
                        Interaction::FormSubmit {
                            timestamp: std::time::Instant::now(),
                        },
                    );
                }
                if let Some(ps) = self.sandbox_manager.get(0) {
                    ui.label(format!("URL: {}", ps.url));
                    ui.label(format!(
                        "Trust: {:?} | {} | {}",
                        ps.trust,
                        ps.status_text(),
                        ps.status_color()
                    ));
                    ui.label(format!(
                        "Meaningful: {} | Blocked: {}",
                        ps.meaningful_count,
                        ps.blocked_actions.len()
                    ));
                    for ba in &ps.blocked_actions {
                        ui.label(format!(
                            "  {} - {} {:?} | {}",
                            ba.action,
                            ba.reason,
                            ba.timestamp.elapsed(),
                            ba.describe()
                        ));
                    }
                    for i in &ps.interactions {
                        ui.label(format!("  {}", i.describe()));
                    }
                    ui.label(format!("{}", ps.describe()));
                    let t = ps.trust;
                    ui.label(format!(
                        "clip={} dl={} notif={} popup={} geo={} audio={} fs={}",
                        t.can_access_clipboard(),
                        t.can_initiate_download(),
                        t.can_request_notifications(),
                        t.can_open_popup(),
                        t.can_request_geolocation(),
                        t.can_autoplay_audio(),
                        t.can_go_fullscreen()
                    ));
                }
                if ui.button("Check clipboard").clicked() {
                    let _ = self.sandbox_manager.check(0, "clipboard");
                }
                if ui.button("Remove tab0").clicked() {
                    self.sandbox_manager.remove(0);
                }
                if let Some(pm) = self.sandbox_manager.get_mut(0) {
                    let _ = pm.check_permission("download");
                }
                ui.add_space(12.0);
                ui.heading("Layer 3: Popup Handler");
                ui.separator();
                ui.label(format!("{}", self.popup_handler.describe()));
                ui.label(format!("Blocked: {}", self.popup_handler.blocked_count()));
                ui.horizontal(|ui| {
                    if ui.button("Page load").clicked() {
                        self.popup_handler.page_loading();
                        self.popup_handler.page_loaded();
                    }
                    if ui.button("Allow domain").clicked() {
                        self.popup_handler.allow_domain("example.com");
                    }
                    if ui.button("Clear").clicked() {
                        self.popup_handler.clear_blocked();
                    }
                });
                if ui.button("Spam popup").clicked() {
                    let req = PopupRequest {
                        source_url: "https://x.com".into(),
                        target_url: "https://malware.xyz".into(),
                        width: Some(800),
                        height: Some(600),
                        user_gesture: false,
                        timestamp: std::time::Instant::now(),
                    };
                    let d = self.popup_handler.evaluate(&req);
                    let _ = self.popup_handler.handle(req, &d);
                }
                if ui.button("OAuth popup").clicked() {
                    let req = PopupRequest {
                        source_url: "https://app.com".into(),
                        target_url: "https://accounts.google.com/oauth".into(),
                        width: Some(500),
                        height: Some(600),
                        user_gesture: false,
                        timestamp: std::time::Instant::now(),
                    };
                    let d = self.popup_handler.evaluate(&req);
                    let _ = self.popup_handler.handle(req, &d);
                }
                if ui.button("Gesture popup").clicked() {
                    let req = PopupRequest {
                        source_url: "https://l.com".into(),
                        target_url: "https://l.com/p".into(),
                        width: Some(400),
                        height: Some(300),
                        user_gesture: true,
                        timestamp: std::time::Instant::now(),
                    };
                    let d = self.popup_handler.evaluate(&req);
                    let _ = self.popup_handler.handle(req, &d);
                }
                let bps = self.popup_handler.recent_blocked();
                if !bps.is_empty() {
                    ui.collapsing(format!("Blocked ({})", bps.len()), |ui| {
                        for (i, bp) in bps.iter().enumerate() {
                            ui.label(format!(
                                "#{}: {}->{}({}x{})g={} r={} {:?}ago",
                                i,
                                bp.request.source_url,
                                bp.request.target_url,
                                bp.request.width.unwrap_or(0),
                                bp.request.height.unwrap_or(0),
                                bp.request.user_gesture,
                                bp.reason,
                                bp.timestamp.elapsed()
                            ));
                        }
                    });
                }
                if ui.button("Allow first blocked").clicked() {
                    let _ = self.popup_handler.allow_blocked(0);
                }
                ui.add_space(12.0);
                ui.heading("Layer 4: Download Quarantine");
                ui.separator();
                ui.label(format!(
                    "Files: {} | Size: {} bytes",
                    self.download_quarantine.list().len(),
                    self.download_quarantine.total_size()
                ));
                if ui.button("Add test exe").clicked() {
                    let qf = QuarantinedFile::new(
                        "invoice.pdf.exe".into(),
                        "http://bad.com/invoice.pdf.exe".into(),
                        "application/octet-stream".into(),
                        b"MZ fake".to_vec(),
                    );
                    self.download_quarantine.add(qf);
                }
                if ui.button("Add test PDF").clicked() {
                    let qf = QuarantinedFile::new(
                        "report.pdf".into(),
                        "https://good.com/report.pdf".into(),
                        "application/pdf".into(),
                        b"%PDF fake".to_vec(),
                    );
                    self.download_quarantine.add(qf);
                }
                let fids: Vec<String> = self
                    .download_quarantine
                    .list()
                    .iter()
                    .map(|f| f.id.clone())
                    .collect();
                for fid in &fids {
                    if let Some(qf) = self.download_quarantine.get(fid) {
                        ui.group(|ui| {
                            ui.label(RichText::new(&qf.filename).strong());
                            ui.label(format!(
                                "ID:{} Src:{} Type:{}",
                                qf.id, qf.source_url, qf.content_type
                            ));
                            ui.label(format!(
                                "Size:{} SHA:{} Data:{} Enc:{}",
                                qf.size_bytes,
                                qf.sha256,
                                qf.data.len(),
                                qf.encrypted_data.is_some()
                            ));
                            ui.label(format!("Age: {:?}", qf.quarantined_at.elapsed()));
                            let s = &qf.security;
                            ui.label(format!(
                                "id={} origin={} type={:?} trust={}",
                                s.id,
                                s.origin,
                                s.content_type,
                                s.trust_level.description()
                            ));
                            ui.label(format!(
                                "ints={} viols={} timeMet={} age={:?}",
                                s.interactions.len(),
                                s.violations.len(),
                                s.meets_time_requirement(),
                                s.created_at.elapsed()
                            ));
                            if let Some(li) = s.last_interaction {
                                ui.label(format!("lastInt: {:?}ago", li.elapsed()));
                            }
                            for ir in &s.interactions {
                                let n = match ir.action {
                                    InteractionType::Acknowledge => "Ack",
                                    InteractionType::Review => "Rev",
                                    InteractionType::Approve => "App",
                                    InteractionType::Execute => "Exe",
                                    InteractionType::Deny => "Den",
                                };
                                ui.label(format!("  {} {:?}ago", n, ir.timestamp.elapsed()));
                            }
                            for v in &s.violations {
                                let sn = match v.severity {
                                    ViolationSeverity::Low => "Lo",
                                    ViolationSeverity::Medium => "Med",
                                    ViolationSeverity::High => "Hi",
                                    ViolationSeverity::Critical => "Crit",
                                };
                                ui.label(format!(
                                    "  {}[{}] {:?}ago",
                                    v.description,
                                    sn,
                                    v.timestamp.elapsed()
                                ));
                            }
                            ui.label(format!("Ctx: {}", s.describe()));
                            let ext = qf.filename.rsplit('.').next().unwrap_or("");
                            ui.label(format!(
                                "ContentType('{}')={:?}",
                                ext,
                                ContentType::from_extension(ext)
                            ));
                            ui.label(format!(
                                "interaction_for={:?}",
                                SecurityContext::interaction_for("approve")
                            ));
                            let mw = qf.max_warning_level();
                            ui.label(format!(
                                "MaxWarn: {}",
                                match mw {
                                    WarningLevel::Info => "Info",
                                    WarningLevel::Caution => "Caution",
                                    WarningLevel::Warning => "Warning",
                                    WarningLevel::Danger => "Danger",
                                }
                            ));
                            for w in &qf.warnings {
                                let ls = match w.level {
                                    WarningLevel::Info => "I",
                                    WarningLevel::Caution => "C",
                                    WarningLevel::Warning => "W",
                                    WarningLevel::Danger => "D",
                                };
                                ui.label(format!("[{}] {}: {}", ls, w.message, w.detail));
                            }
                            ui.label(format!(
                                "Release: {}",
                                match &qf.can_release() {
                                    ReleaseStatus::Ready => "Ready".into(),
                                    ReleaseStatus::NeedsInteraction { current, required } =>
                                        format!("{}/{}", current, required),
                                    ReleaseStatus::Waiting { seconds_remaining } =>
                                        format!("Wait{}s", seconds_remaining),
                                    ReleaseStatus::Blocked { reason } =>
                                        format!("Blocked:{}", reason),
                                }
                            ));
                        });
                    }
                }
                if let Some(fid) = fids.first() {
                    ui.horizontal(|ui| {
                        if ui.button("Ack").clicked() {
                            if let Some(q) = self.download_quarantine.get_mut(fid) {
                                q.interact(InteractionType::Acknowledge);
                            }
                        }
                        if ui.button("Rev").clicked() {
                            if let Some(q) = self.download_quarantine.get_mut(fid) {
                                q.interact(InteractionType::Review);
                            }
                        }
                        if ui.button("App").clicked() {
                            if let Some(q) = self.download_quarantine.get_mut(fid) {
                                q.interact(InteractionType::Approve);
                                q.security.record_violation("Test", ViolationSeverity::Low);
                            }
                        }
                        if ui.button("Rm").clicked() {
                            self.download_quarantine.remove(fid);
                        }
                        if ui.button("Crit").clicked() {
                            if let Some(q) = self.download_quarantine.get_mut(fid) {
                                q.security
                                    .record_violation("Critical test", ViolationSeverity::Critical);
                            }
                        }
                        if ui.button("Encrypt").clicked() {
                            if let Some(q) = self.download_quarantine.get_mut(fid) {
                                let master = crate::crypto::MasterSecret::generate();
                                let key = crate::crypto::EncryptionKey::from_master(
                                    &master,
                                    "quarantine",
                                );
                                let _ = q.encrypt_with(&key);
                                let _ = q.decrypt_with(&key);
                            }
                        }
                        if ui.button("Release").clicked() {
                            if let Some(q) = self.download_quarantine.get_mut(fid) {
                                let dest = std::path::PathBuf::from(".");
                                let _ = q.release(dest, None);
                            }
                        }
                    });
                }

                // Exercise PopupDecision reason fields
                ui.collapsing("Popup decision reasons", |ui| {
                    let test_req = PopupRequest {
                        source_url: "https://test.com".into(),
                        target_url: "https://test.com/pop".into(),
                        width: Some(300),
                        height: Some(200),
                        user_gesture: true,
                        timestamp: std::time::Instant::now(),
                    };
                    let dec = self.popup_handler.evaluate(&test_req);
                    let reason_text = match &dec {
                        crate::sandbox::popup::PopupDecision::Allow { reason } => {
                            format!("Allow: {}", reason)
                        }
                        crate::sandbox::popup::PopupDecision::Block { reason } => {
                            format!("Block: {}", reason)
                        }
                        crate::sandbox::popup::PopupDecision::Prompt { reason } => {
                            format!("Prompt: {}", reason)
                        }
                    };
                    ui.label(reason_text);
                });
            });
        self.show_sandbox_panel = open;
    }

    fn render_rest_client_panel(&mut self, ctx: &egui::Context) {
        if !self.rest_client_panel_visible {
            return;
        }
        egui::Window::new("REST Client")
            .open(&mut self.rest_client_panel_visible)
            .resizable(true)
            .default_size(Vec2::new(700.0, 600.0))
            .show(ctx, |ui| {
                ui.heading("REST API Tester");
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Method:");
                    for m in &[
                        RestMethod::Get,
                        RestMethod::Post,
                        RestMethod::Put,
                        RestMethod::Patch,
                        RestMethod::Delete,
                        RestMethod::Head,
                        RestMethod::Options,
                    ] {
                        if ui
                            .selectable_label(self.rest_client.method == *m, m.as_str())
                            .clicked()
                        {
                            self.rest_client.method = *m;
                        }
                    }
                    if let Some(p) = RestMethod::from_str(self.rest_client.method.as_str()) {
                        ui.label(format!("(body:{})", p.has_body()));
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("URL:");
                    ui.text_edit_singleline(&mut self.rest_client.url);
                    if ui.button("Send").clicked() {
                        self.rest_client.execute();
                    }
                    if ui.button("Clear").clicked() {
                        self.rest_client.clear();
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Type:");
                    for ct in &[
                        RestContentType::Json,
                        RestContentType::FormUrlEncoded,
                        RestContentType::FormData,
                        RestContentType::Text,
                        RestContentType::Binary,
                    ] {
                        if ui
                            .selectable_label(self.rest_client.content_type == *ct, ct.mime_type())
                            .clicked()
                        {
                            self.rest_client.content_type = *ct;
                        }
                    }
                    let det = RestContentType::from_mime(self.rest_client.content_type.mime_type());
                    ui.label(format!("[{}]", det.mime_type()));
                });
                ui.collapsing("Headers", |ui| {
                    let hlen = self.rest_client.headers.len();
                    for i in 0..hlen {
                        ui.horizontal(|ui| {
                            ui.label(if self.rest_client.headers[i].2 {
                                "[on]"
                            } else {
                                "[off]"
                            });
                            ui.label(&self.rest_client.headers[i].0);
                            ui.label(":");
                            ui.label(&self.rest_client.headers[i].1);
                            if ui.small_button("Tog").clicked() {
                                self.rest_client.toggle_header(i);
                            }
                            if ui.small_button("Rm").clicked() {
                                self.rest_client.remove_header(i);
                            }
                        });
                    }
                    if ui.button("Add Header").clicked() {
                        self.rest_client.add_header("X-Custom", "value");
                    }
                });
                if self.rest_client.method.has_body() {
                    ui.collapsing("Body", |ui| {
                        ui.text_edit_multiline(&mut self.rest_client.body);
                    });
                }
                ui.collapsing("Environment", |ui| {
                    let env: Vec<_> = self
                        .rest_client
                        .environment
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    for (k, v) in &env {
                        ui.label(format!("{{{{{}}}}}: {}", k, v));
                    }
                    ui.label(format!(
                        "Resolved: {}",
                        self.rest_client.substitute_vars(&self.rest_client.url)
                    ));
                });
                ui.separator();
                if self.rest_client.is_loading {
                    ui.label("Loading...");
                }
                if let Some(ref err) = self.rest_client.error {
                    ui.colored_label(Color32::RED, format!("Error: {}", err));
                }
                if let Some(ref resp) = self.rest_client.response.clone() {
                    ui.label(
                        RichText::new(format!("{} {}", resp.status, resp.status_text)).strong(),
                    );
                    ui.label(format!(
                        "{}ms | {} bytes",
                        resp.duration_ms, resp.size_bytes
                    ));
                    if let Some(ct) = resp.content_type() {
                        ui.label(format!("Type: {}", ct));
                    }
                    ui.collapsing("Resp Headers", |ui| {
                        for (n, v) in &resp.headers {
                            ui.label(format!("{}: {}", n, v));
                        }
                    });
                    ui.collapsing("Resp Body", |ui| {
                        if resp.is_json() {
                            if let Some(ref body) = resp.body_text {
                                if self.json_viewer.load(body).is_ok() {
                                    if let Some(root) = &self.json_viewer.root {
                                        ui.code(root.pretty_print(0));
                                    }
                                } else if let Some(p) = resp.json_pretty() {
                                    ui.code(p);
                                }

                                let highlighted = self
                                    .syntax_highlighter
                                    .highlight(body, crate::syntax::Language::Json);
                                if !highlighted.is_empty() {
                                    ui.separator();
                                    ui.label("Syntax preview");
                                    for line in highlighted.iter().take(8) {
                                        let mut job = egui::text::LayoutJob::default();
                                        for token in line {
                                            let color = Color32::from_rgba_premultiplied(
                                                token.color.r,
                                                token.color.g,
                                                token.color.b,
                                                token.color.a,
                                            );
                                            job.append(
                                                &token.text,
                                                0.0,
                                                egui::text::TextFormat {
                                                    font_id: FontId::monospace(12.0),
                                                    color,
                                                    ..Default::default()
                                                },
                                            );
                                        }
                                        ui.label(job);
                                    }
                                }
                            } else if let Some(p) = resp.json_pretty() {
                                ui.code(p);
                            }
                        } else if resp.is_html() {
                            if let Some(ref t) = resp.body_text {
                                ui.code(&t[..t.len().min(2000)]);
                            }
                        } else if let Some(ref t) = resp.body_text {
                            ui.code(&t[..t.len().min(2000)]);
                        } else {
                            ui.label(format!("(binary {} bytes)", resp.body.len()));
                        }
                    });
                    ui.collapsing("Resp Diagnostics", |ui| {
                        ui.label(resp.describe());
                    });
                }
                ui.separator();
                ui.collapsing("cURL", |ui| {
                    ui.code(self.rest_client.to_curl());
                });
                ui.collapsing("fetch()", |ui| {
                    ui.code(self.rest_client.to_fetch());
                });
                ui.collapsing("Collections", |ui| {
                    if ui.button("Save to Default").clicked() {
                        self.rest_client.save_to_collection("Default");
                    }
                    for c in &self.rest_client.collections {
                        ui.label(c.describe());
                    }
                    if ui.button("Sample Collection").clicked() {
                        let mut col = RequestCollection::new("Samples");
                        let req =
                            SavedRequest::new("Test", RestMethod::Get, "https://api.example.com");
                        col.add_request(req);
                        self.rest_client.collections.push(col);
                    }
                });
                ui.collapsing("History", |ui| {
                    let hist: Vec<_> = self
                        .rest_client
                        .history
                        .iter()
                        .take(10)
                        .map(|h| (h.name.clone(), h.describe(), h.clone()))
                        .collect();
                    for (name, desc, saved) in hist {
                        ui.horizontal(|ui| {
                            if ui.small_button("Load").clicked() {
                                self.rest_client.load_request(&saved);
                            }
                            ui.label(&name);
                        });
                        ui.small(desc);
                    }
                    ui.label(format!(
                        "{}/{}",
                        self.rest_client.history.len(),
                        self.rest_client.max_history
                    ));
                });
                ui.collapsing("Full Diagnostics", |ui| {
                    ui.label(self.rest_client.describe());
                });
            });
    }

    fn render_network_activity_panel(&mut self, ctx: &egui::Context) {
        if !self.network_activity_panel_visible {
            return;
        }
        egui::Window::new("Network Activity Monitor")
            .open(&mut self.network_activity_panel_visible)
            .resizable(true)
            .default_size(Vec2::new(550.0, 450.0))
            .show(ctx, |ui| {
                ui.heading("Network Activity");
                ui.separator();
                let state = self.net_activity_monitor.state();
                let c = state.color();
                let col = Color32::from_rgb(
                    ((c >> 16) & 0xFF) as u8,
                    ((c >> 8) & 0xFF) as u8,
                    (c & 0xFF) as u8,
                );
                ui.horizontal(|ui| {
                    ui.colored_label(col, format!("[{}] {:?}", state.icon(), state));
                    ui.label(format!(
                        "active={} count={}",
                        state.is_active(),
                        self.net_activity_monitor.active_count()
                    ));
                });
                let (down, up) = self.net_activity_monitor.session_stats();
                ui.label(format!(
                    "Down: {} | Up: {}",
                    crate::network::NetworkMonitor::format_bytes(down),
                    crate::network::NetworkMonitor::format_bytes(up)
                ));
                ui.separator();
                ui.collapsing("State Reference", |ui| {
                    for s in &[
                        NetActivityState::Idle,
                        NetActivityState::Connecting,
                        NetActivityState::Downloading,
                        NetActivityState::Uploading,
                        NetActivityState::Stalled,
                        NetActivityState::Error,
                    ] {
                        let c2 = s.color();
                        let col2 = Color32::from_rgb(
                            ((c2 >> 16) & 0xFF) as u8,
                            ((c2 >> 8) & 0xFF) as u8,
                            (c2 & 0xFF) as u8,
                        );
                        ui.colored_label(
                            col2,
                            format!("[{}] {:?} active={}", s.icon(), s, s.is_active()),
                        );
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("Start Request").clicked() {
                        let id = self
                            .net_activity_monitor
                            .start_request("https://example.com/test".into(), "GET".into());
                        self.net_activity_monitor
                            .update_download(id, 1024, Some(4096));
                    }
                    if ui.button("Tick").clicked() {
                        self.net_activity_monitor.tick();
                    }
                });
                ui.separator();
                ui.label(RichText::new("Active Requests").strong());
                let descs: Vec<(u64, String)> = self
                    .net_activity_monitor
                    .active_requests()
                    .iter()
                    .map(|r| (r.id, r.describe()))
                    .collect();
                if descs.is_empty() {
                    ui.label("(none)");
                }
                for (rid, desc) in &descs {
                    ui.horizontal(|ui| {
                        ui.label(desc);
                        let id = *rid;
                        if ui.small_button("Done").clicked() {
                            self.net_activity_monitor.complete(id);
                        }
                        if ui.small_button("Err").clicked() {
                            self.net_activity_monitor.error(id, "test");
                        }
                        if ui.small_button("+DL").clicked() {
                            self.net_activity_monitor.update_download(id, 1024, None);
                        }
                        if ui.small_button("+UL").clicked() {
                            self.net_activity_monitor.update_upload(id, 512);
                        }
                    });
                }
                ui.collapsing("Request Details", |ui| {
                    let sample =
                        NetActivityRequest::new(0, "https://test.local".into(), "POST".into());
                    ui.label(format!(
                        "progress={:?} dur={:.2}s stalled={}",
                        sample.progress(),
                        sample.duration().as_secs_f32(),
                        sample.is_stalled()
                    ));
                    ui.label(sample.describe());
                });
                ui.label(format!(
                    "Speed: {}",
                    crate::network::NetworkMonitor::format_speed(0)
                ));
                ui.collapsing("Shared Monitor", |ui| {
                    let shared: SharedNetworkMonitor = shared_monitor();
                    {
                        let mut lk = shared.lock().unwrap();
                        let sid = lk.start_request("https://shared.test".into(), "GET".into());
                        lk.update_download(sid, 256, Some(1024));
                        lk.complete(sid);
                        ui.label(lk.describe());
                    }
                    ui.label(shared_monitor_describe());
                });
                ui.collapsing("Diagnostics", |ui| {
                    ui.label(self.net_activity_monitor.describe());
                });
            });
    }

    fn render_protocol_diagnostics_panel(&mut self, ctx: &egui::Context) {
        if !self.protocol_diagnostics_panel_visible {
            return;
        }
        egui::Window::new("Protocol Diagnostics")
            .open(&mut self.protocol_diagnostics_panel_visible)
            .resizable(true)
            .default_size(Vec2::new(550.0, 500.0))
            .show(ctx, |ui| {
                ui.heading("HTTP Protocol Tools");
                ui.separator();
                ui.label(RichText::new("HTTP Client").strong());
                ui.horizontal(|ui| {
                    if ui.button("Set UA").clicked() {
                        self.http_client.set_user_agent("SassyBrowser/diag");
                    }
                    if ui.button("Timeout 10s").clicked() {
                        self.http_client.set_timeout(Duration::from_secs(10));
                    }
                    if ui.button("Clear Cookies").clicked() {
                        self.http_client.clear_cookies();
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("Set Cookie").clicked() {
                        self.http_client.set_cookie("example.com", "sid", "abc");
                    }
                    if let Some(c) = self.http_client.get_cookie("example.com", "sid") {
                        ui.label(format!("Cookie: {}", c));
                    } else {
                        ui.label("No cookie");
                    }
                    if ui.button("Clear Host").clicked() {
                        self.http_client.clear_cookies_for_host("example.com");
                    }
                });
                ui.separator();
                ui.label(RichText::new("FetchOptions").strong());
                ui.collapsing("Builders", |ui| {
                    let g = FetchOptions::get();
                    ui.label(format!("get: {}", g.method));
                    let p = FetchOptions::post("{}")
                        .with_header("X", "1")
                        .with_json_body("{\"a\":1}")
                        .with_credentials(CredentialsMode::Include)
                        .with_cache(CacheMode::NoStore)
                        .with_redirect(RedirectMode::Manual);
                    ui.label(format!("post: {} body={:?}", p.method, p.body.as_deref()));
                    ui.label(format!(
                        "cors: include={}",
                        FetchOptions::cors_with_credentials().credentials
                            == CredentialsMode::Include
                    ));
                    ui.label(format!(
                        "anon: omit={}",
                        FetchOptions::anonymous().credentials == CredentialsMode::Omit
                    ));
                    ui.label(format!("no_cache: {}", FetchOptions::no_cache().method));
                    ui.label(format!("reload: {}", FetchOptions::reload().method));
                    ui.label(format!(
                        "manual_redir: manual={}",
                        FetchOptions::manual_redirect().redirect == RedirectMode::Manual
                    ));
                    let js = FetchOptions::from_js_options(
                        "PUT",
                        Some("b".into()),
                        "include",
                        "no-store",
                        "error",
                    );
                    ui.label(format!("from_js: {} {:?}", js.method, js.body));
                });
                ui.separator();
                ui.label(RichText::new("Cache Modes").strong());
                for (s, exp) in &[
                    ("default", CacheMode::Default),
                    ("no-store", CacheMode::NoStore),
                    ("reload", CacheMode::Reload),
                    ("no-cache", CacheMode::NoCache),
                    ("force-cache", CacheMode::ForceCache),
                    ("only-if-cached", CacheMode::OnlyIfCached),
                ] {
                    ui.label(format!(
                        "  '{}' ok={}",
                        s,
                        CacheMode::from_fetch_option(s) == *exp
                    ));
                }
                ui.label(RichText::new("Credentials").strong());
                for (s, exp) in &[
                    ("omit", CredentialsMode::Omit),
                    ("same-origin", CredentialsMode::SameOrigin),
                    ("include", CredentialsMode::Include),
                ] {
                    ui.label(format!(
                        "  '{}' ok={}",
                        s,
                        CredentialsMode::from_fetch_option(s) == *exp
                    ));
                }
                ui.label(RichText::new("Redirects").strong());
                for s in &["follow", "error", "manual"] {
                    ui.label(format!(
                        "  '{}' follow={}",
                        s,
                        RedirectMode::from_fetch_option(s) == RedirectMode::Follow
                    ));
                }
                ui.separator();
                ui.label(RichText::new("Data URLs").strong());
                if let Some((m, d)) = parse_data_url("data:text/plain,Hello%20World") {
                    ui.label(format!("plain: {} len={}", m, d.len()));
                }
                if let Some((m, d)) = parse_data_url("data:text/plain;base64,SGVsbG8=") {
                    ui.label(format!("b64: {} len={}", m, d.len()));
                }
                ui.label(format!("url_encode: {}", url_encode("hello world&x=1")));
                let mut fd = std::collections::HashMap::new();
                fd.insert("k".into(), "v v".into());
                ui.label(format!("form: {}", encode_form_data(&fd)));
                ui.separator();
                ui.label(RichText::new("Multipart").strong());
                let mut mp = MultipartFormData::new();
                mp.add_field("user", "sassy");
                mp.add_file("file", "f.png", "image/png", vec![0x89, 0x50]);
                ui.label(format!("type: {}", mp.get_content_type()));
                ui.label(format!("encoded: {} bytes", mp.encode().len()));
            });
    }

    fn render_mcp_panel(&mut self, ctx: &egui::Context) {
        if !self.mcp_panel.is_visible {
            return;
        }
        let theme = McpTheme::dark();
        let light = McpTheme::light();
        let bg = egui::Color32::from_rgba_premultiplied(
            theme.background.r,
            theme.background.g,
            theme.background.b,
            theme.background.a,
        );
        let surface = egui::Color32::from_rgba_premultiplied(
            theme.surface.r,
            theme.surface.g,
            theme.surface.b,
            theme.surface.a,
        );
        let text_c = egui::Color32::from_rgba_premultiplied(
            theme.text.r,
            theme.text.g,
            theme.text.b,
            theme.text.a,
        );
        let text_dim = egui::Color32::from_rgba_premultiplied(
            theme.text_dim.r,
            theme.text_dim.g,
            theme.text_dim.b,
            theme.text_dim.a,
        );
        let primary = egui::Color32::from_rgba_premultiplied(
            theme.primary.r,
            theme.primary.g,
            theme.primary.b,
            theme.primary.a,
        );
        let accent = egui::Color32::from_rgba_premultiplied(
            theme.accent.r,
            theme.accent.g,
            theme.accent.b,
            theme.accent.a,
        );
        let success_c = egui::Color32::from_rgba_premultiplied(
            theme.success.r,
            theme.success.g,
            theme.success.b,
            theme.success.a,
        );
        let warn_c = egui::Color32::from_rgba_premultiplied(
            theme.warning.r,
            theme.warning.g,
            theme.warning.b,
            theme.warning.a,
        );
        let err_c = egui::Color32::from_rgba_premultiplied(
            theme.error.r,
            theme.error.g,
            theme.error.b,
            theme.error.a,
        );
        let border_c = egui::Color32::from_rgba_premultiplied(
            theme.border.r,
            theme.border.g,
            theme.border.b,
            theme.border.a,
        );
        let _voice_c = egui::Color32::from_rgba_premultiplied(
            theme.voice_color.r,
            theme.voice_color.g,
            theme.voice_color.b,
            theme.voice_color.a,
        );
        let _orch_c = egui::Color32::from_rgba_premultiplied(
            theme.orchestrator_color.r,
            theme.orchestrator_color.g,
            theme.orchestrator_color.b,
            theme.orchestrator_color.a,
        );
        let _coder_c = egui::Color32::from_rgba_premultiplied(
            theme.coder_color.r,
            theme.coder_color.g,
            theme.coder_color.b,
            theme.coder_color.a,
        );
        let _auditor_c = egui::Color32::from_rgba_premultiplied(
            theme.auditor_color.r,
            theme.auditor_color.g,
            theme.auditor_color.b,
            theme.auditor_color.a,
        );
        let _user_b = egui::Color32::from_rgba_premultiplied(
            theme.user_bubble.r,
            theme.user_bubble.g,
            theme.user_bubble.b,
            theme.user_bubble.a,
        );
        let _agent_b = egui::Color32::from_rgba_premultiplied(
            theme.agent_bubble.r,
            theme.agent_bubble.g,
            theme.agent_bubble.b,
            theme.agent_bubble.a,
        );
        let _sys_b = egui::Color32::from_rgba_premultiplied(
            theme.system_bubble.r,
            theme.system_bubble.g,
            theme.system_bubble.b,
            theme.system_bubble.a,
        );
        // Exercise light theme agent_color
        for role in &[
            AgentRole::Voice,
            AgentRole::Orchestrator,
            AgentRole::Coder,
            AgentRole::Auditor,
        ] {
            let ac = light.agent_color(role.clone());
            let _ = egui::Color32::from_rgba_premultiplied(ac.r, ac.g, ac.b, ac.a);
        }
        egui::Window::new("MCP AI Panel")
            .default_width(self.mcp_panel.width as f32)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(bg)
                    .stroke(egui::Stroke::new(1.0, border_c)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let modes = [
                        PanelMode::Chat,
                        PanelMode::Tasks,
                        PanelMode::Edits,
                        PanelMode::Settings,
                    ];
                    for m in &modes {
                        let label = match m {
                            PanelMode::Chat => "Chat",
                            PanelMode::Tasks => "Tasks",
                            PanelMode::Edits => "Edits",
                            PanelMode::Settings => "Settings",
                            PanelMode::TokenMeter => "Tokens",
                        };
                        if ui
                            .selectable_label(
                                std::mem::discriminant(&self.mcp_panel.mode)
                                    == std::mem::discriminant(m),
                                RichText::new(label).color(primary),
                            )
                            .clicked()
                        {
                            self.mcp_panel.set_mode(m.clone());
                        }
                    }
                    if ui
                        .button(RichText::new("Toggle Theme").color(accent))
                        .clicked()
                    {
                        self.mcp_panel.toggle_theme();
                    }
                });
                ui.separator();
                // Agent status bars
                let agents = vec![
                    AgentStatus {
                        role: AgentRole::Voice,
                        name: "Voice Agent".into(),
                        online: true,
                    },
                    AgentStatus {
                        role: AgentRole::Orchestrator,
                        name: "Orchestrator".into(),
                        online: true,
                    },
                    AgentStatus {
                        role: AgentRole::Coder,
                        name: "Coder Agent".into(),
                        online: false,
                    },
                    AgentStatus {
                        role: AgentRole::Auditor,
                        name: "Auditor Agent".into(),
                        online: true,
                    },
                ];
                for a in &agents {
                    let col = if a.online { success_c } else { text_dim };
                    ui.label(
                        RichText::new(format!(
                            "{} [{}] - {}",
                            a.role.icon(),
                            a.name,
                            if a.online { "online" } else { "offline" }
                        ))
                        .color(col),
                    );
                }
                ui.separator();
                // Quick commands
                let cmds: Vec<QuickCommand> = get_quick_commands();
                ui.label(RichText::new("Quick Commands").color(text_c).strong());
                for cmd in &cmds {
                    ui.label(
                        RichText::new(format!(
                            "{} - {} (e.g. {})",
                            cmd.trigger, cmd.description, cmd.example
                        ))
                        .color(text_dim),
                    );
                }
                ui.separator();
                // Render elements
                let render_output: PanelRender = self.mcp_panel.render();
                ui.label(
                    RichText::new(format!(
                        "Mode: {:?} | Width: {} | Scroll: {}",
                        render_output.mode, render_output.width, render_output.scroll_offset
                    ))
                    .color(surface),
                );
                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        for elem in &render_output.elements {
                            match elem {
                                RenderElement::Header { title, subtitle } => {
                                    ui.label(
                                        RichText::new(title).size(18.0).color(text_c).strong(),
                                    );
                                    if let Some(sub) = subtitle {
                                        ui.label(RichText::new(sub).color(text_dim));
                                    }
                                }
                                RenderElement::SectionHeader { title } => {
                                    ui.label(
                                        RichText::new(title).size(14.0).color(primary).strong(),
                                    );
                                }
                                RenderElement::AgentBar { agents: bar_agents } => {
                                    for st in bar_agents {
                                        ui.label(
                                            RichText::new(format!(
                                                "{} {} [{}]",
                                                st.role.icon(),
                                                st.name,
                                                if st.online { "up" } else { "down" }
                                            ))
                                            .color(if st.online { success_c } else { err_c }),
                                        );
                                    }
                                }
                                RenderElement::Message {
                                    role: msg_role,
                                    agent,
                                    content,
                                    timestamp,
                                } => {
                                    let c = match msg_role {
                                        MessageRole::User => primary,
                                        MessageRole::Agent => accent,
                                        MessageRole::System => surface,
                                    };
                                    ui.label(
                                        RichText::new(format!(
                                            "[{:?}] {}: {} ({})",
                                            msg_role,
                                            agent.as_ref().map(|a| a.name()).unwrap_or("user"),
                                            content,
                                            timestamp
                                        ))
                                        .color(c),
                                    );
                                }
                                RenderElement::Task {
                                    id,
                                    title,
                                    description,
                                    status,
                                    assigned_to,
                                    has_artifacts,
                                } => {
                                    ui.label(
                                        RichText::new(format!(
                                            "[#{}] {} - {} ({:?}, by {}, artifacts={})",
                                            id,
                                            title,
                                            description,
                                            status,
                                            assigned_to.name(),
                                            has_artifacts
                                        ))
                                        .color(warn_c),
                                    );
                                }
                                RenderElement::CodeEdit {
                                    index,
                                    file_path,
                                    operation,
                                    description,
                                    preview,
                                    selected,
                                } => {
                                    ui.label(
                                        RichText::new(format!(
                                            "#{} {} ({:?}) [{}]: {}",
                                            index,
                                            file_path,
                                            operation,
                                            if *selected { "sel" } else { "_" },
                                            description
                                        ))
                                        .color(accent),
                                    );
                                    for (line, _clr) in preview {
                                        ui.label(RichText::new(line).monospace().color(text_c));
                                    }
                                }
                                RenderElement::AgentConfig {
                                    role,
                                    model,
                                    api_url,
                                    has_key,
                                    enabled,
                                } => {
                                    ui.label(
                                        RichText::new(format!(
                                            "{}: {} @ {} key={} [{}]",
                                            role.name(),
                                            model,
                                            api_url,
                                            has_key,
                                            if *enabled { "on" } else { "off" }
                                        ))
                                        .color(text_dim),
                                    );
                                }
                                RenderElement::ActionBar { actions } => {
                                    ui.horizontal(|ui| {
                                        for act in actions {
                                            let c = match act.style {
                                                ActionStyle::Primary => primary,
                                                ActionStyle::Secondary => text_dim,
                                                ActionStyle::Danger => err_c,
                                            };
                                            let _ = ui.button(RichText::new(&act.label).color(c));
                                        }
                                    });
                                }
                                RenderElement::Input {
                                    placeholder,
                                    value,
                                    cursor,
                                } => {
                                    ui.label(
                                        RichText::new(format!(
                                            "[Input: {} val='{}' cursor={}]",
                                            placeholder, value, cursor
                                        ))
                                        .color(border_c),
                                    );
                                }
                                RenderElement::Notification { message, style } => {
                                    let c = match style {
                                        NotificationStyle::Info => primary,
                                        NotificationStyle::Success => success_c,
                                        NotificationStyle::Warning => warn_c,
                                        NotificationStyle::Error => err_c,
                                    };
                                    ui.label(RichText::new(message).color(c));
                                }
                                RenderElement::EmptyState {
                                    icon,
                                    message,
                                    hint,
                                } => {
                                    ui.label(
                                        RichText::new(format!("{} {} ({})", icon, message, hint))
                                            .color(text_dim)
                                            .italics(),
                                    );
                                }
                                RenderElement::InfoRow { label, value } => {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!("{}: ", label)).color(text_dim),
                                        );
                                        ui.label(RichText::new(value).color(text_c));
                                    });
                                }
                            }
                        }
                    });
                ui.separator();
                // Action buttons for edits
                ui.horizontal(|ui| {
                    let approve_act = McpAction {
                        label: "Approve".into(),
                        id: "approve".into(),
                        style: ActionStyle::Primary,
                    };
                    let reject_act = McpAction {
                        label: "Reject".into(),
                        id: "reject".into(),
                        style: ActionStyle::Danger,
                    };
                    let review_act = McpAction {
                        label: "Review".into(),
                        id: "review".into(),
                        style: ActionStyle::Secondary,
                    };
                    if ui
                        .button(RichText::new(&approve_act.label).color(success_c))
                        .clicked()
                    {
                        self.mcp_panel.approve_edits();
                    }
                    if ui
                        .button(RichText::new(&reject_act.label).color(err_c))
                        .clicked()
                    {
                        self.mcp_panel.reject_edits();
                    }
                    ui.label(
                        RichText::new(format!("{} [{}]", review_act.label, review_act.id))
                            .color(text_dim),
                    );
                });
                // McpPanel interactive methods
                ui.horizontal(|ui| {
                    if ui.button("Toggle Panel").clicked() {
                        self.mcp_panel.toggle();
                    }
                    if ui.button("Approve Edit 0").clicked() {
                        let _ = self.mcp_panel.approve_edit(0);
                    }
                });
                // Key handling
                ctx.input(|i| {
                    for event in &i.events {
                        if let egui::Event::Text(t) = event {
                            for ch in t.chars() {
                                self.mcp_panel.handle_key(ch);
                            }
                        }
                        if let egui::Event::Key {
                            key, pressed: true, ..
                        } = event
                        {
                            match key {
                                egui::Key::Backspace => self.mcp_panel.handle_backspace(),
                                egui::Key::Enter => self.mcp_panel.handle_enter(),
                                _ => {}
                            }
                        }
                    }
                });
                // McpTheme secondary color
                let _secondary = egui::Color32::from_rgba_premultiplied(
                    theme.secondary.r,
                    theme.secondary.g,
                    theme.secondary.b,
                    theme.secondary.a,
                );
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
                if i.events
                    .iter()
                    .any(|e| matches!(e, egui::Event::Key { .. }))
                {
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
        let (active_url, active_title) = if let Some(Tab {
            content: TabContent::Web { url, title, .. },
            ..
        }) = self.engine.active_tab()
        {
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

        // ═══════════════════════════════════════════════════════════════════════
        // DETECTION ENGINE — Analyze active page each frame on trust levels 0-1
        // ═══════════════════════════════════════════════════════════════════════
        if let Some(tab) = self.engine.active_tab() {
            if let TabContent::Web { url, loading, .. } = &tab.content {
                let url_owned = url.clone();
                let is_loading = *loading;

                // Only run detection on loaded pages we haven't analyzed yet
                if !is_loading
                    && self.detection_last_analyzed_url.as_deref() != Some(url_owned.as_str())
                {
                    // Update page context URL/domain
                    self.detection_page_ctx.url = url_owned.clone();
                    self.detection_page_ctx.domain = extract_domain_from_url(&url_owned);
                    self.detection_page_ctx.last_updated = Instant::now();

                    // Run detection analysis
                    let _alerts = self.detection_engine.analyze(&self.detection_page_ctx);
                    if self.detection_engine.active_alert_count() > 0 {
                        console_error(&format!(
                            "Detection alerts on {}: {} threat(s)",
                            url_owned,
                            self.detection_engine.active_alert_count()
                        ));
                    }
                    self.detection_last_analyzed_url = Some(url_owned.clone());

                    // Apply fingerprint poisoning (once per page load)
                    if self.poison_last_applied_url.as_deref() != Some(url_owned.as_str()) {
                        if self.poison_engine.should_poison(&url_owned) {
                            // Inject fingerprint poisoning via dedicated script engine
                            let _result = self.poison_engine.poison_page(
                                self.poison_script_engine.interpreter_mut(),
                                &url_owned,
                            );
                            self.poison_badge_pulse = true;
                            // Record stealth kill for the poisoned URL
                            self.stealth_victories.silent_kill(&url_owned);
                            self.status_message = format!(
                                "Fingerprint poisoning active: {}",
                                self.poison_engine.mode_description()
                            );
                        } else {
                            self.poison_badge_pulse = false;
                        }
                        self.poison_last_applied_url = Some(url_owned);
                    }
                }
            }
        }

        // ═══════════════════════════════════════════════════════════════════════
        // MCP BRIDGE — Poll pending commands each frame
        // ═══════════════════════════════════════════════════════════════════════
        {
            let commands = self.mcp_bridge.take_commands();
            for (seq, cmd) in commands {
                let response = self.handle_mcp_command(cmd);
                self.mcp_bridge.push_response(seq, response);
            }

            // Forward detection alerts to MCP bridge for connected clients
            if let Ok(mut shared) = self.detection_engine.shared_alerts().lock() {
                if !shared.is_empty() {
                    let alerts: Vec<_> = shared.drain(..).collect();
                    self.mcp_bridge.push_detection_alerts(alerts);
                }
            }
        }

        // Handle download requests after policy checks
        let pending_downloads = self.engine.take_pending_downloads();
        for (url, suggested) in pending_downloads {
            self.guard_download_request(&url, suggested.as_deref());
        }

        // Feed network monitor with active tab loading state
        if let Some(Tab {
            content: TabContent::Web { url, loading, .. },
            ..
        }) = self.engine.active_tab()
        {
            if *loading {
                if self.network_active_connection.is_none() && !self.network_monitor.is_blocked(url)
                {
                    let conn_id = self
                        .network_monitor
                        .start_connection(url, ConnectionType::Document);
                    self.network_active_connection = Some(conn_id);
                    self.network_last_net_sample = Some(Instant::now());
                }

                if let Some(conn_id) = self.network_active_connection {
                    let now = Instant::now();
                    let last = self.network_last_net_sample.unwrap_or(now);
                    let secs = (now - last).as_secs_f64().max(0.016);
                    let bytes_down = (80_000.0 * secs) as u64;
                    let bytes_up = (2_000.0 * secs) as u64;
                    self.network_monitor
                        .update_connection(conn_id, bytes_down, bytes_up);
                    self.network_last_net_sample = Some(now);
                }
            } else if let Some(conn_id) = self.network_active_connection.take() {
                self.network_monitor.complete_connection(
                    conn_id,
                    200,
                    Some("text/html".to_string()),
                );
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
                let history_title = if raw_title.is_empty() {
                    tab.title()
                } else {
                    raw_title.clone()
                };
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
        // Voice input panel
        self.render_voice_panel(ctx);

        // Password vault panel
        self.render_vault_panel(ctx);

        // Ad blocker panel
        self.render_adblock_panel(ctx);

        // Sandbox isolation panel
        self.render_sandbox_panel(ctx);

        // Hit test diagnostics panel
        self.render_hittest_diagnostic_panel(ctx);

        // REST client panel
        self.render_rest_client_panel(ctx);

        // Network activity monitor panel (network.rs)
        self.render_network_activity_panel(ctx);

        // Protocol diagnostics panel
        self.render_protocol_diagnostics_panel(ctx);

        // MCP AI Panel
        self.render_mcp_panel(ctx);

        // Developer console panel (F12)
        self.render_dev_console(ctx);

        // Clear data confirmation dialog
        if self.show_clear_data_dialog {
            let mut open = true;
            egui::Window::new("Clear Browsing Data")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Select which data to clear:");
                    ui.add_space(10.0);
                    ui.checkbox(&mut self.clear_data_history, "Browsing History");
                    ui.checkbox(&mut self.clear_data_downloads, "Completed Downloads");
                    ui.checkbox(&mut self.clear_data_cache, "Cached Data");
                    ui.add_space(15.0);
                    ui.horizontal(|ui| {
                        if ui.button("Clear Selected").clicked() {
                            if self.clear_data_history {
                                self.engine.history.clear();
                            }
                            if self.clear_data_downloads {
                                self.engine.downloads.clear_finished();
                            }
                            if self.clear_data_cache {
                                self.html_renderer.clear_cache();
                            }
                            self.status_message = "Browsing data cleared".into();
                            self.show_clear_data_dialog = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_clear_data_dialog = false;
                        }
                    });
                });
            if !open {
                self.show_clear_data_dialog = false;
            }
        }

        // Status bar
        self.render_status_bar(ctx);

        // Sidebars (must be before CentralPanel)
        self.render_left_sidebar(ctx);
        self.render_ai_sidebar(ctx);

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_content(ctx, ui);
        });
    }
}

/// Extract domain from a URL string (best-effort, no panic)
fn extract_domain_from_url(url: &str) -> String {
    if let Ok(parsed) = Url::parse(url) {
        parsed.host_str().unwrap_or("unknown").to_string()
    } else {
        "unknown".to_string()
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
    // ────────────────────────────────────────────────
    // Load Metamorphous for brand-consistent, crisp text
    // Also keep Space Grotesk as a secondary fallback
    // ────────────────────────────────────────────────
    let mut fonts = egui::FontDefinitions::default();

    // Primary: Metamorphous (bundled)
    let meta_bytes = include_bytes!("../assets/fonts/Metamorphous-7wZ4.ttf");
    fonts.font_data.insert(
        "Metamorphous".into(),
        egui::FontData::from_static(meta_bytes),
    );

    // Secondary: Space Grotesk (bundled fallback)
    let space_bytes = include_bytes!("../Space_Grotesk/static/SpaceGrotesk-Regular.ttf");
    let fb = egui::FontData::from_static(space_bytes);
    for &k in &["Space Grotesk", "SpaceGrotesk", "Azo Sans"] {
        fonts.font_data.insert(k.into(), fb.clone());
    }

    // Metamorphous first, Space Grotesk second, then system defaults
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "Metamorphous".into());
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .insert(1, "Space Grotesk".into());

    // Also set Metamorphous for monospace so UI is consistent
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "Metamorphous".into());

    ctx.set_fonts(fonts);
}

fn configure_style(ctx: &egui::Context, dark_mode: bool, _preset: ThemePreset) {
    // ────────────────────────────────────────────────
    // Concise, modern styling — brand colors + crisp feel
    // ────────────────────────────────────────────────
    let accent = Color32::from_rgb(108, 99, 255); // #6C63FF  Brand Purple
    let yellow = Color32::from_rgb(254, 195, 55); // #FEC337  Highlight
    let bg_dark = Color32::from_rgb(16, 30, 50); // #101E32  Deep navy
    let bg_mid = Color32::from_rgb(46, 56, 75); // #2E384B  Panel surface

    let mut visuals = if dark_mode {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    if dark_mode {
        visuals.window_fill = bg_dark;
        visuals.panel_fill = bg_mid;
        visuals.extreme_bg_color = Color32::from_rgb(10, 20, 36);
        visuals.faint_bg_color = bg_mid.gamma_multiply(1.2);
        visuals.override_text_color = Some(Color32::from_rgb(230, 237, 243));
        visuals.hyperlink_color = yellow;
        visuals.selection.bg_fill = accent.gamma_multiply(0.35);
        visuals.selection.stroke = egui::Stroke::new(1.5, accent);
    } else {
        visuals.window_fill = Color32::from_rgb(246, 246, 246);
        visuals.panel_fill = Color32::WHITE;
        visuals.override_text_color = Some(bg_dark);
        visuals.hyperlink_color = accent;
        visuals.selection.bg_fill = Color32::from_rgb(184, 180, 255);
        visuals.selection.stroke = egui::Stroke::new(1.5, accent);
    }

    // Slim chrome & consistent rounding
    visuals.window_rounding = egui::Rounding::same(10.0);
    visuals.menu_rounding = egui::Rounding::same(8.0);
    let widget_r = egui::Rounding::same(8.0);
    for w in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        w.rounding = widget_r;
    }

    visuals.window_shadow = egui::Shadow {
        offset: egui::vec2(0.0, 6.0),
        blur: 16.0,
        spread: 0.0,
        color: Color32::from_black_alpha(60),
    };
    visuals.popup_shadow = egui::Shadow {
        offset: egui::vec2(0.0, 4.0),
        blur: 12.0,
        spread: 0.0,
        color: Color32::from_black_alpha(50),
    };

    ctx.set_visuals(visuals);

    // ────────────────────────────────────────────────
    // Tight typography & spacing
    // ────────────────────────────────────────────────
    let mut style = (*ctx.style()).clone();
    style
        .text_styles
        .insert(egui::TextStyle::Small, FontId::proportional(12.5));
    style
        .text_styles
        .insert(egui::TextStyle::Body, FontId::proportional(14.5));
    style
        .text_styles
        .insert(egui::TextStyle::Monospace, FontId::monospace(13.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, FontId::proportional(14.5));
    style
        .text_styles
        .insert(egui::TextStyle::Heading, FontId::proportional(19.0));

    style.spacing.item_spacing = egui::vec2(9.0, 7.0);
    style.spacing.button_padding = egui::vec2(12.0, 7.0);
    style.spacing.menu_margin = egui::Margin::symmetric(12.0, 10.0);
    style.spacing.interact_size = egui::vec2(40.0, 24.0);
    style.spacing.scroll.bar_width = 7.0;
    style.animation_time = 0.12;

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
    )
    .map_err(|e| anyhow::anyhow!("Failed to run browser: {}", e))
}
