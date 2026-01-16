//! Data models for CAGE Orchestrator
//!
//! This module defines all request/response types and internal data structures.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::{NetworkPolicy, ResourceLimits};

// ============================================================================
// Execution Models
// ============================================================================

/// Request to execute code in a sandbox
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteRequest {
    /// Programming language
    #[serde(default = "default_language")]
    pub language: Language,

    /// Code to execute
    pub code: String,

    /// Maximum execution time in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,

    /// Working directory within sandbox
    #[serde(default)]
    pub working_dir: Option<String>,

    /// Additional environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Use persistent interpreter mode (maintains state between executions)
    #[serde(default)]
    pub persistent: bool,
}

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    Python,
    Javascript,
    Bash,
    R,
    Julia,
    Typescript,
    Ruby,
    Go,
    Wasm,
}

impl Language {
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Python => "python",
            Language::Javascript => "javascript",
            Language::Bash => "bash",
            Language::R => "r",
            Language::Julia => "julia",
            Language::Typescript => "typescript",
            Language::Ruby => "ruby",
            Language::Go => "go",
            Language::Wasm => "wasm",
        }
    }

    pub fn command(&self) -> &'static str {
        match self {
            Language::Python => "python",
            Language::Javascript => "node",
            Language::Bash => "bash",
            Language::R => "Rscript",
            Language::Julia => "julia",
            Language::Typescript => "deno",
            Language::Ruby => "ruby",
            Language::Go => "go",
            Language::Wasm => "wasmtime",
        }
    }

    pub fn file_extension(&self) -> &'static str {
        match self {
            Language::Python => "py",
            Language::Javascript => "js",
            Language::Bash => "sh",
            Language::R => "r",
            Language::Julia => "jl",
            Language::Typescript => "ts",
            Language::Ruby => "rb",
            Language::Go => "go",
            Language::Wasm => "wasm",
        }
    }

    pub fn container_image(&self) -> &'static str {
        match self {
            Language::Python => "cage-sandbox:latest",
            Language::Javascript => "cage-sandbox:latest",
            Language::Bash => "cage-sandbox:latest",
            Language::R => "cage-sandbox-r:latest",
            Language::Julia => "cage-sandbox-julia:latest",
            Language::Typescript => "cage-sandbox-typescript:latest",
            Language::Ruby => "cage-sandbox-ruby:latest",
            Language::Go => "cage-sandbox-go:latest",
            Language::Wasm => "cage-sandbox-wasm:latest",
        }
    }
}

/// Response from code execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteResponse {
    /// Unique execution ID
    pub execution_id: Uuid,

    /// Execution status
    pub status: ExecutionStatus,

    /// Standard output
    #[serde(default)]
    pub stdout: String,

    /// Standard error
    #[serde(default)]
    pub stderr: String,

    /// Process exit code
    #[serde(default)]
    pub exit_code: Option<i32>,

    /// Execution duration in milliseconds
    #[serde(default)]
    pub duration_ms: u64,

    /// Files created during execution
    #[serde(default)]
    pub files_created: Vec<String>,

    /// Resource usage during execution
    #[serde(default)]
    pub resource_usage: Option<ResourceUsage>,
}

/// Execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    Success,
    Error,
    Timeout,
    Killed,
}

/// Resource usage during execution
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Memory usage in MB
    pub memory_mb: f64,

    /// CPU percentage
    pub cpu_percent: f64,

    /// Disk usage in MB
    pub disk_mb: f64,

    /// Number of processes
    pub pids: u32,
}

// ============================================================================
// Async Job Models
// ============================================================================

/// Response for async job submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncJobResponse {
    pub job_id: Uuid,
    pub status: JobStatus,
    pub poll_url: String,
}

/// Job status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Timeout,
}

/// Full job status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatusResponse {
    pub job_id: Uuid,
    pub status: JobStatus,
    pub result: Option<ExecuteResponse>,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

// ============================================================================
// File Models
// ============================================================================

/// File information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub file_type: FileType,
    pub size_bytes: u64,
    pub modified_at: DateTime<Utc>,
    #[serde(default)]
    pub permissions: Option<String>,
}

/// File type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    File,
    Directory,
}

/// File list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileListResponse {
    pub path: String,
    pub files: Vec<FileInfo>,
    pub total_size_bytes: u64,
}

