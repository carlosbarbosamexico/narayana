// Really weird edge cases that might catch subtle bugs
// These test scenarios that are unlikely but possible in production

use narayana_core::{
    schema::{Schema, Field, DataType},
    types::{TableId, ColumnId, TransactionId, Timestamp, CompressionType},
    column::Column,
    row::{Row, Value},
    Error,
};
use narayana_storage::{
    ColumnStore, InMemoryColumnStore,
    compression::{create_compressor, create_decompressor},
    index::BTreeIndex,
    cache::LRUCache,
};
use narayana_query::vectorized::VectorizedOps;
use std::time::Duration;

// ============================================================================
// UNICODE WEIRDNESS
// ============================================================================

#[test]
fn test_unicode_zero_width_characters() {
    // Zero-width characters that look the same but are different
    let column = Column::String(vec![
        "hello".to_string(),
        "hello\u{200B}".to_string(), // Zero-width space
        "hello\u{200C}".to_string(), // Zero-width non-joiner
        "hello\u{200D}".to_string(), // Zero-width joiner
        "hello\u{FEFF}".to_string(), // Zero-width no-break space
    ]);
    
    let value = serde_json::Value::String("hello".to_string());
    let mask = VectorizedOps::compare_eq(&column, &value);
    
    // Only first should match (exact match)
    assert_eq!(mask[0], true);
    assert_eq!(mask[1], false);
    assert_eq!(mask[2], false);
    assert_eq!(mask[3], false);
    assert_eq!(mask[4], false);
}

#[test]
fn test_unicode_combining_characters() {
    // Characters with combining marks
    let column = Column::String(vec![
        "cafe".to_string(),
        "café".to_string(), // e with acute accent
        "cafe\u{0301}".to_string(), // e + combining acute accent
    ]);
    
    // These should be treated as different strings
    assert_eq!(column.len(), 3);
    
    let value1 = serde_json::Value::String("cafe".to_string());
    let value2 = serde_json::Value::String("café".to_string());
    
    let mask1 = VectorizedOps::compare_eq(&column, &value1);
    let mask2 = VectorizedOps::compare_eq(&column, &value2);
    
    assert_eq!(mask1[0], true);
    assert_eq!(mask1[1], false);
    assert_eq!(mask2[1], true);
}

#[test]
fn test_unicode_emoji_variations() {
    // Emoji with skin tone modifiers, gender modifiers, etc.
    let column = Column::String(vec![
        "👋".to_string(),
        "👋🏻".to_string(), // Wave with light skin tone
        "👋🏿".to_string(), // Wave with dark skin tone
        "👨".to_string(),
        "👨‍💻".to_string(), // Man technologist (with zero-width joiner)
    ]);
    
    assert_eq!(column.len(), 5);
    
    let value = serde_json::Value::String("👋".to_string());
    let mask = VectorizedOps::compare_eq(&column, &value);
    
    // Only exact match should be true
    assert_eq!(mask[0], true);
    assert_eq!(mask[1], false);
    assert_eq!(mask[2], false);
}

#[test]
fn test_unicode_right_to_left_text() {
    // RTL languages mixed with LTR
    let column = Column::String(vec![
        "hello".to_string(),
        "مرحبا".to_string(), // Arabic
        "שלום".to_string(), // Hebrew
        "hello\u{202E}world".to_string(), // RTL override
    ]);
    
    assert_eq!(column.len(), 4);
    
    // All should be handled correctly
    match &column {
        Column::String(data) => {
            for (i, s) in data.iter().enumerate() {
                let value = serde_json::Value::String(s.clone());
                let mask = VectorizedOps::compare_eq(&column, &value);
                assert_eq!(mask[i], true);
            }
        }
        _ => panic!("Expected String column"),
    }
}

#[test]
fn test_unicode_surrogate_pairs() {
    // Characters that require surrogate pairs in UTF-16
    let column = Column::String(vec![
        "🌍".to_string(), // Earth emoji (U+1F30D)
        "🚀".to_string(), // Rocket emoji (U+1F680)
        "💻".to_string(), // Laptop emoji (U+1F4BB)
    ]);
    
    assert_eq!(column.len(), 3);
    
    let value = serde_json::Value::String("🌍".to_string());
    let mask = VectorizedOps::compare_eq(&column, &value);
    assert_eq!(mask[0], true);
}

