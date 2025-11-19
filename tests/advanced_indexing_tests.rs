// Tests for advanced indexing

use narayana_storage::advanced_indexing::*;
use narayana_core::schema::DataType;
use bytes::Bytes;

#[test]
fn test_skip_index_creation() {
    let mut index = SkipIndex::new(1, 1000);
    assert_eq!(index.column_id, 1);
    assert_eq!(index.block_size, 1000);
}

#[test]
fn test_skip_index_add_block() {
    let mut index = SkipIndex::new(1, 1000);
    index.add_block(
        Bytes::from(vec![1u8]),
        Bytes::from(vec![10u8]),
        0,
    );
    assert_eq!(index.min_values.len(), 1);
    assert_eq!(index.max_values.len(), 1);
}

#[test]
fn test_skip_index_find_blocks() {
    let mut index = SkipIndex::new(1, 1000);
    index.add_block(
        Bytes::from(vec![5u8]),
        Bytes::from(vec![10u8]),
        0,
    );
    index.add_block(
        Bytes::from(vec![15u8]),
        Bytes::from(vec![20u8]),
        1000,
    );
    
    let blocks = index.find_blocks(&Bytes::from(vec![7u8]), &Bytes::from(vec![12u8])).unwrap();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0], 0);
}

#[test]
fn test_bloom_filter_creation() {
    let filter = BloomFilter::new(1000, 0.01).unwrap();
    assert!(filter.bit_count > 0);
    assert!(filter.hash_count > 0);
}

#[test]
fn test_bloom_filter_add_and_check() {
    let mut filter = BloomFilter::new(100, 0.01).unwrap();
    filter.add(b"test").unwrap();
    
    assert!(filter.might_contain(b"test").unwrap());
    // False positives are possible, but "test" should definitely be positive
}

#[test]
fn test_min_max_index_creation() {
    let index = MinMaxIndex::new(1);
    assert_eq!(index.column_id, 1);
}

#[test]
fn test_min_max_index_update() {
    let mut index = MinMaxIndex::new(1);
    index.update(Bytes::from(vec![5u8])).unwrap();
    index.update(Bytes::from(vec![10u8])).unwrap();
    index.update(Bytes::from(vec![3u8])).unwrap();
    
    assert_eq!(index.global_min, Some(Bytes::from(vec![3u8])));
    assert_eq!(index.global_max, Some(Bytes::from(vec![10u8])));
}

#[test]
fn test_min_max_index_might_contain() {
    let mut index = MinMaxIndex::new(1);
    index.update(Bytes::from(vec![5u8])).unwrap();
    index.update(Bytes::from(vec![10u8])).unwrap();
    
    assert!(index.might_contain(&Bytes::from(vec![7u8]), &Bytes::from(vec![8u8])).unwrap());
    assert!(!index.might_contain(&Bytes::from(vec![15u8]), &Bytes::from(vec![20u8])).unwrap());
}

#[test]
fn test_index_manager_creation() {
    let _manager = AdvancedIndexManager::new();
    // Should create successfully
}

#[test]
fn test_index_manager_create_skip_index() {
    let manager = AdvancedIndexManager::new();
    manager.create_skip_index(1, 1000);
    // Should create successfully
}

#[test]
fn test_index_manager_create_bloom_filter() {
    let manager = AdvancedIndexManager::new();
    manager.create_bloom_filter(1, 1000, 0.01);
    // Should create successfully
}

#[test]
fn test_index_manager_create_min_max_index() {
    let manager = AdvancedIndexManager::new();
    manager.create_min_max_index(1);
    // Should create successfully
}

#[test]
fn test_index_manager_might_contain() {
    let manager = AdvancedIndexManager::new();
    manager.create_bloom_filter(1, 100, 0.01);
    
    // Without adding, might_contain should return true (no filter means might contain)
    assert!(manager.might_contain(1, b"test").unwrap());
}

#[test]
fn test_index_manager_find_blocks() {
    let manager = AdvancedIndexManager::new();
    manager.create_skip_index(1, 1000);
    
    let blocks = manager.find_blocks_for_range(
        1,
        &Bytes::from(vec![5u8]),
        &Bytes::from(vec![10u8]),
    ).unwrap();
    // Should return empty if no blocks added
    assert_eq!(blocks.len(), 0);
}

