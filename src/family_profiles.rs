// ============================================================================
// SASSY BROWSER - FAMILY PROFILES
// ============================================================================
// Adult, Teen, and Kid profiles with different permission levels.
// Kids need parent approval for downloads. Time limits. Site allowlists.
// Weekly reports. FINALLY, PARENTAL CONTROLS THAT WORK.
// ============================================================================

#![allow(dead_code, unused_variables, unused_imports)]

use std::collections::{HashMap, HashSet};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ============================================================================
// PROFILE TYPES
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProfileType {
    Admin,   // Full control, manages other profiles
    Adult,   // No restrictions
    Teen,    // Some restrictions, can request access
    Kid,     // Heavy restrictions, needs approval for most things
}

impl ProfileType {
    pub fn icon(&self) -> &'static str {
        match self {
            ProfileType::Admin => "👑",
            ProfileType::Adult => "👤",
            ProfileType::Teen => "🧑",
            ProfileType::Kid => "👶",
        }
    }
    
    pub fn color(&self) -> [u8; 3] {
        match self {
            ProfileType::Admin => [255, 215, 0],   // Gold
            ProfileType::Adult => [100, 150, 255], // Blue
            ProfileType::Teen => [150, 255, 150],  // Green
            ProfileType::Kid => [255, 200, 100],   // Orange
        }
    }
    
    pub fn default_restrictions(&self) -> Restrictions {
        match self {
            ProfileType::Admin | ProfileType::Adult => Restrictions::none(),
            ProfileType::Teen => Restrictions::teen_default(),
            ProfileType::Kid => Restrictions::kid_default(),
        }
    }
}

// ============================================================================
// USER PROFILE
// ============================================================================

#[derive(Debug, Clone)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub profile_type: ProfileType,
    pub avatar: Option<String>,       // Path to avatar image or emoji
    pub pin: Option<String>,          // Hashed PIN for profile switching
    pub created_at: u64,
    pub last_active: u64,
    pub restrictions: Restrictions,
    pub parent_profile_id: Option<String>, // For Teen/Kid profiles
    pub usage_stats: UsageStats,
}

impl Profile {
    pub fn new(name: &str, profile_type: ProfileType) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            id: generate_id(),
            name: name.to_string(),
            profile_type: profile_type.clone(),
            avatar: None,
            pin: None,
            created_at: now,
            last_active: now,
            restrictions: profile_type.default_restrictions(),
            parent_profile_id: None,
            usage_stats: UsageStats::default(),
        }
    }
    
    pub fn is_restricted(&self) -> bool {
        !matches!(self.profile_type, ProfileType::Admin | ProfileType::Adult)
    }
    
    pub fn requires_approval(&self, action: &Action) -> bool {
        match &self.profile_type {
            ProfileType::Admin | ProfileType::Adult => false,
            ProfileType::Teen => {
                matches!(action, 
                    Action::InstallExtension | 
                    Action::ChangeSettings |
                    Action::AccessBlockedSite(_))
            }
            ProfileType::Kid => true, // Kids need approval for everything significant
        }
    }
}

// ============================================================================
// RESTRICTIONS
// ============================================================================

#[derive(Debug, Clone)]
pub struct Restrictions {
    // Site filtering
    pub block_nsfw: bool,
    pub block_violence: bool,
    pub block_gambling: bool,
    pub block_social_media: bool,
    pub use_allowlist_only: bool,        // Only allow sites on allowlist
    pub allowlist: HashSet<String>,
    pub blocklist: HashSet<String>,
    
    // Download restrictions
    pub downloads_need_approval: bool,
    pub block_executable_downloads: bool,
    pub max_download_size_mb: Option<u64>,
    
    // Time limits
    pub daily_limit_minutes: Option<u32>,
    pub bedtime_start: Option<(u8, u8)>,  // (hour, minute) 24h format
    pub bedtime_end: Option<(u8, u8)>,
    pub weekday_limit_minutes: Option<u32>,
    pub weekend_limit_minutes: Option<u32>,
    
