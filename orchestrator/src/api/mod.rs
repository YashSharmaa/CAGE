//! API module for CAGE Orchestrator
//!
//! Provides REST API endpoints for:
//! - Code execution
//! - File management
//! - Session management
//! - Admin operations
//! - Health checks

mod auth;
mod error;
mod handlers;

use std::sync::Arc;

use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::mcp::handler::mcp_websocket_handler;

use crate::AppState;

pub use auth::{AdminAuth, UserAuth};
pub use error::ApiError;

/// Create the main application router
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Health check (no auth)
        .route("/health", get(handlers::health::health_check))
        // Prometheus metrics (no auth for scraping)
        .route("/metrics", get(crate::metrics::metrics_handler))
        // MCP WebSocket endpoint
        .route("/mcp", get(mcp_websocket_handler))
        // API v1 routes
        .nest("/api/v1", api_v1_routes(state.clone()))
        .with_state(state)
}

/// API v1 routes
fn api_v1_routes(_state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        // Execution endpoints
        .route("/execute", post(handlers::execute::execute_code))
        .route("/execute/async", post(handlers::execute::execute_async))
        .route("/jobs/:job_id", get(handlers::execute::get_job_status))
        // File endpoints
        .route("/files", get(handlers::files::list_files))
        .route("/files", post(handlers::files::upload_file))
        .route("/files/*filepath", get(handlers::files::download_file))
        .route("/files/*filepath", delete(handlers::files::delete_file))
        // Session endpoints
        .route("/session", get(handlers::session::get_session))
        .route("/session", post(handlers::session::create_session))
        .route("/session", delete(handlers::session::terminate_session))
        // Package management endpoints
        .route("/packages/install", post(handlers::packages::install_package))
        .route("/packages/installed", get(handlers::packages::list_installed))
        .route("/packages/allowed/:language", get(handlers::packages::list_allowed))
        // Replay endpoints
        .route("/replays", get(handlers::replay::list_replays))
        .route("/replays/:execution_id", get(handlers::replay::get_replay))
        .route("/replays/:execution_id/replay", post(handlers::replay::replay_execution))
        // Admin endpoints (separate auth)
        .nest("/admin", admin_routes())
}

/// Admin API routes
fn admin_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/sessions", get(handlers::admin::list_sessions))
        .route("/sessions/:user_id", get(handlers::admin::get_session_detail))
        .route(
            "/sessions/:user_id",
            delete(handlers::admin::terminate_session),
        )
        .route("/logs", get(handlers::admin::get_logs))
        .route("/logs/stream", get(handlers::logs::log_stream_handler))
        .route("/stats", get(handlers::admin::get_stats))
        .route("/users", get(handlers::admin::list_users))
        .route("/users", post(handlers::admin::upsert_user))
        .route("/users/:user_id", delete(handlers::admin::delete_user))
}
