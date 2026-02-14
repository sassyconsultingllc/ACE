# Sassy Browser -- Privacy & Security Whitepaper

**Sassy Consulting LLC | February 2026**

---

## Premise

Privacy in modern browsers is a lie. Chrome collects browsing history, search queries, and behavioral data. Brave claims privacy but still runs Chromium code with Google's infrastructure baked in. Edge sends data to Microsoft. Even Firefox has telemetry enabled by default.

Sassy Browser takes a different approach: **there is no telemetry code to disable.** Privacy isn't a setting -- it's an architectural constraint.

---

## Data Residency: Everything Stays Local

### What is stored (on device only)

| Data Type | Storage Method | Encryption |
|-----------|---------------|------------|
| Passwords | Local file | ChaCha20-Poly1305 + Argon2id |
| Browsing history | Local database | Optional encryption |
| Bookmarks | Local file | Plaintext (user choice) |
| Settings | Local file | Encrypted alongside passwords |
| Cookies | Local database | Per-session isolation |
| Download history | Local database | Plaintext |
| Extension data | Sandboxed local storage | Per-extension isolation |
| AI conversation history | In-memory only | Not persisted |
| Health watchdog data | In-memory only | Not persisted |
| Protection statistics | Local counters | Plaintext |

### What is transmitted (nothing)

- Zero telemetry endpoints
- Zero crash report collection
- Zero usage analytics
- Zero heartbeat/ping servers
- Zero Google API calls
- Zero Microsoft API calls
- Zero Facebook/Meta API calls
- Zero update checks to third parties

The only network traffic is user-initiated browsing and optional peer-to-peer sync via Tailscale (direct device-to-device, no cloud intermediary).

---

## Password Vault Architecture

### Encryption Pipeline

```
User Master Password
        |
        v
  Argon2id KDF (time=3, memory=65536KB, parallelism=4)
        |
        v
  256-bit Derived Key
        |
        v
  ChaCha20-Poly1305 AEAD
        |
        v
  Encrypted Vault File (local disk)
```

### Why These Algorithms

**Argon2id** -- Winner of the Password Hashing Competition (PHC). Memory-hard function that resists GPU cracking. The `id` variant combines resistance to both side-channel attacks (Argon2i) and GPU parallelism (Argon2d).

**ChaCha20-Poly1305** -- IETF RFC 8439. Chosen over AES-GCM because ChaCha20 is constant-time on all architectures (no timing side channels without AES-NI hardware). Used by Google's QUIC protocol, WireGuard, and Signal.

**Ed25519** -- RFC 8032. Used for device identity and sync authentication. Deterministic signatures prevent nonce-reuse vulnerabilities.

### What We Don't Do

- No cloud backup of passwords (ever)
- No password sharing with the browser vendor
- No password strength data sent anywhere
- No breach-check API calls (no Have I Been Pwned integration that sends hashes)
- No clipboard monitoring of passwords

---

## Smart History Design

### 14.7-Second Delay

Standard browsers record URLs instantly. This means accidental clicks, redirect chains, and mis-typed URLs all pollute history. Sassy waits 14.7 seconds before recording -- if you bounce before then, it's as if you never went there.

### NSFW Auto-Detection

URLs are analyzed locally (no external API) against pattern databases. NSFW content is automatically excluded from:
- History (never recorded)
- Sync (never transmitted to other devices)
- Family profile visibility (hidden from child accounts)
- Autocomplete suggestions (never suggested)

### Family Profile Interaction

| Profile | History Delay | NSFW | Download Policy |
|---------|--------------|------|-----------------|
| Adult | 14.7 seconds | Hide from history | After trust level |
| Teen | 5 seconds | Block + log for parent | Trust + parental log |
| Kid | Immediate | Hard block | Parent approval required |

---

## Anti-Tracking Architecture

### Why Blocking Isn't Enough

Traditional ad blockers and tracker blockers simply prevent network requests. This creates a detectable signal -- trackers know you're blocking them (the request pattern changes). Some trackers specifically detect ad blockers and serve different content.

Sassy takes the opposite approach: **let the trackers run, but feed them poisoned data.**

### Fingerprint Poisoning (741 lines of code)

Every fingerprinting vector returns randomized data:

| Vector | What Trackers See | Reality |
|--------|------------------|---------|
| Canvas fingerprint | Unique but wrong hash | Subtle pixel noise added |
| Audio fingerprint | Unique but wrong hash | Frequency noise injected |
| WebGL renderer | "ANGLE (NVIDIA GeForce...)" | Randomized from pool |
| Font enumeration | 47 fonts installed | Randomized subset |
| Screen resolution | 1920x1080 | Actual may differ |
| Timezone | UTC-5 | May differ |

