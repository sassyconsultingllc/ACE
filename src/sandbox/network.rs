//! Network Sandbox - validate and gate outbound network requests
//!
//! Minimal, safe implementation to start: DNS validation, host allowlist, and gating
//! based on `TrustLevel`. This is intentionally conservative and designed to be
//! expanded as needed.

use super::TrustLevel;
use std::net::ToSocketAddrs;
use std::time::Instant;

/// Basic network sandbox structure
#[derive(Debug, Clone)]
pub struct NetworkSandbox {
    /// Hostnames explicitly allowed by the user or policy
    pub allowed_hosts: Vec<String>,
    /// Last time a resolution/validation occurred
    pub last_validation: Option<Instant>,
}

impl NetworkSandbox {
    pub fn new() -> Self {
        Self {
            allowed_hosts: Vec::new(),
            last_validation: None,
        }
    }

    /// Check whether a hostname can be resolved (best-effort)
    pub fn resolve_host(host: &str) -> bool {
        host.to_socket_addrs().is_ok()
    }

    /// Determine whether network access should be permitted for an origin
    /// based on its `TrustLevel`. By default only `TrustLevel::Established`
    /// grants full network access; this can be relaxed via allowlist.
    pub fn allow_network_for(origin_host: &str, trust: TrustLevel, sandbox: &Self) -> bool {
        if sandbox.allowed_hosts.iter().any(|h| h == origin_host) {
            return true;
        }

        trust.can_access_network()
    }

    /// Add a host to the allowlist
    pub fn allow_host(&mut self, host: &str) {
        if !self.allowed_hosts.contains(&host.to_string()) {
            self.allowed_hosts.push(host.to_string());
        }
    }
}
