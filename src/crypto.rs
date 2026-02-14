//! Cryptographic User Identity and Key Management
//!
//! SECURITY MODEL:
//! ==============================================================================
//! 1. Each user gets a unique Ed25519 key pair during onboarding
//! 2. Private key encrypted with Argon2-derived key from PIN (if set)
//! 3. Symmetric encryption uses ChaCha20-Poly1305 (AEAD)
//! 4. Key derivation uses HKDF-SHA256 for sub-keys
#![allow(dead_code)]

//!
//! WHY THIS MATTERS:
//! - User data encrypted at rest
//! - Sync messages signed with user identity
//! - PIN brute-force resistant (Argon2 is memory-hard)
//! - Keys zeroized from memory when not needed
//!
//! WHAT'S GENERATED ON FIRST RUN:
//! - 32-byte master secret (random)
//! - Ed25519 key pair (identity)
//! - Device ID (random, for sync)
//! - Recovery key (for PIN reset)

use ring::rand::{SecureRandom, SystemRandom};
use ring::signature::{Ed25519KeyPair, KeyPair};
use ring::digest::{digest, SHA256};
#[allow(unused_imports)]
use ring::hmac;
use ring::hkdf;
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{SaltString, PasswordHash};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use zeroize::{Zeroize, ZeroizeOnDrop};
use serde::{Deserialize, Serialize};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use std::time::{SystemTime, UNIX_EPOCH};

/// Size constants
const MASTER_SECRET_LEN: usize = 32;
const DEVICE_ID_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

/// Secure random generator
fn rng() -> SystemRandom {
    SystemRandom::new()
}

/// Generate cryptographically secure random bytes
pub fn random_bytes(len: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; len];
    rng().fill(&mut bytes).expect("RNG failed");
    bytes
}

/// Master secret - the root of all user keys
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct MasterSecret([u8; MASTER_SECRET_LEN]);

impl MasterSecret {
    /// Generate new master secret
    pub fn generate() -> Self {
        let mut secret = [0u8; MASTER_SECRET_LEN];
        rng().fill(&mut secret).expect("RNG failed");
        Self(secret)
    }

    /// Derive from existing bytes (for recovery)
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != MASTER_SECRET_LEN {
            return None;
        }
        let mut secret = [0u8; MASTER_SECRET_LEN];
        secret.copy_from_slice(bytes);
        Some(Self(secret))
    }

    /// Get the raw bytes (use carefully!)
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Derive a sub-key for a specific purpose using HKDF
    pub fn derive_key(&self, purpose: &str) -> [u8; KEY_LEN] {
        let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, b"sassy-browser-v1");
        let prk = salt.extract(&self.0);
        let info = [purpose.as_bytes()];
        let okm = prk.expand(&info, KeyLen).expect("HKDF expand failed");
        
        let mut key = [0u8; KEY_LEN];
        okm.fill(&mut key).expect("HKDF fill failed");
        key
    }
}

/// HKDF output length wrapper
struct KeyLen;
impl hkdf::KeyType for KeyLen {
    fn len(&self) -> usize {
        KEY_LEN
    }
}

/// User identity key pair (Ed25519)
pub struct UserIdentity {
    key_pair: Ed25519KeyPair,
    public_key: Vec<u8>,
}

impl UserIdentity {
    /// Generate new identity from master secret
    pub fn generate(master: &MasterSecret) -> Result<Self, String> {
        // Derive seed for Ed25519 from master secret
        let seed = master.derive_key("identity-ed25519");
        
        let key_pair = Ed25519KeyPair::from_seed_unchecked(&seed)
            .map_err(|e| format!("Failed to create key pair: {:?}", e))?;
        
        let public_key = key_pair.public_key().as_ref().to_vec();
        
        Ok(Self { key_pair, public_key })
    }

    /// Get public key bytes
    pub fn public_key(&self) -> &[u8] {
        &self.public_key
    }

    /// Get public key as base64
    pub fn public_key_b64(&self) -> String {
        BASE64.encode(&self.public_key)
    }

    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.key_pair.sign(message).as_ref().to_vec()
    }

    /// Verify a signature (static method - doesn't need private key)
    pub fn verify(public_key: &[u8], message: &[u8], signature: &[u8]) -> bool {
        use ring::signature::{UnparsedPublicKey, ED25519};
        let public_key = UnparsedPublicKey::new(&ED25519, public_key);
        public_key.verify(message, signature).is_ok()
    }
}

