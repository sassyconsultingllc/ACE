//! MCP Git Integration - Version Control Awareness
//!
//! Provides git repository information to AI agents:
//! - Current branch, status, and history
//! - Staged/unstaged changes
//! - Commit creation (with approval)
//! - Branch management

#![allow(dead_code)]

#[allow(unused_imports)]
use std::collections::HashMap;
#[allow(unused_imports)]
use std::path::{Path, PathBuf};
use std::process::Command;
use chrono::{DateTime, Utc, TimeZone};
use serde::{Deserialize, Serialize};

/// Git operation result
pub type GitResult<T> = Result<T, GitError>;

/// Git errors
#[derive(Debug, Clone)]
pub enum GitError {
    NotARepository(String),
    CommandFailed(String),
    ParseError(String),
    NotConfigured,
    MergeConflict,
    DirtyWorkingTree,
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitError::NotARepository(path) => write!(f, "Not a git repository: {}", path),
            GitError::CommandFailed(msg) => write!(f, "Git command failed: {}", msg),
            GitError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            GitError::NotConfigured => write!(f, "Git not configured"),
            GitError::MergeConflict => write!(f, "Merge conflict detected"),
            GitError::DirtyWorkingTree => write!(f, "Working tree has uncommitted changes"),
        }
    }
}

/// Repository status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoStatus {
    pub path: String,
    pub branch: String,
    pub is_detached: bool,
    pub ahead: u32,
    pub behind: u32,
    pub staged: Vec<FileChange>,
    pub unstaged: Vec<FileChange>,
    pub untracked: Vec<String>,
    pub has_conflicts: bool,
}

/// File change in git
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: String,
    pub status: ChangeStatus,
    pub old_path: Option<String>, // For renames
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Unmerged,
}

impl ChangeStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            ChangeStatus::Added => "+",
            ChangeStatus::Modified => "M",
            ChangeStatus::Deleted => "-",
            ChangeStatus::Renamed => "R",
            ChangeStatus::Copied => "C",
            ChangeStatus::Unmerged => "U",
        }
    }
    
    pub fn color(&self) -> (u8, u8, u8) {
        match self {
            ChangeStatus::Added => (100, 200, 100),
            ChangeStatus::Modified => (200, 200, 100),
            ChangeStatus::Deleted => (200, 100, 100),
            ChangeStatus::Renamed => (100, 150, 200),
            ChangeStatus::Copied => (150, 150, 200),
            ChangeStatus::Unmerged => (255, 100, 100),
        }
    }
}

/// Commit info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub email: String,
    pub date: DateTime<Utc>,
    pub message: String,
    pub parent_hashes: Vec<String>,
}

/// Branch info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
    pub tracking: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub last_commit: Option<String>,
}

