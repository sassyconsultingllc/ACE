//! Sassy Browser v2.0.0 - Pure Rust Web Browser & Universal File Viewer
//!
//! �*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*�
//! NO CHROME. NO GOOGLE. NO WEBKIT. NO WEBVIEW2. NO TELEMETRY. PURE RUST.
//! �*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*�
//!
//! SECURITY ARCHITECTURE - 4 SANDBOX LAYERS:
//! ==============================================================================
//! 1. NETWORK SANDBOX - All content quarantined in memory
//! 2. RENDER SANDBOX - SassyScript engine (no V8, no JIT exploits)  
//! 3. CONTENT SANDBOX - Images/fonts decoded in Rust (no native codec vulns)
//! 4. DOWNLOAD QUARANTINE - Files held in memory, 3 confirms to release
//!
//! WHY PURE RUST MATTERS:
//! ==============================================================================
//! Chrome/Chromium = Google telemetry, V8 JIT exploits, massive attack surface
//! WebView2 = Microsoft's Chrome wrapper, same problems
//! WebKit = Apple's engine, still complex C++ with exploits
//!
//! Sassy Browser = 100% Rust, custom everything, no phone-home, no tracking
//!
//! FILE FORMAT SUPPORT - 100+ formats (see viewers/)

// �*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*�
// CORE MODULES
// �*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*�
mod app;                    // Main application UI (egui)
mod file_handler;           // Universal file type detection and loading
mod viewers;                // File type viewers (PDF, images, etc.)

// �*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*�
// PURE RUST BROWSER ENGINE
// �*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*�
mod js;                     // SassyScript JavaScript interpreter
mod dom;                    // DOM simulation
mod style;                  // CSS engine
mod layout;                 // Layout engine (flexbox, etc.)
mod layout_engine;          // CSS layout engine (taffy-based)
mod paint;                  // Painting/rendering
mod renderer;               // HTML renderer
mod engine;                 // Core browser engine
mod script_engine;          // Script execution
mod html_renderer;          // HTML to egui rendering

// �*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*�
// SECURITY & SANDBOXING
// �*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*�
mod sandbox;                // Page sandbox, popup blocker, download quarantine
mod cookies;                // Cookie storage (local only, no tracking)
mod crypto;                 // Cryptographic identity (local keys)
mod auth;                   // Authentication, licensing, Tailscale, phone sync

// �*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*�
// DISRUPTOR FEATURES - Kills paid software
// �*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*�
mod network_monitor;        // Always-visible activity indicator, NO hidden traffic
mod password_vault;         // Built-in password manager (ChaCha20-Poly1305, Argon2id)
mod smart_history;          // 14.7s delay history, NSFW auto-detection
mod family_profiles;        // Adult/Teen/Kid profiles, time limits, parental controls

// �*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*�
// DEVELOPER TOOLS
// �*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*�
mod console;                // Developer console
mod rest_client;            // Built-in REST client
mod json_viewer;            // JSON pretty-print and navigation
mod syntax;                 // Syntax highlighting
mod markdown;               // Markdown renderer

// �*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*�
// MCP - MULTI-AGENT AI SYSTEM (Grok + Claude, NOT Google)
// �*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*�
mod detection;              // Client-side detection engine with honeypot system
mod poisoning;              // Fingerprint poisoning engine (canvas, audio, WebGL, etc.)
mod behavioral_mimicry;     // Human behavior simulation (mouse, scroll, typing noise)
mod behavioral_mimicry_level4; // Level 4: self-evolving behavioral poisoning
mod stealth_victories;    // Silent anti-tracking warfare (poison counter, honeypot, fake IDs, entropy bomb)
mod mcp;                    // Model Context Protocol orchestrator
mod mcp_panel;              // MCP UI panel
mod mcp_api;                // API clients (xAI Grok, Anthropic Claude - NO Google)
mod mcp_fs;                 // Sandboxed file system for AI
mod mcp_git;                // Git integration
mod mcp_server;             // MCP Server - expose browser as MCP server for Claude Desktop
mod mcp_protocol;           // Native MCP protocol (bincode, NO JSON)
mod mcp_codec;              // Length-prefixed bincode framing (tokio-util codec)
mod mcp_client;             // MCP WebSocket client (tokio-tungstenite)
mod mcp_server_native;      // Native MCP WebSocket server (bincode transport)

// �*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*�
// NETWORKING & SYNC (NO GOOGLE CLOUD)
// �*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*�
mod network;                // HTTP networking (ureq - no Google APIs)
mod protocol;               // Protocol handling
mod sync;                   // Phone sync via Tailscale (peer-to-peer, no cloud)

// �*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*�
// UI & INPUT
// �*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*�
mod ui;                     // UI components
mod input;                  // Input handling
mod hittest;                // Hit testing
mod imaging;                // Image loading/caching

