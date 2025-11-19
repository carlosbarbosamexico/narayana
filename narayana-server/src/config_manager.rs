// Configuration manager for runtime configuration updates

use narayana_core::config::NarayanaConfig;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::Duration;

/// Runtime configuration manager
pub struct ConfigManager {
    config: Arc<RwLock<NarayanaConfig>>,
    watchers: Arc<RwLock<Vec<ConfigWatcherCallback>>>,
}

type ConfigWatcherCallback = Box<dyn Fn(&NarayanaConfig) + Send + Sync>;

impl ConfigManager {
    pub fn new(config: NarayanaConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            watchers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get current configuration
    pub async fn get(&self) -> NarayanaConfig {
        let config = self.config.read().await;
        config.clone()
    }

    /// Update configuration
    pub async fn update(&self, new_config: NarayanaConfig) -> Result<(), String> {
        // Validate new configuration
        new_config.validate()
            .map_err(|e| e.to_string())?;
        
        // Update configuration
        let mut config = self.config.write().await;
        *config = new_config.clone();
        drop(config);
        
        // Notify watchers
        self.notify_watchers(&new_config).await;
        
        Ok(())
    }

    /// Watch for configuration changes
    pub fn watch(&self, watcher: ConfigWatcherCallback) {
        let mut watchers = self.watchers.blocking_write();
        watchers.push(watcher);
    }

    async fn notify_watchers(&self, config: &NarayanaConfig) {
        let watchers = self.watchers.read().await;
        for watcher in watchers.iter() {
            watcher(config);
        }
    }

    /// Reload configuration from file
    pub async fn reload_from_file(&self, path: &str) -> Result<(), String> {
        let new_config = NarayanaConfig::from_file(path)
            .map_err(|e| e.to_string())?;
        self.update(new_config).await
    }

    /// Get configuration section
    pub async fn get_section<T: Clone>(&self, getter: impl Fn(&NarayanaConfig) -> T) -> T {
        let config = self.config.read().await;
        getter(&config)
    }

    /// Update configuration section
    pub async fn update_section<F>(&self, updater: F) -> Result<(), String>
    where
        F: FnOnce(&mut NarayanaConfig) -> Result<(), String>,
    {
        let mut config = self.config.write().await;
        updater(&mut config)?;
        
        // Validate
        config.validate()
            .map_err(|e| e.to_string())?;
        
        let config_clone = config.clone();
        drop(config);
        
        // Notify watchers
        self.notify_watchers(&config_clone).await;
        
        Ok(())
    }
}

/// Configuration hot-reload watcher
pub struct ConfigWatcher {
    path: String,
    check_interval: Duration,
}

impl ConfigWatcher {
    pub fn new(path: String, check_interval: Duration) -> Self {
        Self {
            path,
            check_interval,
        }
    }

    pub async fn start(&self, manager: Arc<ConfigManager>) {
        let path = self.path.clone();
        let interval = self.check_interval;
        
        tokio::spawn(async move {
            let mut last_modified = std::fs::metadata(&path)
                .and_then(|m| m.modified())
                .ok();
            
            let mut interval_timer = tokio::time::interval(interval);
            loop {
                interval_timer.tick().await;
                
                if let Ok(metadata) = std::fs::metadata(&path) {
                    if let Ok(modified) = metadata.modified() {
                        if last_modified.map(|lm| modified > lm).unwrap_or(true) {
                            last_modified = Some(modified);
                            
                            // Reload configuration
                            if let Err(e) = manager.reload_from_file(&path).await {
                                tracing::warn!("Failed to reload config: {}", e);
                            }
                        }
                    }
                }
            }
        });
    }
}

/// Configuration builder for programmatic configuration
pub struct ConfigBuilder {
    config: NarayanaConfig,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: NarayanaConfig::default(),
        }
    }

    pub fn with_instance(mut self, instance: narayana_core::config::InstanceConfig) -> Self {
        self.config.instance = instance;
        self
    }

    pub fn with_storage(mut self, storage: narayana_core::config::StorageConfig) -> Self {
        self.config.storage = storage;
        self
    }

    pub fn with_cache(mut self, cache: narayana_core::config::CacheConfig) -> Self {
        self.config.cache = cache;
        self
    }

    pub fn with_replication(mut self, replication: narayana_core::config::ReplicationConfig) -> Self {
        self.config.replication = replication;
        self
    }

    pub fn with_connection_pool(mut self, pool: narayana_core::config::ConnectionPoolConfig) -> Self {
        self.config.connection_pool = pool;
        self
    }

    pub fn with_query(mut self, query: narayana_core::config::QueryConfig) -> Self {
        self.config.query = query;
        self
    }

    pub fn with_network(mut self, network: narayana_core::config::NetworkConfig) -> Self {
        self.config.network = network;
        self
    }

    pub fn with_performance(mut self, performance: narayana_core::config::PerformanceConfig) -> Self {
        self.config.performance = performance;
        self
    }

    pub fn with_security(mut self, security: narayana_core::config::SecurityConfig) -> Self {
        self.config.security = security;
        self
    }

    pub fn with_monitoring(mut self, monitoring: narayana_core::config::MonitoringConfig) -> Self {
        self.config.monitoring = monitoring;
        self
    }

    pub fn with_custom<T: serde::Serialize>(mut self, key: String, value: T) -> Self {
        self.config.set_custom(key, value);
        self
    }

    pub fn build(self) -> Result<NarayanaConfig, String> {
        self.config.validate()
            .map_err(|e| e.to_string())?;
        Ok(self.config)
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

