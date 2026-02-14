# Sassy Browser -- Feature Deep Dives

**Sassy Consulting LLC | February 2026**

---

## Feature 1: Universal File Viewer (100+ Formats)

### The Problem

Opening a RAW photo requires Adobe Photoshop ($263/year). Viewing a PDB molecular structure requires ChemDraw ($2,600/year). Reading a DOCX file on a machine without Office requires a subscription ($100/year). These are basic file viewing operations.

### Our Solution

Sassy Browser opens 100+ file formats natively. No plugins. No installations. No subscriptions. Drop a file on the browser and it opens.

### Implementation

3,137 lines in `file_handler.rs` detect file types by magic bytes (not extension). 13 specialized viewers handle rendering:

**Images (28 formats):** Standard formats (PNG, JPEG, GIF, WebP, BMP) plus professional formats (CR2, NEF, ARW, DNG, PSD, HEIC, AVIF). Full-quality rendering with zoom and pan.

**Documents (15 formats):** DOCX parsing via `quick-xml` + `zip` crates. ODT via Open Document parsing. RTF via dedicated parser. PDF via `lopdf` + `pdf-extract`. Full text extraction and search.

**Spreadsheets (6 formats):** XLSX/XLS via `calamine` crate. CSV/TSV via streaming parser. Multi-sheet support with navigation.

**3D Models (8 formats):** OBJ via `obj-rs`, STL via `stl_io`, GLTF/GLB via `gltf` crate. Wireframe and solid rendering with rotation.

**Chemical Structures (8 formats):** PDB/mmCIF via `pdbtbx` crate. MOL/SDF via custom parser. Atom-level visualization with element coloring.

**Archives (10 formats):** ZIP, TAR, GZ, XZ, 7Z, RAR, BZ2, ZSTD. Browse contents without extracting. Preview files inside archives.

**Audio (8 formats):** MP3, WAV, FLAC, OGG, AAC via `symphonia` crate. Waveform visualization and playback via `rodio`.

**Video (6 formats):** MP4, WebM, MKV, AVI. Frame extraction and metadata display.

**Ebooks (4 formats):** EPUB via `epub` crate. MOBI/AZW3 via custom parser. Paginated reading with font customization.

**Fonts (4 formats):** TTF, OTF, WOFF, WOFF2 via `fontdue`. Glyph preview with sample text rendering.

---

## Feature 2: Self-Healing Health Watchdog

### The Problem

Browsers degrade over time. Tabs crash. Memory leaks accumulate. Renderers stall. Users restart their browser when things get slow, losing their session state.

### Our Solution

A health watchdog runs every 12 seconds, collects a lightweight snapshot (no I/O, no allocations), scores browser health 0-100, and automatically applies corrective actions when problems are detected.

### How It Works

**Health Score Calculation:**
```
Start at 100
  -15 per crashed tab (last 5 min)
  -8 per renderer stall (last 5 min)
  -20 if memory > 4GB, -10 if > 2GB
  -15 if tabs > 50, -5 if > 30
  -2 per sandbox violation (max -20)
  -3 per detection alert (max -10)
Clamp to 0-100
```

**Automatic Healing:**
When score drops below 85, the watchdog diagnoses the top issue and applies the appropriate fix:
- Close crashed tabs
- Clear HTTP cache
- Suspend background tabs
- Restart render pipeline
- Tighten sandbox restrictions
- Reapply fingerprint poisoning
- Force script garbage collection

**Anti-Flap Protection:**
60-second cooldown between actions. 65% confidence threshold required. 30-snapshot rolling history enables trend detection (improving / stable / degrading).

### Why This Matters

No other browser does this. Chrome, Firefox, and Brave all degrade until the user manually restarts. Sassy stays healthy automatically. This is especially valuable for non-technical users who don't know why their browser is slow.

---

## Feature 3: Active Anti-Tracking Warfare

### The Problem

Traditional tracker blocking has a fatal flaw: it's detectable. When a site's tracking script gets blocked, the tracker knows you're blocking it. Some trackers specifically detect ad blockers and serve different content, nag screens, or paywalls.

### Our Solution

Let the trackers run. Feed them poisoned data. They think they have a valid fingerprint -- but it's fake. Their datasets become unreliable at scale.

### Five Defense Layers

**Layer 1 -- Detection Engine (1,079 LOC):**
Honeypot elements injected into pages. Only fingerprinting scripts access invisible/off-screen elements. When triggered, the tracker is identified and flagged.

