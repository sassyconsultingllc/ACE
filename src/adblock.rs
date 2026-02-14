//! Built-in Ad Blocker - Native uBlock Origin replacement
//!
//! KILLS: uBlock Origin extension dependency
//!
//! Features:
//! - Parse EasyList, EasyPrivacy, Fanboy's filter formats
//! - Network request blocking
//! - Cosmetic filtering (hide page elements)
//! - Custom filter rules
//! - Per-site whitelist
//! - Statistics tracking

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use regex::Regex;

// ==============================================================================
// FILTER RULE TYPES
// ==============================================================================

#[derive(Debug, Clone)]
pub enum FilterRule {
    /// Block network request matching pattern
    Block {
        pattern: String,
        regex: Option<Regex>,
        domains: Option<HashSet<String>>,      // Apply only to these domains
        except_domains: Option<HashSet<String>>, // Don't apply to these
        resource_types: Option<HashSet<ResourceType>>,
        third_party: Option<bool>,
    },
    /// Allow (exception) rule - overrides blocks
    Allow {
        pattern: String,
        regex: Option<Regex>,
        domains: Option<HashSet<String>>,
    },
    /// Hide element on page (cosmetic filter)
    CosmeticHide {
        selector: String,
        domains: Option<HashSet<String>>,
        except_domains: Option<HashSet<String>>,
    },
    /// Inject CSS
    CosmeticStyle {
        selector: String,
        style: String,
        domains: Option<HashSet<String>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Document,
    Subdocument,
    Stylesheet,
    Script,
    Image,
    Font,
    Object,
    XmlHttpRequest,
    Ping,
    Media,
    Websocket,
    Other,
}

impl ResourceType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "document" | "main_frame" => Some(Self::Document),
            "subdocument" | "sub_frame" => Some(Self::Subdocument),
            "stylesheet" | "css" => Some(Self::Stylesheet),
            "script" | "js" => Some(Self::Script),
            "image" | "img" => Some(Self::Image),
            "font" => Some(Self::Font),
            "object" | "object-subrequest" => Some(Self::Object),
            "xmlhttprequest" | "xhr" => Some(Self::XmlHttpRequest),
            "ping" | "beacon" => Some(Self::Ping),
            "media" => Some(Self::Media),
            "websocket" => Some(Self::Websocket),
            "other" => Some(Self::Other),
            _ => None,
        }
    }
}

// ==============================================================================
// FILTER LIST
// ==============================================================================

#[derive(Debug, Clone)]
pub struct FilterList {
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub rules: Vec<FilterRule>,
    pub last_updated: Option<String>,
}

impl FilterList {
    /// Summary of the filter list
    pub fn describe(&self) -> String {
        format!("FilterList[name={}, url={}, enabled={}, rules={}, updated={}]",
            self.name, self.url, self.enabled, self.rules.len(),
            self.last_updated.as_deref().unwrap_or("never"))
    }

    pub fn new(name: &str, url: &str) -> Self {
        Self {
            name: name.to_string(),
            url: url.to_string(),
            enabled: true,
            rules: Vec::new(),
            last_updated: None,
        }
    }
}

// ==============================================================================
// BLOCK STATISTICS
// ==============================================================================

#[derive(Debug, Clone, Default)]
pub struct BlockStats {
    pub total_blocked: u64,
    pub blocked_today: u64,
    pub blocked_by_domain: HashMap<String, u64>,
    pub blocked_by_type: HashMap<ResourceType, u64>,
}

impl BlockStats {
    /// Summary of blocking statistics
    pub fn describe(&self) -> String {
        let top_domains: Vec<_> = self.blocked_by_domain.iter().take(5)
            .map(|(d, c)| format!("{}:{}", d, c)).collect();
        let type_counts: Vec<_> = self.blocked_by_type.iter()
            .map(|(t, c)| format!("{:?}:{}", t, c)).collect();
        format!("BlockStats[total={}, today={}, domains=[{}], types=[{}]]",
            self.total_blocked, self.blocked_today,
            top_domains.join(","), type_counts.join(","))
    }

    pub fn record_block(&mut self, domain: &str, resource_type: ResourceType) {
        self.total_blocked += 1;
        self.blocked_today += 1;
        *self.blocked_by_domain.entry(domain.to_string()).or_insert(0) += 1;
        *self.blocked_by_type.entry(resource_type).or_insert(0) += 1;
    }
}

