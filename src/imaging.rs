//! Image Loading and Decoding
//!
//! Handles loading images from URLs and decoding them into pixel buffers.
//! Supports PNG, JPEG, GIF, and WebP.

use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;

/// RGBA pixel buffer
#[derive(Debug, Clone)]
pub struct ImageData {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGBA format
}

impl ImageData {
    /// Summary of image dimensions and data size
    pub fn describe(&self) -> String {
        format!(
            "ImageData[{}x{}, {} bytes]",
            self.width,
            self.height,
            self.pixels.len()
        )
    }

    pub fn new(width: u32, height: u32) -> Self {
        let pixels = vec![0u8; (width * height * 4) as usize];
        ImageData {
            width,
            height,
            pixels,
        }
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
    /// FIFO order of keys for eviction (simple policy)
    order: VecDeque<String>,
}

impl ImageCache {
    pub fn new() -> Self {
        ImageCache {
            cache: HashMap::new(),
            max_size: 100 * 1024 * 1024, // 100MB
            current_size: 0,
            order: VecDeque::new(),
        }
    }

    pub fn with_max_size(max_size: usize) -> Self {
        ImageCache {
            cache: HashMap::new(),
            max_size,
            current_size: 0,
            order: VecDeque::new(),
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
        // If key already present, remove prior size and its position
        if let Some(prev) = self.cache.remove(&url) {
            if let ImageState::Loaded(img) = prev {
                self.current_size = self.current_size.saturating_sub(img.pixels.len());
            }
            // remove from order deque
            if let Some(pos) = self.order.iter().position(|k| k == &url) {
                self.order.remove(pos);
            }
        }

        // Evict if necessary (FIFO)
        while self.current_size + size > self.max_size && !self.order.is_empty() {
            if let Some(key) = self.order.pop_front() {
                if let Some(ImageState::Loaded(img)) = self.cache.remove(&key) {
                    self.current_size = self.current_size.saturating_sub(img.pixels.len());
                }
            }
        }

        self.current_size += size;
        self.cache.insert(url.clone(), state);
        self.order.push_back(url);
    }

    /// Summary of cache state
    pub fn describe(&self) -> String {
        format!(
            "ImageCache[entries={}, size={}/{} bytes, eviction_queue={}]",
            self.cache.len(),
            self.current_size,
            self.max_size,
            self.order.len()
        )
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
        self.current_size = 0;
        self.order.clear();
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
    // Background prioritized loader channel
    static ref IMAGE_REQUEST_TX: Mutex<mpsc::Sender<(String, usize)>> = {
        let (tx, rx) = mpsc::channel::<(String, usize)>();

        // Read worker count from env or default
        let worker_count = std::env::var("SASSY_IMAGE_WORKERS").ok().and_then(|s| s.parse().ok()).unwrap_or(2usize);
        let max_retries = std::env::var("SASSY_IMAGE_MAX_RETRIES").ok().and_then(|s| s.parse().ok()).unwrap_or(3usize);
        let cache_dir = std::env::var("SASSY_IMAGE_CACHE_DIR").unwrap_or_else(|_| "target/image_cache".into());
        let cache_dir_path = PathBuf::from(cache_dir);
        let _ = fs::create_dir_all(&cache_dir_path);

        // Spawn dispatcher thread which maintains a priority heap and spawns workers
        thread::spawn(move || {
            let mut heap: BinaryHeap<(usize, usize, String)> = BinaryHeap::new();
            static SEQ: AtomicUsize = AtomicUsize::new(0);
            let active = std::sync::Arc::new(AtomicUsize::new(0));

            loop {
                // Collect new requests
                match rx.recv_timeout(Duration::from_millis(200)) {
                    Ok((url, priority)) => {
                        let seq = SEQ.fetch_add(1, Ordering::SeqCst);
                        // Use (priority, reverse_seq, url) - BinaryHeap is max-heap
                        heap.push((priority, usize::MAX - seq, url));
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(_) => break,
                }

                // While we have capacity and tasks, spawn workers
                while active.load(Ordering::SeqCst) < worker_count {
                    if let Some((_priority, _rev_seq, url)) = heap.pop() {
                        let cache_dir = cache_dir_path.clone();
                        let active_ref = active.clone();
                        active_ref.fetch_add(1, Ordering::SeqCst);

                        // Spawn worker thread
                        let url_clone = url.clone();
                        let active_thread = active_ref.clone();
                        thread::spawn(move || {
                            // Worker: try disk cache, otherwise network with retries
                            use base64::Engine;
                            let fname = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&url_clone);
                            let fpath = cache_dir.join(fname);

                            // If exists on disk, try read and decode
                            if fpath.exists() {
                                if let Ok(bytes) = fs::read(&fpath) {
                                    if let Ok(img) = decode_image(&bytes) {
                                        cache_insert_global(&url_clone, ImageState::Loaded(img));
                                        notify_image_update(&url_clone);
                                        active_ref.fetch_sub(1, Ordering::SeqCst);
                                        return;
                                    }
                                }
                            }

                            // Try network with retries/backoff
                            let mut attempt = 0usize;
                            let mut succeeded = false;
                            while attempt < max_retries {
                                attempt += 1;
                                match crate::http_client::get(&url_clone) {
                                    Ok(resp) => {
                                        let mut bytes = Vec::new();
                                        if resp.into_reader().read_to_end(&mut bytes).is_ok() {
                                            // Save to disk (best-effort)
                                            let _ = fs::create_dir_all(&cache_dir);
                                            let _ = fs::write(&fpath, &bytes);
                                                if let Ok(img) = decode_image(&bytes) {
                                                    cache_insert_global(&url_clone, ImageState::Loaded(img));
                                                    notify_image_update(&url_clone);
                                                } else {
                                                    cache_insert_global(&url_clone, ImageState::Error("Failed to decode image".into()));
                                                    notify_image_update(&url_clone);
                                                }
                                            succeeded = true;
                                            break;
                                        }
                                    }
                                    Err(_) => {
                                        // fallthrough to backoff below
                                    }
                                }

                                // exponential backoff
                                let backoff = 50u64.saturating_mul(2u64.pow((attempt - 1) as u32));
                                thread::sleep(Duration::from_millis(backoff));
                            }

                            if !succeeded {
                                cache_insert_global(&url_clone, ImageState::Error("Failed to fetch image after retries".into()));
                                notify_image_update(&url_clone);
                            }

                            active_thread.fetch_sub(1, Ordering::SeqCst);
                        });
                    } else {
                        break;
                    }
                }
            }
        });

        Mutex::new(tx)
    };
    // Optional sender for notifying UI about image updates (url string)
    static ref IMAGE_UPDATE_TX: Mutex<Option<mpsc::Sender<String>>> = Mutex::new(None);
    // Bounded queue of pending image URLs updated by workers; UI will drain this to refresh textures
    static ref IMAGE_UPDATE_QUEUE: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());
}

/// Register a global sender to receive image-update notifications (url strings).
/// UI code can create an `mpsc::channel()` and pass the sender here to receive events
/// whenever a background worker inserts a final ImageState for a url.
pub fn register_image_update_sender(tx: mpsc::Sender<String>) {
    if let Ok(mut guard) = IMAGE_UPDATE_TX.lock() {
        *guard = Some(tx);
    }
}

fn notify_image_update(url: &str) {
    if let Ok(guard) = IMAGE_UPDATE_TX.lock() {
        if let Some(ref sender) = *guard {
            let _ = sender.send(url.to_string());
        }
    }
}

/// Push an update URL onto the internal queue for UI to drain.
pub fn push_image_update_to_queue(url: &str) {
    // Bounded capacity (env or default)
    let cap = std::env::var("SASSY_IMAGE_UPDATE_QUEUE_CAP")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1024usize);
    if let Ok(mut q) = IMAGE_UPDATE_QUEUE.lock() {
        // If already present as newest, skip duplicate to reduce noise
        if q.back().map(|b| b == url).unwrap_or(false) {
            return;
        }
        // If queue already contains the url earlier, remove it to reinsert as newest
        if let Some(pos) = q.iter().position(|v| v == url) {
            q.remove(pos);
        }

        if q.len() >= cap {
            let _ = q.pop_front();
        }
        q.push_back(url.to_string());
    }
}

/// Drain and return pending update URLs in FIFO order.
pub fn drain_image_update_queue() -> Vec<String> {
    if let Ok(mut q) = IMAGE_UPDATE_QUEUE.lock() {
        let mut out = Vec::with_capacity(q.len());
        while let Some(v) = q.pop_front() {
            out.push(v);
        }
        out
    } else {
        Vec::new()
    }
}

/// Access the global image cache
pub fn global_image_cache() -> Arc<RwLock<ImageCache>> {
    GLOBAL_CACHE.clone()
}

/// Convenience: insert into global cache
pub fn cache_insert_global(url: &str, state: ImageState) {
    if let Ok(mut g) = GLOBAL_CACHE.write() {
        g.insert(url.to_string(), state);
    }
}

/// Convenience: get from global cache (cloned)
pub fn cache_get_global(url: &str) -> Option<ImageState> {
    if let Ok(g) = GLOBAL_CACHE.read() {
        return g.get(url).cloned();
    }
    None
}

/// Clear the global cache
pub fn cache_clear_global() {
    if let Ok(mut g) = GLOBAL_CACHE.write() {
        g.clear();
    }
}

/// Enqueue a prioritized image load. Higher `priority` values are handled first.
pub fn enqueue_image_request(url: &str, priority: usize) {
    if let Ok(tx_mutex) = IMAGE_REQUEST_TX.lock() {
        let _ = tx_mutex.send((url.to_string(), priority));
    }
}

/// Load an image from bytes
pub fn decode_image(bytes: &[u8]) -> Result<ImageData, String> {
    use image::GenericImageView;

    let img =
        image::load_from_memory(bytes).map_err(|e| format!("Failed to decode image: {}", e))?;

    let (width, height) = img.dimensions();
    let rgba = img.to_rgba8();
    let pixels = rgba.into_raw();

    Ok(ImageData {
        width,
        height,
        pixels,
    })
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
    let response =
        crate::http_client::get(url).map_err(|e| format!("Failed to fetch image: {}", e))?;

    // Read response body
    let mut bytes = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Failed to read image data: {}", e))?;

    // Decode image
    let img_data = decode_image(&bytes)?;

    // Cache it
    if let Ok(mut cache) = GLOBAL_CACHE.write() {
        cache.insert(url.to_string(), ImageState::Loaded(img_data.clone()));
        notify_image_update(url);
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

/// Request an image to be loaded in the background and invoke `callback` with the final state.
/// The callback is called from a spawned thread; it must be `Send + 'static`.
pub fn request_image(url: &str, callback: Box<dyn Fn(ImageState) + Send + 'static>) {
    // Keep legacy behavior: spawn a dedicated thread and call callback when done.
    let url_s = url.to_string();
    let cb = callback;
    thread::spawn(move || {
        let state = match load_image_blocking(&url_s) {
            Ok(img) => ImageState::Loaded(img),
            Err(e) => ImageState::Error(e),
        };
        cache_insert_global(&url_s, state.clone());
        notify_image_update(&url_s);
        cb(state);
    });
}

/// Trigger a background load for the given `url` and return immediately with a Loading state.
pub fn load_image_background(url: &str) -> ImageState {
    // If already cached, return current state
    if let Some(s) = cache_get_global(url) {
        return s;
    }

    // Mark as loading and enqueue a background fetch with normal priority (0)
    cache_insert_global(url, ImageState::Loading);
    enqueue_image_request(url, 0);
    ImageState::Loading
}

/// Load or return a broken-image placeholder for a failed URL.
pub fn load_or_broken(url: &str, w: u32, h: u32) -> ImageData {
    match load_image_blocking(url) {
        Ok(img) => resize_image(&img, w, h),
        Err(_) => broken_image_icon(w, h),
    }
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

    #[test]
    fn test_data_url_and_cache() {
        // 1x1 GIF transparent pixel (known-good tiny GIF)
        let data_url = "data:image/gif;base64,R0lGODdhAQABAIAAAAAAAP///ywAAAAAAQABAAACAUwAOw==";
        let img = load_data_url(data_url).expect("data url decode");
        assert_eq!(img.width, 1);
        assert_eq!(img.height, 1);

        // Test global cache insert/get/clear
        cache_clear_global();
        assert!(cache_get_global("/missing").is_none());

        let key = "test://img1";
        cache_insert_global(key, ImageState::Loaded(img.clone()));
        let got = cache_get_global(key);
        assert!(got.is_some());
        match got.unwrap() {
            ImageState::Loaded(l) => assert_eq!(l.width, 1),
            _ => panic!("Expected loaded image"),
        }

        cache_clear_global();
        assert!(cache_get_global(key).is_none());
    }

    #[test]
    fn test_with_max_size_and_global_cache_helpers() {
        // Ensure with_max_size constructor exists and behaves
        let c = ImageCache::with_max_size(1024);
        assert_eq!(c.max_size, 1024);

        // Global cache helpers
        cache_clear_global();
        assert!(cache_get_global("/nope").is_none());

        let dummy = ImageData::new(2, 2);
        cache_insert_global("test://dummy", ImageState::Loaded(dummy.clone()));
        if let Some(ImageState::Loaded(img)) = cache_get_global("test://dummy") {
            assert_eq!(img.width, 2);
        } else {
            panic!("expected loaded image in global cache");
        }
        cache_clear_global();
    }

    #[test]
    fn test_broken_icon_and_request_image_error_callback() {
        // Broken icon should produce a valid image buffer
        let bi = broken_image_icon(16, 16);
        assert_eq!(bi.width, 16);
        assert_eq!(bi.height, 16);
        assert_eq!(bi.pixels.len(), 16 * 16 * 4);

        // request_image should call callback even on unreachable URL (error path)
        let (tx, rx) = std::sync::mpsc::channel::<ImageState>();
        request_image(
            "http://127.0.0.1/nonexistent",
            Box::new(move |st| {
                let _ = tx.send(st);
            }),
        );

        // Wait briefly for the spawned thread to invoke callback
        let got = rx.recv_timeout(std::time::Duration::from_secs(2));
        assert!(got.is_ok());
        match got.unwrap() {
            ImageState::Error(_) => {}
            ImageState::Loaded(_) => {}
            ImageState::Loading => panic!("unexpected Loading state in callback"),
        }
    }

    #[test]
    fn test_image_data_describe_and_pixel_ops() {
        let mut img = ImageData::new(4, 4);
        // Exercise describe
        let desc = img.describe();
        assert!(desc.contains("ImageData"));
        assert!(desc.contains("4x4"));

        // Exercise get_pixel / set_pixel
        img.set_pixel(0, 0, [255, 128, 64, 255]);
        let px = img.get_pixel(0, 0);
        assert_eq!(px, [255, 128, 64, 255]);

        // Out-of-bounds returns transparent
        let oob = img.get_pixel(100, 100);
        assert_eq!(oob, [0, 0, 0, 0]);

        // Out-of-bounds set is no-op
        img.set_pixel(100, 100, [1, 2, 3, 4]);
    }

    #[test]
    fn test_image_cache_describe_and_eviction() {
        let mut cache = ImageCache::with_max_size(256);
        let desc = cache.describe();
        assert!(desc.contains("ImageCache"));
        assert!(desc.contains("256"));

        // Insert a small image
        let img = ImageData::new(2, 2);
        cache.insert("test://a".to_string(), ImageState::Loaded(img));
        assert!(cache.get("test://a").is_some());

        // Insert an error state
        cache.insert("test://err".to_string(), ImageState::Error("fail".into()));
        assert!(cache.get("test://err").is_some());

        // Insert a loading state
        cache.insert("test://loading".to_string(), ImageState::Loading);
        assert!(cache.get("test://loading").is_some());

        // Clear
        cache.clear();
        assert!(cache.get("test://a").is_none());
    }

    #[test]
    fn test_image_state_variants() {
        // Construct all ImageState variants
        let loading = ImageState::Loading;
        let loaded = ImageState::Loaded(ImageData::new(1, 1));
        let error = ImageState::Error("test error".to_string());

        // Clone them to exercise the Clone trait
        let _l2 = loading.clone();
        let _l3 = loaded.clone();
        let _e2 = error.clone();

        // Debug format
        let _dbg = format!("{:?}", loading);
    }

    #[test]
    fn test_load_image_background_and_enqueue() {
        cache_clear_global();
        // load_image_background for a non-cached URL should return Loading
        let state = load_image_background("test://bg_test");
        match state {
            ImageState::Loading => {}
            _ => panic!("Expected Loading state from load_image_background"),
        }

        // Enqueue with priority
        enqueue_image_request("test://priority_test", 10);
    }

    #[test]
    fn test_push_and_drain_image_update_queue() {
        // Drain first to clear any residual
        let _ = drain_image_update_queue();

        push_image_update_to_queue("test://url1");
        push_image_update_to_queue("test://url2");
        // Duplicate of newest should be skipped
        push_image_update_to_queue("test://url2");

        let drained = drain_image_update_queue();
        assert!(drained.contains(&"test://url1".to_string()));
        assert!(drained.contains(&"test://url2".to_string()));

        // Queue should be empty after drain
        let empty = drain_image_update_queue();
        assert!(empty.is_empty());
    }

    #[test]
    fn test_register_image_update_sender() {
        let (tx, _rx) = std::sync::mpsc::channel::<String>();
        register_image_update_sender(tx);
    }

    #[test]
    fn test_global_image_cache_arc() {
        let cache = global_image_cache();
        let _guard = cache.read().unwrap();
    }

    #[test]
    fn test_load_data_url_non_base64() {
        // URL-encoded data URL (non-base64)
        let result = load_data_url("data:text/plain,hello%20world");
        // This will fail to decode as image, which is expected
        assert!(result.is_err());
    }

    #[test]
    fn test_load_data_url_invalid() {
        let result = load_data_url("not-a-data-url");
        assert!(result.is_err());

        let result2 = load_data_url("data:no-comma");
        assert!(result2.is_err());
    }

    #[test]
    fn test_load_or_broken_with_invalid_url() {
        // load_or_broken should return a valid placeholder for unreachable URLs
        let img = load_or_broken("http://127.0.0.1:1/nonexistent.png", 16, 16);
        assert_eq!(img.width, 16);
        assert_eq!(img.height, 16);
        assert_eq!(img.pixels.len(), 16 * 16 * 4);
    }
}
