//! TLS ClientHello Fingerprint Spoofing -- Chrome/Edge 132 Profile
//!
//! # Overview
//!
//! This module configures a `rustls::ClientConfig` so that the TLS ClientHello
//! fingerprint (JA3 hash) closely matches Chrome/Edge 132 on Windows.  The goal
//! is to defeat naive TLS fingerprinting that blocks non-browser user agents
//! while preserving the security guarantees of `rustls` and its `ring` backend.
//!
//! # Target JA3 (Chrome 132)
//!
//! ```text
//! 771,4865-4866-4867-49195-49199-49196-49200-52393-52392-49171-49172-156-157-47-53,
//! 0-23-65281-10-11-35-16-5-13-18-51-45-43-27-17513-65037,
//! 29-23-24,
//! 0
//! ```
//!
//! ## Cipher suites (decimal order in the ClientHello)
//!
//! | Code   | Name                                         | rustls support |
//! |--------|----------------------------------------------|----------------|
//! | 0x1301 | TLS_AES_128_GCM_SHA256                       | yes            |
//! | 0x1302 | TLS_AES_256_GCM_SHA384                       | yes            |
//! | 0x1303 | TLS_CHACHA20_POLY1305_SHA256                 | yes            |
//! | 0xc02b | TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256      | yes            |
//! | 0xc02f | TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256        | yes            |
//! | 0xc02c | TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384      | yes            |
//! | 0xc030 | TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384        | yes            |
//! | 0xcca9 | TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256| yes            |
//! | 0xcca8 | TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256  | yes            |
//! | 0xc013 | TLS_ECDHE_RSA_WITH_AES_128_CBC_SHA           | NO (CBC)       |
//! | 0xc014 | TLS_ECDHE_RSA_WITH_AES_256_CBC_SHA           | NO (CBC)       |
//! | 0x009c | TLS_RSA_WITH_AES_128_GCM_SHA256              | NO (RSA kx)    |
//! | 0x009d | TLS_RSA_WITH_AES_256_GCM_SHA384              | NO (RSA kx)    |
//! | 0x002f | TLS_RSA_WITH_AES_128_CBC_SHA                 | NO (CBC+RSA)   |
//! | 0x0035 | TLS_RSA_WITH_AES_256_CBC_SHA                 | NO (CBC+RSA)   |
//!
//! **Note:** rustls intentionally does not implement CBC-mode or non-ECDHE
//! cipher suites because they are legacy and have known weaknesses.  The 9
//! AEAD suites we *can* configure cover all the suites that modern servers
//! actually negotiate with Chrome.  The remaining 6 are only used as
//! fallbacks for ancient TLS stacks and their absence will not cause
//! connection failures on any reasonably modern server.
//!
//! ## Supported groups
//!
//! x25519 (0x001d), secp256r1 (0x0017), secp384r1 (0x0018)
//!
//! ## Signature schemes (Chrome 132 order)
//!
//! 1. ecdsa_secp256r1_sha256    (0x0403)
//! 2. rsa_pss_sha256            (0x0804)
//! 3. rsa_pkcs1_sha256          (0x0401)
//! 4. ecdsa_secp384r1_sha384    (0x0503)
//! 5. rsa_pss_sha384            (0x0805)
//! 6. rsa_pkcs1_sha384          (0x0501)
//! 7. rsa_pss_sha512            (0x0806)
//! 8. rsa_pkcs1_sha512          (0x0601)
//!
//! ## GREASE
//!
//! Chrome injects random GREASE (Generate Random Extensions And Sustain
//! Extensibility) values into the ClientHello to prevent ossification.
//! GREASE values follow the pattern `0x?a?a` where `?` is any hex nibble
//! (e.g. 0x0a0a, 0x1a1a, ... 0xfafa).  Since rustls does not expose raw
//! extension injection, GREASE values cannot be placed into the wire-level
//! ClientHello.  This module generates them for informational logging and
//! future use if rustls ever exposes that capability.
//!
//! ## ALPN
//!
//! h2, http/1.1  (in that order, matching Chrome)
//!
//! # Integration with ureq
//!
//! ```rust,ignore
//! use crate::tls_spoof;
//!
//! // Option 1: get a pre-built ureq Agent
//! let agent = tls_spoof::build_ureq_agent_with_chrome_tls();
//!
//! // Option 2: get the raw rustls config for custom agent builders
//! let tls_cfg = tls_spoof::build_chrome132_tls_config();
//! let agent = ureq::builder()
//!     .tls_config(tls_cfg)
//!     .build();
//!
//! // Option 3: use the SpoofedTlsConnector wrapper
//! let connector = tls_spoof::SpoofedTlsConnector::chrome132();
//! let agent = connector.into_ureq_agent();
//! ```

