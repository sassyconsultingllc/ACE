//! User-based sync sessions
//! 
//! Connection model:
//! 1. Phone connects via Tailscale (encrypted mesh)
//! 2. App shows "Who's browsing?" with user avatars
//! 3. Pick your profile, see your tabs
//! 4. Logout returns to user selection
//!
//! CRYPTOGRAPHY:
//! Each user has cryptographic identity generated at creation:
//! - Ed25519 key pair (for signing sync messages)
//! - Master secret (for data encryption)
//! - PIN protection (Argon2id, memory-hard)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::crypto::{UserCrypto, MasterSecret, UserIdentity, EncryptionKey};

/// A browser user profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub name: String,
    pub avatar: UserAvatar,
    pub color: String,
    pub created_at: u64,
    pub last_active: u64,
    pub is_admin: bool,
    
    /// Cryptographic material (encrypted master, public key, etc.)
    pub crypto: UserCrypto,
}

/// Avatar: initials (default), custom text, or image URL
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UserAvatar {
    #[serde(rename = "initials")]
    Initials { text: String },
    #[serde(rename = "image")]
    Image { url: String },
}

impl Default for UserAvatar {
    fn default() -> Self {
        UserAvatar::Initials { text: "?".into() }
    }
}

impl UserProfile {
    /// Create new user with cryptographic identity
    pub fn new(id: String, name: String, pin: Option<&str>) -> Result<(Self, MasterSecret, String), String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        
        let color = name_to_color(&name);
        let initials = get_initials(&name);
        
        // Generate cryptographic identity
        let (crypto, master) = UserCrypto::create(pin)?;
        let recovery_key = crypto.recovery_key().to_string();
        
        let profile = Self {
            id,
            name,
            avatar: UserAvatar::Initials { text: initials },
            color,
            created_at: now,
            last_active: now,
            is_admin: false,
            crypto,
        };
        
        Ok((profile, master, recovery_key))
    }
    
    #[allow(dead_code)]
    pub fn with_image(mut self, url: &str) -> Self {
        self.avatar = UserAvatar::Image { url: url.into() };
        self
    }
    
    pub fn into_admin(mut self) -> Self {
        self.is_admin = true;
        self
    }
    
    /// Unlock user's master secret with PIN
    pub fn unlock(&self, pin: Option<&str>) -> Result<MasterSecret, String> {
        self.crypto.unlock(pin)
    }
    
    /// Check if user requires PIN
    pub fn requires_pin(&self) -> bool {
        self.crypto.requires_pin()
    }
    
    /// Get user's identity for signing
    #[allow(dead_code)]
    pub fn identity(&self, master: &MasterSecret) -> Result<UserIdentity, String> {
        self.crypto.identity(master)
    }
    
    /// Get encryption key for user data
    #[allow(dead_code)]
    pub fn data_key(&self, master: &MasterSecret) -> EncryptionKey {
        self.crypto.data_key(master)
    }
    
    /// Get encryption key for sync
    #[allow(dead_code)]
    pub fn sync_key(&self, master: &MasterSecret) -> EncryptionKey {
        self.crypto.sync_key(master)
    }
    
    /// Get public key (for verification by others)
    pub fn public_key(&self) -> &str {
        &self.crypto.public_key
    }
    
    /// Get device ID
    pub fn device_id(&self) -> &str {
        self.crypto.device_id.as_str()
    }
    
    /// Change PIN (requires current unlock)
    pub fn change_pin(&mut self, master: &MasterSecret, new_pin: Option<&str>) -> Result<(), String> {
        self.crypto.change_pin(master, new_pin)
    }
    
    pub fn touch(&mut self) {
        self.last_active = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
    }
}

/// Get initials from name (max 2 chars)
fn get_initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

/// Active session
#[derive(Debug, Clone)]
pub struct UserSession {
    pub user_id: String,
    pub session_id: String,
    pub device_name: String,
    pub tailscale_id: Option<String>,
    pub started_at: std::time::Instant,
    pub last_activity: std::time::Instant,
}

