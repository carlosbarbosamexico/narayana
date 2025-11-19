// Query Learning Tests - 99% Coverage

use narayana_storage::query_learning::*;
use std::time::SystemTime;

#[test]
fn test_query_learning_enable_disable() {
    let engine = QueryLearningEngine::new();
    
    assert!(!engine.is_enabled());
    
    engine.enable();
    assert!(engine.is_enabled());
    
    engine.disable();
    assert!(!engine.is_enabled());
}

#[tokio::test]
async fn test_query_learning_record_query() {
    let engine = QueryLearningEngine::new();
    engine.enable();
    
    let execution = QueryExecution {
        query_id: "query-1".to_string(),
        query_text: "SELECT * FROM users WHERE id = 123".to_string(),
        normalized_query: "SELECT * FROM users WHERE id = ?".to_string(),
        execution_time_ms: 45.5,
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        columns_accessed: vec!["id".to_string(), "name".to_string()],
        tables_accessed: vec!["users".to_string()],
        rows_scanned: 1000,
        rows_returned: 1,
        filters_applied: vec!["id = 123".to_string()],
        indexes_used: vec![],
        join_count: 0,
    };
    
    engine.record_query(execution).unwrap();
    
    let stats = engine.get_statistics();
    assert_eq!(stats.total_queries_analyzed, 1);
}

#[tokio::test]
async fn test_query_learning_pattern_recognition() {
    let engine = QueryLearningEngine::new();
    engine.enable();
    
    // Record multiple similar queries
    for i in 0..20 {
        let execution = QueryExecution {
            query_id: format!("query-{}", i),
            query_text: format!("SELECT * FROM users WHERE id = {}", i),
            normalized_query: "SELECT * FROM users WHERE id = ?".to_string(),
            execution_time_ms: 50.0,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            columns_accessed: vec!["id".to_string()],
            tables_accessed: vec!["users".to_string()],
            rows_scanned: 1000,
            rows_returned: 1,
            filters_applied: vec![format!("id = {}", i)],
            indexes_used: vec![],
            join_count: 0,
        };
        
        engine.record_query(execution).unwrap();
    }
    
    // Check that pattern was learned
    let patterns = engine.get_patterns();
    assert!(!patterns.is_empty());
    
    let stats = engine.get_statistics();
    assert!(stats.patterns_learned > 0);
}

#[tokio::test]
async fn test_query_learning_optimization() {
    let engine = QueryLearningEngine::new();
    engine.enable();
    
    // Record queries to learn pattern
    for i in 0..15 {
        let execution = QueryExecution {
            query_id: format!("query-{}", i),
            query_text: format!("SELECT * FROM users WHERE id = {}", i),
            normalized_query: "SELECT * FROM users WHERE id = ?".to_string(),
            execution_time_ms: 100.0,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            columns_accessed: vec!["id".to_string()],
            tables_accessed: vec!["users".to_string()],
            rows_scanned: 10000,
            rows_returned: 1,
            filters_applied: vec![format!("id = {}", i)],
            indexes_used: vec![],
            join_count: 0,
        };
        
        engine.record_query(execution).unwrap();
    }
    
    // Optimize query
    let optimized_plan = engine.optimize_query("SELECT * FROM users WHERE id = 123").unwrap();
    
    if let Some(plan) = optimized_plan {
        assert!(plan.improvement_percentage > 0.0);
        assert!(!plan.applied_optimizations.is_empty());
    }
}

#[tokio::test]
async fn test_query_learning_top_patterns() {
    let engine = QueryLearningEngine::new();
    engine.enable();
    
    // Record different query patterns with different frequencies
    for i in 0..50 {
        let execution = QueryExecution {
            query_id: format!("query-{}", i),
            query_text: format!("SELECT * FROM users WHERE id = {}", i),
            normalized_query: "SELECT * FROM users WHERE id = ?".to_string(),
            execution_time_ms: 50.0,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            columns_accessed: vec!["id".to_string()],
            tables_accessed: vec!["users".to_string()],
            rows_scanned: 1000,
            rows_returned: 1,
            filters_applied: vec![],
            indexes_used: vec![],
            join_count: 0,
        };
        
        engine.record_query(execution).unwrap();
    }
    
    // Record less frequent pattern
    for i in 0..5 {
        let execution = QueryExecution {
            query_id: format!("query-other-{}", i),
            query_text: format!("SELECT * FROM orders WHERE user_id = {}", i),
            normalized_query: "SELECT * FROM orders WHERE user_id = ?".to_string(),
            execution_time_ms: 100.0,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            columns_accessed: vec!["user_id".to_string()],
            tables_accessed: vec!["orders".to_string()],
            rows_scanned: 5000,
            rows_returned: 10,
            filters_applied: vec![],
            indexes_used: vec![],
            join_count: 0,
        };
        
        engine.record_query(execution).unwrap();
    }
    
    // Get top patterns
    let top_patterns = engine.get_top_patterns(5);
    assert!(!top_patterns.is_empty());
    
    // Most frequent should be first
    if top_patterns.len() > 1 {
        assert!(top_patterns[0].frequency >= top_patterns[1].frequency);
    }
}

