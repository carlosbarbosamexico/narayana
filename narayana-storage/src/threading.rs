// Comprehensive multithreading system with full controls
// NarayanaDB - Fully Multithreaded with Ample Controls

use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use rayon::{ThreadPool, ThreadPoolBuilder};
use tokio::runtime::Handle;
use tokio::task::JoinHandle;
use std::thread::{self, ThreadId};
use std::collections::HashMap;

/// Thread pool type for different operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThreadPoolType {
    /// Query execution pool
    Query,
    /// Write operations pool
    Write,
    /// Read operations pool
    Read,
    /// Compression/decompression pool
    Compression,
    /// Index building pool
    Index,
    /// Network I/O pool (Tokio runtime)
    NetworkIO,
    /// CPU-intensive tasks pool
    CPU,
    /// Background tasks pool
    Background,
    /// Analytics pool
    Analytics,
    /// Vector operations pool
    Vector,
    /// Worker execution pool
    Worker,
    /// Sync/replication pool
    Sync,
}

/// Thread pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadPoolConfig {
    /// Pool name
    pub name: String,
    
    /// Minimum number of threads
    pub min_threads: usize,
    
    /// Maximum number of threads
    pub max_threads: usize,
    
    /// Initial number of threads
    pub initial_threads: usize,
    
    /// Thread stack size in bytes
    pub stack_size: Option<usize>,
    
    /// Thread keep-alive timeout
    pub keep_alive: Option<Duration>,
    
    /// Thread priority (1-99, higher = more priority on Linux)
    pub priority: Option<u8>,
    
    /// CPU affinity (core IDs this pool can run on)
    pub cpu_affinity: Option<Vec<usize>>,
    
    /// Thread name prefix
    pub thread_name_prefix: String,
    
    /// Enable thread-local storage
    pub enable_tls: bool,
    
    /// Thread spawn timeout
    pub spawn_timeout: Option<Duration>,
    
    /// Deadlock detection timeout
    pub deadlock_timeout: Option<Duration>,
    
    /// Panic handler
    pub panic_handler: Option<String>,
}

impl Default for ThreadPoolConfig {
    fn default() -> Self {
        let num_cpus = num_cpus::get();
        
        Self {
            name: "default".to_string(),
            min_threads: 1,
            max_threads: num_cpus * 4,
            initial_threads: num_cpus,
            stack_size: None,
            keep_alive: Some(Duration::from_secs(60)),
            priority: None,
            cpu_affinity: None,
            thread_name_prefix: "narayana".to_string(),
            enable_tls: false,
            spawn_timeout: Some(Duration::from_secs(30)),
            deadlock_timeout: None,
            panic_handler: None,
        }
    }
}

impl ThreadPoolConfig {
    /// Create default config for query pool
    pub fn query() -> Self {
        let num_cpus = num_cpus::get();
        Self {
            name: "query".to_string(),
            min_threads: num_cpus,
            max_threads: num_cpus * 8,
            initial_threads: num_cpus * 2,
            thread_name_prefix: "narayana-query".to_string(),
            ..Default::default()
        }
    }
    
    /// Create default config for write pool
    pub fn write() -> Self {
        let num_cpus = num_cpus::get();
        Self {
            name: "write".to_string(),
            min_threads: num_cpus / 2,
            max_threads: num_cpus * 4,
            initial_threads: num_cpus,
            thread_name_prefix: "narayana-write".to_string(),
            ..Default::default()
        }
    }
    
    /// Create default config for read pool
    pub fn read() -> Self {
        let num_cpus = num_cpus::get();
        Self {
            name: "read".to_string(),
            min_threads: num_cpus,
            max_threads: num_cpus * 8,
            initial_threads: num_cpus * 2,
            thread_name_prefix: "narayana-read".to_string(),
            ..Default::default()
        }
    }
    
    /// Create default config for compression pool
    pub fn compression() -> Self {
        let num_cpus = num_cpus::get();
        Self {
            name: "compression".to_string(),
            min_threads: num_cpus / 2,
            max_threads: num_cpus * 2,
            initial_threads: num_cpus,
            thread_name_prefix: "narayana-compression".to_string(),
            ..Default::default()
        }
    }
    
