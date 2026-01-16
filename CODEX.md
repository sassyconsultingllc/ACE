# SASSY BROWSER v2.0.0 - DISRUPTOR CODEX
## Persistent Project Context (Chat-Crash Proof)

**Last Updated:** 2026-01-16 02:00 UTC  
**Build Status:** READY FOR COMPILATION  
**Location:** `V:\sassy-browser-FIXED`  
**Source Files:** 81 Rust files  
**Architecture:** PURE RUST - NO CHROME/GOOGLE/WEBKIT

---

## 🎯 PROJECT GOAL

The end-all-be-all disrupter. People will line up for this browser.

**KILLS these paid dependencies:**
| Product | Annual Cost | Our Replacement |
|---------|-------------|-----------------|
| Adobe Creative Cloud | $504/yr | Image viewer (RAW, PSD, EXR) |
| Microsoft 365 | $100/yr | Document viewer (DOCX, XLSX) |
| ChemDraw | $2,600/yr | Molecular viewer (PDB, MOL) |
| PACS Viewers | $10,000+ | Medical imaging (DICOM) |
| AutoCAD LT | $2,000/yr | CAD viewer (DXF, DWG) |
| Postman Pro | $144/yr | REST client built-in |
| WinRAR | Nagware | Archive support (ZIP, 7Z, RAR) |
| Calibre | Free but ugly | Ebook viewer (EPUB, MOBI) |
| Chrome | Your soul | Pure Rust, no Google |

---

## 🚫 CHROME-FREE ARCHITECTURE

```
WHAT WE USE:                    WHAT WE DON'T USE:
─────────────────────────────   ─────────────────────────────
eframe/egui (Pure Rust GUI)     ❌ Chromium
winit (Pure Rust windowing)     ❌ WebKit
softbuffer (Pure Rust pixels)   ❌ WebView2
fontdue (Pure Rust fonts)       ❌ V8 JavaScript
html5ever (Pure Rust HTML)      ❌ Blink rendering
cssparser (Pure Rust CSS)       ❌ Google APIs
ureq (Pure Rust HTTP)           ❌ Google Analytics
SassyScript (Our JS engine)     ❌ Any telemetry
```

---

## 📁 MODULE MAP (81 source files)

### Core Application
| Module | Lines | Purpose |
|--------|-------|---------|
| `main.rs` | 248 | Entry point, CLI, no-Chrome banner |
| `app.rs` | 1085 | Main egui application, viewer routing |
| `file_handler.rs` | 2055 | Universal file detection, 100+ formats |
| `html_renderer.rs` | ~300 | HTML to egui rendering |

### Pure Rust Browser Engine
| Module | Lines | Purpose |
|--------|-------|---------|
| `engine.rs` | 2077 | Browser engine, event loop, winit |
| `renderer.rs` | ~400 | Render pipeline |
| `paint.rs` | ~400 | Paint operations (softbuffer) |
| `layout.rs` | ~800 | CSS layout (block, inline, flex) |
| `style.rs` | ~700 | CSS cascade |
| `dom.rs` | ~500 | DOM tree |
| `hittest.rs` | ~200 | Click detection |

### SassyScript JavaScript Engine
| Module | Lines | Purpose |
|--------|-------|---------|
| `js/mod.rs` | - | Module exports |
| `js/lexer.rs` | ~400 | Tokenizer |
| `js/parser.rs` | ~600 | AST builder |
| `js/interpreter.rs` | ~1200 | Execution (no V8, no JIT) |
| `js/value.rs` | ~500 | JS values, objects |
| `js/dom.rs` | ~300 | DOM bridge |
| `script_engine.rs` | ~400 | localStorage API |

