# SASSY BROWSER - DISRUPTOR ROADMAP
## Making People Line Up for This Browser

**Goal:** End-all-be-all disruptor that kills $15,000+/year in paid software dependencies

---

## CURRENT STATE (v2.0.0)
- ✅ Compiles (465 warnings)
- ✅ 81 Rust source files
- ✅ 200+ file format support
- ✅ Image editor with crop/resize/filters/export
- ✅ Document editor with rich text/styles/export
- ✅ PDF viewer with search/annotations
- ✅ Chemical/PDB viewer with 3D rotation
- ✅ Spreadsheet viewer (CSV export only)
- ✅ Extensions system (WebExtension compatible)
- ✅ Pure Rust browser engine (no Chrome)

---

## PHASE 1: COMPLETE VIEW/EDIT/SAVE/PRINT (Priority: HIGH)

### 1.1 Spreadsheet Editor (HIGHEST PRIORITY)
**Current:** View only, CSV export
**Needed:** Full XLSX/ODS save with formulas

```rust
// Add to viewers/spreadsheet.rs
- Cell editing with formula bar
- Multi-sheet support
- Basic formulas (SUM, AVG, COUNT, IF, VLOOKUP)
- XLSX export using calamine + simple_xlsx_writer
- ODS export using zip + xml
- Print with headers/gridlines
```

### 1.2 PDF Annotations Save
**Current:** Annotations in memory
**Needed:** Persist to PDF file

```rust
// Add to viewers/pdf.rs
- Use lopdf to write annotation objects
- Support highlight, note, underline, strikethrough
- Form filling (text fields, checkboxes)
- PDF merge/split functionality
```

### 1.3 System Print Dialogs
**Current:** TODO everywhere
**Needed:** Native print for all file types

```rust
// Create src/print.rs enhancements
- Windows: Use winapi PrintDlg
- Cross-platform: Generate PDF then system print
- Print preview with page layout
```

### 1.4 Archive Create/Modify
**Current:** View/extract only
**Needed:** Create ZIP, add to archive

```rust
// Add to viewers/archive.rs
- ZIP creation using zip crate
- Add/remove files from existing archives
- Compression level selection
```

---

## PHASE 2: DISRUPTOR FEATURES (Web3 + Everyday Users)

### 2.1 Built-in Ad Blocker (Native uBlock Origin Replacement)
**Kill:** uBlock Origin extension dependency

```rust
// Create src/adblock.rs
pub struct AdBlocker {
    filter_lists: Vec<FilterList>,  // EasyList, EasyPrivacy, etc.
    custom_rules: Vec<String>,
    stats: BlockStats,
}

impl AdBlocker {
    // Parse uBlock/ABP filter syntax
    pub fn should_block(&self, url: &str, request_type: RequestType) -> bool;
    
    // Cosmetic filtering (hide elements)
    pub fn get_cosmetic_filters(&self, domain: &str) -> Vec<String>;
    
    // Update filter lists
    pub fn update_lists(&mut self) -> Result<(), Error>;
}

Features:
- Parse EasyList, EasyPrivacy, Fanboy's Annoyances
- Cosmetic filtering (CSS element hiding)
- Custom filter rules
- Whitelist/exception management
- Per-site toggle
- Block counter in UI
```

### 2.2 User Script Manager (Tampermonkey Replacement)
**Kill:** Tampermonkey extension dependency

```rust
// Create src/userscripts.rs
pub struct UserScript {
    name: String,
    match_patterns: Vec<String>,
    code: String,
    run_at: RunAt,  // document-start, document-end, document-idle
    grant: Vec<GrantPermission>,  // GM_getValue, GM_xmlhttpRequest, etc.
}

pub struct UserScriptManager {
    scripts: Vec<UserScript>,
    storage: HashMap<String, Value>,
}

Features:
- Parse @metadata headers
- @match, @include, @exclude patterns
- GM_* API implementation:
  - GM_getValue, GM_setValue, GM_deleteValue
  - GM_xmlhttpRequest (fetch with CORS bypass)
  - GM_addStyle (inject CSS)
  - GM_registerMenuCommand
  - GM_notification
- Script editor with syntax highlighting
- Enable/disable per script
- Update checking
```

### 2.3 Web3 Learning Mode (Gentle Onboarding)
**Target:** Middle-aged users learning web3/crypto

```rust
// Create src/web3_assist.rs
pub struct Web3Assistant {
    explanations: HashMap<String, Explanation>,
    skill_level: SkillLevel,
    show_tooltips: bool,
}

Features:
- Detect web3 sites (dApps, DEXs, NFT marketplaces)
- Contextual tooltips explaining:
  - "Connect Wallet" - what this means
  - Gas fees - why and how much
  - Smart contracts - simplified explanation
  - NFTs - what you're actually buying
- "Explain This Page" button
- Scam detection warnings
- Glossary sidebar
- Progressive disclosure (more detail as skill increases)
```

### 2.4 Developer Tools Enhancement
**Current:** Basic console, REST client
**Needed:** Full Chrome DevTools replacement

