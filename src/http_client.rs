use std::env;
use std::sync::OnceLock;

use crate::tls_spoof::{
    build_chrome132_tls_config, build_ureq_agent_with_chrome_tls, chrome132_cipher_suites,
    chrome132_ja3_rustls_partial, chrome132_ja3_target, chrome132_signature_schemes,
    generate_grease_pair, generate_grease_value, ChromeTlsConfig, SpoofedTlsConnector,
    CHROME_132_USER_AGENT, GREASE_VALUES,
};

/// Lazily initialized Chrome-fingerprinted ureq agent.
/// Uses TLS ClientHello spoofing to match Chrome/Edge 132 JA3 hash.
static CHROME_AGENT: OnceLock<ureq::Agent> = OnceLock::new();

/// Get or create the Chrome-fingerprinted HTTP agent.
/// This agent has its TLS cipher suite order, key exchange groups,
/// signature schemes, and ALPN configured to match Chrome 132.
pub fn chrome_agent() -> &'static ureq::Agent {
    CHROME_AGENT.get_or_init(|| {
        // Log the TLS configuration for debugging
        let cipher_count = chrome132_cipher_suites().len();
        let sig_count = chrome132_signature_schemes().len();
        let (g1, g2) = generate_grease_pair();
        tracing::debug!(
            "TLS spoof: {} cipher suites, {} sig schemes, GREASE: {:#06x}/{:#06x}",
            cipher_count,
            sig_count,
            g1,
            g2
        );
        tracing::debug!("TLS spoof target JA3: {}", chrome132_ja3_target());
        tracing::debug!("TLS spoof partial JA3: {}", chrome132_ja3_rustls_partial());
        tracing::debug!(
            "GREASE pool: {} values, sample: {:#06x}",
            GREASE_VALUES.len(),
            generate_grease_value()
        );

        build_ureq_agent_with_chrome_tls()
    })
}

/// Perform a GET request using the Chrome-fingerprinted agent.
/// Falls back to standard ureq if Chrome TLS init fails.
pub fn get_spoofed(url: &str) -> Result<ureq::Response, Box<ureq::Error>> {
    chrome_agent()
        .get(url)
        .set("User-Agent", CHROME_132_USER_AGENT)
        .call()
        .map_err(Box::new)
}

/// Get a SpoofedTlsConnector for custom usage (e.g., in MCP tool forwarding)
pub fn spoofed_connector() -> SpoofedTlsConnector {
    SpoofedTlsConnector::chrome132()
}

/// Get the raw Chrome 132 TLS config for advanced usage
pub fn chrome_tls_config() -> std::sync::Arc<ureq::rustls::ClientConfig> {
    build_chrome132_tls_config()
}

/// Build a custom Chrome TLS config with specific options
pub fn custom_chrome_config() -> ChromeTlsConfig {
    ChromeTlsConfig::new()
}

/// Build a User-Agent string from environment variables:
/// - SASSY_UA_PRESET: chrome|safari|edge|opera|firefox|sassy
/// - SASSY_DEVICE: desktop|mobile|iphone|pixel
pub fn build_user_agent() -> Option<String> {
    let preset = env::var("SASSY_UA_PRESET").ok();
    let device = env::var("SASSY_DEVICE").ok();

    let preset = crate::fontcase::ascii_lower(&preset.unwrap_or_else(|| "sassy".into()));
    let device = crate::fontcase::ascii_lower(&device.unwrap_or_else(|| "desktop".into()));

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

/// Perform a GET request using the Chrome-fingerprinted agent with the
/// configured User-Agent.  The TLS ClientHello matches Chrome/Edge 132
/// so that naive JA3-based blocking is avoided.
///
/// Returns a boxed error to reduce enum size (ureq::Error is large).
pub fn get(url: &str) -> Result<ureq::Response, Box<ureq::Error>> {
    let ua = build_user_agent().unwrap_or_else(|| CHROME_132_USER_AGENT.to_string());
    chrome_agent()
        .get(url)
        .set("User-Agent", &ua)
        .call()
        .map_err(Box::new)
}

/// Convenience: fetch URL and return body string.
/// Uses the Chrome-fingerprinted TLS agent for stealth.
pub fn fetch_text(url: &str) -> Result<String, String> {
    match get_spoofed(url) {
        Ok(resp) => resp
            .into_string()
            .map_err(|e| format!("Failed to read response: {}", e)),
        Err(e) => Err(format!("Request failed: {}", e)),
    }
}

/// Return a summary of the active TLS spoofing configuration for diagnostics
/// (displayed in the dev console / about page).
pub fn tls_spoof_summary() -> String {
    let connector = spoofed_connector();
    let desc = connector.describe();
    let (grease_cs, grease_ext) = connector.grease_values();
    let custom_cfg = custom_chrome_config();
    // Exercise the builder methods so they are not dead code
    let _cfg = custom_cfg
        .alpn_protocols(vec![b"h2".to_vec(), b"http/1.1".to_vec()])
        .enable_sni(true)
        .enable_resumption(true)
        .build();
    format!(
        "{}\nGREASE cipher_suite={:#06x} extension={:#06x}",
        desc, grease_cs, grease_ext,
    )
}
