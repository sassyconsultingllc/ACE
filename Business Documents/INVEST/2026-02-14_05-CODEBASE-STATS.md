# Sassy Browser -- Codebase Statistics & Build Report

**Sassy Consulting LLC | February 14, 2026**
**Build verified at:** V:\sassy-browser-FIXED

---

## Build Status

```
cargo check: 0 errors, 386 warnings
cargo build --release: SUCCESS (12MB binary)
Dead code suppressions: 0 (all warnings are honest)
```

---

## Summary Statistics

| Metric | Value |
|--------|------:|
| Total lines of code | 76,996 |
| Source files (.rs) | 111 |
| Modules declared | 62 |
| Public functions | 1,824 |
| Public structs | 407 |
| Public enums | 187 |
| Unit tests | 456 |
| Test modules | 69 |
| Crate dependencies | 96 |
| Google-owned dependencies | 0 |
| Build errors | 0 |
| `#[allow(dead_code)]` annotations | 0 |

---

## Largest Files (Top 25)

| File | Lines | Purpose |
|------|------:|---------|
| `app.rs` | 7,079 | Main application state, UI, update loop |
| `file_handler.rs` | 3,137 | Universal file type detection (100+ formats) |
| `mcp_server.rs` | 2,856 | MCP server for Claude Desktop integration |
| `engine.rs` | 2,368 | Core browser state machine |
| `voice.rs` | 2,215 | Whisper STT, VAD, microphone (local only) |
| `mcp.rs` | 2,076 | MCP protocol orchestrator |
| `viewers/pdf.rs` | 1,464 | PDF viewer with search and navigation |
| `ui/tabs.rs` | 1,555 | Tab management with drag-drop |
| `viewers/document.rs` | 1,247 | DOCX, ODT, RTF, Markdown viewer |
| `mcp_api.rs` | 1,239 | AI API clients (xAI Grok, Anthropic Claude) |
| `html_renderer.rs` | 1,217 | HTML to egui rendering |
| `auth.rs` | 1,103 | Authentication and licensing |
| `viewers/archive.rs` | 1,082 | ZIP, TAR, 7Z, RAR, GZ viewer |
| `detection.rs` | 1,079 | Anti-tracking detection engine |
| `script_engine.rs` | 1,068 | Script execution environment |
| `password_vault.rs` | 1,060 | Encrypted password storage |
| `adblock.rs` | 1,056 | Ad blocking (EasyList/EasyPrivacy) |
| `userscripts.rs` | 1,002 | User script support |
| `mcp_fs.rs` | 962 | Sandboxed file system for AI |
| `viewers/image.rs` | 936 | Image viewer (RAW, PSD, HEIC, AVIF) |
| `family_profiles.rs` | 935 | Adult/Teen/Kid profile system |
| `console.rs` | 922 | Developer console |
| `json_viewer.rs` | 912 | JSON pretty-printer and tree navigator |
| `ui/render.rs` | 916 | Main rendering loop |
| `layout_engine.rs` | 896 | CSS layout engine (taffy-based) |

---

## Lines by System

| System | Lines | % of Total |
|--------|------:|-----------:|
| Core Application (app, engine, data, main) | 10,280 | 13.4% |
| AI/MCP Integration (10 files) | 10,154 | 13.2% |
| File Viewers (13 viewers) | 8,272 | 10.7% |
| Browser Engine (DOM, CSS, layout, paint, render) | 6,804 | 8.8% |
| UI Components (tabs, sidebar, popup, theme) | 6,581 | 8.5% |
| Security & Anti-Tracking (5 engines) | 3,759 | 4.9% |
| Data Management (vault, history, cookies, crypto) | 3,746 | 4.9% |
| JavaScript Engine (lexer, parser, interpreter) | 3,425 | 4.4% |
| Sync System (P2P, family, encryption) | 1,999 | 2.6% |
| Sandbox System (page, popup, download, network) | 1,748 | 2.3% |
| Self-Healing Watchdog | 691 | 0.9% |
| Other (voice, adblock, file handler, extensions, etc.) | 19,537 | 25.4% |

---

## File Viewer Coverage

