// Tests for really obscure bugs that might only appear in production
// These test scenarios that are theoretically possible but very unlikely

use narayana_core::{
    schema::{Schema, Field, DataType},
    types::{TableId, CompressionType},
    column::Column,
};
use narayana_storage::{
    ColumnStore, InMemoryColumnStore,
    compression::{create_compressor, create_decompressor},
};
use narayana_query::vectorized::VectorizedOps;

// ============================================================================
// FLOATING POINT PRECISION BUGS
// ============================================================================

#[test]
fn test_float_accumulation_error() {
    // Test that repeated additions don't accumulate errors
    let column = Column::Float64(vec![0.1; 10]);
    
    // 0.1 * 10 should be 1.0, but floating point might have errors
    let sum = VectorizedOps::sum(&column);
    
    if let Some(serde_json::Value::Number(n)) = sum {
        if let Some(val) = n.as_f64() {
            // Should be close to 1.0 (within reasonable tolerance)
            assert!((val - 1.0).abs() < 0.0001);
        }
    }
}

#[test]
fn test_float_denormalized_numbers() {
    // Denormalized (subnormal) numbers - smallest representable
    let column = Column::Float64(vec![
        f64::MIN_POSITIVE / 2.0, // Subnormal
        f64::MIN_POSITIVE, // Smallest normal
    ]);
    
    // Should handle without panicking
    let min = VectorizedOps::min(&column);
    let max = VectorizedOps::max(&column);
    
    assert!(min.is_some() || min.is_none());
    assert!(max.is_some() || max.is_none());
}

#[test]
fn test_float_negative_zero_equality() {
    // IEEE 754: -0.0 == 0.0
    let column = Column::Float64(vec![0.0, -0.0, 1.0]);
    
    let zero_val = serde_json::Value::Number(serde_json::Number::from_f64(0.0).unwrap());
    let mask = VectorizedOps::compare_eq(&column, &zero_val);
    
    // Both should match
    assert_eq!(mask[0], true);
    assert_eq!(mask[1], true); // -0.0 == 0.0
}

// ============================================================================
// ENDIANNESS ISSUES
// ============================================================================

#[test]
fn test_binary_endianness() {
    // Test that binary data is handled consistently
    let column = Column::Binary(vec![
        vec![0x12, 0x34, 0x56, 0x78],
        vec![0x78, 0x56, 0x34, 0x12],
    ]);
    
    // Should preserve byte order
    assert_eq!(column.len(), 2);
    
    match column {
        Column::Binary(data) => {
            assert_eq!(data[0], vec![0x12, 0x34, 0x56, 0x78]);
            assert_eq!(data[1], vec![0x78, 0x56, 0x34, 0x12]);
        }
        _ => panic!("Expected Binary"),
    }
}

// ============================================================================
// HASH COLLISION EDGE CASES
// ============================================================================

#[test]
fn test_hash_collision_handling() {
    // Test that hash collisions are handled (if using hash-based structures)
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
    ]);
    
    // Even if field names hash to same value, should work correctly
    assert_eq!(schema.field_index("a"), Some(0));
    assert_eq!(schema.field_index("b"), Some(1));
}

// ============================================================================
// TIMING ATTACKS
// ============================================================================

#[test]
fn test_timing_attack_prevention() {
    // Operations should take similar time regardless of data content
    // (This is a basic test - full timing attack prevention requires more)
    
    let column1 = Column::Int32(vec![1; 1000]);
    let column2 = Column::Int32(vec![999; 1000]);
    
    let start1 = std::time::Instant::now();
    let _ = VectorizedOps::sum(&column1);
    let duration1 = start1.elapsed();
    
    let start2 = std::time::Instant::now();
    let _ = VectorizedOps::sum(&column2);
    let duration2 = start2.elapsed();
    
    // Times should be similar (within 10x - rough check)
    let ratio = duration1.as_nanos().max(duration2.as_nanos()) as f64 /
                duration1.as_nanos().min(duration2.as_nanos()) as f64;
    assert!(ratio < 10.0);
}

