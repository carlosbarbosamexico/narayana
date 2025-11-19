// Tests for hot path optimizations

use narayana_query::hot_path::*;
use narayana_core::column::Column;

#[test]
fn test_string_interner_creation() {
    let interner = StringInterner::new();
    // Should create successfully
}

#[test]
fn test_string_interner_intern() {
    let interner = StringInterner::new();
    let id1 = interner.intern("test");
    let id2 = interner.intern("test");
    assert_eq!(id1, id2); // Same string should get same ID
}

#[test]
fn test_string_interner_get() {
    let interner = StringInterner::new();
    let id = interner.intern("test");
    let retrieved = interner.get(id);
    assert_eq!(retrieved, Some("test".to_string()));
}

#[test]
fn test_string_interner_different_strings() {
    let interner = StringInterner::new();
    let id1 = interner.intern("test1");
    let id2 = interner.intern("test2");
    assert_ne!(id1, id2); // Different strings should get different IDs
}

#[test]
fn test_query_result_cache_creation() {
    let cache = QueryResultCache::new(100);
    // Should create successfully
}

#[test]
fn test_query_result_cache_cache() {
    let cache = QueryResultCache::new(100);
    let columns = vec![Column::Int32(vec![1, 2, 3])];
    cache.cache("SELECT * FROM users", columns.clone(), std::time::Duration::from_secs(60));
    // Should cache successfully
}

#[test]
fn test_query_result_cache_get() {
    let cache = QueryResultCache::new(100);
    let columns = vec![Column::Int32(vec![1, 2, 3])];
    cache.cache("SELECT * FROM users", columns.clone(), std::time::Duration::from_secs(60));
    
    let retrieved = cache.get("SELECT * FROM users");
    assert!(retrieved.is_some());
    match retrieved.unwrap()[0] {
        Column::Int32(ref data) => {
            assert_eq!(data, &vec![1, 2, 3]);
        }
        _ => panic!("Expected Int32 column"),
    }
}

#[test]
fn test_query_result_cache_expiry() {
    let cache = QueryResultCache::new(100);
    let columns = vec![Column::Int32(vec![1, 2, 3])];
    cache.cache("SELECT * FROM users", columns, std::time::Duration::from_millis(1));
    
    // Wait for expiry
    std::thread::sleep(std::time::Duration::from_millis(10));
    
    let retrieved = cache.get("SELECT * FROM users");
    assert!(retrieved.is_none()); // Should be expired
}

#[test]
fn test_simd_column_ops_sum() {
    let data = vec![1i32, 2, 3, 4, 5];
    let sum = SimdColumnOps::sum(&data);
    assert_eq!(sum, 15);
}

#[test]
fn test_simd_column_ops_min() {
    let data = vec![5i32, 2, 8, 1, 9];
    let min = SimdColumnOps::min(&data);
    assert_eq!(min, 1);
}

#[test]
fn test_simd_column_ops_max() {
    let data = vec![5i32, 2, 8, 1, 9];
    let max = SimdColumnOps::max(&data);
    assert_eq!(max, 9);
}

#[test]
fn test_simd_column_ops_filter() {
    let data = vec![1i32, 2, 3, 4, 5];
    let filtered = SimdColumnOps::filter(&data, |&x| x > 2);
    assert_eq!(filtered, vec![3, 4, 5]);
}

