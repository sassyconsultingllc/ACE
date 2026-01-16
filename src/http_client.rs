use std::env;

/// Build a User-Agent string from environment variables:
/// - SASSY_UA_PRESET: chrome|safari|edge|opera|firefox|sassy
/// - SASSY_DEVICE: desktop|mobile|iphone|pixel
pub fn build_user_agent() -> Option<String> {
    let preset = env::var("SASSY_UA_PRESET").ok();
    let device = env::var("SASSY_DEVICE").ok();

    let preset = preset.unwrap_or_else(|| "sassy".into()).to_lowercase();
    let device = device.unwrap_or_else(|| "desktop".into()).to_lowercase();

    let ua = match preset.as_str() {
        "chrome" => {
            match device.as_str() {
                "mobile" | "pixel" => "Mozilla/5.0 (Linux; Android 13; Pixel 6) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/117.0.0.0 Mobile Safari/537.36".to_string(),
                _ => "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/117.0.0.0 Safari/537.36".to_string(),
            }
        }
        "safari" => {
            match device.as_str() {
                "mobile" | "iphone" => "Mozilla/5.0 (iPhone; CPU iPhone OS 16_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.4 Mobile/15E148 Safari/604.1".to_string(),
                _ => "Mozilla/5.0 (Macintosh; Intel Mac OS X 13_4) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.4 Safari/605.1.15".to_string(),
            }
        }
        "edge" => {
            match device.as_str() {
                "mobile" => "Mozilla/5.0 (Linux; Android 13; Pixel 6) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/117.0.0.0 Mobile Safari/537.36 Edg/117.0.2045.60".to_string(),
                _ => "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/117.0.0.0 Safari/537.36 Edg/117.0.2045.60".to_string(),
            }
        }
        "opera" => {
            match device.as_str() {
                "mobile" => "Mozilla/5.0 (Linux; Android 13; Pixel 6) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/117.0.0.0 Mobile Safari/537.36 OPR/103.0.0.0".to_string(),
                _ => "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/117.0.0.0 Safari/537.36 OPR/103.0.0.0".to_string(),
            }
        }
        "firefox" => {
            match device.as_str() {
                "mobile" => "Mozilla/5.0 (Android 13; Mobile; rv:117.0) Gecko/20100101 Firefox/117.0".to_string(),
                _ => "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:117.0) Gecko/20100101 Firefox/117.0".to_string(),
            }
        }
        _ => {
            // Sassy default UA - subtle, less fingerprintable
            match device.as_str() {
                "mobile" | "iphone" | "pixel" => "Mozilla/5.0 (Mobile; rv:1.0) Gecko/20100101 SassyBrowser/2.0".to_string(),
                _ => "Mozilla/5.0 (X11; Linux x86_64) Gecko/20100101 SassyBrowser/2.0".to_string(),
            }
        }
    };

    Some(ua)
}

/// Perform a GET request using `ureq`, attaching the configured User-Agent if present.
pub fn get(url: &str) -> Result<ureq::Response, ureq::Error> {
    let req = ureq::get(url);
    if let Some(ua) = build_user_agent() {
        req.set("User-Agent", &ua).call()
    } else {
        req.call()
    }
}

/// Convenience: fetch URL and return body string
pub fn fetch_text(url: &str) -> Result<String, String> {
    match get(url) {
        Ok(resp) => resp.into_string().map_err(|e| format!("Failed to read response: {}", e)),
        Err(e) => Err(format!("Request failed: {}", e)),
    }
}