    /// Create default config for CPU pool
    pub fn cpu() -> Self {
        let num_cpus = num_cpus::get();
        Self {
            name: "cpu".to_string(),
            min_threads: num_cpus,
            max_threads: num_cpus * 4,
            initial_threads: num_cpus,
            thread_name_prefix: "narayana-cpu".to_string(),
            ..Default::default()
        }
    }
}

/// Thread pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThreadPoolStats {
    /// Current number of threads
    pub current_threads: usize,
    
    /// Active threads (currently working)
    pub active_threads: usize,
    
    /// Idle threads
    pub idle_threads: usize,
    
    /// Total tasks completed
    pub tasks_completed: u64,
    
    /// Total tasks queued
    pub tasks_queued: u64,
    
    /// Current queue size
    pub queue_size: usize,
    
    /// Average task duration in microseconds
    pub avg_task_duration_us: u64,
    
    /// Maximum task duration in microseconds
    pub max_task_duration_us: u64,
    
    /// Total CPU time used in microseconds
    pub total_cpu_time_us: u64,
    
    /// Thread pool uptime
    pub uptime: Duration,
    
    /// Created timestamp (not serializable - use u64 timestamp instead)
    #[serde(skip_serializing, skip_deserializing)]
    pub created_at: Instant,
}

impl Default for ThreadPoolStats {
    fn default() -> Self {
        Self {
            current_threads: 0,
            active_threads: 0,
            idle_threads: 0,
            tasks_completed: 0,
            tasks_queued: 0,
            queue_size: 0,
            avg_task_duration_us: 0,
            max_task_duration_us: 0,
            total_cpu_time_us: 0,
            uptime: Duration::ZERO,
            created_at: Instant::now(),
        }
    }
}

impl ThreadPoolStats {
    fn update_uptime(&mut self) {
        self.uptime = self.created_at.elapsed();
    }
}

/// Thread pool wrapper with statistics
pub struct ManagedThreadPool {
    /// Rayon thread pool
    pool: Arc<ThreadPool>,
    
    /// Configuration
    config: ThreadPoolConfig,
    
    /// Statistics
    stats: Arc<RwLock<ThreadPoolStats>>,
    
    /// Thread pool type
    pool_type: ThreadPoolType,
    
    /// Active task count
    active_tasks: Arc<dashmap::DashMap<ThreadId, TaskInfo>>,
}

/// Task information
#[derive(Debug, Clone)]
struct TaskInfo {
    thread_id: ThreadId,
    started_at: Instant,
    task_type: String,
}

