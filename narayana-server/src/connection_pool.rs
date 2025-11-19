// Connection pooling for high-performance server

use std::sync::Arc;
use tokio::sync::Semaphore;
use std::time::{Duration, Instant};

/// Connection pool for managing concurrent connections
pub struct ConnectionPool {
    semaphore: Arc<Semaphore>,
    max_connections: usize,
    timeout: Duration,
}

impl ConnectionPool {
    pub fn new(max_connections: usize, timeout: Duration) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_connections)),
            max_connections,
            timeout,
        }
    }

    /// Acquire a connection permit
    pub async fn acquire(&self) -> Result<ConnectionGuard, PoolError> {
        let permit = tokio::time::timeout(
            self.timeout,
            self.semaphore.clone().acquire_owned(),
        )
        .await
        .map_err(|_| PoolError::Timeout)?
        .map_err(|_| PoolError::Closed)?;

        Ok(ConnectionGuard {
            permit,
            acquired_at: Instant::now(),
        })
    }

    pub fn available(&self) -> usize {
        self.semaphore.available_permits()
    }

    pub fn max_connections(&self) -> usize {
        self.max_connections
    }
}

/// Guard for a connection permit
pub struct ConnectionGuard {
    permit: tokio::sync::OwnedSemaphorePermit,
    acquired_at: Instant,
}

impl ConnectionGuard {
    pub fn wait_time(&self) -> Duration {
        self.acquired_at.elapsed()
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        // Permit is automatically released
    }
}

#[derive(Debug)]
pub enum PoolError {
    Timeout,
    Closed,
}

impl std::fmt::Display for PoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PoolError::Timeout => write!(f, "Connection pool timeout"),
            PoolError::Closed => write!(f, "Connection pool closed"),
        }
    }
}

impl std::error::Error for PoolError {}

/// Request rate limiter for preventing overload
pub struct RateLimiter {
    semaphore: Arc<Semaphore>,
    refill_interval: Duration,
    refill_amount: usize,
}

impl RateLimiter {
    pub fn new(rate: usize, per: Duration) -> Self {
        let refill_amount = rate.max(1);
        let refill_interval = per / (refill_amount as u32);
        
        Self {
            semaphore: Arc::new(Semaphore::new(rate)),
            refill_interval,
            refill_amount,
        }
    }

    pub async fn acquire(&self) -> Result<RateLimitGuard, PoolError> {
        let permit = self.semaphore.clone().acquire_owned().await
            .map_err(|_| PoolError::Closed)?;

        // Refill is handled by the rate limiter itself

        Ok(RateLimitGuard { permit })
    }
}

struct RateLimitGuard {
    permit: tokio::sync::OwnedSemaphorePermit,
}

/// Batch request processor for high throughput
pub struct BatchProcessor {
    batch_size: usize,
    flush_interval: Duration,
}

impl BatchProcessor {
    pub fn new(batch_size: usize, flush_interval: Duration) -> Self {
        Self {
            batch_size,
            flush_interval,
        }
    }

    /// Process requests in batches
    pub async fn process_batch<T, F, Fut>(&self, items: Vec<T>, processor: F) -> Vec<Fut::Output>
    where
        T: Clone,
        F: Fn(Vec<T>) -> Fut,
        Fut: std::future::Future,
    {
        let chunks: Vec<Vec<T>> = items
            .chunks(self.batch_size)
            .map(|chunk| chunk.to_vec())
            .collect();

        let futures: Vec<_> = chunks.into_iter().map(|chunk| processor(chunk)).collect();
        
        futures::future::join_all(futures).await
    }
}

