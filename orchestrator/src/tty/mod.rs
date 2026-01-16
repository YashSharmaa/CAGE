//! Interactive TTY mode for live debugging
//!
//! Allows attaching to a container's shell for interactive debugging

use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::process::Command;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::container::session::SessionHandle;

/// TTY session for interactive container access
#[derive(Clone)]
pub struct TtySession {
    pub session_id: Uuid,
    pub user_id: String,
    pub container_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// TTY manager for interactive sessions
pub struct TtyManager {
    sessions: Arc<RwLock<std::collections::HashMap<Uuid, TtySession>>>,
    podman_path: String,
}

impl TtyManager {
    pub fn new() -> Self {
        let podman_path = which::which("podman")
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "/opt/podman/bin/podman".to_string());

        Self {
            sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
            podman_path,
        }
    }

    /// Start an interactive TTY session
    pub async fn start_tty(
        &self,
        session: &SessionHandle,
    ) -> Result<Uuid> {
        let container_id = session
            .container_id()
            .await
            .ok_or_else(|| anyhow::anyhow!("Container not started"))?;

        let tty_id = Uuid::new_v4();

        let tty_session = TtySession {
            session_id: tty_id,
            user_id: session.user_id.clone(),
            container_id: container_id.clone(),
            created_at: chrono::Utc::now(),
        };

        self.sessions.write().await.insert(tty_id, tty_session);

        Ok(tty_id)
    }

    /// Execute a command in TTY mode and return output
    pub async fn execute_tty_command(
        &self,
        tty_id: &Uuid,
        command: &str,
    ) -> Result<(String, String)> {
        let session = {
            let sessions = self.sessions.read().await;
            sessions
                .get(tty_id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("TTY session not found"))?
        };

        // Execute command in container
        let output = Command::new(&self.podman_path)
            .args([
                "exec",
                "-u",
                "sandbox",
                &session.container_id,
                "bash",
                "-c",
                command,
            ])
            .output()
            .await
            .context("Failed to execute TTY command")?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok((stdout, stderr))
    }

    /// Stream interactive shell (for WebSocket)
    pub async fn stream_shell(
        &self,
        tty_id: &Uuid,
    ) -> Result<()> {
        let session = {
            let sessions = self.sessions.read().await;
            sessions
                .get(tty_id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("TTY session not found"))?
        };

        // This would typically be used with WebSocket streaming
        // For now, just verify we can spawn an interactive shell
        let mut child = Command::new(&self.podman_path)
            .args([
                "exec",
                "-it",
                "-u",
                "sandbox",
                &session.container_id,
                "bash",
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn interactive shell")?;

        // In a real implementation, this would connect stdin/stdout to WebSocket
        // For now, we just verify it works
        let _ = child.wait().await?;

        Ok(())
    }

    /// Stop a TTY session
    pub async fn stop_tty(&self, tty_id: &Uuid) -> Result<()> {
        self.sessions.write().await.remove(tty_id);
        Ok(())
    }

    /// List active TTY sessions
    pub async fn list_sessions(&self) -> Vec<TtySession> {
        self.sessions
            .read()
            .await
            .values()
            .cloned()
            .collect()
    }
}

impl Default for TtyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tty_manager_creation() {
        let manager = TtyManager::new();
        assert!(!manager.podman_path.is_empty());
    }
}