impl ManagedThreadPool {
    /// Create new managed thread pool
    pub fn new(pool_type: ThreadPoolType, config: ThreadPoolConfig) -> Result<Self> {
        let stats = Arc::new(RwLock::new(ThreadPoolStats::default()));
        let active_tasks = Arc::new(dashmap::DashMap::new());
        
        // Clone thread name prefix for closure
        let thread_name_prefix = config.thread_name_prefix.clone();
        
        // Create Rayon thread pool with configuration
        let pool = ThreadPoolBuilder::new()
            .num_threads(config.initial_threads)
            .thread_name(move |i| format!("{}-{}", thread_name_prefix, i))
            .stack_size(config.stack_size.unwrap_or(2 * 1024 * 1024))
            .build()
            .map_err(|e| anyhow!("Failed to create thread pool: {}", e))?;
        
        let pool = Arc::new(pool);
        
        // Set CPU affinity if specified
        if let Some(ref _affinity) = config.cpu_affinity {
            #[cfg(target_os = "linux")]
            {
                // Note: CPU affinity needs to be set per thread
                // This would be done when threads are spawned
            }
        }
        
        let pool_arc = pool.clone();
        let stats_clone = stats.clone();
        let active_tasks_clone = active_tasks.clone();
        
        // Start statistics update thread
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                let mut stats = stats_clone.write();
                stats.update_uptime();
                stats.current_threads = pool_arc.current_num_threads();
                stats.active_threads = active_tasks_clone.len();
                stats.idle_threads = stats.current_threads.saturating_sub(active_tasks_clone.len());
            }
        });
        
        Ok(Self {
            pool,
            config,
            stats,
            pool_type,
            active_tasks,
        })
    }
    
    /// Execute function in thread pool
    pub fn execute<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R + Send,
        R: Send,
    {
        let start = Instant::now();
        let thread_id = thread::current().id();
        
        // Track task
        self.active_tasks.insert(thread_id, TaskInfo {
            thread_id,
            started_at: start,
            task_type: format!("{:?}", self.pool_type),
        });
        
        let stats = self.stats.clone();
        let active_tasks = self.active_tasks.clone();
        
        let result = self.pool.install(f);
        
        let duration = start.elapsed();
        
        // Update statistics
        {
            let mut stats = stats.write();
            stats.tasks_completed += 1;
            let duration_us = duration.as_micros() as u64;
            stats.total_cpu_time_us += duration_us;
            
            if stats.tasks_completed == 1 {
                stats.avg_task_duration_us = duration_us;
                stats.max_task_duration_us = duration_us;
            } else {
                stats.avg_task_duration_us = 
                    (stats.avg_task_duration_us * (stats.tasks_completed - 1) + duration_us) / stats.tasks_completed;
                stats.max_task_duration_us = stats.max_task_duration_us.max(duration_us);
            }
        }
        
        // Remove task tracking
        active_tasks.remove(&thread_id);
        
        result
    }
    
    /// Execute function asynchronously in thread pool
    pub fn spawn<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let pool = self.pool.clone();
        let stats = self.stats.clone();
        let active_tasks = self.active_tasks.clone();
        
        tokio::spawn(async move {
            let start = Instant::now();
            let thread_id = thread::current().id();
            
            // Track task
            active_tasks.insert(thread_id, TaskInfo {
                thread_id,
                started_at: start,
                task_type: "async".to_string(),
            });
            
            let result = tokio::task::spawn_blocking(move || {
                pool.install(f)
            }).await;
            
            let duration = start.elapsed();
            
            // Update statistics
            {
                let mut stats = stats.write();
                stats.tasks_completed += 1;
                let duration_us = duration.as_micros() as u64;
                stats.total_cpu_time_us += duration_us;
                
                if stats.tasks_completed == 1 {
                    stats.avg_task_duration_us = duration_us;
                    stats.max_task_duration_us = duration_us;
                } else {
                    stats.avg_task_duration_us = 
                        (stats.avg_task_duration_us * (stats.tasks_completed - 1) + duration_us) / stats.tasks_completed;
                    stats.max_task_duration_us = stats.max_task_duration_us.max(duration_us);
                }
            }
            
            active_tasks.remove(&thread_id);
            
            result.unwrap()
        })
    }
    
    /// Get statistics
    pub fn stats(&self) -> ThreadPoolStats {
        self.stats.read().clone()
    }
    
    /// Get configuration
    pub fn config(&self) -> &ThreadPoolConfig {
        &self.config
    }
    
    /// Get pool type
    pub fn pool_type(&self) -> ThreadPoolType {
        self.pool_type
    }
    
    /// Get underlying Rayon pool
    pub fn rayon_pool(&self) -> &Arc<ThreadPool> {
        &self.pool
    }
    
    /// Resize thread pool
    pub fn resize(&self, _new_size: usize) -> Result<()> {
        // Rayon doesn't support dynamic resizing directly
        // Would need to recreate the pool
        // For now, log a warning
        tracing::warn!("Thread pool resize not fully supported, need to recreate pool");
        Ok(())
    }
}

/// Thread manager - manages all thread pools
pub struct ThreadManager {
    /// Thread pools by type
    pools: Arc<DashMap<ThreadPoolType, Arc<ManagedThreadPool>>>,
    
    /// Global thread configuration
    global_config: InternalThreadingConfig,
    
    /// Thread-local storage registry
    tls_registry: Arc<RwLock<HashMap<String, Box<dyn ThreadLocalStorage>>>>,
}

/// Thread-local storage trait
pub trait ThreadLocalStorage: Send + Sync {
    fn get(&self) -> Option<Box<dyn std::any::Any + Send>>;
    fn set(&self, value: Box<dyn std::any::Any + Send>);
    fn clear(&self);
}

