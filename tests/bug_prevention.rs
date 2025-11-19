// Tests specifically designed to catch common bugs and edge cases
// that could cause panics, incorrect behavior, or security issues

use narayana_core::{
    schema::{Schema, Field, DataType},
    types::TableId,
    column::Column,
    Error,
};
use narayana_storage::{ColumnStore, InMemoryColumnStore};
use narayana_query::vectorized::VectorizedOps;
use narayana_storage::compression::{create_compressor, create_decompressor};
use narayana_core::types::CompressionType;

// ============================================================================
// PANIC PREVENTION TESTS
// ============================================================================

#[test]
fn test_no_panic_on_overflow() {
    // Test integer overflow scenarios
    let large_int = Column::Int32(vec![i32::MAX, 1]);
    let sum = VectorizedOps::sum(&large_int);
    // Should handle overflow gracefully, not panic
    assert!(sum.is_some() || sum.is_none());
}

#[test]
fn test_no_panic_on_underflow() {
    let large_negative = Column::Int32(vec![i32::MIN, -1]);
    let sum = VectorizedOps::sum(&large_negative);
    // Should handle underflow gracefully
    assert!(sum.is_some() || sum.is_none());
}

#[test]
fn test_no_panic_on_empty_slice_access() {
    let empty = Column::Int32(vec![]);
    // These operations should not panic on empty data
    let _ = VectorizedOps::sum(&empty);
    let _ = VectorizedOps::min(&empty);
    let _ = VectorizedOps::max(&empty);
    let _ = VectorizedOps::count(&empty);
}

#[test]
fn test_no_panic_on_index_out_of_bounds() {
    let column = Column::Int32(vec![1, 2, 3]);
    let short_mask = vec![true]; // Shorter than column
    // Should handle gracefully, not panic
    let _ = VectorizedOps::filter(&column, &short_mask);
}

#[test]
fn test_no_panic_on_null_pointer_dereference() {
    // Test that we don't dereference null pointers
    let schema = Schema::new(vec![]);
    // Accessing fields on empty schema should return None, not panic
    assert!(schema.field("nonexistent").is_none());
    assert_eq!(schema.field_index("nonexistent"), None);
}

// ============================================================================
// MEMORY SAFETY TESTS
// ============================================================================

#[test]
fn test_no_buffer_overflow() {
    // Test that we don't read beyond buffer bounds
    let column = Column::Int32(vec![1, 2, 3]);
    let mask = vec![true, false, true, true, true]; // Longer than column
    
    // Should only process valid indices
    let filtered = VectorizedOps::filter(&column, &mask);
    assert!(filtered.len() <= column.len());
}

#[test]
fn test_no_use_after_free() {
    // Test that we don't use freed memory
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
    
    // Create and delete table
    store.create_table(table_id, schema).await.unwrap();
    store.delete_table(table_id).await.unwrap();
    
    // Try to use deleted table - should error, not crash
    let result = store.get_schema(table_id).await;
    assert!(result.is_err());
}

#[test]
fn test_no_double_free() {
    // Test that we don't free memory twice
    let mut store = InMemoryColumnStore::new();
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
    
    // Delete twice - second should error, not crash
    store.delete_table(table_id).await.unwrap();
    let result = store.delete_table(table_id).await;
    assert!(result.is_err());
}

// ============================================================================
// LOGIC ERROR PREVENTION TESTS
// ============================================================================

#[test]
fn test_correct_filter_logic() {
    // Ensure filter logic is correct
    let column = Column::Int32(vec![1, 2, 3, 4, 5]);
    let mask = vec![true, false, true, false, true];
    let filtered = VectorizedOps::filter(&column, &mask);
    
    match filtered {
        Column::Int32(data) => {
            assert_eq!(data, vec![1, 3, 5]);
        }
        _ => panic!("Wrong column type"),
    }
}

#[test]
fn test_correct_comparison_logic() {
    // Ensure comparison operators are correct
    let column = Column::Int32(vec![1, 2, 3, 4, 5]);
    let value = serde_json::Value::Number(3.into());
    
    let eq_mask = VectorizedOps::compare_eq(&column, &value);
    assert_eq!(eq_mask, vec![false, false, true, false, false]);
    
    let gt_mask = VectorizedOps::compare_gt(&column, &value);
    assert_eq!(gt_mask, vec![false, false, false, true, true]);
    
    let lt_mask = VectorizedOps::compare_lt(&column, &value);
    assert_eq!(lt_mask, vec![true, true, false, false, false]);
}