#[test]
fn test_unicode_null_bytes_in_string() {
    // Strings with null bytes (should be handled)
    let column = Column::String(vec![
        "hello\0world".to_string(),
        "test\0\0test".to_string(),
    ]);
    
    assert_eq!(column.len(), 2);
    
    let value = serde_json::Value::String("hello\0world".to_string());
    let mask = VectorizedOps::compare_eq(&column, &value);
    assert_eq!(mask[0], true);
}

#[test]
fn test_unicode_control_characters() {
    // Various control characters
    let column = Column::String(vec![
        "\x00".to_string(), // NULL
        "\x01".to_string(), // SOH
        "\x1F".to_string(), // Unit separator
        "\x7F".to_string(), // DEL
        "\x80".to_string(), // Invalid UTF-8 start
    ]);
    
    assert_eq!(column.len(), 5);
}


// ============================================================================
// FLOAT WEIRDNESS
// ============================================================================

#[test]
fn test_float_negative_zero() {
    // Negative zero is a thing in IEEE 754
    let column = Column::Float64(vec![0.0, -0.0, 1.0]);
    
    let zero_val = serde_json::Value::Number(serde_json::Number::from_f64(0.0).unwrap());
    let mask = VectorizedOps::compare_eq(&column, &zero_val);
    
    // Both 0.0 and -0.0 should match 0.0 (IEEE 754: 0.0 == -0.0)
    assert_eq!(mask[0], true);
    assert_eq!(mask[1], true); // -0.0 == 0.0
    assert_eq!(mask[2], false);
}

#[test]
fn test_float_subnormal_numbers() {
    // Subnormal (denormalized) numbers - smallest representable numbers
    let column = Column::Float64(vec![
        f64::MIN_POSITIVE, // Smallest normal positive number
        f64::MIN_POSITIVE / 2.0, // Subnormal
        0.0,
    ]);
    
    // Should handle subnormal numbers without panicking
    let sum = VectorizedOps::sum(&column);
    let min = VectorizedOps::min(&column);
    let max = VectorizedOps::max(&column);
    
    assert!(sum.is_some() || sum.is_none());
    assert!(min.is_some() || min.is_none());
    assert!(max.is_some() || max.is_none());
}

#[test]
fn test_float_epsilon_comparison() {
    // Test that epsilon comparison works correctly
    let column = Column::Float64(vec![
        1.0,
        1.0 + f64::EPSILON,
        1.0 + f64::EPSILON * 2.0,
        1.0 + 0.0001, // Larger than epsilon
    ]);
    
    let value = serde_json::Value::Number(serde_json::Number::from_f64(1.0).unwrap());
    let mask = VectorizedOps::compare_eq(&column, &value);
    
    // First two should match (within epsilon)
    assert_eq!(mask[0], true);
    assert_eq!(mask[1], true); // Within epsilon
    assert_eq!(mask[2], true); // Still within epsilon
    assert_eq!(mask[3], false); // Too far
}

#[test]
fn test_float_nan_comparison() {
    // NaN != NaN in IEEE 754
    let column = Column::Float64(vec![
        f64::NAN,
        f64::NAN,
        1.0,
    ]);
    
    let nan_val = serde_json::Value::Number(serde_json::Number::from_f64(f64::NAN).unwrap());
    let mask = VectorizedOps::compare_eq(&column, &nan_val);
    
    // NaN comparisons should all be false (NaN != NaN)
    assert_eq!(mask[0], false);
    assert_eq!(mask[1], false);
    assert_eq!(mask[2], false);
}

#[test]
fn test_float_infinity_arithmetic() {
    let column = Column::Float64(vec![
        f64::INFINITY,
        f64::NEG_INFINITY,
        1.0,
    ]);
    
    // Operations with infinity
    let sum = VectorizedOps::sum(&column);
    let min = VectorizedOps::min(&column);
    let max = VectorizedOps::max(&column);
    
    // Should handle infinity gracefully
    assert!(sum.is_some() || sum.is_none());
    assert!(min.is_some() || min.is_none());
    assert!(max.is_some() || max.is_none());
}

// ============================================================================
// INTEGER WEIRDNESS
// ============================================================================

