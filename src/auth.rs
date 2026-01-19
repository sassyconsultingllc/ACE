#![allow(dead_code, unused_variables, unused_imports)]
// ============================================================================
// SASSY BROWSER - AUTHENTICATION & LICENSING SYSTEM
// ============================================================================
// First-run key generation, device pairing, and Tailscale mesh integration
// KILLS: Paid browser sync ($99/yr), VPN services ($120/yr), device management
// ============================================================================

use std::fs;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use rand::rngs::OsRng;
use rand::{Rng, RngCore};
use sha2::{Sha256, Digest};

// ============================================================================
// LICENSE TIERS
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum LicenseTier {
    Free,           // Basic features, 3 devices
    Pro,            // All features, 10 devices, priority support
    Team,           // Unlimited devices, team management, SSO
    Enterprise,     // Custom deployment, SLA, dedicated support
}

impl LicenseTier {
    pub fn max_devices(&self) -> usize {
        match self {
            LicenseTier::Free => 3,
            LicenseTier::Pro => 10,
            LicenseTier::Team => 100,
            LicenseTier::Enterprise => usize::MAX,
        }
    }
    
    pub fn features(&self) -> Vec<&'static str> {
        match self {
            LicenseTier::Free => vec![
                "200+ file formats",
                "Basic web browsing",
                "Local file sync",
                "3 device limit",
            ],
            LicenseTier::Pro => vec![
                "All Free features",
                "Cloud sync",
                "Phone app pairing",
                "Tailscale mesh",
                "10 device limit",
                "Priority support",
            ],
            LicenseTier::Team => vec![
                "All Pro features",
                "Team workspaces",
                "SSO integration",
                "Admin dashboard",
                "100 device limit",
                "Shared bookmarks",
            ],
            LicenseTier::Enterprise => vec![
                "All Team features",
                "Custom deployment",
                "Dedicated support",
                "SLA guarantee",
                "Unlimited devices",
                "On-premise option",
            ],
        }
    }
}

// ============================================================================
// DEVICE IDENTITY
// ============================================================================

#[derive(Debug, Clone)]
pub struct DeviceIdentity {
    pub device_id: String,
    pub device_name: String,
    pub device_type: DeviceType,
    pub created_at: u64,
    pub last_seen: u64,
    pub public_key: Vec<u8>,
    pub tailscale_ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeviceType {
    Desktop,
    Laptop,
    Phone,
    Tablet,
    Server,
}

impl DeviceType {
    pub fn icon(&self) -> &'static str {
        match self {
            DeviceType::Desktop => "🖥️",
            DeviceType::Laptop => "💻",
            DeviceType::Phone => "📱",
            DeviceType::Tablet => "📲",
            DeviceType::Server => "🖧",
        }
    }
    
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "desktop" => DeviceType::Desktop,
            "laptop" => DeviceType::Laptop,
            "phone" | "mobile" => DeviceType::Phone,
            "tablet" | "ipad" => DeviceType::Tablet,
            "server" => DeviceType::Server,
            _ => DeviceType::Desktop,
        }
    }
}

// ============================================================================
// MASTER KEY & ENTROPY COLLECTION
// ============================================================================

#[derive(Debug)]
pub struct EntropyCollector {
    pool: Vec<u8>,
    mouse_events: Vec<(i32, i32, u64)>,
    key_timings: Vec<u64>,
    required_bits: usize,
    seed_bits: usize,
}

impl EntropyCollector {
    pub fn new() -> Self {
        Self {
            pool: Vec::with_capacity(256),
            mouse_events: Vec::new(),
            key_timings: Vec::new(),
            required_bits: 256, // 256-bit master key
            seed_bits: 0,
        }
    }
    
    pub fn add_mouse_event(&mut self, x: i32, y: i32) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;
        
        self.mouse_events.push((x, y, now));
        