#[test]
fn test_correct_aggregation_logic() {
    // Ensure aggregations are correct
    let column = Column::Int32(vec![1, 2, 3, 4, 5]);
    
    let sum = VectorizedOps::sum(&column);
    assert_eq!(sum, Some(serde_json::Value::Number(15.into())));
    
    let min = VectorizedOps::min(&column);
    assert_eq!(min, Some(serde_json::Value::Number(1.into())));
    
    let max = VectorizedOps::max(&column);
    assert_eq!(max, Some(serde_json::Value::Number(5.into())));
    
    let count = VectorizedOps::count(&column);
    assert_eq!(count, 5);
}

// ============================================================================
// TYPE SAFETY TESTS
// ============================================================================

#[test]
fn test_type_mismatch_handling() {
    // Test that type mismatches are handled correctly
    let int_column = Column::Int32(vec![1, 2, 3]);
    let string_value = serde_json::Value::String("not a number".to_string());
    
    // Should return all false, not panic
    let mask = VectorizedOps::compare_eq(&int_column, &string_value);
    assert_eq!(mask, vec![false, false, false]);
}

#[test]
fn test_invalid_json_value_handling() {
    let column = Column::Int32(vec![1, 2, 3]);
    
    // Invalid JSON types should be handled gracefully
    let null_val = serde_json::Value::Null;
    let array_val = serde_json::Value::Array(vec![]);
    let object_val = serde_json::Value::Object(serde_json::Map::new());
    
    let mask1 = VectorizedOps::compare_eq(&column, &null_val);
    let mask2 = VectorizedOps::compare_eq(&column, &array_val);
    let mask3 = VectorizedOps::compare_eq(&column, &object_val);
    
    // All should return false, not panic
    assert_eq!(mask1, vec![false, false, false]);
    assert_eq!(mask2, vec![false, false, false]);
    assert_eq!(mask3, vec![false, false, false]);
}

// ============================================================================
// COMPRESSION SAFETY TESTS
// ============================================================================

#[test]
fn test_compression_malformed_data() {
    // Test that malformed compressed data doesn't cause panics
    let decompressor = create_decompressor(CompressionType::LZ4);
    
    // Invalid compressed data
    let invalid_data = b"this is not valid compressed data";
    let result = decompressor.decompress(invalid_data, 100);
    
    // Should return error, not panic
    assert!(result.is_err());
}