/// Pending git operation awaiting approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingGitOp {
    pub id: u64,
    pub operation: GitOperation,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GitOperation {
    Commit { message: String, files: Vec<String> },
    CreateBranch { name: String, from: Option<String> },
    SwitchBranch { name: String },
    Merge { branch: String },
    Stash { message: Option<String> },
    StashPop,
    Reset { mode: ResetMode, target: String },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ResetMode {
    Soft,
    Mixed,
    Hard,
}

/// Git integration for MCP
pub struct McpGit {
    repo_path: Option<PathBuf>,
    pending_ops: Vec<PendingGitOp>,
    next_op_id: u64,
}

impl McpGit {
    pub fn new() -> Self {
        McpGit {
            repo_path: None,
            pending_ops: Vec::new(),
            next_op_id: 1,
        }
    }
    
    /// Set the repository path
    pub fn set_repo(&mut self, path: &str) -> GitResult<()> {
        let path_buf = PathBuf::from(path);
        
        // Check if it's a git repository
        let git_dir = path_buf.join(".git");
        if !git_dir.exists() {
            return Err(GitError::NotARepository(path.to_string()));
        }
        
        self.repo_path = Some(path_buf);
        Ok(())
    }
    
    /// Find git repository from a path (walks up directory tree)
    pub fn find_repo(&mut self, path: &str) -> GitResult<String> {
        let mut current = PathBuf::from(path);
        
        loop {
            let git_dir = current.join(".git");
            if git_dir.exists() {
                let repo_path = current.to_string_lossy().to_string();
                self.repo_path = Some(current);
                return Ok(repo_path);
            }
            
            if !current.pop() {
                return Err(GitError::NotARepository(path.to_string()));
            }
        }
    }
    
    /// Run a git command
    fn git_cmd(&self, args: &[&str]) -> GitResult<String> {
        let repo = self.repo_path.as_ref()
            .ok_or(GitError::NotConfigured)?;
        
        let output = Command::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .map_err(|e| GitError::CommandFailed(e.to_string()))?;
        
        if output.status.success() {
            String::from_utf8(output.stdout)
                .map_err(|e| GitError::ParseError(e.to_string()))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(GitError::CommandFailed(stderr.to_string()))
        }
    }
    
    /// Get repository status
    pub fn status(&self) -> GitResult<RepoStatus> {
        let repo = self.repo_path.as_ref()
            .ok_or(GitError::NotConfigured)?;
        
        // Get current branch
        let branch = self.git_cmd(&["rev-parse", "--abbrev-ref", "HEAD"])?
            .trim()
            .to_string();
        
        let is_detached = branch == "HEAD";
        
        // Get ahead/behind counts
        let (ahead, behind) = self.get_ahead_behind(&branch)?;
        
        // Get status
        let status_output = self.git_cmd(&["status", "--porcelain=v1"])?;
        
        let mut staged = Vec::new();
        let mut unstaged = Vec::new();
        let mut untracked = Vec::new();
        let mut has_conflicts = false;
        
        for line in status_output.lines() {
            if line.len() < 3 {
                continue;
            }
            
            let index_status = line.chars().next().unwrap_or(' ');
            let work_status = line.chars().nth(1).unwrap_or(' ');
            let path = line[3..].to_string();
            
            // Check for conflicts
            if index_status == 'U' || work_status == 'U' {
                has_conflicts = true;
            }
            
            // Untracked files
            if index_status == '?' {
                untracked.push(path);
                continue;
            }
            
            // Staged changes
            if index_status != ' ' && index_status != '?' {
                staged.push(FileChange {
                    path: path.clone(),
                    status: char_to_status(index_status),
                    old_path: None,
                });
            }
            
            // Unstaged changes
            if work_status != ' ' && work_status != '?' {
                unstaged.push(FileChange {
                    path,
                    status: char_to_status(work_status),
                    old_path: None,
                });
            }
        }
        
        Ok(RepoStatus {
            path: repo.to_string_lossy().to_string(),
            branch,
            is_detached,
            ahead,
            behind,
            staged,
            unstaged,
            untracked,
            has_conflicts,
        })
    }
    
    /// Get ahead/behind count for a branch
    fn get_ahead_behind(&self, branch: &str) -> GitResult<(u32, u32)> {
        let result = self.git_cmd(&["rev-list", "--left-right", "--count", &format!("{}...@{{u}}", branch)]);
        
        match result {
            Ok(output) => {
                let parts: Vec<&str> = output.split_whitespace().collect();
                if parts.len() >= 2 {
                    let ahead = parts[0].parse().unwrap_or(0);
                    let behind = parts[1].parse().unwrap_or(0);
                    Ok((ahead, behind))
                } else {
                    Ok((0, 0))
                }
            }
            Err(_) => Ok((0, 0)), // No upstream configured
        }
    }
    
    /// Get commit history
    pub fn log(&self, count: usize) -> GitResult<Vec<Commit>> {
        let output = self.git_cmd(&[
            "log",
            &format!("-{}", count),
            "--format=%H|%h|%an|%ae|%at|%s|%P",
        ])?;
        
        let mut commits = Vec::new();
        
        for line in output.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 6 {
                let timestamp: i64 = parts[4].parse().unwrap_or(0);
                
                commits.push(Commit {
                    hash: parts[0].to_string(),
                    short_hash: parts[1].to_string(),
                    author: parts[2].to_string(),
                    email: parts[3].to_string(),
                    date: Utc.timestamp_opt(timestamp, 0).unwrap(),
                    message: parts[5].to_string(),
                    parent_hashes: parts.get(6)
                        .map(|p| p.split_whitespace().map(String::from).collect())
                        .unwrap_or_default(),
                });
            }
        }
        
        Ok(commits)
    }
    
    /// Get list of branches
    pub fn branches(&self) -> GitResult<Vec<Branch>> {
        let output = self.git_cmd(&[
            "branch",
            "-a",
            "--format=%(refname:short)|%(HEAD)|%(upstream:short)|%(upstream:track)",
        ])?;
        
        let mut branches = Vec::new();
        
        for line in output.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.is_empty() {
                continue;
            }
            
            let name = parts[0].to_string();
            let is_current = parts.get(1).map(|s| *s == "*").unwrap_or(false);
            let is_remote = name.starts_with("remotes/") || name.starts_with("origin/");
            let tracking = parts.get(2).and_then(|s| {
                if s.is_empty() { None } else { Some(s.to_string()) }
            });
            
            // Parse ahead/behind from track info
            let (ahead, behind) = if let Some(track) = parts.get(3) {
                parse_track_info(track)
            } else {
                (0, 0)
            };
            
            branches.push(Branch {
                name,
                is_current,
                is_remote,
                tracking,
                ahead,
                behind,
                last_commit: None,
            });
        }
        
        Ok(branches)
    }
    
    /// Get diff for a file
    pub fn diff_file(&self, path: &str, staged: bool) -> GitResult<String> {
        let args = if staged {
            vec!["diff", "--cached", "--", path]
        } else {
            vec!["diff", "--", path]
        };
        
        self.git_cmd(&args)
    }
    
    /// Get full diff
    pub fn diff(&self, staged: bool) -> GitResult<String> {
        let args = if staged {
            vec!["diff", "--cached"]
        } else {
            vec!["diff"]
        };
        
        self.git_cmd(&args)
    }
    
    /// Stage files
    pub fn stage(&self, paths: &[&str]) -> GitResult<()> {
        let mut args = vec!["add"];
        args.extend(paths);
        self.git_cmd(&args)?;
        Ok(())
    }
    
    /// Unstage files
    pub fn unstage(&self, paths: &[&str]) -> GitResult<()> {
        let mut args = vec!["reset", "HEAD", "--"];
        args.extend(paths);
        self.git_cmd(&args)?;
        Ok(())
    }
    
    /// Queue a commit (requires approval)
    pub fn queue_commit(&mut self, message: &str, files: Option<Vec<String>>) -> u64 {
        let id = self.next_op_id;
        self.next_op_id += 1;
        
        let files = files.unwrap_or_default();
        
        self.pending_ops.push(PendingGitOp {
            id,
            operation: GitOperation::Commit {
                message: message.to_string(),
                files: files.clone(),
            },
            description: format!("Commit: {}", message),
            created_at: Utc::now(),
        });
        
        id
    }
    
    /// Queue a branch creation
    pub fn queue_create_branch(&mut self, name: &str, from: Option<&str>) -> u64 {
        let id = self.next_op_id;
        self.next_op_id += 1;
        
        self.pending_ops.push(PendingGitOp {
            id,
            operation: GitOperation::CreateBranch {
                name: name.to_string(),
                from: from.map(String::from),
            },
            description: format!("Create branch: {}", name),
            created_at: Utc::now(),
        });
        
        id
    }
    
    /// Get pending operations
    pub fn pending_operations(&self) -> &[PendingGitOp] {
        &self.pending_ops
    }
    
    /// Approve and execute a pending operation
    pub fn approve(&mut self, id: u64) -> GitResult<()> {
        let idx = self.pending_ops.iter().position(|op| op.id == id);
        
        if let Some(idx) = idx {
            let op = self.pending_ops.remove(idx);
            self.execute_operation(&op.operation)?;
        }
        
        Ok(())
    }
    
    /// Execute a git operation
    fn execute_operation(&self, op: &GitOperation) -> GitResult<()> {
        match op {
            GitOperation::Commit { message, files } => {
                if !files.is_empty() {
                    let file_refs: Vec<&str> = files.iter().map(|s| s.as_str()).collect();
                    self.stage(&file_refs)?;
                }
                self.git_cmd(&["commit", "-m", message])?;
            }
            
            GitOperation::CreateBranch { name, from } => {
                if let Some(base) = from {
                    self.git_cmd(&["branch", name, base])?;
                } else {
                    self.git_cmd(&["branch", name])?;
                }
            }
            
            GitOperation::SwitchBranch { name } => {
                self.git_cmd(&["checkout", name])?;
            }
            
            GitOperation::Merge { branch } => {
                self.git_cmd(&["merge", branch])?;
            }
            
            GitOperation::Stash { message } => {
                if let Some(msg) = message {
                    self.git_cmd(&["stash", "push", "-m", msg])?;
                } else {
                    self.git_cmd(&["stash"])?;
                }
            }
            
            GitOperation::StashPop => {
                self.git_cmd(&["stash", "pop"])?;
            }
            
            GitOperation::Reset { mode, target } => {
                let mode_arg = match mode {
                    ResetMode::Soft => "--soft",
                    ResetMode::Mixed => "--mixed",
                    ResetMode::Hard => "--hard",
                };
                self.git_cmd(&["reset", mode_arg, target])?;
            }
        }
        
        Ok(())
    }
    
    /// Get file blame
    pub fn blame(&self, path: &str) -> GitResult<Vec<BlameLine>> {
        let output = self.git_cmd(&["blame", "--line-porcelain", path])?;
        
        let mut lines = Vec::new();
        let mut current_commit = String::new();
        let mut current_author = String::new();
        let mut current_time: i64 = 0;
        
        for line in output.lines() {
            if line.len() == 40 && line.chars().all(|c| c.is_ascii_hexdigit()) {
                current_commit = line[..8].to_string();
            } else if let Some(author) = line.strip_prefix("author ") {
                current_author = author.to_string();
            } else if let Some(time) = line.strip_prefix("author-time ") {
                current_time = time.parse().unwrap_or(0);
            } else if let Some(content) = line.strip_prefix('\t') {
                lines.push(BlameLine {
                    commit: current_commit.clone(),
                    author: current_author.clone(),
                    date: Utc.timestamp_opt(current_time, 0).unwrap(),
                    content: content.to_string(),
                });
            }
        }
        
        Ok(lines)
    }
    
    /// Show a specific commit
    pub fn show_commit(&self, hash: &str) -> GitResult<CommitDetails> {
        let output = self.git_cmd(&["show", "--stat", "--format=fuller", hash])?;
        
        let mut details = CommitDetails {
            hash: hash.to_string(),
            author: String::new(),
            author_email: String::new(),
            committer: String::new(),
            committer_email: String::new(),
            date: Utc::now(),
            message: String::new(),
            files_changed: 0,
            insertions: 0,
            deletions: 0,
            changed_files: Vec::new(),
        };
        
        let mut _in_message = false;
        let mut in_stats = false;
        
        for line in output.lines() {
            if let Some(author) = line.strip_prefix("Author: ") {
                if let Some((name, email)) = parse_author(author) {
                    details.author = name;
                    details.author_email = email;
                }
            } else if let Some(committer) = line.strip_prefix("Commit: ") {
                if let Some((name, email)) = parse_author(committer) {
                    details.committer = name;
                    details.committer_email = email;
                }
            } else if line.starts_with("    ") && !in_stats {
                _in_message = true;
                if !details.message.is_empty() {
                    details.message.push('\n');
                }
                details.message.push_str(line.trim());
            } else if line.contains(" | ") {
                in_stats = true;
                let parts: Vec<&str> = line.split(" | ").collect();
                if !parts.is_empty() {
                    details.changed_files.push(parts[0].trim().to_string());
                }
            } else if line.contains("files changed") || line.contains("file changed") {
                // Parse summary line
                for word in line.split_whitespace() {
                    if let Ok(n) = word.parse::<u32>() {
                        if details.files_changed == 0 {
                            details.files_changed = n;
                        } else if line.contains("insertions") && details.insertions == 0 {
                            details.insertions = n;
                        } else if line.contains("deletions") {
                            details.deletions = n;
                        }
                    }
                }
            }
        }
        
        Ok(details)
    }
}

