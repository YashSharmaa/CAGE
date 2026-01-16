//! Execution replay API handlers

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::api::{AdminAuth, ApiError, UserAuth};
use crate::replay::StoredExecution;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct ListReplaysQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub user_id: Option<String>,
}

fn default_limit() -> usize {
    100
}

/// List stored executions (admin or own user)
pub async fn list_replays(
    State(state): State<Arc<AppState>>,
    auth: UserAuth,
    Query(query): Query<ListReplaysQuery>,
) -> Result<Json<Vec<StoredExecution>>, ApiError> {
    let executions = if auth.is_admin {
        // Admin can see all or filter by user
        if let Some(ref user_id) = query.user_id {
            state.replay_manager.list_user_executions(user_id).await
        } else {
            state.replay_manager.list_all().await
        }
    } else {
        // Regular users see only their own
        state.replay_manager.list_user_executions(&auth.user_id).await
    };

    let limited: Vec<_> = executions.into_iter().take(query.limit).collect();

    Ok(Json(limited))
}

/// Get a specific stored execution
pub async fn get_replay(
    State(state): State<Arc<AppState>>,
    auth: UserAuth,
    Path(execution_id): Path<Uuid>,
) -> Result<Json<StoredExecution>, ApiError> {
    let stored = state
        .replay_manager
        .get(&execution_id)
        .await
        .ok_or_else(|| ApiError::NotFound(format!("Execution {} not found", execution_id)))?;

    // Check authorization
    if !auth.is_admin && stored.user_id != auth.user_id {
        return Err(ApiError::Forbidden);
    }

    Ok(Json(stored))
}

/// Replay a stored execution
pub async fn replay_execution(
    State(state): State<Arc<AppState>>,
    auth: UserAuth,
    Path(execution_id): Path<Uuid>,
) -> Result<Json<crate::models::ExecuteResponse>, ApiError> {
    let replay_request = state
        .replay_manager
        .replay(&execution_id)
        .await
        .ok_or_else(|| ApiError::NotFound(format!("Execution {} not found", execution_id)))?;

    // Get stored execution to check ownership
    let stored = state.replay_manager.get(&execution_id).await.unwrap();

    // Check authorization
    if !auth.is_admin && stored.user_id != auth.user_id {
        return Err(ApiError::Forbidden);
    }

    // Execute the same code again
    let response = state
        .container_manager
        .execute_code(&stored.user_id, replay_request)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(response))
}

/// Delete a stored execution
#[allow(dead_code)]
pub async fn delete_replay(
    State(_state): State<Arc<AppState>>,
    _admin: AdminAuth,
    Path(_execution_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    // TODO: Implement deletion from storage
    Ok(StatusCode::NO_CONTENT)
}
