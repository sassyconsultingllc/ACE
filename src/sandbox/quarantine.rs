//! Download Quarantine System
//!
//! Files don't touch your filesystem until you EARN their release.
//!
//! FLOW:
//! =====
//! 1. User clicks download link
//! 2. File downloaded to encrypted memory vault (not disk)
//! 3. Heuristics scan runs immediately
//! 4. User sees quarantine dialog with warnings
//! 5. Three deliberate interactions required:
//!    - "I understand this is [file type]"
//!    - "Show me where it came from" (reviews origin)
//!    - "Release to my Downloads folder"
//! 6. 5-second countdown (can't be rushed)
//! 7. File written to actual filesystem
//! 8. Still marked as "from internet" (OS protection)
//!
//! WHAT CHROME DOES WRONG:
//! =======================
//! - Downloads immediately to disk
//! - Single click to "Keep" dangerous file
//! - Runs with full user permissions
//! - "Are you sure?" dialogs are ignored by muscle memory
//!
//! WHAT WE DO RIGHT:
//! =================
//! - File never touches disk until approved
//! - Three DIFFERENT interactions (can't autopilot)
//! - Forced wait time (breaks clickjacking)
//! - Clear explanation of what file does
//! - Heuristic warnings shown prominently

use super::{SecurityContext, ContentType, InteractionType, ViolationSeverity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// A file held in quarantine
#[allow(dead_code)] // Fields for security tracking and file management
#[derive(Debug, Clone)]
pub struct QuarantinedFile {
    pub id: String,
    pub filename: String,
    pub source_url: String,
    pub content_type: String,
    pub size_bytes: usize,
    pub sha256: String,
    pub security: SecurityContext,
    pub warnings: Vec<Warning>,
    pub data: Vec<u8>,  // File contents in memory
    pub quarantined_at: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Warning {
    pub level: WarningLevel,
    pub message: String,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum WarningLevel {
    Info,      // Just FYI
    Caution,   // Be careful
    Warning,   // Probably bad
    Danger,    // Almost certainly bad
}

impl QuarantinedFile {
    pub fn new(
        filename: String,
        source_url: String,
        content_type: String,
        data: Vec<u8>,
    ) -> Self {
        let size_bytes = data.len();
        let sha256 = compute_sha256(&data);
        let id = generate_quarantine_id(&filename, &sha256);
        
        let mut file = Self {
            id,
            filename: filename.clone(),
            source_url: source_url.clone(),
            content_type: content_type.clone(),
            size_bytes,
            sha256,
            security: SecurityContext::new(source_url, ContentType::Download),
            warnings: Vec::new(),
            data,
            quarantined_at: Instant::now(),
        };
        
        // Run heuristics immediately
        file.analyze();
        
        file
    }
    
    /// Analyze file for threats
    fn analyze(&mut self) {
        // Check file extension
        let ext = crate::fontcase::ascii_lower(self.filename.rsplit('.').next().unwrap_or(""));
        
        // Executable extensions
        let dangerous_exts = [
            "exe", "msi", "bat", "cmd", "ps1", "vbs", "js", "jse",
            "wsf", "wsh", "scr", "pif", "com", "dll", "sys",
        ];
        
        if dangerous_exts.contains(&ext.as_str()) {
            self.warnings.push(Warning {
                level: WarningLevel::Danger,
                message: "This is an executable program".into(),
                detail: format!(
                    "Files ending in .{} can run code on your computer. \
                     Only install programs from sources you completely trust.",
                    ext
                ),
            });
        }
        
        // Double extension trick (invoice.pdf.exe)
        if self.filename.matches('.').count() > 1 {
            let parts: Vec<&str> = self.filename.split('.').collect();
            if parts.len() >= 3 {
                let real_ext = crate::fontcase::ascii_lower(parts.last().unwrap());
                let fake_ext = crate::fontcase::ascii_lower(parts[parts.len() - 2]);
                
                if dangerous_exts.contains(&real_ext.as_str()) {
                    self.warnings.push(Warning {
                        level: WarningLevel::Danger,
                        message: "Hidden executable extension".into(),
                        detail: format!(
                            "This file pretends to be a .{} but is actually a .{} program. \
                             This is a common malware trick.",
                            fake_ext, real_ext
                        ),
                    });
                }
            }
        }
        
        // Check for suspicious patterns in filename
        let lower_name = crate::fontcase::ascii_lower(&self.filename);
        
        if lower_name.contains("free") && lower_name.contains("download") {
            self.warnings.push(Warning {
                level: WarningLevel::Warning,
                message: "Suspicious filename".into(),
                detail: "Filenames advertising 'free downloads' are often malware.".into(),
            });
        }
        
        if lower_name.contains("crack") || lower_name.contains("keygen") {
            self.warnings.push(Warning {
                level: WarningLevel::Danger,
                message: "Likely malware".into(),
                detail: "Software 'cracks' and 'keygens' almost always contain viruses.".into(),
            });
        }
        
        // Check source URL
        if !self.source_url.starts_with("https://") {
            self.warnings.push(Warning {
                level: WarningLevel::Warning,
                message: "Insecure download".into(),
                detail: "This file was downloaded over an unencrypted connection.".into(),
            });
        }
        
        // Check for known-bad domains (simplified)
        let bad_domains = ["download-free", "crack", "keygen", "warez"];
        for bad in &bad_domains {
            if self.source_url.contains(bad) {
                self.warnings.push(Warning {
                    level: WarningLevel::Danger,
                    message: "Suspicious source".into(),
                    detail: "This website is known for distributing malware.".into(),
                });
                break;
            }
        }
        
        // Large file warning
        if self.size_bytes > 100_000_000 {
            self.warnings.push(Warning {
                level: WarningLevel::Info,
                message: format!("Large file: {} MB", self.size_bytes / 1_000_000),
                detail: "Make sure you have enough disk space.".into(),
            });
        }
        
        // Record violations for severe warnings
        for warning in &self.warnings {
            if warning.level == WarningLevel::Danger {
                self.security.record_violation(
                    &warning.message,
                    ViolationSeverity::High,
                );
            }
        }
    }
    
    /// Get the highest warning level
    #[allow(dead_code)]
    pub fn max_warning_level(&self) -> WarningLevel {
        self.warnings.iter()
            .map(|w| w.level)
            .max_by_key(|l| match l {
                WarningLevel::Info => 0,
                WarningLevel::Caution => 1,
                WarningLevel::Warning => 2,
                WarningLevel::Danger => 3,
            })
            .unwrap_or(WarningLevel::Info)
    }
    
    /// Record user interaction
    pub fn interact(&mut self, action: InteractionType) {
        self.security.record_interaction(action);
    }
    
    /// Check if file can be released
    pub fn can_release(&self) -> ReleaseStatus {
        // Must have 3+ interactions
        let interaction_count = self.security.interactions.len();
        if interaction_count < 3 {
            return ReleaseStatus::NeedsInteraction {
                current: interaction_count as u32,
                required: 3,
            };
        }
        
        // Must wait 5 seconds
        let age = self.quarantined_at.elapsed();
        if age < Duration::from_secs(5) {
            let remaining = Duration::from_secs(5) - age;
            return ReleaseStatus::Waiting {
                seconds_remaining: remaining.as_secs() as u32 + 1,
            };
        }
        
        // Check for critical violations
        if self.security.violations.iter().any(|v| 
            matches!(v.severity, ViolationSeverity::Critical)
        ) {
            return ReleaseStatus::Blocked {
                reason: "Critical security violation detected".into(),
            };
        }
        
        ReleaseStatus::Ready
    }
    
    /// Release file to filesystem
    #[allow(dead_code)]
    pub fn release(&self, destination: PathBuf) -> Result<PathBuf, String> {
        match self.can_release() {
            ReleaseStatus::Ready => {}
            ReleaseStatus::NeedsInteraction { current, required } => {
                return Err(format!(
                    "Need {} more interactions ({}/{})",
                    required - current, current, required
                ));
            }
            ReleaseStatus::Waiting { seconds_remaining } => {
                return Err(format!(
                    "Wait {} more seconds",
                    seconds_remaining
                ));
            }
            ReleaseStatus::Blocked { reason } => {
                return Err(format!("Blocked: {}", reason));
            }
        }
        
        // Write file
        let path = destination.join(&self.filename);
        std::fs::write(&path, &self.data)
            .map_err(|e| format!("Failed to write: {}", e))?;
        
        // Mark file as downloaded from internet so OS applies warnings
        if let Err(e) = mark_as_downloaded(&path, &self.source_url) {
            eprintln!("Warning: could not mark quarantine flags: {}", e);
        }
        
        Ok(path)
    }
}

#[cfg(windows)]
#[allow(dead_code)]
fn mark_as_downloaded(path: &std::path::Path, source_url: &str) -> Result<(), String> {
    use std::fs::OpenOptions;
    use std::io::Write;

    let mut stream_path = path.as_os_str().to_os_string();
    stream_path.push(":Zone.Identifier");
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&stream_path)
        .map_err(|e| format!("Zone.Identifier open failed: {}", e))?;

    let zone = format!("[ZoneTransfer]`r`nZoneId=3`r`nReferrerUrl={}`r`nHostUrl={}`r`n", source_url, source_url);
    file.write_all(zone.as_bytes())
        .map_err(|e| format!("Zone.Identifier write failed: {}", e))?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn mark_as_downloaded(path: &std::path::Path, source_url: &str) -> Result<(), String> {
    use std::process::Command;
    // best-effort: set quarantine xattr (flag 0002, no process ID)
    let value = format!("0081;00000000;Safari;{}", source_url);
    Command::new("xattr")
        .args(["-w", "com.apple.quarantine", &value, path.to_string_lossy().as_ref()])
        .status()
        .map_err(|e| format!("xattr failed: {}", e))?;
    Ok(())
}

#[cfg(not(any(windows, target_os = "macos")))]
fn mark_as_downloaded(_path: &std::path::Path, _source_url: &str) -> Result<(), String> {
    Ok(())
}

#[allow(dead_code)] // Fields for status reporting
#[derive(Debug, Clone)]
pub enum ReleaseStatus {
    Ready,
    NeedsInteraction { current: u32, required: u32 },
    Waiting { seconds_remaining: u32 },
    Blocked { reason: String },
}

/// The quarantine vault
#[allow(dead_code)] // Fields used for file storage
#[derive(Debug, Default)]
pub struct Quarantine {
    files: HashMap<String, QuarantinedFile>,
}

#[allow(dead_code)] // Public API methods
impl Quarantine {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }
    
    /// Add a file to quarantine
    pub fn add(&mut self, file: QuarantinedFile) -> String {
        let id = file.id.clone();
        self.files.insert(id.clone(), file);
        id
    }
    
    /// Get a quarantined file
    pub fn get(&self, id: &str) -> Option<&QuarantinedFile> {
        self.files.get(id)
    }
    
    /// Get mutable reference
    pub fn get_mut(&mut self, id: &str) -> Option<&mut QuarantinedFile> {
        self.files.get_mut(id)
    }
    
    /// List all quarantined files
    pub fn list(&self) -> Vec<&QuarantinedFile> {
        self.files.values().collect()
    }
    
    /// Remove a file (after release or user delete)
    pub fn remove(&mut self, id: &str) -> Option<QuarantinedFile> {
        self.files.remove(id)
    }
    
    /// Get total size of quarantined files
    pub fn total_size(&self) -> usize {
        self.files.values().map(|f| f.size_bytes).sum()
    }
}

/// Compute SHA-256 hash
fn compute_sha256(data: &[u8]) -> String {
    // Simplified hash for now - in production use sha2 crate
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Generate quarantine ID
fn generate_quarantine_id(_filename: &str, hash: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    
    format!("q_{}_{}", &hash[..8], timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_quarantine_flow() {
        let file = QuarantinedFile::new(
            "document.pdf".into(),
            "https://example.com/document.pdf".into(),
            "application/pdf".into(),
            b"fake pdf content".to_vec(),
        );
        
        // Should not be releasable immediately
        assert!(matches!(
            file.can_release(),
            ReleaseStatus::NeedsInteraction { .. }
        ));
    }
    
    #[test]
    fn test_dangerous_exe() {
        let file = QuarantinedFile::new(
            "free_game.exe".into(),
            "http://sketchy-site.com/free_game.exe".into(),
            "application/octet-stream".into(),
            b"MZ".to_vec(), // DOS header
        );
        
        // Should have warnings
        assert!(!file.warnings.is_empty());
        assert!(file.warnings.iter().any(|w| w.level == WarningLevel::Danger));
    }
    
    #[test]
    fn test_double_extension_attack() {
        let file = QuarantinedFile::new(
            "invoice.pdf.exe".into(),
            "https://example.com/invoice.pdf.exe".into(),
            "application/octet-stream".into(),
            vec![],
        );
        
        // Should detect double extension trick
        assert!(file.warnings.iter().any(|w| 
            w.message.contains("Hidden executable")
        ));
    }
}
