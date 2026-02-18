// ==============================================================================
// SASSY BROWSER - SMART HISTORY
// ==============================================================================
// History doesn't save for 14.7 seconds. Click a bad link? Hit back. Gone.
// Stay on a page? It's saved. NSFW auto-detected and excluded from sync.
// YOUR HISTORY BECOMES INTENTIONAL.
// ==============================================================================


use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

// The magic number: how long before a page visit becomes "intentional"
pub const INTENT_DELAY_SECS: f64 = 14.7;

// ==============================================================================
// HISTORY ENTRY
// ==============================================================================

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub domain: String,
    pub visit_time: u64,          // Unix timestamp
    pub duration_secs: Option<u64>, // How long user stayed
    pub scroll_depth: f32,         // 0.0 - 1.0, how far they scrolled
    pub is_nsfw: bool,
    pub nsfw_confidence: f32,      // 0.0 - 1.0
    pub exclude_from_sync: bool,
    pub visit_count: u32,
    pub favicon_url: Option<String>,
    pub referrer: Option<String>,
    pub search_query: Option<String>, // If came from search
}

impl HistoryEntry {
    pub fn new(url: &str, title: &str) -> Self {
        let domain = extract_domain(url);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            id: now ^ (url.len() as u64),
            url: url.to_string(),
            title: title.to_string(),
            domain,
            visit_time: now,
            duration_secs: None,
            scroll_depth: 0.0,
            is_nsfw: false,
            nsfw_confidence: 0.0,
            exclude_from_sync: false,
            visit_count: 1,
            favicon_url: None,
            referrer: None,
            search_query: None,
        }
    }
}

// ==============================================================================
// PENDING VISIT (not yet committed to history)
// ==============================================================================

#[derive(Debug, Clone)]
struct PendingVisit {
    url: String,
    title: String,
    started_at: Instant,
    scroll_depth: f32,
    referrer: Option<String>,
}

// ==============================================================================
// NSFW DETECTION
// ==============================================================================

#[derive(Debug, Clone)]
pub struct NsfwDetector {
    // Known NSFW domains (hashed for privacy)
    known_domains: Vec<u64>,
    // Keywords that suggest NSFW content
    keywords: Vec<&'static str>,
    // User-added exclusions
    user_excluded_domains: Vec<String>,
    // Detection sensitivity (0.0 - 1.0)
    sensitivity: f32,
}

impl NsfwDetector {
    pub fn new() -> Self {
        Self {
            known_domains: Vec::new(), // Would be populated from curated list
            keywords: vec![
                "xxx", "porn", "adult", "nsfw", "18+", "mature",
                "explicit", "nude", "naked", "sex", "erotic",
                "onlyfans", "chaturbate", "xvideos", "pornhub",
            ],
            user_excluded_domains: Vec::new(),
            sensitivity: 0.5,
        }
    }
    
    pub fn analyze(&self, url: &str, title: &str) -> (bool, f32) {
        let url_lower = crate::fontcase::ascii_lower(url);
        let title_lower = crate::fontcase::ascii_lower(title);
        let domain = crate::fontcase::ascii_lower(&extract_domain(url));
        
        // Check user exclusions first
        if self.user_excluded_domains.iter().any(|d| domain.contains(d)) {
            return (true, 1.0);
        }
        
        let mut score: f32 = 0.0;
        
        // Check URL for keywords
        for keyword in &self.keywords {
            if url_lower.contains(keyword) {
                score += 0.3;
            }
            if title_lower.contains(keyword) {
                score += 0.4;
            }
            if domain.contains(keyword) {
                score += 0.5;
            }
        }
        
        // Check for suspicious TLDs
        let suspicious_tlds = [".xxx", ".adult", ".sex", ".porn"];
        for tld in suspicious_tlds {
            if domain.ends_with(tld) {
                score += 0.8;
            }
        }
        
        // Cap at 1.0
        score = score.min(1.0);
        
        let is_nsfw = score >= self.sensitivity;
        
        (is_nsfw, score)
    }
    
    pub fn add_excluded_domain(&mut self, domain: &str) {
        if !self.user_excluded_domains.contains(&domain.to_string()) {
            self.user_excluded_domains.push(domain.to_string());
        }
    }
    
    pub fn remove_excluded_domain(&mut self, domain: &str) {
        self.user_excluded_domains.retain(|d| d != domain);
    }
    
    pub fn set_sensitivity(&mut self, sensitivity: f32) {
        self.sensitivity = sensitivity.clamp(0.0, 1.0);
    }
}

// ==============================================================================
// SMART HISTORY MANAGER
// ==============================================================================

