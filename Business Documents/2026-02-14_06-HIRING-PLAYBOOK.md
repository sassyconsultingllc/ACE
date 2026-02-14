# Sassy Browser — Hiring Playbook

**Sassy Consulting LLC | February 2026**

---

## Hiring Philosophy

Build a small, elite team. Every engineer should be able to own entire subsystems. No management layers until 10+ engineers. Remote-first, async-first, timezone-flexible.

---

## Immediate Hires (Pre-Seed, Months 1–6)

### Hire #1: Senior Rust Engineer — Browser Core

**Role:** Co-architect the rendering pipeline and network stack

**Requirements:**
- 3+ years Rust (production, not hobby)
- Experience with: async runtimes (tokio), FFI, unsafe code, memory management
- Bonus: Chromium/Servo/WebKit contribution history
- Bonus: experience with egui, wgpu, or other Rust GUI frameworks
- Must be comfortable with a 77K LOC codebase with active development

**Responsibilities:**
- Web rendering pipeline (HTML/CSS parsing → layout → paint)
- JavaScript engine integration (V8 or SpiderMonkey bindings)
- Network stack optimization (HTTP/2, HTTP/3, connection pooling)
- Performance profiling and optimization
- Code review and architecture decisions

**Compensation:**
- Salary: $140,000–$180,000 (remote, any US timezone)
- Equity: 2–4% (4-year vest, 1-year cliff)
- Benefits: health insurance, equipment budget, conference budget

**Where to Find:**
- Rust community (This Week in Rust job board, Rust Discord, r/rust)
- Servo project contributors
- Systems programming communities
- RustConf networking
- GitHub: search for Rust contributors with browser/networking projects

### Hire #2: Rust Engineer — Security & Privacy

**Role:** Own the anti-tracking, sandbox, and encryption subsystems

**Requirements:**
- 2+ years Rust
- Security background (penetration testing, threat modeling, or security research)
- Cryptography experience (TLS, symmetric/asymmetric encryption)
- Understanding of browser security model (same-origin policy, CSP, CORS)
- Bonus: experience with fingerprinting research or anti-tracking

**Responsibilities:**
- Anti-tracking engine development and evasion research
- Sandbox hardening and process isolation
- Password vault and encryption improvements
- Security audit preparation and remediation
- Vulnerability response process

**Compensation:**
- Salary: $130,000–$170,000
- Equity: 1.5–3%
- Same benefits as Hire #1

**Where to Find:**
- Security conferences (DEF CON, Black Hat, BSides)
- Bug bounty community
- Tor Project contributors
- Privacy-focused open source projects
- InfoSec Twitter/Mastodon

---

## Phase 2 Hires (Post-Seed, Months 7–12)

### Hire #3: Frontend/UI Engineer (Rust + Web)
- egui expertise or willingness to learn
- Extension API and web standards implementation
- Developer tools UI

### Hire #4: DevOps / Build Engineer
- CI/CD pipeline (cross-platform builds: Windows, macOS, Linux)
- Auto-update infrastructure
- Crash reporting and telemetry-free analytics
- Release engineering

### Hire #5: Community / Developer Relations
- Technical writing (documentation, blog posts, tutorials)
- Community management (Discord, GitHub, forums)
- Developer outreach and partnership
- Can double as first marketing hire

---

## Interview Process

### Stage 1: Resume + GitHub Review (Async, 30 min)
- Look for Rust projects, systems programming experience
- Check open source contributions
- Read their code, not just their resume

### Stage 2: Technical Screen (Video, 45 min)
- Discuss their Rust experience and projects
- Architecture discussion: "How would you design X in Sassy Browser?"
- No leetcode. Real problems from our codebase.

### Stage 3: Paid Take-Home Project (4–8 hours, $500 stipend)
- Pick a real issue from our backlog
- Example: "Implement connection pooling for our HTTP client"
- Example: "Add a new file viewer for SVG files"
- Evaluated on: code quality, testing, documentation, Rust idioms

### Stage 4: Team Interview (Video, 60 min)
- Pair programming session on a real feature
- Culture fit: async communication, ownership mentality, privacy values
- Q&A about the company, product, and vision

### Decision: 48 hours after final interview

---

## Sourcing Strategy

### Where Rust Engineers Hang Out
1. **Rust Discord** — #jobs channel
2. **This Week in Rust** — job board ($100/week)
3. **r/rust** — monthly job thread
4. **Rust Users Forum** — jobs category
5. **RustConf** — annual conference, great for networking
6. **GitHub** — search for active Rust contributors
7. **Zulip** — Rust project chat

### Where Security Engineers Hang Out
1. **DEF CON / Black Hat** — recruiting events
2. **BSides** — local security conferences
3. **InfoSec Twitter/Mastodon** — direct outreach
4. **Bugcrowd / HackerOne** — top researchers
5. **SANS** — security training community

### Outreach Templates

**Cold GitHub Outreach:**
> Hi [name], I noticed your work on [project] — really impressive [specific thing]. I'm building a privacy-first browser in Rust (77K LOC, solo so far) and looking for my first engineering hire. Would you be open to a 15-minute chat about the project? No pressure, happy to share the codebase either way.

**Job Board Post:**
> **Sassy Browser** is hiring Rust engineers to build the privacy-first browser. 77K lines of Rust, zero Google dependencies, zero telemetry. You'd be engineer #2 or #3, working directly with the founder. Remote, competitive salary + meaningful equity. We believe browsers should work for users, not advertisers.

---

## Compensation Philosophy

### Principles
1. **Pay market rate** — we compete on mission and equity, not by underpaying
2. **Transparent bands** — everyone knows the ranges
3. **Equity is real** — meaningful percentages, not 0.01% token grants
4. **No negotiation games** — we make our best offer first

### Salary Bands (2026)

| Level | Title | Salary Range | Equity Range |
|-------|-------|:------------|:------------|
| L3 | Engineer | $120K–$150K | 0.5–1.5% |
| L4 | Senior Engineer | $140K–$180K | 1.5–4.0% |
| L5 | Staff Engineer | $170K–$210K | 2.0–5.0% |

### Benefits (All Employees)
- Health/dental/vision insurance (company covers 80%)
- $3,000/year equipment budget
- $2,000/year conference/learning budget
- Unlimited PTO (with 3-week minimum)
- Flexible hours (async-first, core hours 11am–3pm ET overlap)

---

## Culture & Values

### What We Look For
1. **Ownership mentality** — you own subsystems, not tickets
2. **Privacy conviction** — you believe in what we're building
3. **Technical depth** — you can debug a TLS handshake or a memory leak
4. **Communication** — clear writing, async updates, ask questions early
5. **Pragmatism** — ship working code, iterate, don't over-engineer

### What We Don't Care About
- Degree or pedigree
- Years of experience (if the code is good)
- Cover letters
- Whiteboard algorithm performance

---

## Legal Requirements Per Hire

- [ ] Signed employment agreement
- [ ] Invention Assignment Agreement (all IP belongs to company)
- [ ] Confidentiality / NDA
- [ ] Equity agreement (stock options or LLC units)
- [ ] I-9 verification
- [ ] W-4 form
- [ ] State-specific requirements (varies)
- [ ] Benefits enrollment
- [ ] Equipment provisioning

---

**Document Version:** 1.0 | February 14, 2026
**Sassy Consulting LLC**
