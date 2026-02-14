//! Bookmark management - Store and organize bookmarks

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use anyhow::Result;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: Uuid,
    pub url: String,
    pub title: String,
    pub folder_id: Option<Uuid>,
    pub created_at: u64,
    pub favicon_url: Option<String>,
    pub tags: Vec<String>,
}

impl Bookmark {
    pub fn new(url: String, title: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            url,
            title,
            folder_id: None,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            favicon_url: None,
            tags: Vec::new(),
        }
    }
    
    pub fn domain(&self) -> Option<String> {
        url::Url::parse(&self.url).ok()
            .and_then(|u| u.host_str().map(String::from))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkFolder {
    pub id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub created_at: u64,
}

impl BookmarkFolder {
    pub fn new(name: String, parent_id: Option<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            parent_id,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct BookmarkData {
    bookmarks: Vec<Bookmark>,
    folders: Vec<BookmarkFolder>,
}

pub struct BookmarkManager {
    bookmarks: Vec<Bookmark>,
    folders: Vec<BookmarkFolder>,
    storage_path: PathBuf,
    modified: bool,
    
    // Special folder IDs
    pub bookmarks_bar_id: Uuid,
    pub other_bookmarks_id: Uuid,
}

impl BookmarkManager {
    pub fn new() -> Self {
        let storage_path = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sassy-browser")
            .join("bookmarks.json");
        
        let bookmarks_bar_id = Uuid::new_v4();
        let other_bookmarks_id = Uuid::new_v4();
        
        let mut manager = Self {
            bookmarks: Vec::new(),
            folders: vec![
                BookmarkFolder {
                    id: bookmarks_bar_id,
                    name: "Bookmarks Bar".into(),
                    parent_id: None,
                    created_at: 0,
                },
                BookmarkFolder {
                    id: other_bookmarks_id,
                    name: "Other Bookmarks".into(),
                    parent_id: None,
                    created_at: 0,
                },
            ],
            storage_path,
            modified: false,
            bookmarks_bar_id,
            other_bookmarks_id,
        };
        
        let _ = manager.load();
        
        manager
    }
    
    /// Add a bookmark
    pub fn add(&mut self, url: &str, title: &str, folder_id: Option<Uuid>) -> Uuid {
        let mut bookmark = Bookmark::new(url.to_string(), title.to_string());
        bookmark.folder_id = folder_id.or(Some(self.other_bookmarks_id));
        
        let id = bookmark.id;
        self.bookmarks.push(bookmark);
        self.modified = true;
        
        id
    }
    
    /// Add a bookmark to the bookmarks bar
    pub fn add_to_bar(&mut self, url: &str, title: &str) -> Uuid {
        self.add(url, title, Some(self.bookmarks_bar_id))
    }
    
    /// Check if URL is bookmarked
    pub fn is_bookmarked(&self, url: &str) -> bool {
        self.bookmarks.iter().any(|b| b.url == url)
    }
    
    /// Get bookmark by URL
    pub fn get_by_url(&self, url: &str) -> Option<&Bookmark> {
        self.bookmarks.iter().find(|b| b.url == url)
    }
    
    /// Get bookmark by ID
    pub fn get(&self, id: Uuid) -> Option<&Bookmark> {
        self.bookmarks.iter().find(|b| b.id == id)
    }
    
    /// Remove bookmark by URL
    pub fn remove_by_url(&mut self, url: &str) {
        self.bookmarks.retain(|b| b.url != url);
        self.modified = true;
    }
    
    /// Remove bookmark by ID
    pub fn remove(&mut self, id: Uuid) {
        self.bookmarks.retain(|b| b.id != id);
        self.modified = true;
    }
    
    /// Get all bookmarks
    pub fn all(&self) -> &[Bookmark] {
        &self.bookmarks
    }
    
    /// Get bookmarks in a folder
    pub fn in_folder(&self, folder_id: Uuid) -> Vec<&Bookmark> {
        self.bookmarks.iter()
            .filter(|b| b.folder_id == Some(folder_id))
            .collect()
    }
    
    /// Get bookmarks bar items
    pub fn bookmarks_bar(&self) -> Vec<&Bookmark> {
        self.in_folder(self.bookmarks_bar_id)
    }
    
    /// Create a folder
    pub fn create_folder(&mut self, name: &str, parent_id: Option<Uuid>) -> Uuid {
        let folder = BookmarkFolder::new(name.to_string(), parent_id);
        let id = folder.id;
        self.folders.push(folder);
        self.modified = true;
        id
    }
    
    /// Get all folders
    pub fn folders(&self) -> &[BookmarkFolder] {
        &self.folders
    }
    
    /// Get subfolders
    pub fn subfolders(&self, parent_id: Option<Uuid>) -> Vec<&BookmarkFolder> {
        self.folders.iter()
            .filter(|f| f.parent_id == parent_id)
            .collect()
    }
    
    /// Search bookmarks
    pub fn search(&self, query: &str) -> Vec<&Bookmark> {
        let query_lower = crate::fontcase::ascii_lower(query);
        self.bookmarks.iter()
            .filter(|b| {
                crate::fontcase::ascii_lower(&b.url).contains(&query_lower) ||
                crate::fontcase::ascii_lower(&b.title).contains(&query_lower) ||
                b.tags.iter().any(|t| crate::fontcase::ascii_lower(t).contains(&query_lower))
            })
            .collect()
    }
    
    /// Update bookmark
    pub fn update(&mut self, id: Uuid, title: Option<&str>, folder_id: Option<Option<Uuid>>) {
        if let Some(bookmark) = self.bookmarks.iter_mut().find(|b| b.id == id) {
            if let Some(t) = title {
                bookmark.title = t.to_string();
            }
            if let Some(f) = folder_id {
                bookmark.folder_id = f;
            }
            self.modified = true;
        }
    }
    
    /// Save bookmarks to disk
    pub fn save(&mut self) -> Result<()> {
        if !self.modified {
            return Ok(());
        }
        
        if let Some(parent) = self.storage_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let data = BookmarkData {
            bookmarks: self.bookmarks.clone(),
            folders: self.folders.clone(),
        };
        
        let json = serde_json::to_string_pretty(&data)?;
        std::fs::write(&self.storage_path, json)?;
        
        self.modified = false;
        Ok(())
    }
    
    /// Load bookmarks from disk
    pub fn load(&mut self) -> Result<()> {
        if !self.storage_path.exists() {
            return Ok(());
        }
        
        let json = std::fs::read_to_string(&self.storage_path)?;
        let data: BookmarkData = serde_json::from_str(&json)?;
        
        self.bookmarks = data.bookmarks;
        self.folders = data.folders;
        
        // Ensure special folders exist
        if !self.folders.iter().any(|f| f.name == "Bookmarks Bar") {
            self.folders.push(BookmarkFolder {
                id: self.bookmarks_bar_id,
                name: "Bookmarks Bar".into(),
                parent_id: None,
                created_at: 0,
            });
        }
        
        self.modified = false;
        Ok(())
    }
    
    /// Import from HTML bookmark file
    pub fn import_html(&mut self, content: &str) -> Result<usize> {
        // Minimal Netscape-style bookmark import: looks for <A HREF="...">Title</A>
        let mut imported = 0usize;
        for line in content.lines() {
            if let Some(href_pos) = line.to_uppercase().find("HREF=") {
                // Extract URL between quotes
                let after = &line[href_pos + 5..];
                if let Some(first_quote) = after.find('"') {
                    let rest = &after[first_quote + 1..];
                    if let Some(end_quote) = rest.find('"') {
                        let url = &rest[..end_quote];
                        // Title follows after the closing quote
                        let title = rest[end_quote + 1..]
                            .split('>')
                            .nth(1)
                            .and_then(|s| s.split("</A>").next())
                            .unwrap_or(url)
                            .trim();
                        self.add(url, title, None);
                        imported += 1;
                    }
                }
            }
        }
        Ok(imported)
    }
    
    /// Export to HTML bookmark file
    pub fn export_html(&self) -> String {
        let mut html = String::from("<!DOCTYPE NETSCAPE-Bookmark-file-1>\n");
        html.push_str("<META HTTP-EQUIV=\"Content-Type\" CONTENT=\"text/html; charset=UTF-8\">\n");
        html.push_str("<TITLE>Bookmarks</TITLE>\n");
        html.push_str("<H1>Bookmarks</H1>\n");
        html.push_str("<DL><p>\n");
        
        for bookmark in &self.bookmarks {
            html.push_str(&format!(
                "    <DT><A HREF=\"{}\">{}</A>\n",
                bookmark.url, bookmark.title
            ));
        }
        
        html.push_str("</DL><p>\n");
        html
    }
}

impl Default for BookmarkManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for BookmarkManager {
    fn drop(&mut self) {
        let _ = self.save();
    }
}