        // Mix into pool
        self.pool.extend_from_slice(&x.to_le_bytes());
        self.pool.extend_from_slice(&y.to_le_bytes());
        self.pool.extend_from_slice(&now.to_le_bytes());
    }
    
    pub fn add_key_timing(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        
        self.key_timings.push(now);
        self.pool.extend_from_slice(&now.to_le_bytes());
    }
    
    pub fn entropy_bits(&self) -> usize {
        // Estimate entropy from collected data
        let mouse_entropy = self.mouse_events.len() * 4; // ~4 bits per mouse event
        let key_entropy = self.key_timings.len() * 2;     // ~2 bits per keystroke timing
        self.seed_bits + mouse_entropy + key_entropy
    }
    
    pub fn progress(&self) -> f32 {
        (self.entropy_bits() as f32 / self.required_bits as f32).min(1.0)
    }
    
    pub fn is_ready(&self) -> bool {
        self.entropy_bits() >= self.required_bits
    }
    
    pub fn generate_master_key(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.pool);
        
        // Add system entropy
        let mut rng = rand::thread_rng();
        let system_entropy: [u8; 32] = rng.gen();
        hasher.update(&system_entropy);
        
        // Add timestamp
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        hasher.update(&now.to_le_bytes());
        
        let result = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&result);
        key
    }

    pub fn seed_from_os(&mut self, label: &str) {
        if self.seed_bits > 0 {
            return;
        }

        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);

        self.pool.extend_from_slice(&seed);
        self.pool.extend_from_slice(label.as_bytes());
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        self.pool.extend_from_slice(&now.to_le_bytes());

        self.seed_bits = 128; // credit a baseline from OS CSPRNG
    }
}

// ============================================================================
// AUTHENTICATION MANAGER
// ============================================================================

pub struct AuthManager {
    pub config_dir: PathBuf,
    pub device_id: Option<String>,
    pub master_key: Option<[u8; 32]>,
    pub license: LicenseTier,
    pub paired_devices: Vec<DeviceIdentity>,
    pub tailscale_enabled: bool,
    pub tailscale_auth_key: Option<String>,
    pub is_first_run: bool,
    entropy_collector: EntropyCollector,
}

impl AuthManager {
        /// Returns the current generated master key (only if enough entropy is collected)
        pub fn get_master_key(&self) -> Option<[u8; 32]> {
            if self.entropy_collector.is_ready() {
                Some(self.entropy_collector.generate_master_key())
            } else {
                None
            }
        }
    pub fn new() -> Self {
        let config_dir = Self::get_config_dir();
        let is_first_run = !config_dir.join("device.key").exists();
        
        let mut manager = Self {
            config_dir,
            device_id: None,
            master_key: None,
            license: LicenseTier::Free,
            paired_devices: Vec::new(),
            tailscale_enabled: false,
            tailscale_auth_key: None,
            is_first_run,
            entropy_collector: EntropyCollector::new(),
        };
        
        if !is_first_run {
            manager.load_config();
        }
        
        manager
    }
    