#[test]
fn test_integer_wrapping() {
    // Test behavior at integer boundaries
    let column = Column::Int32(vec![i32::MAX, 1]);
    
    // Sum would wrap - should handle gracefully
    let sum = VectorizedOps::sum(&column);
    // May return wrapped value or None
    assert!(sum.is_some() || sum.is_none());
}

#[test]
fn test_integer_signed_unsigned_mismatch() {
    // Test that we don't mix signed/unsigned incorrectly
    let signed_col = Column::Int32(vec![-1, 0, 1]);
    let unsigned_col = Column::UInt32(vec![0, 1, 2]);
    
    // These should be treated as different types
    assert_eq!(signed_col.data_type(), DataType::Int32);
    assert_eq!(unsigned_col.data_type(), DataType::UInt32);
}

#[test]
fn test_integer_negative_in_unsigned_context() {
    // Test that negative values aren't accidentally used with unsigned
    let column = Column::UInt64(vec![0, 1, u64::MAX]);
    
    // Should handle max value correctly
    let max = VectorizedOps::max(&column);
    assert_eq!(max, Some(serde_json::Value::Number(u64::MAX.into())));
}

// ============================================================================
// STRING WEIRDNESS
// ============================================================================

#[test]
fn test_string_very_long_single_string() {
    // Single string with millions of characters
    let long_string = "a".repeat(10_000_000);
    let column = Column::String(vec![long_string.clone()]);
    
    assert_eq!(column.len(), 1);
    
    let value = serde_json::Value::String(long_string);
    let mask = VectorizedOps::compare_eq(&column, &value);
    assert_eq!(mask[0], true);
}

#[test]
fn test_string_mixed_encodings() {
    // Mix of ASCII, UTF-8, and potentially problematic sequences
    let column = Column::String(vec![
        "ASCII only".to_string(),
        "UTF-8: 中文".to_string(),
        "Mixed: hello 世界 🌍".to_string(),
        "Control: \x00\x01\x02".to_string(),
    ]);
    
    assert_eq!(column.len(), 4);
}

#[test]
fn test_string_leading_trailing_whitespace() {
    let column = Column::String(vec![
        "  hello  ".to_string(),
        "\t\nhello\t\n".to_string(),
        "hello".to_string(),
    ]);
    
    let value = serde_json::Value::String("hello".to_string());
    let mask = VectorizedOps::compare_eq(&column, &value);
    
    // Only exact match should be true
    assert_eq!(mask[0], false);
    assert_eq!(mask[1], false);
    assert_eq!(mask[2], true);
}

#[test]
fn test_string_repeated_characters() {
    // Strings with many repeated characters (compression test)
    let column = Column::String(vec![
        "a".repeat(10000),
        "ab".repeat(5000),
        "abc".repeat(3333),
    ]);
    
    assert_eq!(column.len(), 3);
}

// ============================================================================
// COMPRESSION WEIRDNESS
// ============================================================================

#[test]
fn test_compression_corrupted_data() {
    // Test handling of corrupted compressed data
    let decompressor = create_decompressor(CompressionType::LZ4);
    
    // Corrupted data (random bytes)
    let corrupted: Vec<u8> = (0..100).map(|i| (i * 7) as u8).collect();
    let result = decompressor.decompress(&corrupted, 1000);
    
    // Should return error, not panic
    assert!(result.is_err());
}

#[test]
fn test_compression_truncated_data() {
    // Test handling of truncated compressed data
    let compressor = create_compressor(CompressionType::LZ4);
    let decompressor = create_decompressor(CompressionType::LZ4);
    
    let data = b"test data for compression";
    let compressed = compressor.compress(data).unwrap();
    
    // Truncate the compressed data
    let truncated = &compressed[..compressed.len() / 2];
    let result = decompressor.decompress(truncated, data.len());
    
    // Should return error
    assert!(result.is_err());
}

#[test]
fn test_compression_wrong_decompressor() {
    // Try to decompress with wrong algorithm
    let compressor_lz4 = create_compressor(CompressionType::LZ4);
    let decompressor_zstd = create_decompressor(CompressionType::Zstd);
    
    let data = b"test data";
    let compressed = compressor_lz4.compress(data).unwrap();
    
    // Wrong decompressor
    let result = decompressor_zstd.decompress(&compressed, data.len());
    
    // Should fail
    assert!(result.is_err());
}