### Universal File Viewers (13 specialized)
| Module | Lines | Purpose |
|--------|-------|---------|
| `viewers/image.rs` | ~500 | PNG, JPG, RAW, PSD, SVG, EXR |
| `viewers/pdf.rs` | ~400 | PDF with pages, search, zoom |
| `viewers/document.rs` | ~450 | DOCX, ODT, RTF editing |
| `viewers/spreadsheet.rs` | ~500 | XLSX, CSV with formulas |
| `viewers/chemical.rs` | ~400 | PDB, MOL 3D structures |
| `viewers/archive.rs` | ~400 | ZIP, 7Z, RAR tree view |
| `viewers/model3d.rs` | ~400 | OBJ, STL, GLTF viewport |
| `viewers/font.rs` | ~400 | TTF, OTF character map |
| `viewers/audio.rs` | ~400 | MP3, FLAC waveform |
| `viewers/video.rs` | ~386 | MP4, MKV metadata |
| `viewers/ebook.rs` | ~428 | EPUB, MOBI chapters |
| `viewers/text.rs` | ~300 | 100+ lang syntax highlight |
| `viewers/mod.rs` | - | Viewer exports |

### Browser Infrastructure
| Module | Lines | Purpose |
|--------|-------|---------|
| `browser/mod.rs` | 18 | Browser exports |
| `browser/engine.rs` | 682 | Tab coordination |
| `browser/tab.rs` | ~300 | Tab state |
| `browser/history.rs` | ~200 | History manager |
| `browser/bookmarks.rs` | ~200 | Bookmark manager |
| `browser/download.rs` | ~300 | Download manager |
| `browser/webview.rs` | 31 | Stub (no Chrome) |
| `browser/runner.rs` | 22 | Pure Rust launcher |

### Security & Sandbox
| Module | Lines | Purpose |
|--------|-------|---------|
| `sandbox/mod.rs` | - | Sandbox exports |
| `sandbox/page.rs` | ~300 | Page trust levels |
| `sandbox/popup.rs` | ~200 | Smart popup blocker |
| `sandbox/quarantine.rs` | ~400 | Download quarantine |
| `crypto.rs` | ~300 | Local encryption |
| `cookies.rs` | ~300 | Cookie jar (no tracking) |

### Developer Tools
| Module | Lines | Purpose |
|--------|-------|---------|
| `console.rs` | ~400 | Dev console |
| `rest_client.rs` | ~600 | REST API client |
| `json_viewer.rs` | ~300 | JSON pretty-print |
| `syntax.rs` | ~400 | Syntax highlighting |
| `markdown.rs` | ~300 | Markdown renderer |

### MCP AI System (No Google)
| Module | Lines | Purpose |
|--------|-------|---------|
| `mcp.rs` | ~800 | Multi-agent orchestrator |
| `mcp_panel.rs` | ~600 | Chat UI |
| `mcp_api.rs` | ~500 | xAI + Anthropic APIs |
| `mcp_fs.rs` | ~400 | Sandboxed filesystem |
| `mcp_git.rs` | ~500 | Git integration |
| `voice.rs` | ~400 | Whisper STT (offline) |
| `ai.rs` | ~300 | AI runtime |

### Phone Sync (Tailscale P2P)
| Module | Lines | Purpose |
|--------|-------|---------|
| `sync/mod.rs` | - | Sync exports |
| `sync/server.rs` | ~300 | Sync server |
| `sync/protocol.rs` | ~200 | Sync protocol |
| `sync/secure.rs` | ~200 | E2E encryption |
| `sync/family.rs` | ~200 | Family sharing |
| `sync/users.rs` | ~300 | User profiles |

### UI Components
| Module | Lines | Purpose |
|--------|-------|---------|
| `ui/mod.rs` | - | UI exports |
| `ui/tabs.rs` | ~400 | Tab bar |
| `ui/sidebar.rs` | ~200 | Sidebar |
| `ui/popup.rs` | ~300 | Popup manager |
| `ui/network_bar.rs` | ~200 | Network indicator |
| `ui/network.rs` | ~200 | Network state |
| `ui/theme.rs` | ~150 | Themes |
| `ui/input.rs` | ~200 | Input handling |
| `ui/render.rs` | ~300 | UI rendering |

### Utilities
| Module | Lines | Purpose |
|--------|-------|---------|
| `imaging.rs` | ~400 | Image cache |
| `input.rs` | ~200 | Input state |
| `network.rs` | ~300 | HTTP layer |
| `protocol.rs` | ~200 | Protocol handlers |
| `data.rs` | ~200 | Data structures |
| `setup.rs` | ~300 | First-run wizard |
| `update.rs` | ~300 | Auto-update |
| `extensions.rs` | ~400 | Extension system |

