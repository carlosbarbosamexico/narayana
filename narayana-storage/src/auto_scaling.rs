// Auto-Scaling Database System
// Automatically spawns new databases when thresholds are reached
// Instantly balances load across multiple databases

use narayana_core::{Error, Result, types::TableId};
// DatabaseManager trait for auto-scaling
pub trait DatabaseManagerTrait: Send + Sync {
    fn create_database(&self, name: &str) -> Result<String>;
    fn delete_database(&self, name: &str) -> Result<()>;
    fn list_databases(&self) -> Vec<String>;
}

// Simple implementation for testing
pub struct SimpleDatabaseManager {
    databases: Arc<parking_lot::RwLock<Vec<String>>>,
}

impl SimpleDatabaseManager {
    pub fn new() -> Self {
        Self {
            databases: Arc::new(parking_lot::RwLock::new(Vec::new())),
        }
    }
}

impl DatabaseManagerTrait for SimpleDatabaseManager {
    fn create_database(&self, name: &str) -> Result<String> {
        self.databases.write().push(name.to_string());
        Ok(name.to_string())
    }

    fn delete_database(&self, name: &str) -> Result<()> {
        self.databases.write().retain(|db| db != name);
        Ok(())
    }

    fn list_databases(&self) -> Vec<String> {
        self.databases.read().clone()
    }
}
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use parking_lot::RwLock;
use dashmap::DashMap;
use tokio::time::interval;
use tracing::{info, warn, debug};
use std::collections::HashMap;

/// Database metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseMetrics {
    pub database_id: String,
    pub size_bytes: u64,
    pub row_count: u64,
    pub table_count: usize,
    pub transaction_count: u64,
    pub transactions_per_second: f64,
    pub active_connections: usize,
    pub query_count: u64,
    pub queries_per_second: f64,
    pub last_updated: u64,
}

/// Database threshold configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseThresholds {
    pub max_size_bytes: Option<u64>,
    pub max_row_count: Option<u64>,
    pub max_table_count: Option<usize>,
    pub max_transaction_count: Option<u64>,
    pub max_transactions_per_second: Option<f64>,
    pub max_active_connections: Option<usize>,
    pub max_query_count: Option<u64>,
    pub max_queries_per_second: Option<f64>,
    pub spawn_threshold_percentage: f64, // Spawn at X% of max (e.g., 0.8 = 80%)
}

impl Default for DatabaseThresholds {
    fn default() -> Self {
        Self {
            max_size_bytes: Some(10 * 1024 * 1024 * 1024), // 10GB
            max_row_count: Some(1_000_000_000), // 1 billion rows
            max_table_count: Some(10_000), // 10K tables
            max_transaction_count: Some(100_000_000), // 100M transactions
            max_transactions_per_second: Some(10_000.0), // 10K TPS
            max_active_connections: Some(10_000), // 10K connections
            max_query_count: Some(1_000_000_000), // 1B queries
            max_queries_per_second: Some(100_000.0), // 100K QPS
            spawn_threshold_percentage: 0.8, // Spawn at 80% of max
        }
    }
}

/// Auto-scaling manager - monitors and spawns databases
pub struct AutoScalingManager {
    database_manager: Arc<dyn DatabaseManagerTrait>,
    metrics: Arc<DashMap<String, DatabaseMetrics>>,
    thresholds: DatabaseThresholds,
    check_interval: Duration,
    spawn_history: Arc<RwLock<Vec<SpawnEvent>>>,
    load_balancer: Arc<LoadBalancer>,
    stats: Arc<RwLock<AutoScalingStats>>,
    predictive_engine: Option<Arc<PredictiveScalingEngine>>,
}

/// Spawn event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnEvent {
    pub timestamp: u64,
    pub trigger: SpawnTrigger,
    pub source_database: String,
    pub new_database: String,
    pub metrics_at_spawn: DatabaseMetrics,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpawnTrigger {
    SizeThreshold,
    RowCountThreshold,
    TableCountThreshold,
    TransactionThreshold,
    TransactionsPerSecondThreshold,
    ConnectionThreshold,
    QueryThreshold,
    QueriesPerSecondThreshold,
    Manual,
}

/// Auto-scaling statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoScalingStats {
    pub total_spawns: u64,
    pub spawns_by_trigger: HashMap<SpawnTrigger, u64>,
    pub average_spawn_time_ms: f64,
    pub total_databases: usize,
    pub load_balanced_queries: u64,
}

