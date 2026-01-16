//! Code execution handlers

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::api::{ApiError, UserAuth};
use crate::models::{AsyncJobResponse, ExecuteRequest, ExecuteResponse, JobStatus, JobStatusResponse};
use crate::AppState;

/// Execute code synchronously
pub async fn execute_code(
    State(state): State<Arc<AppState>>,
    auth: UserAuth,
    Json(request): Json<ExecuteRequest>,
) -> Result<Json<ExecuteResponse>, ApiError> {
    use crate::audit::{create_execution_event, AuditOutcome};

    // Validate request
    if request.code.is_empty() {
        return Err(ApiError::BadRequest("Code cannot be empty".into()));
    }

    if request.code.len() > 1_000_000 {
        return Err(ApiError::PayloadTooLarge);
    }

    // Check user is enabled
    if !state.config.is_user_enabled(&auth.user_id) {
        return Err(ApiError::Forbidden);
    }

    // Rate limiting check
    if !state.rate_limiter.check_limit(&auth.user_id).await {
        // Audit: rate limit exceeded
        let event = create_execution_event(
            auth.user_id.clone(),
            "rate-limited".to_string(),
            request.language.as_str(),
            AuditOutcome::Denied,
            0,
        );
        state.audit_logger.log(&event);

        return Err(ApiError::TooManyRequests);
    }

    // Static code analysis
    let analysis = state.code_analyzer.analyze(&request.code, request.language);

    if analysis.blocked {
        // Audit: code blocked
        let event = create_execution_event(
            auth.user_id.clone(),
            "blocked".to_string(),
            request.language.as_str(),
            AuditOutcome::Denied,
            0,
        );
        state.audit_logger.log(&event);

        return Err(ApiError::BadRequest(format!(
            "Code blocked due to security analysis: {:?}",
            analysis.warnings
        )));
    }

    // Log analysis warnings
    for warning in &analysis.warnings {
        tracing::warn!(
            user_id = %auth.user_id,
            category = %warning.category,
            severity = ?warning.severity,
            "Code analysis warning"
        );
    }

    // Execute code
    let start_time = std::time::Instant::now();
    let response = state
        .container_manager
        .execute_code(&auth.user_id, request.clone())
        .await?;

    let duration_secs = start_time.elapsed().as_secs_f64();

    // Track statistics
    state.total_executions.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    state.executions_last_hour.write().await.push(chrono::Utc::now());

    let success = response.status == crate::models::ExecutionStatus::Success;

    if !success {
        state.total_errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        state.errors_last_hour.write().await.push(chrono::Utc::now());
    }

    // Clean old time-windowed data
    let cutoff = chrono::Utc::now() - chrono::Duration::hours(1);
    state.executions_last_hour.write().await.retain(|t| *t > cutoff);
    state.errors_last_hour.write().await.retain(|t| *t > cutoff);

    // Record Prometheus metrics
    crate::metrics::record_execution(
        &auth.user_id,
        request.language.as_str(),
        duration_secs,
        success,
    );

    // Audit log
    let outcome = if success {
        AuditOutcome::Success
    } else {
        AuditOutcome::Failure
    };
    let event = create_execution_event(
        auth.user_id.clone(),
        response.execution_id.to_string(),
        request.language.as_str(),
        outcome,
        response.duration_ms,
    );
    state.audit_logger.log(&event);

    // Store for replay
    let _ = state.replay_manager.store(auth.user_id, request, response.clone()).await;

    Ok(Json(response))
}

/// Execute code asynchronously (returns job ID)
pub async fn execute_async(
    State(state): State<Arc<AppState>>,
    auth: UserAuth,
    Json(request): Json<ExecuteRequest>,
) -> Result<Json<AsyncJobResponse>, ApiError> {
    // Validate request
    if request.code.is_empty() {
        return Err(ApiError::BadRequest("Code cannot be empty".into()));
    }

    if request.code.len() > 1_000_000 {
        return Err(ApiError::PayloadTooLarge);
    }

    // Check user is enabled
    if !state.config.is_user_enabled(&auth.user_id) {
        return Err(ApiError::Forbidden);
    }

    // Submit to job queue
    let job_id = state.job_queue.submit(auth.user_id, request).await;

    Ok(Json(AsyncJobResponse {
        job_id,
        status: JobStatus::Queued,
        poll_url: format!("/api/v1/jobs/{}", job_id),
    }))
}

/// Get async job status
pub async fn get_job_status(
    State(state): State<Arc<AppState>>,
    _auth: UserAuth,
    Path(job_id): Path<Uuid>,
) -> Result<Json<JobStatusResponse>, ApiError> {
    state
        .job_queue
        .get_status(&job_id)
        .await
        .ok_or_else(|| ApiError::NotFound(format!("Job {} not found", job_id)))
        .map(Json)
}
