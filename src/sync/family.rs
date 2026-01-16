//! Family device management
//! 
//! Philosophy: Approve devices, not monitor sessions.
//! - Parent approves which devices can connect (one-time)
//! - No content surveillance - that breaks trust
//! - Age-appropriate boundaries that loosen as kid grows
//! - Goal: Security (who connects), not surveillance (what they do)
//!
//! This is "give them a house key" not "install cameras in their room"

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Device trust level
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrustLevel {
    /// Full access, can approve other devices (parent/admin)
    Admin,
    /// Full access, cannot approve devices (trusted family member, teen 13+)
    Trusted,
    /// Limited access, parent notified of new device (child under 13)
    Supervised,
    /// Pending approval
    Pending,
    /// Explicitly blocked
    Blocked,
}

#[allow(dead_code)] // Public API methods
impl TrustLevel {
    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            TrustLevel::Admin => "Admin - full access, can approve devices",
            TrustLevel::Trusted => "Trusted - full access",
            TrustLevel::Supervised => "Supervised - limited access, parent notified",
            TrustLevel::Pending => "Pending - awaiting approval",
            TrustLevel::Blocked => "Blocked - access denied",
        }
    }
}

/// A registered device in the family
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamilyDevice {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub trust_level: TrustLevel,
    pub approved_by: Option<String>,
    pub approved_at: Option<u64>,
    pub created_at: u64,
    pub last_seen: u64,
    pub tailscale_id: Option<String>,
}

/// Family configuration - what gets stored
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FamilyConfig {
    /// All registered devices
    pub devices: HashMap<String, FamilyDevice>,
    
    /// Require admin approval for new devices
    pub require_approval: bool,
    
    /// Auto-approve devices from same Tailscale network
    pub trust_tailscale: bool,
    
    /// Who gets notified of new device requests (device IDs)
    pub notify_admins: Vec<String>,
}

#[allow(dead_code)] // Public API methods for family device management
impl FamilyConfig {
    /// Check if a device is allowed to connect
    pub fn is_allowed(&self, device_id: &str) -> bool {
        if let Some(device) = self.devices.get(device_id) {
            matches!(device.trust_level, 
                TrustLevel::Admin | TrustLevel::Trusted | TrustLevel::Supervised)
        } else {
            // Unknown device - depends on approval setting
            !self.require_approval
        }
    }
    
    /// Check if device can approve other devices
    pub fn can_approve(&self, device_id: &str) -> bool {
        self.devices.get(device_id)
            .map(|d| d.trust_level == TrustLevel::Admin)
            .unwrap_or(false)
    }
    
    /// Register a new device (pending approval)
    pub fn request_device(&mut self, id: String, name: String, owner: String) -> &FamilyDevice {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        
        let device = FamilyDevice {
            id: id.clone(),
            name,
            owner,
            trust_level: if self.require_approval { 
                TrustLevel::Pending 
            } else { 
                TrustLevel::Trusted 
            },
            approved_by: None,
            approved_at: None,
            created_at: now,
            last_seen: now,
            tailscale_id: None,
        };
        
        self.devices.insert(id.clone(), device);
        self.devices.get(&id).unwrap()
    }
    
    /// Approve a pending device
    pub fn approve_device(&mut self, device_id: &str, approver_id: &str, trust: TrustLevel) -> Result<(), String> {
        // Check approver has permission
        if !self.can_approve(approver_id) {
            return Err("Not authorized to approve devices".into());
        }
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        
        if let Some(device) = self.devices.get_mut(device_id) {
            device.trust_level = trust;
            device.approved_by = Some(approver_id.to_string());
            device.approved_at = Some(now);
            Ok(())
        } else {
            Err("Device not found".into())
        }
    }
    
    /// Block a device
    pub fn block_device(&mut self, device_id: &str, blocker_id: &str) -> Result<(), String> {
        if !self.can_approve(blocker_id) {
            return Err("Not authorized".into());
        }
        
        if let Some(device) = self.devices.get_mut(device_id) {
            device.trust_level = TrustLevel::Blocked;
            Ok(())
        } else {
            Err("Device not found".into())
        }
    }
    
    /// Remove a device entirely
    pub fn remove_device(&mut self, device_id: &str, remover_id: &str) -> Result<(), String> {
        if !self.can_approve(remover_id) {
            return Err("Not authorized".into());
        }
        
        self.devices.remove(device_id);
        Ok(())
    }
    
    /// Get pending approval requests
    pub fn pending_devices(&self) -> Vec<&FamilyDevice> {
        self.devices.values()
            .filter(|d| d.trust_level == TrustLevel::Pending)
            .collect()
    }
    
