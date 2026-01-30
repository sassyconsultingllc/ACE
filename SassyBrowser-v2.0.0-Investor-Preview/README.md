# SASSY BROWSER v2.0.0

**Universal File Viewer & Browser — Pure Rust — Zero Paid Dependencies**

```
╔═══════════════════════════════════════════════════════════════════════════════╗
║  ███████╗ █████╗ ███████╗███████╗██╗   ██╗    ██████╗ ██████╗  ██████╗        ║
║  ██╔════╝██╔══██╗██╔════╝██╔════╝╚██╗ ██╔╝    ██╔══██╗██╔══██╗██╔═══██╗       ║
║  ███████╗███████║███████╗███████╗ ╚████╔╝     ██████╔╝██████╔╝██║   ██║       ║
║  ╚════██║██╔══██║╚════██║╚════██║  ╚██╔╝      ██╔══██╗██╔══██╗██║   ██║       ║
║  ███████║██║  ██║███████║███████║   ██║       ██████╔╝██║  ██║╚██████╔╝       ║
║  ╚══════╝╚═╝  ╚═╝╚══════╝╚══════╝   ╚═╝       ╚═════╝ ╚═╝  ╚═╝ ╚═════╝        ║
║                                                                               ║
║  Universal File Viewer • Web Browser • No Chrome • No Google • Pure Rust      ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

## 🎯 The End-All-Be-All Disruptor

**People will line up for this browser.**

One application. 200+ file formats. **Zero paid subscriptions.**

| Software Killed | Annual Cost | Our Feature |
|-----------------|-------------|-------------|
| Adobe Creative Cloud | $504/yr | Image editor (RAW, PSD, EXR, HEIC) |
| Microsoft 365 | $100/yr | DOCX/XLSX editor with formatting |
| ChemDraw | $2,600/yr | Molecular viewer (PDB, MOL, SDF) |
| PACS Viewers | $10,000+ | Medical imaging (DICOM) |
| AutoCAD LT | $2,000/yr | CAD viewer (DXF, DWG) |
| Postman Pro | $144/yr | REST client built-in |
| WinRAR | Nagware | Archive support (ZIP, 7Z, RAR) |
| Calibre | Ugly | Ebook reader (EPUB, MOBI) |
| **Chrome** | Your soul | **Pure Rust, zero Google** |

---

## ⚡ Quick Start

```bash
# Build from source
cargo build --release

# Open any file
sassy-browser.exe document.pdf
sassy-browser.exe molecule.pdb
sassy-browser.exe photo.cr2