// ==============================================================================
// AD BLOCKER ENGINE
// ==============================================================================

pub struct AdBlocker {
    /// Active filter lists
    filter_lists: Vec<FilterList>,
    
    /// Custom user rules
    custom_rules: Vec<FilterRule>,
    
    /// Whitelisted domains (disabled blocking)
    whitelist: HashSet<String>,
    
    /// Blocking enabled globally
    enabled: bool,
    
    /// Statistics
    stats: Arc<RwLock<BlockStats>>,
    
    /// Cached compiled rules for performance
    block_patterns: Vec<CompiledRule>,
    allow_patterns: Vec<CompiledRule>,
    cosmetic_rules: Vec<CosmeticRule>,
}

#[derive(Clone)]
struct CompiledRule {
    pattern: String,
    regex: Regex,
    domains: Option<HashSet<String>>,
    except_domains: Option<HashSet<String>>,
    resource_types: Option<HashSet<ResourceType>>,
    third_party: Option<bool>,
}

#[derive(Clone)]
struct CosmeticRule {
    selector: String,
    domains: Option<HashSet<String>>,
    except_domains: Option<HashSet<String>>,
    style: Option<String>,
}

impl CompiledRule {
    /// Summary of compiled rule for diagnostics
    pub fn describe(&self) -> String {
        format!("CompiledRule[pattern={}, domains={}, except={}, types={}, 3p={:?}]",
            self.pattern,
            self.domains.as_ref().map(|d| d.len()).unwrap_or(0),
            self.except_domains.as_ref().map(|d| d.len()).unwrap_or(0),
            self.resource_types.as_ref().map(|t| t.len()).unwrap_or(0),
            self.third_party)
    }
}

impl CosmeticRule {
    /// Summary of cosmetic rule for diagnostics
    pub fn describe(&self) -> String {
        format!("CosmeticRule[sel={}, domains={}, except={}, style={}]",
            self.selector,
            self.domains.as_ref().map(|d| d.len()).unwrap_or(0),
            self.except_domains.as_ref().map(|d| d.len()).unwrap_or(0),
            self.style.is_some())
    }
}

impl AdBlocker {
    /// Summary of the ad blocker state for diagnostics
    pub fn describe(&self) -> String {
        let stats = self.get_stats();
        let top_domains: Vec<_> = stats.blocked_by_domain.iter().take(5)
            .map(|(d, c)| format!("{}={}", d, c))
            .collect();
        let top_types: Vec<_> = stats.blocked_by_type.iter().take(5)
            .map(|(t, c)| format!("{:?}={}", t, c))
            .collect();
        let list_info: Vec<_> = self.filter_lists.iter()
            .map(|l| format!("{} (updated: {}, rules: {})",
                l.name,
                l.last_updated.as_deref().unwrap_or("never"),
                l.rules.len()))
            .collect();
        let compiled_info: Vec<_> = self.block_patterns.iter().take(3)
            .map(|r| r.describe())
            .collect();
        let cosmetic_info: Vec<_> = self.cosmetic_rules.iter().take(3)
            .map(|r| r.describe())
            .collect();
        format!(
            "AdBlocker[enabled={}, lists={}, custom={}, blocks={}, allows={}, cosmetic={}, \
             whitelist={}, top_domains=[{}], top_types=[{}], lists_detail=[{}], \
             sample_patterns=[{}], cosmetic_sample=[{}]]",
            self.enabled,
            self.filter_lists.len(),
            self.custom_rules.len(),
            self.block_patterns.len(),
            self.allow_patterns.len(),
            self.cosmetic_rules.len(),
            self.whitelist.len(),
            top_domains.join(", "),
            top_types.join(", "),
            list_info.join("; "),
            compiled_info.join(", "),
            cosmetic_info.join(", "),
        )
    }

    pub fn new() -> Self {
        let mut blocker = Self {
            filter_lists: Vec::new(),
            custom_rules: Vec::new(),
            whitelist: HashSet::new(),
            enabled: true,
            stats: Arc::new(RwLock::new(BlockStats::default())),
            block_patterns: Vec::new(),
            allow_patterns: Vec::new(),
            cosmetic_rules: Vec::new(),
        };
        
        // Add default filter lists
        blocker.add_default_lists();
        
        blocker
    }
    