---

## 📊 FILE FORMAT SUPPORT (100+)

### Images (25 formats)
PNG, JPG, JPEG, GIF, WebP, SVG, AVIF, BMP, TIFF, TGA, HDR, EXR, PNM, QOI, DDS, ICO, CR2, CR3, NEF, ARW, DNG, RAF, ORF, RW2, PEF, SRW, PSD, PSB

### Documents (6 formats)
PDF, DOCX, DOC, ODT, RTF, WPD

### Spreadsheets (5 formats)
XLSX, XLS, ODS, CSV, TSV

### Scientific (7 formats)
PDB, MOL, SDF, XYZ, CIF, MOL2, MMCIF

### Archives (8 formats)
ZIP, TAR, GZ, BZ2, XZ, 7Z, RAR, ZST

### 3D Models (8 formats)
OBJ, STL, GLTF, GLB, PLY, FBX, DAE, 3DS

### Fonts (5 formats)
TTF, OTF, WOFF, WOFF2, EOT

### Audio (9 formats)
MP3, FLAC, WAV, OGG, M4A, AAC, WMA, OPUS, AIFF

### Video (9 formats)
MP4, MKV, WebM, AVI, MOV, WMV, FLV, M4V, OGV

### Ebooks (5 formats)
EPUB, MOBI, AZW, AZW3, FB2

### Code/Text (100+ languages)
Rust, Python, JavaScript, TypeScript, C, C++, Java, Go, Ruby, PHP, Swift, Kotlin, Scala, Haskell, Lua, Perl, R, SQL, Shell, PowerShell, HTML, CSS, JSON, YAML, TOML, XML, Markdown, LaTeX, and 80+ more

---

## 🔐 SECURITY MODEL

### 4-Layer Sandbox
```
Layer 1: NETWORK SANDBOX
├── All content quarantined in memory
└── No direct OS network access from pages

Layer 2: RENDER SANDBOX  
├── SassyScript engine (no V8 exploits)
└── DOM is simulation, not OS objects

Layer 3: CONTENT SANDBOX
├── Images decoded in Rust (no codec vulns)
└── Fonts rendered in Rust (no FreeType)

Layer 4: DOWNLOAD QUARANTINE
├── Files held in memory, not filesystem
├── 3 deliberate interactions to release
├── 5 second minimum wait
└── Heuristic warnings shown
```

### Page Trust Levels
```
UNTRUSTED (Red)    → Fresh page, no interactions
CAUTIOUS (Orange)  → 1 meaningful interaction
TRUSTED (Green)    → 3 meaningful interactions
VERIFIED (Blue)    → User explicitly trusts site
```

---

## 🛠️ BUILD COMMANDS

```batch
# Build release (Pure Rust, no Chrome)
cargo build --release

# Run browser
target\release\sassy-browser.exe

# Open specific file
target\release\sassy-browser.exe molecule.pdb
target\release\sassy-browser.exe document.pdf

# Open URL
target\release\sassy-browser.exe https://duckduckgo.com
```

---

## 📝 CHANGELOG v2.0.0

### Added
- 13 specialized file viewers
- 100+ file format support
- Pure Rust browser engine (no Chrome)
- Chrome-free Cargo.toml
- Universal file handler with detection

### Removed
- `wry` dependency (was using WebView2/Chromium)
- `tao` dependency (wry's window manager)
- All Google/Chrome references
- WebView-based rendering

### Changed
- Architecture from Chrome-based to Pure Rust
- main.rs to integrate all modules
- browser/webview.rs to stub
- browser/runner.rs to Pure Rust

---

## 🔗 QUICK LINKS

- **Source:** `V:\sassy-browser-FIXED\src\`
- **Viewers:** `V:\sassy-browser-FIXED\src\viewers\`
- **Cargo.toml:** `V:\sassy-browser-FIXED\Cargo.toml`
- **Backup:** `V:\sassy-browser-BACKUP\`
