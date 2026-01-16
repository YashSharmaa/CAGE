//! Container lifecycle management

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use crate::config::AppConfig;
use crate::models;
use crate::models::{
    AdminSessionSummary, ExecuteRequest, ExecuteResponse, FileInfo, FileListResponse, FileType,
    ResourceUsage, SessionInfo,
};

use super::executor::CodeExecutor;
use super::session::{Session, SessionHandle, SessionState};

/// Manages container lifecycle and sessions
pub struct ContainerManager {
    /// Application configuration
    config: AppConfig,

    /// Active sessions by user ID
    sessions: RwLock<HashMap<String, SessionHandle>>,

    /// Code executor
    executor: CodeExecutor,

    /// Path to podman binary
    podman_path: String,

    /// gVisor runtime (optional)
    gvisor_runtime: Option<Arc<crate::gvisor::GVisorRuntime>>,
}

impl ContainerManager {
    /// Create a new container manager
    pub async fn new(
        config: AppConfig,
        kernel_manager: Arc<crate::persistent::PersistentKernelManager>,
        gvisor_runtime: Option<Arc<crate::gvisor::GVisorRuntime>>,
    ) -> Result<Self> {
        // Find podman
        let podman_path = which::which("podman")
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "/opt/podman/bin/podman".to_string());

        // Verify podman is available
        let output = Command::new(&podman_path)
            .args(["--version"])
            .output()
            .await
            .context("Failed to run podman --version")?;

        if !output.status.success() {
            anyhow::bail!("Podman not available or not working");
        }

        let version = String::from_utf8_lossy(&output.stdout);
        info!(podman_version = %version.trim(), "Podman initialized");

        // Ensure data directory exists
        tokio::fs::create_dir_all(&config.data_dir)
            .await
            .context("Failed to create data directory")?;