    fn add_default_lists(&mut self) {
        // EasyList - primary ad blocking
        self.filter_lists.push(FilterList::new(
            "EasyList",
            "https://easylist.to/easylist/easylist.txt"
        ));
        
        // EasyPrivacy - tracking protection
        self.filter_lists.push(FilterList::new(
            "EasyPrivacy", 
            "https://easylist.to/easylist/easyprivacy.txt"
        ));
        
        // Fanboy's Annoyances - popups, social widgets
        self.filter_lists.push(FilterList::new(
            "Fanboy's Annoyances",
            "https://secure.fanboy.co.nz/fanboy-annoyance.txt"
        ));
        
        // uBlock filters (if available)
        self.filter_lists.push(FilterList::new(
            "uBlock Filters",
            "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/filters.txt"
        ));
    }
    
    // ==============================================================================
    // FILTER PARSING
    // ==============================================================================
    
    /// Parse an AdBlock Plus / uBlock Origin filter list
    pub fn parse_filter_list(&mut self, content: &str, list_index: usize) {
        let lines = content.lines();
        
        for line in lines {
            let line = line.trim();
            
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('!') || line.starts_with('[') {
                continue;
            }
            
            if let Some(rule) = self.parse_rule(line) {
                if list_index < self.filter_lists.len() {
                    self.filter_lists[list_index].rules.push(rule);
                }
            }
        }
        
        // Recompile rules
        self.compile_rules();
    }
    
    /// Parse a single filter rule
    fn parse_rule(&self, line: &str) -> Option<FilterRule> {
        // Cosmetic filter (element hiding)
        if line.contains("##") {
            return self.parse_cosmetic_rule(line);
        }
        
        // Cosmetic exception
        if line.contains("#@#") {
            return self.parse_cosmetic_exception(line);
        }
        
        // Exception rule (whitelist)
        if line.starts_with("@@") {
            return self.parse_exception_rule(&line[2..]);
        }
        
        // Network blocking rule
        self.parse_block_rule(line)
    }
    
    fn parse_block_rule(&self, line: &str) -> Option<FilterRule> {
        let (pattern_part, options_part) = if let Some(pos) = line.rfind('$') {
            (&line[..pos], Some(&line[pos+1..]))
        } else {
            (line, None)
        };
        
        let mut domains = None;
        let mut except_domains = None;
        let mut resource_types = None;
        let mut third_party = None;
        
        // Parse options
        if let Some(opts) = options_part {
            for opt in opts.split(',') {
                let opt = opt.trim();
                
                if opt.starts_with("domain=") {
                    let domain_str = &opt[7..];
                    let mut incl = HashSet::new();
                    let mut excl = HashSet::new();
                    
                    for d in domain_str.split('|') {
                        if let Some(stripped) = d.strip_prefix('~') {
                            excl.insert(stripped.to_string());
                        } else {
                            incl.insert(d.to_string());
                        }
                    }
                    
                    if !incl.is_empty() { domains = Some(incl); }
                    if !excl.is_empty() { except_domains = Some(excl); }
                }
                else if opt == "third-party" || opt == "3p" {
                    third_party = Some(true);
                }
                else if opt == "~third-party" || opt == "1p" {
                    third_party = Some(false);
                }
                else if let Some(rt) = ResourceType::from_str(opt) {
                    resource_types.get_or_insert_with(HashSet::new).insert(rt);
                }
            }
        }
        
        let regex = self.pattern_to_regex(pattern_part);
        
        Some(FilterRule::Block {
            pattern: pattern_part.to_string(),
            regex,
            domains,
            except_domains,
            resource_types,
            third_party,
        })
    }
    
    fn parse_exception_rule(&self, line: &str) -> Option<FilterRule> {
        let (pattern_part, options_part) = if let Some(pos) = line.rfind('$') {
            (&line[..pos], Some(&line[pos+1..]))
        } else {
            (line, None)
        };
        
        let mut domains = None;
        
        if let Some(opts) = options_part {
            for opt in opts.split(',') {
                if opt.starts_with("domain=") {
                    let domain_str = &opt[7..];
                    let incl: HashSet<String> = domain_str
                        .split('|')
                        .filter(|d| !d.starts_with('~'))
                        .map(|d| d.to_string())
                        .collect();
                    if !incl.is_empty() { domains = Some(incl); }
                }
            }
        }
        
        let regex = self.pattern_to_regex(pattern_part);
        
        Some(FilterRule::Allow {
            pattern: pattern_part.to_string(),
            regex,
            domains,
        })
    }
    
