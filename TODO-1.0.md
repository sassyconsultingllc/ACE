# Sassy Browser v1.0.0 - Release Checklist

## Status: READY FOR BUILD

### ✅ Completed Features (16,476 lines of Rust)

#### Core Browser
- [x] HTML5 parsing (html5ever)
- [x] CSS parsing and styling (cssparser)
- [x] Layout engine (block, inline, flex)
- [x] Paint system with text rendering (fontdue)
- [x] SassyScript JavaScript engine (74 tests passing)

#### User Interaction (NEW in 1.0)
- [x] Address bar input (click or Ctrl+L, type, Enter to navigate)
- [x] Link clicking with URL resolution
- [x] Scrolling (mouse wheel, arrow keys, Page Up/Down)
- [x] Cursor changes (pointer on links, text on inputs)
- [x] Hit testing on page content

#### Security (Core Feature)
- [x] Page sandbox - every page untrusted until 3 interactions
- [x] Smart popup blocker (allows captcha/OAuth, blocks spam)
- [x] Download quarantine with heuristics
- [x] 4-layer sandbox architecture

#### UI Chrome
- [x] Network activity bar (XFCE-style, always visible)
- [x] Trust indicator dot (red→orange→green→blue)
- [x] Tab list sidebar
- [x] Tab tile view (Alt+Tab)
- [x] Theme system (light/dark)

#### Phone Sync
- [x] Tailscale integration
- [x] User profiles with PIN
- [x] Family sharing (Google Circles style)

### 📦 To Create MSI Installer

1. **Install WiX Toolset** (if not installed)
   ```
   winget install WixToolset.WixToolset
   ```

2. **Create icon.ico** from assets/icons/icon.svg
   ```powershell
   # Using ImageMagick
   magick convert assets\icons\icon.svg -define icon:auto-resize=256,128,64,48,32,16 installer\icon.ico
   
   # Or online: https://convertio.co/svg-ico/
   ```

3. **Create LICENSE.rtf** from LICENSE
   ```
   Open LICENSE in WordPad, Save As RTF to installer\LICENSE.rtf
   ```

4. **Build MSI**
   ```batch
   cd installer
   build-msi.bat
   ```

### 🚀 Quick Start (No MSI)

```batch
:: Build
cargo build --release

:: Run
target\release\sassy-browser.exe

:: Or with URL
target\release\sassy-browser.exe https://example.com
```

### ⌨️ Keyboard Shortcuts

| Key | Action |
|-----|--------|
| Ctrl+L | Focus address bar |
| Ctrl+T | New tab |
| Ctrl+W | Close tab |
| Alt+Tab | Tab tile view |
| Alt+Left | Back |
| Alt+Right | Forward |
| F5 | Refresh |
| F11 | Fullscreen |
| Escape | Cancel/Close |

### 🔒 Security Model

**Page Trust Progression:**
1. Page loads → Sandboxed (can't access anything sensitive)
2. User scrolls 500+ pixels → Still sandboxed
3. User clicks a real element → 1/3 trust
4. User types in form → 2/3 trust  
5. User submits form → 3/3 = TRUSTED

**What's Blocked Until Trusted:**
- Clipboard access
- Download initiation
- Notification requests
- Popup creation
- Geolocation requests
- Camera/microphone
- Fullscreen requests
- Auto-playing audio

**Anti-Abuse Measures:**
- Elements must be 20x20px minimum (anti-clickjacking)
- Interactions must be 1+ second apart (anti-bot)
- Page must be 2+ seconds old for interactions to count
- Clicks on ad containers don't count

## v1.0.1 Worklog (Past / Present / Future)

### Past (done recently)
- Popup sandbox: gated popups by trust, queued blocked popups, auto-release when domain becomes trusted.
- AI help UX: toolbar "?" button, right-sidebar help pane with status; loads `config/ai.toml` on startup.
- AI providers: OpenAI/Anthropic/Ollama calls wired with error handling; shows last response/error in pane.
- JS DOM bridge: `execute_with_dom` now binds the live `Document`; selectors return real node data (id/tag/class/text/html snapshots).
- Quarantine: released files tagged as internet downloads (Windows Zone.Identifier, macOS quarantine xattr).

### Present (stabilize/verify)
- Run regression checks around AI calls (ensure network failures do not hang UI; consider async to avoid frame hitching).
- Validate DOM bridge correctness against real pages (selector coverage, performance, and safe pointer use).

### Future (planned / needed)
- **Make AI help non-blocking**: offload provider calls async and stream into pane; keep UI responsive.
- **DOM mutation support**: wire attribute setters/append/remove into our DOM model (or safely no-op with clear messaging) and refresh layout when mutated.
- **Event plumbing**: map addEventListener/dispatchEvent to our event model so scripted clicks/input can register as interactions safely.
- Persist AI runtime choices and last responses across sessions when enabled (respect privacy/offline defaults).
- Extend help queries (element-focused "What is this?" and "Is this safe?" from hovered URL) with throttling and sanitization.
