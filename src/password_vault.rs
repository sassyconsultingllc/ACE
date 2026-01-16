// ============================================================================
// SASSY BROWSER - PASSWORD VAULT
// ============================================================================
// Built-in password manager with ChaCha20-Poly1305 encryption, Argon2id PIN
// Syncs via Tailscale. REPLACES: LastPass, 1Password, Chrome passwords
// ============================================================================

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use rand::Rng;
use sha2::{Sha256, Digest};
use zeroize::Zeroize;

// ============================================================================
// CREDENTIAL ENTRY
// ============================================================================

#[derive(Debug, Clone)]
pub struct Credential {
    pub id: String,
    pub title: String,
    pub username: String,
    pub password: String,  // Decrypted in memory, encrypted at rest
    pub url: String,
    pub notes: String,
    pub folder: Option<String>,
    pub tags: Vec<String>,
    pub created_at: u64,
    pub modified_at: u64,
    pub last_used: Option<u64>,
    pub use_count: u32,
    pub favorite: bool,
    pub totp_secret: Option<String>,  // For 2FA
    pub custom_fields: HashMap<String, String>,
}

impl Credential {
    pub fn new(title: &str, username: &str, password: &str, url: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            id: generate_id(),
            title: title.to_string(),
            username: username.to_string(),
            password: password.to_string(),
            url: url.to_string(),
            notes: String::new(),
            folder: None,
            tags: Vec::new(),
            created_at: now,
            modified_at: now,
            last_used: None,
            use_count: 0,
            favorite: false,
            totp_secret: None,
            custom_fields: HashMap::new(),
        }
    }
    
    pub fn domain(&self) -> String {
        extract_domain(&self.url)
    }
    
    pub fn matches_url(&self, url: &str) -> bool {
        let cred_domain = self.domain().to_lowercase();
        let url_domain = extract_domain(url).to_lowercase();
        
        // Exact match or subdomain match
        cred_domain == url_domain || 
            url_domain.ends_with(&format!(".{}", cred_domain)) ||
            cred_domain.ends_with(&format!(".{}", url_domain))
    }
    
    pub fn password_strength(&self) -> PasswordStrength {
        analyze_password(&self.password)
    }
    
    pub fn record_use(&mut self) {
        self.use_count += 1;
        self.last_used = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
    }
}

// ============================================================================
// PASSWORD STRENGTH
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum PasswordStrength {
    VeryWeak,
    Weak,
    Fair,
    Strong,
    VeryStrong,
}

impl PasswordStrength {
    pub fn color(&self) -> [u8; 3] {
        match self {
            PasswordStrength::VeryWeak => [255, 0, 0],
            PasswordStrength::Weak => [255, 128, 0],
            PasswordStrength::Fair => [255, 255, 0],
            PasswordStrength::Strong => [128, 255, 0],
            PasswordStrength::VeryStrong => [0, 255, 0],
        }
    }
    
    pub fn score(&self) -> u8 {
        match self {
            PasswordStrength::VeryWeak => 1,
            PasswordStrength::Weak => 2,
            PasswordStrength::Fair => 3,
            PasswordStrength::Strong => 4,
            PasswordStrength::VeryStrong => 5,
        }
    }
}

pub fn analyze_password(password: &str) -> PasswordStrength {
    let len = password.len();
    let has_lower = password.chars().any(|c| c.is_lowercase());
    let has_upper = password.chars().any(|c| c.is_uppercase());
    let has_digit = password.chars().any(|c| c.is_numeric());
    let has_special = password.chars().any(|c| !c.is_alphanumeric());
    
    let mut score: u32 = 0;
    
    // Length scoring
    if len >= 8 { score += 1; }
    if len >= 12 { score += 1; }
    if len >= 16 { score += 1; }
    if len >= 20 { score += 1; }
    
    // Character variety
    if has_lower { score += 1; }
    if has_upper { score += 1; }
    if has_digit { score += 1; }
    if has_special { score += 2; }
    
    // Penalize common patterns
    let lower = password.to_lowercase();
    let common = ["password", "123456", "qwerty", "admin", "letmein", "welcome"];
    if common.iter().any(|p| lower.contains(p)) {
        score = score.saturating_sub(3);
    }
    
    match score {
        0..=2 => PasswordStrength::VeryWeak,
        3..=4 => PasswordStrength::Weak,
        5..=6 => PasswordStrength::Fair,
        7..=8 => PasswordStrength::Strong,
        _ => PasswordStrength::VeryStrong,
    }
}