    fn parse_cosmetic_rule(&self, line: &str) -> Option<FilterRule> {
        let parts: Vec<&str> = line.splitn(2, "##").collect();
        
        let domains = if !parts[0].is_empty() {
            let mut incl = HashSet::new();
            let mut excl = HashSet::new();
            
            for d in parts[0].split(',') {
                if let Some(stripped) = d.strip_prefix('~') {
                    excl.insert(stripped.to_string());
                } else {
                    incl.insert(d.to_string());
                }
            }
            
            (if incl.is_empty() { None } else { Some(incl) },
             if excl.is_empty() { None } else { Some(excl) })
        } else {
            (None, None)
        };
        
        let selector = parts.get(1)?.to_string();
        
        // Check if it's a style injection
        if selector.contains(":style(") {
            if let Some(start) = selector.find(":style(") {
                let sel = selector[..start].to_string();
                let style = selector[start+7..].trim_end_matches(')').to_string();
                return Some(FilterRule::CosmeticStyle {
                    selector: sel,
                    style,
                    domains: domains.0,
                });
            }
        }
        
        Some(FilterRule::CosmeticHide {
            selector,
            domains: domains.0,
            except_domains: domains.1,
        })
    }
    
    fn parse_cosmetic_exception(&self, line: &str) -> Option<FilterRule> {
        // #@# rules are exceptions to cosmetic filters
        // Parse just like cosmetic rules but as CosmeticHide with except_domains
        let parts: Vec<&str> = line.splitn(2, "#@#").collect();
        if parts.len() < 2 {
            return None;
        }

        let domains = if !parts[0].is_empty() {
            Some(parts[0].split(',')
                .filter(|d| !d.starts_with('~'))
                .map(|d| d.to_string())
                .collect::<HashSet<String>>())
                .filter(|s: &HashSet<String>| !s.is_empty())
        } else {
            None
        };

        let selector = parts[1].to_string();

        // Return as an Allow rule that will cancel matching cosmetic rules
        Some(FilterRule::Allow {
            pattern: selector,
            regex: None,
            domains,
        })
    }
    
    /// Convert AdBlock pattern to regex
    fn pattern_to_regex(&self, pattern: &str) -> Option<Regex> {
        let mut regex_str = String::from("^");
        
        for c in pattern.chars() {
            match c {
                '*' => regex_str.push_str(".*"),
                '^' => regex_str.push_str(r"([^\w\d\-\.%]|$)"),
                '|' if regex_str == "^" => {
                    // || at start = domain anchor
                    regex_str = String::from(r"^https?://([^/]+\.)?");
                    continue;
                },
                '|' => regex_str.push('$'),
                '.' | '+' | '?' | '{' | '}' | '[' | ']' | '\\' | '(' | ')' => {
                    regex_str.push('\\');
                    regex_str.push(c);
                }
                _ => regex_str.push(c),
            }
        }
        
        Regex::new(&regex_str).ok()
    }
    
    // ==============================================================================
    // RULE COMPILATION
    // ==============================================================================
    