// �*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*�
// PRINTING - Universal print system
// �*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*�
mod print;                  // Cross-platform print dialog and preview

// �*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*�
// UTILITIES
// �*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*�
mod ai;                     // AI assistant (optional, prefers local/open models)
mod data;                   // Data structures
mod setup;                  // First-run setup
mod update;                 // Self-update (from OUR servers, not Google/Microsoft)
mod extensions;             // Extension system
mod voice;                  // Voice input (Whisper - runs locally, offline)

// HTTP client helper (builds UA and wraps requests)
mod http_client;
mod tls_spoof;              // TLS ClientHello fingerprint spoofing (Chrome 132)

// Browser module (tabs, history, bookmarks)
mod browser;

// Case normalization helpers (global rule for canonical comparisons)
mod fontcase;

// Ad blocking (EasyList/EasyPrivacy filter lists)
mod adblock;
pub mod icons;                 // SVG icon system (replaces inline Unicode emoji)

// Self-healing health watchdog (monitors + auto-fixes browser internals)
mod health;

use std::env;

fn main() {
    // Initialize logging (NO Google Analytics, NO telemetry, NO phone-home)
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    println!(r#"
�*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*�
�*�                    Sassy Browser v2.0.0 - Pure Rust Edition                   �*�
�*�                                                                               �*�
�*�   No Chrome. No Google. No WebKit. No WebView2. No Telemetry. Just Freedom.   �*�
�*�                                                                               �*�
�*�   Built with 100% Rust: html5ever, cssparser, fontdue, softbuffer, egui       �*�
�*�   Custom JS engine: SassyScript (no V8, no JIT exploits, no WASM vulns)       �*�
�*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*��*�
"#);

    tracing::info!("Starting Sassy Browser v2.0.0 - Pure Rust Edition");
    
    let args: Vec<String> = env::args().collect();
    
    // Command line handling
    if args.len() > 1 {
        match args[1].as_str() {
            "--help" | "-h" => {
                print_help();
                return;
            }
            "--version" | "-v" => {
                println!("Sassy Browser v2.0.0 - Pure Rust Edition");
                println!("No Chrome. No Google. No WebKit. No Telemetry.");
                println!("100% Rust. Zero tracking. Maximum freedom.");
                return;
            }
            "--phone-app" | "-p" => {
                serve_phone_app();
                return;
            }
            "--reset" => {
                reset_data();
                return;
            }
            "--mcp-server" => {
                // Run as MCP server (for Claude Desktop integration)
                run_mcp_server_mode(&args[2..]);
                return;
            }
            "--pure-engine" | "-e" => {
                // Run pure Rust engine (winit + softbuffer)
                run_pure_engine();
                return;
            }
            "--webview" => {
                // Enable system/webview-style fetching (uses bundled HTTP client)
                std::env::set_var("SASSY_ENABLE_WEBVIEW", "1");
                run_browser(None);
                return;
            }
            url if url.starts_with("http") || url.starts_with("file:") => {
                run_browser(Some(url.to_string()));
                return;
            }
            path if std::path::Path::new(path).exists() => {
                // Open file directly � canonicalize for correct Windows handling
                let abs_path = std::fs::canonicalize(path).unwrap_or_else(|_| std::path::PathBuf::from(path));
                #[cfg(windows)]
                {
                    // Windows file:// URLs use forward slashes and start with three slashes: file:///C:/path
                    let s = abs_path.to_string_lossy().replace('\\', "/");
                    run_browser(Some(format!("file:///{}", s)));
                }
                #[cfg(not(windows))]
                {
                    run_browser(Some(format!("file://{}", abs_path.display())));
                }
                return;
            }
            _ => {
                println!("Unknown argument: {}", args[1]);
                print_help();
                return;
            }
        }
    }
    
    run_browser(None);
}

fn run_browser(url: Option<String>) {
    // Pass initial URL through env var so BrowserApp::new can read it
    if let Some(ref u) = url {
        std::env::set_var("SASSY_INITIAL_URL", u);
    }

    // Check first-run setup
    if setup::ensure_setup().is_none() {
        tracing::warn!("Setup cancelled by user");
        return;
    }
    
    // Run the egui-based browser with pure Rust file viewers
    // Web content rendered by our custom engine (dom, style, layout, paint)
    if let Err(e) = app::run_browser() {
        tracing::error!("Browser error: {}", e);
        eprintln!("[x] Fatal error: {}", e);
        std::process::exit(1);
    }
}

fn run_pure_engine() {
    // Run the pure Rust engine directly (winit + softbuffer)
    // This bypasses egui and uses our custom rendering pipeline
    tracing::info!("Starting pure Rust engine (winit + softbuffer)");

    // Use the pure Rust rendering engine (dom + style + layout + paint)
    engine::run_browser(None);
}

fn serve_phone_app() {
    println!("Starting phone sync server...");
    println!("Uses Tailscale for peer-to-peer sync - NO cloud, NO Google, NO tracking");
    println!("Scan the QR code with your phone to connect.");

    // Detect Tailscale for secure peer-to-peer connectivity
    let ts = sync::TailscaleInfo::detect();
    if ts.available {
        if let Some(ref ip) = ts.ip {
            println!("Tailscale detected: {} ({})", ip, ts.hostname.as_deref().unwrap_or("unknown"));
        }
    } else {
        println!("Tailscale not detected � falling back to localhost only.");
    }

    // Start the WebSocket sync server
    let port = 8765u16;
    let mut server = sync::SyncServer::new(port);
    match server.start() {
        Ok(()) => {
            println!("Phone sync server listening on port {}", port);
            if let Some(url) = ts.connection_url(port) {
                println!("Connect URL: {}", url);
            } else {
                println!("Connect URL: ws://127.0.0.1:{}", port);
            }
            println!("Press Ctrl+C to stop.");
            // Block the main thread � server runs on background threads
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
                if !server.is_running() {
                    break;
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to start sync server: {}", e);
            std::process::exit(1);
        }
    }
}

fn reset_data() {
    println!("Resetting all browser data...");
    if let Some(data_dir) = dirs::data_dir() {
        let sassy_dir = data_dir.join("sassy-browser");
        if sassy_dir.exists() {
            if let Err(e) = std::fs::remove_dir_all(&sassy_dir) {
                eprintln!("Failed to remove data: {}", e);
            } else {
                println!("✅ Data reset complete");
            }
        } else {
            println!("No data to reset");
        }
    }
}

fn run_mcp_server_mode(args: &[String]) {
    println!("Starting Sassy Browser MCP Server...");
    println!("Connect from Claude Desktop or any MCP client.");
    
    let mut config = mcp_server::McpServerConfig { enabled: true, ..Default::default() };
    
    // Parse additional args
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--mcp-port" if i + 1 < args.len() => {
                if let Ok(port) = args[i + 1].parse() {
                    config.port = port;
                    config.transport = "socket".to_string();
                }
                i += 1;
            }
            "--mcp-socket" => {
                config.transport = "socket".to_string();
            }
            _ => {}
        }
        i += 1;
    }
    
    mcp_server::run_mcp_server(config);
}