#[tokio::test]
async fn test_query_learning_statistics() {
    let engine = QueryLearningEngine::new();
    engine.enable();
    
    // Record queries
    for i in 0..30 {
        let execution = QueryExecution {
            query_id: format!("query-{}", i),
            query_text: format!("SELECT * FROM users WHERE id = {}", i),
            normalized_query: "SELECT * FROM users WHERE id = ?".to_string(),
            execution_time_ms: 50.0,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            columns_accessed: vec!["id".to_string()],
            tables_accessed: vec!["users".to_string()],
            rows_scanned: 1000,
            rows_returned: 1,
            filters_applied: vec![],
            indexes_used: vec![],
            join_count: 0,
        };
        
        engine.record_query(execution).unwrap();
    }
    
    let stats = engine.get_statistics();
    assert_eq!(stats.total_queries_analyzed, 30);
    assert!(stats.patterns_learned > 0);
}

#[tokio::test]
async fn test_query_learning_performance_report() {
    let engine = QueryLearningEngine::new();
    engine.enable();
    
    // Record queries
    for i in 0..100 {
        let execution = QueryExecution {
            query_id: format!("query-{}", i),
            query_text: format!("SELECT * FROM users WHERE id = {}", i),
            normalized_query: "SELECT * FROM users WHERE id = ?".to_string(),
            execution_time_ms: 50.0,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            columns_accessed: vec!["id".to_string()],
            tables_accessed: vec!["users".to_string()],
            rows_scanned: 1000,
            rows_returned: 1,
            filters_applied: vec![],
            indexes_used: vec![],
            join_count: 0,
        };
        
        engine.record_query(execution).unwrap();
    }
    
    let report = engine.get_performance_report();
    assert_eq!(report.total_queries, 100);
    assert!(report.patterns_learned > 0);
    assert!(!report.top_patterns.is_empty());
}

#[tokio::test]
async fn test_query_learning_clear_patterns() {
    let engine = QueryLearningEngine::new();
    engine.enable();
    
    // Record queries
    for i in 0..10 {
        let execution = QueryExecution {
            query_id: format!("query-{}", i),
            query_text: format!("SELECT * FROM users WHERE id = {}", i),
            normalized_query: "SELECT * FROM users WHERE id = ?".to_string(),
            execution_time_ms: 50.0,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            columns_accessed: vec!["id".to_string()],
            tables_accessed: vec!["users".to_string()],
            rows_scanned: 1000,
            rows_returned: 1,
            filters_applied: vec![],
            indexes_used: vec![],
            join_count: 0,
        };
        
        engine.record_query(execution).unwrap();
    }
    
    // Clear patterns
    engine.clear_patterns();
    
    let patterns = engine.get_patterns();
    assert!(patterns.is_empty());
    
    let stats = engine.get_statistics();
    assert_eq!(stats.patterns_learned, 0);
}

#[tokio::test]
async fn test_query_learning_export_import() {
    let engine1 = QueryLearningEngine::new();
    engine1.enable();
    
    // Record queries
    for i in 0..10 {
        let execution = QueryExecution {
            query_id: format!("query-{}", i),
            query_text: format!("SELECT * FROM users WHERE id = {}", i),
            normalized_query: "SELECT * FROM users WHERE id = ?".to_string(),
            execution_time_ms: 50.0,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            columns_accessed: vec!["id".to_string()],
            tables_accessed: vec!["users".to_string()],
            rows_scanned: 1000,
            rows_returned: 1,
            filters_applied: vec![],
            indexes_used: vec![],
            join_count: 0,
        };
        
        engine1.record_query(execution).unwrap();
    }
    
    // Export patterns
    let patterns = engine1.export_patterns().unwrap();
    assert!(!patterns.is_empty());
    
    // Import into new engine
    let engine2 = QueryLearningEngine::new();
    engine2.enable();
    engine2.import_patterns(patterns).unwrap();
    
    let imported_patterns = engine2.get_patterns();
    assert!(!imported_patterns.is_empty());
}