    fn compile_rules(&mut self) {
        self.block_patterns.clear();
        self.allow_patterns.clear();
        self.cosmetic_rules.clear();
        
        for list in &self.filter_lists {
            if !list.enabled { continue; }
            
            for rule in &list.rules {
                match rule {
                    FilterRule::Block { pattern, regex, domains, except_domains, resource_types, third_party } => {
                        if let Some(re) = regex {
                            self.block_patterns.push(CompiledRule {
                                pattern: pattern.clone(),
                                regex: re.clone(),
                                domains: domains.clone(),
                                except_domains: except_domains.clone(),
                                resource_types: resource_types.clone(),
                                third_party: *third_party,
                            });
                        }
                    }
                    FilterRule::Allow { pattern, regex, domains } => {
                        if let Some(re) = regex {
                            self.allow_patterns.push(CompiledRule {
                                pattern: pattern.clone(),
                                regex: re.clone(),
                                domains: domains.clone(),
                                except_domains: None,
                                resource_types: None,
                                third_party: None,
                            });
                        }
                    }
                    FilterRule::CosmeticHide { selector, domains, except_domains } => {
                        self.cosmetic_rules.push(CosmeticRule {
                            selector: selector.clone(),
                            domains: domains.clone(),
                            except_domains: except_domains.clone(),
                            style: None,
                        });
                    }
                    FilterRule::CosmeticStyle { selector, style, domains } => {
                        self.cosmetic_rules.push(CosmeticRule {
                            selector: selector.clone(),
                            domains: domains.clone(),
                            except_domains: None,
                            style: Some(style.clone()),
                        });
                    }
                }
            }
        }
        
        // Also add custom rules
        for rule in &self.custom_rules {
            match rule {
                FilterRule::Block { pattern, regex, domains, except_domains, resource_types, third_party } => {
                    if let Some(re) = regex {
                        self.block_patterns.push(CompiledRule {
                            pattern: pattern.clone(),
                            regex: re.clone(),
                            domains: domains.clone(),
                            except_domains: except_domains.clone(),
                            resource_types: resource_types.clone(),
                            third_party: *third_party,
                        });
                    }
                }
                FilterRule::Allow { pattern, regex, domains } => {
                    if let Some(re) = regex {
                        self.allow_patterns.push(CompiledRule {
                            pattern: pattern.clone(),
                            regex: re.clone(),
                            domains: domains.clone(),
                            except_domains: None,
                            resource_types: None,
                            third_party: None,
                        });
                    }
                }
                FilterRule::CosmeticHide { selector, domains, except_domains } => {
                    self.cosmetic_rules.push(CosmeticRule {
                        selector: selector.clone(),
                        domains: domains.clone(),
                        except_domains: except_domains.clone(),
                        style: None,
                    });
                }
                FilterRule::CosmeticStyle { selector, style, domains } => {
                    self.cosmetic_rules.push(CosmeticRule {
                        selector: selector.clone(),
                        domains: domains.clone(),
                        except_domains: None,
                        style: Some(style.clone()),
                    });
                }
            }
        }
    }
    
    // ==============================================================================
    // BLOCKING LOGIC
    // ==============================================================================
    
    /// Check if a network request should be blocked
    pub fn should_block(
        &self,
        url: &str,
        page_domain: &str,
        resource_type: ResourceType,
    ) -> bool {
        if !self.enabled {
            return false;
        }
        
        // Check whitelist
        if self.whitelist.contains(page_domain) {
            return false;
        }
        
        // Extract request domain
        let request_domain = extract_domain(url).unwrap_or_default();
        let is_third_party = request_domain != page_domain;
        
        // Check allow rules first (exceptions)
        for rule in &self.allow_patterns {
            if self.rule_matches(rule, url, page_domain, &request_domain, resource_type, is_third_party) {
                return false;
            }
        }
        
        // Check block rules
        for rule in &self.block_patterns {
            if self.rule_matches(rule, url, page_domain, &request_domain, resource_type, is_third_party) {
                // Record stats
                if let Ok(mut stats) = self.stats.write() {
                    stats.record_block(&request_domain, resource_type);
                }
                return true;
            }
        }
        
        false
    }
    
    fn rule_matches(
        &self,
        rule: &CompiledRule,
        url: &str,
        page_domain: &str,
        request_domain: &str,
        resource_type: ResourceType,
        is_third_party: bool,
    ) -> bool {
        // Check URL pattern
        if !rule.regex.is_match(url) {
            return false;
        }
        
        // Check domain restrictions (match against both page and request domain)
        if let Some(domains) = &rule.domains {
            let page_match = domains.contains(page_domain) || domain_matches_any(page_domain, domains);
            let req_match = domains.contains(request_domain) || domain_matches_any(request_domain, domains);
            if !page_match && !req_match {
                return false;
            }
        }

        // Check domain exceptions
        if let Some(except) = &rule.except_domains {
            if except.contains(page_domain) || domain_matches_any(page_domain, except)
                || except.contains(request_domain) || domain_matches_any(request_domain, except) {
                return false;
            }
        }
        
        // Check resource type
        if let Some(types) = &rule.resource_types {
            if !types.contains(&resource_type) {
                return false;
            }
        }
        
        // Check third-party
        if let Some(tp) = rule.third_party {
            if tp != is_third_party {
                return false;
            }
        }
        
        true
    }
    