# Or browse the web
sassy-browser.exe https://duckduckgo.com
```

---

## 📁 Supported Formats (200+)

### 🖼️ Images — View, Edit, Save, Print
**Standard:** PNG, JPG, JPEG, GIF, WebP, BMP, ICO, TIFF, TGA, SVG, AVIF, HEIC
**RAW Camera:** CR2, CR3, NEF, ARW, DNG, ORF, RW2, RAF, PEF, SRW, X3F
**Professional:** PSD, PSB, EXR, HDR, DDS, QOI, OpenEXR

*Edit: Crop, resize, rotate, flip, filters, adjustments, layers*
*Export: PNG, JPEG, WebP, BMP, TIFF*

### 📄 Documents — View, Edit, Save, Print
**Office:** DOCX, DOC, ODT, RTF, WPD
**PDF:** Full viewing, annotations, form fill, merge/split
**Spreadsheet:** XLSX, XLS, ODS, CSV, TSV

*Edit: Rich text, styles, find/replace, track changes*
*Export: DOCX, ODT, RTF, PDF, HTML, Markdown*

### 🧬 Chemical/Biological — 3D Visualization
**Structures:** PDB, MOL, MOL2, SDF, XYZ
**Crystallography:** CIF, mmCIF

*Features: Ball-and-stick, wireframe, spacefill rendering*
*Color by: Element, chain, residue, B-factor*

### 📦 Archives — View, Extract, Create
**Compression:** ZIP, RAR, 7Z, TAR, GZ, XZ, BZ2, ZSTD
**Packages:** JAR, WAR, APK, DEB, RPM

*Features: Tree view, extract, preview contents*

### 🎲 3D Models — Interactive Viewer
**Formats:** OBJ, STL, GLTF, GLB, PLY, FBX, DAE, 3DS

*Features: Rotate, zoom, wireframe, materials*

### 🔤 Fonts — Preview & Install
**Formats:** TTF, OTF, WOFF, WOFF2, EOT

*Features: Character map, glyph preview, metadata*

### 🎵 Audio — Waveform & Playback
**Formats:** MP3, FLAC, WAV, OGG, AAC, M4A, WMA, OPUS, AIFF

*Features: Waveform display, metadata, cover art*

### 🎬 Video — Metadata & Thumbnails
**Formats:** MP4, MKV, WebM, AVI, MOV, WMV, FLV

*Features: Metadata extraction, thumbnail preview*

### 📚 eBooks — Reader Mode
**Formats:** EPUB, MOBI, AZW, AZW3, FB2

*Features: Chapter navigation, TOC, cover display*

### 💻 Code — Syntax Highlighting
**200+ Languages:** Rust, Python, JavaScript, TypeScript, C/C++, Java, Go, Ruby, PHP, Swift, Kotlin, Scala, Haskell, Elixir, and more

*Features: Line numbers, syntax themes, search*

---

## 🛡️ Security Model

### Pure Rust — No Chrome, No WebKit
```
WHAT WE USE:                    WHAT WE DON'T USE:
─────────────────────────────   ─────────────────────────────
eframe/egui (Pure Rust GUI)     ❌ Chromium
winit (Pure Rust windowing)     ❌ WebKit
softbuffer (Pure Rust pixels)   ❌ WebView2
fontdue (Pure Rust fonts)       ❌ V8 JavaScript
html5ever (Pure Rust HTML)      ❌ Blink rendering
SassyScript (Our JS engine)     ❌ Google APIs/Analytics
```

### 4-Layer Sandbox
```
Layer 1: NETWORK SANDBOX      → All content quarantined in memory
Layer 2: RENDER SANDBOX       → SassyScript engine (no V8 exploits)
Layer 3: CONTENT SANDBOX      → Rust parsers (no codec vulns)
Layer 4: DOWNLOAD QUARANTINE  → 3 clicks + 5s wait to release
```

### Page Trust System
- **🔴 Untrusted** — Fresh page, sandboxed
- **🟠 Cautious** — 1 meaningful interaction
- **🟢 Trusted** — 3 meaningful interactions
- **🔵 Verified** — User explicitly trusts site

---

## ⌨️ Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+L` | Focus address bar |
| `Ctrl+T` | New tab |
| `Ctrl+W` | Close tab |
| `Ctrl+Tab` | Next tab |
| `Ctrl+Shift+Tab` | Previous tab |
| `Alt+Left` | Back |
| `Alt+Right` | Forward |
| `F5` | Refresh |
| `F11` | Fullscreen |
| `Ctrl+O` | Open file |
| `Ctrl+S` | Save |
| `Ctrl+P` | Print |
| `Ctrl+Z` | Undo |
| `Ctrl+Y` | Redo |
| `Ctrl+F` | Find |
| `+/-` | Zoom in/out |
| `Ctrl+0` | Reset zoom |

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           USER INTERFACE (egui)                              │
│  ┌──────────┐ ┌───────────┐ ┌─────────────┐ ┌──────────┐ ┌───────────────┐  │
│  │ Tab Bar  │ │ Address   │ │ Network     │ │ Trust    │ │ File Viewer   │  │
│  │ Sidebar  │ │ Bar       │ │ Activity    │ │ Indicator│ │ Panel         │  │
│  └──────────┘ └───────────┘ └─────────────┘ └──────────┘ └───────────────┘  │
├─────────────────────────────────────────────────────────────────────────────┤
│                        UNIVERSAL FILE HANDLER                                │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │ Auto-detect: 200+ formats by extension + magic bytes                   │ │
│  │ Route to: Image | PDF | Document | Spreadsheet | Chemical | Archive... │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────────────────┤
│                         SPECIALIZED VIEWERS                                  │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐  │
│  │ Image   │ │ PDF     │ │ Document│ │ Chemical│ │ Archive │ │ 3D Model│  │
│  │ Editor  │ │ Editor  │ │ Editor  │ │ Viewer  │ │ Manager │ │ Viewer  │  │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘ └─────────┘ └─────────┘  │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐              │
│  │ Audio   │ │ Video   │ │ Ebook   │ │ Font    │ │ Code    │              │
│  │ Player  │ │ Info    │ │ Reader  │ │ Preview │ │ Editor  │              │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘ └─────────┘              │
├─────────────────────────────────────────────────────────────────────────────┤
│                         BROWSER ENGINE (Pure Rust)                           │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────────────────────┐   │
│  │ HTML5    │ │ CSS      │ │ Layout   │ │ SassyScript JS Engine        │   │
│  │ Parser   │ │ Engine   │ │ Engine   │ │ (No JIT, No WASM, No V8)     │   │
│  └──────────┘ └──────────┘ └──────────┘ └──────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 🔧 Building

### Windows
```batch
# Quick build
cargo build --release

# Full build with MSI installer
BUILD-WINDOWS.bat

# PowerShell build (recommended)
.\BUILD-WINDOWS.ps1
```

### Linux/macOS
```bash
cargo build --release
./target/release/sassy-browser
```

### Requirements
- Rust 1.75+ (`rustup.rs`)
- WiX Toolset 3.x (optional, for MSI on Windows)

---

## 📱 Phone Sync

Control your desktop browser from your phone via Tailscale P2P.

1. Scan QR code in Settings > Phone Sync
2. Open web app on phone
3. Share tabs, URLs, and files between devices

No cloud. No accounts. Peer-to-peer encrypted.

---

## 🤖 Built-in AI (MCP)

4-agent system for intelligent assistance:
- **Coordinator** — Routes queries
- **Code** — Programming help
- **Research** — Information lookup  
- **Creative** — Writing assistance

Supports xAI Grok + Anthropic Claude APIs.
No Google. No OpenAI dependency.

---

## 📄 License

MIT License — Sassy Consulting LLC

---

## 🔐 Security Contact

Found a vulnerability? Email security@sassyconsultingllc.com

We take security seriously — that's the whole point of this browser.

---

## 💎 Why Sassy Browser?

Because you shouldn't need:
- **$504/year** to edit a photo
- **$2,600/year** to view a molecule
- **Google tracking** to browse the web
- **10 different apps** to open your files

**One browser. Every format. Zero subscriptions. Pure Rust.**

```
The disruptor is here.
```
