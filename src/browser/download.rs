//! Download management - Handle file downloads

use anyhow::Result;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadState {
    Pending,
    Downloading,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct Download {
    pub id: Uuid,
    pub url: String,
    pub filename: String,
    pub save_path: PathBuf,
    pub total_bytes: Option<u64>,
    pub downloaded_bytes: u64,
    pub state: DownloadState,
    pub error: Option<String>,
    pub mime_type: Option<String>,
    pub started_at: std::time::Instant,
    pub completed_at: Option<std::time::Instant>,
}

impl Download {
    pub fn new(url: String, filename: String, save_path: PathBuf) -> Self {
        Self {
            id: Uuid::new_v4(),
            url,
            filename,
            save_path,
            total_bytes: None,
            downloaded_bytes: 0,
            state: DownloadState::Pending,
            error: None,
            mime_type: None,
            started_at: std::time::Instant::now(),
            completed_at: None,
        }
    }
    
    pub fn progress(&self) -> f32 {
        match self.total_bytes {
            Some(total) if total > 0 => self.downloaded_bytes as f32 / total as f32,
            _ => 0.0,
        }
    }
    
    pub fn speed_bps(&self) -> f64 {
        let elapsed = self.started_at.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.downloaded_bytes as f64 / elapsed
        } else {
            0.0
        }
    }
    
    pub fn is_complete(&self) -> bool {
        self.state == DownloadState::Completed
    }
    
    pub fn can_open(&self) -> bool {
        self.state == DownloadState::Completed && self.save_path.exists()
    }
}

pub struct DownloadManager {
    downloads: Arc<Mutex<Vec<Download>>>,
    download_dir: PathBuf,
    auto_open_files: bool,
}

impl DownloadManager {
    pub fn new() -> Self {
        let download_dir = dirs::download_dir()
            .unwrap_or_else(|| PathBuf::from("."));
        
        Self {
            downloads: Arc::new(Mutex::new(Vec::new())),
            download_dir,
            auto_open_files: true,
        }
    }
    
    pub fn set_download_dir(&mut self, path: PathBuf) {
        self.download_dir = path;
    }
    
    pub fn download_dir(&self) -> &PathBuf {
        &self.download_dir
    }
    
    /// Start a new download
    pub fn start_download(&self, url: &str, suggested_filename: Option<&str>) -> Result<Uuid> {
        let filename = suggested_filename
            .map(String::from)
            .or_else(|| Self::filename_from_url(url))
            .unwrap_or_else(|| "download".into());
        
        let save_path = self.generate_unique_path(&filename);
        
        let mut download = Download::new(
            url.to_string(),
            filename,
            save_path.clone(),
        );
        download.state = DownloadState::Downloading;
        
        let id = download.id;
        
        {
            let mut downloads = self.downloads.lock().unwrap();
            downloads.push(download);
        }
        
        // Start async download
        let downloads = Arc::clone(&self.downloads);
        let url = url.to_string();
        
        std::thread::spawn(move || {
            Self::perform_download(downloads, id, &url, &save_path);
        });
        
        Ok(id)
    }
    
    fn perform_download(
        downloads: Arc<Mutex<Vec<Download>>>,
        id: Uuid,
        url: &str,
        save_path: &PathBuf,
    ) {
        let result = (|| -> Result<()> {
            let response = ureq::get(url)
                .set("User-Agent", "SassyBrowser/2.0")
                .call()?;
            
            // Get content length
            let total_bytes = response.header("content-length")
                .and_then(|s| s.parse::<u64>().ok());
            
            // Update total bytes
            {
                let mut downloads = downloads.lock().unwrap();
                if let Some(d) = downloads.iter_mut().find(|d| d.id == id) {
                    d.total_bytes = total_bytes;
                    d.mime_type = response.header("content-type")
                        .map(String::from);
                }
            }
            
            // Create file
            let mut file = std::fs::File::create(save_path)?;
            
            // Get reader from response
            let mut reader = response.into_reader();
            
            // Download in chunks
            let mut buffer = [0u8; 8192];
            loop {
                let bytes_read = std::io::Read::read(&mut reader, &mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                
                std::io::Write::write_all(&mut file, &buffer[..bytes_read])?;
                
                // Update progress
                {
                    let mut downloads = downloads.lock().unwrap();
                    if let Some(d) = downloads.iter_mut().find(|d| d.id == id) {
                        d.downloaded_bytes += bytes_read as u64;
                        
                        // Check for cancellation
                        if d.state == DownloadState::Cancelled {
                            return Ok(());
                        }
                    }
                }
            }
            
            Ok(())
        })();
        
        // Update final state
        let mut downloads = downloads.lock().unwrap();
        if let Some(d) = downloads.iter_mut().find(|d| d.id == id) {
            match result {
                Ok(()) => {
                    if d.state != DownloadState::Cancelled {
                        d.state = DownloadState::Completed;
                        d.completed_at = Some(std::time::Instant::now());
                    }
                }
                Err(e) => {
                    d.state = DownloadState::Failed;
                    d.error = Some(e.to_string());
                }
            }
        }
    }
    
    fn filename_from_url(url: &str) -> Option<String> {
        url::Url::parse(url).ok()
            .and_then(|u| {
                u.path_segments()
                    .and_then(|segments| segments.last())
                    .filter(|s| !s.is_empty())
                    .map(|s| urlencoding::decode(s).unwrap_or_else(|_| s.into()).to_string())
            })
    }
    
    fn generate_unique_path(&self, filename: &str) -> PathBuf {
        let base = self.download_dir.join(filename);
        
        if !base.exists() {
            return base;
        }
        
        let stem = base.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".into());
        let ext = base.extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        
        for i in 1..1000 {
            let new_path = self.download_dir.join(format!("{} ({}){}", stem, i, ext));
            if !new_path.exists() {
                return new_path;
            }
        }
        
        // Fallback with UUID
        self.download_dir.join(format!("{}_{}{}", stem, Uuid::new_v4(), ext))
    }
    
    /// Get all downloads
    pub fn downloads(&self) -> Vec<Download> {
        self.downloads.lock().unwrap().clone()
    }
    
    /// Get a specific download
    pub fn get_download(&self, id: Uuid) -> Option<Download> {
        self.downloads.lock().unwrap()
            .iter()
            .find(|d| d.id == id)
            .cloned()
    }
    
    /// Cancel a download
    pub fn cancel_download(&self, id: Uuid) {
        let mut downloads = self.downloads.lock().unwrap();
        if let Some(d) = downloads.iter_mut().find(|d| d.id == id) {
            if d.state == DownloadState::Downloading {
                d.state = DownloadState::Cancelled;
            }
        }
    }
    
    /// Remove completed/failed downloads from list
    pub fn clear_finished(&self) {
        let mut downloads = self.downloads.lock().unwrap();
        downloads.retain(|d| {
            d.state == DownloadState::Downloading || d.state == DownloadState::Pending
        });
    }
    
    /// Check if there are active downloads
    pub fn has_active_downloads(&self) -> bool {
        self.downloads.lock().unwrap()
            .iter()
            .any(|d| d.state == DownloadState::Downloading)
    }
}

impl Default for DownloadManager {
    fn default() -> Self {
        Self::new()
    }
}
