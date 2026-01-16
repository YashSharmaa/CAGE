//! Dynamic package installation system
//!
//! Allows users to install vetted packages from an internal registry

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::info;

use crate::models::Language;

/// Package installation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageConfig {
    /// Enable dynamic package installation
    pub enabled: bool,
    /// Internal PyPI mirror URL
    pub pypi_mirror: Option<String>,
    /// Internal npm registry URL
    pub npm_registry: Option<String>,
    /// Internal CRAN mirror URL
    pub cran_mirror: Option<String>,
    /// Maximum packages per session
    pub max_packages_per_session: usize,
}

impl Default for PackageConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            pypi_mirror: None,
            npm_registry: None,
            cran_mirror: None,
            max_packages_per_session: 50,
        }
    }
}

/// Package allowlist manager
pub struct PackageManager {
    config: PackageConfig,
    python_allowlist: Arc<RwLock<HashSet<String>>>,
    npm_allowlist: Arc<RwLock<HashSet<String>>>,
    r_allowlist: Arc<RwLock<HashSet<String>>>,
    installed_packages: Arc<RwLock<HashMap<String, HashSet<String>>>>, // user_id -> packages
}

impl PackageManager {
    pub fn new(config: PackageConfig) -> Self {
        let mut python_allowlist = HashSet::new();
        let mut npm_allowlist = HashSet::new();
        let mut r_allowlist = HashSet::new();

        // Pre-populate with safe, commonly used packages
        // Python
        python_allowlist.extend([
            "requests", "beautifulsoup4", "lxml", "pillow", "openpyxl",
            "python-dateutil", "pytz", "tabulate", "tqdm", "jinja2",
            "pyyaml", "toml", "python-dotenv", "regex", "chardet",
            "jsonschema", "orjson",
        ].iter().map(|s| s.to_string()));

        // NPM
        npm_allowlist.extend([
            "lodash", "moment", "axios", "express", "chalk",
            "commander", "inquirer", "ora", "cli-table3",
        ].iter().map(|s| s.to_string()));

        // R
        r_allowlist.extend([
            "jsonlite", "httr", "xml2", "lubridate", "stringr",
            "readxl", "writexl", "glue",
        ].iter().map(|s| s.to_string()));

        info!(
            python_packages = python_allowlist.len(),
            npm_packages = npm_allowlist.len(),
            r_packages = r_allowlist.len(),
            "Package allowlists initialized"
        );

        Self {
            config,
            python_allowlist: Arc::new(RwLock::new(python_allowlist)),
            npm_allowlist: Arc::new(RwLock::new(npm_allowlist)),
            r_allowlist: Arc::new(RwLock::new(r_allowlist)),
            installed_packages: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if a package is allowed
    pub async fn is_allowed(&self, package: &str, language: Language) -> bool {
        match language {
            Language::Python => self.python_allowlist.read().await.contains(package),
            Language::Javascript => self.npm_allowlist.read().await.contains(package),
            Language::R => self.r_allowlist.read().await.contains(package),
            _ => false,
        }
    }

    /// Install a package in a container
    pub async fn install_package(
        &self,
        user_id: &str,
        container_id: &str,
        package: &str,
        language: Language,
        podman_path: &str,
    ) -> Result<String> {
        if !self.config.enabled {
            anyhow::bail!("Dynamic package installation is disabled");
        }

        // Check allowlist
        if !self.is_allowed(package, language).await {
            anyhow::bail!("Package '{}' is not in the allowlist", package);
        }

        // Check session package limit
        {
            let installed = self.installed_packages.read().await;
            if let Some(packages) = installed.get(user_id) {
                if packages.len() >= self.config.max_packages_per_session {
                    anyhow::bail!("Maximum packages ({}) already installed", self.config.max_packages_per_session);
                }
            }
        }

        info!(user_id = %user_id, package = %package, language = %language.as_str(), "Installing package");

        // Build install command
        let install_cmd = match language {
            Language::Python => {
                let mirror = self.config.pypi_mirror.as_deref().unwrap_or("https://pypi.org/simple");
                format!(
                    "pip install --no-cache-dir --index-url {} --trusted-host {} {}",
                    mirror,
                    mirror.split("://").nth(1).unwrap_or("pypi.org").split('/').next().unwrap_or("pypi.org"),
                    package
                )
            }
            Language::Javascript => {
                let registry = self.config.npm_registry.as_deref().unwrap_or("https://registry.npmjs.org");
                format!("npm install --registry {} {}", registry, package)
            }
            Language::R => {
                let mirror = self.config.cran_mirror.as_deref().unwrap_or("https://cran.rstudio.com");
                format!("R -e \"install.packages('{}', repos='{}')\"", package, mirror)
            }
            _ => anyhow::bail!("Package installation not supported for {:?}", language),
        };

        // Execute installation in container
        let output = Command::new(podman_path)
            .args(["exec", "-u", "sandbox", container_id, "bash", "-c", &install_cmd])
            .output()
            .await
            .context("Failed to execute package install")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Package installation failed: {}", stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        // Track installed package
        {
            let mut installed = self.installed_packages.write().await;
            installed
                .entry(user_id.to_string())
                .or_insert_with(HashSet::new)
                .insert(package.to_string());
        }

        info!(user_id = %user_id, package = %package, "Package installed successfully");

        Ok(stdout)
    }

    /// List installed packages for a user
    pub async fn list_installed(&self, user_id: &str) -> Vec<String> {
        self.installed_packages
            .read()
            .await
            .get(user_id)
            .map(|packages| packages.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Add package to allowlist (admin function)
    pub async fn add_to_allowlist(&self, package: String, language: Language) -> Result<()> {
        match language {
            Language::Python => {
                self.python_allowlist.write().await.insert(package.clone());
            }
            Language::Javascript => {
                self.npm_allowlist.write().await.insert(package.clone());
            }
            Language::R => {
                self.r_allowlist.write().await.insert(package.clone());
            }
            _ => anyhow::bail!("Language not supported"),
        }

        info!(package = %package, language = %language.as_str(), "Added package to allowlist");

        Ok(())
    }

    /// Get allowlist for a language
    pub async fn get_allowlist(&self, language: Language) -> Vec<String> {
        match language {
            Language::Python => self.python_allowlist.read().await.iter().cloned().collect(),
            Language::Javascript => self.npm_allowlist.read().await.iter().cloned().collect(),
            Language::R => self.r_allowlist.read().await.iter().cloned().collect(),
            _ => vec![],
        }
    }

    /// Clear installed packages for a user (on session reset)
    pub async fn clear_user_packages(&self, user_id: &str) {
        self.installed_packages.write().await.remove(user_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_package_allowlist() {
        let config = PackageConfig::default();
        let manager = PackageManager::new(config);

        assert!(manager.is_allowed("requests", Language::Python).await);
        assert!(manager.is_allowed("lodash", Language::Javascript).await);
        assert!(!manager.is_allowed("unknown-package", Language::Python).await);
    }

    #[tokio::test]
    async fn test_add_to_allowlist() {
        let config = PackageConfig::default();
        let manager = PackageManager::new(config);

        manager
            .add_to_allowlist("my-package".to_string(), Language::Python)
            .await
            .unwrap();

        assert!(manager.is_allowed("my-package", Language::Python).await);
    }
}
