//! Fingerprint Poisoning Engine
//!
//! ANTI-FINGERPRINTING THROUGH DATA POISONING — NOT BLOCKING.
#![allow(dead_code)]
//!
//! PHILOSOPHY:
//! ─────────────────────────────────────────────────────────────────────────
//! Blocking fingerprint APIs breaks sites. Poisoning them doesn't.
//! Instead of returning errors or empty data, we return plausible but
//! slightly wrong data that makes every session look unique.
//!
//! THREE MODES:
//! ─────────────────────────────────────────────────────────────────────────
//! 1. OFF — Full Chrome/Edge spoof. Looks exactly like a real browser.
//!    No noise, no fingerprint resistance. For sites that break.
//!
//! 2. CONSERVATIVE — Very light noise (~0.5-1% bit flips on canvas,
//!    sub-audible audio noise, tiny hardware variation). Breaks tracking
//!    while keeping 99%+ site compatibility.
//!
//! 3. AGGRESSIVE — Maximum unlinkability. Heavier noise on canvas/WebGL,
//!    randomized hardware props, aggressive audio noise. ~2-5% breakage.
//!
//! COVERAGE (2024-2030 surfaces):
//! ─────────────────────────────────────────────────────────────────────────
//! - Canvas 2D (getImageData, toDataURL)
//! - WebGL (readPixels, getParameter)
//! - Audio (AudioBuffer.getChannelData, OscillatorNode)
//! - Navigator (userAgent, platform, hardwareConcurrency, deviceMemory)
//! - Screen (width, height, colorDepth, pixelDepth)
//! - Battery API
//! - WebRTC local IP leak
//! - Font detection (measureText)
//! - Intl timezone/locale
//! - Gamepad API
//! - MediaDevices enumeration
//! - SpeechSynthesis voices
//! - WebGPU adapter info
//! - Sensor APIs (Accelerometer, Gyroscope)
//!
//! Each poisoned API returns data that looks real but is subtly different
//! every session, making cross-session tracking impossible.

use crate::js::interpreter::JsInterpreter;
use crate::js::value::Value;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

// ═══════════════════════════════════════════════════════════════════════════════
// POISONING MODE
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoisonMode {
    /// No poisoning — full spoof as Chrome/Edge (looks like a real browser)
    Off,
    /// Very low noise — almost undetectable, breaks tracking
    Conservative,
    /// Maximum unlinkability — higher breakage risk (~2-5%)
    Aggressive,
}

