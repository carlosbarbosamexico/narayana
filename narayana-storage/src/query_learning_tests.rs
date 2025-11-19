#[cfg(test)]
mod query_learning_tests {
    use crate::query_learning::{QueryLearningEngine, QueryExecution};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_test_execution() -> QueryExecution {
        QueryExecution {
            query_id: "test_1".to_string(),
            query_text: "SELECT * FROM users WHERE age > 25".to_string(),
            normalized_query: "SELECT * FROM users WHERE age > ?".to_string(),
            execution_time_ms: 10.5,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            columns_accessed: vec!["age".to_string(), "name".to_string()],
            tables_accessed: vec!["users".to_string()],
            rows_scanned: 1000,
            rows_returned: 150,
            filters_applied: vec!["age > 25".to_string()],
            indexes_used: vec![],
            join_count: 0,
        }
    }

    #[test]
    fn test_query_learning_enable_disable() {
        let engine = QueryLearningEngine::new();
        assert!(!engine.is_enabled());
        
        engine.enable();
        assert!(engine.is_enabled());
        
        engine.disable();
        assert!(!engine.is_enabled());
    }

    #[test]
    fn test_record_query() {
        let engine = QueryLearningEngine::new();
        engine.enable();
        
        let exec = create_test_execution();
        assert!(engine.record_query(exec).is_ok());
        
        let stats = engine.get_statistics();
        assert_eq!(stats.total_queries_analyzed, 1);
    }

    #[test]
    fn test_pattern_learning() {
        let engine = QueryLearningEngine::new();
        engine.enable();
        
        // Record same query multiple times
        for _ in 0..15 {
            let exec = create_test_execution();
            engine.record_query(exec).unwrap();
        }
        
        let patterns = engine.get_patterns();
        assert!(!patterns.is_empty());
        
        let pattern = &patterns[0];
        assert!(pattern.frequency >= 15);
        assert!(pattern.columns_accessed.contains("age"));
        assert!(pattern.tables_accessed.contains("users"));
    }

    #[test]
    fn test_selectivity_computation() {
        let engine = QueryLearningEngine::new();
        engine.enable();
        
        let mut exec = create_test_execution();
        exec.rows_scanned = 1000;
        exec.rows_returned = 100; // 10% selectivity
        
        engine.record_query(exec).unwrap();
        
        let patterns = engine.get_patterns();
        if let Some(pattern) = patterns.first() {
            if let Some(filter) = pattern.filters.first() {
                // Selectivity should be around 0.1 (100/1000)
                assert!((filter.selectivity - 0.1).abs() < 0.01);
            }
        }
    }

    #[test]
    fn test_get_patterns_for_table() {
        let engine = QueryLearningEngine::new();
        engine.enable();
        
        let exec = create_test_execution();
        engine.record_query(exec).unwrap();
        
        let patterns = engine.get_patterns_for_table("users").unwrap();
        assert!(!patterns.is_empty());
        assert!(patterns[0].tables_accessed.contains("users"));
        
        // Non-existent table should return empty
        let empty = engine.get_patterns_for_table("nonexistent").unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn test_top_patterns() {
        let engine = QueryLearningEngine::new();
        engine.enable();
        
        // Create multiple different queries
        for i in 0..20 {
            let mut exec = create_test_execution();
            exec.query_id = format!("query_{}", i);
            exec.query_text = format!("SELECT * FROM table_{}", i % 3);
            engine.record_query(exec).unwrap();
        }
        
        let top = engine.get_top_patterns(5);
        assert!(top.len() <= 5);
        
        // Should be sorted by frequency
        if top.len() > 1 {
            for i in 1..top.len() {
                assert!(top[i-1].frequency >= top[i].frequency);
            }
        }
    }

    #[test]
    fn test_optimization_suggestions() {
        let engine = QueryLearningEngine::new();
        engine.enable();
        
        // Record query many times to trigger suggestions
        for _ in 0..20 {
            let exec = create_test_execution();
            engine.record_query(exec).unwrap();
        }
        
        let patterns = engine.get_patterns();
        if let Some(pattern) = patterns.first() {
            let suggestions = engine.get_optimization_suggestions(&pattern.pattern_id);
            // Should have suggestions if frequency is high enough
            if pattern.frequency >= 10 {
                assert!(!suggestions.is_empty());
            }
        }
    }
}