fn print_help() {
    println!(r#"
Sassy Browser v2.0.0 - Pure Rust Web Browser & Universal File Viewer

NO CHROME. NO GOOGLE. NO WEBKIT. NO WEBVIEW2. NO TELEMETRY.

USAGE:
    sassy-browser [OPTIONS] [URL or FILE]

OPTIONS:
    -h, --help          Show this help
    -v, --version       Show version
    -p, --phone-app     Start phone sync server (Tailscale, no cloud)
    -e, --pure-engine   Run with pure Rust renderer (experimental)
    --reset             Clear all browser data
    --mcp-server        Run as MCP server (for Claude Desktop)
    --mcp-port PORT     MCP server port (default: 9999, implies socket mode)
    --mcp-socket        Use socket transport instead of stdio

EXAMPLES:
    sassy-browser                           Open browser
    sassy-browser https://example.com       Open URL
    sassy-browser document.pdf              Open file
    sassy-browser molecule.pdb              View molecular structure
    sassy-browser --mcp-server              Run as MCP server (stdio)
    sassy-browser --mcp-server --mcp-port 9999  Run MCP server on port 9999

KEYBOARD SHORTCUTS:
    Ctrl+L              Focus address bar
    Ctrl+T              New tab
    Ctrl+W              Close tab
    Ctrl+O              Open file
    Ctrl+F              Find in page
    F5                  Refresh
    F11                 Fullscreen
    F12                 Developer tools

SUPPORTED FILE FORMATS (100+):
    Images:      PNG, JPG, GIF, WebP, SVG, RAW, PSD, AVIF, EXR
    Documents:   PDF, DOCX, ODT, RTF, EPUB, MOBI
    Spreadsheets: XLSX, ODS, CSV
    Scientific:  PDB, MOL, SDF, XYZ, CIF
    Archives:    ZIP, 7Z, RAR, TAR, GZ
    3D Models:   OBJ, STL, GLTF, PLY
    Fonts:       TTF, OTF, WOFF
    Audio:       MP3, FLAC, WAV, OGG
    Video:       MP4, MKV, WebM
    Code:        100+ languages with syntax highlighting

PRIVACY PROMISE:
    ✓ No telemetry
    ✓ No analytics
    ✓ No phone-home
    ✓ No cloud sync (use Tailscale for peer-to-peer)
    ✓ No Google anything
    ✓ No Microsoft anything
    ✓ 100% open source Rust

https://sassyconsultingllc.com/browser
"#);
}
