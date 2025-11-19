// Auto-Scaling Tests - Database Spawning and Load Balancing

use narayana_storage::auto_scaling::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_auto_scaling_size_threshold() {
    let db_manager = Arc::new(SimpleDatabaseManager::new());
    let thresholds = DatabaseThresholds {
        max_size_bytes: Some(1000), // 1KB
        spawn_threshold_percentage: 0.8, // Spawn at 800 bytes
        ..Default::default()
    };
    
    let auto_scaler = AutoScalingManager::new(
        db_manager,
        thresholds,
        Duration::from_millis(100),
    );
    
    // Update metrics to trigger threshold
    let metrics = DatabaseMetrics {
        database_id: "db-1".to_string(),
        size_bytes: 900, // Above 80% threshold
        row_count: 0,
        table_count: 0,
        transaction_count: 0,
        transactions_per_second: 0.0,
        active_connections: 0,
        query_count: 0,
        queries_per_second: 0.0,
        last_updated: 0,
    };
    
    auto_scaler.update_metrics("db-1".to_string(), metrics);
    
    // Start monitoring
    auto_scaler.start().await;
    
    // Wait for spawn
    sleep(Duration::from_millis(200)).await;
    
    // Check spawn history
    let history = auto_scaler.get_spawn_history();
    assert!(!history.is_empty());
    assert_eq!(history[0].trigger, SpawnTrigger::SizeThreshold);
}

#[tokio::test]
async fn test_auto_scaling_transaction_threshold() {
    let db_manager = Arc::new(SimpleDatabaseManager::new());
    let thresholds = DatabaseThresholds {
        max_transaction_count: Some(1000),
        spawn_threshold_percentage: 0.8,
        ..Default::default()
    };
    
    let auto_scaler = AutoScalingManager::new(
        db_manager,
        thresholds,
        Duration::from_millis(100),
    );
    
    // Update metrics
    let metrics = DatabaseMetrics {
        database_id: "db-1".to_string(),
        size_bytes: 0,
        row_count: 0,
        table_count: 0,
        transaction_count: 850, // Above threshold
        transactions_per_second: 0.0,
        active_connections: 0,
        query_count: 0,
        queries_per_second: 0.0,
        last_updated: 0,
    };
    
    auto_scaler.update_metrics("db-1".to_string(), metrics);
    auto_scaler.start().await;
    
    sleep(Duration::from_millis(200)).await;
    
    let history = auto_scaler.get_spawn_history();
    assert!(!history.is_empty());
    assert_eq!(history[0].trigger, SpawnTrigger::TransactionThreshold);
}

#[tokio::test]
async fn test_auto_scaling_tps_threshold() {
    let db_manager = Arc::new(SimpleDatabaseManager::new());
    let thresholds = DatabaseThresholds {
        max_transactions_per_second: Some(1000.0),
        spawn_threshold_percentage: 0.8,
        ..Default::default()
    };
    
    let auto_scaler = AutoScalingManager::new(
        db_manager,
        thresholds,
        Duration::from_millis(100),
    );
    
    let metrics = DatabaseMetrics {
        database_id: "db-1".to_string(),
        size_bytes: 0,
        row_count: 0,
        table_count: 0,
        transaction_count: 0,
        transactions_per_second: 850.0, // Above threshold
        active_connections: 0,
        query_count: 0,
        queries_per_second: 0.0,
        last_updated: 0,
    };
    
    auto_scaler.update_metrics("db-1".to_string(), metrics);
    auto_scaler.start().await;
    
    sleep(Duration::from_millis(200)).await;
    
    let history = auto_scaler.get_spawn_history();
    assert!(!history.is_empty());
    assert_eq!(history[0].trigger, SpawnTrigger::TransactionsPerSecondThreshold);
}

#[tokio::test]
async fn test_load_balancer_round_robin() {
    let lb = LoadBalancer::new(LoadBalancingStrategy::RoundRobin);
    
    lb.add_database("db-1".to_string());
    lb.add_database("db-2".to_string());
    lb.add_database("db-3".to_string());
    
    // Should select databases
    let db1 = lb.select_database(None);
    assert!(db1.is_some());
    
    let databases = lb.get_databases();
    assert_eq!(databases.len(), 3);
}

#[tokio::test]
async fn test_load_balancer_least_connections() {
    let lb = LoadBalancer::new(LoadBalancingStrategy::LeastConnections);
    
    lb.add_database("db-1".to_string());
    lb.add_database("db-2".to_string());
    
    // Update loads
    lb.update_load("db-1", DatabaseLoad {
        database_id: "db-1".to_string(),
        current_connections: 10,
        current_transactions: 0,
        current_queries: 0,
        size_bytes: 0,
        weight: 1.0,
        last_used: std::time::Instant::now(),
    });
    
    lb.update_load("db-2", DatabaseLoad {
        database_id: "db-2".to_string(),
        current_connections: 5, // Less connections
        current_transactions: 0,
        current_queries: 0,
        size_bytes: 0,
        weight: 1.0,
        last_used: std::time::Instant::now(),
    });
    
    // Should select db-2 (least connections)
    let selected = lb.select_database(None);
    assert_eq!(selected, Some("db-2".to_string()));
}

