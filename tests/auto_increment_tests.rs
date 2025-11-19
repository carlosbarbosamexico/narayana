// Tests for auto-increment

use narayana_storage::auto_increment::*;
use narayana_core::types::TableId;

#[test]
fn test_auto_increment_manager_creation() {
    let manager = AutoIncrementManager::new();
    // Should create successfully
}

#[test]
fn test_create_sequence() {
    let manager = AutoIncrementManager::new();
    manager.create_sequence("test_seq".to_string(), 1);
    // Should create successfully
}

#[test]
fn test_next_value() {
    let manager = AutoIncrementManager::new();
    manager.create_sequence("test_seq".to_string(), 1);
    
    let value1 = manager.next("test_seq").unwrap();
    assert_eq!(value1, 1);
    
    let value2 = manager.next("test_seq").unwrap();
    assert_eq!(value2, 2);
}

#[test]
fn test_current_value() {
    let manager = AutoIncrementManager::new();
    manager.create_sequence("test_seq".to_string(), 1);
    
    manager.next("test_seq").unwrap();
    let current = manager.current("test_seq").unwrap();
    assert_eq!(current, 2);
}

#[test]
fn test_set_sequence() {
    let manager = AutoIncrementManager::new();
    manager.create_sequence("test_seq".to_string(), 1);
    
    manager.set("test_seq", 100).unwrap();
    let current = manager.current("test_seq").unwrap();
    assert_eq!(current, 100);
}

#[test]
fn test_reset_sequence() {
    let manager = AutoIncrementManager::new();
    manager.create_sequence("test_seq".to_string(), 1);
    
    manager.next("test_seq").unwrap();
    manager.reset("test_seq", 1).unwrap();
    
    let value = manager.next("test_seq").unwrap();
    assert_eq!(value, 1);
}

#[test]
fn test_table_auto_increment_creation() {
    let auto_inc = TableAutoIncrement::new();
    // Should create successfully
}

#[test]
fn test_table_auto_increment_enable() {
    let auto_inc = TableAutoIncrement::new();
    let table_id = TableId(1);
    auto_inc.enable(table_id, "id", 1);
    // Should enable successfully
}

#[test]
fn test_table_auto_increment_next() {
    let auto_inc = TableAutoIncrement::new();
    let table_id = TableId(1);
    auto_inc.enable(table_id, "id", 1);
    
    let value1 = auto_inc.next(table_id, "id").unwrap();
    assert_eq!(value1, 1);
    
    let value2 = auto_inc.next(table_id, "id").unwrap();
    assert_eq!(value2, 2);
}