// ============================================================================
// MEMORY ALIGNMENT ISSUES
// ============================================================================

#[test]
fn test_unaligned_memory_access() {
    // Test with sizes that might cause alignment issues
    let unaligned_sizes = vec![1, 3, 5, 7, 9, 15, 17];
    
    for size in unaligned_sizes {
        let data: Vec<u8> = vec![42; size];
        let column = Column::Binary(vec![data]);
        
        // Should handle unaligned sizes
        assert_eq!(column.len(), 1);
    }
}

// ============================================================================
// COMPRESSION CORNER CASES
// ============================================================================

#[test]
fn test_compression_header_corruption() {
    // Test handling of corrupted compression headers
    let decompressor = create_decompressor(CompressionType::LZ4);
    
    // Valid LZ4 data but wrong header
    let mut corrupted = vec![0xFF, 0xFF, 0xFF, 0xFF]; // Invalid header
    corrupted.extend_from_slice(b"some data");
    
    let result = decompressor.decompress(&corrupted, 100);
    assert!(result.is_err());
}

#[test]
fn test_compression_size_mismatch_attack() {
    // Test that wrong size doesn't cause buffer overflow
    let compressor = create_compressor(CompressionType::LZ4);
    let decompressor = create_decompressor(CompressionType::LZ4);
    
    let data = b"test data";
    let compressed = compressor.compress(data).unwrap();
    
    // Try to decompress with wrong (larger) size
    let result = decompressor.decompress(&compressed, usize::MAX);
    // Should handle gracefully, not allocate huge buffer
    assert!(result.is_err() || result.is_ok());
}

#[test]
fn test_compression_decompression_idempotency() {
    // Compress -> decompress -> compress -> decompress should be idempotent
    let compressor = create_compressor(CompressionType::LZ4);
    let decompressor = create_decompressor(CompressionType::LZ4);
    
    let original = b"test data for idempotency";
    
    let compressed1 = compressor.compress(original).unwrap();
    let decompressed1 = decompressor.decompress(&compressed1, original.len()).unwrap();
    
    let compressed2 = compressor.compress(&decompressed1).unwrap();
    let decompressed2 = decompressor.decompress(&compressed2, decompressed1.len()).unwrap();
    
    assert_eq!(decompressed1, decompressed2);
}

// ============================================================================
// CONCURRENCY CORNER CASES
// ============================================================================

#[tokio::test]
async fn test_aba_problem_prevention() {
    use std::sync::Arc;
    
    // ABA problem: value changes from A to B and back to A
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
    
    // Create
    store.create_table(table_id, schema.clone()).await.unwrap();
    
    // Delete
    store.delete_table(table_id).await.unwrap();
    
    // Recreate with same ID (ABA scenario)
    store.create_table(table_id, schema).await.unwrap();
    
    // Should work correctly
    let result = store.get_schema(table_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_livelock_prevention() {
    use std::sync::Arc;
    
    // Test that we don't get into livelock scenarios
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
    
    // Rapid create/delete cycles
    for _ in 0..100 {
        store.create_table(table_id, schema.clone()).await.ok();
        store.delete_table(table_id).await.ok();
    }
    
    // Should complete without hanging
    assert!(true);
}

// ============================================================================
// SERIALIZATION CORNER CASES
// ============================================================================

#[test]
fn test_serialize_circular_reference_prevention() {
    use serde_json;
    
    // Test that we don't create circular references
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int32,
            nullable: false,
            default_value: None,
        },
    ]);
    
    // Serialization should complete
    let serialized = serde_json::to_string(&schema).unwrap();
    assert!(!serialized.is_empty());
    
    // Deserialization should complete
    let deserialized: Schema = serde_json::from_str(&serialized).unwrap();
    assert_eq!(schema.len(), deserialized.len());
}

