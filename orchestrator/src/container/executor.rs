//! Code execution within containers

use std::process::Stdio;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::models::{ExecuteRequest, ExecuteResponse, ExecutionStatus, ExecutionSummary, Language};
use crate::persistent::PersistentKernelManager;

use super::session::SessionHandle;

/// Executes code inside containers
pub struct CodeExecutor {
    /// Path to podman binary
    podman_path: String,
    /// Persistent kernel manager
    kernel_manager: std::sync::Arc<PersistentKernelManager>,
}

impl CodeExecutor {
    pub fn new(kernel_manager: std::sync::Arc<PersistentKernelManager>) -> Self {
        // Find podman in PATH or common locations
        let podman_path = which::which("podman")
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "/opt/podman/bin/podman".to_string());

        Self {
            podman_path,
            kernel_manager,
        }
    }

    /// Execute code in a session's container
    pub async fn execute(
        &self,
        session: &SessionHandle,
        request: &ExecuteRequest,
    ) -> Result<ExecuteResponse> {
        let execution_id = Uuid::new_v4();
        let start_time = Instant::now();

        // Acquire execution lock (only one execution at a time per session)
        let _lock = session.acquire_execution_lock().await;

        // Update session activity
        session.touch().await;
        session.increment_executions();

        let container_id = session
            .container_id()
            .await
            .ok_or_else(|| anyhow::anyhow!("Container not started"))?;

        // Check if persistent mode is requested and language is Python
        if request.persistent && request.language == Language::Python {
            return self.execute_persistent(session, request, execution_id, start_time).await;
        }

        // Generate unique filename for code
        let code_filename = format!(
            "exec_{}.{}",
            execution_id.simple(),
            request.language.file_extension()
        );
        let code_path = session.workspace_path.join(&code_filename);
        let container_code_path = format!("/mnt/data/{}", code_filename);

        // Write code to file
        tokio::fs::write(&code_path, &request.code)
            .await
            .context("Failed to write code file")?;

        debug!(
            execution_id = %execution_id,
            language = %request.language.as_str(),
            code_path = %code_path.display(),
            "Executing code"
        );

        // Build the exec command
        let exec_args = self.build_exec_args(
            &container_id,
            &request.language,
            &container_code_path,
            &request.env,
        );

        // Execute with timeout
        let timeout_duration = Duration::from_secs(request.timeout_seconds);
        let result = timeout(timeout_duration, self.run_command(&exec_args)).await;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Clean up code file (but keep output files)
        let _ = tokio::fs::remove_file(&code_path).await;

        // Kill any remaining background processes spawned by the code
        let _ = self.kill_processes(&container_id).await;

        // Process result
        let response = match result {
            Ok(Ok((stdout, stderr, exit_code))) => {
                // Detect OOM kill (exit code 137 = SIGKILL from OOM)
                let status = if exit_code == 0 {
                    ExecutionStatus::Success
                } else if exit_code == 137 {
                    session.increment_errors();
                    warn!(
                        execution_id = %execution_id,
                        user_id = %session.user_id,
                        "Execution killed by OOM"
                    );
                    ExecutionStatus::Killed
                } else {
                    session.increment_errors();
                    ExecutionStatus::Error
                };

                ExecuteResponse {
                    execution_id,
                    status,
                    stdout,
                    stderr,
                    exit_code: Some(exit_code),
                    duration_ms,
                    files_created: self.detect_new_files(session).await,
                    resource_usage: None,
                }
            }
            Ok(Err(e)) => {
                session.increment_errors();
                error!(error = %e, "Execution failed");
                ExecuteResponse {
                    execution_id,
                    status: ExecutionStatus::Error,
                    stdout: String::new(),
                    stderr: format!("Execution error: {}", e),
                    exit_code: None,
                    duration_ms,
                    files_created: vec![],
                    resource_usage: None,
                }
            }
            Err(_) => {
                session.increment_errors();
                warn!(execution_id = %execution_id, "Execution timed out");

                // Kill the process in the container
                let _ = self.kill_processes(&container_id).await;

                ExecuteResponse {
                    execution_id,
                    status: ExecutionStatus::Timeout,
                    stdout: String::new(),
                    stderr: format!(
                        "Execution timed out after {} seconds",
                        request.timeout_seconds
                    ),
                    exit_code: None,
                    duration_ms,
                    files_created: vec![],
                    resource_usage: None,
                }
            }
        };

        // Add to execution history
        let code_hash = self.hash_code(&request.code);
        session
            .add_execution(ExecutionSummary {
                execution_id,
                timestamp: chrono::Utc::now(),
                language: request.language,
                code_hash,
                status: response.status,
                duration_ms,
                exit_code: response.exit_code,
            })
            .await;

        info!(
            execution_id = %execution_id,
            status = ?response.status,
            duration_ms = duration_ms,
            "Execution completed"
        );

        Ok(response)
    }

    /// Build podman exec arguments
    fn build_exec_args(
        &self,
        container_id: &str,
        language: &Language,
        code_path: &str,
        env: &std::collections::HashMap<String, String>,
    ) -> Vec<String> {
        let mut args = vec![
            "exec".to_string(),
            "--user".to_string(),
            "sandbox".to_string(),
            "--workdir".to_string(),
            "/mnt/data".to_string(),
        ];

        // Add environment variables
        for (key, value) in env {
            args.push("-e".to_string());
            args.push(format!("{}={}", key, value));
        }

        args.push(container_id.to_string());

        // Add interpreter and script
        match language {
            Language::Python => {
                args.push("python".to_string());
                args.push("-u".to_string()); // Unbuffered output
                args.push(code_path.to_string());
            }
            Language::Javascript => {
                args.push("node".to_string());
                args.push(code_path.to_string());
            }
            Language::Bash => {
                args.push("bash".to_string());
                args.push(code_path.to_string());
            }
            Language::R => {
                args.push("Rscript".to_string());
                args.push("--vanilla".to_string());
                args.push(code_path.to_string());
            }
            Language::Julia => {
                args.push("julia".to_string());
                args.push(code_path.to_string());
            }
            Language::Typescript => {
                args.push("deno".to_string());
                args.push("run".to_string());
                args.push("--allow-read=/mnt/data".to_string());
                args.push("--allow-write=/mnt/data".to_string());
                args.push(code_path.to_string());
            }
            Language::Ruby => {
                args.push("ruby".to_string());
                args.push(code_path.to_string());
            }
            Language::Go => {
                // Go requires compilation - set cache to /mnt/data (writable)
                args.push("bash".to_string());
                args.push("-c".to_string());
                args.push(format!(
                    "cd /mnt/data && GOTMPDIR=/mnt/data GOCACHE=/mnt/data/.gocache go run {}",
                    code_path.split('/').next_back().unwrap_or(code_path)
                ));
            }
            Language::Wasm => {
                args.push("wasmtime".to_string());
                args.push("run".to_string());
                args.push("--dir=/mnt/data".to_string());
                args.push(code_path.to_string());
            }
        }

        args
    }

    /// Run the podman command and capture output
    async fn run_command(&self, args: &[String]) -> Result<(String, String, i32)> {
        let mut cmd = Command::new(&self.podman_path);
        cmd.args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().context("Failed to spawn podman process")?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let mut stdout_lines = Vec::new();
        let mut stderr_lines = Vec::new();

        // Read both streams concurrently
        loop {
            tokio::select! {
                line = stdout_reader.next_line() => {
                    match line {
                        Ok(Some(line)) => stdout_lines.push(line),
                        Ok(None) => break,
                        Err(e) => {
                            warn!("Error reading stdout: {}", e);
                            break;
                        }
                    }
                }
                line = stderr_reader.next_line() => {
                    match line {
                        Ok(Some(line)) => stderr_lines.push(line),
                        Ok(None) => {}
                        Err(e) => {
                            warn!("Error reading stderr: {}", e);
                        }
                    }
                }
            }
        }

        // Drain remaining stderr
        while let Ok(Some(line)) = stderr_reader.next_line().await {
            stderr_lines.push(line);
        }

        let status = child.wait().await.context("Failed to wait for process")?;
        let exit_code = status.code().unwrap_or(-1);

        Ok((
            stdout_lines.join("\n"),
            stderr_lines.join("\n"),
            exit_code,
        ))
    }

    /// Kill all user processes in a container
    async fn kill_processes(&self, container_id: &str) -> Result<()> {
        let output = Command::new(&self.podman_path)
            .args(["exec", container_id, "pkill", "-u", "sandbox"])
            .output()
            .await?;

        if !output.status.success() {
            debug!("pkill returned non-zero (may be normal if no processes)");
        }

        Ok(())
    }

    /// Detect newly created files in the workspace
    async fn detect_new_files(&self, session: &SessionHandle) -> Vec<String> {
        let mut files = Vec::new();

        if let Ok(mut entries) = tokio::fs::read_dir(&session.workspace_path).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if let Some(name) = path.file_name() {
                    let name_str = name.to_string_lossy();
                    // Exclude execution scripts
                    if !name_str.starts_with("exec_") {
                        files.push(name_str.to_string());
                    }
                }
            }
        }

        files
    }

    /// Hash code for audit logging
    fn hash_code(&self, code: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(code.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Execute code in persistent interpreter mode
    async fn execute_persistent(
        &self,
        session: &SessionHandle,
        request: &ExecuteRequest,
        execution_id: Uuid,
        start_time: std::time::Instant,
    ) -> Result<ExecuteResponse> {
        let container_id = session.container_id().await
            .ok_or_else(|| anyhow::anyhow!("Container not started"))?;

        // Get or start kernel
        let kernel = self.kernel_manager
            .start_kernel(&session.user_id, &container_id, &self.podman_path)
            .await?;

        debug!(
            execution_id = %execution_id,
            kernel_id = %kernel.kernel_id,
            "Executing in persistent kernel"
        );

        // Execute with timeout
        let timeout_duration = std::time::Duration::from_secs(request.timeout_seconds);
        let result = timeout(
            timeout_duration,
            self.kernel_manager.execute_in_kernel(&session.user_id, &request.code, &self.podman_path)
        ).await;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        let response = match result {
            Ok(Ok((stdout, stderr))) => {
                let status = if stderr.is_empty() {
                    ExecutionStatus::Success
                } else {
                    session.increment_errors();
                    ExecutionStatus::Error
                };

                ExecuteResponse {
                    execution_id,
                    status,
                    stdout,
                    stderr,
                    exit_code: Some(0),
                    duration_ms,
                    files_created: self.detect_new_files(session).await,
                    resource_usage: None,
                }
            }
            Ok(Err(e)) => {
                session.increment_errors();
                ExecuteResponse {
                    execution_id,
                    status: ExecutionStatus::Error,
                    stdout: String::new(),
                    stderr: e.to_string(),
                    exit_code: None,
                    duration_ms,
                    files_created: vec![],
                    resource_usage: None,
                }
            }
            Err(_) => {
                session.increment_errors();
                ExecuteResponse {
                    execution_id,
                    status: ExecutionStatus::Timeout,
                    stdout: String::new(),
                    stderr: format!("Execution timed out after {} seconds", request.timeout_seconds),
                    exit_code: None,
                    duration_ms,
                    files_created: vec![],
                    resource_usage: None,
                }
            }
        };

        // Add to execution history
        let code_hash = self.hash_code(&request.code);
        session
            .add_execution(ExecutionSummary {
                execution_id,
                timestamp: chrono::Utc::now(),
                language: request.language,
                code_hash,
                status: response.status,
                duration_ms,
                exit_code: response.exit_code,
            })
            .await;

        info!(
            execution_id = %execution_id,
            status = ?response.status,
            duration_ms = duration_ms,
            mode = "persistent",
            "Execution completed"
        );

        Ok(response)
    }
}

impl Default for CodeExecutor {
    fn default() -> Self {
        Self::new(std::sync::Arc::new(PersistentKernelManager::new()))
    }
}