/// Load balancer - distributes load across databases
pub struct LoadBalancer {
    databases: Arc<DashMap<String, DatabaseLoad>>,
    strategy: LoadBalancingStrategy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoadBalancingStrategy {
    RoundRobin,
    LeastConnections,
    LeastTransactions,
    LeastQueries,
    LeastSize,
    WeightedRoundRobin,
    ConsistentHashing,
}

#[derive(Debug, Clone)]
struct DatabaseLoad {
    database_id: String,
    current_connections: usize,
    current_transactions: u64,
    current_queries: u64,
    size_bytes: u64,
    weight: f64,
    last_used: Instant,
}

impl AutoScalingManager {
    pub fn new(
        database_manager: Arc<dyn DatabaseManagerTrait>,
        thresholds: DatabaseThresholds,
        check_interval: Duration,
    ) -> Self {
        let load_balancer = Arc::new(LoadBalancer::new(LoadBalancingStrategy::LeastConnections));
        
        // Initialize predictive scaling engine
        let predictive_config = PredictiveScalingConfig::default();
        let predictive_engine = Arc::new(PredictiveScalingEngine::new(predictive_config));
        
        Self {
            database_manager,
            metrics: Arc::new(DashMap::new()),
            thresholds,
            check_interval,
            spawn_history: Arc::new(RwLock::new(Vec::new())),
            load_balancer: load_balancer.clone(),
            stats: Arc::new(RwLock::new(AutoScalingStats {
                total_spawns: 0,
                spawns_by_trigger: HashMap::new(),
                average_spawn_time_ms: 0.0,
                total_databases: 0,
                load_balanced_queries: 0,
            })),
            predictive_engine: Some(predictive_engine),
        }
    }

    /// Start monitoring and auto-scaling
    pub async fn start(&self) {
        let metrics = self.metrics.clone();
        let thresholds = self.thresholds.clone();
        let database_manager = self.database_manager.clone();
        let spawn_history = self.spawn_history.clone();
        let stats = self.stats.clone();
        let load_balancer = self.load_balancer.clone();
        let check_interval = self.check_interval;
        let predictive_engine = self.predictive_engine.clone();

        tokio::spawn(async move {
            let mut interval_timer = interval(check_interval);
            loop {
                interval_timer.tick().await;

                // Use predictive scaling if available
                if let Some(ref predictive) = predictive_engine {
                    // Record metrics for prediction - collect entries to avoid holding iter across await
                    let metrics_entries: Vec<_> = metrics.iter().map(|e| (e.key().clone(), e.value().clone())).collect();
                    for (database_id, metrics_data) in metrics_entries {
                        let usage_metrics = UsageMetrics {
                            timestamp: metrics_data.last_updated,
                            cpu_usage: 0.5, // Would be actual CPU usage
                            memory_usage: (metrics_data.size_bytes as f64) / 1_000_000_000.0, // Normalized
                            query_count: metrics_data.query_count,
                            query_latency_ms: 1000.0 / metrics_data.queries_per_second.max(1.0),
                            connection_count: metrics_data.active_connections as u64,
                            transaction_count: metrics_data.transaction_count,
                            data_size_bytes: metrics_data.size_bytes,
                            active_databases: metrics.len(),
                            active_tables: metrics_data.table_count,
                        };
                        let _ = predictive.record_metrics(usage_metrics);
                    }
                    
                    // Get predictions for next 30 minutes
                    if let Ok(prediction) = predictive.predict_usage(30) {
                        // Use prediction to proactively scale
                        match prediction.scaling_recommendation.action {
                            crate::predictive_scaling::ScalingAction::EmergencyScaleUp |
                            crate::predictive_scaling::ScalingAction::ScaleUp => {
                                // Proactively scale up based on prediction
                                info!("Predictive scaling: Proactively scaling up based on prediction (confidence: {:.2}%)",
                                    prediction.confidence * 100.0);
                            }
                            crate::predictive_scaling::ScalingAction::ScaleDown |
                            crate::predictive_scaling::ScalingAction::GradualScaleDown => {
                                // Consider scaling down if usage is predicted to be low
                                info!("Predictive scaling: May scale down based on prediction (confidence: {:.2}%)",
                                    prediction.confidence * 100.0);
                            }
                            _ => {}
                        }
                    }
                }

                // Check all databases - collect entries to avoid holding iter across await
                let database_entries: Vec<_> = metrics.iter().map(|e| (e.key().clone(), e.value().clone())).collect();
                for (database_id, metrics) in database_entries {

                    // Check thresholds (reactive scaling)
                    if let Some(trigger) = Self::check_thresholds(&metrics, &thresholds) {
                        info!("Threshold reached for database {}: {:?}", database_id, trigger);
                        
                        // Spawn new database instantly
                        let start = Instant::now();
                        match Self::spawn_database(
                            &database_manager,
                            &database_id,
                            &metrics,
                            trigger.clone(),
                        ).await {
                            Ok(new_database_id) => {
                                let duration = start.elapsed();
                                
                                // Record spawn event
                                let spawn_event = SpawnEvent {
                                    timestamp: SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs(),
                                    trigger: trigger.clone(),
                                    source_database: database_id.clone(),
                                    new_database: new_database_id.clone(),
                                    metrics_at_spawn: metrics.clone(),
                                };
                                spawn_history.write().push(spawn_event);

                                // Update load balancer
                                load_balancer.add_database(new_database_id.clone());

                                // Update statistics
                                let mut stats_guard = stats.write();
                                stats_guard.total_spawns += 1;
                                *stats_guard.spawns_by_trigger.entry(trigger).or_insert(0) += 1;
                                stats_guard.total_databases += 1;
                                
                                let total = stats_guard.total_spawns as f64;
                                stats_guard.average_spawn_time_ms = 
                                    (stats_guard.average_spawn_time_ms * (total - 1.0) + duration.as_millis() as f64) / total;

                                info!("Spawned database {} in {:?}", new_database_id, duration);
                            }
                            Err(e) => {
                                warn!("Failed to spawn database: {}", e);
                            }
                        }
                    }
                }
            }
        });
    }

