//! Network egress proxy for application-layer filtering
//!
//! HTTP/HTTPS proxy with URL whitelisting and content inspection

use serde::{Deserialize, Serialize};

/// Proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Enable egress proxy
    #[serde(default)]
    pub enabled: bool,
    /// Proxy listen address
    #[serde(default = "default_listen")]
    pub listen_addr: String,
    /// Proxy port
    #[serde(default = "default_port")]
    pub port: u16,
    /// Allowed destination URLs (regex patterns)
    #[serde(default)]
    pub allowed_urls: Vec<String>,
    /// Block by default (if false, allow all except blocked_urls)
    #[serde(default = "default_true")]
    pub block_by_default: bool,
    /// Explicitly blocked URLs
    #[serde(default)]
    pub blocked_urls: Vec<String>,
    /// Log all requests
    #[serde(default = "default_true")]
    pub log_requests: bool,
}

fn default_listen() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    3128
}

fn default_true() -> bool {
    true
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            listen_addr: default_listen(),
            port: default_port(),
            allowed_urls: vec![
                r"^https://api\.example\.com/.*".to_string(),
                r"^https://.*\.safe-domain\.com/.*".to_string(),
            ],
            block_by_default: true,
            blocked_urls: vec![],
            log_requests: true,
        }
    }
}

/// Egress proxy server (implementation in separate module for production)
pub struct EgressProxy {
    config: ProxyConfig,
}

impl EgressProxy {
    pub fn new(config: ProxyConfig) -> Self {
        Self { config }
    }

    /// Check if URL is allowed
    pub fn is_url_allowed(&self, url: &str) -> bool {
        if self.config.block_by_default {
            // Check if URL matches any allowed pattern
            self.config.allowed_urls.iter().any(|pattern| {
                regex::Regex::new(pattern)
                    .map(|re| re.is_match(url))
                    .unwrap_or(false)
            })
        } else {
            // Allow unless explicitly blocked
            !self.config.blocked_urls.iter().any(|pattern| {
                regex::Regex::new(pattern)
                    .map(|re| re.is_match(url))
                    .unwrap_or(false)
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_config_default() {
        let config = ProxyConfig::default();
        assert!(!config.enabled);
        assert!(config.block_by_default);
    }

    #[test]
    fn test_url_filtering() {
        let config = ProxyConfig {
            enabled: true,
            block_by_default: true,
            allowed_urls: vec![r"^https://api\.safe\.com/.*".to_string()],
            blocked_urls: vec![],
            ..Default::default()
        };

        let proxy = EgressProxy::new(config);

        assert!(proxy.is_url_allowed("https://api.safe.com/data"));
        assert!(!proxy.is_url_allowed("https://evil.com/malware"));
    }
}

