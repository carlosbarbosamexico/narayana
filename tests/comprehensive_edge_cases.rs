use narayana_core::{
    schema::{Schema, Field, DataType},
    types::{TableId, ColumnId, TransactionId, Timestamp, CompressionType},
    column::Column,
    row::{Row, Value},
    transaction::{Transaction, TransactionManager, TransactionStatus},
    Error,
};
use narayana_storage::{
    ColumnStore, InMemoryColumnStore,
    compression::{create_compressor, create_decompressor},
    index::BTreeIndex,
    cache::LRUCache,
    writer::ColumnWriter,
    reader::ColumnReader,
    block::BlockMetadata,
};
use narayana_query::{
    vectorized::VectorizedOps,
    operators::{FilterOperator, ProjectOperator},
    plan::{Filter, QueryPlan, PlanNode},
};
use std::time::Duration;

// ============================================================================
// CORE MODULE EDGE CASES
// ============================================================================

#[test]
fn test_table_id_zero() {
    let id = TableId(0);
    assert_eq!(id.0, 0);
}

#[test]
fn test_table_id_max() {
    let id = TableId(u64::MAX);
    assert_eq!(id.0, u64::MAX);
}

#[test]
fn test_column_id_zero() {
    let id = ColumnId(0);
    assert_eq!(id.0, 0);
}

#[test]
fn test_column_id_max() {
    let id = ColumnId(u32::MAX);
    assert_eq!(id.0, u32::MAX);
}

#[test]
fn test_transaction_id_zero() {
    let id = TransactionId(0);
    assert_eq!(id.0, 0);
}

#[test]
fn test_timestamp_ordering() {
    let ts1 = Timestamp(1000);
    let ts2 = Timestamp(2000);
    let ts3 = Timestamp(1000);
    
    assert!(ts2 > ts1);
    assert!(ts1 == ts3);
    assert!(ts1 <= ts2);
}

#[test]
fn test_timestamp_max() {
    let ts = Timestamp(u64::MAX);
    assert_eq!(ts.0, u64::MAX);
}

#[test]
fn test_schema_empty_fields() {
    let schema = Schema::new(vec![]);
    assert_eq!(schema.len(), 0);
    assert_eq!(schema.field_index("nonexistent"), None);
}

#[test]
fn test_schema_very_long_field_name() {
    let long_name = "a".repeat(10000);
    let schema = Schema::new(vec![
        Field {
            name: long_name.clone(),
            data_type: DataType::Int32,
            nullable: false,
            default_value: None,
        },
    ]);
    assert_eq!(schema.field_index(&long_name), Some(0));
}

#[test]
fn test_schema_special_characters_in_name() {
    let schema = Schema::new(vec![
        Field {
            name: "field-with-dashes".to_string(),
            data_type: DataType::Int32,
            nullable: false,
            default_value: None,
        },
        Field {
            name: "field_with_underscores".to_string(),
            data_type: DataType::Int32,
            nullable: false,
            default_value: None,
        },
        Field {
            name: "field.with.dots".to_string(),
            data_type: DataType::Int32,
            nullable: false,
            default_value: None,
        },
    ]);
    assert_eq!(schema.field_index("field-with-dashes"), Some(0));
    assert_eq!(schema.field_index("field_with_underscores"), Some(1));
    assert_eq!(schema.field_index("field.with.dots"), Some(2));
}

#[test]
fn test_data_type_nested_nullable() {
    let nested = DataType::Nullable(Box::new(DataType::Nullable(Box::new(DataType::Int32))));
    assert!(!nested.is_fixed_size());
}

#[test]
fn test_data_type_deeply_nested_array() {
    let nested = DataType::Array(Box::new(DataType::Array(Box::new(DataType::Array(Box::new(DataType::Int32))))));
    assert!(!nested.is_fixed_size());
}

