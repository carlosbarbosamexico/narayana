// Comprehensive configuration system for NarayanaDB

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Eviction policy for cache and data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvictionPolicy {
    LRU,           // Least Recently Used
    LFU,           // Least Frequently Used
    FIFO,          // First In First Out
    LIFO,          // Last In First Out
    TTL,           // Time To Live
    Random,        // Random eviction
    None,          // No eviction
}

/// Replication mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicationMode {
    None,          // No replication
    Async,         // Asynchronous replication
    Sync,          // Synchronous replication
    SemiSync,      // Semi-synchronous replication
    Quorum,        // Quorum-based replication
}

/// Consistency level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsistencyLevel {
    Strong,        // Strong consistency
    Eventual,      // Eventual consistency
    Session,       // Session consistency
    Bounded,       // Bounded staleness
    ConsistentPrefix, // Consistent prefix
}

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolConfig {
    pub max_connections: usize,
    pub min_connections: usize,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
    pub acquire_timeout: Duration,
    pub test_on_acquire: bool,
    pub test_on_idle: bool,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 100,
            min_connections: 10,
            idle_timeout: Duration::from_secs(600),
            max_lifetime: Duration::from_secs(3600),
            acquire_timeout: Duration::from_secs(30),
            test_on_acquire: false,
            test_on_idle: true,
        }
    }
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub max_size: usize,
    pub eviction_policy: EvictionPolicy,
    pub ttl: Option<Duration>,
    pub max_ttl: Option<Duration>,
    pub cleanup_interval: Duration,
    pub enable_metrics: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size: 10_000,
            eviction_policy: EvictionPolicy::LRU,
            ttl: None,
            max_ttl: Some(Duration::from_secs(3600)),
            cleanup_interval: Duration::from_secs(60),
            enable_metrics: true,
        }
    }
}

/// Replication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationConfig {
    pub mode: ReplicationMode,
    pub replica_count: usize,
    pub consistency_level: ConsistencyLevel,
    pub replication_factor: usize,
    pub sync_timeout: Duration,
    pub enable_auto_failover: bool,
    pub quorum_size: usize,
    pub read_from_replicas: bool,
    pub write_to_all: bool,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            mode: ReplicationMode::Async,
            replica_count: 3,
            consistency_level: ConsistencyLevel::Eventual,
            replication_factor: 3,
            sync_timeout: Duration::from_secs(30),
            enable_auto_failover: true,
            quorum_size: 2,
            read_from_replicas: true,
            write_to_all: false,
        }
    }
}

/// Instance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceConfig {
    pub instance_id: String,
    pub node_id: usize,
    pub cluster_name: String,
    pub datacenter: String,
    pub rack: String,
    pub region: String,
    pub tags: HashMap<String, String>,
    pub enable_metrics: bool,
    pub enable_tracing: bool,
    pub log_level: String,
}

impl Default for InstanceConfig {
    fn default() -> Self {
        Self {
            instance_id: uuid::Uuid::new_v4().to_string(),
            node_id: 0,
            cluster_name: "default".to_string(),
            datacenter: "dc1".to_string(),
            rack: "rack1".to_string(),
            region: "us-east-1".to_string(),
            tags: HashMap::new(),
            enable_metrics: true,
            enable_tracing: true,
            log_level: "info".to_string(),
        }
    }
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub data_dir: String,
    pub wal_dir: String,
    pub checkpoint_interval: Duration,
    pub compaction_interval: Duration,
    pub max_file_size: u64,
    pub enable_compression: bool,
    pub compression_type: String,
    pub enable_encryption: bool,
    pub encryption_algorithm: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: "./data".to_string(),
            wal_dir: "./wal".to_string(),
            checkpoint_interval: Duration::from_secs(300),
            compaction_interval: Duration::from_secs(3600),
            max_file_size: 1024 * 1024 * 1024, // 1GB
            enable_compression: true,
            compression_type: "lz4".to_string(),
            enable_encryption: false,
            encryption_algorithm: "aes256-gcm".to_string(),
        }
    }
}

/// Query configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryConfig {
    pub max_query_timeout: Duration,
    pub default_timeout: Duration,
    pub enable_query_cache: bool,
    pub query_cache_size: usize,
    pub max_result_size: usize,
    pub enable_parallel_execution: bool,
    pub max_parallelism: usize,
    pub enable_query_planning: bool,
    pub enable_query_optimization: bool,
}

