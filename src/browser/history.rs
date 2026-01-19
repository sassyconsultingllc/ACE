#![allow(dead_code, unused_variables, unused_imports)]
//! History management - Track browsing history

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
    pub visited_at: u64,  // Unix timestamp
    pub visit_count: u32,
    pub favicon_url: Option<String>,
}

impl HistoryEntry {
    pub fn new(url: String, title: String) -> Self {
        Self {
            url,
            title,
            visited_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            visit_count: 1,
            favicon_url: None,
        }
    }
    
    pub fn domain(&self) -> Option<String> {
        url::Url::parse(&self.url).ok()
            .and_then(|u| u.host_str().map(String::from))
    }
}

pub struct HistoryManager {
    entries: VecDeque<HistoryEntry>,
    max_entries: usize,
    storage_path: PathBuf,
    modified: bool,
}

impl HistoryManager {
    pub fn new() -> Self {
        let storage_path = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sassy-browser")
            .join("history.json");
        
        let mut manager = Self {
            entries: VecDeque::new(),
            max_entries: 10000,
            storage_path,
            modified: false,
        };
        
        // Try to load existing history
        let _ = manager.load();
        
        manager
    }
    
    /// Add a URL to history
    pub fn add(&mut self, url: &str, title: &str) {
        // Skip internal URLs
        if url.starts_with("sassy://") {
            return;
        }
        
        // Check if URL exists and update visit count
        if let Some(entry) = self.entries.iter_mut().find(|e| e.url == url) {
            entry.visit_count += 1;
            entry.visited_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            if !title.is_empty() {
                entry.title = title.to_string();
            }
        } else {
            // Add new entry
            let entry = HistoryEntry::new(url.to_string(), title.to_string());
            self.entries.push_front(entry);
            
            // Trim if over limit
            while self.entries.len() > self.max_entries {
                self.entries.pop_back();
            }
        }
        
        self.modified = true;
    }
    
    /// Get recent history entries
    pub fn recent(&self, limit: usize) -> Vec<&HistoryEntry> {
        self.entries.iter().take(limit).collect()
    }
    
    /// Search history by URL or title
    pub fn search(&self, query: &str) -> Vec<&HistoryEntry> {
        let query_lower = query.to_lowercase();
        self.entries.iter()
            .filter(|e| {
                e.url.to_lowercase().contains(&query_lower) ||
                e.title.to_lowercase().contains(&query_lower)
            })
            .collect()
    }
    
    /// Get all entries
    pub fn all(&self) -> &VecDeque<HistoryEntry> {
        &self.entries
    }
    
    /// Get entries for a specific day
    pub fn for_date(&self, year: i32, month: u32, day: u32) -> Vec<&HistoryEntry> {
        // Calculate start and end timestamps for the day
        // This is a simplified calculation
        let start_of_day = chrono_date_to_timestamp(year, month, day);
        let end_of_day = start_of_day + 86400; // 24 hours
        
        self.entries.iter()
            .filter(|e| e.visited_at >= start_of_day && e.visited_at < end_of_day)
            .collect()
    }
    
    /// Clear all history
    pub fn clear(&mut self) {
        self.entries.clear();
        self.modified = true;
    }
    
    /// Delete a specific entry
    pub fn delete(&mut self, url: &str) {
        self.entries.retain(|e| e.url != url);
        self.modified = true;
    }
    
    /// Save history to disk
    pub fn save(&mut self) -> Result<()> {
        if !self.modified {
            return Ok(());
        }
        
        // Ensure directory exists
        if let Some(parent) = self.storage_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let entries: Vec<_> = self.entries.iter().cloned().collect();
        let json = serde_json::to_string_pretty(&entries)?;
        std::fs::write(&self.storage_path, json)?;
        
        self.modified = false;
        Ok(())
    }
    
    /// Load history from disk
    pub fn load(&mut self) -> Result<()> {
        if !self.storage_path.exists() {
            return Ok(());
        }
        
        let json = std::fs::read_to_string(&self.storage_path)?;
        let entries: Vec<HistoryEntry> = serde_json::from_str(&json)?;
        
        self.entries = entries.into_iter().collect();
        self.modified = false;
        
        Ok(())
    }
    
    /// Get most visited sites
    pub fn most_visited(&self, limit: usize) -> Vec<&HistoryEntry> {
        let mut entries: Vec<_> = self.entries.iter().collect();
        entries.sort_by(|a, b| b.visit_count.cmp(&a.visit_count));
        entries.into_iter().take(limit).collect()
    }
}

impl Default for HistoryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for HistoryManager {
    fn drop(&mut self) {
        let _ = self.save();
    }
}

fn chrono_date_to_timestamp(year: i32, month: u32, day: u32) -> u64 {
    // Simplified date to timestamp conversion
    // For a real implementation, use chrono crate
    let days_since_epoch = (year - 1970) as u64 * 365 + (month - 1) as u64 * 30 + day as u64;
    days_since_epoch * 86400
}
