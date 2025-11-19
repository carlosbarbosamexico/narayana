use narayana_core::{schema::{Schema, Field, DataType}, types::TableId, column::Column};
use narayana_storage::{ColumnStore, InMemoryColumnStore};
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test]
async fn test_concurrent_table_creation() {
    let store = Arc::new(InMemoryColumnStore::new());
    let mut handles = vec![];
    
    for i in 1..=10 {
        let store = store.clone();
        let handle = tokio::spawn(async move {
            let table_id = TableId(i);
            let schema = Schema::new(vec![
                Field {
                    name: "id".to_string(),
                    data_type: DataType::Int64,
                    nullable: false,
                    default_value: None,
                },
            ]);
            store.create_table(table_id, schema).await
        });
        handles.push(handle);
    }
    
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_concurrent_writes() {
    let store = Arc::new(InMemoryColumnStore::new());
    let table_id = TableId(1);
    
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);

    store.create_table(table_id, schema).await.unwrap();
    
    let mut handles = vec![];
    for i in 0..10 {
        let store = store.clone();
        let handle = tokio::spawn(async move {
            let columns = vec![Column::Int64(vec![i as i64])];
            store.write_columns(table_id, columns).await
        });
        handles.push(handle);
    }
    
    for handle in handles {
        let result = handle.await.unwrap();
        // Note: InMemoryColumnStore doesn't share state across clones
        // This tests the API contract, not actual concurrency
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_concurrent_reads() {
    let store = Arc::new(InMemoryColumnStore::new());
    let table_id = TableId(1);
    
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);

    store.create_table(table_id, schema).await.unwrap();
    
    let columns = vec![Column::Int64(vec![1, 2, 3, 4, 5])];
    store.write_columns(table_id, columns).await.unwrap();
    
    let mut handles = vec![];
    for _ in 0..20 {
        let store = store.clone();
        let handle = tokio::spawn(async move {
            store.read_columns(table_id, vec![0], 0, 5).await
        });
        handles.push(handle);
    }
    
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_concurrent_read_write() {
    let store = Arc::new(InMemoryColumnStore::new());
    let table_id = TableId(1);
    
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);

    store.create_table(table_id, schema).await.unwrap();
    
    let write_store = store.clone();
    let read_store = store.clone();
    
    let write_handle = tokio::spawn(async move {
        for i in 0..10 {
            let columns = vec![Column::Int64(vec![i as i64])];
            write_store.write_columns(table_id, columns).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    });
    
    let read_handle = tokio::spawn(async move {
        for _ in 0..10 {
            let _ = read_store.read_columns(table_id, vec![0], 0, 10).await;
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    });
    
    tokio::try_join!(write_handle, read_handle).unwrap();
}

#[tokio::test]
async fn test_concurrent_deletes() {
    let store = Arc::new(InMemoryColumnStore::new());
    
    // Create multiple tables
    for i in 1..=5 {
        let table_id = TableId(i);
        let schema = Schema::new(vec![
            Field {
                name: "id".to_string(),
                data_type: DataType::Int64,
                nullable: false,
                default_value: None,
            },
        ]);
        store.create_table(table_id, schema).await.unwrap();
    }
    
    let mut handles = vec![];
    for i in 1..=5 {
        let store = store.clone();
        let handle = tokio::spawn(async move {
            let table_id = TableId(i);
            store.delete_table(table_id).await
        });
        handles.push(handle);
    }
    
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_race_condition_table_creation() {
    let store = Arc::new(InMemoryColumnStore::new());
    let table_id = TableId(1);
    
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);

    let mut handles = vec![];
    for _ in 0..5 {
        let store = store.clone();
        let schema = schema.clone();
        let handle = tokio::spawn(async move {
            store.create_table(table_id, schema).await
        });
        handles.push(handle);
    }
    
    let mut success_count = 0;
    for handle in handles {
        if handle.await.unwrap().is_ok() {
            success_count += 1;
        }
    }
    
    // Only one should succeed
    assert_eq!(success_count, 1);
}

#[tokio::test]
async fn test_high_concurrency_reads() {
    let store = Arc::new(InMemoryColumnStore::new());
    let table_id = TableId(1);
    
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);

    store.create_table(table_id, schema).await.unwrap();
    
    let columns = vec![Column::Int64((0..1000).collect())];
    store.write_columns(table_id, columns).await.unwrap();
    
    let mut handles = vec![];
    for _ in 0..100 {
        let store = store.clone();
        let handle = tokio::spawn(async move {
            store.read_columns(table_id, vec![0], 0, 1000).await
        });
        handles.push(handle);
    }
    
    let start = std::time::Instant::now();
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
    let duration = start.elapsed();
    
    // Should complete reasonably quickly
    assert!(duration.as_secs() < 5);
}

#[tokio::test]
async fn test_concurrent_schema_access() {
    let store = Arc::new(InMemoryColumnStore::new());
    let table_id = TableId(1);
    
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);

    store.create_table(table_id, schema).await.unwrap();
    
    let mut handles = vec![];
    for _ in 0..50 {
        let store = store.clone();
        let handle = tokio::spawn(async move {
            store.get_schema(table_id).await
        });
        handles.push(handle);
    }
    
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_parallel_operations_different_tables() {
    let store = Arc::new(InMemoryColumnStore::new());
    
    // Create tables
    for i in 1..=10 {
        let table_id = TableId(i);
        let schema = Schema::new(vec![
            Field {
                name: "id".to_string(),
                data_type: DataType::Int64,
                nullable: false,
                default_value: None,
            },
        ]);
        store.create_table(table_id, schema).await.unwrap();
    }
    
    let mut handles = vec![];
    for i in 1..=10 {
        let store = store.clone();
        let handle = tokio::spawn(async move {
            let table_id = TableId(i);
            let columns = vec![Column::Int64(vec![i as i64])];
            store.write_columns(table_id, columns).await?;
            store.read_columns(table_id, vec![0], 0, 1).await?;
            Ok::<(), narayana_core::Error>(())
        });
        handles.push(handle);
    }
    
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