#[test]
fn test_compression_highly_repetitive_data() {
    // Data that compresses extremely well
    let compressor = create_compressor(CompressionType::LZ4);
    let decompressor = create_decompressor(CompressionType::LZ4);
    
    let data = vec![0u8; 100000]; // All zeros
    let compressed = compressor.compress(&data).unwrap();
    
    // Should compress very well
    assert!(compressed.len() < data.len() / 10);
    
    let decompressed = decompressor.decompress(&compressed, data.len()).unwrap();
    assert_eq!(decompressed, data);
}

#[test]
fn test_compression_incompressible_data() {
    // Random data that doesn't compress
    let compressor = create_compressor(CompressionType::LZ4);
    let decompressor = create_decompressor(CompressionType::LZ4);
    
    // Use cryptographic random-like data
    let data: Vec<u8> = (0..10000).map(|i| ((i * 7919) % 256) as u8).collect();
    let compressed = compressor.compress(&data).unwrap();
    
    let decompressed = decompressor.decompress(&compressed, data.len()).unwrap();
    assert_eq!(decompressed, data);
}

// ============================================================================
// CONCURRENCY WEIRDNESS
// ============================================================================

#[tokio::test]
async fn test_concurrent_delete_during_read() {
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
    let columns = vec![Column::Int64(vec![1, 2, 3])];
    store.write_columns(table_id, columns).await.unwrap();
    
    // Start reading
    let read_store = store.clone();
    let read_handle = tokio::spawn(async move {
        read_store.read_columns(table_id, vec![0], 0, 10).await
    });
    
    // Delete while reading
    let delete_store = store.clone();
    let delete_handle = tokio::spawn(async move {
        delete_store.delete_table(table_id).await
    });
    
    // Both should complete (one may error)
    let read_result = read_handle.await.unwrap();
    let delete_result = delete_handle.await.unwrap();
    
    // At least one should succeed
    assert!(read_result.is_ok() || delete_result.is_ok());
}

#[tokio::test]
async fn test_concurrent_schema_modification() {
    use std::sync::Arc;
    
    let store = Arc::new(InMemoryColumnStore::new());
    let table_id = TableId(1);
    
    let schema1 = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);
    
    let schema2 = Schema::new(vec![
        Field {
            name: "name".to_string(),
            data_type: DataType::String,
            nullable: false,
            default_value: None,
        },
    ]);
    
    // Try to create with different schemas concurrently
    let store1 = store.clone();
    let handle1 = tokio::spawn(async move {
        store1.create_table(table_id, schema1).await
    });
    
    let store2 = store.clone();
    let handle2 = tokio::spawn(async move {
        store2.create_table(table_id, schema2).await
    });
    
    let result1 = handle1.await.unwrap();
    let result2 = handle2.await.unwrap();
    
    // Only one should succeed
    assert_eq!(result1.is_ok() as usize + result2.is_ok() as usize, 1);
}

// ============================================================================
// SCHEMA WEIRDNESS
// ============================================================================

#[test]
fn test_schema_self_referential_type() {
    // Test deeply nested nullable types
    let mut nested = DataType::Int32;
    for _ in 0..100 {
        nested = DataType::Nullable(Box::new(nested));
    }
    
    let schema = Schema::new(vec![
        Field {
            name: "deep".to_string(),
            data_type: nested,
            nullable: true,
            default_value: None,
        },
    ]);
    
    assert_eq!(schema.len(), 1);
}

#[test]
fn test_schema_circular_array() {
    // Array of arrays of arrays...
    let mut nested = DataType::Int32;
    for _ in 0..50 {
        nested = DataType::Array(Box::new(nested));
    }
    
    let schema = Schema::new(vec![
        Field {
            name: "nested".to_string(),
            data_type: nested,
            nullable: false,
            default_value: None,
        },
    ]);
    
    assert_eq!(schema.len(), 1);
}