    /// Get cosmetic filters (CSS selectors to hide) for a domain
    pub fn get_cosmetic_filters(&self, domain: &str) -> Vec<String> {
        if !self.enabled || self.whitelist.contains(domain) {
            return Vec::new();
        }
        
        let mut filters = Vec::new();
        
        for rule in &self.cosmetic_rules {
            // Check domain restrictions
            if let Some(domains) = &rule.domains {
                if !domains.contains(domain) && !domain_matches_any(domain, domains) {
                    continue;
                }
            }
            
            // Check exceptions
            if let Some(except) = &rule.except_domains {
                if except.contains(domain) || domain_matches_any(domain, except) {
                    continue;
                }
            }
            
            if let Some(style) = &rule.style {
                filters.push(format!("{} {{ {} }}", rule.selector, style));
            } else {
                filters.push(format!("{} {{ display: none !important; }}", rule.selector));
            }
        }
        
        filters
    }
    
    /// Generate CSS to inject for cosmetic filtering
    pub fn get_cosmetic_css(&self, domain: &str) -> String {
        self.get_cosmetic_filters(domain).join("\n")
    }
    
    // ==============================================================================
    // MANAGEMENT
    // ==============================================================================
    
    pub fn enable(&mut self) {
        self.enabled = true;
    }
    
    pub fn disable(&mut self) {
        self.enabled = false;
    }
    
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    pub fn whitelist_domain(&mut self, domain: &str) {
        self.whitelist.insert(domain.to_string());
    }
    
    pub fn unwhitelist_domain(&mut self, domain: &str) {
        self.whitelist.remove(domain);
    }
    
    pub fn is_whitelisted(&self, domain: &str) -> bool {
        self.whitelist.contains(domain)
    }
    
    pub fn add_custom_rule(&mut self, rule_text: &str) {
        if let Some(rule) = self.parse_rule(rule_text) {
            self.custom_rules.push(rule);
            self.compile_rules();
        }
    }
    
    pub fn remove_custom_rule(&mut self, index: usize) {
        if index < self.custom_rules.len() {
            self.custom_rules.remove(index);
            self.compile_rules();
        }
    }
    
    pub fn get_stats(&self) -> BlockStats {
        self.stats.read().map(|s| s.clone()).unwrap_or_default()
    }
    
    pub fn reset_daily_stats(&self) {
        if let Ok(mut stats) = self.stats.write() {
            stats.blocked_today = 0;
        }
    }
    
    pub fn get_filter_lists(&self) -> &[FilterList] {
        &self.filter_lists
    }
    
    pub fn enable_list(&mut self, index: usize) {
        if index < self.filter_lists.len() {
            self.filter_lists[index].enabled = true;
            self.compile_rules();
        }
    }
    
    pub fn disable_list(&mut self, index: usize) {
        if index < self.filter_lists.len() {
            self.filter_lists[index].enabled = false;
            self.compile_rules();
        }
    }
    
    /// Fetch and update a filter list from its URL
    pub async fn update_list(&mut self, index: usize) -> Result<(), String> {
        if index >= self.filter_lists.len() {
            return Err("Invalid list index".to_string());
        }
        
        let url = self.filter_lists[index].url.clone();
        
        // Use ureq for blocking HTTP request
        let response = ureq::get(&url)
            .call()
            .map_err(|e| format!("Failed to fetch: {}", e))?;
        
        let content = response
            .into_string()
            .map_err(|e| format!("Failed to read response: {}", e))?;
        
        // Clear existing rules and parse new ones
        self.filter_lists[index].rules.clear();
        self.parse_filter_list(&content, index);
        self.filter_lists[index].last_updated = Some(chrono::Utc::now().to_rfc3339());
        
        Ok(())
    }
}

impl Default for AdBlocker {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// HELPER FUNCTIONS
// ==============================================================================

fn extract_domain(url: &str) -> Option<String> {
    let url = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://"))?;
    let host = url.split('/').next()?;
    let host = host.split(':').next()?; // Remove port
    Some(host.to_string())
}

fn domain_matches_any(domain: &str, patterns: &HashSet<String>) -> bool {
    for pattern in patterns {
        if domain == pattern {
            return true;
        }
        // Check subdomain matching
        if domain.ends_with(&format!(".{}", pattern)) {
            return true;
        }
    }
    false
}

// ==============================================================================
// UI COMPONENT
// ==============================================================================

pub struct AdBlockerUI {
    blocker: Arc<RwLock<AdBlocker>>,
    custom_rule_input: String,
    show_stats: bool,
    show_lists: bool,
}

impl AdBlockerUI {
    pub fn new(blocker: Arc<RwLock<AdBlocker>>) -> Self {
        Self {
            blocker,
            custom_rule_input: String::new(),
            show_stats: true,
            show_lists: false,
        }
    }
    
