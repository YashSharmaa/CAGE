//! Health check handler

use std::sync::Arc;
use std::time::Instant;

use axum::{extract::State, Json};
use once_cell::sync::Lazy;

use crate::models::{HealthResponse, HealthStatus};
use crate::AppState;

/// Server start time for uptime calculation
static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

/// Health check endpoint
pub async fn health_check(
    State(state): State<Arc<AppState>>,
) -> Json<HealthResponse> {
    let uptime = START_TIME.elapsed().as_secs();
    let active_sessions = state.container_manager.active_session_count().await;
    let podman_version = state.container_manager.podman_version().await;

    // Determine health status
    let status = if podman_version.is_some() {
        HealthStatus::Healthy
    } else {
        HealthStatus::Degraded
    };

    Json(HealthResponse {
        status,
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime,
        active_sessions,
        podman_version,
    })
}