        Ok(Self {
            config,
            sessions: RwLock::new(HashMap::new()),
            executor: CodeExecutor::new(kernel_manager),
            podman_path,
            gvisor_runtime,
        })
    }

    /// Get or create a session for a user with specific language
    pub async fn get_or_create_session_for_language(&self, user_id: &str, language: models::Language) -> Result<SessionHandle> {
        // Session key includes language for multi-language support
        let session_key = format!("{}_{}", user_id, language.as_str());

        // Check for existing session
        {
            let sessions = self.sessions.read().await;
            if let Some(session) = sessions.get(&session_key) {
                // Check if container is still running
                if session.state().await == SessionState::Running {
                    return Ok(session.clone());
                }
            }
        }

        // Create new session for this language
        self.create_session_with_language_keyed(user_id, language).await
    }

    /// Get or create a session for a user (defaults to Python)
    pub async fn get_or_create_session(&self, user_id: &str) -> Result<SessionHandle> {
        self.get_or_create_session_for_language(user_id, models::Language::Python).await
    }

    /// Create a new session for a user
    pub async fn create_session(&self, user_id: &str) -> Result<SessionHandle> {
        self.create_session_with_language(user_id, models::Language::Python).await
    }

    /// Create a new session for a user with specific language (uses language-specific key)
    async fn create_session_with_language_keyed(&self, user_id: &str, language: models::Language) -> Result<SessionHandle> {
        let session_key = format!("{}_{}", user_id, language.as_str());

        info!(user_id = %user_id, language = %language.as_str(), "Creating new session");

        // Get user-specific limits
        let resource_limits = self.config.get_user_limits(user_id);
        let network_policy = self.config.get_user_network(user_id);

        // Create workspace directory
        let workspace_path = self.config.data_dir.join(format!("user_{}", user_id));
        tokio::fs::create_dir_all(&workspace_path)
            .await
            .context("Failed to create workspace directory")?;

        // Ensure workspace is writable (chmod 777 for development - Podman :U will map UIDs)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::Permissions::from_mode(0o777);
            let _ = tokio::fs::set_permissions(&workspace_path, permissions).await;
        }

        // Create session
        let session = Arc::new(Session::new(
            user_id.to_string(),
            workspace_path.clone(),
            resource_limits.clone(),
            network_policy.clone(),
        ));

        // Start container with language-specific image
        let container_id = self
            .start_container(&session, &resource_limits, &network_policy, language)
            .await?;

        // Update session with container ID
        session.set_container_id(container_id).await;
        session.set_state(SessionState::Running).await;

        // Store session with language-specific key
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_key, session.clone());
        }

        info!(
            user_id = %user_id,
            language = %language.as_str(),
            container_name = %session.container_name,
            "Session created"
        );

        Ok(session)
    }

    /// Create a new session for a user with specific language (public API - uses user_id key only)
    pub async fn create_session_with_language(&self, user_id: &str, language: models::Language) -> Result<SessionHandle> {
        info!(user_id = %user_id, language = %language.as_str(), "Creating new session");

        // Get user-specific limits
        let resource_limits = self.config.get_user_limits(user_id);
        let network_policy = self.config.get_user_network(user_id);

        // Create workspace directory
        let workspace_path = self.config.data_dir.join(format!("user_{}", user_id));
        tokio::fs::create_dir_all(&workspace_path)
            .await
            .context("Failed to create workspace directory")?;

        // Ensure workspace is writable (chmod 777 for development - Podman :U will map UIDs)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::Permissions::from_mode(0o777);
            let _ = tokio::fs::set_permissions(&workspace_path, permissions).await;
        }

        // Create session
        let session = Arc::new(Session::new(
            user_id.to_string(),
            workspace_path.clone(),
            resource_limits.clone(),
            network_policy.clone(),
        ));

        // Start container with language-specific image
        let container_id = self
            .start_container(&session, &resource_limits, &network_policy, language)
            .await?;

        // Update session with container ID
        session.set_container_id(container_id).await;
        session.set_state(SessionState::Running).await;

        // Store session
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(user_id.to_string(), session.clone());
        }

        info!(
            user_id = %user_id,
            container_name = %session.container_name,
            "Session created"
        );

        Ok(session)
    }

    /// Start a container for a session
    async fn start_container(
        &self,
        session: &Session,
        limits: &crate::config::ResourceLimits,
        network: &crate::config::NetworkPolicy,
        language: models::Language,
    ) -> Result<String> {
        let mut args = vec![
            "run".to_string(),
            "--detach".to_string(),
            "--name".to_string(),
            session.container_name.clone(),
            // Resource limits
            "--memory".to_string(),
            format!("{}m", limits.max_memory_mb),
            "--cpus".to_string(),
            format!("{}", limits.max_cpus),
            "--pids-limit".to_string(),
            format!("{}", limits.max_pids),
            // Security options
            "--read-only".to_string(),
            "--tmpfs".to_string(),
            "/tmp:rw,noexec,nosuid,size=100m".to_string(),
            "--security-opt".to_string(),
            "no-new-privileges".to_string(),
            "--cap-drop".to_string(),
            "ALL".to_string(),
            // User
            "--user".to_string(),
            "sandbox".to_string(),
            // Workspace volume with UID mapping
            // For compiled languages (Go, Wasm), allow exec
            "--volume".to_string(),
            format!(
                "{}:/mnt/data:rw,{},nosuid,nodev,U",
                session.workspace_path.display(),
                if matches!(language, models::Language::Go | models::Language::Wasm) {
                    "exec"
                } else {
                    "noexec"
                }
            ),
        ];

        // Network configuration
        if !network.enabled {
            args.push("--network".to_string());
            args.push("none".to_string());
        } else {
            // Network enabled - create user-specific network with whitelist
            let network_name = format!("cage_net_{}", session.user_id);
            args.push("--network".to_string());
            args.push(network_name);
            // Note: Network should be created beforehand with allowed_hosts rules
        }

        // Seccomp profile if configured
        if let Some(ref seccomp_path) = self.config.security.seccomp_profile {
            args.push("--security-opt".to_string());
            args.push(format!("seccomp={}", seccomp_path.display()));
        }

        // GPU support if enabled for user
        if self.config.users.get(&session.user_id).map(|u| u.gpu_enabled).unwrap_or(false) {
            args.push("--device".to_string());
            args.push("/dev/nvidia0".to_string());
            args.push("--device".to_string());
            args.push("/dev/nvidiactl".to_string());
            args.push("--device".to_string());
            args.push("/dev/nvidia-uvm".to_string());
        }

        // gVisor runtime args if enabled
        if let Some(ref gvisor) = self.gvisor_runtime {
            let gvisor_args = gvisor.get_runtime_args();
            if !gvisor_args.is_empty() {
                info!("Starting container with gVisor runtime");
                args.extend(gvisor_args);
            }
        }

        // Image and command - select based on language
        let image = language.container_image();
        args.push(image.to_string());
        args.push("sleep".to_string());
        args.push("infinity".to_string());

        debug!(args = ?args, "Starting container");

        let output = Command::new(&self.podman_path)
            .args(&args)
            .output()
            .await
            .context("Failed to start container")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!(stderr = %stderr, "Container start failed");
            anyhow::bail!("Failed to start container: {}", stderr);
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        info!(container_id = %container_id, "Container started");

        Ok(container_id)
    }

    /// Execute code in a user's session
    pub async fn execute_code(
        &self,
        user_id: &str,
        request: ExecuteRequest,
    ) -> Result<ExecuteResponse> {
        let session = self.get_or_create_session_for_language(user_id, request.language).await?;
        self.executor.execute(&session, &request).await
    }

    /// Get session handle for a user (internal use)
    pub async fn get_session_handle(&self, user_id: &str) -> Option<SessionHandle> {
        let sessions = self.sessions.read().await;
        sessions.get(user_id).cloned()
    }

    /// Get session info for a user
    pub async fn get_session(&self, user_id: &str) -> Option<SessionInfo> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(user_id)?;

        Some(SessionInfo {
            session_id: session.session_id,
            user_id: session.user_id.clone(),
            container_id: session.container_id().await,
            status: session.state().await.into(),
            created_at: session.created_at,
            last_activity: session.last_activity().await,
            resource_limits: session.resource_limits.clone(),
            current_usage: Some(session.current_usage().await),
            network_policy: session.network_policy.clone(),
        })
    }

    /// Terminate a user's session
    pub async fn terminate_session(&self, user_id: &str, purge_data: bool) -> Result<()> {
        let session = {
            let mut sessions = self.sessions.write().await;
            sessions.remove(user_id)
        };

        if let Some(session) = session {
            // Stop and remove container
            if let Some(container_id) = session.container_id().await {
                let _ = Command::new(&self.podman_path)
                    .args(["rm", "-f", &container_id])
                    .output()
                    .await;
            }

            // Optionally purge workspace data
            if purge_data {
                let _ = tokio::fs::remove_dir_all(&session.workspace_path).await;
            }

            info!(user_id = %user_id, "Session terminated");
        }

        Ok(())
    }

    /// List files in a user's workspace
    pub async fn list_files(&self, user_id: &str, path: &str) -> Result<FileListResponse> {
        let session = self.get_or_create_session(user_id).await?;

        let base_path = &session.workspace_path;
        let target_path = if path.is_empty() || path == "/" {
            base_path.clone()
        } else {
            // Sanitize path to prevent directory traversal
            let clean_path = path.trim_start_matches('/');
            let target = base_path.join(clean_path);

            // Ensure we're still within the workspace
            if !target.starts_with(base_path) {
                anyhow::bail!("Invalid path");
            }
            target
        };

        let mut files = Vec::new();
        let mut total_size = 0u64;

        let mut entries = tokio::fs::read_dir(&target_path)
            .await
            .context("Failed to read directory")?;

        while let Some(entry) = entries.next_entry().await? {
            let metadata = entry.metadata().await?;
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files and execution artifacts
            if name.starts_with('.') || name.starts_with("exec_") {
                continue;
            }

            let size = metadata.len();
            total_size += size;

            let file_type = if metadata.is_dir() {
                FileType::Directory
            } else {
                FileType::File
            };

            let modified_at = metadata
                .modified()
                .ok()
                .map(chrono::DateTime::<chrono::Utc>::from)
                .unwrap_or_else(chrono::Utc::now);

            files.push(FileInfo {
                name: name.clone(),
                path: format!("{}/{}", path.trim_end_matches('/'), name),
                file_type,
                size_bytes: size,
                modified_at,
                permissions: None,
            });
        }

        // Sort by name
        files.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(FileListResponse {
            path: path.to_string(),
            files,
            total_size_bytes: total_size,
        })
    }

    /// Read a file from a user's workspace
    pub async fn read_file(&self, user_id: &str, filepath: &str) -> Result<Vec<u8>> {
        let session = self.get_or_create_session(user_id).await?;

        let clean_path = filepath.trim_start_matches('/');
        let target = session.workspace_path.join(clean_path);

        // Security: ensure path is within workspace
        if !target.starts_with(&session.workspace_path) {
            anyhow::bail!("Invalid path");
        }

        let contents = tokio::fs::read(&target)
            .await
            .context("Failed to read file")?;

        Ok(contents)
    }

    /// Write a file to a user's workspace
    pub async fn write_file(
        &self,
        user_id: &str,
        filepath: &str,
        contents: &[u8],
    ) -> Result<String> {
        let session = self.get_or_create_session(user_id).await?;

        // Sanitize filename
        let clean_path = filepath
            .trim_start_matches('/')
            .replace("..", "")
            .replace("//", "/");

        // Only allow safe characters
        if !clean_path
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '/')
        {
            anyhow::bail!("Invalid filename");
        }

        let target = session.workspace_path.join(&clean_path);

        // Security: ensure path is within workspace
        if !target.starts_with(&session.workspace_path) {
            anyhow::bail!("Invalid path");
        }

        // Create parent directories if needed
        if let Some(parent) = target.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(&target, contents)
            .await
            .context("Failed to write file")?;

        // Calculate checksum
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(contents);
        let checksum = format!("{:x}", hasher.finalize());

        Ok(checksum)
    }

    /// Delete a file from a user's workspace
    pub async fn delete_file(&self, user_id: &str, filepath: &str) -> Result<()> {
        let session = self.get_or_create_session(user_id).await?;

        let clean_path = filepath.trim_start_matches('/');
        let target = session.workspace_path.join(clean_path);

        // Security: ensure path is within workspace
        if !target.starts_with(&session.workspace_path) {
            anyhow::bail!("Invalid path");
        }

        if target.is_dir() {
            tokio::fs::remove_dir_all(&target).await?;
        } else {
            tokio::fs::remove_file(&target).await?;
        }

        Ok(())
    }

    /// Get all sessions (for admin)
    pub async fn list_all_sessions(&self) -> Vec<AdminSessionSummary> {
        let sessions = self.sessions.read().await;
        let mut summaries = Vec::new();

        for (user_id, session) in sessions.iter() {
            let usage = session.current_usage().await;

            summaries.push(AdminSessionSummary {
                user_id: user_id.clone(),
                container_id: session.container_id().await,
                status: session.state().await.into(),
                created_at: session.created_at,
                last_activity: session.last_activity().await,
                cpu_percent: usage.cpu_percent,
                memory_mb: usage.memory_mb,
                execution_count: session.execution_count(),
                error_count: session.error_count(),
                warnings: vec![],
            });
        }

        summaries
    }

    /// Get podman binary path
    pub fn podman_path(&self) -> &str {
        &self.podman_path
    }

    /// Get podman version
    pub async fn podman_version(&self) -> Option<String> {
        let output = Command::new(&self.podman_path)
            .args(["--version"])
            .output()
            .await
            .ok()?;

        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            None
        }
    }

    /// Get active session count
    pub async fn active_session_count(&self) -> u64 {
        self.sessions.read().await.len() as u64
    }

    /// Update resource usage for all sessions
    pub async fn update_all_stats(&self) -> Result<()> {
        let sessions = self.sessions.read().await;

        for (_user_id, session) in sessions.iter() {
            if let Some(container_id) = session.container_id().await {
                if let Ok(stats) = self.get_container_stats(&container_id).await {
                    session.update_usage(stats).await;
                }
            }
        }

        Ok(())
    }

    /// Get container resource stats from podman
    async fn get_container_stats(&self, container_id: &str) -> Result<ResourceUsage> {
        let output = Command::new(&self.podman_path)
            .args(["stats", "--no-stream", "--format", "json", container_id])
            .output()
            .await?;

        if !output.status.success() {
            anyhow::bail!("Failed to get container stats");
        }

        let stats_json = String::from_utf8_lossy(&output.stdout);

        // Parse the JSON output from podman stats
        // Format: [{"id":"...","cpu_percent":"0.00%","mem_usage":"1.234MiB / 512MiB",...}]
        let stats: serde_json::Value = serde_json::from_str(stats_json.trim())?;

        let cpu_str = stats[0]["cpu_percent"].as_str().unwrap_or("0.0%");
        let cpu_percent = cpu_str.trim_end_matches('%').parse::<f64>().unwrap_or(0.0);

        let mem_str = stats[0]["mem_usage"].as_str().unwrap_or("0B / 0B");
        let mem_parts: Vec<&str> = mem_str.split(" / ").collect();
        let mem_used = if !mem_parts.is_empty() {
            parse_memory_string(mem_parts[0])
        } else {
            0.0
        };

        let pids_str = stats[0]["pids"].as_str().unwrap_or("0");
        let pids = pids_str.parse::<u32>().unwrap_or(0);

        // Get disk usage from workspace
        let sessions = self.sessions.read().await;
        let disk_mb = if let Some(session) = sessions.values().find(|s| {
            futures::executor::block_on(s.container_id()) == Some(container_id.to_string())
        }) {
            self.get_disk_usage(&session.workspace_path).await.unwrap_or(0.0)
        } else {
            0.0
        };

        Ok(ResourceUsage {
            cpu_percent,
            memory_mb: mem_used,
            disk_mb,
            pids,
        })
    }

    /// Calculate disk usage of a directory in MB
    async fn get_disk_usage(&self, path: &Path) -> Result<f64> {
        let output = Command::new("du")
            .args(["-sm", path.to_str().unwrap_or(".")])
            .output()
            .await?;

        if !output.status.success() {
            return Ok(0.0);
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let size_mb = output_str
            .split_whitespace()
            .next()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        Ok(size_mb)
    }
}

/// Parse memory string like "123.4MiB" or "1.2GiB" to MB
fn parse_memory_string(s: &str) -> f64 {
    let s = s.trim();
    if s.ends_with("GiB") || s.ends_with("GB") {
        let num_str = s.trim_end_matches("GiB").trim_end_matches("GB");
        num_str.parse::<f64>().unwrap_or(0.0) * 1024.0
    } else if s.ends_with("MiB") || s.ends_with("MB") {
        let num_str = s.trim_end_matches("MiB").trim_end_matches("MB");
        num_str.parse::<f64>().unwrap_or(0.0)
    } else if s.ends_with("KiB") || s.ends_with("kB") || s.ends_with("KB") {
        let num_str = s.trim_end_matches("KiB").trim_end_matches("kB").trim_end_matches("KB");
        num_str.parse::<f64>().unwrap_or(0.0) / 1024.0
    } else if s.ends_with('B') {
        let num_str = s.trim_end_matches('B');
        num_str.parse::<f64>().unwrap_or(0.0) / (1024.0 * 1024.0)
    } else {
        0.0
    }
}
