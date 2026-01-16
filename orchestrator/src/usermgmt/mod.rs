//! Dynamic user management with persistence
//!
//! Allows adding/updating/deleting users at runtime via API

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::config::UserConfig;

/// User management database (persisted to JSON file)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct UserDatabase {
    users: HashMap<String, UserConfig>,
    #[serde(skip)]
    file_path: PathBuf,
}

/// Thread-safe user manager
pub struct UserManager {
    database: Arc<RwLock<UserDatabase>>,
}

impl UserManager {
    /// Create new user manager and load from file
    pub async fn new(data_dir: &Path) -> Result<Self> {
        let file_path = data_dir.join("users.json");

        let mut database = if file_path.exists() {
            let content = tokio::fs::read_to_string(&file_path)
                .await
                .context("Failed to read users file")?;
            serde_json::from_str(&content)
                .context("Failed to parse users file")?
        } else {
            UserDatabase::default()
        };

        database.file_path = file_path.clone();

        info!(
            path = %file_path.display(),
            user_count = database.users.len(),
            "User database loaded"
        );

        Ok(Self {
            database: Arc::new(RwLock::new(database)),
        })
    }

    /// Get all users
    pub async fn list_users(&self) -> Vec<UserConfig> {
        self.database
            .read()
            .await
            .users
            .values()
            .cloned()
            .collect()
    }

    /// Get a specific user
    pub async fn get_user(&self, user_id: &str) -> Option<UserConfig> {
        self.database.read().await.users.get(user_id).cloned()
    }

    /// Create or update a user
    pub async fn upsert_user(&self, user: UserConfig) -> Result<bool> {
        let user_id = user.user_id.clone();
        let is_new = !self.database.read().await.users.contains_key(&user_id);

        {
            let mut db = self.database.write().await;
            db.users.insert(user_id.clone(), user);

            // Persist to file
            self.save_database(&db).await?;
        }

        if is_new {
            info!(user_id = %user_id, "User created");
        } else {
            info!(user_id = %user_id, "User updated");
        }

        Ok(is_new)
    }

    /// Delete a user
    pub async fn delete_user(&self, user_id: &str) -> Result<bool> {
        let existed = {
            let mut db = self.database.write().await;
            let result = db.users.remove(user_id).is_some();

            if result {
                // Persist to file
                self.save_database(&db).await?;
            }

            result
        };

        if existed {
            info!(user_id = %user_id, "User deleted");
        } else {
            warn!(user_id = %user_id, "User not found for deletion");
        }

        Ok(existed)
    }

    /// Check if user exists
    pub async fn user_exists(&self, user_id: &str) -> bool {
        self.database.read().await.users.contains_key(user_id)
    }

    /// Save database to file
    async fn save_database(&self, db: &UserDatabase) -> Result<()> {
        let json = serde_json::to_string_pretty(db)
            .context("Failed to serialize user database")?;

        tokio::fs::write(&db.file_path, json)
            .await
            .context("Failed to write users file")?;

        Ok(())
    }

    /// Reload users from config file (for hot-reload integration)
    pub async fn reload_from_config(&self, config_users: HashMap<String, UserConfig>) -> Result<()> {
        let mut db = self.database.write().await;

        // Merge config users with dynamic users
        // Config users take precedence
        for (user_id, user) in config_users {
            db.users.insert(user_id, user);
        }

        info!(user_count = db.users.len(), "Users reloaded from config");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ResourceLimits, NetworkPolicy};

    #[tokio::test]
    async fn test_user_management() {
        let temp_dir = std::env::temp_dir().join("cage_test_users");
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();

        let manager = UserManager::new(&temp_dir).await.unwrap();

        // Create user
        let user = UserConfig {
            user_id: "test_user".to_string(),
            api_key_hash: Some("hash123".to_string()),
            enabled: true,
            resource_limits: Some(ResourceLimits::default()),
            network_policy: Some(NetworkPolicy::default()),
            allowed_languages: vec!["python".to_string()],
            gpu_enabled: false,
        };

        let is_new = manager.upsert_user(user.clone()).await.unwrap();
        assert!(is_new);

        // Get user
        let retrieved = manager.get_user("test_user").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().user_id, "test_user");

        // Update user
        let mut updated_user = user;
        updated_user.enabled = false;
        let is_new = manager.upsert_user(updated_user).await.unwrap();
        assert!(!is_new);

        // Delete user
        let deleted = manager.delete_user("test_user").await.unwrap();
        assert!(deleted);

        // Verify deleted
        assert!(!manager.user_exists("test_user").await);

        // Cleanup
        tokio::fs::remove_dir_all(&temp_dir).await.ok();
    }
}
