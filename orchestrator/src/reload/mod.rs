//! Configuration hot-reload without restarting the server
//!
//! Watches configuration file and reloads on changes

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{info, warn};

use crate::config::AppConfig;

/// Configuration reloader
pub struct ConfigReloader {
    config: Arc<RwLock<AppConfig>>,
    config_path: PathBuf,
    last_modified: Arc<RwLock<Option<std::time::SystemTime>>>,
}

impl ConfigReloader {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            config_path: PathBuf::from("config/cage.yaml"),
            last_modified: Arc::new(RwLock::new(None)),
        }
    }

    /// Get the current configuration
    pub async fn get_config(&self) -> AppConfig {
        self.config.read().await.clone()
    }

    /// Start watching for configuration changes
    pub async fn start_watching(self: Arc<Self>) {
        info!("Starting configuration file watcher");

        loop {
            sleep(Duration::from_secs(30)).await;

            if let Err(e) = self.check_and_reload().await {
                warn!(error = %e, "Failed to reload configuration");
            }
        }
    }

    /// Check if config file changed and reload
    async fn check_and_reload(&self) -> Result<()> {
        let metadata = match tokio::fs::metadata(&self.config_path).await {
            Ok(m) => m,
            Err(_) => return Ok(()), // Config file doesn't exist, skip
        };

        let modified = metadata.modified()?;

        let last_mod = *self.last_modified.read().await;

        if last_mod.is_none() {
            // First check
            *self.last_modified.write().await = Some(modified);
            return Ok(());
        }

        if Some(modified) != last_mod {
            // File changed, reload
            info!("Configuration file changed, reloading...");

            match AppConfig::load() {
                Ok(new_config) => {
                    *self.config.write().await = new_config;
                    *self.last_modified.write().await = Some(modified);
                    info!("Configuration reloaded successfully");
                }
                Err(e) => {
                    warn!(error = %e, "Failed to load new configuration, keeping old config");
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;

    #[test]
    fn test_config_reloader_creation() {
        let config = AppConfig::default();
        let reloader = ConfigReloader::new(config);
        assert!(reloader.config_path.to_str().unwrap().contains("cage.yaml"));
    }
}
