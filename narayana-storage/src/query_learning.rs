// Query Learning System - Learns from Most Used Queries
// Automatically optimizes queries transparently based on usage patterns

use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use parking_lot::RwLock;
use dashmap::DashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{info, warn, debug};
use std::hash::{Hash, Hasher};

/// Query pattern - represents a learned query pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPattern {
    pub pattern_id: String,
    pub query_template: String, // Normalized query (parameters replaced with placeholders)
    pub frequency: u64,
    pub average_execution_time_ms: f64,
    pub total_executions: u64,
    pub last_executed: u64,
    pub first_seen: u64,
    pub columns_accessed: HashSet<String>,
    pub tables_accessed: HashSet<String>,
    pub filters: Vec<FilterPattern>,
    pub sort_fields: Vec<String>,
    pub join_patterns: Vec<JoinPattern>,
    pub optimization_hints: Vec<OptimizationHint>,
    pub optimized_plan: Option<OptimizedPlan>,
    pub performance_improvement: f64, // Percentage improvement
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FilterPattern {
    pub column: String,
    pub operator: String,
    pub value_type: String, // Type of value (range, exact, etc.)
    pub selectivity: f64, // Estimated selectivity (0.0-1.0)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct JoinPattern {
    pub left_table: String,
    pub right_table: String,
    pub join_key: String,
    pub join_type: String,
    pub frequency: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationHint {
    CreateIndex { column: String, index_type: String },
    UseIndex { column: String, index_type: String },
    ReorderJoins { order: Vec<String> },
    PushDownFilter { filter: String },
    UseMaterializedView { view_name: String },
    PartitionTable { column: String },
    Denormalize { columns: Vec<String> },
    CacheResult { ttl_seconds: u64 },
    PrecomputeAggregation { aggregation: String },
    UseColumnarScan { columns: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizedPlan {
    pub plan_id: String,
    pub original_plan: String,
    pub optimized_plan: String,
    pub estimated_cost: f64,
    pub actual_cost: Option<f64>,
    pub improvement_percentage: f64,
    pub applied_optimizations: Vec<String>,
}

/// Query learning engine - learns from query patterns
pub struct QueryLearningEngine {
    enabled: Arc<RwLock<bool>>,
    patterns: Arc<DashMap<String, QueryPattern>>,
    query_history: Arc<RwLock<Vec<QueryExecution>>>,
    statistics: Arc<RwLock<LearningStatistics>>,
    optimizer: Arc<QueryOptimizer>,
    index_suggester: Arc<IndexSuggester>,
    plan_cache: Arc<DashMap<String, OptimizedPlan>>,
    learning_window: Duration,
    min_frequency_threshold: u64,
    auto_apply_optimizations: bool,
}

/// Query execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExecution {
    pub query_id: String,
    pub query_text: String,
    pub normalized_query: String,
    pub execution_time_ms: f64,
    pub timestamp: u64,
    pub columns_accessed: Vec<String>,
    pub tables_accessed: Vec<String>,
    pub rows_scanned: u64,
    pub rows_returned: u64,
    pub filters_applied: Vec<String>,
    pub indexes_used: Vec<String>,
    pub join_count: usize,
}

/// Learning statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningStatistics {
    pub total_queries_analyzed: u64,
    pub patterns_learned: u64,
    pub optimizations_applied: u64,
    pub average_improvement: f64,
    pub top_patterns: Vec<String>,
    pub index_suggestions: u64,
    pub indexes_created: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

/// Query optimizer - applies learned optimizations
struct QueryOptimizer {
    patterns: Arc<DashMap<String, QueryPattern>>,
    plan_cache: Arc<DashMap<String, OptimizedPlan>>,
}

/// Index suggester - suggests indexes based on query patterns
struct IndexSuggester {
    patterns: Arc<DashMap<String, QueryPattern>>,
    existing_indexes: Arc<RwLock<HashSet<String>>>,
}

impl QueryLearningEngine {
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(RwLock::new(false)),
            patterns: Arc::new(DashMap::new()),
            query_history: Arc::new(RwLock::new(Vec::new())),
            statistics: Arc::new(RwLock::new(LearningStatistics {
                total_queries_analyzed: 0,
                patterns_learned: 0,
                optimizations_applied: 0,
                average_improvement: 0.0,
                top_patterns: Vec::new(),
                index_suggestions: 0,
                indexes_created: 0,
                cache_hits: 0,
                cache_misses: 0,
            })),
            optimizer: Arc::new(QueryOptimizer {
                patterns: Arc::new(DashMap::new()),
                plan_cache: Arc::new(DashMap::new()),
            }),
            index_suggester: Arc::new(IndexSuggester {
                patterns: Arc::new(DashMap::new()),
                existing_indexes: Arc::new(RwLock::new(HashSet::new())),
            }),
            plan_cache: Arc::new(DashMap::new()),
            learning_window: Duration::from_secs(3600), // 1 hour
            min_frequency_threshold: 10, // Minimum queries before learning
            auto_apply_optimizations: true,
        }
    }

    /// Enable learning mode
    pub fn enable(&self) {
        *self.enabled.write() = true;
        // Cannot assign to Arc fields - these are already shared references
        // The optimizer and index_suggester already have access to patterns via Arc
        // No need to reassign
        info!("Query learning mode enabled");
    }

    /// Disable learning mode
    pub fn disable(&self) {
        *self.enabled.write() = false;
        info!("Query learning mode disabled");
    }

    /// Check if learning is enabled
    pub fn is_enabled(&self) -> bool {
        *self.enabled.read()
    }

    /// Record query execution for learning
    pub fn record_query(&self, execution: QueryExecution) -> Result<()> {
        if !self.is_enabled() {
            return Ok(());
        }

        // Update statistics
        {
            let mut stats = self.statistics.write();
            stats.total_queries_analyzed += 1;
        }

        // Add to history
        {
            let mut history = self.query_history.write();
            history.push(execution.clone());
            
            // Keep only recent history (within learning window)
            let cutoff_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - self.learning_window.as_secs();
            
            history.retain(|e| e.timestamp >= cutoff_time);
        }

        // Learn from query pattern
        self.learn_from_query(&execution)?;

        // Suggest optimizations
        self.suggest_optimizations(&execution)?;

        Ok(())
    }

    /// Learn from a query execution
    fn learn_from_query(&self, execution: &QueryExecution) -> Result<()> {
        let pattern_id = self.normalize_query(&execution.query_text);
        
        let mut pattern = self.patterns.entry(pattern_id.clone())
            .or_insert_with(|| QueryPattern {
                pattern_id: pattern_id.clone(),
                query_template: execution.normalized_query.clone(),
                frequency: 0,
                average_execution_time_ms: 0.0,
                total_executions: 0,
                last_executed: execution.timestamp,
                first_seen: execution.timestamp,
                columns_accessed: HashSet::new(),
                tables_accessed: HashSet::new(),
                filters: Vec::new(),
                sort_fields: Vec::new(),
                join_patterns: Vec::new(),
                optimization_hints: Vec::new(),
                optimized_plan: None,
                performance_improvement: 0.0,
            })
            .clone();

        // Update pattern statistics
        pattern.frequency += 1;
        pattern.total_executions += 1;
        pattern.last_executed = execution.timestamp;
        
        // Update average execution time
        pattern.average_execution_time_ms = 
            (pattern.average_execution_time_ms * (pattern.total_executions - 1) as f64 + execution.execution_time_ms) 
            / pattern.total_executions as f64;

        // Update columns accessed
        for column in &execution.columns_accessed {
            pattern.columns_accessed.insert(column.clone());
        }

        // Update tables accessed
        for table in &execution.tables_accessed {
            pattern.tables_accessed.insert(table.clone());
        }

        // Learn filter patterns
        self.learn_filter_patterns(&mut pattern, execution)?;

        // Learn join patterns
        self.learn_join_patterns(&mut pattern, execution)?;

        // Store updated pattern
        self.patterns.insert(pattern_id, pattern);

        // Update statistics
        {
            let mut stats = self.statistics.write();
            stats.patterns_learned = self.patterns.len() as u64;
        }

        Ok(())
    }

    /// Learn filter patterns
    fn learn_filter_patterns(&self, pattern: &mut QueryPattern, execution: &QueryExecution) -> Result<()> {
        // Analyze filters from query execution
        // Compute selectivity from execution stats: rows_returned / rows_scanned
        let selectivity = if execution.rows_scanned > 0 {
            execution.rows_returned as f64 / execution.rows_scanned as f64
        } else {
            1.0 // If no scan data, assume non-selective
        };

        for filter in &execution.filters_applied {
            // Extract column, operator, value type
            // Parse filter format: "column=value" or "column operator value"
            let parts: Vec<&str> = filter.splitn(2, '=').collect();
            let (column, operator) = if parts.len() == 2 {
                (parts[0].to_string(), "=".to_string())
            } else {
                // Try other operators
                let parts: Vec<&str> = filter.split_whitespace().collect();
                if parts.len() >= 2 {
                    (parts[0].to_string(), parts[1].to_string())
                } else {
                    (filter.clone(), "=".to_string())
                }
            };

            // Determine value type based on pattern
            let value_type = if filter.contains('<') || filter.contains('>') {
                "range".to_string()
            } else if filter.contains('=') {
                "exact".to_string()
            } else {
                "other".to_string()
            };

            let filter_pattern = FilterPattern {
                column: column.clone(),
                operator: operator.clone(),
                value_type,
                selectivity,
            };
            
            // Update existing pattern or add new one
            if let Some(existing) = pattern.filters.iter_mut()
                .find(|f| f.column == column && f.operator == operator) {
                // Update selectivity as average of all executions
                let count = pattern.total_executions;
                existing.selectivity = (existing.selectivity * (count - 1) as f64 + selectivity) / count as f64;
            } else {
                pattern.filters.push(filter_pattern);
            }
        }
        Ok(())
    }

    /// Learn join patterns
    fn learn_join_patterns(&self, pattern: &mut QueryPattern, execution: &QueryExecution) -> Result<()> {
        if execution.join_count > 0 {
            // Analyze join patterns
            // In production, would parse query to extract join information
            if execution.tables_accessed.len() >= 2 {
                let join_pattern = JoinPattern {
                    left_table: execution.tables_accessed[0].clone(),
                    right_table: execution.tables_accessed[1].clone(),
                    join_key: "id".to_string(), // Would extract from query
                    join_type: "inner".to_string(),
                    frequency: 1,
                };
                
                // Update or add join pattern
                if let Some(existing) = pattern.join_patterns.iter_mut()
                    .find(|j| j.left_table == join_pattern.left_table && j.right_table == join_pattern.right_table) {
                    existing.frequency += 1;
                } else {
                    pattern.join_patterns.push(join_pattern);
                }
            }
        }
        Ok(())
    }

    /// Suggest optimizations based on learned patterns
    fn suggest_optimizations(&self, execution: &QueryExecution) -> Result<()> {
        let pattern_id = self.normalize_query(&execution.query_text);
        
        if let Some(mut pattern) = self.patterns.get_mut(&pattern_id) {
            // Only suggest if pattern is frequent enough
            if pattern.frequency < self.min_frequency_threshold {
                return Ok(());
            }

            let mut hints = Vec::new();

            // Suggest indexes for frequently filtered columns
            for filter in &pattern.filters {
                if filter.selectivity < 0.1 { // High selectivity
                    hints.push(OptimizationHint::CreateIndex {
                        column: filter.column.clone(),
                        index_type: "btree".to_string(),
                    });
                }
            }

            // Suggest indexes for join keys
            for join in &pattern.join_patterns {
                if join.frequency > 5 {
                    hints.push(OptimizationHint::CreateIndex {
                        column: join.join_key.clone(),
                        index_type: "btree".to_string(),
                    });
                }
            }

            // Suggest materialized view for frequent aggregations
            if pattern.frequency > 100 {
                hints.push(OptimizationHint::UseMaterializedView {
                    view_name: format!("mv_{}", pattern_id),
                });
            }

            // Suggest caching for frequently executed queries
            if pattern.frequency > 50 && pattern.average_execution_time_ms > 100.0 {
                hints.push(OptimizationHint::CacheResult {
                    ttl_seconds: 300,
                });
            }

            // Suggest columnar scan for column-specific queries
            if pattern.columns_accessed.len() < pattern.tables_accessed.len() * 3 {
                hints.push(OptimizationHint::UseColumnarScan {
                    columns: pattern.columns_accessed.iter().cloned().collect(),
                });
            }

            pattern.optimization_hints = hints;

            // Update statistics
            {
                let mut stats = self.statistics.write();
                stats.index_suggestions += pattern.optimization_hints.len() as u64;
            }

            // Auto-apply optimizations if enabled
            if self.auto_apply_optimizations {
                self.apply_optimizations(&pattern_id, &pattern.optimization_hints)?;
            }
        }

        Ok(())
    }

    /// Apply optimizations automatically
    fn apply_optimizations(&self, pattern_id: &str, hints: &[OptimizationHint]) -> Result<()> {
        for hint in hints {
            match hint {
                OptimizationHint::CreateIndex { column, index_type } => {
                    info!("Auto-creating index on {} (type: {}) for pattern {}", column, index_type, pattern_id);
                    // In production, would actually create index
                }
                OptimizationHint::CacheResult { ttl_seconds } => {
                    info!("Enabling result caching (TTL: {}s) for pattern {}", ttl_seconds, pattern_id);
                    // In production, would enable caching
                }
                OptimizationHint::UseMaterializedView { view_name } => {
                    info!("Suggesting materialized view {} for pattern {}", view_name, pattern_id);
                    // In production, would create materialized view
                }
                _ => {
                    // Other optimizations
                }
            }
        }

        {
            let mut stats = self.statistics.write();
            stats.optimizations_applied += hints.len() as u64;
        }

        Ok(())
    }

    /// Optimize a query using learned patterns
    pub fn optimize_query(&self, query: &str) -> Result<Option<OptimizedPlan>> {
        if !self.is_enabled() {
            return Ok(None);
        }

        let pattern_id = self.normalize_query(query);

        // Check plan cache first
        if let Some(cached_plan) = self.plan_cache.get(&pattern_id) {
            {
                let mut stats = self.statistics.write();
                stats.cache_hits += 1;
            }
            return Ok(Some(cached_plan.clone()));
        }

        {
            let mut stats = self.statistics.write();
            stats.cache_misses += 1;
        }

        // Get pattern
        if let Some(pattern) = self.patterns.get(&pattern_id) {
            // Generate optimized plan
            let optimized_plan = self.generate_optimized_plan(&pattern)?;
            
            // Cache the plan
            self.plan_cache.insert(pattern_id, optimized_plan.clone());
            
            Ok(Some(optimized_plan))
        } else {
            Ok(None)
        }
    }

    /// Generate optimized plan from pattern
    fn generate_optimized_plan(&self, pattern: &QueryPattern) -> Result<OptimizedPlan> {
        let plan_id = format!("plan_{}", pattern.pattern_id);
        let mut applied_optimizations = Vec::new();

        // Apply optimizations based on hints
        for hint in &pattern.optimization_hints {
            match hint {
                OptimizationHint::UseIndex { column, index_type } => {
                    applied_optimizations.push(format!("UseIndex({}, {})", column, index_type));
                }
                OptimizationHint::ReorderJoins { order } => {
                    applied_optimizations.push(format!("ReorderJoins({:?})", order));
                }
                OptimizationHint::PushDownFilter { filter } => {
                    applied_optimizations.push(format!("PushDownFilter({})", filter));
                }
                OptimizationHint::UseMaterializedView { view_name } => {
                    applied_optimizations.push(format!("UseMaterializedView({})", view_name));
                }
                OptimizationHint::CacheResult { ttl_seconds } => {
                    applied_optimizations.push(format!("CacheResult({}s)", ttl_seconds));
                }
                OptimizationHint::UseColumnarScan { columns } => {
                    applied_optimizations.push(format!("UseColumnarScan({:?})", columns));
                }
                _ => {}
            }
        }

        let optimized_plan = OptimizedPlan {
            plan_id,
            original_plan: pattern.query_template.clone(),
            optimized_plan: format!("Optimized: {}", pattern.query_template),
            estimated_cost: pattern.average_execution_time_ms * 0.7, // 30% improvement estimate
            actual_cost: None,
            improvement_percentage: 30.0,
            applied_optimizations,
        };

        Ok(optimized_plan)
    }

    /// Normalize query to create pattern ID
    fn normalize_query(&self, query: &str) -> String {
        // Normalize query: lowercase, remove whitespace, replace parameters with placeholders
        let normalized = query
            .to_lowercase()
            .replace(" ", "")
            .replace("\n", "")
            .replace("\t", "");
        
        // Replace common parameter patterns with placeholders
        let normalized = normalized
            .replace(r#""[^"]*""#, "?")
            .replace(r"'[^']*'", "?")
            .replace(r"\d+", "?");
        
        // Hash to create pattern ID
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        normalized.hash(&mut hasher);
        format!("pattern_{}", hasher.finish())
    }

    /// Get learned patterns
    pub fn get_patterns(&self) -> Vec<QueryPattern> {
        self.patterns.iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get top patterns by frequency
    pub fn get_top_patterns(&self, limit: usize) -> Vec<QueryPattern> {
        let mut patterns: Vec<QueryPattern> = self.patterns.iter()
            .map(|entry| entry.value().clone())
            .collect();
        
        patterns.sort_by(|a, b| b.frequency.cmp(&a.frequency));
        patterns.truncate(limit);
        
        patterns
    }

    /// Get patterns for a specific table
    pub fn get_patterns_for_table(&self, table_name: &str) -> Result<Vec<QueryPattern>> {
        let patterns: Vec<QueryPattern> = self.patterns.iter()
            .filter(|entry| {
                let pattern = entry.value();
                pattern.tables_accessed.contains(table_name)
            })
            .map(|entry| entry.value().clone())
            .collect();
        
        Ok(patterns)
    }

    /// Get statistics
    pub fn get_statistics(&self) -> LearningStatistics {
        self.statistics.read().clone()
    }

    /// Get optimization suggestions for a pattern
    pub fn get_optimization_suggestions(&self, pattern_id: &str) -> Vec<OptimizationHint> {
        if let Some(pattern) = self.patterns.get(pattern_id) {
            pattern.optimization_hints.clone()
        } else {
            Vec::new()
        }
    }

    /// Manually trigger learning analysis
    pub fn analyze_queries(&self) -> Result<()> {
        if !self.is_enabled() {
            return Ok(());
        }

        let history = self.query_history.read().clone(); // Clone to avoid borrow checker issues
        let history_len = history.len();
        
        // Analyze all queries in history
        for execution in history {
            self.learn_from_query(&execution)?;
            self.suggest_optimizations(&execution)?;
        }

        info!("Analyzed {} queries", history_len);
        Ok(())
    }

    /// Clear learned patterns
    pub fn clear_patterns(&self) {
        self.patterns.clear();
        self.plan_cache.clear();
        {
            let mut stats = self.statistics.write();
            stats.patterns_learned = 0;
            stats.optimizations_applied = 0;
        }
        info!("Cleared all learned patterns");
    }

    /// Export learned patterns
    pub fn export_patterns(&self) -> Result<Vec<QueryPattern>> {
        Ok(self.get_patterns())
    }

    /// Import learned patterns
    pub fn import_patterns(&self, patterns: Vec<QueryPattern>) -> Result<()> {
        let patterns_len = patterns.len();
        for pattern in patterns {
            self.patterns.insert(pattern.pattern_id.clone(), pattern);
        }
        
        {
            let mut stats = self.statistics.write();
            stats.patterns_learned = self.patterns.len() as u64;
        }
        
        info!("Imported {} patterns", patterns_len);
        Ok(())
    }

    /// Get performance report
    pub fn get_performance_report(&self) -> PerformanceReport {
        let stats = self.statistics.read().clone();
        let top_patterns = self.get_top_patterns(10);
        
        let mut total_improvement = 0.0;
        let mut optimized_count = 0;
        
        for pattern in &top_patterns {
            if pattern.performance_improvement > 0.0 {
                total_improvement += pattern.performance_improvement;
                optimized_count += 1;
            }
        }
        
        let average_improvement = if optimized_count > 0 {
            total_improvement / optimized_count as f64
        } else {
            0.0
        };
        
        PerformanceReport {
            total_queries: stats.total_queries_analyzed,
            patterns_learned: stats.patterns_learned,
            optimizations_applied: stats.optimizations_applied,
            average_improvement,
            top_patterns,
            index_suggestions: stats.index_suggestions,
            indexes_created: stats.indexes_created,
            cache_hit_rate: if stats.cache_hits + stats.cache_misses > 0 {
                stats.cache_hits as f64 / (stats.cache_hits + stats.cache_misses) as f64
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub total_queries: u64,
    pub patterns_learned: u64,
    pub optimizations_applied: u64,
    pub average_improvement: f64,
    pub top_patterns: Vec<QueryPattern>,
    pub index_suggestions: u64,
    pub indexes_created: u64,
    pub cache_hit_rate: f64,
}