/// File upload request (JSON variant)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileUploadRequest {
    pub filename: String,
    #[serde(default = "default_path")]
    pub path: String,
    /// Base64 encoded content
    pub content: String,
    #[serde(default = "default_true")]
    pub overwrite: bool,
}

/// File upload response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileUploadResponse {
    pub path: String,
    pub size_bytes: u64,
    pub checksum: String,
}

// ============================================================================
// Session Models
// ============================================================================

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: Uuid,
    pub user_id: String,
    #[serde(default)]
    pub container_id: Option<String>,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub resource_limits: ResourceLimits,
    #[serde(default)]
    pub current_usage: Option<ResourceUsage>,
    pub network_policy: NetworkPolicy,
}

/// Session status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Running,
    Stopped,
    Creating,
    Error,
}

/// Create session request
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateSessionRequest {
    #[serde(default = "default_language")]
    pub language: Language,
    #[serde(default)]
    pub reset_workspace: bool,
}

// ============================================================================
// Admin Models
// ============================================================================

/// Admin session list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminSessionListResponse {
    pub sessions: Vec<AdminSessionSummary>,
    pub total: u64,
    pub offset: u64,
    pub limit: u64,
}

/// Admin session summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminSessionSummary {
    pub user_id: String,
    #[serde(default)]
    pub container_id: Option<String>,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub cpu_percent: f64,
    pub memory_mb: f64,
    pub execution_count: u64,
    pub error_count: u64,
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// Admin session detail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminSessionDetail {
    #[serde(flatten)]
    pub session: SessionInfo,
    pub execution_history: Vec<ExecutionSummary>,
    pub files: Vec<FileInfo>,
    pub security_events: Vec<SecurityEvent>,
}

/// Execution summary for history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub execution_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub language: Language,
    pub code_hash: String,
    pub status: ExecutionStatus,
    pub duration_ms: u64,
    pub exit_code: Option<i32>,
}

/// Security event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: SecurityEventType,
    pub severity: Severity,
    pub details: String,
    #[serde(default)]
    pub source_ip: Option<String>,
}

/// Security event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityEventType {
    NetworkBlocked,
    ResourceExceeded,
    SuspiciousSyscall,
    FileAccessDenied,
}

/// Event severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

// ============================================================================
// System Models
// ============================================================================

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub version: String,
    pub uptime_seconds: u64,
    pub active_sessions: u64,
    #[serde(default)]
    pub podman_version: Option<String>,
}

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// System statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStats {
    pub uptime_seconds: u64,
    pub active_sessions: u64,
    pub total_executions: u64,
    pub executions_last_hour: u64,
    pub average_execution_time_ms: f64,
    pub total_errors: u64,
    pub errors_last_hour: u64,
    pub security_events_last_hour: u64,
    pub resource_usage: SystemResourceUsage,
}

/// System-wide resource usage
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemResourceUsage {
    pub total_cpu_percent: f64,
    pub total_memory_mb: f64,
    pub total_disk_mb: f64,
}

/// Log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub execution_id: Option<String>,
    #[serde(default)]
    pub container_id: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Logs response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogsResponse {
    pub logs: Vec<LogEntry>,
    pub has_more: bool,
}

// ============================================================================
// Error Models
// ============================================================================

/// Standard error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(default)]
    pub details: Option<serde_json::Value>,
    #[serde(default)]
    pub request_id: Option<Uuid>,
}

impl ErrorResponse {
    pub fn new(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
            details: None,
            request_id: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn with_request_id(mut self, request_id: Uuid) -> Self {
        self.request_id = Some(request_id);
        self
    }
}

// ============================================================================
// User Models
// ============================================================================

/// User configuration (for admin API)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfigResponse {
    pub user_id: String,
    pub enabled: bool,
    pub resource_limits: ResourceLimits,
    pub network_policy: NetworkPolicy,
    pub allowed_languages: Vec<String>,
    pub gpu_enabled: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// User list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserListResponse {
    pub users: Vec<UserConfigResponse>,
}

// ============================================================================
// Default Functions
// ============================================================================

fn default_language() -> Language {
    Language::Python
}

fn default_timeout() -> u64 {
    30
}

fn default_path() -> String {
    "/".to_string()
}

fn default_true() -> bool {
    true
}