    // Content restrictions
    pub safe_search_enforced: bool,
    pub block_incognito: bool,
    pub block_vpn_proxy: bool,
    pub block_extension_install: bool,
    pub block_settings_access: bool,
    
    // Activity monitoring
    pub log_all_visits: bool,
    pub send_weekly_report: bool,
    pub notify_blocked_attempts: bool,
}

impl Restrictions {
    pub fn none() -> Self {
        Self {
            block_nsfw: false,
            block_violence: false,
            block_gambling: false,
            block_social_media: false,
            use_allowlist_only: false,
            allowlist: HashSet::new(),
            blocklist: HashSet::new(),
            downloads_need_approval: false,
            block_executable_downloads: false,
            max_download_size_mb: None,
            daily_limit_minutes: None,
            bedtime_start: None,
            bedtime_end: None,
            weekday_limit_minutes: None,
            weekend_limit_minutes: None,
            safe_search_enforced: false,
            block_incognito: false,
            block_vpn_proxy: false,
            block_extension_install: false,
            block_settings_access: false,
            log_all_visits: false,
            send_weekly_report: false,
            notify_blocked_attempts: false,
        }
    }
    
    pub fn teen_default() -> Self {
        let mut r = Self::none();
        r.block_nsfw = true;
        r.block_gambling = true;
        r.safe_search_enforced = true;
        r.block_executable_downloads = true;
        r.log_all_visits = true;
        r.send_weekly_report = true;
        r
    }
    
    pub fn kid_default() -> Self {
        let mut r = Self::teen_default();
        r.block_violence = true;
        r.block_social_media = true;
        r.downloads_need_approval = true;
        r.daily_limit_minutes = Some(120); // 2 hours
        r.bedtime_start = Some((20, 0));   // 8 PM
        r.bedtime_end = Some((7, 0));      // 7 AM
        r.block_incognito = true;
        r.block_extension_install = true;
        r.block_settings_access = true;
        r.notify_blocked_attempts = true;
        
        // Default allowlist for kids
        r.allowlist.insert("youtube.com".to_string());
        r.allowlist.insert("khanacademy.org".to_string());
        r.allowlist.insert("pbskids.org".to_string());
        r.allowlist.insert("nickjr.com".to_string());
        r.allowlist.insert("disney.com".to_string());
        r.allowlist.insert("coolmathgames.com".to_string());
        r.allowlist.insert("nationalgeographic.com".to_string());
        r.allowlist.insert("wikipedia.org".to_string());
        
        r
    }
}

// ============================================================================
// USAGE STATS
// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct UsageStats {
    pub total_time_minutes: u64,
    pub today_time_minutes: u32,
    pub last_reset_date: u64,           // Unix timestamp of last daily reset
    pub sites_visited: HashMap<String, u32>, // domain -> visit count
    pub blocked_attempts: Vec<BlockedAttempt>,
    pub downloads: Vec<DownloadRecord>,
    pub search_queries: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BlockedAttempt {
    pub timestamp: u64,
    pub url: String,
    pub reason: BlockReason,
}

#[derive(Debug, Clone)]
pub enum BlockReason {
    Nsfw,
    Violence,
    Gambling,
    SocialMedia,
    NotOnAllowlist,
    OnBlocklist,
    TimeLimitReached,
    Bedtime,
    ParentBlocked,
}

