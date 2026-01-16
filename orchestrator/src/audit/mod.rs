//! Audit log export to SIEM systems
//!
//! Supports multiple formats: syslog (RFC 5424), CEF (Common Event Format), JSON

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

/// Audit event for SIEM export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub user_id: String,
    pub action: String,
    pub outcome: AuditOutcome,
    pub details: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_ip: Option<String>,
}

/// Audit event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditEventType {
    Authentication,
    Authorization,
    CodeExecution,
    FileOperation,
    SessionManagement,
    ResourceLimit,
    SecurityViolation,
    ConfigChange,
}

/// Audit outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditOutcome {
    Success,
    Failure,
    Denied,
}

/// SIEM export format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiemFormat {
    Syslog,
    Cef,
    Json,
}

/// Audit logger for SIEM export
pub struct AuditLogger {
    format: SiemFormat,
    app_name: String,
}

impl AuditLogger {
    pub fn new(format: SiemFormat) -> Self {
        Self {
            format,
            app_name: "CAGE".to_string(),
        }
    }

    /// Log an audit event
    pub fn log(&self, event: &AuditEvent) {
        match self.format {
            SiemFormat::Syslog => self.log_syslog(event),
            SiemFormat::Cef => self.log_cef(event),
            SiemFormat::Json => self.log_json(event),
        }
    }

    /// Log in syslog format (RFC 5424)
    fn log_syslog(&self, event: &AuditEvent) {
        let priority = match event.outcome {
            AuditOutcome::Success => 6, // Informational
            AuditOutcome::Failure => 4, // Warning
            AuditOutcome::Denied => 3,  // Error
        };

        let timestamp = event.timestamp.to_rfc3339();
        let hostname = "cage-orchestrator";
        let app_name = &self.app_name;

        let message = format!(
            "<{}>1 {} {} {} - - [user=\"{}\" action=\"{}\" outcome=\"{:?}\" event_type=\"{:?}\"] {}",
            priority,
            timestamp,
            hostname,
            app_name,
            event.user_id,
            event.action,
            event.outcome,
            event.event_type,
            self.format_details(&event.details)
        );

        info!(target: "audit", "{}", message);
    }

    /// Log in CEF format (Common Event Format)
    fn log_cef(&self, event: &AuditEvent) {
        // CEF:Version|Device Vendor|Device Product|Device Version|Signature ID|Name|Severity|Extension
        let severity = match event.outcome {
            AuditOutcome::Success => 2,
            AuditOutcome::Failure => 5,
            AuditOutcome::Denied => 8,
        };

        let signature_id = format!("{:?}", event.event_type);
        let name = &event.action;

        let mut extensions = vec![
            format!("suser={}", event.user_id),
            format!("outcome={:?}", event.outcome),
            format!("rt={}", event.timestamp.timestamp_millis()),
        ];

        if let Some(ref exec_id) = event.execution_id {
            extensions.push(format!("externalId={}", exec_id));
        }

        if let Some(ref container_id) = event.container_id {
            extensions.push(format!("cs1={}", container_id));
            extensions.push("cs1Label=ContainerID".to_string());
        }

        if let Some(ref source_ip) = event.source_ip {
            extensions.push(format!("src={}", source_ip));
        }

        for (k, v) in &event.details {
            extensions.push(format!("{}={}", k, v));
        }

        let cef_message = format!(
            "CEF:0|CAGE|Orchestrator|1.0.0|{}|{}|{}|{}",
            signature_id,
            name,
            severity,
            extensions.join(" ")
        );

        info!(target: "audit", "{}", cef_message);
    }

    /// Log in JSON format
    fn log_json(&self, event: &AuditEvent) {
        let json = serde_json::to_string(event).unwrap_or_else(|_| "{}".to_string());
        info!(target: "audit", "{}", json);
    }

    fn format_details(&self, details: &HashMap<String, String>) -> String {
        details
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new(SiemFormat::Json)
    }
}

/// Helper function to create audit events
pub fn create_execution_event(
    user_id: String,
    execution_id: String,
    language: &str,
    outcome: AuditOutcome,
    duration_ms: u64,
) -> AuditEvent {
    let mut details = HashMap::new();
    details.insert("language".to_string(), language.to_string());
    details.insert("duration_ms".to_string(), duration_ms.to_string());

    AuditEvent {
        timestamp: Utc::now(),
        event_type: AuditEventType::CodeExecution,
        user_id,
        action: "CODE_EXECUTION".to_string(),
        outcome,
        details,
        execution_id: Some(execution_id),
        container_id: None,
        source_ip: None,
    }
}

pub fn create_auth_event(
    user_id: String,
    outcome: AuditOutcome,
    source_ip: Option<String>,
) -> AuditEvent {
    AuditEvent {
        timestamp: Utc::now(),
        event_type: AuditEventType::Authentication,
        user_id,
        action: "AUTHENTICATION".to_string(),
        outcome,
        details: HashMap::new(),
        execution_id: None,
        container_id: None,
        source_ip,
    }
}

pub fn create_file_event(
    user_id: String,
    operation: String,
    filename: String,
    size_bytes: u64,
) -> AuditEvent {
    let mut details = HashMap::new();
    details.insert("filename".to_string(), filename);
    details.insert("size_bytes".to_string(), size_bytes.to_string());
    details.insert("operation".to_string(), operation.clone());

    AuditEvent {
        timestamp: Utc::now(),
        event_type: AuditEventType::FileOperation,
        user_id,
        action: operation,
        outcome: AuditOutcome::Success,
        details,
        execution_id: None,
        container_id: None,
        source_ip: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syslog_format() {
        let logger = AuditLogger::new(SiemFormat::Syslog);
        let event = create_execution_event(
            "test_user".to_string(),
            "exec-123".to_string(),
            "python",
            AuditOutcome::Success,
            150,
        );
        logger.log(&event);
        // Should not panic
    }

    #[test]
    fn test_cef_format() {
        let logger = AuditLogger::new(SiemFormat::Cef);
        let event = create_auth_event(
            "test_user".to_string(),
            AuditOutcome::Success,
            Some("192.168.1.100".to_string()),
        );
        logger.log(&event);
        // Should not panic
    }
}
