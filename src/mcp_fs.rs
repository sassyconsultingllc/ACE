//! MCP File System Tools - AI-Controlled File Operations
//!
//! Provides safe, sandboxed file system access for the AI agents.
//! All operations are logged and can be reviewed before execution.


use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::io;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Result type for file operations
pub type FileResult<T> = Result<T, FileError>;

/// File operation errors
#[derive(Debug, Clone)]
pub enum FileError {
    NotFound(String),
    PermissionDenied(String),
    OutsideSandbox(String),
    IoError(String),
    InvalidPath(String),
    AlreadyExists(String),
    TooLarge { path: String, size: u64, max: u64 },
}

impl std::fmt::Display for FileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileError::NotFound(path) => write!(f, "File not found: {}", path),
            FileError::PermissionDenied(path) => write!(f, "Permission denied: {}", path),
            FileError::OutsideSandbox(path) => write!(f, "Path outside sandbox: {}", path),
            FileError::IoError(msg) => write!(f, "IO error: {}", msg),
            FileError::InvalidPath(path) => write!(f, "Invalid path: {}", path),
            FileError::AlreadyExists(path) => write!(f, "File already exists: {}", path),
            FileError::TooLarge { path, size, max } => {
                write!(f, "File too large: {} ({} bytes, max {})", path, size, max)
            }
        }
    }
}

impl From<io::Error> for FileError {
    fn from(err: io::Error) -> Self {
        FileError::IoError(err.to_string())
    }
}

/// File operation log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOperation {
    pub id: u64,
    pub operation_type: OperationType,
    pub path: String,
    pub timestamp: DateTime<Utc>,
    pub status: OperationStatus,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationType {
    Read,
    Create,
    Update,
    Delete,
    Rename,
    Copy,
    CreateDir,
    List,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationStatus {
    Pending,
    Approved,
    Executed,
    Rejected,
    Failed,
}

/// Pending file change awaiting approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingChange {
    pub id: u64,
    pub change_type: ChangeType,
    pub path: String,
    pub content: Option<String>,
    pub old_content: Option<String>,
    pub new_path: Option<String>, // For rename/copy
    pub description: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    CreateFile,
    UpdateFile,
    DeleteFile,
    RenameFile,
    CreateDirectory,
}

/// File info returned by operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<DateTime<Utc>>,
    pub extension: Option<String>,
}

/// Sandboxed file system for MCP
pub struct McpFileSystem {
    /// Allowed root directories
    roots: Vec<PathBuf>,
    
    /// Maximum file size for reading (10 MB)
    max_read_size: u64,
    
    /// Operation history
    history: Vec<FileOperation>,
    next_op_id: u64,
    
    /// Pending changes
    pending: Vec<PendingChange>,
    next_change_id: u64,
    
    /// File patterns to exclude
    exclude_patterns: Vec<String>,
    
    /// Cache of file contents
    cache: HashMap<String, CachedFile>,
    cache_max_size: usize,
}

#[derive(Debug, Clone)]
struct CachedFile {
    content: String,
    modified: DateTime<Utc>,
}

impl McpFileSystem {
    pub fn new() -> Self {
        McpFileSystem {
            roots: Vec::new(),
            max_read_size: 10 * 1024 * 1024, // 10 MB
            history: Vec::new(),
            next_op_id: 1,
            pending: Vec::new(),
            next_change_id: 1,
            exclude_patterns: vec![
                "node_modules".to_string(),
                ".git".to_string(),
                "target".to_string(),
                "__pycache__".to_string(),
                "*.lock".to_string(),
                ".env".to_string(),
            ],
            cache: HashMap::new(),
            cache_max_size: 100,
        }
    }
    
    /// Add a root directory to the sandbox
    pub fn add_root(&mut self, path: &str) -> FileResult<()> {
        let path = PathBuf::from(path);
        if !path.exists() {
            return Err(FileError::NotFound(path.display().to_string()));
        }
        if !path.is_dir() {
            return Err(FileError::InvalidPath(format!(
                "Not a directory: {}", path.display()
            )));
        }
        
        let canonical = path.canonicalize()?;
        if !self.roots.contains(&canonical) {
            self.roots.push(canonical);
        }
        Ok(())
    }
    