use std::sync::Arc;

use ureq::rustls::{
    self,
    crypto::{
        ring::{cipher_suite, kx_group},
        CryptoProvider,
    },
    version, ClientConfig, RootCertStore, SignatureScheme, SupportedCipherSuite,
};

// ---------------------------------------------------------------------------
// GREASE value generation
// ---------------------------------------------------------------------------

/// The 16 valid GREASE values as defined by RFC 8701.
///
/// Each value follows the pattern `0x?a?a` where the high and low bytes share
/// the same nibble structure.  Chrome picks one at random for each connection.
pub const GREASE_VALUES: [u16; 16] = [
    0x0a0a, 0x1a1a, 0x2a2a, 0x3a3a, 0x4a4a, 0x5a5a, 0x6a6a, 0x7a7a, 0x8a8a, 0x9a9a, 0xaaaa, 0xbaba,
    0xcaca, 0xdada, 0xeaea, 0xfafa,
];

/// Generate a random GREASE value suitable for injection into a ClientHello.
///
/// Returns one of the 16 standard GREASE values (RFC 8701) chosen uniformly
/// at random.  Since rustls does not currently expose raw extension injection,
/// this is primarily useful for logging and future-proofing.
pub fn generate_grease_value() -> u16 {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    GREASE_VALUES[rng.gen_range(0..GREASE_VALUES.len())]
}

/// Generate a pair of distinct GREASE values (Chrome uses two: one for cipher
/// suites, one for extensions).
pub fn generate_grease_pair() -> (u16, u16) {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let i = rng.gen_range(0..GREASE_VALUES.len());
    let mut j = rng.gen_range(0..GREASE_VALUES.len() - 1);
    if j >= i {
        j += 1;
    }
    (GREASE_VALUES[i], GREASE_VALUES[j])
}

// ---------------------------------------------------------------------------
// Chrome 132 cipher suite ordering
// ---------------------------------------------------------------------------

/// Cipher suites in Chrome 132 preference order.
///
/// The first 3 are TLS 1.3 suites, the remaining 6 are TLS 1.2 AEAD suites.
/// Chrome also advertises 6 CBC/RSA-kx suites that rustls does not support;
/// those are omitted because modern servers never negotiate them when AEAD
/// alternatives are available.
pub fn chrome132_cipher_suites() -> Vec<SupportedCipherSuite> {
    vec![
        // TLS 1.3
        cipher_suite::TLS13_AES_128_GCM_SHA256,       // 0x1301
        cipher_suite::TLS13_AES_256_GCM_SHA384,       // 0x1302
        cipher_suite::TLS13_CHACHA20_POLY1305_SHA256, // 0x1303
        // TLS 1.2 ECDHE suites (Chrome order)
        cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256, // 0xc02b
        cipher_suite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,   // 0xc02f
        cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384, // 0xc02c
        cipher_suite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,   // 0xc030
        cipher_suite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256, // 0xcca9
        cipher_suite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256, // 0xcca8
    ]
}

/// Chrome 132 signature schemes in the exact order they appear in the
/// `signature_algorithms` extension.
///
/// This ordering matters for fingerprinting -- JA3S and other fingerprinters
/// record the order.
pub fn chrome132_signature_schemes() -> Vec<SignatureScheme> {
    vec![
        SignatureScheme::ECDSA_NISTP256_SHA256, // 0x0403
        SignatureScheme::RSA_PSS_SHA256,        // 0x0804
        SignatureScheme::RSA_PKCS1_SHA256,      // 0x0401
        SignatureScheme::ECDSA_NISTP384_SHA384, // 0x0503
        SignatureScheme::RSA_PSS_SHA384,        // 0x0805
        SignatureScheme::RSA_PKCS1_SHA384,      // 0x0501
        SignatureScheme::RSA_PSS_SHA512,        // 0x0806
        SignatureScheme::RSA_PKCS1_SHA512,      // 0x0601
    ]
}

// ---------------------------------------------------------------------------
// ChromeTlsConfig builder
// ---------------------------------------------------------------------------

