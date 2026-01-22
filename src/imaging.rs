//! Image Loading and Decoding
//!
//! Handles loading images from URLs and decoding them into pixel buffers.
//! Supports PNG, JPEG, GIF, and WebP.

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// RGBA pixel buffer
#[derive(Debug, Clone)]
pub struct ImageData {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,  // RGBA format
}

impl ImageData {
    pub fn new(width: u32, height: u32) -> Self {
        let pixels = vec![0u8; (width * height * 4) as usize];
        ImageData { width, height, pixels }
    }
    
    /// Get pixel at (x, y) as RGBA
    pub fn get_pixel(&self, x: u32, y: u32) -> [u8; 4] {
        if x >= self.width || y >= self.height {
            return [0, 0, 0, 0];
        }
        let idx = ((y * self.width + x) * 4) as usize;
        [
            self.pixels[idx],
            self.pixels[idx + 1],
            self.pixels[idx + 2],
            self.pixels[idx + 3],
        ]
    }
    
    /// Set pixel at (x, y)
    pub fn set_pixel(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = ((y * self.width + x) * 4) as usize;
        self.pixels[idx] = rgba[0];
        self.pixels[idx + 1] = rgba[1];
        self.pixels[idx + 2] = rgba[2];
        self.pixels[idx + 3] = rgba[3];
    }
}

/// Image loading state
#[derive(Debug, Clone)]
pub enum ImageState {
    Loading,
    Loaded(ImageData),
    Error(String),
}

/// Image cache for loaded images
pub struct ImageCache {
    cache: HashMap<String, ImageState>,
    /// Maximum cache size in bytes (default 100MB)
    max_size: usize,
    current_size: usize,
}

impl ImageCache {
    pub fn new() -> Self {
        ImageCache {
            cache: HashMap::new(),
            max_size: 100 * 1024 * 1024, // 100MB
            current_size: 0,
        }
    }
    
    pub fn with_max_size(max_size: usize) -> Self {
        ImageCache {
            cache: HashMap::new(),
            max_size,
            current_size: 0,
        }
    }
    
    /// Get an image from cache
    pub fn get(&self, url: &str) -> Option<&ImageState> {
        self.cache.get(url)
    }
    
    /// Insert an image into cache
    pub fn insert(&mut self, url: String, state: ImageState) {
        // Calculate size
        let size = match &state {
            ImageState::Loaded(img) => img.pixels.len(),
            _ => 0,
        };
        
        // Evict if necessary
        while self.current_size + size > self.max_size && !self.cache.is_empty() {
            // Simple FIFO eviction - just remove first entry
            if let Some(key) = self.cache.keys().next().cloned() {
                if let Some(ImageState::Loaded(img)) = self.cache.remove(&key) {
                    self.current_size = self.current_size.saturating_sub(img.pixels.len());
                }
            }
        }
        
        self.current_size += size;
        self.cache.insert(url, state);
    }
    
    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
        self.current_size = 0;
    }
}

impl Default for ImageCache {
    fn default() -> Self {
        Self::new()
    }
}

// Global image cache
lazy_static::lazy_static! {
    static ref GLOBAL_CACHE: Arc<RwLock<ImageCache>> = Arc::new(RwLock::new(ImageCache::new()));
}

/// Load an image from bytes
pub fn decode_image(bytes: &[u8]) -> Result<ImageData, String> {
    use image::GenericImageView;
    
    let img = image::load_from_memory(bytes)
        .map_err(|e| format!("Failed to decode image: {}", e))?;
    
    let (width, height) = img.dimensions();
    let rgba = img.to_rgba8();
    let pixels = rgba.into_raw();
    
    Ok(ImageData { width, height, pixels })
}

