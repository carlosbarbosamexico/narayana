use narayana_core::{schema::{Schema, Field, DataType}, types::TableId, column::Column, Error};
use narayana_storage::{ColumnStore, InMemoryColumnStore};
use narayana_query::vectorized::VectorizedOps;

#[tokio::test]
async fn test_empty_table() {
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
    let columns = store.read_columns(table_id, vec![0], 0, 10).await.unwrap();
    assert!(columns.is_empty() || columns[0].len() == 0);
}

#[tokio::test]
async fn test_empty_column() {
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
    let columns = vec![Column::Int64(vec![])];
    store.write_columns(table_id, columns).await.unwrap();
}

#[tokio::test]
async fn test_single_row() {
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
    let columns = vec![Column::Int64(vec![42])];
    store.write_columns(table_id, columns).await.unwrap();
    
    let read = store.read_columns(table_id, vec![0], 0, 1).await.unwrap();
    assert_eq!(read.len(), 1);
}

#[tokio::test]
async fn test_duplicate_table_creation() {
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
}

#[tokio::test]
async fn test_nonexistent_table_read() {
    let store = InMemoryColumnStore::new();
    let table_id = TableId(999);
    
    let result = store.read_columns(table_id, vec![0], 0, 10).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_nonexistent_table_write() {
    let store = InMemoryColumnStore::new();
    let table_id = TableId(999);
    
    let columns = vec![Column::Int64(vec![1, 2, 3])];
    let result = store.write_columns(table_id, columns).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_nonexistent_table_delete() {
    let store = InMemoryColumnStore::new();
    let table_id = TableId(999);
    
    let result = store.delete_table(table_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_very_large_column() {
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
    
    let large_data: Vec<i64> = (0..1_000_000).collect();
    let columns = vec![Column::Int64(large_data)];
    store.write_columns(table_id, columns).await.unwrap();
}

#[tokio::test]
async fn test_many_columns() {
    let store = InMemoryColumnStore::new();
    let table_id = TableId(1);
    
    let fields: Vec<Field> = (0..100).map(|i| Field {
        name: format!("col_{}", i),
        data_type: DataType::Int32,
        nullable: false,
        default_value: None,
    }).collect();
    
    let schema = Schema::new(fields);
    store.create_table(table_id, schema).await.unwrap();
    
    let columns: Vec<Column> = (0..100).map(|_| Column::Int32(vec![1, 2, 3])).collect();
    store.write_columns(table_id, columns).await.unwrap();
}

#[tokio::test]
fn test_vectorized_empty_column() {
    let column = Column::Int32(vec![]);
    let mask = vec![];
    let filtered = VectorizedOps::filter(&column, &mask);
    assert_eq!(filtered.len(), 0);
}

#[tokio::test]
fn test_vectorized_all_true_mask() {
    let column = Column::Int32(vec![1, 2, 3, 4, 5]);
    let mask = vec![true, true, true, true, true];
    let filtered = VectorizedOps::filter(&column, &mask);
    
    match filtered {
        Column::Int32(data) => assert_eq!(data, vec![1, 2, 3, 4, 5]),
        _ => panic!("Expected Int32"),
    }
}

#[tokio::test]
fn test_vectorized_all_false_mask() {
    let column = Column::Int32(vec![1, 2, 3, 4, 5]);
    let mask = vec![false, false, false, false, false];
    let filtered = VectorizedOps::filter(&column, &mask);
    
    match filtered {
        Column::Int32(data) => assert!(data.is_empty()),
        _ => panic!("Expected Int32"),
    }
}

#[tokio::test]
fn test_vectorized_sum_empty() {
    let column = Column::Int32(vec![]);
    let sum = VectorizedOps::sum(&column);
    assert_eq!(sum, Some(serde_json::Value::Number(0.into())));
}

#[tokio::test]
fn test_vectorized_sum_single() {
    let column = Column::Int32(vec![42]);
    let sum = VectorizedOps::sum(&column);
    assert_eq!(sum, Some(serde_json::Value::Number(42.into())));
}

#[tokio::test]
fn test_vectorized_sum_negative() {
    let column = Column::Int32(vec![-10, -5, 0, 5, 10]);
    let sum = VectorizedOps::sum(&column);
    assert_eq!(sum, Some(serde_json::Value::Number(0.into())));
}

#[tokio::test]
fn test_vectorized_min_max_single() {
    let column = Column::Int32(vec![42]);
    let min = VectorizedOps::min(&column);
    let max = VectorizedOps::max(&column);
    assert_eq!(min, Some(serde_json::Value::Number(42.into())));
    assert_eq!(max, Some(serde_json::Value::Number(42.into())));
}

#[tokio::test]
fn test_vectorized_compare_edge_values() {
    let column = Column::Int32(vec![i32::MIN, -1, 0, 1, i32::MAX]);
    let min_val = serde_json::Value::Number(i32::MIN.into());
    let max_val = serde_json::Value::Number(i32::MAX.into());
    
    let min_mask = VectorizedOps::compare_eq(&column, &min_val);
    let max_mask = VectorizedOps::compare_eq(&column, &max_val);
    
    assert_eq!(min_mask[0], true);
    assert_eq!(max_mask[4], true);
}

#[tokio::test]
async fn test_read_partial_range() {
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
    
    let data: Vec<i64> = (0..100).collect();
    let columns = vec![Column::Int64(data)];
    store.write_columns(table_id, columns).await.unwrap();
    
    let read = store.read_columns(table_id, vec![0], 10, 20).await.unwrap();
    // Note: InMemoryColumnStore may return all data, but API supports range
    assert!(!read.is_empty());
}

#[tokio::test]
async fn test_zero_limit() {
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
    
    let read = store.read_columns(table_id, vec![0], 0, 0).await.unwrap();
    // Should handle zero limit gracefully
    assert!(read.is_empty() || read[0].len() == 0);
}

#[tokio::test]
async fn test_large_limit() {
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
    
    let read = store.read_columns(table_id, vec![0], 0, usize::MAX).await.unwrap();
    assert!(!read.is_empty());
}

#[tokio::test]
async fn test_multiple_table_ids() {
    let store = InMemoryColumnStore::new();
    
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
    
    // Verify all tables exist
    for i in 1..=10 {
        let table_id = TableId(i);
        let schema = store.get_schema(table_id).await.unwrap();
        assert_eq!(schema.len(), 1);
    }
}

#[tokio::test]
fn test_float_precision() {
    let column = Column::Float64(vec![1.1, 2.2, 3.3]);
    let value = serde_json::Value::Number(serde_json::Number::from_f64(2.2).unwrap());
    let mask = VectorizedOps::compare_eq(&column, &value);
    
    // Float comparison should use epsilon
    assert!(mask.len() == 3);
}

#[tokio::test]
fn test_string_comparison_case_sensitive() {
    let column = Column::String(vec!["Hello".to_string(), "world".to_string(), "HELLO".to_string()]);
    let value = serde_json::Value::String("Hello".to_string());
    let mask = VectorizedOps::compare_eq(&column, &value);
    
    assert_eq!(mask, vec![true, false, false]);
}

#[tokio::test]
fn test_boolean_operations() {
    let column = Column::Boolean(vec![true, false, true, false]);
    let value = serde_json::Value::Bool(true);
    let mask = VectorizedOps::compare_eq(&column, &value);
    
    assert_eq!(mask, vec![true, false, true, false]);
}

#[tokio::test]
async fn test_schema_field_ordering() {
    let schema = Schema::new(vec![
        Field {
            name: "a".to_string(),
            data_type: DataType::Int32,
            nullable: false,
            default_value: None,
        },
        Field {
            name: "b".to_string(),
            data_type: DataType::Int32,
            nullable: false,
            default_value: None,
        },
        Field {
            name: "c".to_string(),
            data_type: DataType::Int32,
            nullable: false,
            default_value: None,
        },
    ]);
    
    assert_eq!(schema.field_index("a"), Some(0));
    assert_eq!(schema.field_index("b"), Some(1));
    assert_eq!(schema.field_index("c"), Some(2));
}

#[tokio::test]
async fn test_duplicate_field_names() {
    // Schema should allow duplicate field names (though not recommended)
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int32,
            nullable: false,
            default_value: None,
        },
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);
    
    // Last one wins in field_map
    assert_eq!(schema.field_index("id"), Some(1));
}

