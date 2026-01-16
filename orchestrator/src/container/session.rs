//! Session management for container sandboxes

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::config::{NetworkPolicy, ResourceLimits};
use crate::models::{ExecutionSummary, ResourceUsage, SecurityEvent, SessionStatus};

/// Maximum number of execution history entries to keep per session
const MAX_EXECUTION_HISTORY: usize = 100;

/// Session state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Creating,
    Running,
    Stopped,
    Error,
}

impl From<SessionState> for SessionStatus {
    fn from(state: SessionState) -> Self {
        match state {
            SessionState::Creating => SessionStatus::Creating,
            SessionState::Running => SessionStatus::Running,
            SessionState::Stopped => SessionStatus::Stopped,
            SessionState::Error => SessionStatus::Error,
        }
    }
}

/// A user session with an associated container
#[derive(Debug)]
pub struct Session {
    /// Unique session ID
    pub session_id: Uuid,

    /// User ID that owns this session
    pub user_id: String,

    /// Podman container ID (once created)
    container_id: RwLock<Option<String>>,

    /// Container name
    pub container_name: String,

    /// Current state
    state: RwLock<SessionState>,

    /// Path to the user's workspace volume
    pub workspace_path: PathBuf,

    /// Resource limits for this session
    pub resource_limits: ResourceLimits,

    /// Network policy for this session
    pub network_policy: NetworkPolicy,

    /// When the session was created
    pub created_at: DateTime<Utc>,

    /// When the session was last active
    last_activity: RwLock<DateTime<Utc>>,

    /// Current resource usage
    current_usage: RwLock<ResourceUsage>,

    /// Execution counter
    execution_count: AtomicU64,

    /// Error counter
    error_count: AtomicU64,

    /// Execution history (ring buffer)
    execution_history: RwLock<VecDeque<ExecutionSummary>>,

    /// Security events
    security_events: RwLock<Vec<SecurityEvent>>,

    /// Lock for execution (only one at a time per session)
    execution_lock: tokio::sync::Mutex<()>,
}

impl Session {
    /// Create a new session
    pub fn new(
        user_id: String,
        workspace_path: PathBuf,
        resource_limits: ResourceLimits,
        network_policy: NetworkPolicy,
    ) -> Self {
        let session_id = Uuid::new_v4();
        let container_name = format!("cage_{}_{}", user_id, session_id.simple());

        Self {
            session_id,
            user_id,
            container_id: RwLock::new(None),
            container_name,
            state: RwLock::new(SessionState::Creating),
            workspace_path,
            resource_limits,
            network_policy,
            created_at: Utc::now(),
            last_activity: RwLock::new(Utc::now()),
            current_usage: RwLock::new(ResourceUsage::default()),
            execution_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            execution_history: RwLock::new(VecDeque::with_capacity(MAX_EXECUTION_HISTORY)),
            security_events: RwLock::new(Vec::new()),
            execution_lock: tokio::sync::Mutex::new(()),
        }
    }

    /// Get current session state
    pub async fn state(&self) -> SessionState {
        *self.state.read().await
    }

    /// Set session state
    pub async fn set_state(&self, state: SessionState) {
        *self.state.write().await = state;
    }

    /// Get container ID
    pub async fn container_id(&self) -> Option<String> {
        self.container_id.read().await.clone()
    }

    /// Set container ID
    pub async fn set_container_id(&self, container_id: String) {
        *self.container_id.write().await = Some(container_id);
    }

    /// Update last activity timestamp
    pub async fn touch(&self) {
        *self.last_activity.write().await = Utc::now();
    }

    /// Get last activity timestamp
    pub async fn last_activity(&self) -> DateTime<Utc> {
        *self.last_activity.read().await
    }

    /// Increment execution counter
    pub fn increment_executions(&self) {
        self.execution_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment error counter
    pub fn increment_errors(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get execution count
    pub fn execution_count(&self) -> u64 {
        self.execution_count.load(Ordering::Relaxed)
    }

    /// Get error count
    pub fn error_count(&self) -> u64 {
        self.error_count.load(Ordering::Relaxed)
    }

    /// Add an execution to history
    pub async fn add_execution(&self, summary: ExecutionSummary) {
        let mut history = self.execution_history.write().await;
        if history.len() >= MAX_EXECUTION_HISTORY {
            history.pop_front();
        }
        history.push_back(summary);
    }

    /// Get execution history
    pub async fn get_execution_history(&self) -> Vec<ExecutionSummary> {
        self.execution_history.read().await.iter().cloned().collect()
    }

    /// Add a security event
    pub async fn add_security_event(&self, event: SecurityEvent) {
        self.security_events.write().await.push(event);
    }

    /// Get security events
    pub async fn get_security_events(&self) -> Vec<SecurityEvent> {
        self.security_events.read().await.clone()
    }

    /// Update current resource usage
    pub async fn update_usage(&self, usage: ResourceUsage) {
        *self.current_usage.write().await = usage;
    }

    /// Get current resource usage
    pub async fn current_usage(&self) -> ResourceUsage {
        self.current_usage.read().await.clone()
    }

    /// Acquire execution lock (ensures one execution at a time)
    pub async fn acquire_execution_lock(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.execution_lock.lock().await
    }

    /// Try to acquire execution lock without blocking
    pub fn try_acquire_execution_lock(&self) -> Option<tokio::sync::MutexGuard<'_, ()>> {
        self.execution_lock.try_lock().ok()
    }
}

/// Thread-safe session handle
pub type SessionHandle = Arc<Session>;