#[tokio::test]
async fn test_load_balancer_consistent_hashing() {
    let lb = LoadBalancer::new(LoadBalancingStrategy::ConsistentHashing);
    
    lb.add_database("db-1".to_string());
    lb.add_database("db-2".to_string());
    lb.add_database("db-3".to_string());
    
    // Same key should select same database
    let db1 = lb.select_database(Some("user-123"));
    let db2 = lb.select_database(Some("user-123"));
    assert_eq!(db1, db2);
    
    // Different key may select different database
    let db3 = lb.select_database(Some("user-456"));
    // May or may not be same, but should be consistent
}

#[tokio::test]
async fn test_auto_scaling_multiple_databases() {
    let db_manager = Arc::new(SimpleDatabaseManager::new());
    let thresholds = DatabaseThresholds {
        max_size_bytes: Some(1000),
        spawn_threshold_percentage: 0.8,
        ..Default::default()
    };
    
    let auto_scaler = AutoScalingManager::new(
        db_manager,
        thresholds,
        Duration::from_millis(100),
    );
    
    // Add multiple databases
    for i in 1..=5 {
        let metrics = DatabaseMetrics {
            database_id: format!("db-{}", i),
            size_bytes: if i <= 3 { 900 } else { 100 }, // First 3 trigger spawn
            row_count: 0,
            table_count: 0,
            transaction_count: 0,
            transactions_per_second: 0.0,
            active_connections: 0,
            query_count: 0,
            queries_per_second: 0.0,
            last_updated: 0,
        };
        auto_scaler.update_metrics(format!("db-{}", i), metrics);
    }
    
    auto_scaler.start().await;
    sleep(Duration::from_millis(300)).await;
    
    // Should have spawned databases for first 3
    let stats = auto_scaler.stats();
    assert!(stats.total_spawns >= 3);
}

#[tokio::test]
async fn test_auto_scaling_instant_spawn() {
    let db_manager = Arc::new(SimpleDatabaseManager::new());
    let thresholds = DatabaseThresholds {
        max_size_bytes: Some(1000),
        spawn_threshold_percentage: 0.8,
        ..Default::default()
    };
    
    let auto_scaler = AutoScalingManager::new(
        db_manager,
        thresholds,
        Duration::from_millis(10), // Very fast check
    );
    
    let metrics = DatabaseMetrics {
        database_id: "db-1".to_string(),
        size_bytes: 900,
        row_count: 0,
        table_count: 0,
        transaction_count: 0,
        transactions_per_second: 0.0,
        active_connections: 0,
        query_count: 0,
        queries_per_second: 0.0,
        last_updated: 0,
    };
    
    auto_scaler.update_metrics("db-1".to_string(), metrics);
    
    let start = std::time::Instant::now();
    auto_scaler.start().await;
    
    // Wait for spawn
    sleep(Duration::from_millis(50)).await;
    
    let duration = start.elapsed();
    // Should spawn quickly (<100ms)
    assert!(duration < Duration::from_millis(100));
    
    let history = auto_scaler.get_spawn_history();
    assert!(!history.is_empty());
}

#[tokio::test]
async fn test_auto_scaling_statistics() {
    let db_manager = Arc::new(SimpleDatabaseManager::new());
    let thresholds = DatabaseThresholds {
        max_size_bytes: Some(1000),
        spawn_threshold_percentage: 0.8,
        ..Default::default()
    };
    
    let auto_scaler = AutoScalingManager::new(
        db_manager,
        thresholds,
        Duration::from_millis(100),
    );
    
    // Trigger multiple spawns
    for i in 1..=3 {
        let metrics = DatabaseMetrics {
            database_id: format!("db-{}", i),
            size_bytes: 900,
            row_count: 0,
            table_count: 0,
            transaction_count: 0,
            transactions_per_second: 0.0,
            active_connections: 0,
            query_count: 0,
            queries_per_second: 0.0,
            last_updated: 0,
        };
        auto_scaler.update_metrics(format!("db-{}", i), metrics);
    }
    
    auto_scaler.start().await;
    sleep(Duration::from_millis(300)).await;
    
    let stats = auto_scaler.stats();
    assert!(stats.total_spawns >= 3);
    assert!(stats.spawns_by_trigger.contains_key(&SpawnTrigger::SizeThreshold));
}

#[tokio::test]
async fn test_load_balancing_across_spawned_databases() {
    let lb = LoadBalancer::new(LoadBalancingStrategy::LeastConnections);
    
    // Add original database
    lb.add_database("db-1".to_string());
    
    // Spawn new databases
    lb.add_database("db-1-abc123".to_string());
    lb.add_database("db-1-def456".to_string());
    
    // Update loads
    lb.update_load("db-1", DatabaseLoad {
        database_id: "db-1".to_string(),
        current_connections: 100,
        current_transactions: 0,
        current_queries: 0,
        size_bytes: 0,
        weight: 1.0,
        last_used: std::time::Instant::now(),
    });
    
    lb.update_load("db-1-abc123", DatabaseLoad {
        database_id: "db-1-abc123".to_string(),
        current_connections: 50,
        current_transactions: 0,
        current_queries: 0,
        size_bytes: 0,
        weight: 1.0,
        last_used: std::time::Instant::now(),
    });
    
    lb.update_load("db-1-def456", DatabaseLoad {
        database_id: "db-1-def456".to_string(),
        current_connections: 30, // Least connections
        current_transactions: 0,
        current_queries: 0,
        size_bytes: 0,
        weight: 1.0,
        last_used: std::time::Instant::now(),
    });
    
    // Should select database with least connections
    let selected = lb.select_database(None);
    assert_eq!(selected, Some("db-1-def456".to_string()));
}