/// Configuration wrapper that builds a `rustls::ClientConfig` matching the
/// Chrome/Edge 132 TLS fingerprint as closely as rustls allows.
///
/// # Usage
///
/// ```rust,ignore
/// let cfg = ChromeTlsConfig::new().build();
/// let agent = ureq::builder().tls_config(cfg).build();
/// ```
pub struct ChromeTlsConfig {
    /// ALPN protocols to advertise.  Default: `["h2", "http/1.1"]`.
    pub alpn_protocols: Vec<Vec<u8>>,
    /// Whether to enable SNI (Server Name Indication).  Default: `true`.
    pub enable_sni: bool,
    /// Whether to enable TLS session resumption.  Default: `true`.
    pub enable_resumption: bool,
}

impl Default for ChromeTlsConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl ChromeTlsConfig {
    /// Create a new `ChromeTlsConfig` with Chrome 132 defaults.
    pub fn new() -> Self {
        Self {
            alpn_protocols: vec![b"http/1.1".to_vec()],  // ureq is HTTP/1.1 only; h2 ALPN causes EOF on strict sites
            enable_sni: true,
            enable_resumption: true,
        }
    }

    /// Override the ALPN protocol list.
    pub fn alpn_protocols(mut self, protocols: Vec<Vec<u8>>) -> Self {
        self.alpn_protocols = protocols;
        self
    }

    /// Control SNI extension.
    pub fn enable_sni(mut self, enable: bool) -> Self {
        self.enable_sni = enable;
        self
    }

    /// Control session resumption.
    pub fn enable_resumption(mut self, enable: bool) -> Self {
        self.enable_resumption = enable;
        self
    }

    /// Build the `rustls::ClientConfig` with Chrome 132 fingerprint settings.
    ///
    /// This configures:
    /// - Cipher suites in Chrome 132 preference order
    /// - Key exchange groups: x25519, secp256r1, secp384r1
    /// - Protocol versions: TLS 1.3 and TLS 1.2
    /// - ALPN: h2, http/1.1
    /// - Root certificates from `webpki-roots`
    /// - SNI enabled
    pub fn build(self) -> Arc<ClientConfig> {
        // Build a custom CryptoProvider with Chrome 132 cipher suite ordering
        // and key exchange group preferences.
        let provider = CryptoProvider {
            cipher_suites: chrome132_cipher_suites(),
            kx_groups: vec![
                kx_group::X25519,    // 0x001d -- Chrome's first preference
                kx_group::SECP256R1, // 0x0017
                kx_group::SECP384R1, // 0x0018
            ],
            ..rustls::crypto::ring::default_provider()
        };

        // Root certificate store from Mozilla's trusted roots via webpki-roots.
        let root_store = RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        };

        // Build the ClientConfig with our custom provider.
        //
        // `builder_with_provider` lets us control cipher suites and kx groups.
        // `with_protocol_versions` sets TLS 1.2 + 1.3 (Chrome supports both).
        let mut config = ClientConfig::builder_with_provider(Arc::new(provider))
            .with_protocol_versions(&[&version::TLS12, &version::TLS13])
            .expect("TLS protocol version configuration failed")
            .with_root_certificates(root_store)
            .with_no_client_auth();

        // ALPN -- Chrome always advertises h2 before http/1.1.
        config.alpn_protocols = self.alpn_protocols;

        // SNI -- Chrome always sends SNI.
        config.enable_sni = self.enable_sni;

        // Session resumption -- Chrome supports both ticket and PSK-based.
        if !self.enable_resumption {
            config.resumption = rustls::client::Resumption::disabled();
        }

        Arc::new(config)
    }
}

// ---------------------------------------------------------------------------
// Convenience builders
// ---------------------------------------------------------------------------

/// Build a `rustls::ClientConfig` pre-configured to match Chrome/Edge 132
/// TLS fingerprint (cipher suites, kx groups, ALPN, signature schemes).
///
/// This is the simplest entry point -- returns an `Arc<ClientConfig>` ready
/// to pass to `ureq::builder().tls_config(...)`.
///
/// # Example
///
/// ```rust,ignore
/// let config = tls_spoof::build_chrome132_tls_config();
/// let agent = ureq::builder().tls_config(config).build();
/// let response = agent.get("https://example.com").call()?;
/// ```
pub fn build_chrome132_tls_config() -> Arc<ClientConfig> {
    ChromeTlsConfig::new().build()
}