impl UserSession {
    pub fn new(user_id: String, device_name: String) -> Self {
        let now = std::time::Instant::now();
        Self {
            user_id,
            session_id: generate_id(),
            device_name,
            tailscale_id: None,
            started_at: now,
            last_activity: now,
        }
    }
    
    pub fn touch(&mut self) {
        self.last_activity = std::time::Instant::now();
    }
}

/// User management
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserManager {
    pub users: HashMap<String, UserProfile>,
    pub allow_guest: bool,
    pub require_pin_for_admin: bool,
}

/// Result of bootstrapping a new user
pub struct BootstrapResult {
    pub user_id: String,
    pub recovery_key: String,
}

#[allow(dead_code)] // Public API methods for user management
impl UserManager {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            allow_guest: false,
            require_pin_for_admin: true,
        }
    }
    
    /// Bootstrap first admin user (returns recovery key!)
    pub fn bootstrap(&mut self, name: String, pin: Option<&str>) -> Result<BootstrapResult, String> {
        let id = generate_id();
        let (user, _master, recovery_key) = UserProfile::new(id.clone(), name, pin)?;
        let user = user.into_admin();
        self.users.insert(id.clone(), user);
        
        Ok(BootstrapResult {
            user_id: id,
            recovery_key,
        })
    }
    
    /// Add a new user (requires admin)
    pub fn add_user(&mut self, name: String, pin: Option<&str>, admin_id: &str) -> Result<(String, String), String> {
        if !self.is_admin(admin_id) {
            return Err("Not authorized".into());
        }
        
        let id = generate_id();
        let (user, _master, recovery_key) = UserProfile::new(id.clone(), name, pin)?;
        self.users.insert(id.clone(), user);
        
        Ok((id, recovery_key))
    }
    
    pub fn remove_user(&mut self, user_id: &str, admin_id: &str) -> Result<(), String> {
        if !self.is_admin(admin_id) {
            return Err("Not authorized".into());
        }
        
        if user_id == admin_id {
            let admin_count = self.users.values().filter(|u| u.is_admin).count();
            if admin_count <= 1 {
                return Err("Cannot remove the only admin".into());
            }
        }
        
        self.users.remove(user_id);
        Ok(())
    }
    
    pub fn get(&self, id: &str) -> Option<&UserProfile> {
        self.users.get(id)
    }
    
    pub fn get_mut(&mut self, id: &str) -> Option<&mut UserProfile> {
        self.users.get_mut(id)
    }
    
    pub fn is_admin(&self, id: &str) -> bool {
        self.users.get(id).map(|u| u.is_admin).unwrap_or(false)
    }
    
    pub fn list_users(&self) -> Vec<&UserProfile> {
        let mut users: Vec<_> = self.users.values().collect();
        users.sort_by(|a, b| a.name.cmp(&b.name));
        users
    }
    
    /// Authenticate user and unlock master secret
    pub fn authenticate(&mut self, user_id: &str, pin: Option<&str>) -> Result<MasterSecret, String> {
        let user = self.users.get_mut(user_id)
            .ok_or("User not found")?;
        
        let master = user.unlock(pin)?;
        user.touch();
        Ok(master)
    }
    
    /// Change user's PIN
    pub fn change_pin(&mut self, user_id: &str, master: &MasterSecret, new_pin: Option<&str>) -> Result<(), String> {
        let user = self.users.get_mut(user_id)
            .ok_or("User not found")?;
        
        user.change_pin(master, new_pin)
    }
    
    pub fn make_admin(&mut self, user_id: &str, admin_id: &str) -> Result<(), String> {
        if !self.is_admin(admin_id) {
            return Err("Not authorized".into());
        }
        
        let user = self.users.get_mut(user_id)
            .ok_or("User not found")?;
        
        user.is_admin = true;
        Ok(())
    }
    
    /// Login by username (finds user by name, creates session)
    pub fn login(&mut self, username: &str, device_name: &str) -> Result<UserSession, String> {
        // Find user by name
        let user = self.users.values_mut()
            .find(|u| u.name.eq_ignore_ascii_case(username))
            .ok_or("User not found")?;
        
        user.touch();
        
        Ok(UserSession {
            user_id: user.id.clone(),
            session_id: generate_id(),
            device_name: device_name.to_string(),
            tailscale_id: None,
            started_at: std::time::Instant::now(),
            last_activity: std::time::Instant::now(),
        })
    }
}