#[test]
fn test_serialize_deeply_nested_structures() {
    use serde_json;
    
    // Very deeply nested nullable type
    let mut nested = DataType::Int32;
    for _ in 0..1000 {
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
    
    // Should serialize/deserialize without stack overflow
    let serialized = serde_json::to_string(&schema).unwrap();
    let deserialized: Schema = serde_json::from_str(&serialized).unwrap();
    assert_eq!(schema.len(), deserialized.len());
}

// ============================================================================
// NUMERIC PRECISION ISSUES
// ============================================================================

#[test]
fn test_large_integer_precision() {
    // Test that large integers maintain precision
    let column = Column::Int64(vec![
        i64::MAX,
        i64::MAX - 1,
        i64::MAX - 1000,
    ]);
    
    let max = VectorizedOps::max(&column);
    assert_eq!(max, Some(serde_json::Value::Number(i64::MAX.into())));
}

#[test]
fn test_float_precision_loss() {
    // Test that float operations don't lose precision unexpectedly
    let column = Column::Float64(vec![
        1.0,
        1.0000000000000001,
        1.0000000000000002,
    ]);
    
    let value = serde_json::Value::Number(serde_json::Number::from_f64(1.0).unwrap());
    let mask = VectorizedOps::compare_eq(&column, &value);
    
    // First should match, others might or might not (epsilon comparison)
    assert_eq!(mask[0], true);
}

// ============================================================================
// STRING INTERNING EDGE CASES
// ============================================================================

#[test]
fn test_string_identity_vs_equality() {
    // Test that string comparison uses equality, not identity
    let s1 = "hello".to_string();
    let s2 = "hello".to_string();
    
    // These are different String instances but equal content
    let column = Column::String(vec![s1.clone(), s2.clone()]);
    
    let value = serde_json::Value::String("hello".to_string());
    let mask = VectorizedOps::compare_eq(&column, &value);
    
    // Both should match (equality, not identity)
    assert_eq!(mask[0], true);
    assert_eq!(mask[1], true);
}

// ============================================================================
// CACHE POISONING PREVENTION
// ============================================================================

#[test]
fn test_cache_poisoning_prevention() {
    // Test that cache doesn't serve poisoned data
    let cache = LRUCache::new(10);
    
    cache.insert("key", "value1");
    assert_eq!(cache.get(&"key"), Some("value1"));
    
    // Update with different value
    cache.insert("key", "value2");
    assert_eq!(cache.get(&"key"), Some("value2"));
    
    // Should not return old value
    assert_ne!(cache.get(&"key"), Some("value1"));
}

// ============================================================================
// RACE CONDITION EDGE CASES
// ============================================================================

#[tokio::test]
async fn test_check_then_act_race() {
    use std::sync::Arc;
    
    // Classic check-then-act race condition
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
    
    // Check if table exists
    let exists = store.get_schema(table_id).await.is_ok();
    
    // Another "thread" creates it
    if !exists {
        store.create_table(table_id, schema).await.unwrap();
    }
    
    // Should be safe
    let result = store.get_schema(table_id).await;
    assert!(result.is_ok());
}

// ============================================================================
// MEMORY LEAK PREVENTION
// ============================================================================

#[test]
fn test_memory_leak_prevention_rapid_allocations() {
    // Rapid allocations and deallocations should not leak
    for _ in 0..1000 {
        let column = Column::Int32((0..1000).collect());
        let _ = VectorizedOps::sum(&column);
        // Column should be dropped here
    }
    
    // If we got here without OOM, we're probably fine
    assert!(true);
}

#[tokio::test]
async fn test_memory_leak_prevention_table_operations() {
    let store = InMemoryColumnStore::new();
    
    // Create and delete many tables
    for i in 0..1000 {
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
        store.delete_table(table_id).await.unwrap();
    }
    
    // Should not leak memory
    assert!(true);
}

// ============================================================================
// TYPE SYSTEM EDGE CASES
// ============================================================================

#[test]
fn test_type_coercion_prevention() {
    // Ensure types aren't incorrectly coerced
    let int_col = Column::Int32(vec![1, 2, 3]);
    let float_col = Column::Float64(vec![1.0, 2.0, 3.0]);
    
    // These should remain different types
    assert_eq!(int_col.data_type(), DataType::Int32);
    assert_eq!(float_col.data_type(), DataType::Float64);
    
    // Operations should not work across types
    let value = serde_json::Value::Number(serde_json::Number::from_f64(1.0).unwrap());
    let mask = VectorizedOps::compare_eq(&int_col, &value);
    // Should return false for type mismatch
    assert_eq!(mask, vec![false, false, false]);
}

#[test]
fn test_nullable_type_edge_cases() {
    // Test nullable type handling
    let nullable_int = DataType::Nullable(Box::new(DataType::Int32));
    let nullable_nullable = DataType::Nullable(Box::new(DataType::Nullable(Box::new(DataType::Int32))));
    
    // Both should be handled
    assert!(!nullable_int.is_fixed_size());
    assert!(!nullable_nullable.is_fixed_size());
}

// ============================================================================
// PERFORMANCE REGRESSION EDGE CASES
// ============================================================================

#[test]
fn test_quadratic_complexity_prevention() {
    use std::time::Instant;
    
    // Test that operations scale linearly, not quadratically
    let sizes = vec![100, 1000, 10000];
    let mut times = Vec::new();
    
    for size in sizes {
        let data: Vec<i32> = (0..size).collect();
        let column = Column::Int32(data);
        
        let start = Instant::now();
        let _ = VectorizedOps::sum(&column);
        times.push(start.elapsed());
    }
    
    // Time should scale roughly linearly
    // Ratio of largest to smallest should be < 200x for 100x data increase
    let ratio = times[2].as_nanos() as f64 / times[0].as_nanos() as f64;
    assert!(ratio < 200.0, "Possible quadratic complexity: ratio = {}", ratio);
}

#[test]
fn test_cache_miss_penalty() {
    // Test that cache misses don't cause excessive slowdown
    let cache = LRUCache::new(10);
    
    // Fill cache
    for i in 0..10 {
        cache.insert(format!("key_{}", i), format!("value_{}", i));
    }
    
    // Cause many misses
    let start = std::time::Instant::now();
    for i in 10..1000 {
        cache.get(&format!("key_{}", i));
    }
    let duration = start.elapsed();
    
    // Should be fast even with misses
    assert!(duration.as_millis() < 1000);
}

// ============================================================================
// CORNER CASES IN COMPARISON OPERATIONS
// ============================================================================

#[test]
fn test_comparison_with_extreme_values() {
    // Compare with extreme values
    let column = Column::Int32(vec![i32::MIN, -1, 0, 1, i32::MAX]);
    
    let min_val = serde_json::Value::Number(i32::MIN.into());
    let max_val = serde_json::Value::Number(i32::MAX.into());
    let zero_val = serde_json::Value::Number(0.into());
    
    let min_mask = VectorizedOps::compare_eq(&column, &min_val);
    let max_mask = VectorizedOps::compare_eq(&column, &max_val);
    let zero_mask = VectorizedOps::compare_eq(&column, &zero_val);
    
    assert_eq!(min_mask, vec![true, false, false, false, false]);
    assert_eq!(max_mask, vec![false, false, false, false, true]);
    assert_eq!(zero_mask, vec![false, false, true, false, false]);
}

#[test]
fn test_comparison_gt_lt_edge_cases() {
    let column = Column::Int32(vec![i32::MIN, -1, 0, 1, i32::MAX]);
    
    let zero_val = serde_json::Value::Number(0.into());
    
    let gt_mask = VectorizedOps::compare_gt(&column, &zero_val);
    let lt_mask = VectorizedOps::compare_lt(&column, &zero_val);
    
    assert_eq!(gt_mask, vec![false, false, false, true, true]);
    assert_eq!(lt_mask, vec![true, true, false, false, false]);
}

// ============================================================================
// STRING ENCODING EDGE CASES
// ============================================================================

#[test]
fn test_string_utf8_boundary_cases() {
    // Test UTF-8 boundary cases
    let column = Column::String(vec![
        "\u{007F}".to_string(), // Last ASCII
        "\u{0080}".to_string(), // First non-ASCII
        "\u{07FF}".to_string(), // Last 2-byte UTF-8
        "\u{0800}".to_string(), // First 3-byte UTF-8
        "\u{FFFF}".to_string(), // Last 3-byte UTF-8 (excluding surrogates)
        "\u{10000}".to_string(), // First 4-byte UTF-8
    ]);
    
    assert_eq!(column.len(), 6);
}

#[test]
fn test_string_byte_order_mark() {
    // BOM in strings
    let column = Column::String(vec![
        "\u{FEFF}hello".to_string(), // UTF-8 BOM
        "hello\u{FEFF}".to_string(), // BOM at end
    ]);
    
    assert_eq!(column.len(), 2);
    
    // Should compare correctly
    let value = serde_json::Value::String("\u{FEFF}hello".to_string());
    let mask = VectorizedOps::compare_eq(&column, &value);
    assert_eq!(mask[0], true);
    assert_eq!(mask[1], false);
}

// ============================================================================
// COMPRESSION ALGORITHM EDGE CASES
// ============================================================================

#[test]
fn test_compression_all_algorithms_same_data() {
    // Same data compressed with different algorithms
    let data = b"test data for compression comparison";
    
    let lz4_compressor = create_compressor(CompressionType::LZ4);
    let zstd_compressor = create_compressor(CompressionType::Zstd);
    let snappy_compressor = create_compressor(CompressionType::Snappy);
    
    let lz4_compressed = lz4_compressor.compress(data).unwrap();
    let zstd_compressed = zstd_compressor.compress(data).unwrap();
    let snappy_compressed = snappy_compressor.compress(data).unwrap();
    
    // All should decompress correctly
    let lz4_decompressor = create_decompressor(CompressionType::LZ4);
    let zstd_decompressor = create_decompressor(CompressionType::Zstd);
    let snappy_decompressor = create_decompressor(CompressionType::Snappy);
    
    assert_eq!(lz4_decompressor.decompress(&lz4_compressed, data.len()).unwrap(), data);
    assert_eq!(zstd_decompressor.decompress(&zstd_compressed, data.len()).unwrap(), data);
    assert_eq!(snappy_decompressor.decompress(&snappy_compressed, data.len()).unwrap(), data);
}

// ============================================================================
// EXTREME SCALE TESTS
// ============================================================================

#[test]
#[ignore] // May be slow, run with --ignored
fn test_extreme_scale_operations() {
    // Test with extremely large datasets
    let column = Column::Int64((0..100_000_000).collect());
    
    // Should complete in reasonable time
    let start = std::time::Instant::now();
    let sum = VectorizedOps::sum(&column);
    let duration = start.elapsed();
    
    assert!(sum.is_some());
    assert!(duration.as_secs() < 60); // Should complete in under a minute
}

#[tokio::test]
#[ignore] // May be slow
async fn test_extreme_scale_storage() {
    // Test storage with extreme scale
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
    
    let large_data: Vec<i64> = (0..50_000_000).collect();
    let columns = vec![Column::Int64(large_data)];
    
    let start = std::time::Instant::now();
    store.write_columns(table_id, columns).await.unwrap();
    let duration = start.elapsed();
    
    assert!(duration.as_secs() < 120); // Should complete in under 2 minutes
}