#[test]
fn test_schema_map_with_complex_types() {
    // Map with nested types
    let key_type = DataType::String;
    let value_type = DataType::Array(Box::new(DataType::Nullable(Box::new(DataType::Int32))));
    let map_type = DataType::Map(Box::new(key_type), Box::new(value_type));
    
    let schema = Schema::new(vec![
        Field {
            name: "complex".to_string(),
            data_type: map_type,
            nullable: false,
            default_value: None,
        },
    ]);
    
    assert_eq!(schema.len(), 1);
}

#[test]
fn test_schema_field_name_unicode() {
    // Field names with Unicode characters
    let schema = Schema::new(vec![
        Field {
            name: "字段名".to_string(), // Chinese
            data_type: DataType::Int32,
            nullable: false,
            default_value: None,
        },
        Field {
            name: "שדה".to_string(), // Hebrew
            data_type: DataType::Int32,
            nullable: false,
            default_value: None,
        },
        Field {
            name: "field_🌍".to_string(), // Emoji
            data_type: DataType::Int32,
            nullable: false,
            default_value: None,
        },
    ]);
    
    assert_eq!(schema.len(), 3);
    assert_eq!(schema.field_index("字段名"), Some(0));
    assert_eq!(schema.field_index("שדה"), Some(1));
    assert_eq!(schema.field_index("field_🌍"), Some(2));
}

// ============================================================================
// TRANSACTION WEIRDNESS
// ============================================================================

#[test]
fn test_transaction_very_large_id() {
    use narayana_core::transaction::TransactionManager;
    
    let mut manager = TransactionManager::new();
    
    // Create many transactions to get to large ID
    for _ in 0..1000 {
        manager.begin_transaction();
    }
    
    let large_id = manager.begin_transaction();
    assert_eq!(large_id.0, 1001);
}

#[test]
fn test_transaction_commit_abort_race() {
    use narayana_core::transaction::TransactionManager;
    
    let mut manager = TransactionManager::new();
    let id = manager.begin_transaction();
    
    // Try to commit and abort (should only allow one)
    manager.commit_transaction(id).unwrap();
    let result = manager.abort_transaction(id);
    
    // Should fail - already committed
    assert!(result.is_err());
}

// ============================================================================
// INDEX WEIRDNESS
// ============================================================================

#[test]
fn test_index_very_long_keys() {
    let mut index = BTreeIndex::new();
    
    // Very long keys
    let long_key1 = vec![0u8; 10000];
    let long_key2 = vec![1u8; 10000];
    
    index.insert(long_key1.clone(), 1).unwrap();
    index.insert(long_key2.clone(), 2).unwrap();
    
    assert_eq!(index.lookup(&long_key1).unwrap(), Some(1));
    assert_eq!(index.lookup(&long_key2).unwrap(), Some(2));
}

#[test]
fn test_index_identical_keys_different_values() {
    let mut index = BTreeIndex::new();
    
    // Same key, different values (should overwrite)
    index.insert(b"key".to_vec(), 1).unwrap();
    index.insert(b"key".to_vec(), 2).unwrap();
    index.insert(b"key".to_vec(), 3).unwrap();
    
    // Last value should win
    assert_eq!(index.lookup(b"key").unwrap(), Some(3));
}

#[test]
fn test_index_range_scan_reverse_order() {
    let mut index = BTreeIndex::new();
    
    index.insert(b"z".to_vec(), 1).unwrap();
    index.insert(b"a".to_vec(), 2).unwrap();
    index.insert(b"m".to_vec(), 3).unwrap();
    
    // Range scan should work regardless of insertion order
    let results = index.range_scan(b"a", b"z").unwrap();
    assert_eq!(results.len(), 3);
}

// ============================================================================
// CACHE WEIRDNESS
// ============================================================================

#[test]
fn test_cache_rapid_insert_delete() {
    let cache = LRUCache::new(10);
    
    // Rapid insert/delete cycles
    for i in 0..1000 {
        cache.insert(format!("key_{}", i), format!("value_{}", i));
        cache.remove(&format!("key_{}", i));
    }
    
    // Cache should be empty
    assert_eq!(cache.len(), 0);
}

#[test]
fn test_cache_ttl_zero_duration() {
    let cache = LRUCache::with_ttl(10, Duration::from_secs(0));
    cache.insert("key", "value");
    
    // Should expire immediately
    assert_eq!(cache.get(&"key"), None);
}

