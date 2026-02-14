# Sassy Browser — Security Audit RFP Template

**Sassy Consulting LLC | February 2026**

---

## Request for Proposal: Security Assessment of Sassy Browser

### 1. Company Overview

Sassy Browser is a privacy-first web browser built entirely in Rust. The application is approximately 77,000 lines of code across 111 source files with 96 crate dependencies and zero Google-owned components. The browser handles sensitive user data including passwords (encrypted with ChaCha20-Poly1305 + Argon2id), browsing history, and family profile configurations — all stored exclusively on-device.

### 2. Scope of Assessment

#### 2.1 Core Components (Priority: Critical)

| Component | LOC (approx) | Description |
|-----------|:------------|-------------|
| Password Vault (crypto.rs) | 2,800 | ChaCha20-Poly1305 encryption, Argon2id key derivation, vault lock/unlock |
| Sandbox System (sandbox.rs) | 2,200 | 4-layer isolation: Page → Popup → Download → Network |
| Network Stack (network.rs) | 3,100 | HTTP client, TLS handling, certificate validation |
| Anti-Tracking (fingerprint_defense.rs, mimicry.rs, detection_engine.rs) | 6,500+ | Fingerprint poisoning, behavioral mimicry, tracker detection |
| Cookie Management (cookies.rs) | 1,800 | Cross-site tracking prevention, cookie isolation |

#### 2.2 Data Handling (Priority: High)

| Component | Description |
|-----------|-------------|
| Local storage encryption | All sensitive data encrypted at rest |
| Family profiles | Age-restricted browsing, NSFW detection |
| Bookmark/history sync | Encrypted sync between devices |
| Download quarantine | File scanning and isolation |

#### 2.3 Architecture Review (Priority: High)

| Area | Description |
|------|-------------|
| Dependency audit | 96 crate dependencies — supply chain risk assessment |
| Memory safety | Rust guarantees + any unsafe blocks |
| Process isolation | Tab sandboxing model |
| Update mechanism | Auto-update integrity and authentication |

#### 2.4 Out of Scope

- Third-party websites and their security
- Operating system vulnerabilities
- Physical security
- Social engineering

### 3. Assessment Types Requested

#### 3.1 Source Code Review
- Manual review of critical security components
- Automated static analysis (cargo-audit, cargo-deny, clippy security lints)
- Unsafe code block audit
- Cryptographic implementation review

#### 3.2 Dynamic Testing
- Fuzzing of input parsers (HTML, CSS, JavaScript, file formats)
- Network protocol testing (TLS, HTTP/2, WebSocket)
- Extension API security testing
- Sandbox escape attempts

#### 3.3 Threat Modeling
- STRIDE analysis of browser architecture
- Attack surface mapping
- Data flow analysis
- Trust boundary identification

#### 3.4 Dependency Analysis
- Known vulnerability scan of all 96 crates
- License compliance verification
- Supply chain risk assessment
- Transitive dependency analysis

### 4. Deliverables

1. **Executive Summary** — high-level findings for stakeholders
2. **Technical Report** — detailed vulnerability descriptions with:
   - CVSS v3.1 scoring
   - Proof-of-concept where applicable
   - Remediation recommendations with priority
   - Code-level fix suggestions
3. **Threat Model Document** — architecture-level security analysis
4. **Dependency Audit Report** — supply chain findings
5. **Retest Report** — verification of fixes (included in engagement)

### 5. Timeline

| Phase | Duration | Activities |
|-------|:---------|------------|
| Kickoff & Scoping | 1 week | Architecture review, codebase walkthrough, threat modeling |
| Source Code Review | 2–3 weeks | Manual review + automated scanning |
| Dynamic Testing | 1–2 weeks | Fuzzing, network testing, sandbox testing |
| Report Writing | 1 week | Findings consolidation, severity rating |
| Remediation Support | 1 week | Assist with fixes, answer questions |
| Retest | 1 week | Verify all critical/high findings are resolved |
| **Total** | **7–9 weeks** | |

### 6. Preferred Audit Firms

| Firm | Specialty | Est. Cost | Notes |
|------|-----------|:----------|-------|
| **Cure53** | Browser security, open source | $30K–$50K | Audited Mullvad, Brave, Tor Browser |
| **Trail of Bits** | Systems security, Rust expertise | $40K–$80K | Strong Rust/crypto background |
| **NCC Group** | Full-spectrum security | $50K–$100K | Enterprise-grade, browser experience |
| **Include Security** | Application security | $25K–$45K | Good value, thorough |
| **Doyensec** | Application security | $20K–$40K | Browser extension expertise |

### 7. Access Requirements

- Full source code access (private GitHub repository)
- Build environment setup assistance
- Architecture documentation
- Direct communication with lead developer (founder)
- Test environment with debug builds

### 8. Confidentiality

- All findings are confidential to Sassy Consulting LLC
- NDA required before source code access
- Public disclosure only with written permission
- Coordinated disclosure for any critical findings

### 9. Evaluation Criteria

| Criteria | Weight |
|----------|:------|
| Relevant browser/Rust security experience | 30% |
| Methodology and approach | 25% |
| Team qualifications and availability | 20% |
| Cost | 15% |
| Timeline flexibility | 10% |

### 10. Contact

Proposals should be submitted to: [founder email]
Deadline: [date]
Questions accepted until: [date - 1 week]

---

## Internal Notes (Do Not Include in Sent RFP)

### Budget Allocation
- $40,000 allocated in pre-seed budget for security audit
- Can negotiate payment schedule (50% upfront, 50% on delivery)
- Consider starting with Cure53 — best browser security track record

### Timing
- Schedule audit after first two engineering hires join
- Allow 2 weeks for codebase cleanup before auditors review
- Retest should align with v2.1 release timeline

### Priority Findings to Watch For
1. Cryptographic implementation flaws in password vault
2. Sandbox escape vectors
3. Memory safety issues in unsafe blocks
4. Supply chain vulnerabilities in dependencies
5. Anti-tracking bypasses that could compromise user privacy

---

**Document Version:** 1.0 | February 14, 2026
**Sassy Consulting LLC**
