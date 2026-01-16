//! Configuration module for CAGE Orchestrator
//!
//! Supports configuration via:
//! - YAML/TOML config files
//! - Environment variables (with CAGE_ prefix)
//! - Command line arguments (future)

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Host to bind to
    #[serde(default = "default_host")]
    pub host: String,

    /// Port to listen on
    #[serde(default = "default_port")]
    pub port: u16,

    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// Path to store user workspaces
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,

    /// Container image to use for sandboxes
    #[serde(default = "default_sandbox_image")]
    pub sandbox_image: String,

    /// Podman socket path (if using API instead of CLI)
    #[serde(default)]
    pub podman_socket: Option<String>,

    /// Stop containers when orchestrator shuts down
    #[serde(default = "default_true")]
    pub stop_containers_on_shutdown: bool,

    /// Default resource limits for containers
    #[serde(default)]
    pub default_limits: ResourceLimits,

    /// Default network policy
    #[serde(default)]
    pub default_network: NetworkPolicy,

    /// Security settings
    #[serde(default)]
    pub security: SecurityConfig,

    /// User configurations (can be loaded from separate file)
    #[serde(default)]
    pub users: HashMap<String, UserConfig>,

    /// Admin API settings
    #[serde(default)]
    pub admin: AdminConfig,

    /// Metrics settings
    #[serde(default)]
    pub metrics: MetricsConfig,

    /// gVisor runtime settings
    #[serde(default)]
    pub gvisor: crate::gvisor::GVisorConfig,

    /// Distributed multi-node settings
    #[serde(default)]
    pub distributed: crate::distributed::DistributedConfig,

    /// Dynamic package installation settings
    #[serde(default)]
    pub packages: crate::packages::PackageConfig,

    /// Jaeger distributed tracing settings
    #[serde(default)]
    pub jaeger: crate::jaeger::JaegerConfig,

    /// OPA policy settings
    #[serde(default)]
    pub opa: crate::opa::OpaConfig,

    /// HashiCorp Vault settings
    #[serde(default)]
    pub vault: crate::vault::VaultConfig,

    /// Code signing settings
    #[serde(default)]
    pub signing: crate::signing::SigningConfig,

    /// Alert system settings
    #[serde(default)]
    pub alerts: crate::alerts::AlertConfig,

    /// Network egress proxy settings
    #[serde(default)]
    pub proxy: crate::proxy::ProxyConfig,
}

/// Resource limits for a container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory in megabytes
    #[serde(default = "default_memory_mb")]
    pub max_memory_mb: u64,

    /// Maximum CPU cores (can be fractional)
    #[serde(default = "default_cpus")]
    pub max_cpus: f64,

    /// Maximum number of processes
    #[serde(default = "default_pids")]
    pub max_pids: u32,

    /// Maximum execution time in seconds
    #[serde(default = "default_timeout")]
    pub max_execution_seconds: u64,

    /// Maximum disk usage in megabytes
    #[serde(default = "default_disk_mb")]
    pub max_disk_mb: u64,
}

/// Network policy for a container
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkPolicy {
    /// Whether network access is enabled
    #[serde(default)]
    pub enabled: bool,

    /// Allowed destination hosts/IPs
    #[serde(default)]
    pub allowed_hosts: Vec<String>,

    /// Allowed destination ports
    #[serde(default)]
    pub allowed_ports: Vec<u16>,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Path to custom seccomp profile
    #[serde(default)]
    pub seccomp_profile: Option<PathBuf>,

    /// Enable read-only root filesystem
    #[serde(default = "default_true")]
    pub read_only_rootfs: bool,

    /// Drop all capabilities
    #[serde(default = "default_true")]
    pub drop_all_caps: bool,

    /// Enable no-new-privileges
    #[serde(default = "default_true")]
    pub no_new_privileges: bool,

    /// JWT secret for API authentication
    #[serde(default = "default_jwt_secret")]
    pub jwt_secret: String,

    /// JWT token expiration in seconds
    #[serde(default = "default_jwt_expiration")]
    pub jwt_expiration_seconds: u64,

    /// Admin API token (for TUI access)
    #[serde(default)]
    pub admin_token: Option<String>,
}

/// Per-user configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    /// User ID
    pub user_id: String,

    /// API key (hashed with Argon2)
    #[serde(default)]
    pub api_key_hash: Option<String>,

    /// Whether the user is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Custom resource limits (overrides defaults)
    #[serde(default)]
    pub resource_limits: Option<ResourceLimits>,

    /// Custom network policy (overrides defaults)
    #[serde(default)]
    pub network_policy: Option<NetworkPolicy>,

    /// Allowed languages
    #[serde(default = "default_languages")]
    pub allowed_languages: Vec<String>,

    /// Enable GPU access
    #[serde(default)]
    pub gpu_enabled: bool,
}

/// Admin API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfig {
    /// Enable admin API endpoints
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Require authentication for admin endpoints
    #[serde(default = "default_true")]
    pub require_auth: bool,

    /// Admin users (user_id -> is_admin)
    #[serde(default)]
    pub admin_users: Vec<String>,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable Prometheus metrics endpoint
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Metrics endpoint path
    #[serde(default = "default_metrics_path")]
    pub path: String,
}