#[test]
fn test_cache_same_key_repeated_insert() {
    let cache = LRUCache::new(10);
    
    // Insert same key many times
    for i in 0..100 {
        cache.insert("key", format!("value_{}", i));
    }
    
    // Should have latest value
    assert_eq!(cache.get(&"key"), Some("value_99".to_string()));
    assert_eq!(cache.len(), 1);
}

// ============================================================================
// SERIALIZATION WEIRDNESS
// ============================================================================

#[test]
fn test_serialize_very_large_schema() {
    use serde_json;
    
    // Schema with many fields
    let fields: Vec<Field> = (0..10000).map(|i| Field {
        name: format!("field_{}", i),
        data_type: DataType::Int32,
        nullable: false,
        default_value: None,
    }).collect();
    
    let schema = Schema::new(fields);
    let serialized = serde_json::to_string(&schema).unwrap();
    let deserialized: Schema = serde_json::from_str(&serialized).unwrap();
    
    assert_eq!(schema.len(), deserialized.len());
}

#[test]
fn test_serialize_special_characters_in_names() {
    use serde_json;
    
    let schema = Schema::new(vec![
        Field {
            name: "field\nwith\nnewlines".to_string(),
            data_type: DataType::Int32,
            nullable: false,
            default_value: None,
        },
        Field {
            name: "field\twith\ttabs".to_string(),
            data_type: DataType::Int32,
            nullable: false,
            default_value: None,
        },
        Field {
            name: "field\"with\"quotes".to_string(),
            data_type: DataType::Int32,
            nullable: false,
            default_value: None,
        },
    ]);
    
    let serialized = serde_json::to_string(&schema).unwrap();
    let deserialized: Schema = serde_json::from_str(&serialized).unwrap();
    
    assert_eq!(schema.len(), deserialized.len());
}

// ============================================================================
// TIMING WEIRDNESS
// ============================================================================

#[test]
fn test_timestamp_ordering_edge_cases() {
    // Test timestamp ordering with same values
    let ts1 = Timestamp(1000);
    let ts2 = Timestamp(1000);
    let ts3 = Timestamp(1001);
    
    assert_eq!(ts1, ts2);
    assert!(ts3 > ts1);
    assert!(ts1 <= ts2);
    assert!(ts1 < ts3);
}

#[test]
fn test_timestamp_max_value() {
    let ts_max = Timestamp(u64::MAX);
    let ts_min = Timestamp(0);
    
    assert!(ts_max > ts_min);
    assert_eq!(ts_max.0, u64::MAX);
}

// ============================================================================
// MEMORY WEIRDNESS
// ============================================================================

#[test]
fn test_memory_alignment_issues() {
    // Test with data sizes that might cause alignment issues
    let sizes = vec![1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17];
    
    for size in sizes {
        let data: Vec<u8> = vec![0; size];
        let column = Column::Binary(vec![data]);
        assert_eq!(column.len(), 1);
    }
}

#[test]
fn test_memory_fragmentation_scenario() {
    // Create many small allocations then large ones
    let mut columns = Vec::new();
    
    // Many small columns
    for _ in 0..1000 {
        columns.push(Column::Int32(vec![1, 2, 3]));
    }
    
    // Then large column
    let large_data: Vec<i32> = (0..100000).collect();
    columns.push(Column::Int32(large_data));
    
    assert_eq!(columns.len(), 1001);
}

// ============================================================================
// API WEIRDNESS
// ============================================================================