/// Encryption key for user data
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct EncryptionKey([u8; KEY_LEN]);

impl EncryptionKey {
    /// Derive from master secret
    pub fn from_master(master: &MasterSecret, purpose: &str) -> Self {
        Self(master.derive_key(purpose))
    }

    /// Encrypt data with ChaCha20-Poly1305
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let cipher = ChaCha20Poly1305::new_from_slice(&self.0)
            .map_err(|e| format!("Cipher init failed: {:?}", e))?;
        
        // Generate random nonce
        let nonce_bytes = random_bytes(NONCE_LEN);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = cipher.encrypt(nonce, plaintext)
            .map_err(|e| format!("Encryption failed: {:?}", e))?;
        
        // Prepend nonce to ciphertext
        let mut result = nonce_bytes;
        result.extend(ciphertext);
        Ok(result)
    }

    /// Decrypt data
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        if data.len() < NONCE_LEN {
            return Err("Data too short".into());
        }

        let cipher = ChaCha20Poly1305::new_from_slice(&self.0)
            .map_err(|e| format!("Cipher init failed: {:?}", e))?;
        
        let nonce = Nonce::from_slice(&data[..NONCE_LEN]);
        let ciphertext = &data[NONCE_LEN..];
        
        cipher.decrypt(nonce, ciphertext)
            .map_err(|e| format!("Decryption failed: {:?}", e))
    }
}

/// PIN/password hasher using Argon2id
pub struct PinHasher;

impl PinHasher {
    /// Hash a PIN/password (returns encoded hash string)
    pub fn hash(pin: &str) -> Result<String, String> {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let argon2 = Argon2::default();
        
        argon2.hash_password(pin.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| format!("Hash failed: {:?}", e))
    }

    /// Verify a PIN against stored hash
    pub fn verify(pin: &str, hash: &str) -> bool {
        let parsed = match PasswordHash::new(hash) {
            Ok(h) => h,
            Err(_) => return false,
        };
        
        Argon2::default()
            .verify_password(pin.as_bytes(), &parsed)
            .is_ok()
    }

    /// Derive encryption key from PIN (for encrypting master secret)
    pub fn derive_key(pin: &str, salt: &[u8]) -> Result<[u8; KEY_LEN], String> {
        let argon2 = Argon2::default();
        let mut key = [0u8; KEY_LEN];
        
        argon2.hash_password_into(pin.as_bytes(), salt, &mut key)
            .map_err(|e| format!("Key derivation failed: {:?}", e))?;
        
        Ok(key)
    }
}

/// Device identifier (random, for sync)
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DeviceId(String);

impl DeviceId {
    pub fn generate() -> Self {
        let bytes = random_bytes(DEVICE_ID_LEN);
        Self(format!("device_{}", hex::encode(&bytes[..8])))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// Hex encoding helper (avoid another dependency)
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

/// Recovery key (for PIN reset)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecoveryKey {
    /// 24 words or base64 encoded key
    pub encoded: String,
    /// Hash of the recovery key for verification
    pub verification_hash: String,
}

impl RecoveryKey {
    /// Generate new recovery key from master secret
    pub fn generate(master: &MasterSecret) -> Self {
        // Recovery key is derived deterministically
        let recovery_bytes = master.derive_key("recovery-key");
        let encoded = BASE64.encode(recovery_bytes);
        
        // Create verification hash
        let hash = digest(&SHA256, &recovery_bytes);
        let verification_hash = BASE64.encode(hash.as_ref());
        
        Self { encoded, verification_hash }
    }