// ============================================================================
// PASSWORD GENERATOR
// ============================================================================

#[derive(Debug, Clone)]
pub struct PasswordGeneratorOptions {
    pub length: usize,
    pub lowercase: bool,
    pub uppercase: bool,
    pub numbers: bool,
    pub symbols: bool,
    pub exclude_ambiguous: bool,  // 0, O, l, 1, I
    pub exclude_chars: String,
    pub min_numbers: usize,
    pub min_symbols: usize,
}

impl Default for PasswordGeneratorOptions {
    fn default() -> Self {
        Self {
            length: 20,
            lowercase: true,
            uppercase: true,
            numbers: true,
            symbols: true,
            exclude_ambiguous: true,
            exclude_chars: String::new(),
            min_numbers: 2,
            min_symbols: 2,
        }
    }
}

pub fn generate_password(opts: &PasswordGeneratorOptions) -> String {
    let mut rng = rand::thread_rng();
    
    let lowercase = "abcdefghjkmnpqrstuvwxyz"; // Ambiguous removed
    let uppercase = "ABCDEFGHJKMNPQRSTUVWXYZ";
    let numbers = "23456789";
    let symbols = "!@#$%^&*()_+-=[]{}|;:,.<>?";
    
    let lowercase_full = "abcdefghijklmnopqrstuvwxyz";
    let uppercase_full = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let numbers_full = "0123456789";
    
    let mut charset = String::new();
    
    if opts.lowercase {
        charset.push_str(if opts.exclude_ambiguous { lowercase } else { lowercase_full });
    }
    if opts.uppercase {
        charset.push_str(if opts.exclude_ambiguous { uppercase } else { uppercase_full });
    }
    if opts.numbers {
        charset.push_str(if opts.exclude_ambiguous { numbers } else { numbers_full });
    }
    if opts.symbols {
        charset.push_str(symbols);
    }
    
    // Remove excluded chars
    for c in opts.exclude_chars.chars() {
        charset = charset.replace(c, "");
    }
    
    if charset.is_empty() {
        return String::new();
    }
    
    let chars: Vec<char> = charset.chars().collect();
    let mut password: Vec<char> = Vec::with_capacity(opts.length);
    
    // Ensure minimums
    if opts.numbers && opts.min_numbers > 0 {
        let num_chars: Vec<char> = (if opts.exclude_ambiguous { numbers } else { numbers_full })
            .chars().collect();
        for _ in 0..opts.min_numbers {
            password.push(num_chars[rng.gen_range(0..num_chars.len())]);
        }
    }
    
    if opts.symbols && opts.min_symbols > 0 {
        let sym_chars: Vec<char> = symbols.chars().collect();
        for _ in 0..opts.min_symbols {
            password.push(sym_chars[rng.gen_range(0..sym_chars.len())]);
        }
    }
    
    // Fill rest
    while password.len() < opts.length {
        password.push(chars[rng.gen_range(0..chars.len())]);
    }
    
    // Shuffle
    for i in (1..password.len()).rev() {
        let j = rng.gen_range(0..=i);
        password.swap(i, j);
    }
    
    password.into_iter().collect()
}

// ============================================================================
// VAULT ENCRYPTION
// ============================================================================

struct VaultCrypto {
    master_key: [u8; 32],
}

