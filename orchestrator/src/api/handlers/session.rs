//! Session management handlers

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::api::{ApiError, UserAuth};
use crate::models::{CreateSessionRequest, SessionInfo};
use crate::AppState;

/// Get current session info
pub async fn get_session(
    State(state): State<Arc<AppState>>,
    auth: UserAuth,
) -> Result<Json<SessionInfo>, ApiError> {
    let session = state
        .container_manager
        .get_session(&auth.user_id)
        .await
        .ok_or_else(|| ApiError::NotFound("No active session".into()))?;

    Ok(Json(session))
}

/// Create or restart session
pub async fn create_session(
    State(state): State<Arc<AppState>>,
    auth: UserAuth,
    Json(request): Json<CreateSessionRequest>,
) -> Result<(StatusCode, Json<SessionInfo>), ApiError> {
    // Check if session already exists
    let existing = state.container_manager.get_session(&auth.user_id).await;

    // If reset requested, terminate existing session first
    if request.reset_workspace {
        let _ = state
            .container_manager
            .terminate_session(&auth.user_id, true)
            .await;
    }

    // Create new session
    let _session = state
        .container_manager
        .get_or_create_session(&auth.user_id)
        .await?;

    let session_info = state
        .container_manager
        .get_session(&auth.user_id)
        .await
        .ok_or_else(|| ApiError::Internal("Failed to get created session".into()))?;

    let status = if existing.is_some() && !request.reset_workspace {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };

    Ok((status, Json(session_info)))
}

/// Query parameters for session termination
#[derive(Debug, Deserialize)]
pub struct TerminateQuery {
    #[serde(default)]
    pub purge_data: bool,
}

/// Terminate session
pub async fn terminate_session(
    State(state): State<Arc<AppState>>,
    auth: UserAuth,
    Query(query): Query<TerminateQuery>,
) -> Result<StatusCode, ApiError> {
    // Check if session exists
    if state.container_manager.get_session(&auth.user_id).await.is_none() {
        return Err(ApiError::NotFound("No active session".into()));
    }

    state
        .container_manager
        .terminate_session(&auth.user_id, query.purge_data)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