**Key design:** Poisoning is consistent per-domain. So a single site gets the same fake fingerprint every visit (things still work), but two different sites get different fake fingerprints (they can't correlate across domains).

### Behavioral Mimicry (948 lines, 4 levels)

Bot detection systems analyze mouse movements, scroll patterns, and typing cadence to identify automated browsers. Sassy injects human-like noise:

- **Level 1:** Random jitter on mouse coordinates
- **Level 2:** Realistic cursor acceleration curves (ease-in/ease-out)
- **Level 3:** Humanized typing with natural speed variation and occasional pauses
- **Level 4:** Self-evolving models that adapt when detectors update their heuristics

### TLS ClientHello Spoofing (694 lines)

Every TLS connection has a fingerprint (JA3/JA4 hash) based on the cipher suites, extensions, and ordering in the ClientHello message. Sassy's TLS fingerprint is identical to Chrome 132. To any server or CDN, the connection looks like it came from a standard Chrome browser.

### Detection Engine with Honeypots (1,079 lines)

Sassy injects invisible elements into pages that only fingerprinting scripts would access. When a script reads a honeypot element, it triggers an alert and the tracker is flagged. This provides real-time visibility into which sites are actively trying to fingerprint you.

---

## Sandbox Architecture

### Trust Gradient (Not Permission Popups)

Standard browsers ask for permissions via popup dialogs. Users learn to click "Allow" reflexively. This is security theater.

Sassy uses a trust gradient. Permissions are earned through genuine user interaction over time:

```
Visit 1:  RED    -- No permissions at all
Visit 2:  ORANGE -- Read-only clipboard
Visit 3:  YELLOW -- Downloads with confirmation
Visit 4+: GREEN  -- Full permissions (still requires explicit grants)
```

Trust is per-domain and cannot be accelerated by scripts. The minimum interaction interval is 1 second (prevents bot-driven trust escalation).

### Download Quarantine

Every download is held in quarantine:

1. File type verified by magic bytes (not extension)
2. Executable files flagged for additional review
3. Archive contents listed before extraction
4. Files released to filesystem only after user confirmation

### Network Transparency

The network monitor shows every single request the browser makes. Unlike Chrome (which silently sends pings to Google, pre-fetches pages, and checks Safe Browsing), Sassy makes zero requests that the user didn't initiate.

---

## Sync Architecture

### Peer-to-Peer Only

Sassy uses Tailscale's mesh VPN for device-to-device sync. There is no cloud server. Data travels directly from one device to another over an encrypted WireGuard tunnel.

### What Syncs

- Passwords (re-encrypted for transport)
- Bookmarks
- History (respecting NSFW exclusions)
- Settings
- Family profile configurations

### What Does Not Sync

- Cookies (per-device)
- Cache (per-device)
- Extension data (per-device)
- AI conversation history (in-memory only)

---

## Compliance Considerations

| Standard | Relevance |
|----------|-----------|
| GDPR | No data collection = no GDPR obligations |
| CCPA | No data sale = no CCPA obligations |
| HIPAA | Local-only processing suitable for medical data viewing |
| SOC2 | No cloud infrastructure to audit |
| COPPA | Family profiles with parental controls built in |

---

## Threat Model

### What We Protect Against

| Threat | Protection |
|--------|-----------|
| Mass surveillance | Zero telemetry, encrypted local storage |
| Tracker correlation | Fingerprint poisoning (per-domain) |
| Bot detection | 4-level behavioral mimicry |
| TLS fingerprinting | Chrome 132 ClientHello spoofing |
| Drive-by downloads | 4-stage download quarantine |
| Clickjacking | 20x20px minimum click targets |
| Permission fishing | Trust gradient (no first-visit permissions) |
| Data exfiltration | Network monitor, no hidden traffic |
| Password theft | ChaCha20-Poly1305 + Argon2id vault |
| Browser fingerprinting | Active poisoning on all vectors |

### What We Don't Protect Against

| Threat | Why Not |
|--------|---------|
| Nation-state targeted attacks | Requires Tor-level anonymity |
| Physical device access | Requires full-disk encryption (OS level) |
| Keyloggers | Requires OS-level protection |
| Malicious extensions | Extension system is sandboxed but not fully hardened yet |

---

**Document Version:** 2.1 | February 14, 2026
**Sassy Consulting LLC**