impl BlockReason {
    pub fn description(&self) -> &'static str {
        match self {
            BlockReason::Nsfw => "Adult content blocked",
            BlockReason::Violence => "Violent content blocked",
            BlockReason::Gambling => "Gambling site blocked",
            BlockReason::SocialMedia => "Social media blocked",
            BlockReason::NotOnAllowlist => "Site not on allowed list",
            BlockReason::OnBlocklist => "Site is blocked",
            BlockReason::TimeLimitReached => "Daily time limit reached",
            BlockReason::Bedtime => "It's bedtime!",
            BlockReason::ParentBlocked => "Blocked by parent",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DownloadRecord {
    pub timestamp: u64,
    pub filename: String,
    pub url: String,
    pub size_bytes: u64,
    pub approved: bool,
    pub approved_by: Option<String>,
}

// ============================================================================
// ACTIONS REQUIRING APPROVAL
// ============================================================================

#[derive(Debug, Clone)]
pub enum Action {
    Download { filename: String, url: String, size: u64 },
    AccessBlockedSite(String),
    ExtendTimeLimit { minutes: u32 },
    InstallExtension,
    ChangeSettings,
    AddToAllowlist(String),
    RemoveFromBlocklist(String),
}

#[derive(Debug, Clone)]
pub struct ApprovalRequest {
    pub id: String,
    pub profile_id: String,
    pub action: Action,
    pub timestamp: u64,
    pub status: ApprovalStatus,
    pub parent_response: Option<String>,
    pub responded_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Denied,
    Expired,
}

// ============================================================================
// WEEKLY REPORT
// ============================================================================

#[derive(Debug, Clone)]
pub struct WeeklyReport {
    pub profile_id: String,
    pub profile_name: String,
    pub week_start: u64,
    pub week_end: u64,
    pub total_time_minutes: u32,
    pub daily_breakdown: [u32; 7],  // Minutes per day, Mon-Sun
    pub top_sites: Vec<(String, u32)>, // (domain, minutes)
    pub blocked_attempts: u32,
    pub downloads: u32,
    pub search_queries_count: u32,
    pub most_active_hour: u8,
}

// ============================================================================
// PROFILE MANAGER
// ============================================================================

pub struct ProfileManager {
    profiles: Vec<Profile>,
    active_profile_id: Option<String>,
    approval_requests: Vec<ApprovalRequest>,
    session_start: std::time::Instant,
    last_activity: std::time::Instant,
}

impl ProfileManager {
        /// Public getter for all approval requests (for UI display)
        pub fn all_approval_requests(&self) -> &[ApprovalRequest] {
            &self.approval_requests
        }
    pub fn new() -> Self {
        Self {
            profiles: Vec::new(),
            active_profile_id: None,
            approval_requests: Vec::new(),
            session_start: std::time::Instant::now(),
            last_activity: std::time::Instant::now(),
        }
    }
    
    // ========================================================================
    // PROFILE MANAGEMENT
    // ========================================================================
    
    pub fn create_admin(&mut self, name: &str, pin: &str) -> String {
        let mut profile = Profile::new(name, ProfileType::Admin);
        profile.pin = Some(hash_pin(pin));
        let id = profile.id.clone();
        self.profiles.push(profile);
        id
    }
    
    pub fn create_profile(&mut self, name: &str, profile_type: ProfileType, parent_id: Option<&str>) -> Result<String, String> {
        // Verify parent exists for restricted profiles
        if matches!(profile_type, ProfileType::Teen | ProfileType::Kid) {
            if let Some(pid) = parent_id {
                let parent = self.profiles.iter().find(|p| p.id == pid);
                if parent.is_none() {
                    return Err("Parent profile not found".to_string());
                }
                if !matches!(parent.unwrap().profile_type, ProfileType::Admin | ProfileType::Adult) {
                    return Err("Parent must be Admin or Adult".to_string());
                }
            } else {
                return Err("Teen/Kid profiles require a parent".to_string());
            }
        }
        
        let mut profile = Profile::new(name, profile_type);
        profile.parent_profile_id = parent_id.map(|s| s.to_string());
        let id = profile.id.clone();
        self.profiles.push(profile);
        Ok(id)
    }
    
    pub fn delete_profile(&mut self, id: &str) -> Result<(), String> {
        // Can't delete the last admin
        let admin_count = self.profiles.iter()
            .filter(|p| p.profile_type == ProfileType::Admin && p.id != id)
            .count();
        
        let profile = self.profiles.iter().find(|p| p.id == id);
        if let Some(p) = profile {
            if p.profile_type == ProfileType::Admin && admin_count == 0 {
                return Err("Cannot delete the last admin profile".to_string());
            }
        }
        
        self.profiles.retain(|p| p.id != id);
        Ok(())
    }
    
    pub fn switch_profile(&mut self, id: &str, pin: Option<&str>) -> Result<(), String> {
        let profile = self.profiles.iter().find(|p| p.id == id)
            .ok_or("Profile not found")?;
        
        // Verify PIN if set
        if let Some(stored_pin) = &profile.pin {
            let provided_pin = pin.ok_or("PIN required")?;
            if !verify_pin(provided_pin, stored_pin) {
                return Err("Incorrect PIN".to_string());
            }
        }
        
        // Check if profile is allowed at this time
        if let Some(reason) = self.check_time_restrictions(&profile.restrictions) {
            return Err(reason.description().to_string());
        }
        
        self.active_profile_id = Some(id.to_string());
        self.session_start = std::time::Instant::now();
        
        // Update last_active
        if let Some(p) = self.profiles.iter_mut().find(|p| p.id == id) {
            p.last_active = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
        }
        
        Ok(())
    }
    
    pub fn active_profile(&self) -> Option<&Profile> {
        self.active_profile_id.as_ref()
            .and_then(|id| self.profiles.iter().find(|p| &p.id == id))
    }
    
    pub fn active_profile_mut(&mut self) -> Option<&mut Profile> {
        let id = self.active_profile_id.clone()?;
        self.profiles.iter_mut().find(|p| p.id == id)
    }
    
    pub fn profiles(&self) -> &[Profile] {
        &self.profiles
    }
    
    pub fn get_profile(&self, id: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.id == id)
    }
    
    pub fn get_profile_mut(&mut self, id: &str) -> Option<&mut Profile> {
        self.profiles.iter_mut().find(|p| p.id == id)
    }
    
    // ========================================================================
    // ACCESS CONTROL
    // ========================================================================
    
    pub fn can_access_url(&mut self, url: &str) -> Result<(), BlockReason> {
        let profile = match self.active_profile() {
            Some(p) => p.clone(),
            None => return Ok(()), // No profile = no restrictions
        };
        
        let restrictions = &profile.restrictions;
        let domain = extract_domain(url).to_lowercase();
        
        // Check time limits first
        if let Some(reason) = self.check_time_restrictions(restrictions) {
            self.record_blocked_attempt(url, reason.clone());
            return Err(reason);
        }
        
        // Check allowlist mode
        if restrictions.use_allowlist_only
            && !restrictions.allowlist.iter().any(|a| domain.contains(&a.to_lowercase())) {
                let reason = BlockReason::NotOnAllowlist;
                self.record_blocked_attempt(url, reason.clone());
                return Err(reason);
            }
        
        // Check blocklist
        if restrictions.blocklist.iter().any(|b| domain.contains(&b.to_lowercase())) {
            let reason = BlockReason::OnBlocklist;
            self.record_blocked_attempt(url, reason.clone());
            return Err(reason);
        }
        
        // Check content categories
        if restrictions.block_nsfw && is_nsfw_domain(&domain) {
            let reason = BlockReason::Nsfw;
            self.record_blocked_attempt(url, reason.clone());
            return Err(reason);
        }
        
        if restrictions.block_gambling && is_gambling_domain(&domain) {
            let reason = BlockReason::Gambling;
            self.record_blocked_attempt(url, reason.clone());
            return Err(reason);
        }
        
        if restrictions.block_social_media && is_social_media_domain(&domain) {
            let reason = BlockReason::SocialMedia;
            self.record_blocked_attempt(url, reason.clone());
            return Err(reason);
        }
        
        Ok(())
    }
    
    pub fn can_download(&self, filename: &str, size: u64) -> Result<(), String> {
        let profile = match self.active_profile() {
            Some(p) => p,
            None => return Ok(()),
        };
        
        let restrictions = &profile.restrictions;
        
        if restrictions.downloads_need_approval {
            return Err("Download requires parent approval".to_string());
        }
        
        if restrictions.block_executable_downloads {
            let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
            let executables = ["exe", "msi", "bat", "cmd", "ps1", "sh", "app", "dmg", "deb", "rpm"];
            if executables.contains(&ext.as_str()) {
                return Err("Executable downloads are blocked".to_string());
            }
        }
        
        if let Some(max_mb) = restrictions.max_download_size_mb {
            if size > max_mb * 1024 * 1024 {
                return Err(format!("File too large (max {} MB)", max_mb));
            }
        }
        
        Ok(())
    }
    
    fn check_time_restrictions(&self, restrictions: &Restrictions) -> Option<BlockReason> {
        let now = chrono_lite::now();
        let (hour, minute) = (now.hour, now.minute);
        let is_weekend = now.weekday >= 5;
        
        // Check bedtime
        if let (Some(start), Some(end)) = (restrictions.bedtime_start, restrictions.bedtime_end) {
            let current_mins = hour as u32 * 60 + minute as u32;
            let start_mins = start.0 as u32 * 60 + start.1 as u32;
            let end_mins = end.0 as u32 * 60 + end.1 as u32;
            
            // Handle overnight bedtime (e.g., 20:00 - 07:00)
            if start_mins > end_mins {
                if current_mins >= start_mins || current_mins < end_mins {
                    return Some(BlockReason::Bedtime);
                }
            } else if current_mins >= start_mins && current_mins < end_mins {
                return Some(BlockReason::Bedtime);
            }
        }
        
        // Check daily limit
        if let Some(profile) = self.active_profile() {
            let limit = if is_weekend {
                restrictions.weekend_limit_minutes.or(restrictions.daily_limit_minutes)
            } else {
                restrictions.weekday_limit_minutes.or(restrictions.daily_limit_minutes)
            };
            
            if let Some(limit_mins) = limit {
                if profile.usage_stats.today_time_minutes >= limit_mins {
                    return Some(BlockReason::TimeLimitReached);
                }
            }
        }
        
        None
    }
    
    fn record_blocked_attempt(&mut self, url: &str, reason: BlockReason) {
        if let Some(profile) = self.active_profile_mut() {
            if profile.restrictions.log_all_visits || profile.restrictions.notify_blocked_attempts {
                profile.usage_stats.blocked_attempts.push(BlockedAttempt {
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    url: url.to_string(),
                    reason,
                });
            }
        }
    }
    
    // ========================================================================
    // USAGE TRACKING
    // ========================================================================
    
    pub fn record_activity(&mut self) {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_activity);
        self.last_activity = now;
        
        // Only count if less than 5 minutes since last activity
        if elapsed < Duration::from_secs(300) {
            if let Some(profile) = self.active_profile_mut() {
                let minutes = (elapsed.as_secs() / 60) as u32;
                profile.usage_stats.today_time_minutes += minutes.max(1);
                profile.usage_stats.total_time_minutes += minutes.max(1) as u64;
            }
        }
    }
    
