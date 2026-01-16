//! Open Policy Agent (OPA) integration for fine-grained access control
//!
//! Evaluates policies before code execution using OPA REST API

use std::collections::HashMap;

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, info, warn};

/// OPA configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpaConfig {
    /// Enable OPA policy evaluation
    pub enabled: bool,
    /// OPA server URL
    pub server_url: String,
    /// Policy package name
    pub policy_package: String,
    /// Decision path (e.g., "allow" for package/policy/allow)
    pub decision_path: String,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
}

impl Default for OpaConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            server_url: "http://127.0.0.1:8181".to_string(),
            policy_package: "cage.authz".to_string(),
            decision_path: "allow".to_string(),
            timeout_seconds: 5,
        }
    }
}

/// OPA policy evaluator
pub struct OpaEvaluator {
    config: OpaConfig,
    client: Client,
}

/// Input for OPA policy evaluation
#[derive(Debug, Serialize)]
pub struct PolicyInput {
    pub user_id: String,
    pub action: String,
    pub resource: String,
    pub language: String,
    pub code_hash: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// OPA decision result
#[derive(Debug, Deserialize)]
pub struct OpaDecision {
    pub result: Option<bool>,
}

impl OpaEvaluator {
    pub fn new(config: OpaConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { config, client }
    }

    /// Evaluate policy for code execution
    pub async fn evaluate_execution(&self, input: PolicyInput) -> Result<bool> {
        if !self.config.enabled {
            // If OPA is disabled, allow by default
            return Ok(true);
        }

        debug!(
            user_id = %input.user_id,
            action = %input.action,
            language = %input.language,
            "Evaluating OPA policy"
        );

        let url = format!(
            "{}/v1/data/{}/{}",
            self.config.server_url, self.config.policy_package, self.config.decision_path
        );

        let request_body = json!({
            "input": input
        });

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .context("Failed to send OPA request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            warn!(
                status = %status,
                body = %body,
                "OPA request failed"
            );
            anyhow::bail!("OPA policy evaluation failed: {}", status);
        }

        let decision: OpaDecision = response.json().await.context("Failed to parse OPA response")?;

        let allowed = decision.result.unwrap_or(false);

        info!(
            user_id = %input.user_id,
            allowed = allowed,
            "OPA policy evaluated"
        );

        Ok(allowed)
    }

    /// Check if OPA is enabled and reachable
    pub async fn health_check(&self) -> bool {
        if !self.config.enabled {
            return true;
        }

        let url = format!("{}/health", self.config.server_url);

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
    fn test_opa_config_default() {
        let config = OpaConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.server_url, "http://127.0.0.1:8181");
    }

    #[tokio::test]
    async fn test_opa_disabled() {
        let config = OpaConfig::default();
        let evaluator = OpaEvaluator::new(config);

        let input = PolicyInput {
            user_id: "test".to_string(),
            action: "execute".to_string(),
            resource: "sandbox".to_string(),
            language: "python".to_string(),
            code_hash: "abc123".to_string(),
            metadata: HashMap::new(),
        };

        // Should allow when disabled
        let result = evaluator.evaluate_execution(input).await.unwrap();
        assert!(result);
    }
}