    /// Check if a path is within the sandbox
    fn is_in_sandbox(&self, path: &Path) -> bool {
        if let Ok(canonical) = path.canonicalize() {
            self.roots.iter().any(|root| canonical.starts_with(root))
        } else {
            // Path doesn't exist yet, check parent
            if let Some(parent) = path.parent() {
                if let Ok(canonical) = parent.canonicalize() {
                    return self.roots.iter().any(|root| canonical.starts_with(root));
                }
            }
            false
        }
    }
    
    /// Check if a path should be excluded
    fn should_exclude(&self, path: &Path) -> bool {
        let _path_str = path.to_string_lossy();
        for pattern in &self.exclude_patterns {
            if let Some(ext) = pattern.strip_prefix("*.") {
                // Extension pattern
                if let Some(file_ext) = path.extension() {
                    if file_ext == ext {
                        return true;
                    }
                }
            } else {
                // Directory/file name pattern
                for component in path.components() {
                    if component.as_os_str().to_string_lossy() == *pattern {
                        return true;
                    }
                }
            }
        }
        false
    }
    
    /// Read a file's contents
    pub fn read_file(&mut self, path: &str) -> FileResult<String> {
        let path_buf = PathBuf::from(path);

        // Security check
        if !self.is_in_sandbox(&path_buf) {
            return Err(FileError::OutsideSandbox(path.to_string()));
        }

        // Check if file exists
        if !path_buf.exists() {
            return Err(FileError::NotFound(path.to_string()));
        }

        // Return cached content if fresh
        if let Some(cached) = self.get_cached(path) {
            return Ok(cached.to_string());
        }

        // Check file size
        let metadata = fs::metadata(&path_buf)?;
        if metadata.len() > self.max_read_size {
            return Err(FileError::TooLarge {
                path: path.to_string(),
                size: metadata.len(),
                max: self.max_read_size,
            });
        }

        // Read file
        let content = fs::read_to_string(&path_buf)?;
        
        // Log operation
        self.log_operation(OperationType::Read, path, OperationStatus::Executed, None);
        
        // Update cache
        self.cache_file(path, &content);
        
        Ok(content)
    }
    
    /// Read specific lines from a file
    pub fn read_lines(&mut self, path: &str, start: usize, end: usize) -> FileResult<Vec<String>> {
        let content = self.read_file(path)?;
        let lines: Vec<String> = content.lines()
            .skip(start.saturating_sub(1))
            .take(end.saturating_sub(start.saturating_sub(1)))
            .map(String::from)
            .collect();
        Ok(lines)
    }
    
