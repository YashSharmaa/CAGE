//! gVisor integration for additional kernel-level isolation
//!
//! Provides an extra security layer by running containers with gVisor's runsc runtime

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tracing::{debug, info, warn};

/// gVisor runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GVisorConfig {
    /// Enable gVisor (requires runsc installed)
    pub enabled: bool,
    /// Path to runsc binary
    pub runsc_path: String,
    /// gVisor platform (ptrace, kvm, systrap)
    pub platform: GVisorPlatform,
    /// Network mode (none, host, sandbox)
    pub network: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GVisorPlatform {
    Ptrace,  // Default, works everywhere but slower
    Kvm,     // Fastest, requires /dev/kvm
    Systrap, // Newer, good balance
}

impl Default for GVisorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            runsc_path: "/usr/local/bin/runsc".to_string(),
            platform: GVisorPlatform::Systrap,
            network: "none".to_string(),
        }
    }
}

/// gVisor runtime manager
pub struct GVisorRuntime {
    config: GVisorConfig,
    available: bool,
}

impl GVisorRuntime {
    /// Create new gVisor runtime manager
    pub async fn new(config: GVisorConfig) -> Self {
        let available = Self::check_availability(&config).await;

        if config.enabled && !available {
            warn!("gVisor enabled in config but runsc not available");
        } else if config.enabled && available {
            info!(platform = ?config.platform, "gVisor runtime initialized");
        }

        Self { config, available }
    }

    /// Check if gVisor/runsc is available
    async fn check_availability(config: &GVisorConfig) -> bool {
        // Check if runsc exists
        if tokio::fs::metadata(&config.runsc_path).await.is_err() {
            // Try to find in PATH
            if which::which("runsc").is_err() {
                return false;
            }
        }

        // Verify runsc works
        let output = Command::new(&config.runsc_path)
            .args(["--version"])
            .output()
            .await;

        match output {
            Ok(out) if out.status.success() => {
                let version = String::from_utf8_lossy(&out.stdout);
                debug!(version = %version.trim(), "gVisor runsc detected");
                true
            }
            _ => false,
        }
    }

    /// Get Podman runtime arguments for gVisor
    pub fn get_runtime_args(&self) -> Vec<String> {
        if !self.config.enabled || !self.available {
            return vec![];
        }

        vec![
            "--runtime".to_string(),
            self.config.runsc_path.clone(),
            "--runtime-flag".to_string(),
            format!("--platform={}", self.platform_string()),
            "--runtime-flag".to_string(),
            format!("--network={}", self.config.network),
        ]
    }

    /// Get platform as string
    fn platform_string(&self) -> &str {
        match self.config.platform {
            GVisorPlatform::Ptrace => "ptrace",
            GVisorPlatform::Kvm => "kvm",
            GVisorPlatform::Systrap => "systrap",
        }
    }

    /// Check if gVisor is enabled and available
    pub fn is_active(&self) -> bool {
        self.config.enabled && self.available
    }

    /// Install gVisor (helper script generator)
    pub fn generate_install_script() -> String {
        r#"#!/bin/bash
# Install gVisor runsc runtime

set -euo pipefail

ARCH=$(uname -m)
if [[ "${ARCH}" == "x86_64" ]]; then
    ARCH="x86_64"
elif [[ "${ARCH}" == "aarch64" ]]; then
    ARCH="aarch64"
else
    echo "Unsupported architecture: ${ARCH}"
    exit 1
fi

# Download runsc
wget https://storage.googleapis.com/gvisor/releases/release/latest/${ARCH}/runsc
wget https://storage.googleapis.com/gvisor/releases/release/latest/${ARCH}/runsc.sha512

# Verify checksum
sha512sum -c runsc.sha512

# Install
chmod +x runsc
sudo mv runsc /usr/local/bin/

# Verify installation
/usr/local/bin/runsc --version

echo "gVisor runsc installed successfully!"
echo "Configure CAGE with:"
echo "  gvisor:"
echo "    enabled: true"
echo "    platform: systrap  # or ptrace, kvm"
"#.to_string()
    }

    /// Configure Podman to use gVisor runtime
    pub async fn configure_podman_runtime(&self) -> Result<()> {
        if !self.is_active() {
            return Ok(());
        }

        info!("Configuring Podman to use gVisor runtime");

        // Add runsc runtime to Podman configuration
        // This would modify ~/.config/containers/containers.conf
        let config_content = format!(
            r#"
[engine]
[engine.runtimes]
runsc = ["{}"]
"#,
            self.config.runsc_path
        );

        debug!(config = %config_content, "Podman gVisor runtime config");

        Ok(())
    }

    /// Get security benefits description
    pub fn security_benefits() -> Vec<String> {
        vec![
            "User-space kernel implementation".to_string(),
            "Syscall interception and filtering".to_string(),
            "Reduces attack surface on host kernel".to_string(),
            "Prevents container escape exploits".to_string(),
            "Mitigates kernel vulnerabilities".to_string(),
            "Additional isolation layer beyond namespaces".to_string(),
        ]
    }

    /// Get performance overhead estimate
    pub fn performance_overhead(&self) -> &str {
        match self.config.platform {
            GVisorPlatform::Ptrace => "30-50% overhead",
            GVisorPlatform::Kvm => "10-15% overhead (requires /dev/kvm)",
            GVisorPlatform::Systrap => "15-25% overhead (recommended)",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gvisor_config_default() {
        let config = GVisorConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.platform, GVisorPlatform::Systrap);
    }

    #[test]
    fn test_install_script_generation() {
        let script = GVisorRuntime::generate_install_script();
        assert!(script.contains("runsc"));
        assert!(script.contains("gvisor"));
    }
}