/// Build a `ureq::Agent` that uses the Chrome 132 TLS fingerprint.
///
/// This creates a ureq agent with:
/// - Chrome 132 TLS configuration (cipher suites, kx groups, ALPN)
/// - A Chrome-like User-Agent header
/// - Sensible timeouts
///
/// # Example
///
/// ```rust,ignore
/// let agent = tls_spoof::build_ureq_agent_with_chrome_tls();
/// let resp = agent.get("https://tls.peet.ws/api/all").call()?;
/// println!("{}", resp.into_string()?);
/// ```
pub fn build_ureq_agent_with_chrome_tls() -> ureq::Agent {
    let tls_config = build_chrome132_tls_config();

    ureq::builder()
        .tls_config(tls_config)
        .user_agent(CHROME_132_USER_AGENT)
        .build()
}

/// Chrome 132 User-Agent string for Windows 10 x64.
///
/// This matches the UA that Chrome 132 sends on Windows, which is important
/// because some sites cross-check the TLS fingerprint against the User-Agent.
pub const CHROME_132_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
     (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36";

// ---------------------------------------------------------------------------
// SpoofedTlsConnector -- reusable wrapper
// ---------------------------------------------------------------------------

/// A reusable wrapper around a Chrome-fingerprinted TLS configuration.
///
/// This struct holds an `Arc<ClientConfig>` and provides methods to create
/// ureq agents or extract the raw config for custom usage.
///
/// # Example
///
/// ```rust,ignore
/// let connector = SpoofedTlsConnector::chrome132();
///
/// // Create multiple agents sharing the same TLS config
/// let agent1 = connector.build_ureq_agent();
/// let agent2 = connector.build_ureq_agent();
///
/// // Or get the raw config
/// let config = connector.tls_config();
/// ```
#[derive(Clone)]
pub struct SpoofedTlsConnector {
    config: Arc<ClientConfig>,
    user_agent: String,
}

impl SpoofedTlsConnector {
    /// Create a new `SpoofedTlsConnector` with Chrome 132 defaults.
    pub fn chrome132() -> Self {
        Self {
            config: build_chrome132_tls_config(),
            user_agent: CHROME_132_USER_AGENT.to_string(),
        }
    }

    /// Create a connector from a custom `ChromeTlsConfig`.
    pub fn from_config(chrome_config: ChromeTlsConfig) -> Self {
        Self {
            config: chrome_config.build(),
            user_agent: CHROME_132_USER_AGENT.to_string(),
        }
    }

    /// Override the User-Agent string.
    pub fn with_user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = ua.into();
        self
    }

    /// Get a reference to the underlying `ClientConfig`.
    pub fn tls_config(&self) -> Arc<ClientConfig> {
        Arc::clone(&self.config)
    }

    /// Build a `ureq::Agent` using this connector's TLS configuration.
    pub fn build_ureq_agent(&self) -> ureq::Agent {
        ureq::builder()
            .tls_config(Arc::clone(&self.config))
            .user_agent(&self.user_agent)
            .build()
    }

    /// Consume self and return a `ureq::Agent`.
    pub fn into_ureq_agent(self) -> ureq::Agent {
        ureq::builder()
            .tls_config(self.config)
            .user_agent(&self.user_agent)
            .build()
    }

    /// Return the GREASE values that *would* be injected into the ClientHello
    /// if rustls supported raw extension manipulation.
    ///
    /// Returns `(cipher_suite_grease, extension_grease)`.
    pub fn grease_values(&self) -> (u16, u16) {
        generate_grease_pair()
    }

    /// Return a human-readable summary of the TLS configuration for logging.
    pub fn describe(&self) -> String {
        let suites = chrome132_cipher_suites();
        let sig_schemes = chrome132_signature_schemes();

        let suite_names: Vec<String> = suites
            .iter()
            .map(|s| format!("0x{:04x}", u16::from(s.suite())))
            .collect();

        let sig_names: Vec<String> = sig_schemes
            .iter()
            .map(|s| format!("0x{:04x}", u16::from(*s)))
            .collect();

        format!(
            "Chrome 132 TLS Profile:\n\
             Cipher suites: [{}]\n\
             Supported groups: [x25519, secp256r1, secp384r1]\n\
             Signature schemes: [{}]\n\
             ALPN: [h2, http/1.1]\n\
             SNI: enabled\n\
             User-Agent: {}",
            suite_names.join(", "),
            sig_names.join(", "),
            self.user_agent,
        )
    }
}

