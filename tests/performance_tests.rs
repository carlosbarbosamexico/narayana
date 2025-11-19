// Tests for performance optimizations

use narayana_storage::performance::*;
use narayana_core::column::Column;

#[test]
fn test_zero_copy_column_slice() {
    let column = Column::Int32(vec![1, 2, 3, 4, 5]);
    let slice = ZeroCopyColumnSlice::slice(&column, 1, 3);
    match slice {
        Column::Int32(data) => {
            assert_eq!(data, vec![2, 3, 4]);
        }
        _ => panic!("Expected Int32 column"),
    }
}

#[test]
fn test_batch_writer_creation() {
    let writer = BatchWriter::new(1000);
    // Should create successfully
}

#[test]
fn test_batch_writer_add() {
    let mut writer = BatchWriter::new(1000);
    let column = Column::Int32(vec![1, 2, 3]);
    writer.add(column);
    assert_eq!(writer.count(), 1);
}

#[test]
fn test_batch_writer_flush() {
    let mut writer = BatchWriter::new(1000);
    let column = Column::Int32(vec![1, 2, 3]);
    writer.add(column);
    let batch = writer.flush();
    assert_eq!(batch.len(), 1);
    assert_eq!(writer.count(), 0);
}

#[test]
fn test_parallel_reader_creation() {
    let reader = ParallelColumnReader::new(4);
    // Should create successfully
}

#[tokio::test]
async fn test_parallel_reader_read() {
    let reader = ParallelColumnReader::new(4);
    let columns = vec![
        Column::Int32(vec![1, 2, 3]),
        Column::Int32(vec![4, 5, 6]),
    ];
    let result = reader.read_parallel(columns).await;
    assert_eq!(result.len(), 2);
}

#[test]
fn test_memory_pool_creation() {
    let pool = MemoryPool::new(1000, 1024);
    // Should create successfully
}

#[test]
fn test_memory_pool_allocate() {
    let pool = MemoryPool::new(1000, 1024);
    let buffer = pool.allocate();
    assert_eq!(buffer.len(), 1024);
}

#[test]
fn test_memory_pool_deallocate() {
    let pool = MemoryPool::new(1000, 1024);
    let buffer = pool.allocate();
    pool.deallocate(buffer);
    // Should deallocate successfully
}

#[test]
fn test_compression_optimizer_select() {
    let optimizer = CompressionOptimizer::new();
    let data = vec![1u8; 1000];
    let algorithm = optimizer.select_best_algorithm(&data);
    // Should select an algorithm
    assert!(matches!(algorithm, narayana_core::types::CompressionType::LZ4 | 
                          narayana_core::types::CompressionType::Zstd | 
                          narayana_core::types::CompressionType::Snappy));
}

#[test]
fn test_bloom_filter_creation() {
    let filter = BloomFilter::new(1000, 0.01);
    // Should create successfully
}

#[test]
fn test_bloom_filter_add_check() {
    let mut filter = BloomFilter::new(100, 0.01);
    filter.add(b"test");
    assert!(filter.might_contain(b"test"));
}

#[test]
fn test_column_statistics_update() {
    let mut stats = ColumnStatistics::new();
    let column = Column::Int32(vec![1, 2, 3, 4, 5]);
    stats.update(&column);
    assert_eq!(stats.count, 5);
    assert_eq!(stats.min, Some(1));
    assert_eq!(stats.max, Some(5));
}

#[test]
fn test_prefetch_manager_creation() {
    let manager = PrefetchManager::new();
    // Should create successfully
}

#[test]
fn test_prefetch_manager_prefetch() {
    let manager = PrefetchManager::new();
    manager.prefetch(narayana_core::types::TableId(1), vec![0, 1, 2]);
    // Should prefetch successfully
}