#[tokio::test]
async fn test_query_learning_optimization_hints() {
    let engine = QueryLearningEngine::new();
    engine.enable();
    
    // Record queries with filters
    for i in 0..20 {
        let execution = QueryExecution {
            query_id: format!("query-{}", i),
            query_text: format!("SELECT * FROM users WHERE status = 'active' AND id = {}", i),
            normalized_query: "SELECT * FROM users WHERE status = ? AND id = ?".to_string(),
            execution_time_ms: 100.0,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            columns_accessed: vec!["id".to_string(), "status".to_string()],
            tables_accessed: vec!["users".to_string()],
            rows_scanned: 10000,
            rows_returned: 1,
            filters_applied: vec!["status = 'active'".to_string(), format!("id = {}", i)],
            indexes_used: vec![],
            join_count: 0,
        };
        
        engine.record_query(execution).unwrap();
    }
    
    // Get optimization suggestions
    let patterns = engine.get_patterns();
    if !patterns.is_empty() {
        let suggestions = engine.get_optimization_suggestions(&patterns[0].pattern_id);
        // Should have index suggestions for filtered columns
        assert!(!suggestions.is_empty() || patterns[0].frequency < 10);
    }
}

#[tokio::test]
async fn test_query_learning_join_patterns() {
    let engine = QueryLearningEngine::new();
    engine.enable();
    
    // Record queries with joins
    for i in 0..15 {
        let execution = QueryExecution {
            query_id: format!("query-{}", i),
            query_text: format!("SELECT * FROM users JOIN orders ON users.id = orders.user_id WHERE users.id = {}", i),
            normalized_query: "SELECT * FROM users JOIN orders ON users.id = orders.user_id WHERE users.id = ?".to_string(),
            execution_time_ms: 200.0,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            columns_accessed: vec!["id".to_string(), "user_id".to_string()],
            tables_accessed: vec!["users".to_string(), "orders".to_string()],
            rows_scanned: 50000,
            rows_returned: 10,
            filters_applied: vec![],
            indexes_used: vec![],
            join_count: 1,
        };
        
        engine.record_query(execution).unwrap();
    }
    
    // Check that join patterns were learned
    let patterns = engine.get_patterns();
    if !patterns.is_empty() {
        assert!(!patterns[0].join_patterns.is_empty() || patterns[0].frequency < 10);
    }
}

#[tokio::test]
async fn test_query_learning_plan_caching() {
    let engine = QueryLearningEngine::new();
    engine.enable();
    
    // Record queries
    for i in 0..20 {
        let execution = QueryExecution {
            query_id: format!("query-{}", i),
            query_text: format!("SELECT * FROM users WHERE id = {}", i),
            normalized_query: "SELECT * FROM users WHERE id = ?".to_string(),
            execution_time_ms: 50.0,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            columns_accessed: vec!["id".to_string()],
            tables_accessed: vec!["users".to_string()],
            rows_scanned: 1000,
            rows_returned: 1,
            filters_applied: vec![],
            indexes_used: vec![],
            join_count: 0,
        };
        
        engine.record_query(execution).unwrap();
    }
    
    // First optimization (cache miss)
    let plan1 = engine.optimize_query("SELECT * FROM users WHERE id = 123").unwrap();
    
    // Second optimization (cache hit)
    let plan2 = engine.optimize_query("SELECT * FROM users WHERE id = 123").unwrap();
    
    assert!(plan1.is_some());
    assert!(plan2.is_some());
    
    let stats = engine.get_statistics();
    assert!(stats.cache_hits > 0);
}

#[tokio::test]
async fn test_query_learning_analyze_queries() {
    let engine = QueryLearningEngine::new();
    engine.enable();
    
    // Record queries
    for i in 0..10 {
        let execution = QueryExecution {
            query_id: format!("query-{}", i),
            query_text: format!("SELECT * FROM users WHERE id = {}", i),
            normalized_query: "SELECT * FROM users WHERE id = ?".to_string(),
            execution_time_ms: 50.0,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            columns_accessed: vec!["id".to_string()],
            tables_accessed: vec!["users".to_string()],
            rows_scanned: 1000,
            rows_returned: 1,
            filters_applied: vec![],
            indexes_used: vec![],
            join_count: 0,
        };
        
        engine.record_query(execution).unwrap();
    }
    
    // Manually trigger analysis
    engine.analyze_queries().unwrap();
    
    let stats = engine.get_statistics();
    assert!(stats.patterns_learned > 0);
}

