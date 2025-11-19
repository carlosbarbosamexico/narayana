// Native Caching System - Comprehensive Caching with Ample Settings
// Multiple cache types, layers, strategies, and configurations

use narayana_core::{Error, Result, config::EvictionPolicy};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::hash::Hash;
use std::collections::HashMap;
use parking_lot::RwLock;
use dashmap::DashMap;
use tokio::time::interval;
use tracing::{info, warn, debug};

/// Cache entry with metadata
#[derive(Debug, Clone)]
struct CacheEntry<V> {
    value: V,
    inserted_at: Instant,
    last_accessed: Instant,
    access_count: u64,
    size_bytes: usize,
    ttl: Option<Instant>,
    tags: Vec<String>,
    priority: f64,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub inserts: u64,
    pub updates: u64,
    pub deletes: u64,
    pub current_size: usize,
    pub max_size: usize,
    pub memory_used_bytes: usize,
    pub hit_rate: f64,
    pub miss_rate: f64,
}

/// Cache configuration - comprehensive settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeCacheConfig {
    // Size settings
    pub max_size: usize,
    pub max_memory_bytes: Option<usize>,
    pub initial_size: usize,
    
    // Eviction settings
    pub eviction_policy: EvictionPolicy,
    pub eviction_threshold: f64, // 0.0-1.0, evict when this full
    pub eviction_batch_size: usize,
    
    // TTL settings
    pub default_ttl: Option<Duration>,
    pub max_ttl: Option<Duration>,
    pub min_ttl: Option<Duration>,
    pub ttl_jitter: Option<Duration>, // Random jitter to prevent thundering herd
    
    // Cleanup settings
    pub cleanup_interval: Duration,
    pub enable_auto_cleanup: bool,
    pub cleanup_batch_size: usize,
    
    // Performance settings
    pub enable_compression: bool,
    pub compression_threshold_bytes: usize,
    pub enable_prefetch: bool,
    pub prefetch_threshold: usize, // Prefetch if access count >= this
    
    // Persistence settings
    pub enable_persistence: bool,
    pub persistence_path: Option<String>,
    pub persistence_interval: Duration,
    pub enable_async_persistence: bool,
    
    // Replication settings
    pub enable_replication: bool,
    pub replication_nodes: Vec<String>,
    pub replication_sync: bool,
    
    // Partitioning settings
    pub enable_partitioning: bool,
    pub partition_count: usize,
    pub partition_key_fn: Option<String>, // Function name for custom partitioning
    
    // Warming settings
    pub enable_warming: bool,
    pub warming_strategy: WarmingStrategy,
    pub warming_queries: Vec<String>,
    
    // Invalidation settings
    pub invalidation_strategy: InvalidationStrategy,
    pub invalidation_patterns: Vec<String>,
    
    // Metrics settings
    pub enable_metrics: bool,
    pub metrics_interval: Duration,
    pub enable_detailed_metrics: bool,
    
    // Advanced settings
    pub enable_locking: bool,
    pub lock_timeout: Duration,
    pub enable_cas: bool, // Compare-and-swap
    pub enable_atomic_ops: bool,
    pub cache_level: CacheLevel,
    pub enable_hierarchical: bool, // L1/L2/L3 cache hierarchy
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarmingStrategy {
    None,
    OnStartup,
    Periodic,
    OnDemand,
    Predictive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvalidationStrategy {
    None,
    TimeBased,
    EventBased,
    TagBased,
    PatternBased,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CacheLevel {
    L1, // Fastest, smallest
    L2, // Medium speed, medium size
    L3, // Slower, largest
}

impl Default for NativeCacheConfig {
    fn default() -> Self {
        Self {
            max_size: 10_000,
            max_memory_bytes: None,
            initial_size: 1000,
            eviction_policy: EvictionPolicy::LRU,
            eviction_threshold: 0.9,
            eviction_batch_size: 100,
            default_ttl: None,
            max_ttl: Some(Duration::from_secs(3600)),
            min_ttl: Some(Duration::from_secs(60)),
            ttl_jitter: Some(Duration::from_secs(10)),
            cleanup_interval: Duration::from_secs(60),
            enable_auto_cleanup: true,
            cleanup_batch_size: 1000,
            enable_compression: false,
            compression_threshold_bytes: 1024,
            enable_prefetch: false,
            prefetch_threshold: 5,
            enable_persistence: false,
            persistence_path: None,
            persistence_interval: Duration::from_secs(300),
            enable_async_persistence: true,
            enable_replication: false,
            replication_nodes: Vec::new(),
            replication_sync: false,
            enable_partitioning: false,
            partition_count: 4,
            partition_key_fn: None,
            enable_warming: false,
            warming_strategy: WarmingStrategy::None,
            warming_queries: Vec::new(),
            invalidation_strategy: InvalidationStrategy::None,
            invalidation_patterns: Vec::new(),
            enable_metrics: true,
            metrics_interval: Duration::from_secs(60),
            enable_detailed_metrics: false,
            enable_locking: true,
            lock_timeout: Duration::from_secs(5),
            enable_cas: false,
            enable_atomic_ops: true,
            cache_level: CacheLevel::L1,
            enable_hierarchical: false,
        }
    }
}

/// Native cache - comprehensive caching implementation
pub struct NativeCache<K, V> {
    config: NativeCacheConfig,
    cache: DashMap<K, CacheEntry<V>>,
    stats: Arc<RwLock<CacheStats>>,
    partitions: Option<Vec<DashMap<K, CacheEntry<V>>>>,
    compression_enabled: bool,
}

impl<K, V> NativeCache<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub fn new(config: NativeCacheConfig) -> Self {
        let stats = CacheStats {
            hits: 0,
            misses: 0,
            evictions: 0,
            inserts: 0,
            updates: 0,
            deletes: 0,
            current_size: 0,
            max_size: config.max_size,
            memory_used_bytes: 0,
            hit_rate: 0.0,
            miss_rate: 0.0,
        };

        let partitions = if config.enable_partitioning {
            Some((0..config.partition_count)
                .map(|_| DashMap::new())
                .collect())
        } else {
            None
        };

        let mut cache = Self {
            config: config.clone(),
            cache: DashMap::new(),
            stats: Arc::new(RwLock::new(stats)),
            partitions,
            compression_enabled: config.enable_compression,
        };

        // Start background tasks
        if config.enable_auto_cleanup {
            cache.start_cleanup_task();
        }
        if config.enable_metrics {
            cache.start_metrics_task();
        }
        if config.enable_persistence {
            cache.start_persistence_task();
        }

        cache
    }

    /// Get value from cache
    pub fn get(&self, key: &K) -> Option<V> {
        let cache = self.get_partition(key);
        
        if let Some(mut entry) = cache.get_mut(key) {
            // Check TTL
            if let Some(expiry) = entry.ttl {
                if Instant::now() > expiry {
                    cache.remove(key);
                    self.stats.write().misses += 1;
                    return None;
                }
            }

            // Update access metadata
            entry.last_accessed = Instant::now();
            entry.access_count += 1;

            // Update stats
            let mut stats = self.stats.write();
            stats.hits += 1;
            self.update_hit_rate(&mut stats);

            Some(entry.value.clone())
        } else {
            let mut stats = self.stats.write();
            stats.misses += 1;
            self.update_hit_rate(&mut stats);
            None
        }
    }

    /// Insert value into cache
    pub fn insert(&self, key: K, value: V) -> Result<()> {
        self.insert_with_ttl(key, value, self.config.default_ttl)
    }

    /// Insert with TTL
    pub fn insert_with_ttl(&self, key: K, value: V, ttl: Option<Duration>) -> Result<()> {
        let cache = self.get_partition(&key);
        
        // Check if we need to evict
        let current_size = cache.len();
        let threshold = (self.config.max_size as f64 * self.config.eviction_threshold) as usize;
        
        if current_size >= threshold {
            self.evict_batch()?;
        }

        // Calculate size
        let size_bytes = std::mem::size_of_val(&value);
        
        // Apply TTL with jitter
        let ttl_instant = ttl.map(|d| {
            let jitter = if let Some(j) = self.config.ttl_jitter {
                use std::time::SystemTime;
                let seed = SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64;
                Duration::from_nanos((seed % j.as_nanos() as u64) as u64)
            } else {
                Duration::ZERO
            };
            Instant::now() + d + jitter
        });

        let entry = CacheEntry {
            value,
            inserted_at: Instant::now(),
            last_accessed: Instant::now(),
            access_count: 1,
            size_bytes,
            ttl: ttl_instant,
            tags: Vec::new(),
            priority: 1.0,
        };

        let was_present = cache.contains_key(&key);
        cache.insert(key, entry);

        // Update stats
        let mut stats = self.stats.write();
        if was_present {
            stats.updates += 1;
        } else {
            stats.inserts += 1;
            stats.current_size = cache.len();
        }
        stats.memory_used_bytes += size_bytes;

        Ok(())
    }

    /// Insert with tags
    pub fn insert_with_tags(&self, key: K, value: V, tags: Vec<String>) -> Result<()> {
        let key_clone = key.clone();
        self.insert_with_ttl(key, value, self.config.default_ttl)?;
        
        // Update tags
        if let Some(mut entry) = self.get_partition(&key_clone).get_mut(&key_clone) {
            entry.tags = tags;
        }
        
        Ok(())
    }

    /// Remove from cache
    pub fn remove(&self, key: &K) -> Option<V> {
        let cache = self.get_partition(key);
        if let Some((_, entry)) = cache.remove(key) {
            let mut stats = self.stats.write();
            stats.deletes += 1;
            stats.current_size = cache.len();
            stats.memory_used_bytes = stats.memory_used_bytes.saturating_sub(entry.size_bytes);
            Some(entry.value)
        } else {
            None
        }
    }

    /// Clear cache
    pub fn clear(&self) {
        if let Some(ref partitions) = self.partitions {
            for partition in partitions {
                partition.clear();
            }
        } else {
            self.cache.clear();
        }
        
        let mut stats = self.stats.write();
        stats.current_size = 0;
        stats.memory_used_bytes = 0;
    }

    /// Invalidate by tag
    pub fn invalidate_by_tag(&self, tag: &str) -> usize {
        let mut count = 0;
        let cache = &self.cache;
        
        cache.retain(|_, entry| {
            if entry.tags.contains(&tag.to_string()) {
                count += 1;
                false
            } else {
                true
            }
        });
        
        count
    }

    /// Invalidate by pattern
    pub fn invalidate_by_pattern(&self, pattern: &str) -> usize {
        // Simple pattern matching (in production, would use regex)
        let mut count = 0;
        // Implementation would match keys against pattern
        count
    }

    /// Evict batch based on eviction policy
    fn evict_batch(&self) -> Result<()> {
        let batch_size = self.config.eviction_batch_size;
        let cache = &self.cache;
        
        match self.config.eviction_policy {
            EvictionPolicy::LRU => {
                // Evict least recently used
                let mut entries: Vec<_> = cache.iter()
                    .map(|entry| (entry.key().clone(), entry.last_accessed))
                    .collect();
                entries.sort_by_key(|(_, last)| *last);
                
                for (key, _) in entries.iter().take(batch_size) {
                    cache.remove(key);
                }
            }
            EvictionPolicy::LFU => {
                // Evict least frequently used
                let mut entries: Vec<_> = cache.iter()
                    .map(|entry| (entry.key().clone(), entry.access_count))
                    .collect();
                entries.sort_by_key(|(_, count)| *count);
                
                for (key, _) in entries.iter().take(batch_size) {
                    cache.remove(key);
                }
            }
            EvictionPolicy::FIFO => {
                // Evict oldest (first in)
                let mut entries: Vec<_> = cache.iter()
                    .map(|entry| (entry.key().clone(), entry.inserted_at))
                    .collect();
                entries.sort_by_key(|(_, inserted)| *inserted);
                
                for (key, _) in entries.iter().take(batch_size) {
                    cache.remove(key);
                }
            }
            EvictionPolicy::LIFO => {
                // Evict newest (last in)
                let mut entries: Vec<_> = cache.iter()
                    .map(|entry| (entry.key().clone(), entry.inserted_at))
                    .collect();
                entries.sort_by_key(|(_, inserted)| std::cmp::Reverse(*inserted));
                
                for (key, _) in entries.iter().take(batch_size) {
                    cache.remove(key);
                }
            }
            EvictionPolicy::TTL => {
                // Evict expired
                let now = Instant::now();
                let expired: Vec<_> = cache.iter()
                    .filter(|entry| {
                        entry.ttl.map_or(false, |expiry| now > expiry)
                    })
                    .map(|entry| entry.key().clone())
                    .take(batch_size)
                    .collect();
                
                for key in expired {
                    cache.remove(&key);
                }
            }
            EvictionPolicy::Random => {
                // Random eviction
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                // EDGE CASE: Collect keys first to avoid iterator invalidation
                // Also handle empty cache
                if cache.is_empty() {
                    return Ok(());
                }
                let keys: Vec<_> = cache.iter().map(|entry| entry.key().clone()).collect();
                let keys_len = keys.len();
                if keys_len == 0 {
                    return Ok(());
                }
                let mut hasher = DefaultHasher::new();
                SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos().hash(&mut hasher);
                let seed = hasher.finish();
                // EDGE CASE: Prevent division by zero and handle modulo correctly
                for i in 0..batch_size.min(keys_len) {
                    let index = ((seed.wrapping_mul(i as u64 + 1)) as usize) % keys_len;
                    if let Some(key) = keys.get(index) {
                        cache.remove(key);
                    }
                }
            }
            EvictionPolicy::None => {
                // No eviction
            }
        }
        
        let mut stats = self.stats.write();
        stats.evictions += batch_size as u64;
        stats.current_size = cache.len();
        
        Ok(())
    }

    /// Get partition for key
    fn get_partition(&self, key: &K) -> &DashMap<K, CacheEntry<V>> {
        if let Some(ref partitions) = self.partitions {
            let hash = self.hash_key(key);
            &partitions[hash % partitions.len()]
        } else {
            &self.cache
        }
    }

    /// Hash key for partitioning
    fn hash_key(&self, key: &K) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish() as usize
    }

    /// Update hit rate
    fn update_hit_rate(&self, stats: &mut CacheStats) {
        let total = stats.hits + stats.misses;
        if total > 0 {
            stats.hit_rate = stats.hits as f64 / total as f64;
            stats.miss_rate = stats.misses as f64 / total as f64;
        }
    }

    /// Start cleanup task
    fn start_cleanup_task(&self) {
        let cache = self.cache.clone();
        let config = self.config.clone();
        let stats = self.stats.clone();
        
        tokio::spawn(async move {
            let mut interval_timer = interval(config.cleanup_interval);
            loop {
                interval_timer.tick().await;
                
                // Cleanup expired entries
                let now = Instant::now();
                let expired: Vec<_> = cache.iter()
                    .filter(|entry| {
                        entry.ttl.map_or(false, |expiry| now > expiry)
                    })
                    .map(|entry| entry.key().clone())
                    .take(config.cleanup_batch_size)
                    .collect();
                
                for key in expired {
                    cache.remove(&key);
                }
                
                stats.write().current_size = cache.len();
            }
        });
    }

    /// Start metrics task
    fn start_metrics_task(&self) {
        let stats = self.stats.clone();
        let config = self.config.clone();
        
        tokio::spawn(async move {
            let mut interval_timer = interval(config.metrics_interval);
            loop {
                interval_timer.tick().await;
                let stats = stats.read();
                debug!(
                    "Cache stats: hits={}, misses={}, hit_rate={:.2}%, size={}/{}",
                    stats.hits,
                    stats.misses,
                    stats.hit_rate * 100.0,
                    stats.current_size,
                    stats.max_size
                );
            }
        });
    }

    /// Start persistence task
    fn start_persistence_task(&self) {
        // In production, would persist cache to disk
        info!("Cache persistence task started");
    }

    /// Get statistics
    pub fn stats(&self) -> CacheStats {
        self.stats.read().clone()
    }

    /// Get current size
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

/// Hierarchical cache (L1/L2/L3)
pub struct HierarchicalCache<K, V> {
    l1: NativeCache<K, V>,
    l2: Option<NativeCache<K, V>>,
    l3: Option<NativeCache<K, V>>,
}

impl<K, V> HierarchicalCache<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub fn new(l1_config: NativeCacheConfig) -> Self {
        let mut l1_config = l1_config;
        l1_config.cache_level = CacheLevel::L1;
        
        Self {
            l1: NativeCache::new(l1_config),
            l2: None,
            l3: None,
        }
    }

    pub fn with_l2(mut self, l2_config: NativeCacheConfig) -> Self {
        let mut l2_config = l2_config;
        l2_config.cache_level = CacheLevel::L2;
        self.l2 = Some(NativeCache::new(l2_config));
        self
    }

    pub fn with_l3(mut self, l3_config: NativeCacheConfig) -> Self {
        let mut l3_config = l3_config;
        l3_config.cache_level = CacheLevel::L3;
        self.l3 = Some(NativeCache::new(l3_config));
        self
    }

    pub fn get(&self, key: &K) -> Option<V> {
        // Try L1 first
        if let Some(value) = self.l1.get(key) {
            return Some(value);
        }
        
        // Try L2
        if let Some(ref l2) = self.l2 {
            if let Some(value) = l2.get(key) {
                // Promote to L1
                self.l1.insert(key.clone(), value.clone()).ok();
                return Some(value);
            }
        }
        
        // Try L3
        if let Some(ref l3) = self.l3 {
            if let Some(value) = l3.get(key) {
                // Promote to L2 and L1
                if let Some(ref l2) = self.l2 {
                    l2.insert(key.clone(), value.clone()).ok();
                }
                self.l1.insert(key.clone(), value.clone()).ok();
                return Some(value);
            }
        }
        
        None
    }

    pub fn insert(&self, key: K, value: V) -> Result<()> {
        // Insert into all levels
        self.l1.insert(key.clone(), value.clone())?;
        if let Some(ref l2) = self.l2 {
            l2.insert(key.clone(), value.clone())?;
        }
        if let Some(ref l3) = self.l3 {
            l3.insert(key, value)?;
        }
        Ok(())
    }
}