pub struct SmartHistory {
    // Committed history
    entries: Vec<HistoryEntry>,
    // Pending visits (not yet committed)
    pending: HashMap<String, PendingVisit>,
    // Recently navigated away (for "undo" within grace period)
    recent_navigations: VecDeque<(String, Instant)>,
    // NSFW detector
    nsfw_detector: NsfwDetector,
    // Settings
    intent_delay: Duration,
    max_entries: usize,
    auto_exclude_nsfw: bool,
    incognito_mode: bool,
    /// Auto-expire committed entries older than this (None = keep forever)
    retention_days: Option<u64>,
    // Stats
    total_visits: u64,
    nsfw_blocked: u64,
    entries_pruned: u64,
}

impl SmartHistory {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            pending: HashMap::new(),
            recent_navigations: VecDeque::with_capacity(100),
            nsfw_detector: NsfwDetector::new(),
            intent_delay: Duration::from_secs_f64(INTENT_DELAY_SECS),
            max_entries: 100_000,
            auto_exclude_nsfw: true,
            incognito_mode: false,
            retention_days: None,
            total_visits: 0,
            nsfw_blocked: 0,
            entries_pruned: 0,
        }
    }

    /// Analyze a URL/title for NSFW characteristics (convenience wrapper)
    pub fn analyze(&self, url: &str, title: &str) -> (bool, f32) {
        self.nsfw_detector.analyze(url, title)
    }
    
    // ==============================================================================
    // VISIT TRACKING
    // ==============================================================================
    
    /// Called when user navigates to a page
    pub fn visit(&mut self, url: &str, title: &str, referrer: Option<&str>) {
        if self.incognito_mode {
            return;
        }
        
        // Skip internal pages
        if url.starts_with("sassy://") || url.starts_with("about:") {
            return;
        }
        
        self.total_visits += 1;
        
        // Check for NSFW
        let (is_nsfw, _confidence) = self.nsfw_detector.analyze(url, title);
        if is_nsfw && self.auto_exclude_nsfw {
            self.nsfw_blocked += 1;
            // Still track in pending but mark for exclusion
        }
        
        // Add to pending
        let pending = PendingVisit {
            url: url.to_string(),
            title: title.to_string(),
            started_at: Instant::now(),
            scroll_depth: 0.0,
            referrer: referrer.map(|s| s.to_string()),
        };
        
        self.pending.insert(url.to_string(), pending);
    }
    
    /// Called when user scrolls on the page
    pub fn update_scroll(&mut self, url: &str, depth: f32) {
        if let Some(pending) = self.pending.get_mut(url) {
            pending.scroll_depth = pending.scroll_depth.max(depth);
        }
    }
    
    /// Called when user navigates away from a page
    pub fn leave(&mut self, url: &str) {
        if let Some(pending) = self.pending.remove(url) {
            let elapsed = pending.started_at.elapsed();
            
            // Only commit if stayed longer than intent delay
            if elapsed >= self.intent_delay {
                self.commit_visit(pending, elapsed);
            } else {
                // Track recent navigation for potential "undo"
                self.recent_navigations.push_front((url.to_string(), Instant::now()));
                if self.recent_navigations.len() > 100 {
                    self.recent_navigations.pop_back();
                }
            }
        }
    }
    
    /// Manually commit a visit (e.g., user bookmarked the page)
    pub fn force_commit(&mut self, url: &str) {
        if let Some(pending) = self.pending.remove(url) {
            let elapsed = pending.started_at.elapsed();
            self.commit_visit(pending, elapsed);
        }
    }
    
    fn commit_visit(&mut self, pending: PendingVisit, duration: Duration) {
        let (is_nsfw, confidence) = self.nsfw_detector.analyze(&pending.url, &pending.title);
        
        // Check if we already have this URL
        if let Some(existing) = self.entries.iter_mut().find(|e| e.url == pending.url) {
            existing.visit_count += 1;
            existing.visit_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            existing.duration_secs = Some(duration.as_secs());
            existing.scroll_depth = pending.scroll_depth;
            existing.title = pending.title; // Update title in case it changed
            return;
        }
        
        let mut entry = HistoryEntry::new(&pending.url, &pending.title);
        entry.duration_secs = Some(duration.as_secs());
        entry.scroll_depth = pending.scroll_depth;
        entry.referrer = pending.referrer;
        entry.is_nsfw = is_nsfw;
        entry.nsfw_confidence = confidence;
        entry.exclude_from_sync = is_nsfw && self.auto_exclude_nsfw;
        
        self.entries.push(entry);
        
        // Prune if over limit
        if self.entries.len() > self.max_entries {
            self.prune();
        }
    }
    
    /// Process pending visits (call periodically)
    pub fn tick(&mut self) {
        let now = Instant::now();
        let intent_delay = self.intent_delay;
        
        // Find visits that have exceeded the intent delay
        let to_commit: Vec<_> = self.pending.iter()
            .filter(|(_, v)| now.duration_since(v.started_at) >= intent_delay)
            .map(|(k, _)| k.clone())
            .collect();
        
        for url in to_commit {
            if let Some(pending) = self.pending.remove(&url) {
                let elapsed = pending.started_at.elapsed();
                self.commit_visit(pending, elapsed);
            }
        }
        
        // Sweep committed entries past retention policy
        if let Some(days) = self.retention_days {
            let retention_secs = days * 86400;
            let now_epoch = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let before = self.entries.len();
            self.entries.retain(|e| {
                now_epoch.saturating_sub(e.visit_time) < retention_secs
            });
            let removed = before - self.entries.len();
            self.entries_pruned += removed as u64;
        }

        // Clean up old recent navigations
        let cutoff = now - Duration::from_secs(60);
        self.recent_navigations.retain(|(_, t)| *t > cutoff);
    }
    
    // ==============================================================================
    // QUERIES
    // ==============================================================================
    
    pub fn search(&self, query: &str) -> Vec<&HistoryEntry> {
        let query_lower = crate::fontcase::ascii_lower(query);

        self.entries.iter()
            .filter(|e| {
                crate::fontcase::ascii_lower(&e.url).contains(&query_lower) ||
                crate::fontcase::ascii_lower(&e.title).contains(&query_lower) ||
                crate::fontcase::ascii_lower(&e.domain).contains(&query_lower)
            })
            .collect()
    }
    
    pub fn recent(&self, count: usize) -> Vec<&HistoryEntry> {
        let mut entries: Vec<_> = self.entries.iter().collect();
        entries.sort_by(|a, b| b.visit_time.cmp(&a.visit_time));
        entries.truncate(count);
        entries
    }
    
    pub fn for_domain(&self, domain: &str) -> Vec<&HistoryEntry> {
        let domain_lower = crate::fontcase::ascii_lower(domain);
        self.entries.iter()
            .filter(|e| crate::fontcase::ascii_lower(&e.domain) == domain_lower)
            .collect()
    }
    
    pub fn most_visited(&self, count: usize) -> Vec<&HistoryEntry> {
        let mut entries: Vec<_> = self.entries.iter().collect();
        entries.sort_by(|a, b| b.visit_count.cmp(&a.visit_count));
        entries.truncate(count);
        entries
    }
    
    pub fn syncable(&self) -> Vec<&HistoryEntry> {
        self.entries.iter()
            .filter(|e| !e.exclude_from_sync)
            .collect()
    }
    
    pub fn nsfw_entries(&self) -> Vec<&HistoryEntry> {
        self.entries.iter()
            .filter(|e| e.is_nsfw)
            .collect()
    }
    
    pub fn by_date(&self, start: u64, end: u64) -> Vec<&HistoryEntry> {
        self.entries.iter()
            .filter(|e| e.visit_time >= start && e.visit_time <= end)
            .collect()
    }
    
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
    
    pub fn total_count(&self) -> usize {
        self.entries.len()
    }
    
    pub fn stats(&self) -> HistoryStats {
        HistoryStats {
            total_visits: self.total_visits,
            committed_entries: self.entries.len() as u64,
            pending_entries: self.pending.len() as u64,
            nsfw_blocked: self.nsfw_blocked,
            entries_pruned: self.entries_pruned,
            unique_domains: self.entries.iter()
                .map(|e| &e.domain)
                .collect::<std::collections::HashSet<_>>()
                .len() as u64,
        }
    }
    
    // ==============================================================================
    // MANAGEMENT
    // ==============================================================================
    
    pub fn delete(&mut self, id: u64) {
        self.entries.retain(|e| e.id != id);
    }
    
    pub fn delete_for_domain(&mut self, domain: &str) {
        let domain_lower = crate::fontcase::ascii_lower(domain);
        self.entries.retain(|e| crate::fontcase::ascii_lower(&e.domain) != domain_lower);
    }
    
    pub fn delete_range(&mut self, start: u64, end: u64) {
        self.entries.retain(|e| e.visit_time < start || e.visit_time > end);
    }
    
    pub fn clear(&mut self) {
        self.entries.clear();
        self.pending.clear();
    }
    
    pub fn clear_nsfw(&mut self) {
        self.entries.retain(|e| !e.is_nsfw);
    }

    pub fn exclude_domain(&mut self, domain: &str) {
        self.nsfw_detector.add_excluded_domain(domain);
        for entry in self.entries.iter_mut().filter(|e| e.domain == domain) {
            entry.is_nsfw = true;
            entry.exclude_from_sync = true;
            entry.nsfw_confidence = entry.nsfw_confidence.max(1.0);
        }
    }

    pub fn include_domain(&mut self, domain: &str) {
        self.nsfw_detector.remove_excluded_domain(domain);
        for entry in self.entries.iter_mut().filter(|e| e.domain == domain) {
            entry.exclude_from_sync = false;
            entry.is_nsfw = false;
        }
    }
    
    fn prune(&mut self) {
        // Remove oldest entries until under limit
        self.entries.sort_by(|a, b| b.visit_time.cmp(&a.visit_time));
        let removed = self.entries.len() - self.max_entries;
        self.entries.truncate(self.max_entries);
        self.entries_pruned += removed as u64;
    }
    
    // ==============================================================================
    // SETTINGS
    // ==============================================================================
    
    pub fn set_intent_delay(&mut self, seconds: f64) {
        self.intent_delay = Duration::from_secs_f64(seconds.max(0.0));
    }
    
    pub fn intent_delay_secs(&self) -> f64 {
        self.intent_delay.as_secs_f64()
    }
    
    pub fn set_auto_exclude_nsfw(&mut self, enabled: bool) {
        self.auto_exclude_nsfw = enabled;
    }
    
    pub fn set_incognito(&mut self, enabled: bool) {
        self.incognito_mode = enabled;
        if enabled {
            self.pending.clear();
        }
    }

    /// Set retention policy: entries older than `days` are auto-expired on tick().
    /// Pass `None` to keep entries forever (default).
    pub fn set_retention_policy(&mut self, days: Option<u64>) {
        self.retention_days = days;
    }

    pub fn retention_days(&self) -> Option<u64> {
        self.retention_days
    }
    
    pub fn is_incognito(&self) -> bool {
        self.incognito_mode
    }
    
    pub fn nsfw_detector(&mut self) -> &mut NsfwDetector {
        &mut self.nsfw_detector
    }
    
    // ==============================================================================
    // UNDO NAVIGATION
    // ==============================================================================
    
    /// Check if a URL was recently navigated away from (within grace period)
    pub fn was_recently_left(&self, url: &str) -> bool {
        let grace = Duration::from_secs(30);
        let now = Instant::now();
        
        self.recent_navigations.iter()
            .any(|(u, t)| u == url && now.duration_since(*t) < grace)
    }
    
    /// Get URLs that were navigated away from within the last N seconds
    pub fn recent_abandoned(&self, within_secs: u64) -> Vec<&str> {
        let cutoff = Instant::now() - Duration::from_secs(within_secs);
        
        self.recent_navigations.iter()
            .filter(|(_, t)| *t > cutoff)
            .map(|(u, _)| u.as_str())
            .collect()
    }
}