    /// List directory contents
    pub fn list_dir(&mut self, path: &str) -> FileResult<Vec<FileInfo>> {
        let path_buf = PathBuf::from(path);
        
        if !self.is_in_sandbox(&path_buf) {
            return Err(FileError::OutsideSandbox(path.to_string()));
        }
        
        if !path_buf.is_dir() {
            return Err(FileError::InvalidPath(format!("Not a directory: {}", path)));
        }
        
        let mut entries = Vec::new();
        
        for entry in fs::read_dir(&path_buf)? {
            let entry = entry?;
            let entry_path = entry.path();
            
            // Skip excluded files
            if self.should_exclude(&entry_path) {
                continue;
            }
            
            let metadata = entry.metadata()?;
            
            entries.push(FileInfo {
                path: entry_path.to_string_lossy().to_string(),
                is_dir: metadata.is_dir(),
                size: metadata.len(),
                modified: metadata.modified().ok().map(DateTime::<Utc>::from),
                extension: entry_path.extension().map(|e| e.to_string_lossy().to_string()),
            });
        }
        
        // Sort directories first, then by name
        entries.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.path.cmp(&b.path),
            }
        });
        
        self.log_operation(OperationType::List, path, OperationStatus::Executed, None);
        
        Ok(entries)
    }
    
    /// List directory recursively
    pub fn list_recursive(&mut self, path: &str, max_depth: usize) -> FileResult<Vec<FileInfo>> {
        let mut all_files = Vec::new();
        self.list_recursive_internal(Path::new(path), max_depth, 0, &mut all_files)?;
        Ok(all_files)
    }
    
    fn list_recursive_internal(
        &self,
        path: &Path,
        max_depth: usize,
        current_depth: usize,
        results: &mut Vec<FileInfo>,
    ) -> FileResult<()> {
        if current_depth > max_depth {
            return Ok(());
        }
        
        if !self.is_in_sandbox(path) || self.should_exclude(path) {
            return Ok(());
        }
        
        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let entry_path = entry.path();
                
                if self.should_exclude(&entry_path) {
                    continue;
                }
                
                let metadata = entry.metadata()?;
                
                results.push(FileInfo {
                    path: entry_path.to_string_lossy().to_string(),
                    is_dir: metadata.is_dir(),
                    size: metadata.len(),
                    modified: metadata.modified().ok().map(DateTime::<Utc>::from),
                    extension: entry_path.extension().map(|e| e.to_string_lossy().to_string()),
                });
                
                if metadata.is_dir() {
                    self.list_recursive_internal(&entry_path, max_depth, current_depth + 1, results)?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Queue a file creation (doesn't execute until approved)
    pub fn queue_create(&mut self, path: &str, content: &str, description: &str) -> FileResult<u64> {
        let path_buf = PathBuf::from(path);
        
        if !self.is_in_sandbox(&path_buf) {
            return Err(FileError::OutsideSandbox(path.to_string()));
        }
        
        if path_buf.exists() {
            return Err(FileError::AlreadyExists(path.to_string()));
        }
        
        let id = self.next_change_id;
        self.next_change_id += 1;
        
        self.pending.push(PendingChange {
            id,
            change_type: ChangeType::CreateFile,
            path: path.to_string(),
            content: Some(content.to_string()),
            old_content: None,
            new_path: None,
            description: description.to_string(),
            created_at: Utc::now(),
        });
        
        self.log_operation(OperationType::Create, path, OperationStatus::Pending, Some(description));
        
        Ok(id)
    }
    
    /// Queue a file update
    pub fn queue_update(&mut self, path: &str, content: &str, description: &str) -> FileResult<u64> {
        let path_buf = PathBuf::from(path);
        
        if !self.is_in_sandbox(&path_buf) {
            return Err(FileError::OutsideSandbox(path.to_string()));
        }
        
        if !path_buf.exists() {
            return Err(FileError::NotFound(path.to_string()));
        }
        
        // Read current content for diff
        let old_content = fs::read_to_string(&path_buf)?;
        
        let id = self.next_change_id;
        self.next_change_id += 1;
        
        self.pending.push(PendingChange {
            id,
            change_type: ChangeType::UpdateFile,
            path: path.to_string(),
            content: Some(content.to_string()),
            old_content: Some(old_content),
            new_path: None,
            description: description.to_string(),
            created_at: Utc::now(),
        });
        
        self.log_operation(OperationType::Update, path, OperationStatus::Pending, Some(description));
        
        Ok(id)
    }
    
    /// Queue a file deletion
    pub fn queue_delete(&mut self, path: &str, description: &str) -> FileResult<u64> {
        let path_buf = PathBuf::from(path);
        
        if !self.is_in_sandbox(&path_buf) {
            return Err(FileError::OutsideSandbox(path.to_string()));
        }
        
        if !path_buf.exists() {
            return Err(FileError::NotFound(path.to_string()));
        }
        
        let old_content = fs::read_to_string(&path_buf).ok();
        
        let id = self.next_change_id;
        self.next_change_id += 1;
        
        self.pending.push(PendingChange {
            id,
            change_type: ChangeType::DeleteFile,
            path: path.to_string(),
            content: None,
            old_content,
            new_path: None,
            description: description.to_string(),
            created_at: Utc::now(),
        });
        
        self.log_operation(OperationType::Delete, path, OperationStatus::Pending, Some(description));
        
        Ok(id)
    }
    
    /// Queue a file rename for approval
    pub fn queue_rename(&mut self, path: &str, new_path: &str, description: &str) -> FileResult<u64> {
        let path_buf = PathBuf::from(path);
        let new_path_buf = PathBuf::from(new_path);

        if !self.is_in_sandbox(&path_buf) {
            return Err(FileError::OutsideSandbox(path.to_string()));
        }
        if !self.is_in_sandbox(&new_path_buf) {
            return Err(FileError::OutsideSandbox(new_path.to_string()));
        }
        if !path_buf.exists() {
            return Err(FileError::NotFound(path.to_string()));
        }

        let id = self.next_change_id;
        self.next_change_id += 1;

        self.pending.push(PendingChange {
            id,
            change_type: ChangeType::RenameFile,
            path: path.to_string(),
            content: None,
            old_content: None,
            new_path: Some(new_path.to_string()),
            description: description.to_string(),
            created_at: Utc::now(),
        });

        self.log_operation(OperationType::Rename, path, OperationStatus::Pending, Some(description));

        Ok(id)
    }

    /// Queue a directory creation for approval
    pub fn queue_mkdir(&mut self, path: &str, description: &str) -> FileResult<u64> {
        let path_buf = PathBuf::from(path);

        if !self.is_in_sandbox(&path_buf) {
            return Err(FileError::OutsideSandbox(path.to_string()));
        }

        let id = self.next_change_id;
        self.next_change_id += 1;

        self.pending.push(PendingChange {
            id,
            change_type: ChangeType::CreateDirectory,
            path: path.to_string(),
            content: None,
            old_content: None,
            new_path: None,
            description: description.to_string(),
            created_at: Utc::now(),
        });

        self.log_operation(OperationType::CreateDir, path, OperationStatus::Pending, Some(description));

        Ok(id)
    }

    /// Copy a file (executed immediately, logged)
    pub fn copy_file(&mut self, src: &str, dst: &str) -> FileResult<()> {
        let src_buf = PathBuf::from(src);
        let dst_buf = PathBuf::from(dst);

        if !self.is_in_sandbox(&src_buf) {
            return Err(FileError::OutsideSandbox(src.to_string()));
        }
        if !self.is_in_sandbox(&dst_buf) {
            return Err(FileError::OutsideSandbox(dst.to_string()));
        }
        if !src_buf.exists() {
            return Err(FileError::NotFound(src.to_string()));
        }

        fs::copy(&src_buf, &dst_buf)?;
        self.log_operation(OperationType::Copy, src, OperationStatus::Executed,
            Some(&format!("Copied to {}", dst)));

        Ok(())
    }

    /// Get all pending changes
    pub fn pending_changes(&self) -> &[PendingChange] {
        &self.pending
    }
    
    /// Approve and execute a pending change
    pub fn approve(&mut self, id: u64) -> FileResult<()> {
        let idx = self.pending.iter().position(|c| c.id == id);

        if let Some(idx) = idx {
            let change = self.pending.remove(idx);
            let op_type = match change.change_type {
                ChangeType::CreateFile => OperationType::Create,
                ChangeType::UpdateFile => OperationType::Update,
                ChangeType::DeleteFile => OperationType::Delete,
                ChangeType::RenameFile => OperationType::Rename,
                ChangeType::CreateDirectory => OperationType::CreateDir,
            };
            self.log_operation(op_type, &change.path.clone(), OperationStatus::Approved, None);
            match self.execute_change(&change) {
                Ok(()) => {}
                Err(e) => {
                    self.log_operation(op_type, &change.path, OperationStatus::Failed,
                        Some(&e.to_string()));
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    /// Approve and execute all pending changes
    pub fn approve_all(&mut self) -> FileResult<Vec<u64>> {
        let changes: Vec<PendingChange> = self.pending.drain(..).collect();
        let mut executed = Vec::new();

        for change in changes {
            let op_type = match change.change_type {
                ChangeType::CreateFile => OperationType::Create,
                ChangeType::UpdateFile => OperationType::Update,
                ChangeType::DeleteFile => OperationType::Delete,
                ChangeType::RenameFile => OperationType::Rename,
                ChangeType::CreateDirectory => OperationType::CreateDir,
            };
            self.log_operation(op_type, &change.path.clone(), OperationStatus::Approved, None);
            match self.execute_change(&change) {
                Ok(()) => executed.push(change.id),
                Err(e) => {
                    self.log_operation(op_type, &change.path, OperationStatus::Failed,
                        Some(&e.to_string()));
                    return Err(e);
                }
            }
        }

        Ok(executed)
    }
    
    /// Reject a pending change
    pub fn reject(&mut self, id: u64) -> bool {
        if let Some(idx) = self.pending.iter().position(|c| c.id == id) {
            let change = self.pending.remove(idx);
            self.log_operation(
                match change.change_type {
                    ChangeType::CreateFile => OperationType::Create,
                    ChangeType::UpdateFile => OperationType::Update,
                    ChangeType::DeleteFile => OperationType::Delete,
                    ChangeType::RenameFile => OperationType::Rename,
                    ChangeType::CreateDirectory => OperationType::CreateDir,
                },
                &change.path,
                OperationStatus::Rejected,
                None,
            );
            true
        } else {
            false
        }
    }
    
    /// Reject all pending changes
    pub fn reject_all(&mut self) {
        let changes: Vec<PendingChange> = self.pending.drain(..).collect();
        for change in changes {
            self.log_operation(
                match change.change_type {
                    ChangeType::CreateFile => OperationType::Create,
                    ChangeType::UpdateFile => OperationType::Update,
                    ChangeType::DeleteFile => OperationType::Delete,
                    ChangeType::RenameFile => OperationType::Rename,
                    ChangeType::CreateDirectory => OperationType::CreateDir,
                },
                &change.path,
                OperationStatus::Rejected,
                None,
            );
        }
    }
    
    /// Execute a change
    fn execute_change(&mut self, change: &PendingChange) -> FileResult<()> {
        match change.change_type {
            ChangeType::CreateFile => {
                let path = Path::new(&change.path);
                
                // Create parent directories
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                
                // Write file
                let content = change.content.as_ref().ok_or_else(|| {
                    FileError::InvalidPath("No content for create".to_string())
                })?;
                fs::write(path, content)?;
                
                self.log_operation(OperationType::Create, &change.path, OperationStatus::Executed, None);
            }
            
            ChangeType::UpdateFile => {
                let content = change.content.as_ref().ok_or_else(|| {
                    FileError::InvalidPath("No content for update".to_string())
                })?;
                fs::write(&change.path, content)?;
                
                // Invalidate cache
                self.cache.remove(&change.path);
                
                self.log_operation(OperationType::Update, &change.path, OperationStatus::Executed, None);
            }
            
            ChangeType::DeleteFile => {
                fs::remove_file(&change.path)?;
                self.cache.remove(&change.path);
                
                self.log_operation(OperationType::Delete, &change.path, OperationStatus::Executed, None);
            }
            
            ChangeType::RenameFile => {
                let new_path = change.new_path.as_ref().ok_or_else(|| {
                    FileError::InvalidPath("No new path for rename".to_string())
                })?;
                fs::rename(&change.path, new_path)?;
                self.cache.remove(&change.path);
                
                self.log_operation(OperationType::Rename, &change.path, OperationStatus::Executed, None);
            }
            
            ChangeType::CreateDirectory => {
                fs::create_dir_all(&change.path)?;
                
                self.log_operation(OperationType::CreateDir, &change.path, OperationStatus::Executed, None);
            }
        }
        
        Ok(())
    }
    
    /// Log an operation
    fn log_operation(&mut self, op_type: OperationType, path: &str, status: OperationStatus, details: Option<&str>) {
        self.history.push(FileOperation {
            id: self.next_op_id,
            operation_type: op_type,
            path: path.to_string(),
            timestamp: Utc::now(),
            status,
            details: details.map(String::from),
        });
        self.next_op_id += 1;
        
        // Limit history size
        while self.history.len() > 1000 {
            self.history.remove(0);
        }
    }
    
    /// Cache a file
    fn cache_file(&mut self, path: &str, content: &str) {
        // Evict oldest entries if cache is full
        while self.cache.len() >= self.cache_max_size {
            if let Some(oldest) = self.cache.keys().next().cloned() {
                self.cache.remove(&oldest);
            }
        }
        
        self.cache.insert(path.to_string(), CachedFile {
            content: content.to_string(),
            modified: Utc::now(),
        });
    }
    
    /// Get cached file content if available and fresh
    pub fn get_cached(&self, path: &str) -> Option<&str> {
        self.cache.get(path).and_then(|cached| {
            // Only return cached content if it was cached within the last 5 minutes
            let age = Utc::now() - cached.modified;
            if age.num_seconds() < 300 {
                Some(cached.content.as_str())
            } else {
                None
            }
        })
    }

    /// Get operation history
    pub fn history(&self) -> &[FileOperation] {
        &self.history
    }
    
    /// Search for files by pattern
    pub fn search_files(&mut self, pattern: &str) -> FileResult<Vec<FileInfo>> {
        let mut matches = Vec::new();
        
        for root in &self.roots.clone() {
            self.search_in_dir(root, pattern, &mut matches)?;
        }
        
        Ok(matches)
    }
    
    fn search_in_dir(&self, dir: &Path, pattern: &str, matches: &mut Vec<FileInfo>) -> FileResult<()> {
        if !dir.is_dir() || self.should_exclude(dir) {
            return Ok(());
        }
        
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if self.should_exclude(&path) {
                continue;
            }
            
            let name = path.file_name()
                .map(|n| crate::fontcase::ascii_lower(&n.to_string_lossy()))
                .unwrap_or_default();
            
            if crate::fontcase::ascii_lower(&name).contains(&crate::fontcase::ascii_lower(pattern)) {
                let metadata = entry.metadata()?;
                matches.push(FileInfo {
                    path: path.to_string_lossy().to_string(),
                    is_dir: metadata.is_dir(),
                    size: metadata.len(),
                    modified: metadata.modified().ok().map(DateTime::<Utc>::from),
                    extension: path.extension().map(|e| e.to_string_lossy().to_string()),
                });
            }
            
            if path.is_dir() {
                self.search_in_dir(&path, pattern, matches)?;
            }
        }
        
        Ok(())
    }
    
    /// Search file contents with grep
    pub fn grep(&mut self, pattern: &str, file_pattern: Option<&str>) -> FileResult<Vec<GrepMatch>> {
        let mut matches = Vec::new();
        
        for root in &self.roots.clone() {
            self.grep_in_dir(root, pattern, file_pattern, &mut matches)?;
        }
        
        Ok(matches)
    }
    
    fn grep_in_dir(
        &mut self,
        dir: &Path,
        pattern: &str,
        file_pattern: Option<&str>,
        matches: &mut Vec<GrepMatch>,
    ) -> FileResult<()> {
        if !dir.is_dir() || self.should_exclude(dir) {
            return Ok(());
        }
        
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if self.should_exclude(&path) {
                continue;
            }
            
            if path.is_file() {
                // Check file pattern
                if let Some(fp) = file_pattern {
                    if let Some(ext) = path.extension() {
                        if !ext.to_string_lossy().eq_ignore_ascii_case(fp.trim_start_matches("*.")) {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }
                
                // Read and search file
                if let Ok(content) = self.read_file(&path.to_string_lossy()) {
                    for (line_num, line) in content.lines().enumerate() {
                        if crate::fontcase::ascii_lower(line).contains(&crate::fontcase::ascii_lower(pattern)) {
                            matches.push(GrepMatch {
                                file: path.to_string_lossy().to_string(),
                                line: line_num + 1,
                                content: line.to_string(),
                                column: crate::fontcase::ascii_lower(line).find(&crate::fontcase::ascii_lower(pattern)),
                            });
                        }
                    }
                }
            } else if path.is_dir() {
                self.grep_in_dir(&path, pattern, file_pattern, matches)?;
            }
        }
        
        Ok(())
    }
}

impl Default for McpFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Grep match result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepMatch {
    pub file: String,
    pub line: usize,
    pub content: String,
    pub column: Option<usize>,
}

/// Generate a unified diff between old and new content
pub fn generate_diff(old: &str, new: &str, path: &str) -> String {
    let mut diff = String::new();
    diff.push_str(&format!("--- a/{}\n", path));
    diff.push_str(&format!("+++ b/{}\n", path));
    
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    
    // Simple line-by-line diff
    let mut i = 0;
    let mut j = 0;
    let mut hunk_start = 0;
    let mut hunk_lines = Vec::new();
    
    while i < old_lines.len() || j < new_lines.len() {
        if i < old_lines.len() && j < new_lines.len() && old_lines[i] == new_lines[j] {
            if !hunk_lines.is_empty() {
                hunk_lines.push(format!(" {}", old_lines[i]));
            }
            i += 1;
            j += 1;
        } else if j < new_lines.len() && (i >= old_lines.len() || !old_lines.contains(&new_lines[j])) {
            if hunk_lines.is_empty() {
                hunk_start = i;
            }
            hunk_lines.push(format!("+{}", new_lines[j]));
            j += 1;
        } else if i < old_lines.len() {
            if hunk_lines.is_empty() {
                hunk_start = i;
            }
            hunk_lines.push(format!("-{}", old_lines[i]));
            i += 1;
        }
    }
    
    if !hunk_lines.is_empty() {
        diff.push_str(&format!("@@ -{},{} +{},{} @@\n", 
            hunk_start + 1, old_lines.len(),
            hunk_start + 1, new_lines.len()
        ));
        for line in hunk_lines {
            diff.push_str(&line);
            diff.push('\n');
        }
    }
    
    diff
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_diff_generation() {
        let old = "line 1\nline 2\nline 3";
        let new = "line 1\nmodified line 2\nline 3";
        
        let diff = generate_diff(old, new, "test.txt");
        assert!(diff.contains("-line 2"));
        assert!(diff.contains("+modified line 2"));
    }
    
    #[test]
    fn test_file_system_creation() {
        let fs = McpFileSystem::new();
        assert!(fs.roots.is_empty());
        assert!(fs.pending.is_empty());
    }
}
