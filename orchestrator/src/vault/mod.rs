//! HashiCorp Vault integration for secret management
//!
//! Retrieves API keys, JWT secrets, and other sensitive data from Vault

use std::collections::HashMap;

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Vault configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    /// Enable Vault integration
    pub enabled: bool,
    /// Vault server address
    pub address: String,
    /// Vault token for authentication
    pub token: String,
    /// Path to secrets (e.g., "secret/data/cage")
    pub secret_path: String,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            address: "http://127.0.0.1:8200".to_string(),
            token: String::new(),
            secret_path: "secret/data/cage".to_string(),
            timeout_seconds: 10,
        }
    }
}

/// Vault secret response
#[derive(Debug, Deserialize)]
struct VaultResponse {
    data: VaultData,
}

#[derive(Debug, Deserialize)]
struct VaultData {
    data: HashMap<String, serde_json::Value>,
}

/// Vault client for secret retrieval
pub struct VaultClient {
    config: VaultConfig,
    client: Client,
}

impl VaultClient {
    pub fn new(config: VaultConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { config, client }
    }

    /// Get a secret from Vault
    pub async fn get_secret(&self, key: &str) -> Result<String> {
        if !self.config.enabled {
            anyhow::bail!("Vault is not enabled");
        }

        debug!(key = %key, "Retrieving secret from Vault");

        let url = format!("{}/v1/{}", self.config.address, self.config.secret_path);

        let response = self
            .client
            .get(&url)
            .header("X-Vault-Token", &self.config.token)
            .send()
            .await
            .context("Failed to send Vault request")?;

        if !response.status().is_success() {
            let status = response.status();
            warn!(status = %status, "Vault request failed");
            anyhow::bail!("Vault request failed: {}", status);
        }

        let vault_response: VaultResponse = response
            .json()
            .await
            .context("Failed to parse Vault response")?;

        let value = vault_response
            .data
            .data
            .get(key)
            .and_then(|v| v.as_str())
            .context("Secret not found in Vault")?
            .to_string();

        info!(key = %key, "Secret retrieved from Vault");

        Ok(value)
    }

    /// Get multiple secrets at once
    pub async fn get_secrets(&self) -> Result<HashMap<String, String>> {
        if !self.config.enabled {
            return Ok(HashMap::new());
        }

        let url = format!("{}/v1/{}", self.config.address, self.config.secret_path);

        let response = self
            .client
            .get(&url)
            .header("X-Vault-Token", &self.config.token)
            .send()
            .await
            .context("Failed to send Vault request")?;

        if !response.status().is_success() {
            anyhow::bail!("Vault request failed: {}", response.status());
        }

        let vault_response: VaultResponse = response.json().await?;

        let secrets: HashMap<String, String> = vault_response
            .data
            .data
            .iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect();

        info!(count = secrets.len(), "Retrieved secrets from Vault");

        Ok(secrets)
    }

    /// Health check for Vault connectivity
    pub async fn health_check(&self) -> bool {
        if !self.config.enabled {
            return true;
        }

        let url = format!("{}/v1/sys/health", self.config.address);

        match self.client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_config_default() {
        let config = VaultConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.address, "http://127.0.0.1:8200");
    }

    #[tokio::test]
    async fn test_vault_disabled() {
        let config = VaultConfig::default();
        let client = VaultClient::new(config);

        // Should error when trying to get secret while disabled
        let result = client.get_secret("test").await;
        assert!(result.is_err());
    }
}