    /// Verify a recovery key
    pub fn verify(&self, provided: &str) -> bool {
        if let Ok(decoded) = BASE64.decode(provided.trim()) {
            let hash = digest(&SHA256, &decoded);
            BASE64.encode(hash.as_ref()) == self.verification_hash
        } else {
            false
        }
    }
}

/// Stored user cryptographic material
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserCrypto {
    /// Device ID (not secret)
    pub device_id: DeviceId,
    
    /// Public key (not secret)
    pub public_key: String,
    
    /// Encrypted master secret (encrypted with PIN-derived key if PIN set)
    pub encrypted_master: String,
    
    /// Salt for PIN key derivation
    pub pin_salt: String,
    
    /// PIN hash (for verification)
    pub pin_hash: Option<String>,
    
    /// Recovery key info
    pub recovery: RecoveryKey,
    
    /// Created timestamp
    pub created_at: u64,
}

impl UserCrypto {
    /// Create new user crypto during onboarding
    pub fn create(pin: Option<&str>) -> Result<(Self, MasterSecret), String> {
        let master = MasterSecret::generate();
        let identity = UserIdentity::generate(&master)?;
        let device_id = DeviceId::generate();
        let recovery = RecoveryKey::generate(&master);
        
        // Generate salt for PIN key derivation
        let pin_salt = random_bytes(16);
        let pin_salt_b64 = BASE64.encode(&pin_salt);
        
        // Encrypt master secret
        let (encrypted_master, pin_hash) = if let Some(pin) = pin {
            // PIN provided - encrypt master with PIN-derived key
            let pin_key = PinHasher::derive_key(pin, &pin_salt)?;
            let cipher = ChaCha20Poly1305::new_from_slice(&pin_key)
                .map_err(|e| format!("Cipher init failed: {:?}", e))?;
            
            let nonce_bytes = random_bytes(NONCE_LEN);
            let nonce = Nonce::from_slice(&nonce_bytes);
            
            let ciphertext = cipher.encrypt(nonce, master.as_bytes())
                .map_err(|e| format!("Encryption failed: {:?}", e))?;
            
            let mut encrypted = nonce_bytes;
            encrypted.extend(ciphertext);
            
            (
                BASE64.encode(&encrypted),
                Some(PinHasher::hash(pin)?),
            )
        } else {
            // No PIN - store master directly (base64 encoded)
            // Still protected by OS filesystem permissions
            (BASE64.encode(master.as_bytes()), None)
        };
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        
        let user_crypto = Self {
            device_id,
            public_key: identity.public_key_b64(),
            encrypted_master,
            pin_salt: pin_salt_b64,
            pin_hash,
            recovery,
            created_at: now,
        };
        
        Ok((user_crypto, master))
    }

    /// Unlock master secret with PIN
    pub fn unlock(&self, pin: Option<&str>) -> Result<MasterSecret, String> {
        if self.pin_hash.is_some() {
            // PIN required
            let pin = pin.ok_or("PIN required")?;
            
            // Verify PIN hash first (fast fail)
            if !PinHasher::verify(pin, self.pin_hash.as_ref().unwrap()) {
                return Err("Incorrect PIN".into());
            }
            
            // Derive key and decrypt
            let pin_salt = BASE64.decode(&self.pin_salt)
                .map_err(|e| format!("Salt decode failed: {:?}", e))?;
            let pin_key = PinHasher::derive_key(pin, &pin_salt)?;
            
            let encrypted = BASE64.decode(&self.encrypted_master)
                .map_err(|e| format!("Encrypted master decode failed: {:?}", e))?;
            
            if encrypted.len() < NONCE_LEN {
                return Err("Invalid encrypted data".into());
            }
            
            let cipher = ChaCha20Poly1305::new_from_slice(&pin_key)
                .map_err(|e| format!("Cipher init failed: {:?}", e))?;
            
            let nonce = Nonce::from_slice(&encrypted[..NONCE_LEN]);
            let ciphertext = &encrypted[NONCE_LEN..];
            
            let master_bytes = cipher.decrypt(nonce, ciphertext)
                .map_err(|_| "Decryption failed - wrong PIN")?;
            
            MasterSecret::from_bytes(&master_bytes)
                .ok_or_else(|| "Invalid master secret length".into())
        } else {
            // No PIN - just decode
            let master_bytes = BASE64.decode(&self.encrypted_master)
                .map_err(|e| format!("Master decode failed: {:?}", e))?;
            
            MasterSecret::from_bytes(&master_bytes)
                .ok_or_else(|| "Invalid master secret length".into())
        }
    }

    /// Check if PIN is required
    pub fn requires_pin(&self) -> bool {
        self.pin_hash.is_some()
    }

