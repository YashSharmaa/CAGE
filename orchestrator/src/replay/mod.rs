//! Execution replay capability
//!
//! Stores complete execution details and allows replaying them for debugging

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::models::{ExecuteRequest, ExecuteResponse};

/// Stored execution for replay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredExecution {
    pub execution_id: Uuid,
    pub user_id: String,
    pub timestamp: DateTime<Utc>,
    pub request: ExecuteRequest,
    pub response: ExecuteResponse,
    pub code: String,
}

/// Execution replay manager
pub struct ReplayManager {
    executions: Arc<RwLock<HashMap<Uuid, StoredExecution>>>,
    storage_dir: PathBuf,
    max_stored: usize,
}

impl ReplayManager {
    /// Create new replay manager
    pub async fn new(data_dir: &Path, max_stored: usize) -> Result<Self> {
        let storage_dir = data_dir.join("replays");
        tokio::fs::create_dir_all(&storage_dir)
            .await
            .context("Failed to create replay storage directory")?;

        let manager = Self {
            executions: Arc::new(RwLock::new(HashMap::new())),
            storage_dir,
            max_stored,
        };

        // Load existing replays
        manager.load_all().await?;

        Ok(manager)
    }

    /// Store an execution for replay
    pub async fn store(
        &self,
        user_id: String,
        request: ExecuteRequest,
        response: ExecuteResponse,
    ) -> Result<Uuid> {
        let execution_id = response.execution_id;

        let stored = StoredExecution {
            execution_id,
            user_id,
            timestamp: Utc::now(),
            request: request.clone(),
            response,
            code: request.code.clone(),
        };

        // Store in memory
        {
            let mut executions = self.executions.write().await;

            // Limit stored executions
            if executions.len() >= self.max_stored {
                // Remove oldest
                if let Some(oldest_id) = executions
                    .values()
                    .min_by_key(|e| e.timestamp)
                    .map(|e| e.execution_id)
                {
                    executions.remove(&oldest_id);
                }
            }

            executions.insert(execution_id, stored.clone());
        }

        // Persist to file
        self.save_to_file(&stored).await?;

        Ok(execution_id)
    }

    /// Get a stored execution
    pub async fn get(&self, execution_id: &Uuid) -> Option<StoredExecution> {
        self.executions.read().await.get(execution_id).cloned()
    }

    /// List all stored executions
    pub async fn list_all(&self) -> Vec<StoredExecution> {
        let mut executions: Vec<_> = self.executions.read().await.values().cloned().collect();
        executions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        executions
    }

    /// List executions for a specific user
    pub async fn list_user_executions(&self, user_id: &str) -> Vec<StoredExecution> {
        let executions = self.executions.read().await;
        let mut user_execs: Vec<_> = executions
            .values()
            .filter(|e| e.user_id == user_id)
            .cloned()
            .collect();
        user_execs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        user_execs
    }

    /// Replay an execution (execute the same code again)
    pub async fn replay(&self, execution_id: &Uuid) -> Option<ExecuteRequest> {
        self.executions
            .read()
            .await
            .get(execution_id)
            .map(|e| e.request.clone())
    }

    /// Save execution to file
    async fn save_to_file(&self, execution: &StoredExecution) -> Result<()> {
        let filename = format!("{}.json", execution.execution_id);
        let file_path = self.storage_dir.join(filename);

        let json = serde_json::to_string_pretty(execution)
            .context("Failed to serialize execution")?;

        tokio::fs::write(&file_path, json)
            .await
            .context("Failed to write replay file")?;

        Ok(())
    }

    /// Load all replays from storage directory
    async fn load_all(&self) -> Result<()> {
        let mut entries = tokio::fs::read_dir(&self.storage_dir).await?;

        let mut loaded = 0;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    if let Ok(execution) = serde_json::from_str::<StoredExecution>(&content) {
                        self.executions
                            .write()
                            .await
                            .insert(execution.execution_id, execution);
                        loaded += 1;
                    }
                }
            }
        }

        tracing::info!(loaded = loaded, "Loaded stored executions for replay");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_replay_manager() {
        let temp_dir = std::env::temp_dir().join("cage_test_replay");
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();

        let manager = ReplayManager::new(&temp_dir, 100).await.unwrap();

        let request = ExecuteRequest {
            language: crate::models::Language::Python,
            code: "print('test')".to_string(),
            timeout_seconds: 30,
            working_dir: None,
            env: HashMap::new(),
            persistent: false,
        };

        let response = ExecuteResponse {
            execution_id: Uuid::new_v4(),
            status: crate::models::ExecutionStatus::Success,
            stdout: "test\n".to_string(),
            stderr: String::new(),
            exit_code: Some(0),
            duration_ms: 100,
            files_created: vec![],
            resource_usage: None,
        };

        let exec_id = manager
            .store("test_user".to_string(), request.clone(), response)
            .await
            .unwrap();

        // Retrieve
        let stored = manager.get(&exec_id).await;
        assert!(stored.is_some());
        assert_eq!(stored.unwrap().code, "print('test')");

        // Replay
        let replay_req = manager.replay(&exec_id).await;
        assert!(replay_req.is_some());
        assert_eq!(replay_req.unwrap().code, "print('test')");

        // Cleanup
        tokio::fs::remove_dir_all(&temp_dir).await.ok();
    }
}
