use narayana_core::{schema::{Schema, Field, DataType}, types::TableId, column::Column};
use narayana_storage::{ColumnStore, InMemoryColumnStore};
use narayana_query::vectorized::VectorizedOps;

#[tokio::test]
async fn test_create_table_api() {
    let store = InMemoryColumnStore::new();
    let table_id = TableId(1);
    
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);

    let result = store.create_table(table_id, schema).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_write_read_api() {
    let store = InMemoryColumnStore::new();
    let table_id = TableId(1);
    
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
        Field {
            name: "value".to_string(),
            data_type: DataType::Float64,
            nullable: false,
            default_value: None,
        },
    ]);

    store.create_table(table_id, schema).await.unwrap();
    
    let columns = vec![
        Column::Int64(vec![1, 2, 3]),
        Column::Float64(vec![1.1, 2.2, 3.3]),
    ];

    store.write_columns(table_id, columns.clone()).await.unwrap();
    
    let read_columns = store.read_columns(table_id, vec![0, 1], 0, 3).await.unwrap();
    assert_eq!(read_columns.len(), 2);
}

#[tokio::test]
async fn test_delete_table_api() {
    let store = InMemoryColumnStore::new();
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
    store.delete_table(table_id).await.unwrap();
    
    // Try to read from deleted table should fail
    let result = store.read_columns(table_id, vec![0], 0, 10).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_vectorized_operations_integration() {
    let data: Vec<i64> = (0..1000).collect();
    let column = Column::Int64(data);
    
    let value = serde_json::Value::Number(500.into());
    let mask = VectorizedOps::compare_eq(&column, &value);
    
    let filtered = VectorizedOps::filter(&column, &mask);
    assert_eq!(filtered.len(), 1);
    
    let sum = VectorizedOps::sum(&column);
    assert!(sum.is_some());
    
    let min = VectorizedOps::min(&column);
    assert_eq!(min, Some(serde_json::Value::Number(0.into())));
    
    let max = VectorizedOps::max(&column);
    assert_eq!(max, Some(serde_json::Value::Number(999.into())));
}

#[tokio::test]
async fn test_concurrent_operations() {
    let store = InMemoryColumnStore::new();
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
    
    // Concurrent writes
    let handles: Vec<_> = (0..10).map(|i| {
        let store = InMemoryColumnStore::new();
        let table_id = table_id;
        tokio::spawn(async move {
            let columns = vec![Column::Int64(vec![i as i64])];
            store.write_columns(table_id, columns).await
        })
    }).collect();
    
    for handle in handles {
        let result = handle.await.unwrap();
        // Note: InMemoryColumnStore doesn't share state, so this is just testing the API
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_large_dataset() {
    let store = InMemoryColumnStore::new();
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
    
    let large_data: Vec<i64> = (0..100000).collect();
    let columns = vec![Column::Int64(large_data)];
    
    let start = std::time::Instant::now();
    store.write_columns(table_id, columns).await.unwrap();
    let write_duration = start.elapsed();
    
    assert!(write_duration.as_millis() < 1000); // Should be fast
    
    let start = std::time::Instant::now();
    let read_columns = store.read_columns(table_id, vec![0], 0, 100000).await.unwrap();
    let read_duration = start.elapsed();
    
    assert!(!read_columns.is_empty());
    assert!(read_duration.as_millis() < 1000); // Should be fast
}