    /// Update last seen timestamp
    pub fn touch_device(&mut self, device_id: &str) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        
        if let Some(device) = self.devices.get_mut(device_id) {
            device.last_seen = now;
        }
    }
    
    /// Upgrade trust level (e.g., kid turns 13)
    pub fn upgrade_trust(&mut self, device_id: &str, upgrader_id: &str, new_level: TrustLevel) -> Result<(), String> {
        if !self.can_approve(upgrader_id) {
            return Err("Not authorized".into());
        }
        
        // Can't upgrade to Admin without being Admin
        if new_level == TrustLevel::Admin && !self.can_approve(upgrader_id) {
            return Err("Cannot grant Admin".into());
        }
        
        if let Some(device) = self.devices.get_mut(device_id) {
            device.trust_level = new_level;
            Ok(())
        } else {
            Err("Device not found".into())
        }
    }
    
    /// First device becomes admin automatically
    pub fn bootstrap_admin(&mut self, id: String, name: String, owner: String) -> &FamilyDevice {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        
        let device = FamilyDevice {
            id: id.clone(),
            name,
            owner,
            trust_level: TrustLevel::Admin,
            approved_by: Some("self".into()),
            approved_at: Some(now),
            created_at: now,
            last_seen: now,
            tailscale_id: None,
        };
        
        self.devices.insert(id.clone(), device);
        self.notify_admins.push(id.clone());
        self.devices.get(&id).unwrap()
    }
}

/// What info is shared about connected devices (privacy-respecting)
#[allow(dead_code)] // Fields for device presence tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevicePresence {
    pub device_id: String,
    pub device_name: String,
    pub owner: String,
    pub online: bool,
    pub last_seen: u64,
    // Intentionally NO: current_url, tabs, history, etc.
}

/// Notification types (not surveillance)
#[allow(dead_code)] // Variants for notification system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FamilyNotification {
    /// New device wants to join
    DeviceRequest {
        device: FamilyDevice,
    },
    /// Device was approved
    DeviceApproved {
        device_id: String,
        device_name: String,
        approved_by: String,
    },
    /// Device was blocked
    DeviceBlocked {
        device_id: String,
        device_name: String,
    },
    /// Device came online (optional, can disable)
    DeviceOnline {
        device_id: String,
        device_name: String,
    },
    /// Device went offline (optional, can disable)
    DeviceOffline {
        device_id: String,
        device_name: String,
    },
}

/// What we explicitly DON'T do:
/// 
/// - Track browsing history
/// - Monitor current tabs/URLs
/// - Log session activity
/// - Record screen time details
/// - Content filtering (that's a separate concern)
/// - Keylogging or form capture
/// - Location tracking
/// 
/// Why: Surveillance doesn't build trust. It teaches kids to hide.
/// Better: Have conversations, build mutual respect, age-appropriate freedom.
/// 
/// The only question we answer: "Is this device allowed to connect?"

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_family_flow() {
        let mut config = FamilyConfig::default();
        config.require_approval = true;
        
        // Parent sets up first (becomes admin)
        config.bootstrap_admin("parent-phone".into(), "Dad's Phone".into(), "Dad".into());
        
        // Kid requests to join
        config.request_device("kid-phone".into(), "Alex's Phone".into(), "Alex".into());
        
        // Verify kid is pending
        assert_eq!(config.pending_devices().len(), 1);
        assert!(!config.is_allowed("kid-phone"));
        
        // Parent approves
        config.approve_device("kid-phone", "parent-phone", TrustLevel::Trusted).unwrap();
        
        // Now kid can connect
        assert!(config.is_allowed("kid-phone"));
        assert!(config.pending_devices().is_empty());
        
        // Kid can't approve others
        assert!(!config.can_approve("kid-phone"));
        
        // Parent can
        assert!(config.can_approve("parent-phone"));
    }
    
    #[test]
    fn test_trust_upgrade() {
        let mut config = FamilyConfig::default();
        config.bootstrap_admin("parent".into(), "Parent".into(), "Parent".into());
        config.request_device("kid".into(), "Kid".into(), "Kid".into());
        
        // Start supervised (under 13)
        config.approve_device("kid", "parent", TrustLevel::Supervised).unwrap();
        
        // Kid turns 13, upgrade to trusted
        config.upgrade_trust("kid", "parent", TrustLevel::Trusted).unwrap();
        
        let kid = config.devices.get("kid").unwrap();
        assert_eq!(kid.trust_level, TrustLevel::Trusted);
    }
}
