// Tests for ML integration

use narayana_query::ml_integration::*;
use narayana_core::column::Column;

#[test]
fn test_vector_ops_dot_product() {
    let a = vec![1.0f32, 2.0, 3.0];
    let b = vec![4.0f32, 5.0, 6.0];
    
    let dot = VectorOps::dot_product(&a, &b);
    assert_eq!(dot, 32.0); // 1*4 + 2*5 + 3*6 = 32
}

#[test]
fn test_vector_ops_euclidean_distance() {
    let a = vec![0.0f32, 0.0];
    let b = vec![3.0f32, 4.0];
    
    let distance = VectorOps::euclidean_distance(&a, &b);
    assert_eq!(distance, 5.0); // sqrt(3^2 + 4^2) = 5
}

#[test]
fn test_vector_ops_cosine_similarity() {
    let a = vec![1.0f32, 0.0];
    let b = vec![1.0f32, 0.0];
    
    let similarity = VectorOps::cosine_similarity(&a, &b);
    assert!((similarity - 1.0).abs() < 0.001); // Should be 1.0
}

#[test]
fn test_vector_ops_norm() {
    let v = vec![3.0f32, 4.0];
    let norm = VectorOps::norm(&v);
    assert_eq!(norm, 5.0); // sqrt(3^2 + 4^2) = 5
}

#[test]
fn test_vector_ops_normalize() {
    let v = vec![3.0f32, 4.0];
    let normalized = VectorOps::normalize(&v);
    let norm = VectorOps::norm(&normalized);
    assert!((norm - 1.0).abs() < 0.001); // Should be normalized
}

#[test]
fn test_array_ops_to_float_array_int32() {
    let column = Column::Int32(vec![1, 2, 3]);
    let arrays = ArrayOps::to_float_array(&column);
    assert_eq!(arrays.len(), 3);
    assert_eq!(arrays[0], vec![1.0f32]);
}

#[test]
fn test_array_ops_to_float_array_float64() {
    let column = Column::Float64(vec![1.0, 2.0, 3.0]);
    let arrays = ArrayOps::to_float_array(&column);
    assert_eq!(arrays.len(), 3);
    assert_eq!(arrays[0], vec![1.0f32]);
}