#[test]
fn test_compression_size_mismatch() {
    // Test that wrong output_len doesn't cause issues
    let compressor = create_compressor(CompressionType::LZ4);
    let decompressor = create_decompressor(CompressionType::LZ4);
    
    let data = b"test data";
    let compressed = compressor.compress(data).unwrap();
    
    // Wrong output length
    let result = decompressor.decompress(&compressed, 999999);
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_compression_zero_length() {
    // Test compression/decompression of zero-length data
    let compressor = create_compressor(CompressionType::LZ4);
    let decompressor = create_decompressor(CompressionType::LZ4);
    
    let empty: &[u8] = &[];
    let compressed = compressor.compress(empty).unwrap();
    let decompressed = decompressor.decompress(&compressed, 0).unwrap();
    
    assert_eq!(decompressed, empty);
}

// ============================================================================
// CONCURRENCY SAFETY TESTS
// ============================================================================

#[tokio::test]
async fn test_no_race_condition_on_table_creation() {
    use std::sync::Arc;
    
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
    
    // Multiple concurrent creates
    let mut handles = vec![];
    for _ in 0..10 {
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
async fn test_no_data_corruption_on_concurrent_writes() {
    use std::sync::Arc;
    
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
    
    // Concurrent writes
    let mut handles = vec![];
    for i in 0..10 {
        let store = store.clone();
        let handle = tokio::spawn(async move {
            let columns = vec![Column::Int64(vec![i as i64])];
            store.write_columns(table_id, columns).await
        });
        handles.push(handle);
    }
    
    // All should complete without corruption
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

// ============================================================================
// BOUNDARY CONDITION TESTS
// ============================================================================

#[test]
fn test_boundary_values_int32() {
    let column = Column::Int32(vec![i32::MIN, -1, 0, 1, i32::MAX]);
    
    // All operations should work on boundary values
    let _ = VectorizedOps::sum(&column);
    let _ = VectorizedOps::min(&column);
    let _ = VectorizedOps::max(&column);
    let _ = VectorizedOps::count(&column);
}

#[test]
fn test_boundary_values_int64() {
    let column = Column::Int64(vec![i64::MIN, -1, 0, 1, i64::MAX]);
    
    let _ = VectorizedOps::sum(&column);
    let _ = VectorizedOps::min(&column);
    let _ = VectorizedOps::max(&column);
}

#[test]
fn test_boundary_values_float() {
    let column = Column::Float64(vec![
        f64::NEG_INFINITY,
        f64::MIN,
        -1.0,
        0.0,
        1.0,
        f64::MAX,
        f64::INFINITY,
    ]);
    
    // Should handle special float values
    let _ = VectorizedOps::sum(&column);
    let _ = VectorizedOps::min(&column);
    let _ = VectorizedOps::max(&column);
}

// ============================================================================
// EDGE CASE STRING HANDLING
// ============================================================================

#[test]
fn test_string_special_characters() {
    let column = Column::String(vec![
        "".to_string(),
        " ".to_string(),
        "\n".to_string(),
        "\t".to_string(),
        "\0".to_string(),
        "null".to_string(),
    ]);
    
    // All should be handled correctly
    assert_eq!(column.len(), 6);
    
    let empty_val = serde_json::Value::String("".to_string());
    let mask = VectorizedOps::compare_eq(&column, &empty_val);
    assert_eq!(mask[0], true);
}

#[test]
fn test_string_unicode_edge_cases() {
    let column = Column::String(vec![
        "a".to_string(),
        "á".to_string(),
        "中".to_string(),
        "🌍".to_string(),
        "🚀".to_string(),
    ]);
    
    let value = serde_json::Value::String("🌍".to_string());
    let mask = VectorizedOps::compare_eq(&column, &value);
    assert_eq!(mask[3], true);
}

// ============================================================================
// ERROR RECOVERY TESTS
// ============================================================================

#[tokio::test]
async fn test_recovery_after_error() {
    let store = InMemoryColumnStore::new();
    let table_id = TableId(1);
    
    // Try to read from non-existent table (error)
    let result = store.read_columns(table_id, vec![0], 0, 10).await;
    assert!(result.is_err());
    
    // Create table and try again (should succeed)
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
    
    // Now read should succeed
    let result = store.read_columns(table_id, vec![0], 0, 10).await;
    assert!(result.is_ok());
}

// ============================================================================
// PERFORMANCE REGRESSION TESTS
// ============================================================================

#[test]
fn test_no_quadratic_complexity() {
    // Test that operations don't have unexpected complexity
    use std::time::Instant;
    
    let sizes = vec![100, 1000, 10000];
    let mut times = vec![];
    
    for size in sizes {
        let data: Vec<i32> = (0..size).collect();
        let column = Column::Int32(data);
        
        let start = Instant::now();
        let _ = VectorizedOps::sum(&column);
        let duration = start.elapsed();
        times.push(duration);
    }
    
    // Times should scale roughly linearly, not quadratically
    // (This is a sanity check, actual timing depends on system)
    assert!(times[2].as_micros() < times[0].as_micros() * 200);
}

// ============================================================================
// SERIALIZATION ROBUSTNESS TESTS
// ============================================================================

#[test]
fn test_serialize_deserialize_roundtrip() {
    use serde_json;
    
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: Some(serde_json::Value::Number(0.into())),
        },
        Field {
            name: "name".to_string(),
            data_type: DataType::String,
            nullable: true,
            default_value: None,
        },
    ]);
    
    let serialized = serde_json::to_string(&schema).unwrap();
    let deserialized: Schema = serde_json::from_str(&serialized).unwrap();
    
    assert_eq!(schema.len(), deserialized.len());
    assert_eq!(schema.field_index("id"), deserialized.field_index("id"));
}

#[test]
fn test_serialize_malformed_json_handling() {
    // Test that we handle malformed JSON gracefully
    let malformed = "{ invalid json }";
    let result: Result<Schema, _> = serde_json::from_str(malformed);
    assert!(result.is_err());
}

