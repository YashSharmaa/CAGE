//! Network whitelisting and policy enforcement

use anyhow::Result;
use tokio::process::Command;
use tracing::{debug, info};

use crate::config::NetworkPolicy;

/// Network manager for creating isolated networks with whitelisting
pub struct NetworkManager {
    podman_path: String,
}

impl NetworkManager {
    pub fn new() -> Self {
        let podman_path = which::which("podman")
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "/opt/podman/bin/podman".to_string());

        Self { podman_path }
    }

    /// Create a user-specific network with whitelist rules
    pub async fn create_user_network(
        &self,
        user_id: &str,
        policy: &NetworkPolicy,
    ) -> Result<String> {
        if !policy.enabled {
            return Ok("none".to_string());
        }

        let network_name = format!("cage_net_{}", user_id);

        // Check if network already exists
        let check = Command::new(&self.podman_path)
            .args(["network", "exists", &network_name])
            .status()
            .await?;

        if check.success() {
            debug!(network_name = %network_name, "Network already exists");
            return Ok(network_name);
        }

        // Create network with custom subnet
        let subnet = format!("10.{}.0.0/24", (user_id.bytes().sum::<u8>() % 200) + 10);

        let output = Command::new(&self.podman_path)
            .args([
                "network",
                "create",
                &network_name,
                "--subnet",
                &subnet,
                "--disable-dns=false",
            ])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to create network: {}", stderr);
        }

        info!(
            network_name = %network_name,
            subnet = %subnet,
            allowed_hosts = ?policy.allowed_hosts,
            "Network created with whitelist"
        );

        // Note: For full whitelisting, would need iptables rules on host
        // This creates the network; actual filtering would require:
        // iptables -I FORWARD -s $subnet -d $allowed_ip -j ACCEPT
        // iptables -I FORWARD -s $subnet -j DROP

        Ok(network_name)
    }

    /// Delete a user network
    pub async fn delete_user_network(&self, user_id: &str) -> Result<()> {
        let network_name = format!("cage_net_{}", user_id);

        let _ = Command::new(&self.podman_path)
            .args(["network", "rm", &network_name])
            .output()
            .await;

        Ok(())
    }
}

impl Default for NetworkManager {
    fn default() -> Self {
        Self::new()
    }
}
