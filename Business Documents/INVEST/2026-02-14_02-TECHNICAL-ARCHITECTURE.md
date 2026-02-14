# Sassy Browser -- Technical Architecture

**Sassy Consulting LLC | February 2026**

---

## System Architecture

```
+------------------------------------------------------------------+
|                      SASSY BROWSER v2.1                           |
+------------------------------------------------------------------+
|                                                                  |
|  UI Layer (egui/eframe -- immediate mode, 60fps)                |
|  +----------+----------+----------+----------+----------+        |
|  |   Tabs   | Sidebar  | Network  |  Tools   |  Themes  |        |
|  | Drag-drop| Bookmarks|   Bar    | Console  | Light/   |        |
|  | 1555 LOC | History  | Real-time| REST     | Dark     |        |
|  |          | Protect  | 687 LOC  | JSON     | Custom   |        |
|  +----------+----------+----------+----------+----------+        |
|                                                                  |
+------------------------------------------------------------------+
|                                                                  |
|  Security Layer (4 Sandboxes + Active Defense)                  |
|  +-------------+-------------+-------------+-------------+      |
|  |    Page     |   Popup     |  Download   |   Network   |      |
|  |   Sandbox   |   Blocker   |  Quarantine |   Monitor   |      |
|  | Trust grad. | Smart filter| File verify | All traffic |      |
|  | 400 LOC    | 385 LOC    | 528 LOC    | 501 LOC    |      |
|  +------+------+------+------+------+------+------+------+      |
|         |             |             |             |              |
|  +------v-------------v-------------v-------------v------+      |
|  |  Detection (1079) + Poisoning (741) + Mimicry (948)   |      |
|  |  TLS Spoofing (694) + Stealth Tracker (297)           |      |
|  +-------------------------------------------------------+      |
|                                                                  |
+------------------------------------------------------------------+
|                                                                  |
|  Self-Healing Health Watchdog (691 LOC)                         |
|  12-second intervals | Auto-diagnosis | Healing actions          |
|  Score 0-100 | Trend tracking | Cooldown anti-flap              |
|                                                                  |
+------------------------------------------------------------------+
|                                                                  |
|  Browser Engine (Pure Rust -- no Chromium, no WebKit)           |
|  +----------+----------+----------+----------+----------+        |
|  |   DOM    |  Style   | Layout   |  Paint   | Render   |        |
|  |  753 LOC | 594 LOC  | 1490 LOC | 496 LOC  | 382 LOC  |        |
|  +----------+----------+----------+----------+----------+        |
|                                                                  |
|  JavaScript Engine (SassyScript)                                |
|  +----------+----------+----------+----------+                   |
|  |  Lexer   |  Parser  | Interp.  |  DOM     |                   |
|  |  487 LOC | 749 LOC  | 800 LOC  |  Bindings|                   |
|  +----------+----------+----------+----------+                   |
|                                                                  |
+------------------------------------------------------------------+
|                                                                  |
|  AI/MCP Layer (10,154 LOC total)                                |
|  Orchestrator + Server + Client + API + Protocol                 |
|  Bincode transport (not JSON) | WebSocket | Sandboxed FS         |
|                                                                  |
+------------------------------------------------------------------+
|                                                                  |
|  Data Layer (ALL LOCAL -- zero cloud, zero telemetry)           |
|  +-------------+-------------+-------------+-------------+      |
|  |  Password   |   History   |  Bookmarks  |    Sync     |      |
|  |   Vault     | 14.7s delay |   Local     | Tailscale   |      |
|  | ChaCha20    | NSFW detect |   Files     | P2P only    |      |
|  | 1060 LOC   | 629 LOC    | 330 LOC    | 1999 LOC   |      |
|  +-------------+-------------+-------------+-------------+      |
|                                                                  |
+------------------------------------------------------------------+
```

---

## Module Breakdown by Category

### Lines of Code by System

| System | Lines | Files | Description |
|--------|------:|------:|-------------|
| Core Application | 10,280 | 4 | Main app, engine, data, entry point |
| Browser Engine | 6,804 | 8 | DOM, CSS, layout, paint, render, script |
| JavaScript Engine | 3,425 | 7 | Lexer, parser, interpreter, DOM bindings |
| Security & Anti-Tracking | 3,759 | 5 | Detection, poisoning, mimicry, TLS spoof |
| Sandbox System | 1,748 | 5 | Page, popup, download, network sandboxes |
| Self-Healing | 691 | 1 | Health watchdog |
| AI/MCP System | 10,154 | 10 | Orchestrator, server, client, protocol |
| UI Components | 6,581 | 10 | Tabs, sidebar, popups, themes, input |
| File Viewers | 8,272 | 13 | PDF, docs, images, archives, 3D, chem |
| Sync System | 1,999 | 6 | P2P sync, family profiles, encryption |
| Data Management | 3,746 | 5 | Passwords, history, cookies, crypto |
| Additional | 19,537 | 37 | Voice, adblock, file handler, extensions |
| **Total** | **76,996** | **111** | |

