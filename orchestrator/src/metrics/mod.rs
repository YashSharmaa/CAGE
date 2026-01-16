//! Prometheus metrics export
//!
//! Provides metrics endpoint for monitoring and alerting

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse};
use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use once_cell::sync::Lazy;

use crate::AppState;

/// Prometheus metrics recorder
static METRICS_HANDLE: Lazy<PrometheusHandle> = Lazy::new(|| {
    PrometheusBuilder::new()
        .set_buckets_for_metric(
            Matcher::Full("cage_execution_duration_seconds".to_string()),
            &[0.01, 0.05, 0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0],
        )
        .unwrap()
        .install_recorder()
        .unwrap()
});

/// Initialize metrics system
pub fn init_metrics() {
    // Force initialization
    Lazy::force(&METRICS_HANDLE);
}

/// Record an execution
pub fn record_execution(user_id: &str, language: &str, duration_secs: f64, success: bool) {
    counter!("cage_executions_total", "user" => user_id.to_string(), "language" => language.to_string()).increment(1);

    if success {
        counter!("cage_executions_success_total", "user" => user_id.to_string(), "language" => language.to_string()).increment(1);
    } else {
        counter!("cage_executions_error_total", "user" => user_id.to_string(), "language" => language.to_string()).increment(1);
    }

    histogram!("cage_execution_duration_seconds", "user" => user_id.to_string(), "language" => language.to_string()).record(duration_secs);
}

/// Record session metrics
pub fn record_session_created(user_id: &str) {
    counter!("cage_sessions_created_total", "user" => user_id.to_string()).increment(1);
    gauge!("cage_active_sessions").increment(1.0);
}

/// Record session termination
pub fn record_session_terminated(user_id: &str) {
    counter!("cage_sessions_terminated_total", "user" => user_id.to_string()).increment(1);
    gauge!("cage_active_sessions").decrement(1.0);
}

/// Record file operations
pub fn record_file_operation(user_id: &str, operation: &str, size_bytes: u64) {
    counter!("cage_file_operations_total", "user" => user_id.to_string(), "operation" => operation.to_string()).increment(1);
    counter!("cage_file_bytes_total", "user" => user_id.to_string(), "operation" => operation.to_string()).increment(size_bytes);
}

/// Record resource usage
pub fn record_resource_usage(cpu_percent: f64, memory_mb: f64, disk_mb: f64) {
    gauge!("cage_cpu_usage_percent").set(cpu_percent);
    gauge!("cage_memory_usage_mb").set(memory_mb);
    gauge!("cage_disk_usage_mb").set(disk_mb);
}

/// Record security events
pub fn record_security_event(event_type: &str, severity: &str) {
    counter!("cage_security_events_total", "type" => event_type.to_string(), "severity" => severity.to_string()).increment(1);
}

/// Prometheus metrics endpoint handler
pub async fn metrics_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Update gauges with current values
    gauge!("cage_active_sessions").set(state.container_manager.active_session_count().await as f64);
    gauge!("cage_total_executions").set(state.total_executions.load(std::sync::atomic::Ordering::Relaxed) as f64);
    gauge!("cage_total_errors").set(state.total_errors.load(std::sync::atomic::Ordering::Relaxed) as f64);

    // Render metrics
    let metrics = METRICS_HANDLE.render();

    (
        StatusCode::OK,
        [("Content-Type", "text/plain; version=0.0.4")],
        metrics,
    )
}