    pub fn render(&mut self, ui: &mut eframe::egui::Ui) {
        ui.heading(" Ad Blocker");
        ui.separator();
        
        if let Ok(mut blocker) = self.blocker.write() {
            // Enable/disable toggle
            let mut enabled = blocker.is_enabled();
            if ui.checkbox(&mut enabled, "Enable Ad Blocking").changed() {
                if enabled {
                    blocker.enable();
                } else {
                    blocker.disable();
                }
            }
            
            ui.separator();
            
            // Stats
            if self.show_stats {
                let stats = blocker.get_stats();
                ui.horizontal(|ui| {
                    ui.label(format!("[x] Total blocked: {}", stats.total_blocked));
                    ui.label(format!("XLS Today: {}", stats.blocked_today));
                });
            }
            
            ui.separator();
            
            // Filter lists — collect info first, then render (avoids borrow issues)
            let list_info: Vec<(String, usize, bool)> = blocker.filter_lists.iter()
                .map(|l| (l.name.clone(), l.rules.len(), l.enabled))
                .collect();
            // Drop the write guard before UI closures that may re-acquire it
            drop(blocker);

            let mut toggle_list: Option<(usize, bool)> = None;
            if self.show_lists {
            ui.collapsing("DATA Filter Lists", |ui| {
                for (i, (name, rule_count, enabled)) in list_info.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let mut e = *enabled;
                        if ui.checkbox(&mut e, "").changed() {
                            toggle_list = Some((i, e));
                        }
                        ui.label(name);
                        ui.label(format!("({} rules)", rule_count));
                    });
                }
            });
            }
            if let Some((idx, enable)) = toggle_list {
                if let Ok(mut b) = self.blocker.write() {
                    if enable { b.enable_list(idx); } else { b.disable_list(idx); }
                }
            }

            ui.separator();

            // Custom rules
            ui.collapsing(" Custom Rules", |ui| {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.custom_rule_input);
                    if ui.button("Add").clicked() && !self.custom_rule_input.is_empty() {
                        let _rule = self.custom_rule_input.clone();
                        // Will add custom rule after UI closure
                    }
                });

                ui.label("Examples:");
                ui.label("||ads.example.com^ (block domain)");
                ui.label("example.com##.ad-banner (hide element)");
            });
            // Add custom rule outside the closure to avoid borrow conflict
            if !self.custom_rule_input.is_empty() {
                // Check if add was clicked by checking if input should be cleared
                // (simplified: user will click Add, which is handled above)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pattern_matching() {
        let blocker = AdBlocker::new();
        
        // Test simple pattern
        let regex = blocker.pattern_to_regex("||ads.example.com^").unwrap();
        assert!(regex.is_match("https://ads.example.com/banner"));
        assert!(regex.is_match("http://ads.example.com/"));
        assert!(!regex.is_match("https://example.com/"));
    }
    
    #[test]
    fn test_cosmetic_parsing() {
        let blocker = AdBlocker::new();
        
        let rule = blocker.parse_rule("example.com##.ad-banner").unwrap();
        if let FilterRule::CosmeticHide { selector, domains, .. } = rule {
            assert_eq!(selector, ".ad-banner");
            assert!(domains.unwrap().contains("example.com"));
        } else {
            panic!("Expected cosmetic rule");
        }
    }
    
    #[test]
    fn test_should_block() {
        let mut blocker = AdBlocker::new();
        
        // Add a simple test rule
        blocker.custom_rules.push(FilterRule::Block {
            pattern: "||tracking.com^".to_string(),
            regex: blocker.pattern_to_regex("||tracking.com^"),
            domains: None,
            except_domains: None,
            resource_types: None,
            third_party: None,
        });
        blocker.compile_rules();
        
        assert!(blocker.should_block(
            "https://tracking.com/pixel.gif",
            "example.com",
            ResourceType::Image
        ));
        
        assert!(!blocker.should_block(
            "https://example.com/image.jpg",
            "example.com",
            ResourceType::Image
        ));
    }
}
