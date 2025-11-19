use narayana_core::{schema::{Schema, Field, DataType}, types::TableId, column::Column};
use narayana_storage::{ColumnStore, InMemoryColumnStore};
use narayana_query::vectorized::VectorizedOps;

#[tokio::test]
async fn test_integration_write_read() {
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
        Column::Int64(vec![1, 2, 3, 4, 5]),
        Column::Float64(vec![1.1, 2.2, 3.3, 4.4, 5.5]),
    ];

    store.write_columns(table_id, columns).await.unwrap();
    
    let read_columns = store.read_columns(table_id, vec![0, 1], 0, 5).await.unwrap();
    assert_eq!(read_columns.len(), 2);
}

#[tokio::test]
async fn test_vectorized_operations() {
    let data: Vec<i64> = (0..1000).collect();
    let column = Column::Int64(data);
    
    let value = serde_json::Value::Number(500.into());
    let mask = VectorizedOps::compare_eq(&column, &value);
    
    let filtered = VectorizedOps::filter(&column, &mask);
    assert_eq!(filtered.len(), 1);
    
    let sum = VectorizedOps::sum(&column);
    assert!(sum.is_some());
}

#[tokio::test]
async fn test_integration_full_workflow() {
    let store = InMemoryColumnStore::new();
    let table_id = TableId(1);
    
    // Create table
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
        Field {
            name: "name".to_string(),
            data_type: DataType::String,
            nullable: false,
            default_value: None,
        },
        Field {
            name: "score".to_string(),
            data_type: DataType::Float64,
            nullable: false,
            default_value: None,
        },
    ]);

    store.create_table(table_id, schema).await.unwrap();
    
    // Write data
    let columns = vec![
        Column::Int64(vec![1, 2, 3, 4, 5]),
        Column::String(vec!["Alice".to_string(), "Bob".to_string(), "Charlie".to_string(), "David".to_string(), "Eve".to_string()]),
        Column::Float64(vec![95.5, 87.0, 92.5, 78.5, 99.0]),
    ];
    store.write_columns(table_id, columns).await.unwrap();
    
    // Read data
    let read_columns = store.read_columns(table_id, vec![0, 1, 2], 0, 5).await.unwrap();
    assert_eq!(read_columns.len(), 3);
    
    // Query with filter
    let score_column = &read_columns[2];
    let high_score = serde_json::Value::Number(serde_json::Number::from_f64(90.0).unwrap());
    let mask = VectorizedOps::compare_gt(score_column, &high_score);
    let filtered = VectorizedOps::filter(score_column, &mask);
    
    assert!(filtered.len() >= 2); // At least 95.5, 92.5, 99.0
    
    // Get schema
    let schema = store.get_schema(table_id).await.unwrap();
    assert_eq!(schema.len(), 3);
}

#[tokio::test]
async fn test_integration_multiple_tables() {
    let store = InMemoryColumnStore::new();
    
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
        
        let columns = vec![Column::Int64(vec![i as i64])];
        store.write_columns(table_id, columns).await.unwrap();
    }
    
    // Read from all tables
    for i in 1..=5 {
        let table_id = TableId(i);
        let read = store.read_columns(table_id, vec![0], 0, 1).await.unwrap();
        assert!(!read.is_empty());
    }
}

#[tokio::test]
async fn test_integration_delete_and_recreate() {
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

    // Create, write, delete
    store.create_table(table_id, schema.clone()).await.unwrap();
    let columns = vec![Column::Int64(vec![1, 2, 3])];
    store.write_columns(table_id, columns).await.unwrap();
    store.delete_table(table_id).await.unwrap();
    
    // Recreate
    store.create_table(table_id, schema).await.unwrap();
    let columns = vec![Column::Int64(vec![4, 5, 6])];
    store.write_columns(table_id, columns).await.unwrap();
    
    let read = store.read_columns(table_id, vec![0], 0, 3).await.unwrap();
    assert!(!read.is_empty());
}

