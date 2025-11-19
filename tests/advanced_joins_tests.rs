// Tests for advanced joins

use narayana_storage::advanced_joins::*;
use narayana_core::types::TableId;

#[test]
fn test_advanced_join_executor_creation() {
    let executor = AdvancedJoinExecutor::new();
    // Should create successfully
}

#[test]
fn test_hash_join() {
    let executor = AdvancedJoinExecutor::new();
    let left_table = TableId(1);
    let right_table = TableId(2);
    let condition = JoinCondition {
        left_column: "id".to_string(),
        right_column: "user_id".to_string(),
        operator: JoinOperator::Eq,
    };
    
    let result = executor.hash_join(left_table, right_table, condition).unwrap();
    // Should execute successfully
}

#[test]
fn test_merge_join() {
    let executor = AdvancedJoinExecutor::new();
    let left_table = TableId(1);
    let right_table = TableId(2);
    let condition = JoinCondition {
        left_column: "id".to_string(),
        right_column: "user_id".to_string(),
        operator: JoinOperator::Eq,
    };
    
    let result = executor.merge_join(left_table, right_table, condition).unwrap();
    // Should execute successfully
}

#[test]
fn test_select_algorithm() {
    let executor = AdvancedJoinExecutor::new();
    let condition = JoinCondition {
        left_column: "id".to_string(),
        right_column: "user_id".to_string(),
        operator: JoinOperator::Eq,
    };
    
    // Small tables -> NestedLoop
    let algo = executor.select_algorithm(100, 100, &condition);
    assert!(matches!(algo, JoinAlgorithm::NestedLoop));
    
    // Equality -> Hash
    let algo = executor.select_algorithm(10000, 10000, &condition);
    assert!(matches!(algo, JoinAlgorithm::Hash));
}

#[test]
fn test_execute_join() {
    let executor = AdvancedJoinExecutor::new();
    let left_table = TableId(1);
    let right_table = TableId(2);
    let condition = JoinCondition {
        left_column: "id".to_string(),
        right_column: "user_id".to_string(),
        operator: JoinOperator::Eq,
    };
    
    let result = executor.execute_join(
        left_table,
        right_table,
        JoinType::Inner,
        condition,
        1000,
        1000,
    ).unwrap();
    // Should execute successfully
}

#[test]
fn test_foreign_key_manager_creation() {
    let manager = ForeignKeyManager::new();
    // Should create successfully
}

#[test]
fn test_create_foreign_key() {
    let manager = ForeignKeyManager::new();
    let fk = ForeignKey {
        name: "fk_user".to_string(),
        table: TableId(1),
        column: "user_id".to_string(),
        referenced_table: TableId(2),
        referenced_column: "id".to_string(),
        on_delete: OnDeleteAction::Cascade,
        on_update: OnUpdateAction::Restrict,
    };
    
    manager.create_foreign_key(fk).unwrap();
}

#[tokio::test]
async fn test_foreign_key_validate() {
    let manager = ForeignKeyManager::new();
    let fk = ForeignKey {
        name: "fk_user".to_string(),
        table: TableId(1),
        column: "user_id".to_string(),
        referenced_table: TableId(2),
        referenced_column: "id".to_string(),
        on_delete: OnDeleteAction::Cascade,
        on_update: OnUpdateAction::Restrict,
    };
    
    manager.create_foreign_key(fk).unwrap();
    let valid = manager.validate(TableId(1), "user_id", b"1").unwrap();
    assert!(valid);
}

