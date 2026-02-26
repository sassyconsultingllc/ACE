# Sassy Browser

**Web browser and universal file viewer — pure Rust — zero Google code**

[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://rustup.rs)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-blue.svg)]()

Sassy Browser is a web browser built from scratch in Rust. It opens, edits, saves, and prints 200+ file formats with no external paid dependencies, no Chromium, no V8, and no Google code of any kind.

The JavaScript engine is [Boa v0.21](https://boajs.dev/) — 94% ECMAScript test262 compliant, pure Rust, embeddable, MIT licensed. No JIT compiler means no JIT-spray attack surface.

---

## What it replaces

| Software | Annual Cost | What Sassy Browser does instead |
|----------|-------------|----------------------------------|
| Adobe Photoshop + Lightroom | $504/yr | Image editor — RAW, PSD, EXR, HEIC, crop, resize, filters, export |
| Adobe Acrobat Pro | $240/yr | PDF annotate, highlight, draw, form fill, merge, split, print |
| Microsoft 365 | $100/yr | DOCX/XLSX editor with formatting, styles, formulas |
| ChemDraw | $2,600/yr | Molecular viewer — PDB, MOL, SDF, 3D rendering |
| PACS Viewer | $10,000+/yr | DICOM medical imaging viewer |
| AutoCAD LT | $2,000/yr | CAD viewer — DXF, DWG, OBJ, STL, GLTF |
| Postman Pro | $144/yr | REST client built in |
| 1Password / LastPass | $36–60/yr | ChaCha20-Poly1305 vault, Argon2id KDF, your key |
| WinRAR | Nagware | ZIP, 7Z, RAR, TAR, GZ, XZ, BZ2, ZSTD |
| Calibre | — | EPUB, MOBI, AZW3 reader |
| Chrome / Edge / Brave | Your data | Pure Rust, zero Chromium, zero tracking |

---

## Quick start

```bash
# Build from source
cargo build --release

# Open a file
./target/release/sassy-browser document.pdf
./target/release/sassy-browser molecule.pdb
./target/release/sassy-browser photo.cr2

# Browse the web
./target/release/sassy-browser https://duckduckgo.com
```

---

## Supported formats (200+)

### Images — view, edit, save, print
**Standard:** PNG, JPG, GIF, WebP, BMP, ICO, TIFF, TGA, SVG, AVIF, HEIC
**RAW camera:** CR2, CR3, NEF, ARW, DNG, ORF, RW2, RAF, PEF, SRW, X3F
**Professional:** PSD, PSB, EXR, HDR, DDS, QOI

Crop, resize, rotate, flip, filters, adjustments. Export to PNG, JPEG, WebP, BMP, TIFF.

### Documents — view, edit, save, print
**Office:** DOCX, DOC, ODT, RTF, WPD
**PDF:** Annotations, form fill, merge/split, print
**Spreadsheet:** XLSX, XLS, ODS, CSV, TSV — multi-sheet, formulas

### Chemical and biological — 3D visualization
**Structures:** PDB, MOL, MOL2, SDF, XYZ, CIF, mmCIF, FASTA
**Medical:** DICOM

Ball-and-stick, wireframe, spacefill rendering. Color by element, chain, residue, B-factor.

### Archives
ZIP, RAR, 7Z, TAR, GZ, XZ, BZ2, ZSTD, JAR, WAR, APK, DEB, RPM

### 3D models
OBJ, STL, GLTF, GLB, PLY, FBX, DAE, 3DS — rotate, zoom, wireframe, materials

### Audio
MP3, FLAC, WAV, OGG, AAC, M4A, OPUS, AIFF — waveform display, metadata, cover art

### Video
MP4, MKV, WebM, AVI, MOV — metadata, thumbnail preview

### eBooks
EPUB, MOBI, AZW, AZW3, FB2 — chapter navigation, TOC

### Fonts
TTF, OTF, WOFF, WOFF2, EOT — glyph preview, character map

### Code — 200+ languages
Syntax highlighting and editing for Rust, Python, JavaScript, TypeScript, C/C++, Java, Go, Ruby, PHP, Swift, Kotlin, and 190+ more.

---

## Architecture

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
│                         BROWSER ENGINE (pure Rust)                           │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────────────────────┐   │
│  │ HTML5    │ │ CSS      │ │ Layout   │ │ Boa JS Engine v0.21          │   │
│  │ Parser   │ │ Engine   │ │ Engine   │ │ 94% ES2025 · pure Rust       │   │
│  └──────────┘ └──────────┘ └──────────┘ └──────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Dependencies — what we use and what we deliberately don't

```
WHAT WE USE                     WHAT WE DON'T USE
──────────────────────────────  ──────────────────────────────
eframe/egui  — pure Rust GUI    Chromium
winit        — windowing        WebKit / WebView2
softbuffer   — pixel buffer     V8 JavaScript engine
fontdue      — font rendering   Blink rendering engine
html5ever    — HTML parsing     Google APIs / Analytics
Boa v0.21    — JS engine        Any C++ browser engine
```

No JIT compiler. No JIT-spray attack surface. Boa runs a bytecode VM with an optimizer pass — deliberate security decision, not a performance limitation.

---

## Security model

### 4-layer sandbox

```
Layer 1: NETWORK SANDBOX      → all content quarantined in memory before render
Layer 2: RENDER SANDBOX       → Boa JS engine (no V8 exploits, no JIT spray)
Layer 3: CONTENT SANDBOX      → Rust parsers (memory-safe by construction)
Layer 4: DOWNLOAD QUARANTINE  → 3 confirmations + 5-second delay before filesystem access
```

### Page trust system

Pages start with zero permissions. Trust is earned through meaningful interactions, not assumed.

| Level | Trigger | Permissions unlocked |
|-------|---------|----------------------|
| Untrusted | Fresh page load | Sandboxed render only |
| Cautious | 1 meaningful interaction | Basic clipboard read |
| Trusted | 3 meaningful interactions | Standard permissions |
| Verified | Explicit user grant | Full permissions |

### Password vault

ChaCha20-Poly1305 encryption. Argon2id key derivation. The key is generated on your machine and never leaves it. Same cryptographic primitives as WireGuard and Signal.

### Network monitor

Every active connection is visible in the UI at all times — domain, bytes per second, protocol. No background connections without user awareness.

---

## Building

### Requirements
- Rust 1.75+ — install via [rustup.rs](https://rustup.rs)
- WiX Toolset 3.x — optional, Windows MSI installer only

### Windows
```batch
cargo build --release

# Full installer build
BUILD-WINDOWS.bat

# PowerShell
.\BUILD-WINDOWS.ps1
```

### Linux / macOS
```bash
cargo build --release
./target/release/sassy-browser
```

---

## Keyboard shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+L` | Focus address bar |
| `Ctrl+T` | New tab |
| `Ctrl+W` | Close tab |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Next / previous tab |
| `Alt+Left` / `Alt+Right` | Back / forward |
| `F5` | Refresh |
| `F11` | Fullscreen |
| `Ctrl+O` | Open file |
| `Ctrl+S` | Save |
| `Ctrl+P` | Print |
| `Ctrl+Z` / `Ctrl+Y` | Undo / redo |
| `Ctrl+F` | Find |
| `+` / `-` / `Ctrl+0` | Zoom in / out / reset |

---

## Phone sync

Tab and file sync between desktop and phone via Tailscale P2P. No cloud. No accounts.

1. Settings → Phone Sync → scan QR code
2. Open web app on phone
3. Share tabs, URLs, and files peer-to-peer — end-to-end encrypted

---

## Built-in AI (MCP)

Optional 4-agent system. Disabled by default, no data sent anywhere without explicit configuration.

- **Coordinator** — routes queries between agents
- **Code** — programming assistance
- **Research** — information retrieval
- **Creative** — writing assistance

Supports xAI Grok and Anthropic Claude APIs. No Google. No OpenAI dependency.

---

## Current state and known limitations

This is an honest accounting of what works and what doesn't.

**Complete:**
- Image viewing and editing — all listed formats
- PDF viewing, annotation, merge/split
- DOCX/XLSX viewing and basic editing
- Chemical/biological 3D viewing — PDB, MOL, SDF, DICOM
- Archive browsing and extraction
- 3D model viewing
- Code syntax highlighting — 200+ languages
- Built-in ad blocker (EasyList + EasyPrivacy)
- Password vault
- Network monitor
- Page trust/sandbox system

**In progress:**
- Round-trip edit/save for 3D models and chemical structures (viewing works, save-back in development)
- Cross-platform builds — currently Windows primary; Linux and macOS builds require platform-specific windowing work
- Full CSS compliance — layout engine handles most pages; complex CSS grid/flexbox edge cases ongoing
- Boa v0.21 integration — replacing internal interpreter with Boa for improved ES2025 compliance

**Known tradeoff:**
JavaScript-heavy web apps (Google Docs, Figma, Notion) run slower than Chromium-based browsers. Boa's bytecode VM is not a JIT compiler — this is intentional. Sassy Browser is built for everyday browsing and file work, not as a drop-in replacement for heavy web apps.

---

## Status

Active development. 77 commits. 5 branches. 1 release tag.

Crowdfunding campaign launching on Kickstarter and Indiegogo to fund cross-platform builds, independent security audit, and v1.0 release. Details at [sassyconsultingllc.com/browser](https://sassyconsultingllc.com/browser).

---

## License

MIT — Sassy Consulting LLC

---

## Security contact

Found a vulnerability? Email shane@sassyconsultingllc.com

Responsible disclosure appreciated. We will respond within 48 hours.