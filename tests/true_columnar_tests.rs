// Tests for true columnar operations

use narayana_storage::true_columnar::*;
use narayana_core::column::Column;

#[test]
fn test_column_ops_sum_int32() {
    let column = Column::Int32(vec![1, 2, 3, 4, 5]);
    let sum = TrueColumnarOps::sum(&column);
    assert_eq!(sum, Some(15));
}

#[test]
fn test_column_ops_sum_int64() {
    let column = Column::Int64(vec![10, 20, 30]);
    let sum = TrueColumnarOps::sum(&column);
    assert_eq!(sum, Some(60));
}

#[test]
fn test_column_ops_sum_float64() {
    let column = Column::Float64(vec![1.5, 2.5, 3.5]);
    let sum = TrueColumnarOps::sum(&column);
    assert!((sum.unwrap() - 7.5).abs() < 0.001);
}

#[test]
fn test_column_ops_sum_empty() {
    let column = Column::Int32(vec![]);
    let sum = TrueColumnarOps::sum(&column);
    assert_eq!(sum, None);
}

#[test]
fn test_column_ops_min_int32() {
    let column = Column::Int32(vec![5, 2, 8, 1, 9]);
    let min = TrueColumnarOps::min(&column);
    assert_eq!(min, Some(1));
}

#[test]
fn test_column_ops_max_int32() {
    let column = Column::Int32(vec![5, 2, 8, 1, 9]);
    let max = TrueColumnarOps::max(&column);
    assert_eq!(max, Some(9));
}

#[test]
fn test_column_ops_count() {
    let column = Column::Int32(vec![1, 2, 3, 4, 5]);
    let count = TrueColumnarOps::count(&column);
    assert_eq!(count, 5);
}

#[test]
fn test_column_ops_avg() {
    let column = Column::Int32(vec![1, 2, 3, 4, 5]);
    let avg = TrueColumnarOps::avg(&column);
    assert_eq!(avg, Some(3.0));
}

#[test]
fn test_column_ops_filter() {
    let column = Column::Int32(vec![1, 2, 3, 4, 5]);
    let filtered = TrueColumnarOps::filter(&column, |&x| x > 2);
    match filtered {
        Column::Int32(data) => {
            assert_eq!(data, vec![3, 4, 5]);
        }
        _ => panic!("Expected Int32 column"),
    }
}

#[test]
fn test_column_ops_map() {
    let column = Column::Int32(vec![1, 2, 3]);
    let mapped = TrueColumnarOps::map(&column, |x| x * 2);
    match mapped {
        Column::Int32(data) => {
            assert_eq!(data, vec![2, 4, 6]);
        }
        _ => panic!("Expected Int32 column"),
    }
}

#[test]
fn test_simd_ops_sum() {
    let data = vec![1i32, 2, 3, 4, 5];
    let sum = SimdColumnOps::sum(&data);
    assert_eq!(sum, 15);
}

#[test]
fn test_simd_ops_min() {
    let data = vec![5i32, 2, 8, 1, 9];
    let min = SimdColumnOps::min(&data);
    assert_eq!(min, 1);
}

#[test]
fn test_simd_ops_max() {
    let data = vec![5i32, 2, 8, 1, 9];
    let max = SimdColumnOps::max(&data);
    assert_eq!(max, 9);
}

#[test]
fn test_cache_aligned_access() {
    let data = vec![1i32, 2, 3, 4, 5];
    let aligned = CacheAlignedAccess::align(&data);
    // Should align data for cache efficiency
    assert_eq!(aligned.len(), 5);
}

