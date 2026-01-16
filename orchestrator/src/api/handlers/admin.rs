//! Admin API handlers

use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use once_cell::sync::Lazy;
use serde::Deserialize;

use crate::api::{AdminAuth, ApiError};
use crate::models::{
    AdminSessionDetail, AdminSessionListResponse, LogEntry, LogLevel, LogsResponse,
    SessionStatus, SystemResourceUsage, SystemStats, UserConfigResponse, UserListResponse,
};
use crate::AppState;

/// Server start time for uptime calculation
static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

/// Query parameters for session listing
#[derive(Debug, Deserialize)]
pub struct ListSessionsQuery {
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
}

fn default_status() -> String {
    "all".to_string()
}

fn default_limit() -> u64 {
    100
}

/// List all sessions
pub async fn list_sessions(
    State(state): State<Arc<AppState>>,
    _admin: AdminAuth,
    Query(query): Query<ListSessionsQuery>,
) -> Result<Json<AdminSessionListResponse>, ApiError> {
    // Update stats for all sessions before returning
    let _ = state.container_manager.update_all_stats().await;

    let mut sessions = state.container_manager.list_all_sessions().await;

    // Filter by status
    if query.status != "all" {
        let target_status = match query.status.as_str() {
            "running" => Some(SessionStatus::Running),
            "stopped" => Some(SessionStatus::Stopped),
            _ => None,
        };

        if let Some(status) = target_status {
            sessions.retain(|s| s.status == status);
        }
    }

    let total = sessions.len() as u64;

    // Apply pagination
    let sessions: Vec<_> = sessions
        .into_iter()
        .skip(query.offset as usize)
        .take(query.limit as usize)
        .collect();

    Ok(Json(AdminSessionListResponse {
        sessions,
        total,
        offset: query.offset,
        limit: query.limit,
    }))
}

/// Get detailed session information
pub async fn get_session_detail(
    State(state): State<Arc<AppState>>,
    _admin: AdminAuth,
    Path(user_id): Path<String>,
) -> Result<Json<AdminSessionDetail>, ApiError> {
    let session = state
        .container_manager
        .get_session(&user_id)
        .await
        .ok_or_else(|| ApiError::NotFound(format!("Session for user {} not found", user_id)))?;

    // Get files in workspace
    let files = state
        .container_manager
        .list_files(&user_id, "/")
        .await
        .map(|r| r.files)
        .unwrap_or_default();

    // Get execution history and security events from session
    let session_handle = state.container_manager.get_session_handle(&user_id).await;
    let execution_history = if let Some(handle) = &session_handle {
        handle.get_execution_history().await
    } else {
        vec![]
    };
    let security_events = if let Some(handle) = session_handle {
        handle.get_security_events().await
    } else {
        vec![]
    };

    Ok(Json(AdminSessionDetail {
        session,
        execution_history,
        files,
        security_events,
    }))
}

/// Force terminate a session
pub async fn terminate_session(
    State(state): State<Arc<AppState>>,
    _admin: AdminAuth,
    Path(user_id): Path<String>,
    Query(query): Query<TerminateQuery>,
) -> Result<StatusCode, ApiError> {
    state
        .container_manager
        .terminate_session(&user_id, query.purge_data)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct TerminateQuery {
    #[serde(default)]
    pub purge_data: bool,
}

/// Query parameters for logs
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct LogsQuery {
    pub user_id: Option<String>,
    pub level: Option<String>,
    pub since: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default = "default_log_limit")]
    pub limit: u64,
}

fn default_log_limit() -> u64 {
    100
}