    fn get_config_dir() -> PathBuf {
        let base = if cfg!(windows) {
            std::env::var("APPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("C:\\Users\\Public"))
        } else if cfg!(target_os = "macos") {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/"))
                .join("Library/Application Support")
        } else {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/"))
                .join(".config")
        };
        
        let dir = base.join("SassyBrowser");
        let _ = fs::create_dir_all(&dir);
        dir
    }
    
    pub fn add_entropy_mouse(&mut self, x: i32, y: i32) {
        self.entropy_collector.add_mouse_event(x, y);
    }
    
    pub fn add_entropy_key(&mut self) {
        self.entropy_collector.add_key_timing();
    }
    
    pub fn entropy_progress(&self) -> f32 {
        self.entropy_collector.progress()
    }
    
    pub fn is_entropy_ready(&self) -> bool {
        self.entropy_collector.is_ready()
    }

    pub fn seed_entropy(&mut self, source_label: &str) {
        self.entropy_collector.seed_from_os(source_label);
    }
    
    pub fn complete_first_run(&mut self, device_name: &str, device_type: DeviceType) -> Result<String, String> {
        if !self.entropy_collector.is_ready() {
            return Err("Not enough entropy collected. Keep moving your mouse!".to_string());
        }
        
        // Generate master key
        let master_key = self.entropy_collector.generate_master_key();
        
        // Generate device ID from master key
        let mut hasher = Sha256::new();
        hasher.update(&master_key);
        hasher.update(device_name.as_bytes());
        let device_id = hex::encode(&hasher.finalize()[..16]);
        
        // Create device identity
        let device = DeviceIdentity {
            device_id: device_id.clone(),
            device_name: device_name.to_string(),
            device_type,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            last_seen: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            public_key: master_key[..16].to_vec(), // First half as "public" identifier
            tailscale_ip: None,
        };
        
        self.device_id = Some(device_id.clone());
        self.master_key = Some(master_key);
        self.paired_devices.push(device);
        self.is_first_run = false;
        
        self.save_config()?;
        
        Ok(device_id)
    }
    
    fn save_config(&self) -> Result<(), String> {
        let _ = fs::create_dir_all(&self.config_dir);
        
        // Save device key (encrypted in production)
        if let Some(ref key) = self.master_key {
            let key_path = self.config_dir.join("device.key");
            fs::write(&key_path, hex::encode(key))
                .map_err(|e| format!("Failed to save device key: {}", e))?;
        }
        
        // Save device ID
        if let Some(ref id) = self.device_id {
            let id_path = self.config_dir.join("device.id");
            fs::write(&id_path, id)
                .map_err(|e| format!("Failed to save device ID: {}", e))?;
        }
        
        // Save license tier
        let license_path = self.config_dir.join("license.txt");
        let license_str = match self.license {
            LicenseTier::Free => "free",
            LicenseTier::Pro => "pro",
            LicenseTier::Team => "team",
            LicenseTier::Enterprise => "enterprise",
        };
        fs::write(&license_path, license_str)
            .map_err(|e| format!("Failed to save license: {}", e))?;
        
        // Save Tailscale config
        if self.tailscale_enabled {
            let ts_path = self.config_dir.join("tailscale.conf");
            let ts_config = format!(
                "enabled=true\nauth_key={}\n",
                self.tailscale_auth_key.as_deref().unwrap_or("")
            );
            fs::write(&ts_path, ts_config)
                .map_err(|e| format!("Failed to save Tailscale config: {}", e))?;
        }
        
        // Save paired devices
        let devices_path = self.config_dir.join("devices.json");
        let devices_json = self.serialize_devices();
        fs::write(&devices_path, devices_json)
            .map_err(|e| format!("Failed to save devices: {}", e))?;
        
        Ok(())
    }
    
    fn load_config(&mut self) {
        // Load device key
        let key_path = self.config_dir.join("device.key");
        if let Ok(key_hex) = fs::read_to_string(&key_path) {
            if let Ok(key_bytes) = hex::decode(key_hex.trim()) {
                if key_bytes.len() == 32 {
                    let mut key = [0u8; 32];
                    key.copy_from_slice(&key_bytes);
                    self.master_key = Some(key);
                }
            }
        }
        
        // Load device ID
        let id_path = self.config_dir.join("device.id");
        if let Ok(id) = fs::read_to_string(&id_path) {
            self.device_id = Some(id.trim().to_string());
        }
        
        // Load license
        let license_path = self.config_dir.join("license.txt");
        if let Ok(license_str) = fs::read_to_string(&license_path) {
            self.license = match license_str.trim() {
                "pro" => LicenseTier::Pro,
                "team" => LicenseTier::Team,
                "enterprise" => LicenseTier::Enterprise,
                _ => LicenseTier::Free,
            };
        }
        
        // Load Tailscale config
        let ts_path = self.config_dir.join("tailscale.conf");
        if let Ok(ts_config) = fs::read_to_string(&ts_path) {
            for line in ts_config.lines() {
                if line.starts_with("enabled=true") {
                    self.tailscale_enabled = true;
                } else if line.starts_with("auth_key=") {
                    self.tailscale_auth_key = Some(line[9..].to_string());
                }
            }
        }
        
        // Load paired devices
        let devices_path = self.config_dir.join("devices.json");
        if let Ok(devices_json) = fs::read_to_string(&devices_path) {
            self.deserialize_devices(&devices_json);
        }
    }
    
    fn serialize_devices(&self) -> String {
        let mut json = String::from("[\n");
        for (i, device) in self.paired_devices.iter().enumerate() {
            if i > 0 { json.push_str(",\n"); }
            json.push_str(&format!(
                r#"  {{"id":"{}","name":"{}","type":"{:?}","created":{},"last_seen":{},"tailscale_ip":{}}}"#,
                device.device_id,
                device.device_name,
                device.device_type,
                device.created_at,
                device.last_seen,
                device.tailscale_ip.as_ref().map(|ip| format!("\"{}\"", ip)).unwrap_or("null".to_string())
            ));
        }
        json.push_str("\n]");
        json
    }
    
    fn deserialize_devices(&mut self, _json: &str) {
        // Simple JSON parsing (production would use serde_json)
        self.paired_devices.clear();
        // TODO: Implement proper JSON parsing
    }
    
    // ========================================================================
    // DEVICE PAIRING
    // ========================================================================
    
    pub fn generate_pairing_code(&self) -> String {
        // Generate 6-digit pairing code
        let mut rng = rand::thread_rng();
        let code: u32 = rng.gen_range(100000..999999);
        format!("{}", code)
    }
    
    pub fn generate_qr_data(&self, pairing_code: &str) -> String {
        // Generate QR code data for phone app
        let device_id = self.device_id.as_deref().unwrap_or("unknown");
        let tailscale_ip = self.tailscale_auth_key.as_deref().unwrap_or("");
        
        format!(
            "sassy://pair?code={}&device={}&ts={}",
            pairing_code,
            device_id,
            tailscale_ip
        )
    }
    
    pub fn pair_device(&mut self, device: DeviceIdentity) -> Result<(), String> {
        // Check device limit
        if self.paired_devices.len() >= self.license.max_devices() {
            return Err(format!(
                "Device limit reached ({} devices for {} tier)",
                self.license.max_devices(),
                match self.license {
                    LicenseTier::Free => "Free",
                    LicenseTier::Pro => "Pro",
                    LicenseTier::Team => "Team",
                    LicenseTier::Enterprise => "Enterprise",
                }
            ));
        }
        
        // Check if device already paired
        if self.paired_devices.iter().any(|d| d.device_id == device.device_id) {
            // Update existing device
            for d in &mut self.paired_devices {
                if d.device_id == device.device_id {
                    d.last_seen = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    d.tailscale_ip = device.tailscale_ip.clone();
                }
            }
        } else {
            self.paired_devices.push(device);
        }
        
        self.save_config()?;
        Ok(())
    }
    
    pub fn unpair_device(&mut self, device_id: &str) -> Result<(), String> {
        let original_len = self.paired_devices.len();
        self.paired_devices.retain(|d| d.device_id != device_id);
        
        if self.paired_devices.len() == original_len {
            return Err("Device not found".to_string());
        }
        
        self.save_config()?;
        Ok(())
    }
}

// ============================================================================
// TAILSCALE INTEGRATION
// ============================================================================

pub struct TailscaleManager {
    pub enabled: bool,
    pub auth_key: Option<String>,
    pub hostname: Option<String>,
    pub ip_address: Option<String>,
    pub peers: Vec<TailscalePeer>,
    pub status: TailscaleStatus,
}

#[derive(Debug, Clone)]
pub struct TailscalePeer {
    pub hostname: String,
    pub ip_address: String,
    pub os: String,
    pub online: bool,
    pub last_seen: u64,
    pub is_sassy_device: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TailscaleStatus {
    NotInstalled,
    Stopped,
    NeedsLogin,
    Running,
    Error(String),
}

impl TailscaleManager {
    pub fn new() -> Self {
        Self {
            enabled: false,
            auth_key: None,
            hostname: None,
            ip_address: None,
            peers: Vec::new(),
            status: TailscaleStatus::NotInstalled,
        }
    }
    