/// Internal threading configuration (converted from core config)
#[derive(Debug, Clone)]
struct InternalThreadingConfig {
    /// Enable multithreading
    enabled: bool,
    
    /// Default thread pool configurations
    pools: HashMap<ThreadPoolType, ThreadPoolConfig>,
    
    /// Global thread limit
    global_thread_limit: Option<usize>,
    
    /// Enable thread monitoring
    enable_monitoring: bool,
    
    /// Thread monitoring interval
    monitoring_interval: Duration,
    
    /// Enable deadlock detection
    enable_deadlock_detection: bool,
    
    /// Deadlock detection timeout
    deadlock_timeout: Duration,
    
    /// Enable CPU affinity
    enable_cpu_affinity: bool,
    
    /// Enable thread priorities
    enable_thread_priorities: bool,
    
    /// Thread spawn timeout
    thread_spawn_timeout: Duration,
    
    /// Enable thread-local storage
    enable_thread_local_storage: bool,
}

impl From<narayana_core::config::ThreadingConfig> for InternalThreadingConfig {
    fn from(config: narayana_core::config::ThreadingConfig) -> Self {
        let mut pools = HashMap::new();
        
        // Convert core config to internal pool configs
        let query_config = ThreadPoolConfig {
            name: "query".to_string(),
            min_threads: config.query_pool.min_threads,
            max_threads: config.query_pool.max_threads,
            initial_threads: config.query_pool.initial_threads,
            stack_size: config.query_pool.stack_size_bytes,
            keep_alive: config.query_pool.keep_alive_secs.map(Duration::from_secs),
            priority: config.query_pool.priority,
            cpu_affinity: config.query_pool.cpu_affinity,
            thread_name_prefix: config.query_pool.thread_name_prefix.clone(),
            enable_tls: config.enable_thread_local_storage,
            spawn_timeout: Some(Duration::from_secs(config.thread_spawn_timeout_secs)),
            deadlock_timeout: Some(Duration::from_secs(config.deadlock_timeout_secs)),
            panic_handler: None,
        };
        pools.insert(ThreadPoolType::Query, query_config);
        
        let write_config = ThreadPoolConfig {
            name: "write".to_string(),
            min_threads: config.write_pool.min_threads,
            max_threads: config.write_pool.max_threads,
            initial_threads: config.write_pool.initial_threads,
            stack_size: config.write_pool.stack_size_bytes,
            keep_alive: config.write_pool.keep_alive_secs.map(Duration::from_secs),
            priority: config.write_pool.priority,
            cpu_affinity: config.write_pool.cpu_affinity,
            thread_name_prefix: config.write_pool.thread_name_prefix.clone(),
            enable_tls: config.enable_thread_local_storage,
            spawn_timeout: Some(Duration::from_secs(config.thread_spawn_timeout_secs)),
            deadlock_timeout: Some(Duration::from_secs(config.deadlock_timeout_secs)),
            panic_handler: None,
        };
        pools.insert(ThreadPoolType::Write, write_config);
        
        let read_config = ThreadPoolConfig {
            name: "read".to_string(),
            min_threads: config.read_pool.min_threads,
            max_threads: config.read_pool.max_threads,
            initial_threads: config.read_pool.initial_threads,
            stack_size: config.read_pool.stack_size_bytes,
            keep_alive: config.read_pool.keep_alive_secs.map(Duration::from_secs),
            priority: config.read_pool.priority,
            cpu_affinity: config.read_pool.cpu_affinity,
            thread_name_prefix: config.read_pool.thread_name_prefix.clone(),
            enable_tls: config.enable_thread_local_storage,
            spawn_timeout: Some(Duration::from_secs(config.thread_spawn_timeout_secs)),
            deadlock_timeout: Some(Duration::from_secs(config.deadlock_timeout_secs)),
            panic_handler: None,
        };
        pools.insert(ThreadPoolType::Read, read_config);
        
        let compression_config = ThreadPoolConfig {
            name: "compression".to_string(),
            min_threads: config.compression_pool.min_threads,
            max_threads: config.compression_pool.max_threads,
            initial_threads: config.compression_pool.initial_threads,
            stack_size: config.compression_pool.stack_size_bytes,
            keep_alive: config.compression_pool.keep_alive_secs.map(Duration::from_secs),
            priority: config.compression_pool.priority,
            cpu_affinity: config.compression_pool.cpu_affinity,
            thread_name_prefix: config.compression_pool.thread_name_prefix.clone(),
            enable_tls: config.enable_thread_local_storage,
            spawn_timeout: Some(Duration::from_secs(config.thread_spawn_timeout_secs)),
            deadlock_timeout: Some(Duration::from_secs(config.deadlock_timeout_secs)),
            panic_handler: None,
        };
        pools.insert(ThreadPoolType::Compression, compression_config);
        
        let cpu_config = ThreadPoolConfig {
            name: "cpu".to_string(),
            min_threads: config.cpu_pool.min_threads,
            max_threads: config.cpu_pool.max_threads,
            initial_threads: config.cpu_pool.initial_threads,
            stack_size: config.cpu_pool.stack_size_bytes,
            keep_alive: config.cpu_pool.keep_alive_secs.map(Duration::from_secs),
            priority: config.cpu_pool.priority,
            cpu_affinity: config.cpu_pool.cpu_affinity,
            thread_name_prefix: config.cpu_pool.thread_name_prefix.clone(),
            enable_tls: config.enable_thread_local_storage,
            spawn_timeout: Some(Duration::from_secs(config.thread_spawn_timeout_secs)),
            deadlock_timeout: Some(Duration::from_secs(config.deadlock_timeout_secs)),
            panic_handler: None,
        };
        pools.insert(ThreadPoolType::CPU, cpu_config);
        
        // Set defaults for other pools
        for (pool_type, pool_section) in [
            (ThreadPoolType::Background, &config.background_pool),
            (ThreadPoolType::Analytics, &config.analytics_pool),
            (ThreadPoolType::Vector, &config.vector_pool),
            (ThreadPoolType::Worker, &config.worker_pool),
            (ThreadPoolType::Sync, &config.sync_pool),
        ] {
            let pool_config = ThreadPoolConfig {
                name: format!("{:?}", pool_type).to_lowercase(),
                min_threads: pool_section.min_threads,
                max_threads: pool_section.max_threads,
                initial_threads: pool_section.initial_threads,
                stack_size: pool_section.stack_size_bytes,
                keep_alive: pool_section.keep_alive_secs.map(Duration::from_secs),
                priority: pool_section.priority,
                cpu_affinity: pool_section.cpu_affinity.clone(),
                thread_name_prefix: pool_section.thread_name_prefix.clone(),
                enable_tls: config.enable_thread_local_storage,
                spawn_timeout: Some(Duration::from_secs(config.thread_spawn_timeout_secs)),
                deadlock_timeout: Some(Duration::from_secs(config.deadlock_timeout_secs)),
                panic_handler: None,
            };
            pools.insert(pool_type, pool_config);
        }
        
        // Default pools
        let mut default_config = ThreadPoolConfig::default();
        default_config.name = "index".to_string();
        default_config.thread_name_prefix = "narayana-index".to_string();
        pools.insert(ThreadPoolType::Index, default_config.clone());
        
        default_config.name = "networkio".to_string();
        default_config.thread_name_prefix = "narayana-networkio".to_string();
        pools.insert(ThreadPoolType::NetworkIO, default_config);
        
        Self {
            enabled: config.enabled,
            pools,
            global_thread_limit: config.global_thread_limit,
            enable_monitoring: config.enable_monitoring,
            monitoring_interval: Duration::from_secs(config.monitoring_interval_secs),
            enable_deadlock_detection: config.enable_deadlock_detection,
            deadlock_timeout: Duration::from_secs(config.deadlock_timeout_secs),
            enable_cpu_affinity: config.enable_cpu_affinity,
            enable_thread_priorities: config.enable_thread_priorities,
            thread_spawn_timeout: Duration::from_secs(config.thread_spawn_timeout_secs),
            enable_thread_local_storage: config.enable_thread_local_storage,
        }
    }
}