impl Default for McpGit {
    fn default() -> Self {
        Self::new()
    }
}

/// Line from git blame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameLine {
    pub commit: String,
    pub author: String,
    pub date: DateTime<Utc>,
    pub content: String,
}

/// Detailed commit information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitDetails {
    pub hash: String,
    pub author: String,
    pub author_email: String,
    pub committer: String,
    pub committer_email: String,
    pub date: DateTime<Utc>,
    pub message: String,
    pub files_changed: u32,
    pub insertions: u32,
    pub deletions: u32,
    pub changed_files: Vec<String>,
}

// Helper functions

fn char_to_status(c: char) -> ChangeStatus {
    match c {
        'A' => ChangeStatus::Added,
        'M' => ChangeStatus::Modified,
        'D' => ChangeStatus::Deleted,
        'R' => ChangeStatus::Renamed,
        'C' => ChangeStatus::Copied,
        'U' => ChangeStatus::Unmerged,
        _ => ChangeStatus::Modified,
    }
}

fn parse_track_info(track: &str) -> (u32, u32) {
    let mut ahead = 0;
    let mut behind = 0;
    
    if track.contains("ahead") {
        if let Some(n) = track.split("ahead ").nth(1) {
            ahead = n.split(']').next()
                .and_then(|s| s.split(',').next())
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
        }
    }
    
    if track.contains("behind") {
        if let Some(n) = track.split("behind ").nth(1) {
            behind = n.split(']').next()
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
        }
    }
    
    (ahead, behind)
}

fn parse_author(s: &str) -> Option<(String, String)> {
    // Format: "Name <email>"
    if let Some(email_start) = s.find('<') {
        let name = s[..email_start].trim().to_string();
        let email = s[email_start+1..].trim_end_matches('>').to_string();
        Some((name, email))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_status_parsing() {
        let status = char_to_status('M');
        assert_eq!(status, ChangeStatus::Modified);
        
        let status = char_to_status('A');
        assert_eq!(status, ChangeStatus::Added);
    }
    
    #[test]
    fn test_track_info_parsing() {
        let (ahead, behind) = parse_track_info("[ahead 3, behind 2]");
        assert_eq!(ahead, 3);
        assert_eq!(behind, 2);
    }
    
    #[test]
    fn test_author_parsing() {
        let (name, email) = parse_author("John Doe <john@example.com>").unwrap();
        assert_eq!(name, "John Doe");
        assert_eq!(email, "john@example.com");
    }
}