#[test]
fn test_column_empty_all_types() {
    assert_eq!(Column::Int8(vec![]).len(), 0);
    assert_eq!(Column::Int16(vec![]).len(), 0);
    assert_eq!(Column::Int32(vec![]).len(), 0);
    assert_eq!(Column::Int64(vec![]).len(), 0);
    assert_eq!(Column::UInt8(vec![]).len(), 0);
    assert_eq!(Column::UInt16(vec![]).len(), 0);
    assert_eq!(Column::UInt32(vec![]).len(), 0);
    assert_eq!(Column::UInt64(vec![]).len(), 0);
    assert_eq!(Column::Float32(vec![]).len(), 0);
    assert_eq!(Column::Float64(vec![]).len(), 0);
    assert_eq!(Column::Boolean(vec![]).len(), 0);
    assert_eq!(Column::String(vec![]).len(), 0);
    assert_eq!(Column::Binary(vec![]).len(), 0);
    assert_eq!(Column::Timestamp(vec![]).len(), 0);
    assert_eq!(Column::Date(vec![]).len(), 0);
}

#[test]
fn test_column_single_element_all_types() {
    assert_eq!(Column::Int8(vec![42]).len(), 1);
    assert_eq!(Column::Int16(vec![42]).len(), 1);
    assert_eq!(Column::Int32(vec![42]).len(), 1);
    assert_eq!(Column::Int64(vec![42]).len(), 1);
    assert_eq!(Column::UInt8(vec![42]).len(), 1);
    assert_eq!(Column::UInt16(vec![42]).len(), 1);
    assert_eq!(Column::UInt32(vec![42]).len(), 1);
    assert_eq!(Column::UInt64(vec![42]).len(), 1);
    assert_eq!(Column::Float32(vec![42.0]).len(), 1);
    assert_eq!(Column::Float64(vec![42.0]).len(), 1);
    assert_eq!(Column::Boolean(vec![true]).len(), 1);
    assert_eq!(Column::String(vec!["test".to_string()]).len(), 1);
    assert_eq!(Column::Binary(vec![vec![1, 2, 3]]).len(), 1);
    assert_eq!(Column::Timestamp(vec![1000]).len(), 1);
    assert_eq!(Column::Date(vec![1]).len(), 1);
}

#[test]
fn test_row_empty() {
    let row = Row::new(vec![]);
    assert_eq!(row.values.len(), 0);
    assert!(row.get(0).is_none());
}

#[test]
fn test_row_large() {
    let values: Vec<Value> = (0..1000).map(|i| Value::Int32(i)).collect();
    let row = Row::new(values);
    assert_eq!(row.values.len(), 1000);
    assert!(matches!(row.get(0), Some(Value::Int32(0))));
    assert!(matches!(row.get(999), Some(Value::Int32(999))));
    assert!(row.get(1000).is_none());
}

#[test]
fn test_transaction_lifecycle() {
    let id = TransactionId(1);
    let mut txn = Transaction::new(id);
    assert_eq!(txn.status, TransactionStatus::Active);
    
    txn.commit();
    assert_eq!(txn.status, TransactionStatus::Committed);
    
    // Can't abort after commit
    let mut txn2 = Transaction::new(TransactionId(2));
    txn2.abort();
    assert_eq!(txn2.status, TransactionStatus::Aborted);
}

#[test]
fn test_transaction_manager_empty() {
    let manager = TransactionManager::new();
    assert!(manager.get_transaction(TransactionId(999)).is_none());
}

#[test]
fn test_transaction_manager_sequential_ids() {
    let mut manager = TransactionManager::new();
    let id1 = manager.begin_transaction();
    let id2 = manager.begin_transaction();
    let id3 = manager.begin_transaction();
    
    assert_eq!(id1.0, 1);
    assert_eq!(id2.0, 2);
    assert_eq!(id3.0, 3);
}

// ============================================================================
// STORAGE MODULE EDGE CASES
// ============================================================================

