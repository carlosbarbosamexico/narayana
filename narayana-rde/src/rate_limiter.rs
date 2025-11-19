// Rate limiting for webhook deliveries

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Rate limiter for subscription deliveries
/// Uses a sliding window algorithm to limit delivery rate
pub struct SubscriptionRateLimiter {
    // Map from subscription ID to delivery timestamps
    deliveries: Arc<RwLock<HashMap<String, Vec<Instant>>>>,
}

impl SubscriptionRateLimiter {
    pub fn new() -> Self {
        Self {
            deliveries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if delivery is allowed and record it
    /// Returns the delay needed before delivery (0 if immediate)
    pub async fn check_and_record(
        &self,
        subscription_id: &str,
        max_per_second: Option<f64>,
    ) -> Duration {
        // If no rate limit configured, allow immediately
        let Some(rate_limit) = max_per_second else {
            return Duration::ZERO;
        };

        // Validate rate limit (must be positive and reasonable)
        if rate_limit <= 0.0 || rate_limit > 10000.0 {
            // Invalid rate limit, allow immediately (don't block)
            tracing::warn!("Invalid rate limit {} for subscription {}, allowing delivery", rate_limit, subscription_id);
            return Duration::ZERO;
        }

        let mut deliveries = self.deliveries.write().await;
        let now = Instant::now();
        let window = Duration::from_secs(1);

        // Cleanup old entries periodically to prevent memory growth
        if deliveries.len() > 100_000 {
            deliveries.retain(|_id, times| {
                times.retain(|&time| now.duration_since(time) < window * 2);
                !times.is_empty()
            });
        }

        // Get or create entry for this subscription
        let entry = deliveries.entry(subscription_id.to_string()).or_insert_with(Vec::new);

        // Remove deliveries outside the 1-second window
        entry.retain(|&time| now.duration_since(time) < window);

        // Check if we're at the limit
        let current_count = entry.len() as f64;
        
        if current_count >= rate_limit {
            // Calculate delay needed (spread remaining deliveries over the window)
            // If we have rate_limit deliveries in the last second, wait until the oldest one expires
            if let Some(oldest) = entry.first() {
                let elapsed = now.duration_since(*oldest);
                if elapsed < window {
                    let delay = window - elapsed;
                    // Add a small buffer to ensure we're past the window
                    return delay + Duration::from_millis(10);
                }
            }
            // Fallback: wait a fraction of the window
            return Duration::from_secs_f64(1.0 / rate_limit);
        }

        // Record this delivery
        entry.push(now);
        
        // Sort to keep oldest first (for efficient cleanup)
        entry.sort();

        Duration::ZERO
    }

    /// Clean up old entries (call periodically)
    pub async fn cleanup(&self) {
        let mut deliveries = self.deliveries.write().await;
        let now = Instant::now();
        let window = Duration::from_secs(2);

        deliveries.retain(|_id, times| {
            times.retain(|&time| now.duration_since(time) < window);
            !times.is_empty()
        });
    }
}

impl Default for SubscriptionRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