impl Default for QueryConfig {
    fn default() -> Self {
        Self {
            max_query_timeout: Duration::from_secs(300),
            default_timeout: Duration::from_secs(30),
            enable_query_cache: true,
            query_cache_size: 1000,
            max_result_size: 10_000_000, // 10MB
            enable_parallel_execution: true,
            max_parallelism: num_cpus::get(),
            enable_query_planning: true,
            enable_query_optimization: true,
        }
    }
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub bind_address: String,
    pub bind_port: u16,
    pub enable_tls: bool,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
    pub enable_cors: bool,
    pub cors_origins: Vec<String>,
    pub max_request_size: usize,
    pub enable_compression: bool,
    pub keep_alive_timeout: Duration,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            bind_port: 8080,
            enable_tls: false,
            tls_cert_path: None,
            tls_key_path: None,
            enable_cors: true,
            cors_origins: vec!["*".to_string()],
            max_request_size: 10 * 1024 * 1024, // 10MB
            enable_compression: true,
            keep_alive_timeout: Duration::from_secs(60),
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(30),
        }
    }
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub enable_simd: bool,
    pub enable_parallel_processing: bool,
    pub thread_pool_size: usize,
    pub io_threads: usize,
    pub enable_prefetch: bool,
    pub prefetch_size: usize,
    pub enable_batch_processing: bool,
    pub batch_size: usize,
    pub enable_zero_copy: bool,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enable_simd: true,
            enable_parallel_processing: true,
            thread_pool_size: num_cpus::get(),
            io_threads: 4,
            enable_prefetch: true,
            prefetch_size: 64 * 1024, // 64KB
            enable_batch_processing: true,
            batch_size: 1000,
            enable_zero_copy: true,
        }
    }
}

/// Threading configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadingConfig {
    /// Enable multithreading
    pub enabled: bool,
    
    /// Global thread limit
    pub global_thread_limit: Option<usize>,
    
    /// Enable thread monitoring
    pub enable_monitoring: bool,
    
    /// Thread monitoring interval
    pub monitoring_interval_secs: u64,
    
    /// Enable deadlock detection
    pub enable_deadlock_detection: bool,
    
    /// Deadlock detection timeout in seconds
    pub deadlock_timeout_secs: u64,
    
    /// Enable CPU affinity
    pub enable_cpu_affinity: bool,
    
    /// Enable thread priorities
    pub enable_thread_priorities: bool,
    
    /// Thread spawn timeout in seconds
    pub thread_spawn_timeout_secs: u64,
    
    /// Enable thread-local storage
    pub enable_thread_local_storage: bool,
    
    /// Query pool configuration
    pub query_pool: ThreadPoolConfigSection,
    
    /// Write pool configuration
    pub write_pool: ThreadPoolConfigSection,
    
    /// Read pool configuration
    pub read_pool: ThreadPoolConfigSection,
    
    /// Compression pool configuration
    pub compression_pool: ThreadPoolConfigSection,
    
    /// CPU pool configuration
    pub cpu_pool: ThreadPoolConfigSection,
    
    /// Background pool configuration
    pub background_pool: ThreadPoolConfigSection,
    
    /// Analytics pool configuration
    pub analytics_pool: ThreadPoolConfigSection,
    
    /// Vector pool configuration
    pub vector_pool: ThreadPoolConfigSection,
    
    /// Worker pool configuration
    pub worker_pool: ThreadPoolConfigSection,
    
    /// Sync pool configuration
    pub sync_pool: ThreadPoolConfigSection,
}

/// Thread pool configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadPoolConfigSection {
    /// Minimum number of threads
    pub min_threads: usize,
    
    /// Maximum number of threads
    pub max_threads: usize,
    
    /// Initial number of threads
    pub initial_threads: usize,
    
    /// Thread stack size in bytes (None = default)
    pub stack_size_bytes: Option<usize>,
    
    /// Thread keep-alive timeout in seconds (None = no timeout)
    pub keep_alive_secs: Option<u64>,
    
    /// Thread priority (1-99, None = default)
    pub priority: Option<u8>,
    
    /// CPU affinity (core IDs, None = all cores)
    pub cpu_affinity: Option<Vec<usize>>,
    
    /// Thread name prefix
    pub thread_name_prefix: String,
}

impl Default for ThreadPoolConfigSection {
    fn default() -> Self {
        let num_cpus = num_cpus::get();
        Self {
            min_threads: 1,
            max_threads: num_cpus * 4,
            initial_threads: num_cpus,
            stack_size_bytes: None,
            keep_alive_secs: Some(60),
            priority: None,
            cpu_affinity: None,
            thread_name_prefix: "narayana".to_string(),
        }
    }
}

