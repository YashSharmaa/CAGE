//! CAGE Orchestrator library
//!
//! This library provides secure sandboxed code execution for LLM agents.

use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

pub mod alerts;
pub mod analysis;
pub mod api;
pub mod audit;
pub mod config;
pub mod container;
pub mod distributed;
pub mod gvisor;
pub mod jaeger;
pub mod jobs;
pub mod logging;
pub mod mcp;
pub mod metrics;
pub mod models;
pub mod network;
pub mod opa;
pub mod packages;
pub mod persistent;
pub mod proxy;
pub mod ratelimit;
pub mod reload;
pub mod replay;
pub mod signing;
pub mod tty;
pub mod usermgmt;
pub mod vault;

/// Application state shared across all handlers
pub struct AppState {
    pub config: config::AppConfig,
    pub container_manager: container::ContainerManager,
    pub job_queue: jobs::JobQueue,
    pub kernel_manager: Arc<persistent::PersistentKernelManager>,
    pub total_executions: AtomicU64,
    pub total_errors: AtomicU64,
    pub executions_last_hour: Arc<RwLock<Vec<DateTime<Utc>>>>,
    pub errors_last_hour: Arc<RwLock<Vec<DateTime<Utc>>>>,
    pub rate_limiter: Arc<ratelimit::RateLimiter>,
    pub code_analyzer: Arc<analysis::CodeAnalyzer>,
    pub audit_logger: Arc<audit::AuditLogger>,
    pub tty_manager: Arc<tty::TtyManager>,
    pub package_manager: Arc<packages::PackageManager>,
    pub user_manager: Arc<usermgmt::UserManager>,
    pub replay_manager: Arc<replay::ReplayManager>,
    pub alert_manager: Arc<alerts::AlertManager>,
    pub opa_evaluator: Arc<opa::OpaEvaluator>,
    pub vault_client: Option<Arc<vault::VaultClient>>,
    pub signature_verifier: Arc<signing::SignatureVerifier>,
}