impl VaultCrypto {
    fn new(pin: &str, salt: &[u8]) -> Result<Self, String> {
        // Derive key from PIN using Argon2id
        let argon2 = Argon2::default();
        
        let mut key = [0u8; 32];
        argon2.hash_password_into(
            pin.as_bytes(),
            salt,
            &mut key
        ).map_err(|e| format!("Key derivation failed: {}", e))?;
        
        Ok(Self { master_key: key })
    }
    
    fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let cipher = ChaCha20Poly1305::new_from_slice(&self.master_key)
            .map_err(|e| format!("Cipher init failed: {}", e))?;
        
        // Generate random nonce
        let mut rng = rand::thread_rng();
        let mut nonce_bytes = [0u8; 12];
        rng.fill(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = cipher.encrypt(nonce, plaintext)
            .map_err(|e| format!("Encryption failed: {}", e))?;
        
        // Prepend nonce to ciphertext
        let mut result = nonce_bytes.to_vec();
        result.extend(ciphertext);
        
        Ok(result)
    }
    
    fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, String> {
        if ciphertext.len() < 12 {
            return Err("Ciphertext too short".to_string());
        }
        
        let cipher = ChaCha20Poly1305::new_from_slice(&self.master_key)
            .map_err(|e| format!("Cipher init failed: {}", e))?;
        
        let nonce = Nonce::from_slice(&ciphertext[..12]);
        let plaintext = cipher.decrypt(nonce, &ciphertext[12..])
            .map_err(|e| format!("Decryption failed: {}", e))?;
        
        Ok(plaintext)
    }
}

impl Drop for VaultCrypto {
    fn drop(&mut self) {
        self.master_key.zeroize();
    }
}

// ============================================================================
// PASSWORD VAULT
// ============================================================================

pub struct PasswordVault {
    credentials: Vec<Credential>,
    folders: Vec<String>,
    config_dir: PathBuf,
    salt: [u8; 16],
    pin_hash: Option<String>,
    is_unlocked: bool,
    crypto: Option<VaultCrypto>,
    auto_lock_seconds: u64,
    last_activity: std::time::Instant,
    breach_check_enabled: bool,
}

impl PasswordVault {
    pub fn new(config_dir: PathBuf) -> Self {
        Self {
            credentials: Vec::new(),
            folders: vec!["Personal".to_string(), "Work".to_string(), "Finance".to_string()],
            config_dir,
            salt: [0u8; 16],
            pin_hash: None,
            is_unlocked: false,
            crypto: None,
            auto_lock_seconds: 300, // 5 minutes
            last_activity: std::time::Instant::now(),
            breach_check_enabled: true,
        }
    }
    
    pub fn is_setup(&self) -> bool {
        self.pin_hash.is_some()
    }
    
    pub fn is_unlocked(&self) -> bool {
        self.is_unlocked
    }
    
    pub fn setup(&mut self, pin: &str) -> Result<(), String> {
        if pin.len() < 4 {
            return Err("PIN must be at least 4 characters".to_string());
        }
        
        // Generate salt
        let mut rng = rand::thread_rng();
        rng.fill(&mut self.salt);
        
        // Hash PIN with Argon2id
        let argon2 = Argon2::default();
        let salt_string = SaltString::encode_b64(&self.salt)
            .map_err(|e| format!("Salt encoding failed: {}", e))?;
        
        let hash = argon2.hash_password(pin.as_bytes(), &salt_string)
            .map_err(|e| format!("Hashing failed: {}", e))?;
        
        self.pin_hash = Some(hash.to_string());
        
        // Initialize crypto
        self.crypto = Some(VaultCrypto::new(pin, &self.salt)?);
        self.is_unlocked = true;
        self.last_activity = std::time::Instant::now();
        
        self.save()?;
        
        Ok(())
    }
    
