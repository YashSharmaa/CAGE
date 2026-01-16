//! CAGE Orchestrator - Secure sandboxed code execution for LLM agents
//!
//! This is the main entry point for the CAGE orchestrator service.
//! It manages Podman containers and provides a REST API for code execution.

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use tokio::signal;
use axum::http::HeaderName;
use tower_http::cors::{Any, CorsLayer};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

use cage_orchestrator::AppState;
use cage_orchestrator::api::create_router;
use cage_orchestrator::config::AppConfig;
use cage_orchestrator::container::ContainerManager;
use cage_orchestrator::jobs::JobQueue;
use cage_orchestrator::logging::init_logging;
use cage_orchestrator::persistent::PersistentKernelManager;

use std::sync::atomic::AtomicU64;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = AppConfig::load()?;

    // Initialize logging
    init_logging(&config.log_level)?;

    info!(
        version = env!("CARGO_PKG_VERSION"),
        "Starting CAGE Orchestrator"
    );

    // Initialize metrics system
    cage_orchestrator::metrics::init_metrics();
    info!("Prometheus metrics initialized");

    // Initialize persistent kernel manager
    let kernel_manager = Arc::new(PersistentKernelManager::new());

    // Initialize rate limiter (60 requests per minute per user)
    let rate_limiter = Arc::new(cage_orchestrator::ratelimit::RateLimiter::new(60.0));
    info!("Rate limiter initialized (60 req/min per user)");

    // Initialize code analyzer (warning mode, not blocking)
    let code_analyzer = Arc::new(cage_orchestrator::analysis::CodeAnalyzer::new(false));
    info!("Static code analyzer initialized");

    // Initialize audit logger (JSON format)
    let audit_logger = Arc::new(cage_orchestrator::audit::AuditLogger::new(cage_orchestrator::audit::SiemFormat::Json));
    info!("Audit logger initialized (JSON format)");

    // Initialize TTY manager
    let tty_manager = Arc::new(cage_orchestrator::tty::TtyManager::new());
    info!("TTY manager initialized");

    // Initialize package manager
    let package_manager = Arc::new(cage_orchestrator::packages::PackageManager::new(config.packages.clone()));
    info!("Package manager initialized");

    // Initialize user manager
    let user_manager = Arc::new(cage_orchestrator::usermgmt::UserManager::new(&config.data_dir).await?);
    info!("User manager initialized");

    // Initialize replay manager
    let replay_manager = Arc::new(cage_orchestrator::replay::ReplayManager::new(&config.data_dir, 1000).await?);
    info!("Replay manager initialized (max 1000 executions)");

    // Initialize alert manager
    let alert_manager = Arc::new(cage_orchestrator::alerts::AlertManager::new(config.alerts.clone()));
    info!("Alert manager initialized");

    // Initialize OPA evaluator
    let opa_evaluator = Arc::new(cage_orchestrator::opa::OpaEvaluator::new(config.opa.clone()));
    if config.opa.enabled {
        if opa_evaluator.health_check().await {
            info!(server = %config.opa.server_url, "OPA policy engine connected");
        } else {
            warn!("OPA enabled but server not reachable");
        }
    }

    // Initialize Vault client if enabled
    let vault_client = if config.vault.enabled {
        let client = Arc::new(cage_orchestrator::vault::VaultClient::new(config.vault.clone()));
        if client.health_check().await {
            info!(address = %config.vault.address, "Vault connected");
        } else {
            warn!("Vault enabled but server not reachable");
        }
        Some(client)
    } else {
        None
    };

    // Initialize signature verifier
    let signature_verifier = Arc::new(cage_orchestrator::signing::SignatureVerifier::new(config.signing.clone()));
    if config.signing.enabled {
        info!(
            require_signature = config.signing.require_signature,
            trusted_keys = config.signing.trusted_keys.len(),
            "Code signing verification enabled"
        );
    }

    // Initialize Jaeger tracer if enabled
    let _jaeger_tracer = if config.jaeger.enabled {
        let tracer = Arc::new(cage_orchestrator::jaeger::JaegerTracer::new(config.jaeger.clone()).await);
        if tracer.is_active() {
            info!(
                endpoint = %config.jaeger.agent_endpoint,
                service = %config.jaeger.service_name,
                sampling_rate = config.jaeger.sampling_rate,
                "Jaeger tracing active"
            );
        }
        Some(tracer)
    } else {
        None
    };

    // Initialize gVisor runtime if enabled
    let gvisor_runtime = if config.gvisor.enabled {
        let runtime = Arc::new(cage_orchestrator::gvisor::GVisorRuntime::new(config.gvisor.clone()).await);
        if runtime.is_active() {
            info!(
                platform = ?config.gvisor.platform,
                overhead = runtime.performance_overhead(),
                "gVisor runtime active"
            );
        }
        Some(runtime)
    } else {
        None
    };

    // Initialize distributed state manager if enabled
    let distributed_manager = if config.distributed.enabled {
        let manager = Arc::new(
            cage_orchestrator::distributed::DistributedStateManager::new(config.distributed.clone())
                .await?
        );

        // Register this node
        let addr = format!("{}:{}", config.host, config.port);
        manager.register_node(addr).await?;

        info!(
            node_id = %manager.node_id(),
            redis_url = %config.distributed.redis_url,
            "Distributed mode enabled"
        );

        Some(manager)
    } else {
        None
    };

    // Initialize container manager
    let container_manager = ContainerManager::new(config.clone(), kernel_manager.clone(), gvisor_runtime).await?;

    // Create shared application state
    let state = Arc::new(AppState {
        config: config.clone(),
        container_manager,
        job_queue: JobQueue::new(),
        kernel_manager: kernel_manager.clone(),
        total_executions: AtomicU64::new(0),
        total_errors: AtomicU64::new(0),
        executions_last_hour: Arc::new(RwLock::new(Vec::new())),
        errors_last_hour: Arc::new(RwLock::new(Vec::new())),
        rate_limiter,
        code_analyzer,
        audit_logger,
        tty_manager,
        package_manager,
        user_manager,
        replay_manager,
        alert_manager,
        opa_evaluator,
        vault_client,
        signature_verifier,
    });

    // Start background stats collection task
    let stats_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            let _ = stats_state.container_manager.update_all_stats().await;
        }
    });

    // Start async job worker
    let job_state = state.clone();
    tokio::spawn(async move {
        loop {
            if let Some((job_id, user_id, request)) = job_state.job_queue.get_next_queued().await {
                let result = job_state.container_manager.execute_code(&user_id, request).await;
                match result {
                    Ok(response) => job_state.job_queue.complete(&job_id, response).await,
                    Err(e) => {
                        let error_response = cage_orchestrator::models::ExecuteResponse {
                            execution_id: job_id,
                            status: cage_orchestrator::models::ExecutionStatus::Error,
                            stdout: String::new(),
                            stderr: e.to_string(),
                            exit_code: None,
                            duration_ms: 0,
                            files_created: vec![],
                            resource_usage: None,
                        };
                        job_state.job_queue.complete(&job_id, error_response).await;
                    }
                }
            } else {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        }
    });

    // Clean up old jobs periodically
    let cleanup_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            cleanup_state.job_queue.cleanup_old_jobs().await;
            cleanup_state.rate_limiter.cleanup_old_buckets().await;
        }
    });

    // Start distributed heartbeat task if enabled
    if let Some(manager) = distributed_manager {
        let heartbeat_state = state.clone();
        manager.start_heartbeat_task(move || {
            let active_sessions = futures::executor::block_on(
                heartbeat_state.container_manager.active_session_count()
            );
            let total_executions = heartbeat_state.total_executions.load(std::sync::atomic::Ordering::Relaxed);
            (active_sessions, total_executions)
        }).await;
    }

    // Build the router with all routes and middleware
    let x_request_id = HeaderName::from_static("x-request-id");
    let app = create_router(state.clone())
        .layer(TraceLayer::new_for_http())
        .layer(SetRequestIdLayer::new(x_request_id.clone(), MakeRequestUuid))
        .layer(PropagateRequestIdLayer::new(x_request_id))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    // Bind to address
    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    info!(%addr, "Listening on");

    // Create the server
    let listener = tokio::net::TcpListener::bind(addr).await?;

    // Start server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Shutting down...");

    // Cleanup: stop all containers if configured to do so
    if config.stop_containers_on_shutdown {
        warn!("Stopping all managed containers...");
        // state.container_manager.stop_all().await?;
    }

    info!("CAGE Orchestrator stopped");
    Ok(())
}

/// Handle shutdown signals gracefully
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