```rust
// Enhance src/console.rs + create src/devtools/
- Network tab with request/response inspection
- Elements tab with DOM tree + CSS rules
- Sources tab with breakpoints
- Performance tab with timeline
- Application tab (localStorage, cookies, etc.)
- Lighthouse-style audits
```

---

## PHASE 3: EVERYDAY USER FEATURES

### 3.1 Simplified Mode
**Target:** Non-technical users

```rust
// Create src/ui/simple_mode.rs
Features:
- Large, clear icons
- Simplified settings (privacy slider instead of checkboxes)
- "Just Works" defaults
- No technical jargon
- Guided setup wizard
- "What does this do?" help bubbles
```

### 3.2 Reading Mode
**Kill:** Readability extension

```rust
// Enhance src/html_renderer.rs
Features:
- Extract article content
- Remove ads, sidebars, navigation
- Customizable fonts/sizes/themes
- Text-to-speech integration
- Estimated reading time
```

### 3.3 Screenshot/Capture Tool
**Kill:** Lightshot, Snipping Tool

```rust
// Create src/screenshot.rs
Features:
- Full page capture (scrolling)
- Selection capture
- Annotate with arrows, text, highlights
- Direct share/upload
- GIF recording
```

---

## PHASE 4: SCIENTIFIC/PROFESSIONAL

### 4.1 DICOM Viewer (Medical Imaging)
**Kill:** $10,000+ PACS viewers

```rust
// Create src/viewers/dicom.rs
Features:
- Parse DICOM format
- Windowing (contrast/brightness)
- Measurements
- Multi-slice viewing
- HIPAA-compliant local-only processing
```

### 4.2 CAD Viewer (DXF/DWG)
**Kill:** AutoCAD LT ($2,000/yr)

```rust
// Create src/viewers/cad.rs
Features:
- Parse DXF (ASCII and binary)
- Basic DWG support
- Layer visibility
- Measurement tools
- 2D pan/zoom
```

### 4.3 LaTeX/Math Renderer
**Kill:** Overleaf dependency

```rust
// Create src/viewers/latex.rs
Features:
- Render LaTeX equations
- Live preview
- Export to PNG/SVG/PDF
- Common symbol palette
```

---

## PHASE 5: MOBILE/SYNC

### 5.1 Enhanced Phone Sync
**Current:** Basic Tailscale P2P
**Needed:** Full feature parity

```rust
// Enhance src/sync/
Features:
- Tab sync (real-time)
- History sync
- Bookmark sync
- Password vault sync
- Reading list sync
- "Send to phone" for any file
```

### 5.2 PWA Phone App
**Current:** Basic HTML
**Needed:** Full native-like experience

```
// Enhance phone-app/
Features:
- Service worker for offline
- Push notifications
- Native-like navigation
- Camera for QR scanning
- Share target for receiving URLs
```

---

## WEBSITE UPDATES NEEDED

### Browser Landing Page (sassyconsultingllc.com/browser)

1. **Update Stats:**
   - Lines of Rust: 28,000+
   - File formats: 200+
   - Install size: actual current size

2. **Add Disruptor Calculator:**
   Interactive cost savings calculator showing:
   - Software they currently use
   - Annual cost
   - How Sassy Browser replaces it
   - Total savings (animated counter)

3. **Add File Format Browser:**
   Interactive grid showing all supported formats with:
   - View capability
   - Edit capability
   - Save capability
   - Print capability

4. **Add "Try Online" Demo:**
   WebAssembly version for testing in browser

5. **Soften Technical Messaging:**
   Current: "Built in Rust, 4 sandboxes"
   Better: "Built for people who just want the internet to work"

6. **Add Testimonial Section:**
   - "My mom finally stopped calling me for tech support"
   - "No more $50/month software subscriptions"
   - "I can open ANY file now"

7. **Add Comparison Table:**
   | Feature | Chrome | Brave | Sassy |
   |---------|--------|-------|-------|
   | Tracks you | ❌ | Kinda | ✅ Never |
   | Opens PDB files | ❌ | ❌ | ✅ |
   | Opens RAW photos | ❌ | ❌ | ✅ |
   | Built-in ad blocker | ❌ | ✅ | ✅ |
   | User scripts | ❌ | ❌ | ✅ |

---

## IMPLEMENTATION ORDER

### Week 1: Core Viewer Completion
1. Spreadsheet XLSX save
2. PDF annotation save  
3. System print dialogs
4. Archive creation

### Week 2: Ad Blocker & User Scripts
1. Ad blocker with EasyList
2. User script manager
3. UI integration

### Week 3: User Experience
1. Simplified mode
2. Reading mode
3. Screenshot tool
4. Web3 assistant tooltips

### Week 4: Website & Polish
1. Update landing page
2. Create online demo
3. Documentation
4. Reduce warnings to <30

---

## SUCCESS METRICS

- **Downloads:** 1,000+ first month
- **Reviews:** 4.5+ stars
- **Forum Activity:** Active community
- **Cost Savings Reported:** $50,000+ collective

---

*Last Updated: 2026-01-23*
*Version: 2.0.0-disruptor-planning*