    pub fn record_site_visit(&mut self, domain: &str) {
        if let Some(profile) = self.active_profile_mut() {
            if profile.restrictions.log_all_visits {
                *profile.usage_stats.sites_visited
                    .entry(domain.to_string())
                    .or_insert(0) += 1;
            }
        }
    }

    pub fn record_download(&mut self, filename: &str, url: &str, size_bytes: u64, approved: bool, approved_by: Option<String>) {
        if let Some(profile) = self.active_profile_mut() {
            profile.usage_stats.downloads.push(DownloadRecord {
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                filename: filename.to_string(),
                url: url.to_string(),
                size_bytes,
                approved,
                approved_by,
            });
        }
    }
    
    pub fn record_search(&mut self, query: &str) {
        if let Some(profile) = self.active_profile_mut() {
            if profile.restrictions.log_all_visits {
                profile.usage_stats.search_queries.push(query.to_string());
                // Keep last 1000
                if profile.usage_stats.search_queries.len() > 1000 {
                    profile.usage_stats.search_queries.remove(0);
                }
            }
        }
    }
    
    pub fn reset_daily_stats(&mut self) {
        let today = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() / 86400;
        
        for profile in &mut self.profiles {
            let last_reset = profile.usage_stats.last_reset_date / 86400;
            if today > last_reset {
                profile.usage_stats.today_time_minutes = 0;
                profile.usage_stats.last_reset_date = today * 86400;
            }
        }
    }
    