    pub fn unlock(&mut self, pin: &str) -> Result<(), String> {
        let hash_str = self.pin_hash.as_ref()
            .ok_or("Vault not set up")?;
        
        let hash = PasswordHash::new(hash_str)
            .map_err(|e| format!("Invalid hash: {}", e))?;
        
        let argon2 = Argon2::default();
        argon2.verify_password(pin.as_bytes(), &hash)
            .map_err(|_| "Incorrect PIN")?;
        
        self.crypto = Some(VaultCrypto::new(pin, &self.salt)?);
        self.is_unlocked = true;
        self.last_activity = std::time::Instant::now();
        
        self.load()?;
        
        Ok(())
    }
    
    pub fn lock(&mut self) {
        // Clear sensitive data
        for cred in &mut self.credentials {
            cred.password.zeroize();
            if let Some(ref mut totp) = cred.totp_secret {
                totp.zeroize();
            }
        }
        self.credentials.clear();
        self.crypto = None;
        self.is_unlocked = false;
    }
    
    pub fn check_auto_lock(&mut self) -> bool {
        if self.is_unlocked && self.last_activity.elapsed().as_secs() > self.auto_lock_seconds {
            self.lock();
            return true;
        }
        false
    }
    
    pub fn touch(&mut self) {
        self.last_activity = std::time::Instant::now();
    }
    
    // ========================================================================
    // CREDENTIAL MANAGEMENT
    // ========================================================================
    
    pub fn add(&mut self, credential: Credential) -> Result<(), String> {
        if !self.is_unlocked {
            return Err("Vault is locked".to_string());
        }
        
        self.credentials.push(credential);
        self.save()?;
        self.touch();
        
        Ok(())
    }
    
    pub fn update(&mut self, id: &str, credential: Credential) -> Result<(), String> {
        if !self.is_unlocked {
            return Err("Vault is locked".to_string());
        }
        
        if let Some(cred) = self.credentials.iter_mut().find(|c| c.id == id) {
            *cred = credential;
            cred.modified_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            self.save()?;
            self.touch();
            Ok(())
        } else {
            Err("Credential not found".to_string())
        }
    }
    
    pub fn delete(&mut self, id: &str) -> Result<(), String> {
        if !self.is_unlocked {
            return Err("Vault is locked".to_string());
        }
        
        let original_len = self.credentials.len();
        self.credentials.retain(|c| c.id != id);
        
        if self.credentials.len() == original_len {
            return Err("Credential not found".to_string());
        }
        
        self.save()?;
        self.touch();
        
        Ok(())
    }
    
    pub fn get(&self, id: &str) -> Option<&Credential> {
        if !self.is_unlocked {
            return None;
        }
        self.credentials.iter().find(|c| c.id == id)
    }
    
    pub fn find_for_url(&self, url: &str) -> Vec<&Credential> {
        if !self.is_unlocked {
            return Vec::new();
        }
        
        self.credentials.iter()
            .filter(|c| c.matches_url(url))
            .collect()
    }
    
    pub fn search(&self, query: &str) -> Vec<&Credential> {
        if !self.is_unlocked {
            return Vec::new();
        }
        
        let query_lower = query.to_lowercase();
        
        self.credentials.iter()
            .filter(|c| {
                c.title.to_lowercase().contains(&query_lower) ||
                c.username.to_lowercase().contains(&query_lower) ||
                c.url.to_lowercase().contains(&query_lower) ||
                c.notes.to_lowercase().contains(&query_lower) ||
                c.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
            })
            .collect()
    }
    
    pub fn all(&self) -> &[Credential] {
        if !self.is_unlocked {
            return &[];
        }
        &self.credentials
    }
    
    pub fn by_folder(&self, folder: Option<&str>) -> Vec<&Credential> {
        if !self.is_unlocked {
            return Vec::new();
        }
        
        self.credentials.iter()
            .filter(|c| c.folder.as_deref() == folder)
            .collect()
    }
    