/// Load an image from a URL (blocking)
pub fn load_image_blocking(url: &str) -> Result<ImageData, String> {
    // Check cache first
    if let Ok(cache) = GLOBAL_CACHE.read() {
        if let Some(ImageState::Loaded(img)) = cache.get(url) {
            return Ok(img.clone());
        }
        if let Some(ImageState::Error(e)) = cache.get(url) {
            return Err(e.clone());
        }
    }
    
    // Mark as loading
    if let Ok(mut cache) = GLOBAL_CACHE.write() {
        cache.insert(url.to_string(), ImageState::Loading);
    }
    
    // Fetch the image (use http_client to attach User-Agent if configured)
    let response = crate::http_client::get(url)
        .map_err(|e| format!("Failed to fetch image: {}", e))?;
    
    // Read response body
    let mut bytes = Vec::new();
    response.into_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Failed to read image data: {}", e))?;
    
    // Decode image
    let img_data = decode_image(&bytes)?;
    
    // Cache it
    if let Ok(mut cache) = GLOBAL_CACHE.write() {
        cache.insert(url.to_string(), ImageState::Loaded(img_data.clone()));
    }
    
    Ok(img_data)
}

/// Load image from data URL
pub fn load_data_url(data_url: &str) -> Result<ImageData, String> {
    // Format: data:image/png;base64,<data>
    if !data_url.starts_with("data:") {
        return Err("Not a data URL".to_string());
    }
    
    let rest = &data_url[5..];
    let comma_idx = rest.find(',').ok_or("Invalid data URL format")?;
    let meta = &rest[..comma_idx];
    let data = &rest[comma_idx + 1..];
    
    // Check if base64 encoded
    let bytes = if meta.contains(";base64") {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|e| format!("Failed to decode base64: {}", e))?
    } else {
        // URL encoded
        urlencoding::decode(data)
            .map_err(|e| format!("Failed to decode URL encoding: {}", e))?
            .into_owned()
            .into_bytes()
    };
    
    decode_image(&bytes)
}

/// Resize an image (simple nearest-neighbor for now)
pub fn resize_image(img: &ImageData, new_width: u32, new_height: u32) -> ImageData {
    let mut resized = ImageData::new(new_width, new_height);
    
    let x_ratio = img.width as f32 / new_width as f32;
    let y_ratio = img.height as f32 / new_height as f32;
    
    for y in 0..new_height {
        for x in 0..new_width {
            let src_x = (x as f32 * x_ratio) as u32;
            let src_y = (y as f32 * y_ratio) as u32;
            let pixel = img.get_pixel(src_x, src_y);
            resized.set_pixel(x, y, pixel);
        }
    }
    
    resized
}

/// Create a placeholder image (checkerboard pattern)
pub fn placeholder_image(width: u32, height: u32) -> ImageData {
    let mut img = ImageData::new(width, height);
    
    for y in 0..height {
        for x in 0..width {
            let is_light = ((x / 8) + (y / 8)) % 2 == 0;
            let color = if is_light { 200 } else { 180 };
            img.set_pixel(x, y, [color, color, color, 255]);
        }
    }
    
    img
}

/// Create a broken image icon
pub fn broken_image_icon(width: u32, height: u32) -> ImageData {
    let mut img = ImageData::new(width, height);
    
    // Light gray background
    for y in 0..height {
        for x in 0..width {
            img.set_pixel(x, y, [240, 240, 240, 255]);
        }
    }
    
    // Red X
    let margin = (width.min(height) / 4) as i32;
    let w = width as i32;
    let h = height as i32;
    
    for i in 0..width.min(height) as i32 - margin * 2 {
        let x1 = (margin + i) as u32;
        let y1 = (margin + i) as u32;
        let x2 = (w - margin - 1 - i) as u32;
        let y2 = (margin + i) as u32;
        
        if x1 < width && y1 < height {
            img.set_pixel(x1, y1, [200, 50, 50, 255]);
        }
        if x2 < width && y2 < height {
            img.set_pixel(x2, (h - margin - 1 - i) as u32, [200, 50, 50, 255]);
        }
    }
    
    img
}

use std::io::Read;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_placeholder() {
        let img = placeholder_image(64, 64);
        assert_eq!(img.width, 64);
        assert_eq!(img.height, 64);
        assert_eq!(img.pixels.len(), 64 * 64 * 4);
    }
    
    #[test]
    fn test_resize() {
        let img = placeholder_image(100, 100);
        let resized = resize_image(&img, 50, 50);
        assert_eq!(resized.width, 50);
        assert_eq!(resized.height, 50);
    }
}