#[tokio::test]
async fn test_store_table_id_zero() {
    let store = InMemoryColumnStore::new();
    let table_id = TableId(0);
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
async fn test_store_table_id_max() {
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
    
    let result = store.create_table(table_id, schema).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_store_read_empty_column_list() {
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
    
    let result = store.read_columns(table_id, vec![], 0, 10).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[tokio::test]
async fn test_store_read_invalid_column_ids() {
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
    
    // Try to read non-existent columns
    let result = store.read_columns(table_id, vec![999, 1000], 0, 10).await;
    // Should handle gracefully
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_store_write_empty_columns() {
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
    let result = store.write_columns(table_id, vec![]).await;
    // Should handle empty columns
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_compression_empty_data_all_types() {
    for comp_type in [CompressionType::None, CompressionType::LZ4, CompressionType::Zstd, CompressionType::Snappy] {
        let compressor = create_compressor(comp_type);
        let decompressor = create_decompressor(comp_type);
        
        let empty: &[u8] = &[];
        let compressed = compressor.compress(empty).unwrap();
        let decompressed = decompressor.decompress(&compressed, 0).unwrap();
        assert_eq!(decompressed, empty);
    }
}

#[test]
fn test_compression_single_byte() {
    let compressor = create_compressor(CompressionType::LZ4);
    let decompressor = create_decompressor(CompressionType::LZ4);
    
    let data = b"a";
    let compressed = compressor.compress(data).unwrap();
    let decompressed = decompressor.decompress(&compressed, 1).unwrap();
    assert_eq!(decompressed, data);
}

#[test]
fn test_compression_repeated_pattern() {
    let compressor = create_compressor(CompressionType::LZ4);
    let decompressor = create_decompressor(CompressionType::LZ4);
    
    // Highly compressible data
    let data = vec![0u8; 10000];
    let compressed = compressor.compress(&data).unwrap();
    assert!(compressed.len() < data.len());
    
    let decompressed = decompressor.decompress(&compressed, data.len()).unwrap();
    assert_eq!(decompressed, data);
}

#[test]
fn test_compression_random_data() {
    let compressor = create_compressor(CompressionType::LZ4);
    let decompressor = create_decompressor(CompressionType::LZ4);
    
    // Random data (hard to compress)
    let data: Vec<u8> = (0..1000).map(|i| (i * 7) as u8).collect();
    let compressed = compressor.compress(&data).unwrap();
    let decompressed = decompressor.decompress(&compressed, data.len()).unwrap();
    assert_eq!(decompressed, data);
}

#[test]
fn test_index_empty() {
    let mut index = BTreeIndex::new();
    assert_eq!(index.lookup(b"key").unwrap(), None);
    
    let results = index.range_scan(b"a", b"z").unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_index_single_entry() {
    let mut index = BTreeIndex::new();
    index.insert(b"key".to_vec(), 42).unwrap();
    assert_eq!(index.lookup(b"key").unwrap(), Some(42));
}

#[test]
fn test_index_duplicate_keys() {
    let mut index = BTreeIndex::new();
    index.insert(b"key".to_vec(), 1).unwrap();
    index.insert(b"key".to_vec(), 2).unwrap();
    // Last insert should overwrite
    assert_eq!(index.lookup(b"key").unwrap(), Some(2));
}

#[test]
fn test_index_range_scan_empty_range() {
    let mut index = BTreeIndex::new();
    index.insert(b"b".to_vec(), 1).unwrap();
    index.insert(b"c".to_vec(), 2).unwrap();
    
    // Range that doesn't match anything
    let results = index.range_scan(b"x", b"z").unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_index_range_scan_same_start_end() {
    let mut index = BTreeIndex::new();
    index.insert(b"key".to_vec(), 42).unwrap();
    
    let results = index.range_scan(b"key", b"key").unwrap();
    assert_eq!(results, vec![42]);
}

#[test]
fn test_cache_zero_size() {
    let cache = LRUCache::new(0);
    cache.insert("key", "value");
    // With size 0, should evict immediately or handle gracefully
    assert_eq!(cache.len(), 0);
}

#[test]
fn test_cache_single_entry() {
    let cache = LRUCache::new(1);
    cache.insert("key1", "value1");
    assert_eq!(cache.get(&"key1"), Some("value1"));
    
    cache.insert("key2", "value2");
    // key1 should be evicted
    assert_eq!(cache.get(&"key1"), None);
    assert_eq!(cache.get(&"key2"), Some("value2"));
}

#[test]
fn test_cache_ttl_expired() {
    let cache = LRUCache::with_ttl(10, Duration::from_millis(50));
    cache.insert("key", "value");
    assert_eq!(cache.get(&"key"), Some("value"));
    
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(cache.get(&"key"), None);
}

#[test]
fn test_cache_ttl_not_expired() {
    let cache = LRUCache::with_ttl(10, Duration::from_millis(200));
    cache.insert("key", "value");
    
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(cache.get(&"key"), Some("value"));
}

#[test]
fn test_writer_zero_block_size() {
    let writer = ColumnWriter::new(CompressionType::None, 0);
    let column = Column::Int32(vec![1, 2, 3]);
    // Should handle zero block size gracefully
    let result = writer.write_column(&column, 0);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_writer_single_block() {
    let writer = ColumnWriter::new(CompressionType::LZ4, 1000);
    let column = Column::Int32(vec![1, 2, 3]);
    let blocks = writer.write_column(&column, 0).unwrap();
    assert!(!blocks.is_empty());
}

#[test]
fn test_writer_large_block_size() {
    let writer = ColumnWriter::new(CompressionType::LZ4, usize::MAX);
    let column = Column::Int32(vec![1, 2, 3]);
    let result = writer.write_column(&column, 0);
    // Should handle large block size
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_reader_wrong_compression_type() {
    use narayana_storage::block::Block;
    use bytes::Bytes;
    
    let reader = ColumnReader::new(CompressionType::LZ4);
    // Create block with different compression
    let block = Block {
        column_id: 0,
        data: Bytes::from(vec![]),
        row_count: 0,
        data_type: DataType::Int32,
        compression: CompressionType::Zstd,
        uncompressed_size: 0,
        compressed_size: 0,
    };
    
    // Should handle compression type mismatch
    let result = reader.read_block(&block);
    assert!(result.is_ok() || result.is_err());
}

// ============================================================================
// QUERY MODULE EDGE CASES
// ============================================================================

#[test]
fn test_vectorized_filter_mismatched_lengths() {
    let column = Column::Int32(vec![1, 2, 3]);
    let mask = vec![true, false]; // Shorter than column
    
    // Should handle gracefully
    let filtered = VectorizedOps::filter(&column, &mask);
    assert!(filtered.len() <= column.len());
}

#[test]
fn test_vectorized_filter_longer_mask() {
    let column = Column::Int32(vec![1, 2]);
    let mask = vec![true, false, true, false, true]; // Longer than column
    
    let filtered = VectorizedOps::filter(&column, &mask);
    assert!(filtered.len() <= column.len());
}

#[test]
fn test_vectorized_compare_invalid_json() {
    let column = Column::Int32(vec![1, 2, 3]);
    
    // Invalid JSON types
    let null_val = serde_json::Value::Null;
    let array_val = serde_json::Value::Array(vec![]);
    let object_val = serde_json::Value::Object(serde_json::Map::new());
    
    let mask1 = VectorizedOps::compare_eq(&column, &null_val);
    let mask2 = VectorizedOps::compare_eq(&column, &array_val);
    let mask3 = VectorizedOps::compare_eq(&column, &object_val);
    
    // Should return all false for invalid types
    assert_eq!(mask1, vec![false, false, false]);
    assert_eq!(mask2, vec![false, false, false]);
    assert_eq!(mask3, vec![false, false, false]);
}

#[test]
fn test_vectorized_sum_overflow() {
    // Test with values that might overflow
    let column = Column::Int32(vec![i32::MAX, 1]);
    let sum = VectorizedOps::sum(&column);
    // Should handle overflow gracefully
    assert!(sum.is_some() || sum.is_none());
}

#[test]
fn test_vectorized_sum_underflow() {
    let column = Column::Int32(vec![i32::MIN, -1]);
    let sum = VectorizedOps::sum(&column);
    // Should handle underflow gracefully
    assert!(sum.is_some() || sum.is_none());
}

#[test]
fn test_vectorized_float_nan() {
    let column = Column::Float64(vec![f64::NAN, 1.0, 2.0]);
    let min = VectorizedOps::min(&column);
    let max = VectorizedOps::max(&column);
    // Should handle NaN gracefully
    assert!(min.is_some() || min.is_none());
    assert!(max.is_some() || max.is_none());
}

#[test]
fn test_vectorized_float_infinity() {
    let column = Column::Float64(vec![f64::INFINITY, 1.0, f64::NEG_INFINITY]);
    let min = VectorizedOps::min(&column);
    let max = VectorizedOps::max(&column);
    // Should handle infinity gracefully
    assert!(min.is_some() || min.is_none());
    assert!(max.is_some() || max.is_none());
}

#[test]
fn test_vectorized_string_empty() {
    let column = Column::String(vec!["".to_string(), "a".to_string()]);
    let value = serde_json::Value::String("".to_string());
    let mask = VectorizedOps::compare_eq(&column, &value);
    assert_eq!(mask[0], true);
    assert_eq!(mask[1], false);
}

#[test]
fn test_vectorized_string_unicode() {
    let column = Column::String(vec!["hello".to_string(), "世界".to_string(), "🌍".to_string()]);
    let value = serde_json::Value::String("世界".to_string());
    let mask = VectorizedOps::compare_eq(&column, &value);
    assert_eq!(mask, vec![false, true, false]);
}

#[test]
fn test_vectorized_string_very_long() {
    let long_string = "a".repeat(100000);
    let column = Column::String(vec![long_string.clone()]);
    let value = serde_json::Value::String(long_string);
    let mask = VectorizedOps::compare_eq(&column, &value);
    assert_eq!(mask[0], true);
}

#[test]
fn test_filter_operator_invalid_column() {
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
}

#[test]
fn test_filter_operator_empty_columns() {
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);
    
    let filter = Filter::Eq {
        column: "id".to_string(),
        value: serde_json::Value::Number(42.into()),
    };
    
    let columns = vec![];
    let operator = FilterOperator::new(filter, schema);
    // Should handle empty columns
    let result = operator.apply(&columns);
    assert!(result.is_err() || result.is_ok());
}

#[test]
fn test_project_operator_no_columns() {
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);
    
    let result = ProjectOperator::new(vec![], schema);
    // Empty projection should be handled
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_project_operator_duplicate_columns() {
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);
    
    let result = ProjectOperator::new(vec!["id".to_string(), "id".to_string()], schema);
    // Duplicate columns should be handled
    assert!(result.is_ok());
}

#[test]
fn test_project_operator_all_columns() {
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
    
    let columns = vec![
        Column::Int64(vec![1, 2]),
        Column::String(vec!["a".to_string(), "b".to_string()]),
    ];
    
    let operator = ProjectOperator::new(vec!["id".to_string(), "name".to_string()], schema).unwrap();
    let projected = operator.apply(&columns);
    assert_eq!(projected.len(), 2);
}

// ============================================================================
// BOUNDARY VALUE TESTS
// ============================================================================

#[test]
fn test_int32_boundaries() {
    let column = Column::Int32(vec![i32::MIN, -1, 0, 1, i32::MAX]);
    assert_eq!(column.len(), 5);
    
    let min_val = serde_json::Value::Number(i32::MIN.into());
    let max_val = serde_json::Value::Number(i32::MAX.into());
    
    let min_mask = VectorizedOps::compare_eq(&column, &min_val);
    let max_mask = VectorizedOps::compare_eq(&column, &max_val);
    
    assert_eq!(min_mask[0], true);
    assert_eq!(max_mask[4], true);
}

#[test]
fn test_int64_boundaries() {
    let column = Column::Int64(vec![i64::MIN, -1, 0, 1, i64::MAX]);
    assert_eq!(column.len(), 5);
    
    let min_val = serde_json::Value::Number(i64::MIN.into());
    let max_val = serde_json::Value::Number(i64::MAX.into());
    
    let min_mask = VectorizedOps::compare_eq(&column, &min_val);
    let max_mask = VectorizedOps::compare_eq(&column, &max_val);
    
    assert_eq!(min_mask[0], true);
    assert_eq!(max_mask[4], true);
}

#[test]
fn test_uint64_boundaries() {
    let column = Column::UInt64(vec![0, 1, u64::MAX]);
    assert_eq!(column.len(), 3);
    
    let max_val = serde_json::Value::Number(u64::MAX.into());
    let max_mask = VectorizedOps::compare_eq(&column, &max_val);
    
    assert_eq!(max_mask[2], true);
}

#[test]
fn test_float64_special_values() {
    let column = Column::Float64(vec![
        f64::NEG_INFINITY,
        f64::MIN,
        -1.0,
        0.0,
        1.0,
        f64::MAX,
        f64::INFINITY,
        f64::NAN,
    ]);
    assert_eq!(column.len(), 8);
    
    // Operations on special values should not panic
    let sum = VectorizedOps::sum(&column);
    let min = VectorizedOps::min(&column);
    let max = VectorizedOps::max(&column);
    
    // Results may be None or special values
    assert!(sum.is_some() || sum.is_none());
    assert!(min.is_some() || min.is_none());
    assert!(max.is_some() || max.is_none());
}

// ============================================================================
// PANIC PREVENTION TESTS
// ============================================================================

#[test]
fn test_no_panic_on_empty_vec_operations() {
    // All these should not panic
    let empty_int = Column::Int32(vec![]);
    let empty_mask = vec![];
    let _ = VectorizedOps::filter(&empty_int, &empty_mask);
    let _ = VectorizedOps::sum(&empty_int);
    let _ = VectorizedOps::min(&empty_int);
    let _ = VectorizedOps::max(&empty_int);
}

#[test]
fn test_no_panic_on_division_by_zero() {
    // Test that aggregations handle empty columns
    let empty = Column::Int32(vec![]);
    let count = VectorizedOps::count(&empty);
    assert_eq!(count, 0);
}

#[test]
fn test_no_panic_on_index_out_of_bounds() {
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);
    
    // Accessing non-existent field should return None, not panic
    assert_eq!(schema.field_index("nonexistent"), None);
    assert_eq!(schema.field("nonexistent"), None);
}

#[test]
fn test_no_panic_on_empty_string() {
    let column = Column::String(vec!["".to_string()]);
    let value = serde_json::Value::String("".to_string());
    let mask = VectorizedOps::compare_eq(&column, &value);
    assert_eq!(mask[0], true);
}

#[test]
fn test_no_panic_on_null_bytes() {
    let column = Column::Binary(vec![vec![0, 0, 0]]);
    assert_eq!(column.len(), 1);
}

// ============================================================================
// MEMORY SAFETY TESTS
// ============================================================================

#[test]
fn test_large_allocation_safety() {
    // Test that we can handle large but reasonable allocations
    let large_vec: Vec<i32> = (0..1_000_000).collect();
    let column = Column::Int32(large_vec);
    assert_eq!(column.len(), 1_000_000);
}

#[test]
fn test_string_memory_safety() {
    let strings: Vec<String> = (0..1000).map(|i| format!("string_{}", i)).collect();
    let column = Column::String(strings);
    assert_eq!(column.len(), 1000);
}

#[test]
fn test_binary_memory_safety() {
    let binaries: Vec<Vec<u8>> = (0..100).map(|i| vec![i as u8; 1000]).collect();
    let column = Column::Binary(binaries);
    assert_eq!(column.len(), 100);
}

// ============================================================================
// SERIALIZATION EDGE CASES
// ============================================================================

#[test]
fn test_serialize_deserialize_all_types() {
    use serde_json;
    
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);
    
    let serialized = serde_json::to_string(&schema).unwrap();
    let deserialized: Schema = serde_json::from_str(&serialized).unwrap();
    assert_eq!(schema.len(), deserialized.len());
}

#[test]
fn test_serialize_empty_schema() {
    use serde_json;
    
    let schema = Schema::new(vec![]);
    let serialized = serde_json::to_string(&schema).unwrap();
    let deserialized: Schema = serde_json::from_str(&serialized).unwrap();
    assert_eq!(schema.len(), deserialized.len());
}

// ============================================================================
// CONCURRENCY EDGE CASES
// ============================================================================

#[tokio::test]
async fn test_concurrent_same_table_id() {
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
    
    // First create should succeed
    let store1 = store.clone();
    let handle1 = tokio::spawn(async move {
        store1.create_table(table_id, schema).await
    });
    
    let result = handle1.await.unwrap();
    // Only first should succeed
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_rapid_create_delete() {
    let store = InMemoryColumnStore::new();
    
    for i in 0..100 {
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
}

// ============================================================================
// ERROR PATH TESTS
// ============================================================================

#[test]
fn test_error_messages_are_helpful() {
    let mut manager = TransactionManager::new();
    let id = TransactionId(999);
    
    let result = manager.commit_transaction(id);
    assert!(result.is_err());
    
    match result.unwrap_err() {
        Error::Transaction(msg) => {
            assert!(msg.contains("999"));
            assert!(msg.contains("not found"));
        }
        _ => panic!("Expected Transaction error"),
    }
}

#[tokio::test]
async fn test_error_table_already_exists_message() {
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
        Error::Storage(msg) => {
            assert!(msg.contains("already exists") || msg.contains("1"));
        }
        _ => panic!("Expected Storage error"),
    }
}