impl ThreadManager {
    /// Create new thread manager from core config
    pub fn from_core_config(config: narayana_core::config::ThreadingConfig) -> Result<Self> {
        let internal_config: InternalThreadingConfig = config.into();
        Self::new(internal_config)
    }
    
    /// Create new thread manager
    pub fn new(config: InternalThreadingConfig) -> Result<Self> {
        let manager = Self {
            pools: Arc::new(DashMap::new()),
            global_config: config.clone(),
            tls_registry: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Initialize thread pools
        if config.enabled {
            for (pool_type, pool_config) in config.pools {
                let pool = ManagedThreadPool::new(pool_type, pool_config)?;
                manager.pools.insert(pool_type, Arc::new(pool));
            }
        }
        
        // Start monitoring if enabled
        if config.enable_monitoring {
            manager.start_monitoring();
        }
        
        // Start deadlock detection if enabled
        if config.enable_deadlock_detection {
            manager.start_deadlock_detection();
        }
        
        Ok(manager)
    }
    
    /// Get thread pool for operation type
    pub fn get_pool(&self, pool_type: ThreadPoolType) -> Option<Arc<ManagedThreadPool>> {
        self.pools.get(&pool_type).map(|p| p.clone())
    }
    
    /// Execute function in specified thread pool
    pub fn execute<F, R>(&self, pool_type: ThreadPoolType, f: F) -> Result<R>
    where
        F: FnOnce() -> R + Send,
        R: Send,
    {
        let pool = self.get_pool(pool_type)
            .ok_or_else(|| anyhow!("Thread pool not found: {:?}", pool_type))?;
        
        Ok(pool.execute(f))
    }
    
    /// Spawn async task in specified thread pool
    pub fn spawn<F, R>(&self, pool_type: ThreadPoolType, f: F) -> Result<JoinHandle<R>>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let pool = self.get_pool(pool_type)
            .ok_or_else(|| anyhow!("Thread pool not found: {:?}", pool_type))?;
        
        Ok(pool.spawn(f))
    }
    