    pub fn favorites(&self) -> Vec<&Credential> {
        if !self.is_unlocked {
            return Vec::new();
        }
        
        self.credentials.iter()
            .filter(|c| c.favorite)
            .collect()
    }
    
    pub fn recently_used(&self, count: usize) -> Vec<&Credential> {
        if !self.is_unlocked {
            return Vec::new();
        }
        
        let mut creds: Vec<_> = self.credentials.iter()
            .filter(|c| c.last_used.is_some())
            .collect();
        
        creds.sort_by(|a, b| b.last_used.cmp(&a.last_used));
        creds.truncate(count);
        creds
    }
    
    pub fn weak_passwords(&self) -> Vec<&Credential> {
        if !self.is_unlocked {
            return Vec::new();
        }
        
        self.credentials.iter()
            .filter(|c| {
                let strength = c.password_strength();
                matches!(strength, PasswordStrength::VeryWeak | PasswordStrength::Weak)
            })
            .collect()
    }
    
    pub fn reused_passwords(&self) -> HashMap<String, Vec<&Credential>> {
        let mut groups: HashMap<String, Vec<&Credential>> = HashMap::new();
        
        if !self.is_unlocked {
            return groups;
        }
        
        for cred in &self.credentials {
            // Hash password for comparison (don't store actual password as key)
            let mut hasher = Sha256::new();
            hasher.update(cred.password.as_bytes());
            let hash = format!("{:x}", hasher.finalize());
            
            groups.entry(hash).or_default().push(cred);
        }
        
        // Only return groups with duplicates
        groups.into_iter()
            .filter(|(_, v)| v.len() > 1)
            .collect()
    }
    
    // ========================================================================
    // PERSISTENCE
    // ========================================================================
    
    fn vault_path(&self) -> PathBuf {
        self.config_dir.join("vault.enc")
    }
    
    fn config_path(&self) -> PathBuf {
        self.config_dir.join("vault.conf")
    }
    
    pub fn save(&self) -> Result<(), String> {
        let crypto = self.crypto.as_ref()
            .ok_or("Vault is locked")?;
        
        // Serialize credentials
        let mut data = String::new();
        for cred in &self.credentials {
            data.push_str(&serialize_credential(cred));
            data.push_str("\n---\n");
        }
        
        // Encrypt
        let encrypted = crypto.encrypt(data.as_bytes())?;
        
        // Write to file
        let _ = fs::create_dir_all(&self.config_dir);
        fs::write(self.vault_path(), &encrypted)
            .map_err(|e| format!("Failed to write vault: {}", e))?;
        
        // Save config (non-sensitive)
        let config = format!(
            "salt={}\npin_hash={}\nauto_lock={}\n",
            hex::encode(&self.salt),
            self.pin_hash.as_deref().unwrap_or(""),
            self.auto_lock_seconds
        );
        fs::write(self.config_path(), config)
            .map_err(|e| format!("Failed to write config: {}", e))?;
        
        Ok(())
    }
    
    pub fn load(&mut self) -> Result<(), String> {
        // Load config
        if let Ok(config) = fs::read_to_string(self.config_path()) {
            for line in config.lines() {
                if let Some(value) = line.strip_prefix("salt=") {
                    if let Ok(salt_bytes) = hex::decode(value) {
                        if salt_bytes.len() == 16 {
                            self.salt.copy_from_slice(&salt_bytes);
                        }
                    }
                } else if let Some(value) = line.strip_prefix("pin_hash=") {
                    if !value.is_empty() {
                        self.pin_hash = Some(value.to_string());
                    }
                } else if let Some(value) = line.strip_prefix("auto_lock=") {
                    if let Ok(secs) = value.parse() {
                        self.auto_lock_seconds = secs;
                    }
                }
            }
        }
        
        // Load encrypted vault
        let crypto = match &self.crypto {
            Some(c) => c,
            None => return Ok(()), // Not unlocked yet
        };
        
        let encrypted = match fs::read(self.vault_path()) {
            Ok(data) => data,
            Err(_) => return Ok(()), // No vault file yet
        };
        
        let decrypted = crypto.decrypt(&encrypted)?;
        let data = String::from_utf8(decrypted)
            .map_err(|_| "Invalid vault data")?;
        
        // Parse credentials
        self.credentials.clear();
        for block in data.split("\n---\n") {
            if !block.trim().is_empty() {
                if let Some(cred) = deserialize_credential(block) {
                    self.credentials.push(cred);
                }
            }
        }
        
        Ok(())
    }
    