    pub fn check_installation(&mut self) -> bool {
        // Check if Tailscale CLI is available
        let output = if cfg!(windows) {
            std::process::Command::new("where")
                .arg("tailscale")
                .output()
        } else {
            std::process::Command::new("which")
                .arg("tailscale")
                .output()
        };
        
        match output {
            Ok(o) if o.status.success() => {
                self.status = TailscaleStatus::Stopped;
                true
            }
            _ => {
                self.status = TailscaleStatus::NotInstalled;
                false
            }
        }
    }
    
    pub fn get_status(&mut self) -> TailscaleStatus {
        let output = std::process::Command::new("tailscale")
            .arg("status")
            .arg("--json")
            .output();
        
        match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                self.parse_status(&stdout);
                self.status.clone()
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                if stderr.contains("not logged in") || stderr.contains("NeedsLogin") {
                    self.status = TailscaleStatus::NeedsLogin;
                } else if stderr.contains("stopped") {
                    self.status = TailscaleStatus::Stopped;
                } else {
                    self.status = TailscaleStatus::Error(stderr.to_string());
                }
                self.status.clone()
            }
            Err(e) => {
                self.status = TailscaleStatus::Error(e.to_string());
                self.status.clone()
            }
        }
    }
    
    fn parse_status(&mut self, json: &str) {
        // Simple JSON parsing for Tailscale status
        // Production would use serde_json
        
        self.peers.clear();
        self.status = TailscaleStatus::Running;
        
        // Extract Self IP
        if let Some(start) = json.find("\"TailscaleIPs\"") {
            if let Some(ip_start) = json[start..].find("\"100.") {
                let ip_str = &json[start + ip_start + 1..];
                if let Some(ip_end) = ip_str.find("\"") {
                    self.ip_address = Some(ip_str[..ip_end].to_string());
                }
            }
        }
        
        // Extract hostname
        if let Some(start) = json.find("\"Self\"") {
            if let Some(hn_start) = json[start..].find("\"HostName\":\"") {
                let hn_str = &json[start + hn_start + 12..];
                if let Some(hn_end) = hn_str.find("\"") {
                    self.hostname = Some(hn_str[..hn_end].to_string());
                }
            }
        }
    }
    