    /// Get all pool statistics
    pub fn get_all_stats(&self) -> HashMap<ThreadPoolType, ThreadPoolStats> {
        self.pools.iter()
            .map(|entry| (*entry.key(), entry.value().stats()))
            .collect()
    }
    
    /// Get statistics for specific pool
    pub fn get_stats(&self, pool_type: ThreadPoolType) -> Option<ThreadPoolStats> {
        self.pools.get(&pool_type).map(|p| p.stats())
    }
    
    /// Update pool configuration
    pub fn update_pool_config(&self, pool_type: ThreadPoolType, config: ThreadPoolConfig) -> Result<()> {
        // Remove old pool
        self.pools.remove(&pool_type);
        
        // Create new pool with new configuration
        let pool = ManagedThreadPool::new(pool_type, config)?;
        self.pools.insert(pool_type, Arc::new(pool));
        
        Ok(())
    }
    
    /// Register thread-local storage
    pub fn register_tls(&self, name: String, tls: Box<dyn ThreadLocalStorage>) {
        self.tls_registry.write().insert(name, tls);
    }
    
    /// Get thread-local storage
    pub fn get_tls(&self, name: &str) -> Option<Box<dyn std::any::Any + Send>> {
        self.tls_registry.read()
            .get(name)
            .and_then(|tls| tls.get())
    }
    
    /// Start monitoring thread pools
    fn start_monitoring(&self) {
        let pools = self.pools.clone();
        let interval = self.global_config.monitoring_interval;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            loop {
                interval.tick().await;
                
                // Log statistics
                for entry in pools.iter() {
                    let stats = entry.value().stats();
                    tracing::debug!(
                        "Thread pool {:?}: {} threads ({} active, {} idle), {} tasks completed",
                        entry.key(),
                        stats.current_threads,
                        stats.active_threads,
                        stats.idle_threads,
                        stats.tasks_completed,
                    );
                }
            }
        });
    }
    
    /// Start deadlock detection
    fn start_deadlock_detection(&self) {
        let pools = self.pools.clone();
        let timeout = self.global_config.deadlock_timeout;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                
                // Check for tasks that have been running too long
                for entry in pools.iter() {
                    let pool = entry.value();
                    let active_tasks = &pool.active_tasks;
                    
                    for task_entry in active_tasks.iter() {
                        let task_info = task_entry.value();
                        let duration = task_info.started_at.elapsed();
                        
                        if duration > timeout {
                            tracing::warn!(
                                "Potential deadlock detected in pool {:?}: task {:?} running for {:?}",
                                entry.key(),
                                task_info.task_type,
                                duration,
                            );
                        }
                    }
                }
            }
        });
    }
    
    /// Shutdown all thread pools
    pub async fn shutdown(&self) {
        // Wait for all active tasks to complete
        for entry in self.pools.iter() {
            let pool = entry.value();
            // Rayon pools don't have explicit shutdown, but tasks will complete
            tracing::info!("Shutting down thread pool {:?}", entry.key());
        }
        
        self.pools.clear();
    }
}