    /// Change PIN (requires current unlock)
    pub fn change_pin(&mut self, master: &MasterSecret, new_pin: Option<&str>) -> Result<(), String> {
        // Generate new salt
        let pin_salt = random_bytes(16);
        self.pin_salt = BASE64.encode(&pin_salt);
        
        if let Some(pin) = new_pin {
            // Encrypt with new PIN
            let pin_key = PinHasher::derive_key(pin, &pin_salt)?;
            let cipher = ChaCha20Poly1305::new_from_slice(&pin_key)
                .map_err(|e| format!("Cipher init failed: {:?}", e))?;
            
            let nonce_bytes = random_bytes(NONCE_LEN);
            let nonce = Nonce::from_slice(&nonce_bytes);
            
            let ciphertext = cipher.encrypt(nonce, master.as_bytes())
                .map_err(|e| format!("Encryption failed: {:?}", e))?;
            
            let mut encrypted = nonce_bytes;
            encrypted.extend(ciphertext);
            
            self.encrypted_master = BASE64.encode(&encrypted);
            self.pin_hash = Some(PinHasher::hash(pin)?);
        } else {
            // Remove PIN
            self.encrypted_master = BASE64.encode(master.as_bytes());
            self.pin_hash = None;
        }
        
        Ok(())
    }

    /// Get user identity from master
    pub fn identity(&self, master: &MasterSecret) -> Result<UserIdentity, String> {
        UserIdentity::generate(master)
    }

    /// Get data encryption key
    pub fn data_key(&self, master: &MasterSecret) -> EncryptionKey {
        EncryptionKey::from_master(master, "user-data")
    }

    /// Get sync encryption key
    pub fn sync_key(&self, master: &MasterSecret) -> EncryptionKey {
        EncryptionKey::from_master(master, "sync-data")
    }

    /// Show recovery key (call only during setup!)
    pub fn recovery_key(&self) -> &str {
        &self.recovery.encoded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_crypto_no_pin() {
        let (crypto, master) = UserCrypto::create(None).unwrap();
        
        assert!(!crypto.requires_pin());
        assert!(!crypto.public_key.is_empty());
        
        // Unlock without PIN
        let unlocked = crypto.unlock(None).unwrap();
        assert_eq!(master.as_bytes(), unlocked.as_bytes());
    }

    #[test]
    fn test_user_crypto_with_pin() {
        let (crypto, master) = UserCrypto::create(Some("1234")).unwrap();
        
        assert!(crypto.requires_pin());
        
        // Wrong PIN fails
        assert!(crypto.unlock(Some("0000")).is_err());
        assert!(crypto.unlock(None).is_err());
        
        // Correct PIN works
        let unlocked = crypto.unlock(Some("1234")).unwrap();
        assert_eq!(master.as_bytes(), unlocked.as_bytes());
    }

    #[test]
    fn test_identity_signing() {
        let (crypto, master) = UserCrypto::create(None).unwrap();
        let identity = crypto.identity(&master).unwrap();
        
        let message = b"Hello, World!";
        let signature = identity.sign(message);
        
        // Verify with public key
        assert!(UserIdentity::verify(identity.public_key(), message, &signature));
        
        // Wrong message fails
        assert!(!UserIdentity::verify(identity.public_key(), b"Wrong", &signature));
    }

    #[test]
    fn test_encryption() {
        let (crypto, master) = UserCrypto::create(None).unwrap();
        let key = crypto.data_key(&master);
        
        let plaintext = b"Sensitive user data";
        let encrypted = key.encrypt(plaintext).unwrap();
        
        assert_ne!(&encrypted, plaintext);
        
        let decrypted = key.decrypt(&encrypted).unwrap();
        assert_eq!(&decrypted, plaintext);
    }

    #[test]
    fn test_pin_change() {
        let (mut crypto, master) = UserCrypto::create(Some("old")).unwrap();
        
        // Change to new PIN
        crypto.change_pin(&master, Some("new")).unwrap();
        
        // Old PIN no longer works
        assert!(crypto.unlock(Some("old")).is_err());
        
        // New PIN works
        assert!(crypto.unlock(Some("new")).is_ok());
    }

    #[test]
    fn test_recovery_key() {
        let (crypto, _master) = UserCrypto::create(None).unwrap();
        
        let recovery = crypto.recovery_key();
        assert!(crypto.recovery.verify(recovery));
        assert!(!crypto.recovery.verify("wrong"));
    }
}