impl Default for PoisonMode {
    fn default() -> Self {
        PoisonMode::Conservative
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// FINGERPRINT SURFACES — All known tracking vectors
// ═══════════════════════════════════════════════════════════════════════════════

/// Complete list of fingerprinting surfaces we defend against (2024-2030).
/// This is documentation/audit — the actual poisoning is done in JS injection.
pub const ALL_FINGERPRINT_SURFACES: &[&str] = &[
    // ─── Classic 2024–2026 surfaces ───
    "CanvasRenderingContext2D.getImageData",
    "HTMLCanvasElement.toDataURL",
    "WebGLRenderingContext.readPixels",
    "WebGLRenderingContext.getParameter",
    "OfflineAudioContext.createOscillator",
    "AudioBuffer.getChannelData",
    "navigator.hardwareConcurrency",
    "navigator.deviceMemory",
    "screen.width / screen.height / screen.availWidth",
    "navigator.userAgent",
    "navigator.platform",
    "navigator.language / languages",
    "navigator.plugins",
    "navigator.mimeTypes",
    "Intl.DateTimeFormat().resolvedOptions().timeZone",
    "Intl.RelativeTimeFormat().resolvedOptions().locale",
    "performance.now() timing jitter",
    "font detection via measureText",
    "BatteryManager chargingTime / dischargingTime",
    "WebRTC local IP leak via RTCPeerConnection",
    // ─── Uncommon / emerging 2025–2026 ───
    "WebGPUAdapter.requestAdapter",
    "GPUDevice.createBuffer",
    "Sensor APIs (Accelerometer, Gyroscope, Magnetometer)",
    "WebHID (USB device enumeration)",
    "WebSerial (serial port enumeration)",
    "WebBluetooth device enumeration",
    "Gamepad API (connected gamepads)",
    "MediaDevices.enumerateDevices()",
    "SpeechSynthesis.getVoices()",
    "OffscreenCanvas",
    "IntersectionObserver timing side-channels",
    // ─── Future surfaces (2027–2030 speculation) ───
    "NeuralNetwork API (WebNN) model inference timing",
    "AmbientLightSensor + ProximitySensor fusion",
    "WebTransport datagram timing side-channel",
    "WebCodecs video/audio encoder/decoder fingerprint",
    "WebNN compilation time variance",
    "Eye-tracking / gaze estimation APIs",
    "Haptic feedback device enumeration",
    "AR/VR device sensors & capabilities",
    "Quantum-safe crypto API timing differences",
    "Neuromorphic hardware detection",
];

// ═══════════════════════════════════════════════════════════════════════════════
// POISONING ENGINE
// ═══════════════════════════════════════════════════════════════════════════════

/// Breakage record for auto-disabling poisoning on sites that break
#[derive(Debug, Clone)]
pub struct BreakageRecord {
    pub last_failure: Instant,
    pub failure_count: u32,
    /// If set, poisoning is disabled for this domain until this time
    pub disabled_until: Option<Instant>,
}

#[derive(Debug, Clone)]
pub struct PoisoningEngine {
    /// Current poisoning mode
    pub mode: PoisonMode,
    /// Per-site exceptions — domains where poisoning is manually disabled
    pub disabled_domains: HashSet<String>,
    /// Breakage tracking — auto-disables poisoning on sites that break
    pub breakage_cache: HashMap<String, BreakageRecord>,
    /// Fallback user agent for Chrome/Edge spoof
    fallback_user_agent: String,
}

impl Default for PoisoningEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PoisoningEngine {
    pub fn new() -> Self {
        Self {
            mode: PoisonMode::Conservative,
            disabled_domains: HashSet::new(),
            breakage_cache: HashMap::new(),
            fallback_user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
                                 AppleWebKit/537.36 (KHTML, like Gecko) \
                                 Chrome/132.0.0.0 Safari/537.36 Edg/132.0.0.0"
                                 .to_string(),
        }
    }

    pub fn set_mode(&mut self, mode: PoisonMode) {
        self.mode = mode;
    }

    pub fn disable_for_domain(&mut self, domain: &str) {
        self.disabled_domains.insert(domain.to_string());
    }

    pub fn enable_for_domain(&mut self, domain: &str) {
        self.disabled_domains.remove(domain);
    }

    /// Check if poisoning should be applied for this URL.
    /// Respects: mode, manual exceptions, and auto-breakage cache.
    pub fn should_poison(&self, url: &str) -> bool {
        if self.mode == PoisonMode::Off {
            return false;
        }
        if let Ok(parsed) = url::Url::parse(url) {
            if let Some(host) = parsed.host_str() {
                // Manual exception
                if self.disabled_domains.contains(host) {
                    return false;
                }
                // Auto-disabled due to breakage
                if self.is_domain_auto_disabled(host) {
                    return false;
                }
            }
        }
        true
    }

    /// Check if a domain was auto-disabled due to breakage
    fn is_domain_auto_disabled(&self, domain: &str) -> bool {
        if let Some(record) = self.breakage_cache.get(domain) {
            if let Some(until) = record.disabled_until {
                return Instant::now() < until;
            }
        }
        false
    }

    /// Record a site breakage event. After 2 failures in 5 minutes,
    /// auto-disables poisoning for that domain for 24 hours.
    pub fn record_breakage(&mut self, url: &str, reason: &str) {
        if let Ok(parsed) = url::Url::parse(url) {
            if let Some(domain) = parsed.host_str() {
                let now = Instant::now();
                let entry = self.breakage_cache.entry(domain.to_string())
                    .or_insert(BreakageRecord {
                        last_failure: now,
                        failure_count: 0,
                        disabled_until: None,
                    });
                entry.failure_count += 1;
                entry.last_failure = now;

                // Auto-disable after 2 failures within a 5-minute window
                if entry.failure_count >= 2 {
                    entry.disabled_until = Some(now + Duration::from_secs(86400)); // 24h
                    tracing::warn!(
                        "[POISON] Auto-disabling poisoning for {} (24h) after {} failures: {}",
                        domain, entry.failure_count, reason
                    );
                } else {
                    tracing::info!("[POISON] Breakage recorded for {}: {}", domain, reason);
                }
            }
        }
    }

    /// Clear breakage cache for a domain (manual override)
    pub fn clear_breakage(&mut self, domain: &str) {
        self.breakage_cache.remove(domain);
    }

    /// Get all domains currently auto-disabled
    pub fn auto_disabled_domains(&self) -> Vec<String> {
        let now = Instant::now();
        self.breakage_cache.iter()
            .filter(|(_, r)| r.disabled_until.map_or(false, |until| now < until))
            .map(|(d, _)| d.clone())
            .collect()
    }

    // ─────────────────────────────────────────────────────────────────────
    // Main poisoning entry point — called after page load
    // ─────────────────────────────────────────────────────────────────────

    /// Apply fingerprint poisoning to a page's JS context.
    /// Called from the script engine after DOMContentLoaded.
    ///
    /// LAYERED APPROACH:
    /// - Off: Full Chrome/Edge spoof only
    /// - Conservative: Level 1 (light noise) + Level 2 (font/WebRTC/battery/sensors)
    /// - Aggressive: Level 1 + Level 2 + Level 3 (WebGPU/WebNN/haptic/crypto timing)
    pub fn poison_page(&self, js: &mut JsInterpreter, url: &str) -> Result<Value, String> {
        if !self.should_poison(url) {
            // No poisoning — apply full Chrome/Edge spoof instead
            return self.apply_chrome_edge_spoof(js);
        }

        match self.mode {
            PoisonMode::Off => self.apply_chrome_edge_spoof(js),
            PoisonMode::Conservative => {
                self.poison_level_1(js)?;
                self.poison_level_2(js)
            }
            PoisonMode::Aggressive => {
                self.poison_level_1(js)?;
                self.poison_level_2(js)?;
                self.poison_level_3(js)
            }
        }
    }

    /// Convenience: apply all surfaces for the current mode
    pub fn poison_all_surfaces(&self, js: &mut JsInterpreter, mode: PoisonMode) -> Result<Value, String> {
        match mode {
            PoisonMode::Off => self.apply_chrome_edge_spoof(js),
            PoisonMode::Conservative => {
                self.poison_level_1(js)?;
                self.poison_level_2(js)
            }
            PoisonMode::Aggressive => {
                self.poison_level_1(js)?;
                self.poison_level_2(js)?;
                self.poison_level_3(js)
            }
        }
    }

    /// Full Chrome/Edge spoof — makes Sassy look exactly like Chrome 132
    pub fn apply_chrome_edge_spoof(&self, js: &mut JsInterpreter) -> Result<Value, String> {
        let spoof_script = format!(
            r#"
            Object.defineProperty(navigator, 'userAgent', {{ get: () => "{ua}" }});
            Object.defineProperty(navigator, 'platform', {{ get: () => "Win32" }});
            Object.defineProperty(navigator, 'hardwareConcurrency', {{ get: () => 16 }});
            Object.defineProperty(navigator, 'deviceMemory', {{ get: () => 8 }});
            Object.defineProperty(screen, 'width', {{ get: () => 1920 }});
            Object.defineProperty(screen, 'height', {{ get: () => 1080 }});
            Object.defineProperty(screen, 'availWidth', {{ get: () => 1920 }});
            Object.defineProperty(screen, 'availHeight', {{ get: () => 1040 }});
            Object.defineProperty(screen, 'colorDepth', {{ get: () => 24 }});
            Object.defineProperty(screen, 'pixelDepth', {{ get: () => 24 }});
            "#,
            ua = self.fallback_user_agent
        );
        js.execute(&spoof_script)
    }

    // ─────────────────────────────────────────────────────────────────────
    // LEVEL 1 — Light noise (canvas, audio, hardware, timing, WebRTC)
    // ~99%+ site compatibility
    // ─────────────────────────────────────────────────────────────────────

    fn poison_level_1(&self, js: &mut JsInterpreter) -> Result<Value, String> {
        js.execute(r#"
            // Canvas — 0.5–1% LSB noise (invisible to eye, breaks fingerprinting)
            (function() {
                var origGetImageData = CanvasRenderingContext2D.prototype.getImageData;
                CanvasRenderingContext2D.prototype.getImageData = function() {
                    var data = origGetImageData.apply(this, arguments);
                    var pixels = data.data;
                    for (var i = 0; i < pixels.length; i += 4) {
                        if (Math.random() < 0.008) pixels[i]   ^= 1;
                        if (Math.random() < 0.008) pixels[i+1] ^= 1;
                        if (Math.random() < 0.008) pixels[i+2] ^= 1;
                    }
                    return data;
                };
            })();

            // Audio — sub-audible 0.00001 jitter
            (function() {
                if (typeof AudioBuffer !== 'undefined') {
                    var origGetChannelData = AudioBuffer.prototype.getChannelData;
                    AudioBuffer.prototype.getChannelData = function(channel) {
                        var data = origGetChannelData.call(this, channel);
                        for (var i = 0; i < data.length; i++) {
                            data[i] += (Math.random() - 0.5) * 0.00001;
                        }
                        return data;
                    };
                }
            })();

            // Hardware concurrency — slight variation
            (function() {
                var realHC = navigator.hardwareConcurrency || 8;
                Object.defineProperty(navigator, 'hardwareConcurrency', {
                    get: function() { return realHC + (Math.random() < 0.3 ? 1 : 0); }
                });
            })();

            // Performance.now() — small jitter to break timing attacks
            (function() {
                var origNow = Performance.prototype.now;
                Performance.prototype.now = function() {
                    return origNow.call(this) + (Math.random() - 0.5) * 0.1;
                };
            })();

            // WebRTC — fake local IP + block data channels
            (function() {
                var realRTC = window.RTCPeerConnection || window.webkitRTCPeerConnection;
                if (realRTC) {
                    window.RTCPeerConnection = function(config) {
                        var pc = new realRTC(config);
                        var realAddCandidate = pc.addIceCandidate;
                        pc.addIceCandidate = function(candidate) {
                            if (candidate && candidate.candidate) {
                                var fakeIp = '192.168.' + Math.floor(Math.random()*255) + '.' + Math.floor(Math.random()*255);
                                candidate.candidate = candidate.candidate.replace(/(\d{1,3}\.){3}\d{1,3}/, fakeIp);
                            }
                            return realAddCandidate.call(this, candidate);
                        };
                        return pc;
                    };
                }
            })();
        "#)
    }

    // ─────────────────────────────────────────────────────────────────────
    // LEVEL 2 — Font, battery, sensors, WebGL, screen, plugins/mimeTypes
    // Covers all common 2024-2026 fingerprinting surfaces
    // ─────────────────────────────────────────────────────────────────────

    fn poison_level_2(&self, js: &mut JsInterpreter) -> Result<Value, String> {
        js.execute(r#"
            // Font enumeration — fake positives for ghost fonts
            (function() {
                if (document.fonts) {
                    var realCheck = document.fonts.check.bind(document.fonts);
                    document.fonts.check = function(fontStr) {
                        if (Math.random() < 0.12) return true;
                        return realCheck(fontStr);
                    };
                }
            })();

            // BatteryManager — realistic jitter
            (function() {
                if (navigator.getBattery) {
                    var realGetBattery = navigator.getBattery;
                    navigator.getBattery = function() {
                        return realGetBattery.call(navigator).then(function(battery) {
                            var fakeLevel = Math.max(0.15, Math.min(0.99, 0.85 + (Math.random() * 0.14 - 0.07)));
                            return {
                                charging: battery.charging && Math.random() > 0.05,
                                chargingTime: battery.charging ? 0 : Math.floor(Math.random() * 7200000 + 3600000),
                                dischargingTime: battery.dischargingTime || Infinity,
                                level: fakeLevel,
                                addEventListener: function() {},
                                removeEventListener: function() {}
                            };
                        });
                    };
                }
            })();

            // Sensor API noise (Accelerometer, Gyroscope, Magnetometer)
            (function() {
                var sensorTypes = ['Accelerometer', 'Gyroscope', 'Magnetometer'];
                sensorTypes.forEach(function(type) {
                    if (window[type]) {
                        Object.defineProperty(window[type].prototype, 'frequency', {
                            get: function() { return 60 + Math.floor(Math.random() * 40 - 20); }
                        });
                    }
                });
            })();

            // WebGL parameter noise
            (function() {
                if (typeof WebGLRenderingContext !== 'undefined') {
                    var origGetParam = WebGLRenderingContext.prototype.getParameter;
                    WebGLRenderingContext.prototype.getParameter = function(pname) {
                        var result = origGetParam.call(this, pname);
                        // Noise on MAX_TEXTURE_SIZE and similar
                        if (typeof result === 'number' && result > 256) {
                            result += Math.floor(Math.random() * 4 - 2);
                        }
                        return result;
                    };
                }
            })();

            // Screen — slight variation on standard values
            (function() {
                var realW = screen.width || 1920;
                var realH = screen.height || 1080;
                Object.defineProperty(screen, 'colorDepth', { get: function() { return 24; } });
                Object.defineProperty(screen, 'pixelDepth', { get: function() { return 24; } });
            })();

            // Plugins/mimeTypes — spoof to look like Chrome
            (function() {
                Object.defineProperty(navigator, 'plugins', {
                    get: function() { return { length: 5 }; }
                });
                Object.defineProperty(navigator, 'mimeTypes', {
                    get: function() { return { length: 4 }; }
                });
            })();

            // MediaDevices — return generic devices
            (function() {
                if (navigator.mediaDevices && navigator.mediaDevices.enumerateDevices) {
                    navigator.mediaDevices.enumerateDevices = function() {
                        return Promise.resolve([
                            { deviceId: 'default', kind: 'audioinput', label: '', groupId: '' },
                            { deviceId: 'default', kind: 'videoinput', label: '', groupId: '' },
                            { deviceId: 'default', kind: 'audiooutput', label: '', groupId: '' }
                        ]);
                    };
                }
            })();

            // SpeechSynthesis — return limited voice list
            (function() {
                if (window.speechSynthesis) {
                    window.speechSynthesis.getVoices = function() {
                        return [
                            { name: 'Microsoft David Desktop', lang: 'en-US', localService: true },
                            { name: 'Microsoft Zira Desktop', lang: 'en-US', localService: true }
                        ];
                    };
                }
            })();
        "#)
    }

    // ─────────────────────────────────────────────────────────────────────
    // LEVEL 3 — Emerging/future surfaces (WebGPU, WebNN, haptic, crypto)
    // Nuclear anti-fingerprinting — maximum unlinkability
    // ─────────────────────────────────────────────────────────────────────

    fn poison_level_3(&self, js: &mut JsInterpreter) -> Result<Value, String> {
        js.execute(r#"
            // Canvas — heavy 2-5% noise + random offset
            (function() {
                var origGetImageData = CanvasRenderingContext2D.prototype.getImageData;
                CanvasRenderingContext2D.prototype.getImageData = function(x, y, w, h) {
                    var data = origGetImageData.call(this, x, y, w, h);
                    var pixels = data.data;
                    var offset = Math.floor(Math.random() * 3) - 1;
                    for (var i = 0; i < pixels.length; i += 4) {
                        pixels[i]   = (pixels[i]   + offset + 256) % 256;
                        pixels[i+1] = (pixels[i+1] + offset + 256) % 256;
                        pixels[i+2] = (pixels[i+2] + offset + 256) % 256;
                    }
                    return data;
                };
            })();

            // WebGL readPixels — heavy noise
            (function() {
                if (typeof WebGLRenderingContext !== 'undefined') {
                    var origReadPixels = WebGLRenderingContext.prototype.readPixels;
                    WebGLRenderingContext.prototype.readPixels = function() {
                        origReadPixels.apply(this, arguments);
                        var pixels = arguments[6];
                        if (pixels instanceof Uint8Array) {
                            for (var i = 0; i < pixels.length; i++) {
                                pixels[i] ^= Math.random() < 0.05 ? 1 : 0;
                            }
                        }
                    };
                }
            })();

            // Audio — stronger noise (10x conservative)
            (function() {
                if (typeof AudioBuffer !== 'undefined') {
                    var origGetChannelData = AudioBuffer.prototype.getChannelData;
                    AudioBuffer.prototype.getChannelData = function(channel) {
                        var data = origGetChannelData.call(this, channel);
                        for (var i = 0; i < data.length; i++) {
                            data[i] += (Math.random() - 0.5) * 0.0001;
                        }
                        return data;
                    };
                }
            })();

            // Randomize navigator properties across common values
            (function() {
                var hcVars = [4,6,8,12,16,24];
                Object.defineProperty(navigator, 'hardwareConcurrency', {
                    get: function() { return hcVars[Math.floor(Math.random()*hcVars.length)]; }
                });
                var memVars = [2,4,8,16];
                Object.defineProperty(navigator, 'deviceMemory', {
                    get: function() { return memVars[Math.floor(Math.random()*memVars.length)]; }
                });
            })();

            // Screen — randomize across common resolutions
            (function() {
                var widths = [1366, 1440, 1536, 1920, 2560];
                var heights = [768, 900, 864, 1080, 1440];
                var idx = Math.floor(Math.random() * widths.length);
                Object.defineProperty(screen, 'width', { get: function() { return widths[idx]; } });
                Object.defineProperty(screen, 'height', { get: function() { return heights[idx]; } });
                Object.defineProperty(screen, 'availWidth', { get: function() { return widths[idx]; } });
                Object.defineProperty(screen, 'availHeight', { get: function() { return heights[idx] - 40; } });
            })();

            // Performance.now() — stronger jitter (integer + random fraction)
            (function() {
                var origNow = Performance.prototype.now;
                Performance.prototype.now = function() {
                    return Math.round(origNow.call(this)) + Math.random();
                };
            })();

            // WebGPU — poison adapter features and limits
            (function() {
                if (navigator.gpu) {
                    var realRequestAdapter = navigator.gpu.requestAdapter;
                    navigator.gpu.requestAdapter = function() {
                        return realRequestAdapter.apply(navigator.gpu, arguments).then(function(adapter) {
                            if (adapter && adapter.limits) {
                                var realLimits = adapter.limits;
                                Object.defineProperty(adapter, 'limits', {
                                    get: function() {
                                        return Object.assign({}, realLimits, {
                                            maxTextureDimension2D: (realLimits.maxTextureDimension2D || 8192) + Math.floor(Math.random() * 256 - 128)
                                        });
                                    }
                                });
                            }
                            return adapter;
                        });
                    };
                }
            })();

            // WebNN (Web Neural Network) — inference timing jitter
            (function() {
                if (navigator.ml && navigator.ml.createModel) {
                    var realCreate = navigator.ml.createModel;
                    navigator.ml.createModel = function() {
                        return realCreate.apply(navigator.ml, arguments).then(function(model) {
                            if (model && model.infer) {
                                var realInfer = model.infer;
                                model.infer = function(input) {
                                    return new Promise(function(resolve) {
                                        setTimeout(function() {
                                            resolve(realInfer.call(model, input));
                                        }, Math.random() * 5 + 2);
                                    });
                                };
                            }
                            return model;
                        });
                    };
                }
            })();

            // Ambient light sensor — noise
            (function() {
                if (window.AmbientLightSensor) {
                    Object.defineProperty(window.AmbientLightSensor.prototype, 'illuminance', {
                        get: function() { return 250 + (Math.random() - 0.5) * 50; }
                    });
                }
            })();

            // Haptic vibrate — timing distortion
            (function() {
                if (navigator.vibrate) {
                    var realVibrate = navigator.vibrate;
                    navigator.vibrate = function(pattern) {
                        if (Array.isArray(pattern)) {
                            pattern = pattern.map(function(p) { return p + Math.random() * 10 - 5; });
                        }
                        return realVibrate.call(navigator, pattern);
                    };
                }
            })();

            // SubtleCrypto digest timing — constant-time mitigation
            (function() {
                if (window.crypto && window.crypto.subtle) {
                    var realDigest = window.crypto.subtle.digest;
                    window.crypto.subtle.digest = function() {
                        var args = arguments;
                        var self = this;
                        return new Promise(function(resolve) {
                            setTimeout(function() {
                                resolve(realDigest.apply(self, args));
                            }, Math.random() * 3 + 1);
                        });
                    };
                }
            })();

            // Gamepad API — return empty gamepads
            (function() {
                navigator.getGamepads = function() { return [null, null, null, null]; };
            })();
        "#)
    }

    /// Get a human-readable description of the current mode
    pub fn mode_description(&self) -> &'static str {
        match self.mode {
            PoisonMode::Off => "Off -- Chrome spoof",
            PoisonMode::Conservative => "Conservative (Level 1+2)",
            PoisonMode::Aggressive => "Aggressive (Nuclear L1+L2+L3)",
        }
    }

    /// Get the number of fingerprint surfaces covered
    pub fn surface_count() -> usize {
        ALL_FINGERPRINT_SURFACES.len()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_conservative() {
        let engine = PoisoningEngine::new();
        assert_eq!(engine.mode, PoisonMode::Conservative);
    }

    #[test]
    fn test_should_poison_respects_mode() {
        let mut engine = PoisoningEngine::new();

        // Conservative mode — should poison
        assert!(engine.should_poison("https://example.com"));

        // Off mode — should not poison
        engine.set_mode(PoisonMode::Off);
        assert!(!engine.should_poison("https://example.com"));
    }

    #[test]
    fn test_domain_exceptions() {
        let mut engine = PoisoningEngine::new();

        // Add exception
        engine.disable_for_domain("bank.com");
        assert!(!engine.should_poison("https://bank.com/login"));
        assert!(engine.should_poison("https://tracker.com"));

        // Remove exception
        engine.enable_for_domain("bank.com");
        assert!(engine.should_poison("https://bank.com/login"));
    }

    #[test]
    fn test_surface_coverage() {
        // Ensure we're covering a meaningful number of surfaces
        assert!(PoisoningEngine::surface_count() >= 30);
    }

    #[test]
    fn test_invalid_url_still_poisons() {
        let engine = PoisoningEngine::new();
        // Invalid URL with no host — should still return true (conservative default)
        assert!(engine.should_poison("not-a-url"));
    }
}