    pub fn start(&mut self) -> Result<(), String> {
        let output = std::process::Command::new("tailscale")
            .arg("up")
            .output()
            .map_err(|e| format!("Failed to start Tailscale: {}", e))?;
        
        if output.status.success() {
            self.status = TailscaleStatus::Running;
            self.enabled = true;
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Tailscale error: {}", stderr))
        }
    }
    
    pub fn start_with_auth_key(&mut self, auth_key: &str) -> Result<(), String> {
        let output = std::process::Command::new("tailscale")
            .arg("up")
            .arg("--authkey")
            .arg(auth_key)
            .output()
            .map_err(|e| format!("Failed to start Tailscale: {}", e))?;
        
        if output.status.success() {
            self.auth_key = Some(auth_key.to_string());
            self.status = TailscaleStatus::Running;
            self.enabled = true;
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Tailscale error: {}", stderr))
        }
    }
    
    pub fn stop(&mut self) -> Result<(), String> {
        let output = std::process::Command::new("tailscale")
            .arg("down")
            .output()
            .map_err(|e| format!("Failed to stop Tailscale: {}", e))?;
        
        if output.status.success() {
            self.status = TailscaleStatus::Stopped;
            self.enabled = false;
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Tailscale error: {}", stderr))
        }
    }
    
    pub fn get_peers(&mut self) -> Vec<TailscalePeer> {
        let output = std::process::Command::new("tailscale")
            .arg("status")
            .output();
        
        if let Ok(o) = output {
            if o.status.success() {
                let stdout = String::from_utf8_lossy(&o.stdout);
                self.parse_peers(&stdout);
            }
        }
        
        self.peers.clone()
    }
    