    pub fn remaining_time_minutes(&self) -> Option<u32> {
        let profile = self.active_profile()?;
        let restrictions = &profile.restrictions;
        
        let limit = restrictions.daily_limit_minutes?;
        Some(limit.saturating_sub(profile.usage_stats.today_time_minutes))
    }
    
    // ========================================================================
    // APPROVAL REQUESTS
    // ========================================================================
    
    pub fn request_approval(&mut self, action: Action) -> String {
        let request = ApprovalRequest {
            id: generate_id(),
            profile_id: self.active_profile_id.clone().unwrap_or_default(),
            action,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            status: ApprovalStatus::Pending,
            parent_response: None,
            responded_at: None,
        };
        
        let id = request.id.clone();
        self.approval_requests.push(request);
        id
    }
    
    pub fn approve_request(&mut self, request_id: &str, parent_id: &str) -> Result<(), String> {
        let request = self.approval_requests.iter_mut()
            .find(|r| r.id == request_id)
            .ok_or("Request not found")?;
        
        // Verify parent has permission
        let parent = self.profiles.iter().find(|p| p.id == parent_id)
            .ok_or("Parent profile not found")?;
        
        if !matches!(parent.profile_type, ProfileType::Admin | ProfileType::Adult) {
            return Err("Only Admin/Adult can approve requests".to_string());
        }
        
        request.status = ApprovalStatus::Approved;
        request.responded_at = Some(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs());
        
        Ok(())
    }
    