// ==============================================================================
// STATS
// ==============================================================================

#[derive(Debug, Clone)]
pub struct HistoryStats {
    pub total_visits: u64,
    pub committed_entries: u64,
    pub pending_entries: u64,
    pub nsfw_blocked: u64,
    pub entries_pruned: u64,
    pub unique_domains: u64,
}

// ==============================================================================
// HELPERS
// ==============================================================================

fn extract_domain(url: &str) -> String {
    let url = url.trim_start_matches("https://")
        .trim_start_matches("http://");
    
    url.split('/').next()
        .unwrap_or(url)
        .split(':').next()
        .unwrap_or(url)
        .to_string()
}

// ==============================================================================
// TESTS
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    
    #[test]
    fn test_intent_delay() {
        let mut history = SmartHistory::new();
        history.set_intent_delay(0.1); // 100ms for testing
        
        history.visit("https://example.com", "Example", None);
        
        // Leave immediately - should NOT be committed
        history.leave("https://example.com");
        assert_eq!(history.total_count(), 0);
        
        // Visit again and wait
        history.visit("https://example.com", "Example", None);
        thread::sleep(Duration::from_millis(150));
        history.tick();
        
        // Should be committed now
        assert_eq!(history.total_count(), 1);
    }
    
    #[test]
    fn test_nsfw_detection() {
        let detector = NsfwDetector::new();
        
        let (is_nsfw, _) = detector.analyze("https://example.com", "Normal Site");
        assert!(!is_nsfw);
        
        let (is_nsfw, _) = detector.analyze("https://pornsite.xxx", "Adult Content");
        assert!(is_nsfw);
    }
    
    #[test]
    fn test_search() {
        let mut history = SmartHistory::new();
        history.set_intent_delay(0.0);
        
        history.visit("https://rust-lang.org", "Rust Programming", None);
        history.tick();
        history.visit("https://python.org", "Python", None);
        history.tick();
        
        let results = history.search("rust");
        assert_eq!(results.len(), 1);
        assert!(results[0].url.contains("rust"));
    }
}
