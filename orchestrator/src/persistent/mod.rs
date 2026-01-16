//! Persistent interpreter mode using Jupyter kernels

pub mod kernel;

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Jupyter kernel connection info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelInfo {
    pub kernel_id: Uuid,
    pub user_id: String,
    pub container_id: String,
    pub kernel_port: u16,
    pub shell_port: u16,
    pub iopub_port: u16,
    pub stdin_port: u16,
    pub control_port: u16,
    pub key: String,
}

/// Manager for persistent Jupyter kernels
pub struct PersistentKernelManager {
    kernels: Arc<RwLock<HashMap<String, KernelInfo>>>,
}

impl PersistentKernelManager {
    pub fn new() -> Self {
        Self {
            kernels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start a Jupyter kernel in a container
    pub async fn start_kernel(
        &self,
        user_id: &str,
        container_id: &str,
        podman_path: &str,
    ) -> Result<KernelInfo> {
        // Check if kernel already exists
        {
            let kernels = self.kernels.read().await;
            if let Some(kernel) = kernels.get(user_id) {
                return Ok(kernel.clone());
            }
        }

        // Start the kernel using the kernel module
        let kernel_info = kernel::start_jupyter_kernel(container_id, user_id, podman_path).await?;

        self.kernels.write().await.insert(user_id.to_string(), kernel_info.clone());

        Ok(kernel_info)
    }

    /// Execute code in persistent kernel
    pub async fn execute_in_kernel(
        &self,
        user_id: &str,
        code: &str,
        podman_path: &str,
    ) -> Result<(String, String)> {
        let kernel = self.kernels.read().await.get(user_id).cloned()
            .ok_or_else(|| anyhow::anyhow!("No kernel for user"))?;

        kernel::execute_in_kernel(&kernel, code, podman_path).await
    }

    /// Stop a kernel
    pub async fn stop_kernel(&self, user_id: &str) -> Result<()> {
        self.kernels.write().await.remove(user_id);
        Ok(())
    }

    /// Get kernel info
    pub async fn get_kernel(&self, user_id: &str) -> Option<KernelInfo> {
        self.kernels.read().await.get(user_id).cloned()
    }
}

impl Default for PersistentKernelManager {
    fn default() -> Self {
        Self::new()
    }
}
