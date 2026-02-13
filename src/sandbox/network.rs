//! Network Sandbox - validate and gate outbound network requests
//!
//! Provides DNS validation, host allowlist/blocklist, trust-based gating,
//! and per-host rate limiting to prevent abuse from untrusted pages.

use super::TrustLevel;
use std::collections::HashMap;
use std::net::ToSocketAddrs;
use std::time::Instant;

/// Maximum connections per host per window (10 connections / 1 second)
const RATE_LIMIT_WINDOW_SECS: f64 = 1.0;
const RATE_LIMIT_MAX_PER_WINDOW: usize = 10;

/// Per-host rate-limit tracker
#[derive(Debug, Clone)]
struct HostRateLimit {
    /// Timestamps of recent connection attempts
    attempts: Vec<Instant>,
}

impl HostRateLimit {
    fn new() -> Self {
        Self { attempts: Vec::new() }
    }

    /// Record an attempt and return true if within limit
    fn check_and_record(&mut self) -> bool {
        let now = Instant::now();
        let window = std::time::Duration::from_secs_f64(RATE_LIMIT_WINDOW_SECS);
        self.attempts.retain(|t| now.duration_since(*t) < window);
        if self.attempts.len() >= RATE_LIMIT_MAX_PER_WINDOW {
            return false; // Rate limited
        }
        self.attempts.push(now);
        true
    }
}

/// Network sandbox with allowlist, blocklist, and rate limiting
#[derive(Debug, Clone)]
pub struct NetworkSandbox {
    /// Hostnames explicitly allowed by the user or policy
    pub allowed_hosts: Vec<String>,
    /// Hostnames explicitly blocked
    blocked_hosts: Vec<String>,
    /// Per-host rate limit trackers
    rate_limits: HashMap<String, HostRateLimit>,
    /// Last time a resolution/validation occurred
    pub last_validation: Option<Instant>,
    /// Stats
    pub connections_blocked: u64,
    pub rate_limited_count: u64,
}

/// Known-bad domain patterns (malware, phishing, crypto-mining)
const BLOCKED_PATTERNS: &[&str] = &[
    "coinhive.com", "coin-hive.com", "jsecoin.com", "cryptoloot.pro",
    "minero.cc", "webminepool.com", "ppoi.org", "monerominer.rocks",
];

impl NetworkSandbox {
    pub fn new() -> Self {
        Self {
            allowed_hosts: Vec::new(),
            blocked_hosts: BLOCKED_PATTERNS.iter().map(|s| s.to_string()).collect(),
            rate_limits: HashMap::new(),
            last_validation: None,
            connections_blocked: 0,
            rate_limited_count: 0,
        }
    }

    /// Check whether a hostname can be resolved (best-effort)
    pub fn resolve_host(host: &str) -> bool {
        host.to_socket_addrs().is_ok()
    }

    /// Determine whether network access should be permitted for an origin.
    /// Checks blocklist, rate limits, and trust level.
    pub fn allow_network_for(&mut self, origin_host: &str, trust: TrustLevel) -> bool {
        // Always block known-bad hosts
        if self.is_blocked(origin_host) {
            self.connections_blocked += 1;
            return false;
        }

        // Allowlisted hosts bypass trust check (but not blocklist or rate limit)
        let trust_ok = self.allowed_hosts.iter().any(|h| h == origin_host)
            || trust.can_access_network();

        if !trust_ok {
            self.connections_blocked += 1;
            return false;
        }

        // Rate limiting for non-allowlisted hosts
        if !self.allowed_hosts.iter().any(|h| h == origin_host) {
            let limiter = self.rate_limits
                .entry(origin_host.to_string())
                .or_insert_with(HostRateLimit::new);
            if !limiter.check_and_record() {
                self.rate_limited_count += 1;
                return false;
            }
        }

        self.last_validation = Some(Instant::now());
        true
    }

    /// Check if a host matches the blocklist
    pub fn is_blocked(&self, host: &str) -> bool {
        self.blocked_hosts.iter().any(|blocked| host.contains(blocked))
    }

    /// Add a host to the allowlist
    pub fn allow_host(&mut self, host: &str) {
        if !self.allowed_hosts.contains(&host.to_string()) {
            self.allowed_hosts.push(host.to_string());
        }
    }

    /// Add a host to the blocklist
    pub fn block_host(&mut self, host: &str) {
        if !self.blocked_hosts.contains(&host.to_string()) {
            self.blocked_hosts.push(host.to_string());
        }
    }

    /// Clean up stale rate limit entries (call periodically)
    pub fn cleanup(&mut self) {
        let now = Instant::now();
        let window = std::time::Duration::from_secs(5);
        self.rate_limits.retain(|_, rl| {
            rl.attempts.iter().any(|t| now.duration_since(*t) < window)
        });
    }
}