    pub fn deny_request(&mut self, request_id: &str, _parent_id: &str, reason: Option<&str>) -> Result<(), String> {
        let request = self.approval_requests.iter_mut()
            .find(|r| r.id == request_id)
            .ok_or("Request not found")?;
        
        request.status = ApprovalStatus::Denied;
        request.parent_response = reason.map(|s| s.to_string());
        request.responded_at = Some(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs());
        
        Ok(())
    }
    
    pub fn pending_requests(&self) -> Vec<&ApprovalRequest> {
        self.approval_requests.iter()
            .filter(|r| r.status == ApprovalStatus::Pending)
            .collect()
    }
    
    pub fn pending_for_parent(&self, parent_id: &str) -> Vec<&ApprovalRequest> {
        self.approval_requests.iter()
            .filter(|r| {
                r.status == ApprovalStatus::Pending &&
                self.profiles.iter()
                    .find(|p| p.id == r.profile_id)
                    .map(|p| p.parent_profile_id.as_deref() == Some(parent_id))
                    .unwrap_or(false)
            })
            .collect()
    }
    
    // ========================================================================
    // WEEKLY REPORTS
    // ========================================================================
    
    pub fn generate_weekly_report(&self, profile_id: &str) -> Option<WeeklyReport> {
        let profile = self.profiles.iter().find(|p| p.id == profile_id)?;
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let week_start = now - (now % 604800); // Start of week
        let week_end = week_start + 604800;
        
        let mut top_sites: Vec<_> = profile.usage_stats.sites_visited.iter()
            .map(|(d, c)| (d.clone(), *c))
            .collect();
        top_sites.sort_by(|a, b| b.1.cmp(&a.1));
        top_sites.truncate(10);
        
        Some(WeeklyReport {
            profile_id: profile_id.to_string(),
            profile_name: profile.name.clone(),
            week_start,
            week_end,
            total_time_minutes: profile.usage_stats.today_time_minutes, // Simplified
            daily_breakdown: [0; 7], // Would need more detailed tracking
            top_sites,
            blocked_attempts: profile.usage_stats.blocked_attempts.len() as u32,
            downloads: profile.usage_stats.downloads.len() as u32,
            search_queries_count: profile.usage_stats.search_queries.len() as u32,
            most_active_hour: 12, // Would need hourly tracking
        })
    }
}