### Public API Surface

| Category | Count |
|----------|------:|
| Public functions | 1,824 |
| Public structs | 407 |
| Public enums | 187 |
| Unit tests | 456 |
| Test modules | 69 |

---

## Security Architecture

### 4-Layer Defense

**Layer 1 -- Page Sandbox:** Per-domain trust gradient. New sites start at Red (no permissions). Trust builds through genuine user interaction (not scripts). Minimum 20x20px click targets prevent clickjacking.

**Layer 2 -- Popup Blocker:** Distinguishes user-initiated from script-initiated popups. Rate limited to 5 per 30 seconds. Smart filtering, not blanket blocking.

**Layer 3 -- Download Quarantine:** All downloads held in quarantine. File type verified by magic bytes (not extension). Executable analysis before release to filesystem.

**Layer 4 -- Network Monitor:** Every network request visible in real-time. No silent pings, no hidden beacon calls. Tracker blocking at the domain level.

### Active Anti-Tracking (Unique to Sassy)

| System | Lines | What It Does |
|--------|------:|--------------|
| Detection Engine | 1,079 | Honeypot elements detect fingerprinting attempts |
| Fingerprint Poisoning | 741 | Returns randomized canvas/audio/WebGL/font data |
| Behavioral Mimicry | 948 | Simulates human mouse/scroll/typing patterns (4 levels) |
| TLS Spoofing | 694 | ClientHello matches Chrome 132 exactly (JA3/JA4) |
| Stealth Tracker | 297 | Counts victories: ads blocked, trackers poisoned |

No other browser does all five. Brave does partial fingerprint blocking. Sassy poisons, mimics, and fights back.

### Cryptographic Stack

| Purpose | Algorithm | Notes |
|---------|-----------|-------|
| Passwords | ChaCha20-Poly1305 | IETF RFC 8439 |
| Key derivation | Argon2id | PHC competition winner |
| Identity | Ed25519 | RFC 8032 |
| Entropy | Mouse + timing | Seeded CSPRNG |
| TLS identity | Chrome 132 | Custom JA3/JA4 match |

---

## Self-Healing Watchdog

Runs every 12 seconds. Collects lightweight health snapshot (no I/O, no allocations). When health score drops below 85, diagnoses the issue and applies corrective action automatically.

**Health Score Formula:** Starts at 100, deducts for crashes (-15 each), stalls (-8 each), high memory (-10 to -20), too many tabs (-5 to -15), sandbox violations (-2 each), detection alerts (-3 each).

**Healing Actions:**

| Trigger | Action | Confidence |
|---------|--------|-----------|
| 3+ crashes | Close crashed tabs | 95% |
| Memory > 5.2GB | Clear HTTP cache | 90% |
| 35+ tabs | Suspend background tabs | 85% |
| 4+ stalls | Restart renderer | 80% |
| 5+ violations | Tighten sandbox | 85% |
| Tracking detected | Reapply poisoning | 80% |

60-second cooldown between actions prevents flapping. 30-snapshot rolling history enables trend detection.

---

## Performance Characteristics

| Metric | Sassy | Chrome |
|--------|-------|--------|
| Binary size | 12MB | 300MB+ |
| Memory per tab | <100MB est. | 500MB+ |
| Startup time | <1 second | 2-5 seconds |
| Compile time | 7-10 seconds (check) | N/A |
| Build from source | Yes (cargo build) | Requires Google infra |

---

## Dependency Philosophy

96 crate dependencies. Zero Google-owned crates. Pure Rust stack.

| Category | Crates |
|----------|--------|
| GUI | eframe, egui, egui_extras |
| Browser | html5ever, cssparser, selectors |
| Crypto | ring, chacha20poly1305, argon2, ed25519-dalek |
| Images | image, resvg, rawloader, psd |
| Documents | lopdf, pdf-extract, calamine, quick-xml, epub |
| 3D | obj-rs, stl_io, gltf, ply-rs |
| Audio | symphonia, rodio |
| Archives | tar, flate2, xz2, bzip2, zstd, sevenz-rust |
| Network | ureq, url, tokio-tungstenite |
| Fonts | fontdue (pure Rust, no FreeType) |

---

**Document Version:** 2.1 | February 14, 2026
**Sassy Consulting LLC**
