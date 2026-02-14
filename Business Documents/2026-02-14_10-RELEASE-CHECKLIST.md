# Sassy Browser — Release Checklist (v2.1)

**Sassy Consulting LLC | February 2026**

---

## Pre-Release: Code Quality

### Compilation & Warnings
- [ ] Zero compiler errors on all target platforms (Windows, macOS, Linux)
- [ ] Warnings reduced to under 30 (current: 386 — in progress)
- [ ] All `#[allow(dead_code)]` removed — warnings are honest
- [ ] `cargo clippy` passes with no denials
- [ ] `cargo fmt` applied to all source files
- [ ] No TODO/FIXME comments in shipped code (move to issue tracker)

### Testing
- [ ] All 456+ unit tests passing
- [ ] Integration test suite passing
- [ ] Manual QA checklist completed (see below)
- [ ] Fuzzing run completed on input parsers (HTML, CSS, file formats)
- [ ] Memory leak testing (Valgrind on Linux, Instruments on macOS)
- [ ] Performance benchmarks within acceptable thresholds

### Security
- [ ] `cargo audit` — zero known vulnerabilities
- [ ] `cargo deny` — all dependency licenses acceptable
- [ ] All unsafe blocks reviewed and documented
- [ ] Encryption implementation reviewed (password vault, sync)
- [ ] Sandbox escape testing completed
- [ ] Anti-tracking effectiveness validated against top 100 sites

### Dependencies
- [ ] All 96 crate dependencies at latest stable versions
- [ ] No yanked crates
- [ ] NOTICES file updated with all third-party licenses
- [ ] Dependency tree reviewed for supply chain risks

---

## Pre-Release: Build & Distribution

### Build Pipeline
- [ ] CI/CD pipeline building all platforms successfully
- [ ] Windows: .msi installer + portable .zip
- [ ] macOS: .dmg installer (Intel + Apple Silicon universal)
- [ ] Linux: .deb, .rpm, .AppImage, .tar.gz
- [ ] All builds signed with code signing certificates
- [ ] Reproducible builds verified (same source → same binary)

### Code Signing
- [ ] Windows: EV code signing certificate obtained ($400–600/year)
- [ ] macOS: Apple Developer certificate ($99/year)
- [ ] Linux: GPG key for package signing
- [ ] All certificates documented and backed up securely

### Auto-Update
- [ ] Update server configured and tested
- [ ] Update manifest signed and verified
- [ ] Rollback mechanism tested
- [ ] Staged rollout capability (1% → 10% → 50% → 100%)
- [ ] Update notification UX tested

---

## Pre-Release: Documentation

### User-Facing
- [ ] README.md updated with current features
- [ ] INSTALL.md updated with all platform instructions
- [ ] CHANGELOG.md updated with all changes since last release
- [ ] In-app help text reviewed and updated
- [ ] Keyboard shortcuts documentation
- [ ] Privacy policy published at website URL
- [ ] Terms of service published at website URL

### Developer-Facing
- [ ] API documentation (extension API, MCP endpoints)
- [ ] Architecture documentation
- [ ] Contributing guidelines (if open source)
- [ ] Build instructions for contributors

---

## Pre-Release: Legal & Compliance

### Intellectual Property
- [ ] All copyright headers present in source files
- [ ] LICENSE file in repository root
- [ ] Third-party license NOTICES file complete
- [ ] No GPL-licensed code in proprietary builds
- [ ] Trademark "Sassy Browser" — application filed or pending

### Privacy Compliance
- [ ] Privacy policy accurately reflects data handling
- [ ] COPPA compliance verified (family profiles serve minors)
- [ ] GDPR statement accurate (no EU data processing)
- [ ] CCPA statement accurate (no California consumer data)
- [ ] No analytics/telemetry code present (audit verified)

### Export Controls
- [ ] Encryption export classification determined
- [ ] BIS notification filed if required
- [ ] Country restriction list reviewed

---

## Release Day