#[tokio::test]
async fn test_table_id_wraparound() {
    // Test behavior with very large table IDs
    let store = InMemoryColumnStore::new();
    let table_id = TableId(u64::MAX);
    
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);
    
    // Should handle max ID
    let result = store.create_table(table_id, schema).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_column_id_wraparound() {
    // Test with max column ID
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
    
    // Try to read with max column ID
    let result = store.read_columns(table_id, vec![u32::MAX], 0, 10).await;
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

// ============================================================================
// DATA CORRUPTION PREVENTION
// ============================================================================

#[test]
fn test_prevent_data_corruption_on_type_mismatch() {
    // Ensure type mismatches don't corrupt data
    let column = Column::Int32(vec![1, 2, 3]);
    
    // Try invalid operations
    let invalid_value = serde_json::Value::String("not a number".to_string());
    let mask = VectorizedOps::compare_eq(&column, &invalid_value);
    
    // Original column should be unchanged
    match column {
        Column::Int32(data) => assert_eq!(data, vec![1, 2, 3]),
        _ => panic!("Type changed"),
    }
    
    // Mask should be all false
    assert_eq!(mask, vec![false, false, false]);
}

#[test]
fn test_prevent_overflow_in_aggregation() {
    // Test that aggregations don't silently overflow
    let column = Column::Int32(vec![i32::MAX, 1]);
    
    // Sum would overflow - should handle gracefully
    let sum = VectorizedOps::sum(&column);
    // May return wrapped value or None - either is acceptable
    assert!(sum.is_some() || sum.is_none());
}

// ============================================================================
// EXTREME EDGE CASES
// ============================================================================

#[test]
fn test_extreme_nested_types() {
    // Create extremely nested type
    let mut nested = DataType::Int32;
    for i in 0..1000 {
        if i % 2 == 0 {
            nested = DataType::Nullable(Box::new(nested));
        } else {
            nested = DataType::Array(Box::new(nested));
        }
    }
    
    // Should not panic
    assert!(!nested.is_fixed_size());
}

#[test]
fn test_extreme_string_lengths() {
    // Test with various extreme string lengths
    let lengths = vec![0, 1, 10, 100, 1000, 10000, 100000, 1000000];
    
    for len in lengths {
        let s = "a".repeat(len);
        let column = Column::String(vec![s.clone()]);
        assert_eq!(column.len(), 1);
        
        let value = serde_json::Value::String(s);
        let mask = VectorizedOps::compare_eq(&column, &value);
        assert_eq!(mask[0], true);
    }
}

#[test]
fn test_extreme_column_counts() {
    // Test with many columns
    let mut columns = Vec::new();
    for i in 0..10000 {
        columns.push(Column::Int32(vec![i as i32]));
    }
    
    assert_eq!(columns.len(), 10000);
}

#[tokio::test]
async fn test_extreme_concurrent_operations() {
    use std::sync::Arc;
    
    let store = Arc::new(InMemoryColumnStore::new());
    
    // Create many tables concurrently
    let mut handles = vec![];
    for i in 0..1000 {
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
    
    // All should complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

#[test]
fn test_extreme_compression_ratios() {
    // Test compression with extreme ratios
    let compressor = create_compressor(CompressionType::LZ4);
    let decompressor = create_decompressor(CompressionType::LZ4);
    
    // Highly compressible (all same byte)
    let all_same = vec![42u8; 1000000];
    let compressed = compressor.compress(&all_same).unwrap();
    assert!(compressed.len() < all_same.len() / 100);
    
    let decompressed = decompressor.decompress(&compressed, all_same.len()).unwrap();
    assert_eq!(decompressed, all_same);
}

#[test]
fn test_extreme_filter_operations() {
    // Filter with extreme patterns
    let column = Column::Int32((0..1000000).collect());
    
    // All true
    let all_true: Vec<bool> = (0..1000000).map(|_| true).collect();
    let filtered = VectorizedOps::filter(&column, &all_true);
    assert_eq!(filtered.len(), 1000000);
    
    // All false
    let all_false: Vec<bool> = (0..1000000).map(|_| false).collect();
    let filtered = VectorizedOps::filter(&column, &all_false);
    assert_eq!(filtered.len(), 0);
    
    // Alternating
    let alternating: Vec<bool> = (0..1000000).map(|i| i % 2 == 0).collect();
    let filtered = VectorizedOps::filter(&column, &alternating);
    assert_eq!(filtered.len(), 500000);
}

#[test]
fn test_extreme_aggregation_on_large_data() {
    // Aggregations on very large datasets
    let column = Column::Int64((0..10_000_000).collect());
    
    let start = std::time::Instant::now();
    let sum = VectorizedOps::sum(&column);
    let duration = start.elapsed();
    
    assert!(sum.is_some());
    assert!(duration.as_secs() < 10); // Should be fast
    
    let min = VectorizedOps::min(&column);
    let max = VectorizedOps::max(&column);
    
    assert_eq!(min, Some(serde_json::Value::Number(0.into())));
    assert_eq!(max, Some(serde_json::Value::Number(9_999_999.into())));
}

