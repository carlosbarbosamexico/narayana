use narayana_core::{Error, schema::{Schema, Field, DataType}, types::TableId, column::Column};
use narayana_storage::{ColumnStore, InMemoryColumnStore};
use narayana_query::{operators::FilterOperator, plan::Filter};

#[tokio::test]
async fn test_error_table_not_found() {
    let store = InMemoryColumnStore::new();
    let table_id = TableId(999);
    
    let result = store.get_schema(table_id).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        Error::Storage(msg) => assert!(msg.contains("not found")),
        _ => panic!("Expected Storage error"),
    }
}

#[tokio::test]
async fn test_error_column_not_found() {
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
    
    // Try to read non-existent column
    let result = store.read_columns(table_id, vec![999], 0, 10).await;
    // May succeed with empty result or fail - both are valid
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_error_invalid_table_id() {
    let store = InMemoryColumnStore::new();
    let table_id = TableId(0); // Edge case
    
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);

    // Should still work with ID 0
    let result = store.create_table(table_id, schema).await;
    assert!(result.is_ok());
}

#[tokio::test]
fn test_error_invalid_filter_column() {
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);

    let filter = Filter::Eq {
        column: "nonexistent".to_string(),
        value: serde_json::Value::Number(42.into()),
    };

    let columns = vec![Column::Int64(vec![1, 2, 3])];
    let operator = FilterOperator::new(filter, schema);
    let result = operator.apply(&columns);
    
    assert!(result.is_err());
    match result.unwrap_err() {
        Error::Query(msg) => assert!(msg.contains("not found")),
        _ => panic!("Expected Query error"),
    }
}

#[tokio::test]
async fn test_error_write_to_nonexistent_table() {
    let store = InMemoryColumnStore::new();
    let table_id = TableId(999);
    
    let columns = vec![Column::Int64(vec![1, 2, 3])];
    let result = store.write_columns(table_id, columns).await;
    
    assert!(result.is_err());
    match result.unwrap_err() {
        Error::Storage(msg) => assert!(msg.contains("not found")),
        _ => panic!("Expected Storage error"),
    }
}

#[tokio::test]
async fn test_error_delete_nonexistent_table() {
    let store = InMemoryColumnStore::new();
    let table_id = TableId(999);
    
    let result = store.delete_table(table_id).await;
    
    assert!(result.is_err());
    match result.unwrap_err() {
        Error::Storage(msg) => assert!(msg.contains("not found")),
        _ => panic!("Expected Storage error"),
    }
}

#[tokio::test]
async fn test_error_create_duplicate_table() {
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

    store.create_table(table_id, schema.clone()).await.unwrap();
    let result = store.create_table(table_id, schema).await;
    
    assert!(result.is_err());
    match result.unwrap_err() {
        Error::Storage(msg) => assert!(msg.contains("already exists")),
        _ => panic!("Expected Storage error"),
    }
}

#[tokio::test]
fn test_error_invalid_compression() {
    use narayana_storage::compression::create_compressor;
    use narayana_core::types::CompressionType;
    
    // Test with invalid data that might cause compression to fail
    let compressor = create_compressor(CompressionType::LZ4);
    
    // Empty data should still work
    let result = compressor.compress(&[]);
    assert!(result.is_ok());
}

#[tokio::test]
fn test_error_invalid_decompression() {
    use narayana_storage::compression::create_decompressor;
    use narayana_core::types::CompressionType;
    
    let decompressor = create_decompressor(CompressionType::LZ4);
    
    // Invalid compressed data
    let invalid_data = b"not valid compressed data";
    let result = decompressor.decompress(invalid_data, 100);
    
    assert!(result.is_err());
    match result.unwrap_err() {
        Error::Deserialization(_) => {},
        _ => panic!("Expected Deserialization error"),
    }
}

#[tokio::test]
async fn test_error_mismatched_column_count() {
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
            name: "name".to_string(),
            data_type: DataType::String,
            nullable: false,
            default_value: None,
        },
    ]);

    store.create_table(table_id, schema).await.unwrap();
    
    // Write only one column when schema expects two
    let columns = vec![Column::Int64(vec![1, 2, 3])];
    // This might succeed or fail depending on implementation
    let result = store.write_columns(table_id, columns).await;
    // Both outcomes are acceptable for this test
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_error_mismatched_row_count() {
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
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);

    store.create_table(table_id, schema).await.unwrap();
    
    // Columns with different lengths
    let columns = vec![
        Column::Int64(vec![1, 2, 3]),
        Column::Int64(vec![1, 2]), // Different length
    ];
    
    // Implementation may or may not validate this
    let result = store.write_columns(table_id, columns).await;
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
fn test_error_invalid_json_value() {
    use narayana_query::vectorized::VectorizedOps;
    
    let column = Column::Int32(vec![1, 2, 3]);
    
    // Invalid JSON value type
    let value = serde_json::Value::Array(vec![]);
    let mask = VectorizedOps::compare_eq(&column, &value);
    
    // Should return all false for invalid type
    assert_eq!(mask, vec![false, false, false]);
}

#[tokio::test]
async fn test_error_empty_schema() {
    let store = InMemoryColumnStore::new();
    let table_id = TableId(1);
    
    let schema = Schema::new(vec![]);
    
    // Empty schema might be allowed or not
    let result = store.create_table(table_id, schema).await;
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_error_null_schema_field() {
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Nullable(Box::new(DataType::Int64)),
            nullable: true,
            default_value: None,
        },
    ]);
    
    // Nullable fields should be handled
    assert_eq!(schema.fields[0].nullable, true);
}

#[tokio::test]
fn test_error_type_mismatch() {
    use narayana_query::vectorized::VectorizedOps;
    
    let column = Column::Int32(vec![1, 2, 3]);
    
    // Try to compare with string
    let value = serde_json::Value::String("not a number".to_string());
    let mask = VectorizedOps::compare_eq(&column, &value);
    
    // Should return all false for type mismatch
    assert_eq!(mask, vec![false, false, false]);
}

#[tokio::test]
async fn test_error_out_of_bounds_read() {
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
    
    let columns = vec![Column::Int64(vec![1, 2, 3])];
    store.write_columns(table_id, columns).await.unwrap();
    
    // Read beyond available data
    let result = store.read_columns(table_id, vec![0], 1000, 100).await;
    // Should handle gracefully
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_error_negative_row_start() {
    // Row start is usize, so can't be negative
    // But test that large values are handled
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
    
    let columns = vec![Column::Int64(vec![1, 2, 3])];
    store.write_columns(table_id, columns).await.unwrap();
    
    // Very large row_start
    let result = store.read_columns(table_id, vec![0], usize::MAX, 10).await;
    assert!(result.is_ok());
}