| Category | Formats | Viewer File |
|----------|---------|-------------|
| PDF | PDF | `viewers/pdf.rs` (1,464 LOC) |
| Documents | DOCX, ODT, RTF, TXT, MD | `viewers/document.rs` (1,247 LOC) |
| Archives | ZIP, TAR, GZ, XZ, 7Z, RAR, BZ2, ZSTD | `viewers/archive.rs` (1,082 LOC) |
| Images | PNG, JPEG, GIF, WebP, BMP, RAW, PSD, HEIC, AVIF | `viewers/image.rs` (936 LOC) |
| Spreadsheets | XLSX, XLS, CSV, TSV, ODS | `viewers/spreadsheet.rs` (677 LOC) |
| Audio | MP3, WAV, FLAC, OGG, AAC, M4A | `viewers/audio.rs` (633 LOC) |
| Chemical | PDB, mmCIF, MOL, SDF, FASTA | `viewers/chemical.rs` (496 LOC) |
| Ebooks | EPUB, MOBI, AZW3, FB2 | `viewers/ebook.rs` (428 LOC) |
| 3D Models | OBJ, STL, GLTF, GLB, PLY, 3DS | `viewers/model3d.rs` (416 LOC) |
| Video | MP4, WebM, MKV, AVI | `viewers/video.rs` (374 LOC) |
| Fonts | TTF, OTF, WOFF, WOFF2 | `viewers/font.rs` (259 LOC) |
| Text/Code | 50+ languages | `viewers/text.rs` (202 LOC) |

---

## Security Module Coverage

| Module | Lines | Tests | Purpose |
|--------|------:|------:|---------|
| `detection.rs` | 1,079 | Yes | Honeypot detection, tracker pattern matching |
| `poisoning.rs` | 741 | Yes | Canvas, audio, WebGL, font fingerprint poisoning |
| `behavioral_mimicry.rs` | 432 | Yes | Mouse, scroll, typing humanization |
| `behavioral_mimicry_level4.rs` | 516 | Yes | Self-evolving behavioral models |
| `tls_spoof.rs` | 694 | Yes | Chrome 132 TLS ClientHello impersonation |
| `stealth_victories.rs` | 297 | Yes | Anti-tracking victory counters |
| `hunter_suite.rs` | 416 | Yes | Active threat hunting |
| `sandbox/page.rs` | 400 | Yes | Per-page isolation with trust gradient |
| `sandbox/popup.rs` | 385 | Yes | Smart popup filtering |
| `sandbox/quarantine.rs` | 528 | Yes | Download quarantine and scanning |
| `sandbox/network.rs` | 155 | Yes | Network request sandboxing |
| `adblock.rs` | 1,056 | Yes | EasyList/EasyPrivacy filter parsing |
| `health.rs` | 691 | Yes | Self-healing watchdog (16 tests) |

---

## Test Distribution

| Directory | Test Count | Files with Tests |
|-----------|----------:|----------------:|
| `src/*.rs` | 380 | 45 |
| `src/js/*.rs` | 30 | 4 |
| `src/ui/*.rs` | 15 | 5 |
| `src/viewers/*.rs` | 12 | 6 |
| `src/sandbox/*.rs` | 8 | 3 |
| `src/sync/*.rs` | 6 | 3 |
| `src/browser/*.rs` | 5 | 3 |
| **Total** | **456** | **69** |

---

## Warning Reduction Progress

| Date | Warnings | Notes |
|------|----------|-------|
| Jan 23 | 649 | Initial assessment |
| Feb 12 | 468 | First wiring pass |
| Feb 13 | 309 | Agent wiring (with suppressions) |
| Feb 13 | 69 | Suppressed (misleading) |
| Feb 14 | 336 | Honest count (all suppressions removed) |
| Feb 14 | 386 | After adding health.rs + test modules |
| Target | <30 | Production ready |

All `#[allow(dead_code)]` annotations have been removed. The 386 warnings are honest -- they represent code that exists but needs to be wired into the application flow or exercised by tests.

---

## Dependency Audit

**Zero Google-owned crates.** Full dependency list available in `Cargo.toml` (96 crates).

Key security-relevant dependencies:
- `ring 0.17` -- BoringSSL-derived crypto (audited)
- `chacha20poly1305 0.10` -- AEAD encryption (RustCrypto, audited)
- `argon2 0.5` -- Password hashing (RustCrypto)
- `ed25519-dalek 2.1` -- Digital signatures (audited)
- `rand 0.8` -- CSPRNG (audited)

No dependencies phone home. No dependencies require Google accounts. No dependencies include telemetry.

---

**Document Version:** 2.1 | February 14, 2026
**Sassy Consulting LLC**
