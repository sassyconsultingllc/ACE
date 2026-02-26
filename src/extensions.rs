// Extensions - WebExtension-compatible extension support

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub manifest_version: u32,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub content_scripts: Vec<ContentScript>,
    #[serde(default)]
    pub background: Option<BackgroundScript>,
    #[serde(default)]
    pub browser_action: Option<BrowserAction>,
    #[serde(default)]
    pub page_action: Option<PageAction>,
    #[serde(default)]
    pub icons: HashMap<String, String>,
    #[serde(default)]
    pub web_accessible_resources: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentScript {
    pub matches: Vec<String>,
    #[serde(default)]
    pub js: Vec<String>,
    #[serde(default)]
    pub css: Vec<String>,
    #[serde(default = "default_run_at")]
    pub run_at: String,
    #[serde(default)]
    pub all_frames: bool,
}

fn default_run_at() -> String {
    "document_idle".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundScript {
    #[serde(default)]
    pub scripts: Vec<String>,
    #[serde(default)]
    pub service_worker: Option<String>,
    #[serde(default = "default_persistent")]
    pub persistent: bool,
}

fn default_persistent() -> bool {
    false
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserAction {
    #[serde(default)]
    pub default_icon: Option<String>,
    #[serde(default)]
    pub default_title: Option<String>,
    #[serde(default)]
    pub default_popup: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageAction {
    #[serde(default)]
    pub default_icon: Option<String>,
    #[serde(default)]
    pub default_title: Option<String>,
    #[serde(default)]
    pub default_popup: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Extension {
    pub id: String,
    pub path: String,
    pub manifest: ExtensionManifest,
    pub enabled: bool,
    pub content_scripts_code: HashMap<String, String>,
    pub background_script_code: Option<String>,
}

impl Extension {
    pub fn load(path: &str) -> Result<Self, String> {
        let manifest_path = Path::new(path).join("manifest.json");
        let manifest_content = fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Failed to read manifest: {}", e))?;

        let manifest: ExtensionManifest = serde_json::from_str(&manifest_content)
            .map_err(|e| format!("Failed to parse manifest: {}", e))?;

        let id = crate::fontcase::ascii_lower(&manifest.name).replace(' ', "-");

        // Load content scripts
        let mut content_scripts_code = HashMap::new();
        for cs in &manifest.content_scripts {
            for js_file in &cs.js {
                let js_path = Path::new(path).join(js_file);
                if let Ok(code) = fs::read_to_string(&js_path) {
                    content_scripts_code.insert(js_file.clone(), code);
                }
            }
        }

        // Load background script
        let background_script_code = if let Some(ref bg) = manifest.background {
            if let Some(ref sw) = bg.service_worker {
                let bg_path = Path::new(path).join(sw);
                fs::read_to_string(&bg_path).ok()
            } else if !bg.scripts.is_empty() {
                let bg_path = Path::new(path).join(&bg.scripts[0]);
                fs::read_to_string(&bg_path).ok()
            } else {
                None
            }
        } else {
            None
        };

        Ok(Extension {
            id,
            path: path.to_string(),
            manifest,
            enabled: true,
            content_scripts_code,
            background_script_code,
        })
    }

    pub fn matches_url(&self, url: &str) -> Vec<&ContentScript> {
        self.manifest
            .content_scripts
            .iter()
            .filter(|cs| {
                cs.matches
                    .iter()
                    .any(|pattern| Self::match_pattern(pattern, url))
            })
            .collect()
    }

    fn match_pattern(pattern: &str, url: &str) -> bool {
        if pattern == "<all_urls>" {
            return true;
        }

        // Parse pattern: scheme://host/path
        let parts: Vec<&str> = pattern.splitn(2, "://").collect();
        if parts.len() != 2 {
            return false;
        }

        let scheme_pattern = parts[0];
        let rest = parts[1];

        // Check scheme
        let url_scheme = if url.starts_with("https://") {
            "https"
        } else if url.starts_with("http://") {
            "http"
        } else {
            return false;
        };

        if scheme_pattern != "*" && scheme_pattern != url_scheme {
            return false;
        }

        // Parse host and path
        let (host_pattern, path_pattern) = if let Some(idx) = rest.find('/') {
            (&rest[..idx], &rest[idx..])
        } else {
            (rest, "/*")
        };

        // Get URL host and path
        let url_without_scheme = if let Some(stripped) = url.strip_prefix("https://") {
            stripped
        } else if let Some(stripped) = url.strip_prefix("http://") {
            stripped
        } else {
            url
        };

        let (url_host, url_path) = if let Some(idx) = url_without_scheme.find('/') {
            (&url_without_scheme[..idx], &url_without_scheme[idx..])
        } else {
            (url_without_scheme, "/")
        };

        // Match host
        if host_pattern != "*" {
            if let Some(suffix) = host_pattern.strip_prefix("*.") {
                if !url_host.ends_with(suffix) && url_host != suffix {
                    return false;
                }
            } else if host_pattern != url_host {
                return false;
            }
        }

        // Match path
        if path_pattern != "/*" {
            if let Some(prefix) = path_pattern.strip_suffix('*') {
                if !url_path.starts_with(prefix) {
                    return false;
                }
            } else if path_pattern != url_path {
                return false;
            }
        }

        true
    }

    pub fn get_content_script_code(&self, js_file: &str) -> Option<&String> {
        self.content_scripts_code.get(js_file)
    }
}

pub struct ExtensionManager {
    pub extensions: Vec<Extension>,
    pub storage: HashMap<String, HashMap<String, String>>,
}

impl ExtensionManager {
    pub fn new() -> Self {
        ExtensionManager {
            extensions: Vec::new(),
            storage: HashMap::new(),
        }
    }

    pub fn load_extension(&mut self, path: &str) -> Result<(), String> {
        let extension = Extension::load(path)?;
        let id = extension.id.clone();

        // Initialize storage
        self.storage.insert(id.clone(), HashMap::new());

        // Run background script if present
        if let Some(ref _bg_code) = extension.background_script_code {
            // Background scripts would run in a separate context
            println!(
                "[Extension] Loaded background script for: {}",
                extension.manifest.name
            );
        }

        self.extensions.push(extension);
        Ok(())
    }

    pub fn unload_extension(&mut self, id: &str) {
        self.extensions.retain(|e| e.id != id);
        self.storage.remove(id);
    }

    pub fn enable_extension(&mut self, id: &str) {
        if let Some(ext) = self.extensions.iter_mut().find(|e| e.id == id) {
            ext.enabled = true;
        }
    }

    pub fn disable_extension(&mut self, id: &str) {
        if let Some(ext) = self.extensions.iter_mut().find(|e| e.id == id) {
            ext.enabled = false;
        }
    }

    pub fn get_content_scripts(&self, url: &str) -> Vec<String> {
        let mut scripts = Vec::new();

        for ext in &self.extensions {
            if !ext.enabled {
                continue;
            }

            for cs in ext.matches_url(url) {
                for js_file in &cs.js {
                    if let Some(code) = ext.get_content_script_code(js_file) {
                        scripts.push(code.clone());
                    }
                }
            }
        }

        scripts
    }

    pub fn get_content_styles(&self, url: &str) -> Vec<String> {
        let mut styles = Vec::new();

        for ext in &self.extensions {
            if !ext.enabled {
                continue;
            }

            for cs in ext.matches_url(url) {
                for css_file in &cs.css {
                    let css_path = Path::new(&ext.path).join(css_file);
                    if let Ok(css) = fs::read_to_string(&css_path) {
                        styles.push(css);
                    }
                }
            }
        }

        styles
    }

    pub fn set_storage(&mut self, ext_id: &str, key: &str, value: &str) {
        if let Some(storage) = self.storage.get_mut(ext_id) {
            storage.insert(key.to_string(), value.to_string());
        }
    }

    pub fn get_storage(&self, ext_id: &str, key: &str) -> Option<&String> {
        self.storage.get(ext_id).and_then(|s| s.get(key))
    }

    pub fn remove_storage(&mut self, ext_id: &str, key: &str) {
        if let Some(storage) = self.storage.get_mut(ext_id) {
            storage.remove(key);
        }
    }

    pub fn list_extensions(&self) -> Vec<ExtensionInfo> {
        self.extensions
            .iter()
            .map(|e| ExtensionInfo {
                id: e.id.clone(),
                name: e.manifest.name.clone(),
                version: e.manifest.version.clone(),
                description: e.manifest.description.clone(),
                enabled: e.enabled,
            })
            .collect()
    }
}

impl Default for ExtensionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ExtensionInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
}

// Extension API that would be exposed to content scripts
pub struct ExtensionAPI {
    extension_id: String,
}

impl ExtensionAPI {
    pub fn new(ext_id: &str) -> Self {
        ExtensionAPI {
            extension_id: ext_id.to_string(),
        }
    }

    pub fn send_message(&self, _message: &str) -> Result<String, String> {
        // Would communicate with background script
        Ok("{}".to_string())
    }

    pub fn get_url(&self, path: &str) -> String {
        format!("sassy-extension://{}/{}", self.extension_id, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn load_extension_and_apply_assets() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path();

        // Create a simple manifest.json
        let manifest = serde_json::json!({
            "manifest_version": 2,
            "name": "Test Ext",
            "version": "0.1",
            "content_scripts": [
                {
                    "matches": ["<all_urls>"],
                    "js": ["script.js"],
                    "css": ["style.css"]
                }
            ]
        });

        fs::write(path.join("manifest.json"), manifest.to_string()).expect("write manifest");
        fs::write(path.join("script.js"), "console.log('hi');").expect("write js");
        fs::write(path.join("style.css"), "p { color: red; }").expect("write css");

        let mut mgr = ExtensionManager::new();
        mgr.load_extension(path.to_str().unwrap())
            .expect("load ext");

        let styles = mgr.get_content_styles("http://example/");
        assert!(!styles.is_empty(), "styles should be found");

        let scripts = mgr.get_content_scripts("http://example/");
        assert!(!scripts.is_empty(), "scripts should be found");

        // Now test applying styles into HtmlRenderer
        use crate::html_renderer::HtmlRenderer;
        let mut renderer = HtmlRenderer::new();
        let html = "<html><head><title>Test</title></head><body><p>Hello</p></body></html>";
        renderer.parse_html(html);
        renderer.apply_content_styles(&styles);

        let doc = renderer.cached_doc.expect("doc");
        assert!(
            !doc.styles.is_empty(),
            "document styles should include extension styles"
        );
    }
}
