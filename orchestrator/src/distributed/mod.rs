//! Distributed multi-node orchestration
//!
//! Enables horizontal scaling across multiple orchestrator nodes using Redis for state

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Distributed state configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedConfig {
    /// Enable distributed mode
    pub enabled: bool,
    /// Redis connection URL
    pub redis_url: String,
    /// This node's ID (auto-generated if not set)
    pub node_id: Option<String>,
    /// Heartbeat interval in seconds
    pub heartbeat_interval: u64,
    /// Node considered dead after this many seconds without heartbeat
    pub node_timeout: u64,
}

impl Default for DistributedConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            redis_url: "redis://127.0.0.1:6379".to_string(),
            node_id: None,
            heartbeat_interval: 5,
            node_timeout: 30,
        }
    }
}

/// Node information for cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_id: String,
    pub address: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    pub active_sessions: u64,
    pub total_executions: u64,
    pub status: NodeStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeStatus {
    Active,
    Degraded,
    Dead,
}

/// Session routing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRoute {
    pub user_id: String,
    pub node_id: String,
    pub container_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Distributed orchestrator state manager
pub struct DistributedStateManager {
    config: DistributedConfig,
    node_id: String,
    #[allow(dead_code)]
    redis_client: Option<RedisClient>,
    local_state: Arc<RwLock<LocalState>>,
}

/// Redis client wrapper using redis-rs
struct RedisClient {
    connection: redis::aio::ConnectionManager,
}

impl RedisClient {
    async fn new(url: String) -> Result<Self> {
        let client = redis::Client::open(url.as_str())
            .context("Failed to create Redis client")?;

        let connection = redis::aio::ConnectionManager::new(client)
            .await
            .context("Failed to connect to Redis")?;

        info!("Connected to Redis at {}", url);
        Ok(Self { connection })
    }

    #[allow(dead_code)]
    async fn set(&self, key: &str, value: &str, expiry: Option<Duration>) -> Result<()> {
        let mut conn = self.connection.clone();

        if let Some(exp) = expiry {
            redis::cmd("SETEX")
                .arg(key)
                .arg(exp.as_secs())
                .arg(value)
                .query_async::<_, ()>(&mut conn)
                .await
                .context("Redis SETEX failed")?;
        } else {
            redis::cmd("SET")
                .arg(key)
                .arg(value)
                .query_async::<_, ()>(&mut conn)
                .await
                .context("Redis SET failed")?;
        }

        debug!(key = %key, expiry = ?expiry, "Redis SET");
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<String>> {
        let mut conn = self.connection.clone();

        let value: Option<String> = redis::cmd("GET")
            .arg(key)
            .query_async(&mut conn)
            .await
            .context("Redis GET failed")?;

        debug!(key = %key, found = value.is_some(), "Redis GET");
        Ok(value)
    }

    async fn hset(&self, hash: &str, field: &str, value: &str) -> Result<()> {
        let mut conn = self.connection.clone();

        redis::cmd("HSET")
            .arg(hash)
            .arg(field)
            .arg(value)
            .query_async::<_, ()>(&mut conn)
            .await
            .context("Redis HSET failed")?;

        debug!(hash = %hash, field = %field, "Redis HSET");
        Ok(())
    }

    async fn hgetall(&self, hash: &str) -> Result<HashMap<String, String>> {
        let mut conn = self.connection.clone();

        let result: HashMap<String, String> = redis::cmd("HGETALL")
            .arg(hash)
            .query_async(&mut conn)
            .await
            .context("Redis HGETALL failed")?;

        debug!(hash = %hash, count = result.len(), "Redis HGETALL");
        Ok(result)
    }
}

#[derive(Debug, Default)]
struct LocalState {
    sessions: HashMap<String, SessionRoute>,
    nodes: HashMap<String, NodeInfo>,
}

impl DistributedStateManager {
    /// Get this node's ID
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// Create new distributed state manager
    pub async fn new(config: DistributedConfig) -> Result<Self> {
        let node_id = config
            .node_id
            .clone()
            .unwrap_or_else(|| format!("node-{}", Uuid::new_v4().simple()));

        let redis_client = if config.enabled {
            match RedisClient::new(config.redis_url.clone()).await {
                Ok(client) => {
                    info!(node_id = %node_id, redis_url = %config.redis_url, "Connected to Redis for distributed state");
                    Some(client)
                }
                Err(e) => {
                    error!(error = %e, "Failed to connect to Redis, running in standalone mode");
                    None
                }
            }
        } else {
            None
        };

        Ok(Self {
            config,
            node_id,
            redis_client,
            local_state: Arc::new(RwLock::new(LocalState::default())),
        })
    }

    /// Register this node in the cluster
    pub async fn register_node(&self, address: String) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let node_info = NodeInfo {
            node_id: self.node_id.clone(),
            address,
            started_at: chrono::Utc::now(),
            last_heartbeat: chrono::Utc::now(),
            active_sessions: 0,
            total_executions: 0,
            status: NodeStatus::Active,
        };

        let json = serde_json::to_string(&node_info)?;