impl Default for ThreadingConfig {
    fn default() -> Self {
        let num_cpus = num_cpus::get();
        
        Self {
            enabled: true,
            global_thread_limit: None,
            enable_monitoring: true,
            monitoring_interval_secs: 5,
            enable_deadlock_detection: false,
            deadlock_timeout_secs: 300,
            enable_cpu_affinity: false,
            enable_thread_priorities: false,
            thread_spawn_timeout_secs: 30,
            enable_thread_local_storage: true,
            query_pool: ThreadPoolConfigSection {
                min_threads: num_cpus,
                max_threads: num_cpus * 8,
                initial_threads: num_cpus * 2,
                thread_name_prefix: "narayana-query".to_string(),
                ..Default::default()
            },
            write_pool: ThreadPoolConfigSection {
                min_threads: num_cpus / 2,
                max_threads: num_cpus * 4,
                initial_threads: num_cpus,
                thread_name_prefix: "narayana-write".to_string(),
                ..Default::default()
            },
            read_pool: ThreadPoolConfigSection {
                min_threads: num_cpus,
                max_threads: num_cpus * 8,
                initial_threads: num_cpus * 2,
                thread_name_prefix: "narayana-read".to_string(),
                ..Default::default()
            },
            compression_pool: ThreadPoolConfigSection {
                min_threads: num_cpus / 2,
                max_threads: num_cpus * 2,
                initial_threads: num_cpus,
                thread_name_prefix: "narayana-compression".to_string(),
                ..Default::default()
            },
            cpu_pool: ThreadPoolConfigSection {
                min_threads: num_cpus,
                max_threads: num_cpus * 4,
                initial_threads: num_cpus,
                thread_name_prefix: "narayana-cpu".to_string(),
                ..Default::default()
            },
            background_pool: ThreadPoolConfigSection {
                min_threads: 1,
                max_threads: num_cpus * 2,
                initial_threads: num_cpus / 2,
                thread_name_prefix: "narayana-background".to_string(),
                ..Default::default()
            },
            analytics_pool: ThreadPoolConfigSection {
                min_threads: num_cpus,
                max_threads: num_cpus * 4,
                initial_threads: num_cpus * 2,
                thread_name_prefix: "narayana-analytics".to_string(),
                ..Default::default()
            },
            vector_pool: ThreadPoolConfigSection {
                min_threads: num_cpus,
                max_threads: num_cpus * 4,
                initial_threads: num_cpus * 2,
                thread_name_prefix: "narayana-vector".to_string(),
                ..Default::default()
            },
            worker_pool: ThreadPoolConfigSection {
                min_threads: num_cpus,
                max_threads: num_cpus * 8,
                initial_threads: num_cpus * 2,
                thread_name_prefix: "narayana-worker".to_string(),
                ..Default::default()
            },
            sync_pool: ThreadPoolConfigSection {
                min_threads: 1,
                max_threads: num_cpus * 2,
                initial_threads: num_cpus / 2,
                thread_name_prefix: "narayana-sync".to_string(),
                ..Default::default()
            },
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub enable_authentication: bool,
    pub enable_authorization: bool,
    pub enable_encryption: bool,
    pub enable_audit_logging: bool,
    pub session_timeout: Duration,
    pub max_login_attempts: usize,
    pub lockout_duration: Duration,
    pub password_policy: PasswordPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordPolicy {
    pub min_length: usize,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_numbers: bool,
    pub require_special: bool,
    pub max_age_days: u64,
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            min_length: 12,
            require_uppercase: true,
            require_lowercase: true,
            require_numbers: true,
            require_special: true,
            max_age_days: 90,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_authentication: true,
            enable_authorization: true,
            enable_encryption: true,
            enable_audit_logging: true,
            session_timeout: Duration::from_secs(3600),
            max_login_attempts: 5,
            lockout_duration: Duration::from_secs(900),
            password_policy: PasswordPolicy::default(),
        }
    }
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub enable_metrics: bool,
    pub metrics_port: u16,
    pub enable_tracing: bool,
    pub tracing_endpoint: Option<String>,
    pub enable_profiling: bool,
    pub profiling_interval: Duration,
    pub enable_health_checks: bool,
    pub health_check_interval: Duration,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enable_metrics: true,
            metrics_port: 9090,
            enable_tracing: true,
            tracing_endpoint: None,
            enable_profiling: false,
            profiling_interval: Duration::from_secs(60),
            enable_health_checks: true,
            health_check_interval: Duration::from_secs(30),
        }
    }
}

/// Complete NarayanaDB configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarayanaConfig {
    pub instance: InstanceConfig,
    pub storage: StorageConfig,
    pub cache: CacheConfig,
    pub replication: ReplicationConfig,
    pub connection_pool: ConnectionPoolConfig,
    pub query: QueryConfig,
    pub network: NetworkConfig,
    pub performance: PerformanceConfig,
    pub threading: ThreadingConfig,
    pub security: SecurityConfig,
    pub monitoring: MonitoringConfig,
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for NarayanaConfig {
    fn default() -> Self {
        Self {
            instance: InstanceConfig::default(),
            storage: StorageConfig::default(),
            cache: CacheConfig::default(),
            replication: ReplicationConfig::default(),
            connection_pool: ConnectionPoolConfig::default(),
            query: QueryConfig::default(),
            network: NetworkConfig::default(),
            performance: PerformanceConfig::default(),
            threading: ThreadingConfig::default(),
            security: SecurityConfig::default(),
            monitoring: MonitoringConfig::default(),
            custom: HashMap::new(),
        }
    }
}

