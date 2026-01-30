//! Sample Rust Code - Sassy Browser Demo
//! This demonstrates syntax highlighting capabilities

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct FileViewer {
    supported_formats: Vec<String>,
    cache: HashMap<String, Vec<u8>>,
}

impl FileViewer {
    pub fn new() -> Self {
        Self {
            supported_formats: vec![
                "pdf", "docx", "xlsx", "pptx",
                "png", "jpg", "svg", "webp",
                "pdb", "mol", "fasta", "sdf",
            ].into_iter().map(String::from).collect(),
            cache: HashMap::new(),
        }
    }

    pub fn can_open(&self, extension: &str) -> bool {
        self.supported_formats.contains(&extension.to_lowercase())
    }

    pub fn open_file(&mut self, path: &str) -> Result<(), String> {
        let data = std::fs::read(path)
            .map_err(|e| format!("Failed to read: {}", e))?;
        self.cache.insert(path.to_string(), data);
        Ok(())
    }
}

fn main() {
    let viewer = FileViewer::new();
    println!("Sassy Browser supports {} formats", viewer.supported_formats.len());
}
