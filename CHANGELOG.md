# Changelog

## [1.0.1] - 2025-12-26

### Changed
- **Rust 2024 Edition** - Upgraded from 2021 (requires Rust 1.85+)
- Fixed winit 0.29 compatibility (removed 0.30 ApplicationHandler API)
- Fixed LayoutEngine and StyleEngine API usage
- Fixed font path resolution

### Fixed
- Compilation errors with StyledNode/HtmlParser references
- Duplicate module declarations in ui/mod.rs
- Layout bounds field access

### Security Audit Plan
- **Round 1**: OpenAI o1-pro - crypto.rs, sandbox/*.rs, quarantine.rs
- **Round 2**: Google Gemini 2.0 Pro - Full src/ directory holistic review
- **Round 3**: Compare findings, investigate conflicts

## [1.0.0] - 2025-12-25 🎄

### Added
- **Address Bar Input** - Click or Ctrl+L to type URLs, Enter to navigate
- **Link Clicking** - Click links to navigate, proper URL resolution
- **Page Sandbox** - Every page sandboxed until 3 meaningful interactions
- **Smart Popup Blocker** - Blocks spam, allows captchas/OAuth/payments
- **Network Activity Bar** - Always visible XFCE-style indicator
- **Trust Indicator** - Visual dot showing page trust level
- **Quarantine UI** - Download quarantine dialog with warnings
- **AI Helper** - XP-style "?" button (off by default), Easter eggs → Foodie Finder discounts
- **Cryptographic Identity** - Ed25519, ChaCha20-Poly1305, Argon2id
- **Phone Sync** - Tailscale mesh, no cloud servers
- **Family Profiles** - Adult/Teen/Kid with different sandbox parameters

### Security
- 4-layer sandbox architecture
- No JIT compilation (prevents spectre/meltdown class attacks)
- Pure Rust (memory safe, no buffer overflows)
- Pages must EARN trust through real user engagement

### Technical
- 17,267 lines of Rust
- SassyScript JS engine (74 tests passing)
- 12MB install size, 4 simultaneous sandboxes

## [0.4.0] - 2025-12-25
- Production GUI, first-run wizard, 4-layer sandbox, auto-update, family profiles

## [0.3.0] - 2025-12-24
- Tailscale phone sync, tab tile view, four-edge sidebars, theme system

## [0.2.0] - 2025-12-23
- Full UI chrome, tab management, basic styling

## [0.1.0] - 2025-12-22
- Initial release: SassyScript JS engine, HTML5 parsing, CSS styling, basic layout