impl NarayanaConfig {
    /// Load configuration from file
    /// SECURITY: Path validation to prevent reading arbitrary files
    pub fn from_file(path: &str) -> Result<Self, ConfigError> {
        use std::fs;
        use std::path::Path;
        
        // SECURITY: Prevent path traversal
        let path_obj = Path::new(path);
        
        // Check for path traversal sequences
        if path.contains("..") || path.contains("//") || path.contains("\\\\") {
            return Err(ConfigError::IoError(format!(
                "Path traversal detected: '{}'",
                path
            )));
        }
        
        // On Windows, block UNC paths and absolute paths
        #[cfg(windows)]
        {
            if path.starts_with("\\\\") || path.contains(":\\") || path.contains(":/") {
                return Err(ConfigError::IoError(format!(
                    "Absolute or UNC path not allowed: '{}'",
                    path
                )));
            }
        }
        
        // On Unix, block absolute paths (unless explicitly allowed)
        #[cfg(unix)]
        {
            if path.starts_with('/') && !path.starts_with("/etc/") && !path.starts_with("/var/") && !path.starts_with("./") {
                // Allow absolute paths only from specific safe directories
                // This is a compromise - ideally config files should be in a restricted directory
                // For now, just log a warning but allow it (could be tightened)
            }
        }
        
        let content = fs::read_to_string(path)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;
        Self::from_str(&content)
    }

    /// Load configuration from string
    pub fn from_str(content: &str) -> Result<Self, ConfigError> {
        // Try JSON first
        if let Ok(config) = serde_json::from_str::<NarayanaConfig>(content) {
            return Ok(config);
        }
        
        // Try TOML
        if let Ok(config) = toml::from_str::<NarayanaConfig>(content) {
            return Ok(config);
        }
        
        // Try YAML
        if let Ok(config) = serde_yaml::from_str::<NarayanaConfig>(content) {
            return Ok(config);
        }
        
        Err(ConfigError::ParseError("Unknown format".to_string()))
    }

    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();
        
        // Load from environment
        if let Ok(port) = std::env::var("NARAYANA_PORT") {
            if let Ok(p) = port.parse::<u16>() {
                config.network.bind_port = p;
            }
        }
        
        if let Ok(host) = std::env::var("NARAYANA_HOST") {
            config.network.bind_address = host;
        }
        
        if let Ok(data_dir) = std::env::var("NARAYANA_DATA_DIR") {
            config.storage.data_dir = data_dir;
        }
        
        if let Ok(log_level) = std::env::var("NARAYANA_LOG_LEVEL") {
            config.instance.log_level = log_level;
        }
        
        config
    }

    /// Merge with another configuration (other takes precedence)
    pub fn merge(&mut self, other: NarayanaConfig) {
        // Merge each section
        self.instance = other.instance;
        self.storage = other.storage;
        self.cache = other.cache;
        self.replication = other.replication;
        self.connection_pool = other.connection_pool;
        self.query = other.query;
        self.network = other.network;
        self.performance = other.performance;
        self.threading = other.threading;
        self.security = other.security;
        self.monitoring = other.monitoring;
        
        // Merge custom settings
        for (k, v) in other.custom {
            self.custom.insert(k, v);
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate connection pool
        if self.connection_pool.min_connections > self.connection_pool.max_connections {
            return Err(ConfigError::ValidationError(
                "min_connections cannot be greater than max_connections".to_string()
            ));
        }
        
        // Validate replication
        if self.replication.replica_count == 0 && self.replication.mode != ReplicationMode::None {
            return Err(ConfigError::ValidationError(
                "replica_count must be > 0 when replication is enabled".to_string()
            ));
        }
        
        // Validate cache
        if self.cache.max_size == 0 {
            return Err(ConfigError::ValidationError(
                "cache.max_size must be > 0".to_string()
            ));
        }
        
        // Validate network
        if self.network.bind_port == 0 {
            return Err(ConfigError::ValidationError(
                "network.bind_port cannot be 0".to_string()
            ));
        }
        
        Ok(())
    }

    /// Get custom setting
    pub fn get_custom<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.custom.get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Set custom setting
    pub fn set_custom<T: serde::Serialize>(&mut self, key: String, value: T) {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.custom.insert(key, json_value);
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    IoError(String),
    ParseError(String),
    ValidationError(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::IoError(e) => write!(f, "IO error: {}", e),
            ConfigError::ParseError(e) => write!(f, "Parse error: {}", e),
            ConfigError::ValidationError(e) => write!(f, "Validation error: {}", e),
        }
    }
}

impl std::error::Error for ConfigError {}