/// What phone sees for login
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLoginInfo {
    pub id: String,
    pub name: String,
    pub avatar: UserAvatar,
    pub color: String,
    pub requires_pin: bool,
    pub is_admin: bool,
    pub public_key: String,
    pub device_id: String,
}

impl From<&UserProfile> for UserLoginInfo {
    fn from(user: &UserProfile) -> Self {
        Self {
            id: user.id.clone(),
            name: user.name.clone(),
            avatar: user.avatar.clone(),
            color: user.color.clone(),
            requires_pin: user.requires_pin(),
            is_admin: user.is_admin,
            public_key: user.public_key().to_string(),
            device_id: user.device_id().to_string(),
        }
    }
}

fn generate_id() -> String {
    use crate::crypto::random_bytes;
    
    let bytes = random_bytes(8);
    format!("user_{}", bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>())
}

fn name_to_color(name: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    crate::fontcase::ascii_lower(name).hash(&mut hasher);
    let hash = hasher.finish();
    
    // Muted, professional palette
    let colors = [
        "#dc2626", "#ea580c", "#d97706", "#ca8a04", 
        "#65a30d", "#16a34a", "#059669", "#0d9488",
        "#0891b2", "#0284c7", "#2563eb", "#4f46e5",
        "#7c3aed", "#9333ea", "#c026d3", "#db2777",
    ];
    
    colors[(hash as usize) % colors.len()].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_initials() {
        assert_eq!(get_initials("Shane Anderson"), "SA");
        assert_eq!(get_initials("Alex"), "A");
        assert_eq!(get_initials("Mary Jane Watson"), "MJ");
    }
    
    #[test]
    fn test_user_flow() {
        let mut mgr = UserManager::new();
        
        let result = mgr.bootstrap("Dad".into(), None).unwrap();
        let admin_id = result.user_id;
        
        // Recovery key should be present
        assert!(!result.recovery_key.is_empty());
        
        mgr.add_user("Mom".into(), None, &admin_id).unwrap();
        mgr.add_user("Alex".into(), None, &admin_id).unwrap();
        
        assert_eq!(mgr.list_users().len(), 3);
    }
    
    #[test]
    fn test_pin_auth() {
        let mut mgr = UserManager::new();
        let result = mgr.bootstrap("Parent".into(), Some("1234")).unwrap();
        let id = result.user_id;
        
        // Wrong PIN fails
        assert!(mgr.authenticate(&id, Some("0000")).is_err());
        assert!(mgr.authenticate(&id, None).is_err());
        
        // Correct PIN returns master secret
        let master = mgr.authenticate(&id, Some("1234")).unwrap();
        assert!(!master.as_bytes().is_empty());
    }
    
    #[test]
    fn test_crypto_identity() {
        let mut mgr = UserManager::new();
        let result = mgr.bootstrap("Tester".into(), None).unwrap();
        let id = result.user_id;
        
        let master = mgr.authenticate(&id, None).unwrap();
        let user = mgr.get(&id).unwrap();
        
        let identity = user.identity(&master).unwrap();
        
        // Sign and verify
        let msg = b"Test message";
        let sig = identity.sign(msg);
        
        use crate::crypto::UserIdentity;
        assert!(UserIdentity::verify(identity.public_key(), msg, &sig));
    }
}