    // ========================================================================
    // EXPORT/IMPORT
    // ========================================================================
    
    pub fn export_csv(&self) -> Result<String, String> {
        if !self.is_unlocked {
            return Err("Vault is locked".to_string());
        }
        
        let mut csv = String::from("title,url,username,password,notes,folder,tags\n");
        
        for cred in &self.credentials {
            csv.push_str(&format!(
                "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
                escape_csv(&cred.title),
                escape_csv(&cred.url),
                escape_csv(&cred.username),
                escape_csv(&cred.password),
                escape_csv(&cred.notes),
                escape_csv(cred.folder.as_deref().unwrap_or("")),
                escape_csv(&cred.tags.join(",")),
            ));
        }
        
        Ok(csv)
    }
    
    pub fn import_csv(&mut self, csv: &str) -> Result<usize, String> {
        if !self.is_unlocked {
            return Err("Vault is locked".to_string());
        }
        
        let mut count = 0;
        let mut lines = csv.lines();
        
        // Skip header
        lines.next();
        
        for line in lines {
            let fields: Vec<&str> = parse_csv_line(line);
            if fields.len() >= 4 {
                let mut cred = Credential::new(
                    fields.get(0).unwrap_or(&""),
                    fields.get(2).unwrap_or(&""),
                    fields.get(3).unwrap_or(&""),
                    fields.get(1).unwrap_or(&""),
                );
                
                if let Some(notes) = fields.get(4) {
                    cred.notes = notes.to_string();
                }
                if let Some(folder) = fields.get(5) {
                    if !folder.is_empty() {
                        cred.folder = Some(folder.to_string());
                    }
                }
                if let Some(tags) = fields.get(6) {
                    cred.tags = tags.split(',').map(|s| s.trim().to_string()).collect();
                }
                
                self.credentials.push(cred);
                count += 1;
            }
        }
        
        if count > 0 {
            self.save()?;
        }
        
        Ok(count)
    }
}

// ============================================================================
// HELPERS
// ============================================================================

fn generate_id() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 16] = rng.gen();
    hex::encode(bytes)
}

fn extract_domain(url: &str) -> String {
    let url = url.trim_start_matches("https://")
        .trim_start_matches("http://");
    
    url.split('/').next()
        .unwrap_or(url)
        .split(':').next()
        .unwrap_or(url)
        .to_string()
}

fn serialize_credential(cred: &Credential) -> String {
    format!(
        "id={}\ntitle={}\nusername={}\npassword={}\nurl={}\nnotes={}\nfolder={}\ntags={}\ncreated={}\nmodified={}\nlast_used={}\nuse_count={}\nfavorite={}\ntotp={}",
        cred.id,
        base64::encode(&cred.title),
        base64::encode(&cred.username),
        base64::encode(&cred.password),
        base64::encode(&cred.url),
        base64::encode(&cred.notes),
        cred.folder.as_deref().unwrap_or(""),
        cred.tags.join(","),
        cred.created_at,
        cred.modified_at,
        cred.last_used.map(|t| t.to_string()).unwrap_or_default(),
        cred.use_count,
        cred.favorite,
        cred.totp_secret.as_deref().map(|s| base64::encode(s)).unwrap_or_default(),
    )
}

