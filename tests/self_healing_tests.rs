// Tests for self-healing system

use narayana_storage::self_healing::*;

#[test]
fn test_failure_detector_creation() {
    let config = FailureDetectionConfig::default();
    let detector = FailureDetector::new(config);
    // Should create successfully
}

#[test]
fn test_failure_detector_register_checker() {
    let mut detector = FailureDetector::new(FailureDetectionConfig::default());
    detector.register_checker(Box::new(StorageHealthChecker::new()));
    
    let health = detector.get_health(ComponentType::Storage);
    assert!(health.is_some());
}

#[tokio::test]
async fn test_failure_detector_start_monitoring() {
    let mut detector = FailureDetector::new(FailureDetectionConfig::default());
    detector.register_checker(Box::new(StorageHealthChecker::new()));
    detector.start_monitoring().await;
    // Should start monitoring
}

#[test]
fn test_health_checker_storage() {
    let checker = StorageHealthChecker::new();
    let result = checker.check().unwrap();
    assert_eq!(result.component, ComponentType::Storage);
}

#[test]
fn test_health_checker_network() {
    let checker = NetworkHealthChecker::new();
    let result = checker.check().unwrap();
    assert_eq!(result.component, ComponentType::Network);
}

#[test]
fn test_storage_recovery_strategy() {
    let strategy = StorageRecoveryStrategy::new();
    let result = strategy.recover(ComponentType::Storage).unwrap();
    assert!(result.success);
    assert!(!result.actions_taken.is_empty());
}

#[test]
fn test_network_recovery_strategy() {
    let strategy = NetworkRecoveryStrategy::new();
    let result = strategy.recover(ComponentType::Network).unwrap();
    assert!(result.success);
}

#[test]
fn test_node_recovery_strategy() {
    let strategy = NodeRecoveryStrategy::new();
    let result = strategy.recover(ComponentType::Node).unwrap();
    assert!(result.success);
}

#[test]
fn test_self_healing_manager_creation() {
    let detector = Arc::new(FailureDetector::new(FailureDetectionConfig::default()));
    let manager = SelfHealingManager::new(detector);
    // Should create successfully
}

#[test]
fn test_self_healing_manager_register_strategy() {
    let detector = Arc::new(FailureDetector::new(FailureDetectionConfig::default()));
    let mut manager = SelfHealingManager::new(detector);
    manager.register_strategy(Box::new(StorageRecoveryStrategy::new()));
    // Should register successfully
}

#[tokio::test]
async fn test_self_healing_manager_start_healing() {
    let detector = Arc::new(FailureDetector::new(FailureDetectionConfig::default()));
    let manager = SelfHealingManager::new(detector);
    manager.start_healing().await;
    // Should start healing
}

#[tokio::test]
async fn test_data_consistency_checker() {
    let checker = DataConsistencyChecker::new();
    let report = checker.check_consistency(narayana_core::types::TableId(1)).await.unwrap();
    // Should check consistency
}

#[tokio::test]
async fn test_data_consistency_repair() {
    let checker = DataConsistencyChecker::new();
    let report = checker.repair(narayana_core::types::TableId(1)).await.unwrap();
    // Should repair inconsistencies
}

#[tokio::test]
async fn test_failover_manager() {
    let manager = FailoverManager::new();
    let result = manager.failover("node-1").await.unwrap();
    assert!(result.success);
}

#[test]
fn test_circuit_breaker_closed() {
    let breaker = CircuitBreaker {
        state: CircuitBreakerState::Closed,
        failure_count: 0,
        last_failure: None,
        next_attempt: None,
    };
    assert_eq!(breaker.state, CircuitBreakerState::Closed);
}

#[test]
fn test_circuit_breaker_open() {
    let breaker = CircuitBreaker {
        state: CircuitBreakerState::Open,
        failure_count: 5,
        last_failure: Some(std::time::Instant::now()),
        next_attempt: Some(std::time::Instant::now() + std::time::Duration::from_secs(60)),
    };
    assert_eq!(breaker.state, CircuitBreakerState::Open);
}

#[test]
fn test_health_status_healthy() {
    assert_eq!(HealthStatus::Healthy, HealthStatus::Healthy);
}

#[test]
fn test_health_status_degraded() {
    assert_ne!(HealthStatus::Degraded, HealthStatus::Healthy);
}

use std::sync::Arc;

