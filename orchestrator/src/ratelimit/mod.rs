//! Rate limiting middleware using token bucket algorithm
//!
//! Prevents abuse by limiting requests per user per time window

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

/// Token bucket for rate limiting
#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    capacity: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            tokens: capacity,
            capacity,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    fn try_consume(&mut self, tokens: f64) -> bool {
        self.refill();

        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();

        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;
    }

    fn remaining(&mut self) -> f64 {
        self.refill();
        self.tokens
    }
}

/// Rate limiter with per-user buckets
pub struct RateLimiter {
    buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
    capacity: f64,
    refill_rate: f64,
}

impl RateLimiter {
    /// Create a new rate limiter
    ///
    /// # Arguments
    /// * `requests_per_minute` - Maximum requests allowed per minute per user
    pub fn new(requests_per_minute: f64) -> Self {
        let capacity = requests_per_minute;
        let refill_rate = requests_per_minute / 60.0; // Convert to per-second

        Self {
            buckets: Arc::new(RwLock::new(HashMap::new())),
            capacity,
            refill_rate,
        }
    }

    /// Check if a request is allowed for a user
    pub async fn check_limit(&self, user_id: &str) -> bool {
        self.check_limit_n(user_id, 1.0).await
    }

    /// Check if N tokens are available for a user
    pub async fn check_limit_n(&self, user_id: &str, tokens: f64) -> bool {
        let mut buckets = self.buckets.write().await;

        let bucket = buckets
            .entry(user_id.to_string())
            .or_insert_with(|| TokenBucket::new(self.capacity, self.refill_rate));

        bucket.try_consume(tokens)
    }

    /// Get remaining tokens for a user
    pub async fn remaining(&self, user_id: &str) -> f64 {
        let mut buckets = self.buckets.write().await;

        let bucket = buckets
            .entry(user_id.to_string())
            .or_insert_with(|| TokenBucket::new(self.capacity, self.refill_rate));

        bucket.remaining()
    }

    /// Clean up old buckets periodically
    pub async fn cleanup_old_buckets(&self) {
        let cutoff = Instant::now() - Duration::from_secs(3600);

        let mut buckets = self.buckets.write().await;
        buckets.retain(|_, bucket| bucket.last_refill > cutoff);
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(60.0) // 60 requests per minute default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(10.0); // 10 per minute

        // Should allow first 10 requests
        for _ in 0..10 {
            assert!(limiter.check_limit("user1").await);
        }

        // 11th should be denied
        assert!(!limiter.check_limit("user1").await);

        // Different user should have own bucket
        assert!(limiter.check_limit("user2").await);
    }

    #[tokio::test]
    async fn test_refill() {
        let limiter = RateLimiter::new(60.0); // 60 per minute = 1 per second

        // Consume all tokens
        for _ in 0..60 {
            assert!(limiter.check_limit("user1").await);
        }
        assert!(!limiter.check_limit("user1").await);

        // Wait for refill
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Should allow 2 more requests (2 seconds * 1 per second)
        assert!(limiter.check_limit("user1").await);
        assert!(limiter.check_limit("user1").await);
    }
}