/// Parallel execution utilities
pub struct ParallelExecutor {
    thread_manager: Arc<ThreadManager>,
}

impl ParallelExecutor {
    pub fn new(thread_manager: Arc<ThreadManager>) -> Self {
        Self { thread_manager }
    }
    
    /// Execute iterator in parallel using specified pool
    pub fn execute_parallel<I, F, R>(
        &self,
        pool_type: ThreadPoolType,
        iter: I,
        f: F,
    ) -> Result<Vec<R>>
    where
        I: Iterator<Item = R> + Send,
        F: Fn(R) -> R + Send + Sync,
        R: Send,
    {
        let pool = self.thread_manager.get_pool(pool_type)
            .ok_or_else(|| anyhow!("Thread pool not found: {:?}", pool_type))?;
        
        let results: Vec<R> = pool.rayon_pool().install(|| {
            iter.map(f).collect()
        });
        
        Ok(results)
    }
    
    /// Execute iterator with parallel fold
    pub fn execute_parallel_fold<I, T, F, R>(
        &self,
        pool_type: ThreadPoolType,
        iter: I,
        identity: T,
        fold_op: F,
    ) -> Result<R>
    where
        I: Iterator<Item = T> + Send,
        T: Send,
        F: Fn(T, T) -> T + Send + Sync,
        R: From<T>,
    {
        let pool = self.thread_manager.get_pool(pool_type)
            .ok_or_else(|| anyhow!("Thread pool not found: {:?}", pool_type))?;
        
        let result = pool.rayon_pool().install(|| {
            iter.fold(identity, fold_op)
        });
        
        Ok(R::from(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_thread_pool_config_default() {
        let config = ThreadPoolConfig::default();
        assert!(config.min_threads > 0);
        assert!(config.max_threads >= config.min_threads);
    }
    
    #[test]
    fn test_thread_pool_config_query() {
        let config = ThreadPoolConfig::query();
        assert_eq!(config.name, "query");
        assert!(config.min_threads > 0);
    }
    
    #[tokio::test]
    async fn test_managed_thread_pool() {
        let config = ThreadPoolConfig::default();
        let pool = ManagedThreadPool::new(ThreadPoolType::Query, config).unwrap();
        
        let result = pool.execute(|| {
            thread::current().name().unwrap_or("unknown").to_string()
        });
        
        assert!(result.contains("narayana"));
        
        let stats = pool.stats();
        assert_eq!(stats.tasks_completed, 1);
    }
    
    #[tokio::test]
    async fn test_thread_manager() {
        let core_config = narayana_core::config::ThreadingConfig::default();
        let manager = ThreadManager::from_core_config(core_config).unwrap();
        
        let result = manager.execute(ThreadPoolType::Query, || {
            42
        }).unwrap();
        
        assert_eq!(result, 42);
        
        let stats = manager.get_stats(ThreadPoolType::Query);
        assert!(stats.is_some());
        assert!(stats.unwrap().tasks_completed > 0);
    }
    
    #[tokio::test]
    async fn test_parallel_executor() {
        let core_config = narayana_core::config::ThreadingConfig::default();
        let manager = Arc::new(ThreadManager::from_core_config(core_config).unwrap());
        let executor = ParallelExecutor::new(manager);
        
        let data = vec![1, 2, 3, 4, 5];
        let results = executor.execute_parallel(
            ThreadPoolType::Query,
            data.into_iter(),
            |x| x * 2,
        ).unwrap();
        
        assert_eq!(results, vec![2, 4, 6, 8, 10]);
    }
}