/// Get system logs
pub async fn get_logs(
    State(_state): State<Arc<AppState>>,
    _admin: AdminAuth,
    Query(query): Query<LogsQuery>,
) -> Result<Json<LogsResponse>, ApiError> {
    // Note: In production, logs would be read from a file or log aggregator
    // For now, return a sample message indicating logs are available via orchestrator stdout
    let sample_log = LogEntry {
        timestamp: chrono::Utc::now(),
        level: LogLevel::Info,
        message: "Log retrieval is functional. In production, configure log file path or use log aggregator.".to_string(),
        user_id: query.user_id,
        execution_id: None,
        container_id: None,
        metadata: std::collections::HashMap::new(),
    };

    Ok(Json(LogsResponse {
        logs: vec![sample_log],
        has_more: false,
    }))
}

/// Get system statistics
pub async fn get_stats(
    State(state): State<Arc<AppState>>,
    _admin: AdminAuth,
) -> Result<Json<SystemStats>, ApiError> {
    let uptime = START_TIME.elapsed().as_secs();
    let active_sessions = state.container_manager.active_session_count().await;

    // Get global execution stats
    let total_executions = state.total_executions.load(std::sync::atomic::Ordering::Relaxed);
    let total_errors = state.total_errors.load(std::sync::atomic::Ordering::Relaxed);

    // Get time-windowed stats
    let executions_last_hour = state.executions_last_hour.read().await.len() as u64;
    let errors_last_hour = state.errors_last_hour.read().await.len() as u64;

    Ok(Json(SystemStats {
        uptime_seconds: uptime,
        active_sessions,
        total_executions,
        executions_last_hour,
        average_execution_time_ms: 0.0, // TODO: Track execution times
        total_errors,
        errors_last_hour,
        security_events_last_hour: 0, // TODO: Implement security event tracking
        resource_usage: SystemResourceUsage::default(),
    }))
}

/// List configured users
pub async fn list_users(
    State(state): State<Arc<AppState>>,
    _admin: AdminAuth,
) -> Result<Json<UserListResponse>, ApiError> {
    // Get users from dynamic user manager
    let dynamic_users = state.user_manager.list_users().await;

    let users: Vec<UserConfigResponse> = dynamic_users
        .into_iter()
        .map(|config| UserConfigResponse {
            user_id: config.user_id.clone(),
            enabled: config.enabled,
            resource_limits: config
                .resource_limits
                .clone()
                .unwrap_or_else(|| state.config.default_limits.clone()),
            network_policy: config
                .network_policy
                .clone()
                .unwrap_or_else(|| state.config.default_network.clone()),
            allowed_languages: config.allowed_languages.clone(),
            gpu_enabled: config.gpu_enabled,
            created_at: None,
            updated_at: None,
        })
        .collect();

    Ok(Json(UserListResponse { users }))
}

/// Create or update a user
pub async fn upsert_user(
    State(state): State<Arc<AppState>>,
    _admin: AdminAuth,
    Json(user_req): Json<UserConfigResponse>,
) -> Result<(StatusCode, Json<UserConfigResponse>), ApiError> {
    // Validate user_id
    if user_req.user_id.is_empty() || user_req.user_id.contains(['/', '\\', '.', ' ']) {
        return Err(ApiError::BadRequest("Invalid user_id".into()));
    }

    // Convert UserConfigResponse to UserConfig
    let user_config = crate::config::UserConfig {
        user_id: user_req.user_id.clone(),
        api_key_hash: None, // API key hash should be set separately for security
        enabled: user_req.enabled,
        resource_limits: Some(user_req.resource_limits.clone()),
        network_policy: Some(user_req.network_policy.clone()),
        allowed_languages: user_req.allowed_languages.clone(),
        gpu_enabled: user_req.gpu_enabled,
    };

    let is_new = state
        .user_manager
        .upsert_user(user_config)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let status = if is_new {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };

    Ok((status, Json(user_req)))
}

/// Delete a user
pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    _admin: AdminAuth,
    Path(user_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let existed = state
        .user_manager
        .delete_user(&user_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    if !existed {
        return Err(ApiError::NotFound(format!("User {} not found", user_id)));
    }

    Ok(StatusCode::NO_CONTENT)
}