**Layer 2 -- Fingerprint Poisoning (741 LOC):**
Every fingerprinting API returns subtly randomized data. Canvas returns have pixel-level noise. Audio API has frequency injection. WebGL reports a fake renderer. Font enumeration returns a randomized subset. Per-domain consistency means sites still work -- but can't correlate across domains.

**Layer 3 -- Behavioral Mimicry (948 LOC, 4 levels):**
Mouse movements get human-like jitter and acceleration curves. Scroll patterns include realistic deceleration. Typing has natural speed variation and pauses. Level 4 models self-evolve when detectors update.

**Layer 4 -- TLS Spoofing (694 LOC):**
Every TLS handshake matches Chrome 132 exactly. Cipher suites, extensions, ordering, GREASE values -- all identical. JA3 and JA4 fingerprints match genuine Chrome. No server or CDN can distinguish Sassy from Chrome by its TLS signature.

**Layer 5 -- Stealth Victory Tracking (297 LOC):**
Silent counters track every blocked ad, stopped tracker, poisoned fingerprint, caught honeypot trigger, and deployed entropy bomb. Shown in the Protection panel so users see their browser fighting for them.

### Why Five Layers

Each layer addresses a different tracking vector. Blocking alone (what ad blockers do) is one dimension. Poisoning, mimicry, TLS spoofing, and detection are four additional dimensions that no other browser implements.

---

## Feature 4: Family Safety Profiles

### The Problem

Parents want safe browsing for their children but current solutions are either too complex (Google Family Link requires a Google account and separate app), too expensive (third-party parental controls), or too easy to bypass.

### Our Solution

Family profiles are built into the browser itself. No separate app. No cloud account. No subscription. Three profile types with age-appropriate defaults.

### Profile Types

**Adult Profile:**
- Full browsing with 14.7-second history delay
- NSFW content auto-excluded from history (not blocked)
- Full download access after trust gradient
- All security features active

**Teen Profile:**
- 5-second history delay
- NSFW content blocked and logged for parent review
- Downloads require trust + logged
- Higher trust threshold (4 interactions vs 3)
- Time limits configurable by parent

**Kid Profile:**
- Immediate history recording (full transparency)
- NSFW content hard blocked
- Downloads require parent approval
- Highest trust threshold (5 interactions + parent override)
- Allowlist mode available (only approved sites)

### Smart NSFW Detection

URL patterns analyzed locally (no external API calls). Detection runs against pattern databases stored on device. Classified content is:
- Excluded from history (Adult profile)
- Blocked and logged (Teen profile)
- Hard blocked (Kid profile)
- Never synced to other devices
- Never shown in autocomplete

### Why Built-In Matters

External parental controls run as system services or browser extensions. Tech-savvy teens can disable them, use a different browser, or use incognito mode. Sassy's family profiles are part of the browser itself -- there's no workaround except using a different browser entirely.

---

## Feature 5: MCP/AI Integration

### The Problem

Browser-based AI assistants (Chrome's Gemini, Edge's Copilot, Brave's Leo) are locked to a single AI vendor. They send browsing data to cloud APIs. They can't be extended or customized.

### Our Solution

Sassy implements the Model Context Protocol (MCP), an open standard for AI integration. It works as both an MCP client (connecting to AI services) and an MCP server (exposing browser capabilities to AI assistants like Claude Desktop).

### Architecture (10,154 lines total)

**MCP Orchestrator (2,076 LOC):** Routes tasks to appropriate AI agents. Supports multiple providers simultaneously (xAI Grok for fast tasks, Anthropic Claude for complex reasoning, local models for offline fallback).

**MCP Server (2,856 LOC):** Exposes browser capabilities as MCP tools. Claude Desktop (or any MCP client) can browse the web, extract text, take screenshots, fill forms, manage bookmarks, and access the sandboxed file system -- all through Sassy's security layer.

**Sandboxed File System (962 LOC):** AI operations get a restricted file system view. No access to system files. Approval workflow for writes. All operations logged and auditable.

**Native Protocol (542 LOC):** Uses bincode (binary) instead of JSON for performance. Length-prefixed framing via tokio-util codec. WebSocket transport via tokio-tungstenite.

### Why MCP Matters

MCP is an open standard backed by Anthropic. It's becoming the standard way AI assistants interact with tools. By implementing MCP natively, Sassy becomes part of the AI ecosystem -- not just another browser with a chat sidebar.

---

**Document Version:** 2.1 | February 14, 2026
**Sassy Consulting LLC**