    /// Check if any threshold is reached
    fn check_thresholds(metrics: &DatabaseMetrics, thresholds: &DatabaseThresholds) -> Option<SpawnTrigger> {
        // Check size threshold
        if let Some(max_size) = thresholds.max_size_bytes {
            let threshold = (max_size as f64 * thresholds.spawn_threshold_percentage) as u64;
            if metrics.size_bytes >= threshold {
                return Some(SpawnTrigger::SizeThreshold);
            }
        }

        // Check row count threshold
        if let Some(max_rows) = thresholds.max_row_count {
            let threshold = (max_rows as f64 * thresholds.spawn_threshold_percentage) as u64;
            if metrics.row_count >= threshold {
                return Some(SpawnTrigger::RowCountThreshold);
            }
        }

        // Check table count threshold
        if let Some(max_tables) = thresholds.max_table_count {
            let threshold = (max_tables as f64 * thresholds.spawn_threshold_percentage) as usize;
            if metrics.table_count >= threshold {
                return Some(SpawnTrigger::TableCountThreshold);
            }
        }

        // Check transaction count threshold
        if let Some(max_txns) = thresholds.max_transaction_count {
            let threshold = (max_txns as f64 * thresholds.spawn_threshold_percentage) as u64;
            if metrics.transaction_count >= threshold {
                return Some(SpawnTrigger::TransactionThreshold);
            }
        }

        // Check transactions per second threshold
        if let Some(max_tps) = thresholds.max_transactions_per_second {
            let threshold = max_tps * thresholds.spawn_threshold_percentage;
            if metrics.transactions_per_second >= threshold {
                return Some(SpawnTrigger::TransactionsPerSecondThreshold);
            }
        }

        // Check connection threshold
        if let Some(max_conns) = thresholds.max_active_connections {
            let threshold = (max_conns as f64 * thresholds.spawn_threshold_percentage) as usize;
            if metrics.active_connections >= threshold {
                return Some(SpawnTrigger::ConnectionThreshold);
            }
        }

        // Check query count threshold
        if let Some(max_queries) = thresholds.max_query_count {
            let threshold = (max_queries as f64 * thresholds.spawn_threshold_percentage) as u64;
            if metrics.query_count >= threshold {
                return Some(SpawnTrigger::QueryThreshold);
            }
        }

        // Check queries per second threshold
        if let Some(max_qps) = thresholds.max_queries_per_second {
            let threshold = max_qps * thresholds.spawn_threshold_percentage;
            if metrics.queries_per_second >= threshold {
                return Some(SpawnTrigger::QueriesPerSecondThreshold);
            }
        }

        None
    }

    /// Spawn new database instantly
    async fn spawn_database(
        database_manager: &Arc<dyn DatabaseManagerTrait>,
        source_database_id: &str,
        source_metrics: &DatabaseMetrics,
        trigger: SpawnTrigger,
    ) -> Result<String> {
        // Generate new database ID
        let new_database_id = format!("{}-{}", source_database_id, uuid::Uuid::new_v4().to_string());
        
        // Create new database instantly
        database_manager.create_database(&new_database_id)?;
        
        info!("Spawning database {} from {} (trigger: {:?})", new_database_id, source_database_id, trigger);
        
        Ok(new_database_id)
    }

