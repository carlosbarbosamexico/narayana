// Tests for error propagation through the system

use narayana_core::Error;
use narayana_storage::{ColumnStore, InMemoryColumnStore};
use narayana_core::{schema::{Schema, Field, DataType}, types::TableId};

#[tokio::test]
async fn test_error_propagation_table_not_found() {
    let store = InMemoryColumnStore::new();
    let table_id = TableId(999);
    
    // Error should propagate correctly
    let result = store.get_schema(table_id).await;
    assert!(result.is_err());
    
    match result.unwrap_err() {
        Error::Storage(msg) => {
            assert!(msg.contains("not found") || msg.contains("999"));
        }
        _ => panic!("Expected Storage error"),
    }
}

#[tokio::test]
async fn test_error_propagation_chain() {
    let store = InMemoryColumnStore::new();
    let table_id = TableId(999);
    
    // Error chain: read -> table not found -> storage error
    let result = store.read_columns(table_id, vec![0], 0, 10).await;
    assert!(result.is_err());
    
    // Error should be properly typed
    match result.unwrap_err() {
        Error::Storage(_) => {},
        _ => panic!("Expected Storage error"),
    }
}

#[tokio::test]
async fn test_error_propagation_write_to_nonexistent() {
    let store = InMemoryColumnStore::new();
    let table_id = TableId(999);
    
    use narayana_core::column::Column;
    let columns = vec![Column::Int64(vec![1, 2, 3])];
    
    let result = store.write_columns(table_id, columns).await;
    assert!(result.is_err());
    
    // Error should propagate
    match result.unwrap_err() {
        Error::Storage(msg) => {
            assert!(msg.contains("not found") || msg.contains("999"));
        }
        _ => panic!("Expected Storage error"),
    }
}

