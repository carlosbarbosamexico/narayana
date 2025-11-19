// Tests for mutable data handling

use narayana_storage::mutable_data::*;
use narayana_core::types::TableId;
use std::collections::HashMap;

#[test]
fn test_mutable_data_manager_creation() {
    let manager = MutableDataManager::new();
    // Should create successfully
}

#[test]
fn test_update() {
    let manager = MutableDataManager::new();
    let table_id = TableId(1);
    
    manager.update(table_id, 1, "column".to_string(), b"value".to_vec()).unwrap();
    
    let updates = manager.get_updates(table_id, 1);
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].column, "column");
}

#[test]
fn test_delete() {
    let manager = MutableDataManager::new();
    let table_id = TableId(1);
    
    manager.delete(table_id, 1).unwrap();
    assert!(manager.is_deleted(table_id, 1));
}

#[test]
fn test_is_deleted() {
    let manager = MutableDataManager::new();
    let table_id = TableId(1);
    
    assert!(!manager.is_deleted(table_id, 1));
    manager.delete(table_id, 1).unwrap();
    assert!(manager.is_deleted(table_id, 1));
}

#[tokio::test]
async fn test_merge_updates() {
    let manager = MutableDataManager::new();
    let table_id = TableId(1);
    
    manager.update(table_id, 1, "column".to_string(), b"value".to_vec()).unwrap();
    let count = manager.merge_updates(table_id).await.unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn test_compact_deletes() {
    let manager = MutableDataManager::new();
    let table_id = TableId(1);
    
    manager.delete(table_id, 1).unwrap();
    let count = manager.compact_deletes(table_id).await.unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn test_upsert() {
    let manager = MutableDataManager::new();
    let table_id = TableId(1);
    let mut data = HashMap::new();
    data.insert("column".to_string(), b"value".to_vec());
    
    manager.upsert(table_id, 1, data).await.unwrap();
    // Should upsert successfully
}

#[test]
fn test_delta_storage_creation() {
    let storage = DeltaStorage::new(TableId(1));
    // Should create successfully
}

#[test]
fn test_delta_storage_apply_delta() {
    let mut storage = DeltaStorage::new(TableId(1));
    storage.apply_delta(1, "column".to_string(), b"value".to_vec());
    // Should apply successfully
}

#[test]
fn test_delta_storage_get() {
    let mut storage = DeltaStorage::new(TableId(1));
    storage.apply_delta(1, "column".to_string(), b"value".to_vec());
    
    let value = storage.get(1, "column");
    assert_eq!(value, Some(b"value".to_vec()));
}

#[tokio::test]
async fn test_delta_storage_merge() {
    let mut storage = DeltaStorage::new(TableId(1));
    storage.apply_delta(1, "column".to_string(), b"value".to_vec());
    
    let count = storage.merge().await.unwrap();
    assert_eq!(count, 1);
}