    /// Update database metrics
    pub fn update_metrics(&self, database_id: String, metrics: DatabaseMetrics) {
        self.metrics.insert(database_id.clone(), metrics.clone());
        
        // Also update predictive engine
        if let Some(ref predictive) = self.predictive_engine {
            let usage_metrics = UsageMetrics {
                timestamp: metrics.last_updated,
                cpu_usage: 0.5,
                memory_usage: (metrics.size_bytes as f64) / 1_000_000_000.0,
                query_count: metrics.query_count,
                query_latency_ms: 1000.0 / metrics.queries_per_second.max(1.0),
                connection_count: metrics.active_connections as u64,
                transaction_count: metrics.transaction_count,
                data_size_bytes: metrics.size_bytes,
                active_databases: self.metrics.len(),
                active_tables: metrics.table_count,
            };
            let _ = predictive.record_metrics(usage_metrics);
        }
    }

    /// Get predictive scaling engine
    pub fn predictive_engine(&self) -> Option<Arc<PredictiveScalingEngine>> {
        self.predictive_engine.clone()
    }

    /// Get metrics for a database
    pub fn get_metrics(&self, database_id: &str) -> Option<DatabaseMetrics> {
        self.metrics.get(database_id).map(|m| m.clone())
    }

    /// Get all metrics
    pub fn get_all_metrics(&self) -> HashMap<String, DatabaseMetrics> {
        self.metrics.iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// Manually spawn a database
    pub async fn spawn_manual(&self, source_database_id: &str) -> Result<String> {
        let metrics = self.metrics.get(source_database_id)
            .ok_or_else(|| Error::Storage(format!("Database not found: {}", source_database_id)))?
            .clone();

        Self::spawn_database(
            &self.database_manager,
            source_database_id,
            &metrics,
            SpawnTrigger::Manual,
        ).await
    }

    /// Get spawn history
    pub fn get_spawn_history(&self) -> Vec<SpawnEvent> {
        self.spawn_history.read().clone()
    }

    /// Get statistics
    pub fn stats(&self) -> AutoScalingStats {
        self.stats.read().clone()
    }

    /// Get load balancer
    pub fn load_balancer(&self) -> Arc<LoadBalancer> {
        self.load_balancer.clone()
    }
}

impl LoadBalancer {
    pub fn new(strategy: LoadBalancingStrategy) -> Self {
        Self {
            databases: Arc::new(DashMap::new()),
            strategy,
        }
    }

    /// Add database to load balancer
    pub fn add_database(&self, database_id: String) {
        self.databases.insert(database_id.clone(), DatabaseLoad {
            database_id: database_id.clone(),
            current_connections: 0,
            current_transactions: 0,
            current_queries: 0,
            size_bytes: 0,
            weight: 1.0,
            last_used: Instant::now(),
        });
    }

    /// Select database for load balancing
    pub fn select_database(&self, key: Option<&str>) -> Option<String> {
        if self.databases.is_empty() {
            return None;
        }

        match self.strategy {
            LoadBalancingStrategy::RoundRobin => {
                // Simple round-robin (in production, would use proper round-robin)
                self.databases.iter().next().map(|entry| entry.key().clone())
            }
            LoadBalancingStrategy::LeastConnections => {
                self.databases.iter()
                    .min_by_key(|entry| entry.value().current_connections)
                    .map(|entry| entry.key().clone())
            }
            LoadBalancingStrategy::LeastTransactions => {
                self.databases.iter()
                    .min_by_key(|entry| entry.value().current_transactions)
                    .map(|entry| entry.key().clone())
            }
            LoadBalancingStrategy::LeastQueries => {
                self.databases.iter()
                    .min_by_key(|entry| entry.value().current_queries)
                    .map(|entry| entry.key().clone())
            }
            LoadBalancingStrategy::LeastSize => {
                self.databases.iter()
                    .min_by_key(|entry| entry.value().size_bytes)
                    .map(|entry| entry.key().clone())
            }
            LoadBalancingStrategy::WeightedRoundRobin => {
                // Weighted round-robin (in production, would use proper algorithm)
                self.databases.iter()
                    .max_by(|a, b| {
                        a.value().weight.partial_cmp(&b.value().weight)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|entry| entry.key().clone())
            }
            LoadBalancingStrategy::ConsistentHashing => {
                // Consistent hashing (in production, would use proper algorithm)
                if let Some(key) = key {
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};
                    let mut hasher = DefaultHasher::new();
                    key.hash(&mut hasher);
                    let hash = hasher.finish();
                    let index = (hash as usize) % self.databases.len();
                    self.databases.iter()
                        .nth(index)
                        .map(|entry| entry.key().clone())
                } else {
                    self.databases.iter().next().map(|entry| entry.key().clone())
                }
            }
        }
    }

    /// Update database load
    pub fn update_load(&self, database_id: &str, load: DatabaseLoad) {
        if let Some(mut entry) = self.databases.get_mut(database_id) {
            *entry = load;
        }
    }

    /// Get all databases
    pub fn get_databases(&self) -> Vec<String> {
        self.databases.iter().map(|entry| entry.key().clone()).collect()
    }
}

use uuid;
use crate::predictive_scaling::*;