// ============================================================================
// SIMPLE TIME MODULE (no chrono dependency)
// ============================================================================

mod chrono_lite {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    pub struct Now {
        pub hour: u8,
        pub minute: u8,
        pub weekday: u8, // 0=Mon, 6=Sun
    }
    
    pub fn now() -> Now {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Simple calculation (ignoring timezone for now)
        let days = secs / 86400;
        let day_secs = secs % 86400;
        let hour = (day_secs / 3600) as u8;
        let minute = ((day_secs % 3600) / 60) as u8;
        let weekday = ((days + 3) % 7) as u8; // Jan 1, 1970 was Thursday (3)
        
        Now { hour, minute, weekday }
    }
}

// ============================================================================
// HELPERS
// ============================================================================

fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", now)
}

fn hash_pin(pin: &str) -> String {
    // Simple hash for demo - in production use Argon2
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    pin.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn verify_pin(pin: &str, stored: &str) -> bool {
    hash_pin(pin) == stored
}

pub fn extract_domain(url: &str) -> String {
    let url = url.trim_start_matches("https://")
        .trim_start_matches("http://");
    
    url.split('/').next()
        .unwrap_or(url)
        .split(':').next()
        .unwrap_or(url)
        .to_string()
}

fn is_nsfw_domain(domain: &str) -> bool {
    let nsfw_keywords = ["porn", "xxx", "adult", "nsfw", "sex", "nude"];
    nsfw_keywords.iter().any(|k| domain.contains(k))
}

fn is_gambling_domain(domain: &str) -> bool {
    let gambling = ["casino", "poker", "bet365", "draftkings", "fanduel", "gambling", "slots"];
    gambling.iter().any(|k| domain.contains(k))
}

fn is_social_media_domain(domain: &str) -> bool {
    let social = ["facebook.com", "instagram.com", "twitter.com", "x.com", "tiktok.com", 
                  "snapchat.com", "reddit.com", "discord.com"];
    social.iter().any(|s| domain.contains(s))
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_profile_creation() {
        let mut manager = ProfileManager::new();
        
        let admin_id = manager.create_admin("Parent", "1234");
        assert!(manager.get_profile(&admin_id).is_some());
        
        let kid_id = manager.create_profile("Child", ProfileType::Kid, Some(&admin_id));
        assert!(kid_id.is_ok());
    }
    
    #[test]
    fn test_kid_restrictions() {
        let restrictions = Restrictions::kid_default();
        
        assert!(restrictions.block_nsfw);
        assert!(restrictions.block_violence);
        assert!(restrictions.downloads_need_approval);
        assert!(restrictions.daily_limit_minutes.is_some());
    }
    
    #[test]
    fn test_domain_detection() {
        assert!(is_nsfw_domain("example-porn.com"));
        assert!(!is_nsfw_domain("example.com"));
        
        assert!(is_gambling_domain("bet365.com"));
        assert!(!is_gambling_domain("news.com"));
        
        assert!(is_social_media_domain("facebook.com"));
        assert!(!is_social_media_domain("github.com"));
    }
}