// Default value functions
fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_data_dir() -> PathBuf {
    PathBuf::from("/var/lib/cage")
}

fn default_sandbox_image() -> String {
    "cage-sandbox:latest".to_string()
}

fn default_memory_mb() -> u64 {
    1024
}

fn default_cpus() -> f64 {
    1.0
}

fn default_pids() -> u32 {
    100
}

fn default_timeout() -> u64 {
    30
}

fn default_disk_mb() -> u64 {
    1024
}

fn default_true() -> bool {
    true
}

fn default_jwt_secret() -> String {
    // Generate a random secret if not configured
    // In production, this MUST be set explicitly
    use rand::Rng;
    let secret: [u8; 32] = rand::thread_rng().gen();
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, secret)
}

fn default_jwt_expiration() -> u64 {
    3600 // 1 hour
}

fn default_languages() -> Vec<String> {
    vec!["python".to_string()]
}

fn default_metrics_path() -> String {
    "/metrics".to_string()
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: default_memory_mb(),
            max_cpus: default_cpus(),
            max_pids: default_pids(),
            max_execution_seconds: default_timeout(),
            max_disk_mb: default_disk_mb(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            seccomp_profile: None,
            read_only_rootfs: true,
            drop_all_caps: true,
            no_new_privileges: true,
            jwt_secret: default_jwt_secret(),
            jwt_expiration_seconds: default_jwt_expiration(),
            admin_token: None,
        }
    }
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            require_auth: true,
            admin_users: vec![],
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            path: default_metrics_path(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            log_level: default_log_level(),
            data_dir: default_data_dir(),
            sandbox_image: default_sandbox_image(),
            podman_socket: None,
            stop_containers_on_shutdown: true,
            default_limits: ResourceLimits::default(),
            default_network: NetworkPolicy::default(),
            security: SecurityConfig::default(),
            users: HashMap::new(),
            admin: AdminConfig::default(),
            metrics: MetricsConfig::default(),
            gvisor: crate::gvisor::GVisorConfig::default(),
            distributed: crate::distributed::DistributedConfig::default(),
            packages: crate::packages::PackageConfig::default(),
            jaeger: crate::jaeger::JaegerConfig::default(),
            opa: crate::opa::OpaConfig::default(),
            vault: crate::vault::VaultConfig::default(),
            signing: crate::signing::SigningConfig::default(),
            alerts: crate::alerts::AlertConfig::default(),
            proxy: crate::proxy::ProxyConfig::default(),
        }
    }
}

impl AppConfig {
    /// Load configuration from file and environment variables
    pub fn load() -> Result<Self> {
        // Try to load .env file if present
        let _ = dotenvy::dotenv();

        // Build configuration
        let config = config::Config::builder()
            // Start with defaults
            .add_source(config::Config::try_from(&AppConfig::default())?)
            // Load from config file if present
            .add_source(
                config::File::with_name("config/cage")
                    .required(false)
            )
            .add_source(
                config::File::with_name("/etc/cage/config")
                    .required(false)
            )
            // Override with environment variables (CAGE_ prefix)
            .add_source(
                config::Environment::with_prefix("CAGE")
                    .separator("__")
                    .try_parsing(true)
            )
            .build()
            .context("Failed to build configuration")?;

        let app_config: AppConfig = config.try_deserialize()
            .context("Failed to deserialize configuration")?;

        // Validate configuration
        app_config.validate()?;

        Ok(app_config)
    }

    /// Validate configuration values
    fn validate(&self) -> Result<()> {
        if self.port == 0 {
            anyhow::bail!("Port cannot be 0");
        }

        if self.default_limits.max_memory_mb < 64 {
            anyhow::bail!("Minimum memory limit is 64MB");
        }

        if self.default_limits.max_execution_seconds == 0 {
            anyhow::bail!("Execution timeout cannot be 0");
        }

        if self.security.jwt_secret.len() < 16 {
            anyhow::bail!("JWT secret must be at least 16 characters");
        }

        Ok(())
    }

    /// Get effective limits for a user (user-specific or defaults)
    pub fn get_user_limits(&self, user_id: &str) -> ResourceLimits {
        self.users
            .get(user_id)
            .and_then(|u| u.resource_limits.clone())
            .unwrap_or_else(|| self.default_limits.clone())
    }

    /// Get effective network policy for a user
    pub fn get_user_network(&self, user_id: &str) -> NetworkPolicy {
        self.users
            .get(user_id)
            .and_then(|u| u.network_policy.clone())
            .unwrap_or_else(|| self.default_network.clone())
    }

    /// Check if a user is enabled
    pub fn is_user_enabled(&self, user_id: &str) -> bool {
        self.users
            .get(user_id)
            .map(|u| u.enabled)
            .unwrap_or(true) // Allow users not in config by default
    }

    /// Check if a user is an admin
    pub fn is_admin(&self, user_id: &str) -> bool {
        self.admin.admin_users.contains(&user_id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.port, 8080);
        assert_eq!(config.default_limits.max_memory_mb, 1024);
        assert!(!config.default_network.enabled);
    }

    #[test]
    fn test_validation() {
        let config = AppConfig {
            port: 0,
            ..AppConfig::default()
        };
        assert!(config.validate().is_err());
    }
}
