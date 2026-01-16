//! Alert system for Slack, PagerDuty, and email notifications
//!
//! Sends alerts for security events, errors, and resource thresholds

use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, error, info, warn};

/// Alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    /// Enable alerting
    pub enabled: bool,
    /// Slack webhook URL
    pub slack_webhook_url: Option<String>,
    /// PagerDuty integration key
    pub pagerduty_key: Option<String>,
    /// Email SMTP settings (optional)
    pub email: Option<EmailConfig>,
    /// Alert on execution errors
    pub alert_on_errors: bool,
    /// Alert on security events
    pub alert_on_security: bool,
    /// Alert on resource thresholds
    pub alert_on_resources: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub smtp_server: String,
    pub smtp_port: u16,
    pub from_address: String,
    pub to_addresses: Vec<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            slack_webhook_url: None,
            pagerduty_key: None,
            email: None,
            alert_on_errors: true,
            alert_on_security: true,
            alert_on_resources: true,
        }
    }
}

/// Alert severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Alert message
#[derive(Debug, Clone)]
pub struct Alert {
    pub severity: AlertSeverity,
    pub title: String,
    pub message: String,
    pub user_id: Option<String>,
    pub execution_id: Option<String>,
}

/// Alert manager
pub struct AlertManager {
    config: AlertConfig,
    client: Client,
}

impl AlertManager {
    pub fn new(config: AlertConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());

        if config.enabled {
            info!("Alert system initialized");
            if config.slack_webhook_url.is_some() {
                info!("Slack alerts enabled");
            }
            if config.pagerduty_key.is_some() {
                info!("PagerDuty alerts enabled");
            }
        }

        Self { config, client }
    }

    /// Send an alert
    pub async fn send(&self, alert: &Alert) {
        if !self.config.enabled {
            return;
        }

        // Send to Slack
        if let Some(ref webhook_url) = self.config.slack_webhook_url {
            if let Err(e) = self.send_slack(webhook_url, alert).await {
                error!(error = %e, "Failed to send Slack alert");
            }
        }

        // Send to PagerDuty
        if let Some(ref integration_key) = self.config.pagerduty_key {
            if let Err(e) = self.send_pagerduty(integration_key, alert).await {
                error!(error = %e, "Failed to send PagerDuty alert");
            }
        }

        // Send email (if configured)
        if self.config.email.is_some() {
            // Email sending would go here
            debug!("Email alerting not yet implemented");
        }
    }

    /// Send Slack notification
    async fn send_slack(&self, webhook_url: &str, alert: &Alert) -> Result<()> {
        let color = match alert.severity {
            AlertSeverity::Info => "#36a64f",
            AlertSeverity::Warning => "#ff9900",
            AlertSeverity::Error => "#ff0000",
            AlertSeverity::Critical => "#990000",
        };

        let mut fields = vec![];

        if let Some(ref user_id) = alert.user_id {
            fields.push(json!({
                "title": "User",
                "value": user_id,
                "short": true
            }));
        }

        if let Some(ref exec_id) = alert.execution_id {
            fields.push(json!({
                "title": "Execution ID",
                "value": exec_id,
                "short": true
            }));
        }

        let payload = json!({
            "attachments": [{
                "color": color,
                "title": alert.title,
                "text": alert.message,
                "fields": fields,
                "footer": "CAGE Orchestrator",
                "ts": chrono::Utc::now().timestamp()
            }]
        });

        let response = self.client.post(webhook_url).json(&payload).send().await?;

        if !response.status().is_success() {
            warn!(status = %response.status(), "Slack webhook returned error");
        }

        Ok(())
    }

    /// Send PagerDuty event
    async fn send_pagerduty(&self, integration_key: &str, alert: &Alert) -> Result<()> {
        let severity = match alert.severity {
            AlertSeverity::Info => "info",
            AlertSeverity::Warning => "warning",
            AlertSeverity::Error => "error",
            AlertSeverity::Critical => "critical",
        };

        let payload = json!({
            "routing_key": integration_key,
            "event_action": "trigger",
            "payload": {
                "summary": alert.title,
                "severity": severity,
                "source": "cage-orchestrator",
                "custom_details": {
                    "message": alert.message,
                    "user_id": alert.user_id,
                    "execution_id": alert.execution_id,
                }
            }
        });

        let response = self
            .client
            .post("https://events.pagerduty.com/v2/enqueue")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            warn!(status = %response.status(), "PagerDuty API returned error");
        }

        Ok(())
    }

    /// Create alert for execution error
    pub fn execution_error(user_id: String, execution_id: String, error: String) -> Alert {
        Alert {
            severity: AlertSeverity::Error,
            title: "Code Execution Failed".to_string(),
            message: format!("Execution failed: {}", error),
            user_id: Some(user_id),
            execution_id: Some(execution_id),
        }
    }

    /// Create alert for security event
    pub fn security_event(user_id: String, event_type: String, details: String) -> Alert {
        Alert {
            severity: AlertSeverity::Warning,
            title: format!("Security Event: {}", event_type),
            message: details,
            user_id: Some(user_id),
            execution_id: None,
        }
    }

    /// Create alert for resource threshold
    pub fn resource_threshold(user_id: String, resource: String, value: f64, limit: f64) -> Alert {
        Alert {
            severity: AlertSeverity::Warning,
            title: format!("Resource Threshold Exceeded: {}", resource),
            message: format!("{} usage: {:.1}% (limit: {:.1}%)", resource, value, limit),
            user_id: Some(user_id),
            execution_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_config_default() {
        let config = AlertConfig::default();
        assert!(!config.enabled);
        assert!(config.alert_on_errors);
    }

    #[test]
    fn test_create_alerts() {
        let alert = AlertManager::execution_error(
            "user1".to_string(),
            "exec-123".to_string(),
            "Timeout".to_string(),
        );
        assert_eq!(alert.severity, AlertSeverity::Error);
        assert!(alert.title.contains("Failed"));

        let sec_alert = AlertManager::security_event(
            "user2".to_string(),
            "NETWORK_BLOCKED".to_string(),
            "Attempted network access".to_string(),
        );
        assert_eq!(sec_alert.severity, AlertSeverity::Warning);
    }
}