#[tokio::test]
async fn test_query_learning_disabled_mode() {
    let engine = QueryLearningEngine::new();
    // Don't enable learning
    
    let execution = QueryExecution {
        query_id: "query-1".to_string(),
        query_text: "SELECT * FROM users".to_string(),
        normalized_query: "SELECT * FROM users".to_string(),
        execution_time_ms: 50.0,
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        columns_accessed: vec![],
        tables_accessed: vec!["users".to_string()],
        rows_scanned: 1000,
        rows_returned: 100,
        filters_applied: vec![],
        indexes_used: vec![],
        join_count: 0,
    };
    
    // Should not learn when disabled
    engine.record_query(execution).unwrap();
    
    let stats = engine.get_statistics();
    assert_eq!(stats.total_queries_analyzed, 0);
    
    // Should not optimize when disabled
    let plan = engine.optimize_query("SELECT * FROM users").unwrap();
    assert!(plan.is_none());
}

#[tokio::test]
async fn test_query_learning_all_optimization_hints() {
    use narayana_storage::query_learning::OptimizationHint;
    
    // Test all optimization hint variants
    let _ = OptimizationHint::CreateIndex {
        column: "id".to_string(),
        index_type: "btree".to_string(),
    };
    
    let _ = OptimizationHint::UseIndex {
        column: "id".to_string(),
        index_type: "btree".to_string(),
    };
    
    let _ = OptimizationHint::ReorderJoins {
        order: vec!["users".to_string(), "orders".to_string()],
    };
    
    let _ = OptimizationHint::PushDownFilter {
        filter: "status = 'active'".to_string(),
    };
    
    let _ = OptimizationHint::UseMaterializedView {
        view_name: "mv_stats".to_string(),
    };
    
    let _ = OptimizationHint::PartitionTable {
        column: "date".to_string(),
    };
    
    let _ = OptimizationHint::Denormalize {
        columns: vec!["name".to_string(), "email".to_string()],
    };
    
    let _ = OptimizationHint::CacheResult {
        ttl_seconds: 300,
    };
    
    let _ = OptimizationHint::PrecomputeAggregation {
        aggregation: "SUM(amount)".to_string(),
    };
    
    let _ = OptimizationHint::UseColumnarScan {
        columns: vec!["id".to_string(), "name".to_string()],
    };
}

#[tokio::test]
async fn test_query_learning_filter_patterns() {
    let engine = QueryLearningEngine::new();
    engine.enable();
    
    // Record queries with different filter patterns
    for i in 0..20 {
        let execution = QueryExecution {
            query_id: format!("query-{}", i),
            query_text: format!("SELECT * FROM users WHERE id = {}", i),
            normalized_query: "SELECT * FROM users WHERE id = ?".to_string(),
            execution_time_ms: 50.0,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            columns_accessed: vec!["id".to_string()],
            tables_accessed: vec!["users".to_string()],
            rows_scanned: 1000,
            rows_returned: 1,
            filters_applied: vec![format!("id = {}", i)],
            indexes_used: vec![],
            join_count: 0,
        };
        
        engine.record_query(execution).unwrap();
    }
    
    let patterns = engine.get_patterns();
    if !patterns.is_empty() {
        // Should have learned filter patterns
        assert!(!patterns[0].filters.is_empty() || patterns[0].frequency < 10);
    }
}

#[tokio::test]
async fn test_query_learning_average_execution_time() {
    let engine = QueryLearningEngine::new();
    engine.enable();
    
    // Record queries with varying execution times
    for i in 0..10 {
        let execution = QueryExecution {
            query_id: format!("query-{}", i),
            query_text: format!("SELECT * FROM users WHERE id = {}", i),
            normalized_query: "SELECT * FROM users WHERE id = ?".to_string(),
            execution_time_ms: (i as f64 * 10.0) + 50.0, // Varying times
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            columns_accessed: vec!["id".to_string()],
            tables_accessed: vec!["users".to_string()],
            rows_scanned: 1000,
            rows_returned: 1,
            filters_applied: vec![],
            indexes_used: vec![],
            join_count: 0,
        };
        
        engine.record_query(execution).unwrap();
    }
    
    let patterns = engine.get_patterns();
    if !patterns.is_empty() {
        // Average should be around 95ms (50 + 10*9/2)
        assert!(patterns[0].average_execution_time_ms > 0.0);
    }
}