fn deserialize_credential(data: &str) -> Option<Credential> {
    let mut cred = Credential::new("", "", "", "");
    
    for line in data.lines() {
        if let Some(value) = line.strip_prefix("id=") {
            cred.id = value.to_string();
        } else if let Some(value) = line.strip_prefix("title=") {
            cred.title = String::from_utf8(base64::decode(value).ok()?).ok()?;
        } else if let Some(value) = line.strip_prefix("username=") {
            cred.username = String::from_utf8(base64::decode(value).ok()?).ok()?;
        } else if let Some(value) = line.strip_prefix("password=") {
            cred.password = String::from_utf8(base64::decode(value).ok()?).ok()?;
        } else if let Some(value) = line.strip_prefix("url=") {
            cred.url = String::from_utf8(base64::decode(value).ok()?).ok()?;
        } else if let Some(value) = line.strip_prefix("notes=") {
            cred.notes = String::from_utf8(base64::decode(value).ok()?).ok()?;
        } else if let Some(value) = line.strip_prefix("folder=") {
            if !value.is_empty() {
                cred.folder = Some(value.to_string());
            }
        } else if let Some(value) = line.strip_prefix("tags=") {
            cred.tags = value.split(',').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect();
        } else if let Some(value) = line.strip_prefix("created=") {
            cred.created_at = value.parse().unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("modified=") {
            cred.modified_at = value.parse().unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("last_used=") {
            if !value.is_empty() {
                cred.last_used = value.parse().ok();
            }
        } else if let Some(value) = line.strip_prefix("use_count=") {
            cred.use_count = value.parse().unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("favorite=") {
            cred.favorite = value == "true";
        } else if let Some(value) = line.strip_prefix("totp=") {
            if !value.is_empty() {
                cred.totp_secret = String::from_utf8(base64::decode(value).ok()?).ok();
            }
        }
    }
    
    if cred.id.is_empty() {
        None
    } else {
        Some(cred)
    }
}

fn escape_csv(s: &str) -> String {
    s.replace("\"", "\"\"")
}

fn parse_csv_line(line: &str) -> Vec<&str> {
    // Simple CSV parsing (doesn't handle all edge cases)
    let mut fields = Vec::new();
    let mut in_quotes = false;
    let mut start = 0;
    
    for (i, c) in line.char_indices() {
        if c == '"' {
            in_quotes = !in_quotes;
        } else if c == ',' && !in_quotes {
            let field = &line[start..i].trim_matches('"');
            fields.push(*field);
            start = i + 1;
        }
    }
    
    // Last field
    if start < line.len() {
        fields.push(line[start..].trim_matches('"'));
    }
    
    fields
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_password_strength() {
        assert_eq!(analyze_password("123"), PasswordStrength::VeryWeak);
        assert_eq!(analyze_password("password123"), PasswordStrength::VeryWeak);
        assert_eq!(analyze_password("MyP@ssw0rd!"), PasswordStrength::Strong);
        assert_eq!(analyze_password("Tr0ub4dor&3#Horse$Battery"), PasswordStrength::VeryStrong);
    }
    
    #[test]
    fn test_password_generator() {
        let opts = PasswordGeneratorOptions::default();
        let password = generate_password(&opts);
        
        assert_eq!(password.len(), 20);
        assert!(password.chars().any(|c| c.is_lowercase()));
        assert!(password.chars().any(|c| c.is_uppercase()));
        assert!(password.chars().any(|c| c.is_numeric()));
        assert!(password.chars().any(|c| !c.is_alphanumeric()));
    }
    
    #[test]
    fn test_credential_url_matching() {
        let cred = Credential::new("Test", "user", "pass", "https://example.com");
        
        assert!(cred.matches_url("https://example.com/login"));
        assert!(cred.matches_url("https://www.example.com"));
        assert!(cred.matches_url("https://sub.example.com"));
        assert!(!cred.matches_url("https://other.com"));
    }
}
