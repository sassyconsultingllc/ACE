//! Secure sync via Tailscale/WireGuard mesh
//! 
//! Instead of exposing a port on the local network:
//! 1. Bind ONLY to Tailscale interface (100.x.x.x) or localhost
//! 2. Phone connects via Tailscale IP
//! 3. Tailscale handles encryption + device auth
//! 4. Optional device approval in browser UI
//!
//! No pairing codes, no tokens, no open ports.
//! Tailscale's MagicDNS means phone can use "desktop.tailnet" instead of IP.

use std::net::{IpAddr, Ipv4Addr, TcpListener, UdpSocket};
use std::process::Command;
use serde::{Deserialize, Serialize};

/// Network binding mode
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BindMode {
    /// Only localhost - most secure, requires port forwarding via Tailscale
    Localhost,
    /// Tailscale interface only (100.x.x.x) - secure, direct connection
    TailscaleOnly,
    /// All interfaces - NOT recommended, legacy mode
    AllInterfaces,
}

/// Detected Tailscale status
#[derive(Debug, Clone)]
pub struct TailscaleInfo {
    pub available: bool,
    pub ip: Option<IpAddr>,
    pub hostname: Option<String>,
    pub tailnet: Option<String>,
    pub magic_dns: Option<String>,
}

impl TailscaleInfo {
    /// Detect Tailscale configuration
    pub fn detect() -> Self {
        let mut info = Self {
            available: false,
            ip: None,
            hostname: None,
            tailnet: None,
            magic_dns: None,
        };
        
        // Try to get Tailscale IP from CLI
        if let Ok(output) = Command::new("tailscale").args(["ip", "-4"]).output() {
            if output.status.success() {
                if let Ok(ip_str) = String::from_utf8(output.stdout) {
                    if let Ok(ip) = ip_str.trim().parse::<Ipv4Addr>() {
                        // Tailscale IPs are in 100.x.x.x range
                        if ip.octets()[0] == 100 {
                            info.available = true;
                            info.ip = Some(IpAddr::V4(ip));
                        }
                    }
                }
            }
        }
        
        // Get hostname for MagicDNS
        if info.available {
            if let Ok(output) = Command::new("tailscale").args(["status", "--json"]).output() {
                if output.status.success() {
                    if let Ok(json) = String::from_utf8(output.stdout) {
                        // Parse just what we need (simplified)
                        if let Some(start) = json.find("\"Self\"") {
                            if let Some(dns_start) = json[start..].find("\"DNSName\":\"") {
                                let dns_slice = &json[start + dns_start + 11..];
                                if let Some(end) = dns_slice.find('"') {
                                    let dns_name = &dns_slice[..end];
                                    info.magic_dns = Some(dns_name.to_string());
                                    
                                    // Extract hostname and tailnet
                                    let parts: Vec<&str> = dns_name.split('.').collect();
                                    if parts.len() >= 2 {
                                        info.hostname = Some(parts[0].to_string());
                                        info.tailnet = Some(parts[1..].join("."));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Fallback: Check for 100.x.x.x interface directly
        if !info.available {
            info.ip = find_tailscale_interface();
            info.available = info.ip.is_some();
        }
        
        info
    }
    
    /// Get the connection URL for phone
    #[allow(dead_code)]
    pub fn connection_url(&self, port: u16) -> Option<String> {
        if let Some(ref dns) = self.magic_dns {
            // Use MagicDNS name (works across networks)
            Some(format!("ws://{}:{}", dns.trim_end_matches('.'), port))
        } else { self.ip.map(|ip| format!("ws://{}:{}", ip, port)) }
    }
}

/// Find Tailscale interface by scanning for 100.x.x.x
fn find_tailscale_interface() -> Option<IpAddr> {
    // Bind to 0.0.0.0 and connect to a Tailscale IP to find our interface
    // This is a common trick to find the local IP for a specific route
    
    // Try connecting to Tailscale's DERP servers or a known Tailscale IP
    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
        // Try to "connect" to an IP in the Tailscale range
        // This doesn't send packets, just sets up routing
        if socket.connect("100.100.100.100:80").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                let ip = addr.ip();
                if let IpAddr::V4(v4) = ip {
                    if v4.octets()[0] == 100 {
                        return Some(ip);
                    }
                }
            }
        }
    }
    
    None
}

/// Secure sync configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub enabled: bool,
    pub port: u16,
    pub bind_mode: BindMode,
    pub require_approval: bool,
    pub approved_devices: Vec<String>,
    pub auto_approve_tailscale: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 8765,
            bind_mode: BindMode::TailscaleOnly,
            require_approval: true,
            approved_devices: Vec::new(),
            auto_approve_tailscale: true, // Trust Tailscale-authenticated devices
        }
    }
}

impl SyncConfig {
    /// Get the bind address based on mode and available interfaces
    #[allow(dead_code)]
    pub fn bind_address(&self, tailscale: &TailscaleInfo) -> Result<String, String> {
        match self.bind_mode {
            BindMode::Localhost => {
                Ok(format!("127.0.0.1:{}", self.port))
            }
            BindMode::TailscaleOnly => {
                if let Some(ip) = tailscale.ip {
                    Ok(format!("{}:{}", ip, self.port))
                } else {
                    Err("Tailscale not available. Install Tailscale or change bind mode.".into())
                }
            }
            BindMode::AllInterfaces => {
                // Not recommended but available for legacy/testing
                eprintln!("âš ï¸  WARNING: Binding to all interfaces is insecure!");
                Ok(format!("0.0.0.0:{}", self.port))
            }
        }
    }
}

/// Device info from connection
#[allow(dead_code)] // Fields for connected device tracking
#[derive(Debug, Clone)]
pub struct ConnectedDevice {
    pub ip: IpAddr,
    pub tailscale_hostname: Option<String>,
    pub user_agent: Option<String>,
    pub connected_at: std::time::Instant,
    pub approved: bool,
}

#[allow(dead_code)] // Public API methods
impl ConnectedDevice {
    /// Check if this device is from Tailscale
    pub fn is_tailscale(&self) -> bool {
        if let IpAddr::V4(v4) = self.ip {
            v4.octets()[0] == 100
        } else {
            false
        }
    }
    
    /// Get display name for UI
    pub fn display_name(&self) -> String {
        if let Some(ref hostname) = self.tailscale_hostname {
            hostname.clone()
        } else {
            self.ip.to_string()
        }
    }
}

/// Secure server wrapper
#[allow(dead_code)] // Fields for secure server state
pub struct SecureSyncServer {
    pub config: SyncConfig,
    pub tailscale: TailscaleInfo,
    pub connected_devices: Vec<ConnectedDevice>,
    listener: Option<TcpListener>,
}

#[allow(dead_code)] // Public API methods for secure sync server
impl SecureSyncServer {
    pub fn new(config: SyncConfig) -> Self {
        let tailscale = TailscaleInfo::detect();
        
        Self {
            config,
            tailscale,
            connected_devices: Vec::new(),
            listener: None,
        }
    }
    
    /// Start the secure server
    pub fn start(&mut self) -> Result<(), String> {
        let bind_addr = self.config.bind_address(&self.tailscale)?;
        
        self.listener = Some(
            TcpListener::bind(&bind_addr)
                .map_err(|e| format!("Failed to bind to {}: {}", bind_addr, e))?
        );
        
        if let Some(ref listener) = self.listener {
            listener.set_nonblocking(true)
                .map_err(|e| format!("Failed to set nonblocking: {}", e))?;
        }
        
        Ok(())
    }
    
    /// Check if a connection should be allowed
    pub fn should_allow(&self, ip: IpAddr) -> bool {
        // Always allow localhost
        if ip.is_loopback() {
            return true;
        }
        
        // Check if Tailscale IP
        let is_tailscale = if let IpAddr::V4(v4) = ip {
            v4.octets()[0] == 100
        } else {
            false
        };
        
        // If from Tailscale and auto-approve is on, allow
        if is_tailscale && self.config.auto_approve_tailscale {
            return true;
        }
        
        // Check approved list
        if self.config.approved_devices.contains(&ip.to_string()) {
            return true;
        }
        
        // If approval required, deny by default
        !self.config.require_approval
    }
    
    /// Get connection info for display
    pub fn connection_info(&self) -> String {
        if let Some(url) = self.tailscale.connection_url(self.config.port) {
            if let Some(ref hostname) = self.tailscale.hostname {
                format!(
                    "Connect from phone:\n   \
                     Tailscale: {}\n   \
                     Direct: {}:{}\n\n\
                     Ensure phone is on same Tailscale network.",
                    hostname,
                    self.tailscale.ip.map(|ip| ip.to_string()).unwrap_or_default(),
                    self.config.port
                )
            } else {
                format!("Connect via Tailscale: {}", url)
            }
        } else {
            match self.config.bind_mode {
                BindMode::Localhost => {
                    "Bound to localhost only. Use Tailscale port forwarding.".into()
                }
                BindMode::TailscaleOnly => {
                    "âš ï¸ Tailscale not detected. Install Tailscale to enable phone sync.".into()
                }
                BindMode::AllInterfaces => {
                    format!("âš ï¸ INSECURE: Listening on 0.0.0.0:{}", self.config.port)
                }
            }
        }
    }
    
    /// Get QR code data for phone app
    pub fn qr_data(&self) -> Option<String> {
        self.tailscale.connection_url(self.config.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tailscale_ip_detection() {
        // Just verify detection doesn't panic
        let info = TailscaleInfo::detect();
        println!("Tailscale available: {}", info.available);
        if let Some(ip) = info.ip {
            println!("Tailscale IP: {}", ip);
        }
    }
    
    #[test]
    fn test_bind_modes() {
        let config = SyncConfig::default();
        let ts = TailscaleInfo {
            available: true,
            ip: Some(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1))),
            hostname: Some("mypc".into()),
            tailnet: Some("tailnet.ts.net".into()),
            magic_dns: Some("mypc.tailnet.ts.net".into()),
        };
        
        let addr = config.bind_address(&ts).unwrap();
        assert!(addr.starts_with("100.64.0.1:"));
    }
    
    #[test]
    fn test_should_allow() {
        let server = SecureSyncServer::new(SyncConfig::default());
        
        // Localhost always allowed
        assert!(server.should_allow(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
        
        // Tailscale IPs allowed with auto_approve
        assert!(server.should_allow(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 5))));
        
        // Random IPs denied
        assert!(!server.should_allow(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 50))));
    }
}