        if let Some(ref client) = self.redis_client {
            client
                .hset("cage:nodes", &self.node_id, &json)
                .await
                .context("Failed to register node in Redis")?;

            info!(node_id = %self.node_id, "Node registered in cluster");
        }

        // Store locally too
        self.local_state.write().await.nodes.insert(self.node_id.clone(), node_info);

        Ok(())
    }

    /// Send heartbeat to indicate node is alive
    pub async fn heartbeat(&self, active_sessions: u64, total_executions: u64) -> Result<()> {
        if !self.config.enabled || self.redis_client.is_none() {
            return Ok(());
        }

        let mut nodes = self.local_state.write().await;
        if let Some(node) = nodes.nodes.get_mut(&self.node_id) {
            node.last_heartbeat = chrono::Utc::now();
            node.active_sessions = active_sessions;
            node.total_executions = total_executions;
            node.status = NodeStatus::Active;

            let json = serde_json::to_string(&node)?;

            if let Some(ref client) = self.redis_client {
                client.hset("cage:nodes", &self.node_id, &json).await?;
            }
        }

        Ok(())
    }

    /// Route a user session to a node (consistent hashing)
    pub async fn route_session(&self, user_id: &str) -> String {
        if !self.config.enabled {
            return self.node_id.clone();
        }

        // Get all active nodes
        let nodes = self.get_active_nodes().await;

        if nodes.is_empty() {
            return self.node_id.clone();
        }

        // Consistent hashing: hash user_id and mod by node count
        let hash = Self::hash_user_id(user_id);
        let node_index = (hash % nodes.len() as u64) as usize;

        nodes[node_index].node_id.clone()
    }

    /// Register a session on this node
    pub async fn register_session(&self, user_id: String, container_id: String) -> Result<()> {
        let route = SessionRoute {
            user_id: user_id.clone(),
            node_id: self.node_id.clone(),
            container_id,
            created_at: chrono::Utc::now(),
        };

        // Store in Redis
        if let Some(ref client) = self.redis_client {
            let json = serde_json::to_string(&route)?;
            client
                .hset("cage:sessions", &user_id, &json)
                .await
                .context("Failed to register session in Redis")?;
        }

        // Store locally
        self.local_state.write().await.sessions.insert(user_id, route);

        Ok(())
    }

    /// Find which node has a user's session
    pub async fn find_session_node(&self, user_id: &str) -> Option<String> {
        // Check local state first
        {
            let state = self.local_state.read().await;
            if let Some(route) = state.sessions.get(user_id) {
                return Some(route.node_id.clone());
            }
        }

        // Check Redis if distributed
        if let Some(ref client) = self.redis_client {
            if let Ok(Some(json)) = client.get(&format!("cage:session:{}", user_id)).await {
                if let Ok(route) = serde_json::from_str::<SessionRoute>(&json) {
                    return Some(route.node_id);
                }
            }
        }

        None
    }

    /// Get all active nodes in cluster
    pub async fn get_active_nodes(&self) -> Vec<NodeInfo> {
        let mut nodes = Vec::new();

        if let Some(ref client) = self.redis_client {
            if let Ok(node_map) = client.hgetall("cage:nodes").await {
                for (_node_id, json) in node_map {
                    if let Ok(node) = serde_json::from_str::<NodeInfo>(&json) {
                        // Check if node is still alive
                        let age = chrono::Utc::now()
                            .signed_duration_since(node.last_heartbeat)
                            .num_seconds() as u64;

                        if age < self.config.node_timeout {
                            nodes.push(node);
                        }
                    }
                }
            }
        }

        // Include this node
        if let Some(node) = self.local_state.read().await.nodes.get(&self.node_id) {
            if !nodes.iter().any(|n| n.node_id == self.node_id) {
                nodes.push(node.clone());
            }
        }

        nodes
    }

    /// Simple hash function for consistent hashing
    fn hash_user_id(user_id: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        user_id.hash(&mut hasher);
        hasher.finish()
    }

    /// Start background heartbeat task
    pub async fn start_heartbeat_task(
        self: Arc<Self>,
        get_stats: impl Fn() -> (u64, u64) + Send + 'static,
    ) {
        if !self.config.enabled {
            return;
        }

        info!("Starting heartbeat task");

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(Duration::from_secs(self.config.heartbeat_interval));

            loop {
                interval.tick().await;

                let (active_sessions, total_executions) = get_stats();

                if let Err(e) = self.heartbeat(active_sessions, total_executions).await {
                    warn!(error = %e, "Heartbeat failed");
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_distributed_manager_creation() {
        let config = DistributedConfig::default();
        let manager = DistributedStateManager::new(config).await.unwrap();
        assert!(!manager.config.enabled);
    }

    #[test]
    fn test_user_routing_deterministic() {
        let hash1 = DistributedStateManager::hash_user_id("user1");
        let hash2 = DistributedStateManager::hash_user_id("user1");
        assert_eq!(hash1, hash2); // Same user always hashes to same value

        let hash3 = DistributedStateManager::hash_user_id("user2");
        assert_ne!(hash1, hash3); // Different users hash differently
    }
}