    fn parse_peers(&mut self, status: &str) {
        self.peers.clear();
        
        for line in status.lines().skip(1) { // Skip header
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let peer = TailscalePeer {
                    ip_address: parts[0].to_string(),
                    hostname: parts[1].to_string(),
                    os: parts.get(3).unwrap_or(&"unknown").to_string(),
                    online: parts.get(2).map(|s| *s != "offline").unwrap_or(false),
                    last_seen: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    is_sassy_device: parts[1].to_lowercase().contains("sassy"),
                };
                self.peers.push(peer);
            }
        }
    }
    
    pub fn send_file(&self, peer_ip: &str, file_path: &str) -> Result<(), String> {
        // Use Tailscale's built-in file sharing
        let output = std::process::Command::new("tailscale")
            .arg("file")
            .arg("cp")
            .arg(file_path)
            .arg(format!("{}:", peer_ip))
            .output()
            .map_err(|e| format!("Failed to send file: {}", e))?;
        
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("File transfer failed: {}", stderr))
        }
    }
    
    pub fn receive_files(&self) -> Result<Vec<String>, String> {
        // Check for incoming files
        let output = std::process::Command::new("tailscale")
            .arg("file")
            .arg("get")
            .arg("--wait=1s")
            .arg(".")
            .output()
            .map_err(|e| format!("Failed to receive files: {}", e))?;
        
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let files: Vec<String> = stdout.lines()
                .map(|s| s.to_string())
                .collect();
            Ok(files)
        } else {
            Ok(Vec::new()) // No files waiting
        }
    }
}

// ============================================================================
// PHONE APP SYNC PROTOCOL
// ============================================================================

pub struct PhoneSync {
    pub connected: bool,
    pub phone_device: Option<DeviceIdentity>,
    pub sync_queue: Vec<SyncItem>,
    pub last_sync: u64,
}

#[derive(Debug, Clone)]
pub struct SyncItem {
    pub item_type: SyncType,
    pub data: Vec<u8>,
    pub timestamp: u64,
    pub synced: bool,
}

#[derive(Debug, Clone)]
pub enum SyncType {
    Bookmark,
    History,
    Tab,
    File,
    Setting,
    Credential,
}

impl PhoneSync {
    pub fn new() -> Self {
        Self {
            connected: false,
            phone_device: None,
            sync_queue: Vec::new(),
            last_sync: 0,
        }
    }
    
    pub fn connect_via_tailscale(&mut self, tailscale: &TailscaleManager) -> Result<(), String> {
        // Find Sassy phone app in Tailscale peers
        for peer in &tailscale.peers {
            if peer.is_sassy_device && peer.os.to_lowercase().contains("android")
                || peer.os.to_lowercase().contains("ios") {
                self.phone_device = Some(DeviceIdentity {
                    device_id: peer.hostname.clone(),
                    device_name: peer.hostname.clone(),
                    device_type: DeviceType::Phone,
                    created_at: 0,
                    last_seen: peer.last_seen,
                    public_key: Vec::new(),
                    tailscale_ip: Some(peer.ip_address.clone()),
                });
                self.connected = true;
                return Ok(());
            }
        }
        
        Err("No Sassy phone app found in Tailscale network".to_string())
    }
    
    pub fn queue_sync(&mut self, item_type: SyncType, data: Vec<u8>) {
        self.sync_queue.push(SyncItem {
            item_type,
            data,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            synced: false,
        });
    }
    
    pub fn sync_all(&mut self, tailscale: &TailscaleManager) -> Result<usize, String> {
        if !self.connected {
            return Err("Phone not connected".to_string());
        }

        let phone_ip = self.phone_device.as_ref()
            .and_then(|d| d.tailscale_ip.clone())
            .ok_or("No phone IP available")?;
        // Use tailscale reference to keep it active and mark phone IP as touched
        let _peer_count = tailscale.peers.len();
        let _ = &phone_ip;
        
        let mut synced_count = 0;
        
        for item in &mut self.sync_queue {
            if !item.synced {
                // In production, this would use a proper sync protocol
                // For now, we just mark as synced
                item.synced = true;
                synced_count += 1;
            }
        }
        
        self.last_sync = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Ok(synced_count)
    }
}