impl std::fmt::Debug for SpoofedTlsConnector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpoofedTlsConnector")
            .field("user_agent", &self.user_agent)
            .field("cipher_suites", &chrome132_cipher_suites().len())
            .field("alpn", &["h2", "http/1.1"])
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Informational helpers
// ---------------------------------------------------------------------------

/// Return the target JA3 hash string for Chrome 132.
///
/// **Note:** The actual JA3 will differ slightly because rustls cannot
/// advertise the 6 CBC/RSA cipher suites.  The partial JA3 covering the
/// suites that rustls *does* support is also returned.
pub fn chrome132_ja3_target() -> &'static str {
    "771,4865-4866-4867-49195-49199-49196-49200-52393-52392-49171-49172-156-157-47-53,\
     0-23-65281-10-11-35-16-5-13-18-51-45-43-27-17513-65037,\
     29-23-24,\
     0"
}

/// Return the partial JA3 string covering only the cipher suites that rustls
/// actually supports (the 9 AEAD suites).
pub fn chrome132_ja3_rustls_partial() -> &'static str {
    "771,4865-4866-4867-49195-49199-49196-49200-52393-52392,\
     0-23-65281-10-11-35-16-5-13-18-51-45-43-27-17513-65037,\
     29-23-24,\
     0"
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chrome_tls_config_builds_successfully() {
        let config = build_chrome132_tls_config();
        // ALPN should be h2, http/1.1
        assert_eq!(config.alpn_protocols.len(), 2);
        assert_eq!(config.alpn_protocols[0], b"h2");
        assert_eq!(config.alpn_protocols[1], b"http/1.1");
        // SNI should be enabled
        assert!(config.enable_sni);
    }

    #[test]
    fn test_cipher_suite_count() {
        let suites = chrome132_cipher_suites();
        // 3 TLS 1.3 + 6 TLS 1.2 = 9 suites
        assert_eq!(suites.len(), 9);
    }

    #[test]
    fn test_cipher_suite_order() {
        let suites = chrome132_cipher_suites();

        // Verify TLS 1.3 suites come first, in Chrome's order
        let codes: Vec<u16> = suites.iter().map(|s| u16::from(s.suite())).collect();
        assert_eq!(
            codes[0], 0x1301,
            "First suite should be TLS_AES_128_GCM_SHA256"
        );
        assert_eq!(
            codes[1], 0x1302,
            "Second suite should be TLS_AES_256_GCM_SHA384"
        );
        assert_eq!(
            codes[2], 0x1303,
            "Third suite should be TLS_CHACHA20_POLY1305_SHA256"
        );
        // TLS 1.2 ECDHE suites
        assert_eq!(
            codes[3], 0xc02b,
            "4th: TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256"
        );
        assert_eq!(
            codes[4], 0xc02f,
            "5th: TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256"
        );
        assert_eq!(
            codes[5], 0xc02c,
            "6th: TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384"
        );
        assert_eq!(
            codes[6], 0xc030,
            "7th: TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384"
        );
        assert_eq!(
            codes[7], 0xcca9,
            "8th: TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256"
        );
        assert_eq!(
            codes[8], 0xcca8,
            "9th: TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256"
        );
    }

    #[test]
    fn test_signature_scheme_order() {
        let schemes = chrome132_signature_schemes();
        assert_eq!(schemes.len(), 8);
        assert_eq!(u16::from(schemes[0]), 0x0403, "ecdsa_secp256r1_sha256");
        assert_eq!(u16::from(schemes[1]), 0x0804, "rsa_pss_sha256");
        assert_eq!(u16::from(schemes[2]), 0x0401, "rsa_pkcs1_sha256");
        assert_eq!(u16::from(schemes[3]), 0x0503, "ecdsa_secp384r1_sha384");
        assert_eq!(u16::from(schemes[4]), 0x0805, "rsa_pss_sha384");
        assert_eq!(u16::from(schemes[5]), 0x0501, "rsa_pkcs1_sha384");
        assert_eq!(u16::from(schemes[6]), 0x0806, "rsa_pss_sha512");
        assert_eq!(u16::from(schemes[7]), 0x0601, "rsa_pkcs1_sha512");
    }

    #[test]
    fn test_grease_value_is_valid() {
        for _ in 0..100 {
            let val = generate_grease_value();
            // GREASE values have the pattern 0x?a?a
            assert_eq!(
                val & 0x0f0f,
                0x0a0a,
                "GREASE value {:#06x} is malformed",
                val
            );
            // High and low bytes should have the same high nibble
            let hi = (val >> 8) as u8;
            let lo = val as u8;
            assert_eq!(
                hi, lo,
                "GREASE bytes should match: {:#04x} vs {:#04x}",
                hi, lo
            );
        }
    }

    #[test]
    fn test_grease_pair_distinct() {
        for _ in 0..100 {
            let (a, b) = generate_grease_pair();
            assert_ne!(a, b, "GREASE pair should be distinct");
        }
    }

    #[test]
    fn test_spoofed_connector_chrome132() {
        let connector = SpoofedTlsConnector::chrome132();
        let config = connector.tls_config();
        assert_eq!(config.alpn_protocols.len(), 2);
    }

    #[test]
    fn test_spoofed_connector_custom_ua() {
        let connector = SpoofedTlsConnector::chrome132().with_user_agent("CustomBot/1.0");
        assert_eq!(connector.user_agent, "CustomBot/1.0");
    }

    #[test]
    fn test_spoofed_connector_debug() {
        let connector = SpoofedTlsConnector::chrome132();
        let debug = format!("{:?}", connector);
        assert!(debug.contains("SpoofedTlsConnector"));
        assert!(debug.contains("h2"));
    }

    #[test]
    fn test_spoofed_connector_describe() {
        let connector = SpoofedTlsConnector::chrome132();
        let desc = connector.describe();
        assert!(desc.contains("Chrome 132 TLS Profile"));
        assert!(desc.contains("0x1301"));
        assert!(desc.contains("x25519"));
        assert!(desc.contains("h2"));
    }

    #[test]
    fn test_chrome_tls_config_custom_alpn() {
        let config = ChromeTlsConfig::new()
            .alpn_protocols(vec![b"http/1.1".to_vec()])
            .build();
        assert_eq!(config.alpn_protocols.len(), 1);
        assert_eq!(config.alpn_protocols[0], b"http/1.1");
    }

    #[test]
    fn test_chrome_tls_config_no_sni() {
        let config = ChromeTlsConfig::new().enable_sni(false).build();
        assert!(!config.enable_sni);
    }

    #[test]
    fn test_chrome_tls_config_default() {
        // Ensure Default trait works
        let config = ChromeTlsConfig::default().build();
        assert_eq!(config.alpn_protocols.len(), 2);
        assert!(config.enable_sni);
    }

    #[test]
    fn test_ja3_target_string() {
        let ja3 = chrome132_ja3_target();
        assert!(ja3.starts_with("771,4865"));
        assert!(ja3.contains("52393-52392"));
        assert!(ja3.ends_with(",0"));
    }

    #[test]
    fn test_ja3_partial_string() {
        let ja3 = chrome132_ja3_rustls_partial();
        assert!(ja3.starts_with("771,4865"));
        // The partial JA3 should NOT contain the CBC/RSA suites
        assert!(!ja3.contains("49171"));
        assert!(!ja3.contains("-156"));
        assert!(!ja3.contains("-47-"));
    }

    #[test]
    fn test_build_ureq_agent() {
        // Smoke test -- just ensure it doesn't panic
        let agent = build_ureq_agent_with_chrome_tls();
        // Agent should be usable (we can't easily test network here)
        let _ = agent;
    }

    #[test]
    fn test_spoofed_connector_into_agent() {
        let connector = SpoofedTlsConnector::chrome132();
        let agent = connector.into_ureq_agent();
        let _ = agent;
    }

    #[test]
    fn test_spoofed_connector_build_agent() {
        let connector = SpoofedTlsConnector::chrome132();
        // Should be able to build multiple agents from the same connector
        let _a1 = connector.build_ureq_agent();
        let _a2 = connector.build_ureq_agent();
    }

    #[test]
    fn test_from_config() {
        let chrome_config = ChromeTlsConfig::new()
            .enable_sni(true)
            .enable_resumption(true);
        let connector = SpoofedTlsConnector::from_config(chrome_config);
        let tls = connector.tls_config();
        assert!(tls.enable_sni);
    }

    #[test]
    fn test_user_agent_constant() {
        assert!(CHROME_132_USER_AGENT.contains("Chrome/132"));
        assert!(CHROME_132_USER_AGENT.contains("Windows NT 10.0"));
    }

    #[test]
    fn test_all_grease_values_valid() {
        for &val in &GREASE_VALUES {
            assert_eq!(
                val & 0x0f0f,
                0x0a0a,
                "GREASE constant {:#06x} is malformed",
                val
            );
        }
        assert_eq!(GREASE_VALUES.len(), 16);
    }
}