### Deployment
- [ ] Git tag created: `v2.1.0`
- [ ] GitHub release created with binaries attached
- [ ] Website download page updated
- [ ] Auto-update manifest published
- [ ] Package manager submissions initiated (Homebrew, Chocolatey, etc.)

### Announcements
- [ ] Blog post published: "Sassy Browser v2.1 Release"
- [ ] Hacker News post: "Show HN: Sassy Browser v2.1"
- [ ] Reddit posts: r/privacy, r/rust, r/browsers
- [ ] Twitter/X announcement
- [ ] Product Hunt listing (if launch day)
- [ ] Newsletter sent to mailing list
- [ ] Press outreach to tech journalists

### Monitoring
- [ ] Crash reporting active and monitored
- [ ] Download statistics tracking
- [ ] Community channels monitored (Discord, GitHub Issues, Reddit)
- [ ] Error rate baselines established
- [ ] Performance metrics baselines established

---

## Post-Release (First 48 Hours)

### Immediate Response
- [ ] Monitor for critical bugs in community feedback
- [ ] Respond to GitHub issues within 4 hours
- [ ] Hotfix process ready if critical vulnerability found
- [ ] Community manager active in Discord/Reddit

### First Week
- [ ] Collect user feedback and categorize
- [ ] Identify top 5 user-reported issues
- [ ] Plan v2.1.1 patch release if needed
- [ ] Update investor stakeholders on launch metrics
- [ ] Internal retrospective on release process

---

## Manual QA Checklist

### Core Browsing
- [ ] Navigate to 20 popular websites (Google, YouTube, Reddit, etc.)
- [ ] Form submission works (login forms, search, contact forms)
- [ ] File download works (PDF, images, zip files)
- [ ] Bookmark add/edit/delete/organize
- [ ] History recording and search
- [ ] Tab management (open, close, reorder, pin, duplicate)
- [ ] Multiple windows
- [ ] Incognito/private browsing mode

### Privacy Features
- [ ] Ad blocking on top 10 ad-heavy sites
- [ ] Tracker blocking verified (check network requests)
- [ ] Fingerprint poisoning active (test on EFF Panopticlick)
- [ ] Cookie isolation between domains
- [ ] HTTPS upgrade working
- [ ] Download quarantine and scanning

### Password Vault
- [ ] Create new vault with master password
- [ ] Save credentials for a site
- [ ] Auto-fill credentials on revisit
- [ ] Edit saved credentials
- [ ] Delete saved credentials
- [ ] Export/import vault
- [ ] Lock vault on timeout
- [ ] Vault encryption verified (inspect storage file)

### Family Profiles
- [ ] Create Adult profile
- [ ] Create Teen profile (verify restrictions)
- [ ] Create Kid profile (verify restrictions)
- [ ] NSFW detection blocking
- [ ] Profile switching
- [ ] Per-profile bookmark and history isolation

### File Viewer
- [ ] PDF viewing
- [ ] Image viewing (PNG, JPG, GIF, WebP, SVG)
- [ ] Text file viewing with syntax highlighting
- [ ] Markdown rendering
- [ ] Video playback
- [ ] Audio playback

### Developer Tools
- [ ] MCP connection and commands
- [ ] Extension loading
- [ ] Console/DevTools equivalent
- [ ] Network request inspector

### Platform-Specific
- [ ] Windows: installer, uninstaller, default browser setting
- [ ] macOS: DMG mount, app bundle, Gatekeeper approval
- [ ] Linux: package install, desktop integration, file associations

---

## Emergency Procedures

### Critical Vulnerability Found Post-Release
1. Assess severity (CVSS score)
2. If CVSS > 7.0: begin hotfix immediately
3. Prepare patched release within 24 hours
4. Force-push auto-update (skip staged rollout)
5. Publish security advisory
6. Notify any affected users

### Catastrophic Bug (Data Loss, Crashes)
1. Pull download links if affecting >5% of users
2. Issue immediate communication on all channels
3. Hotfix and re-release within 12 hours
4. Offer manual fix instructions for affected users
5. Post-mortem within 1 week

---

**Document Version:** 1.0 | February 14, 2026
**Sassy Consulting LLC**