// ============================================================================
// FIRST RUN UI STATE
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum FirstRunStep {
    Welcome,
    EntropyCollection,
    DeviceSetup,
    TailscaleSetup,
    PhonePairing,
    Complete,
}

pub struct FirstRunState {
    pub step: FirstRunStep,
    pub device_name: String,
    pub device_type: DeviceType,
    pub enable_tailscale: bool,
    pub enable_phone_sync: bool,
    pub pairing_code: Option<String>,
    pub error_message: Option<String>,
    pub entropy_started_at: Option<Instant>,
    pub entropy_seeded: bool,
    pub entropy_seed_label: String,
    pub entropy_min_seconds: u64,
}

impl Default for FirstRunState {
    fn default() -> Self {
        Self {
            step: FirstRunStep::Welcome,
            device_name: hostname::get()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|_| "My Computer".to_string()),
            device_type: DeviceType::Desktop,
            enable_tailscale: false,
            enable_phone_sync: false,
            pairing_code: None,
            error_message: None,
            entropy_started_at: None,
            entropy_seeded: false,
            entropy_seed_label: "OS random seed".to_string(),
            entropy_min_seconds: 30,
        }
    }
}

impl FirstRunState {
    pub fn next_step(&mut self) {
        self.step = match self.step {
            FirstRunStep::Welcome => FirstRunStep::EntropyCollection,
            FirstRunStep::EntropyCollection => FirstRunStep::DeviceSetup,
            FirstRunStep::DeviceSetup => {
                if self.enable_tailscale {
                    FirstRunStep::TailscaleSetup
                } else if self.enable_phone_sync {
                    FirstRunStep::PhonePairing
                } else {
                    FirstRunStep::Complete
                }
            }
            FirstRunStep::TailscaleSetup => {
                if self.enable_phone_sync {
                    FirstRunStep::PhonePairing
                } else {
                    FirstRunStep::Complete
                }
            }
            FirstRunStep::PhonePairing => FirstRunStep::Complete,
            FirstRunStep::Complete => FirstRunStep::Complete,
        };
    }
    
    pub fn prev_step(&mut self) {
        self.step = match self.step {
            FirstRunStep::Welcome => FirstRunStep::Welcome,
            FirstRunStep::EntropyCollection => FirstRunStep::Welcome,
            FirstRunStep::DeviceSetup => FirstRunStep::EntropyCollection,
            FirstRunStep::TailscaleSetup => FirstRunStep::DeviceSetup,
            FirstRunStep::PhonePairing => {
                if self.enable_tailscale {
                    FirstRunStep::TailscaleSetup
                } else {
                    FirstRunStep::DeviceSetup
                }
            }
            FirstRunStep::Complete => FirstRunStep::PhonePairing,
        };
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_entropy_collection() {
        let mut collector = EntropyCollector::new();
        
        // Simulate mouse movements
        for i in 0..100 {
            collector.add_mouse_event(i * 10, i * 5);
        }
        
        // Simulate key presses
        for _ in 0..50 {
            collector.add_key_timing();
        }
        
        assert!(collector.entropy_bits() > 0);
        assert!(collector.is_ready());
        
        let key = collector.generate_master_key();
        assert_eq!(key.len(), 32);
    }
    
    #[test]
    fn test_license_tiers() {
        assert_eq!(LicenseTier::Free.max_devices(), 3);
        assert_eq!(LicenseTier::Pro.max_devices(), 10);
        assert_eq!(LicenseTier::Team.max_devices(), 100);
        assert!(LicenseTier::Enterprise.max_devices() > 1000);
    }
    
    #[test]
    fn test_device_type_icons() {
        assert_eq!(DeviceType::Desktop.icon(), "🖥️");
        assert_eq!(DeviceType::Phone.icon(), "📱");
        assert_eq!(DeviceType::Tablet.icon(), "📲");
    }
}
