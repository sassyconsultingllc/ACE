# Sassy Browser v1.0.1

**The Security-First Web Browser**

```
  ╔═══════════════════════════════════════╗
  ║       Sassy Browser v1.0.1            ║
  ║  Pure Rust | SassyScript | Sandboxed  ║
  ╚═══════════════════════════════════════╝
```

## Why Sassy Browser?

Other browsers let websites do whatever they want. Sassy Browser makes websites **earn your trust**.

### The Problem with Chrome/Firefox

1. You visit a sketchy site
2. Page immediately requests notifications, clipboard access, location
3. You click "Allow" (muscle memory)
4. Malware installed, spam notifications forever

### How Sassy Browser Works

1. You visit a sketchy site
2. Page is **sandboxed** - can't access ANYTHING sensitive
3. Site must earn trust through **3 meaningful interactions**
4. Most malicious sites give up, they can't trick you fast enough

## Features

### 🛡️ Page Sandbox
Every page loads in a restricted sandbox. To unlock capabilities:
- **Click** on visible elements (20x20px minimum, anti-clickjacking)
- **Type** in form fields (3+ characters)
- **Scroll** to read content (500+ pixels, user-initiated)
- **Submit** a form

Need 3 different interactions, spaced 1+ seconds apart.

### 🚫 Smart Popup Blocker
Not a dumb "block all popups" approach:
- ✅ **Allows**: OAuth (Google, GitHub, etc), CAPTCHA, payments (Stripe, PayPal)
- ❌ **Blocks**: Spam, ads, drive-by popups, suspicious sizes

### 📶 Network Activity Bar
Always know what's happening:
- Visual indicator when browser is loading (XFCE style)
- Active connection count
- Bytes transferred
- Idle/busy state clear at a glance

### 🔒 Download Quarantine
Files can't just run:
- Held in memory, not filesystem
- 3 deliberate clicks to release
- 5 second forced wait
- Heuristic warnings shown

### 📱 Phone Sync
Control your desktop browser from your phone via Tailscale.

## Building

```bash
# Windows
cargo build --release
copy target\release\sassy-browser.exe C:\SassyBrowser\

# Or use the MSI installer
cd installer
build-msi.bat
```

## Usage

```
sassy-browser.exe [URL]

Keyboard Shortcuts:
  Ctrl+L        Focus address bar
  Ctrl+T        New tab
  Ctrl+W        Close tab
  Alt+Tab       Tab tile view
  Alt+Left      Back
  Alt+Right     Forward
  F5            Refresh
  F11           Fullscreen
```

## Architecture

```
┌─────────────────────────────────────────────────┐
│                 USER INTERFACE                   │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌────────┐ │
│  │ Tabs    │ │ Address │ │ Network │ │ Trust  │ │
│  │ Sidebar │ │   Bar   │ │   Bar   │ │ Dot    │ │
│  └─────────┘ └─────────┘ └─────────┘ └────────┘ │
├─────────────────────────────────────────────────┤
│                 SANDBOX LAYERS                   │
│  ┌─────────────────────────────────────────────┐ │
│  │ 1. Page Sandbox - Untrusted until 3 clicks  │ │
│  │ 2. Popup Handler - Smart allow/block        │ │
│  │ 3. Network Sandbox - Memory quarantine      │ │
│  │ 4. Download Quarantine - Wait + confirm     │ │
│  └─────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────┤
│                   ENGINES                        │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐        │
│  │ HTML5    │ │ CSS      │ │ Layout   │        │
│  │ Parser   │ │ Engine   │ │ Engine   │        │
│  └──────────┘ └──────────┘ └──────────┘        │
│  ┌──────────────────────────────────────┐      │
│  │         SassyScript JS Engine        │      │
│  │    (No JIT, No WASM, Pure Rust)      │      │
│  └──────────────────────────────────────┘      │
└─────────────────────────────────────────────────┘
```

## License

MIT License - Sassy Consulting LLC

## Security

Found a vulnerability? Email security@sassyconsultingllc.com

We take security seriously - that's the whole point of this browser.
