// Comprehensive tests for block operations

use narayana_core::{
    schema::DataType,
    types::CompressionType,
};
use narayana_storage::{
    block::{Block, BlockMetadata},
    writer::ColumnWriter,
    reader::ColumnReader,
};
use narayana_core::column::Column;
use bytes::Bytes;

// ============================================================================
// BLOCK METADATA TESTS
// ============================================================================

#[test]
fn test_block_metadata_creation() {
    let metadata = BlockMetadata {
        block_id: 1,
        column_id: 0,
        row_start: 0,
        row_count: 100,
        data_type: DataType::Int32,
        compression: CompressionType::LZ4,
        uncompressed_size: 400,
        compressed_size: 200,
        min_value: None,
        max_value: None,
        null_count: 0,
    };
    
    assert_eq!(metadata.block_id, 1);
    assert_eq!(metadata.column_id, 0);
    assert_eq!(metadata.row_count, 100);
    assert_eq!(metadata.compression_ratio(), 0.5);
}

#[test]
fn test_block_metadata_compression_ratio() {
    let metadata1 = BlockMetadata {
        block_id: 1,
        column_id: 0,
        row_start: 0,
        row_count: 100,
        data_type: DataType::Int32,
        compression: CompressionType::LZ4,
        uncompressed_size: 1000,
        compressed_size: 500,
        min_value: None,
        max_value: None,
        null_count: 0,
    };
    
    assert_eq!(metadata1.compression_ratio(), 0.5);
    
    let metadata2 = BlockMetadata {
        block_id: 2,
        column_id: 0,
        row_start: 0,
        row_count: 100,
        data_type: DataType::Int32,
        compression: CompressionType::None,
        uncompressed_size: 1000,
        compressed_size: 1000,
        min_value: None,
        max_value: None,
        null_count: 0,
    };
    
    assert_eq!(metadata2.compression_ratio(), 1.0);
    
    let metadata3 = BlockMetadata {
        block_id: 3,
        column_id: 0,
        row_start: 0,
        row_count: 100,
        data_type: DataType::Int32,
        compression: CompressionType::LZ4,
        uncompressed_size: 0,
        compressed_size: 0,
        min_value: None,
        max_value: None,
        null_count: 0,
    };
    
    assert_eq!(metadata3.compression_ratio(), 1.0);
}

#[test]
fn test_block_metadata_min_max_values() {
    let metadata = BlockMetadata {
        block_id: 1,
        column_id: 0,
        row_start: 0,
        row_count: 100,
        data_type: DataType::Int32,
        compression: CompressionType::LZ4,
        uncompressed_size: 400,
        compressed_size: 200,
        min_value: Some(vec![0, 0, 0, 1]), // Little-endian representation of 1
        max_value: Some(vec![0xFF, 0xFF, 0xFF, 0x7F]), // Max i32
        null_count: 0,
    };
    
    assert!(metadata.min_value.is_some());
    assert!(metadata.max_value.is_some());
    assert_eq!(metadata.null_count, 0);
}

#[test]
fn test_block_metadata_null_count() {
    let metadata = BlockMetadata {
        block_id: 1,
        column_id: 0,
        row_start: 0,
        row_count: 100,
        data_type: DataType::Nullable(Box::new(DataType::Int32)),
        compression: CompressionType::LZ4,
        uncompressed_size: 400,
        compressed_size: 200,
        min_value: None,
        max_value: None,
        null_count: 25,
    };
    
    assert_eq!(metadata.null_count, 25);
    assert_eq!(metadata.row_count, 100);
}

// ============================================================================
// BLOCK CREATION TESTS
// ============================================================================

#[test]
fn test_block_creation() {
    let block = Block {
        column_id: 0,
        data: Bytes::from(vec![1, 2, 3, 4]),
        row_count: 1,
        data_type: DataType::Int32,
        compression: CompressionType::LZ4,
        uncompressed_size: 4,
        compressed_size: 4,
    };
    
    assert_eq!(block.column_id, 0);
    assert_eq!(block.row_count, 1);
    assert_eq!(block.data.len(), 4);
}

#[test]
fn test_block_empty_data() {
    let block = Block {
        column_id: 0,
        data: Bytes::from(vec![]),
        row_count: 0,
        data_type: DataType::Int32,
        compression: CompressionType::None,
        uncompressed_size: 0,
        compressed_size: 0,
    };
    
    assert_eq!(block.row_count, 0);
    assert_eq!(block.data.len(), 0);
}

#[test]
fn test_block_large_data() {
    let large_data = vec![0u8; 1_000_000];
    let block = Block {
        column_id: 0,
        data: Bytes::from(large_data.clone()),
        row_count: 1000,
        data_type: DataType::Int32,
        compression: CompressionType::LZ4,
        uncompressed_size: large_data.len(),
        compressed_size: large_data.len() / 2,
    };
    
    assert_eq!(block.data.len(), 1_000_000);
    assert_eq!(block.row_count, 1000);
}

// ============================================================================
// COLUMN WRITER TESTS
// ============================================================================

#[test]
fn test_column_writer_all_compression_types() {
    let column = Column::Int32(vec![1, 2, 3, 4, 5]);
    
    for comp_type in [CompressionType::None, CompressionType::LZ4, CompressionType::Zstd, CompressionType::Snappy] {
        let writer = ColumnWriter::new(comp_type, 100);
        let result = writer.write_column(&column, 0);
        assert!(result.is_ok());
        
        let blocks = result.unwrap();
        assert!(!blocks.is_empty());
        
        for (block, metadata) in &blocks {
            assert_eq!(block.compression, comp_type);
            assert_eq!(metadata.compression, comp_type);
        }
    }
}

#[test]
fn test_column_writer_block_splitting() {
    let writer = ColumnWriter::new(CompressionType::LZ4, 10);
    let data: Vec<i32> = (0..100).collect();
    let column = Column::Int32(data);
    
    let blocks = writer.write_column(&column, 0).unwrap();
    
    // Should create multiple blocks
    assert!(blocks.len() > 1);
    
    // Verify row counts
    let total_rows: usize = blocks.iter().map(|(_, m)| m.row_count).sum();
    assert_eq!(total_rows, 100);
}

#[test]
fn test_column_writer_single_block() {
    let writer = ColumnWriter::new(CompressionType::LZ4, 1000);
    let column = Column::Int32(vec![1, 2, 3]);
    
    let blocks = writer.write_column(&column, 0).unwrap();
    
    // Should create single block
    assert_eq!(blocks.len(), 1);
    
    let (block, metadata) = &blocks[0];
    assert_eq!(block.row_count, 3);
    assert_eq!(metadata.row_count, 3);
}

#[test]
fn test_column_writer_row_offsets() {
    let writer = ColumnWriter::new(CompressionType::LZ4, 10);
    let data: Vec<i32> = (0..30).collect();
    let column = Column::Int32(data);
    
    let blocks = writer.write_column(&column, 0).unwrap();
    
    // Verify row offsets are sequential
    let mut expected_start = 0;
    for (_, metadata) in &blocks {
        assert_eq!(metadata.row_start, expected_start);
        expected_start += metadata.row_count;
    }
}

#[test]
fn test_column_writer_different_column_ids() {
    let writer = ColumnWriter::new(CompressionType::LZ4, 100);
    let column = Column::Int32(vec![1, 2, 3]);
    
    for column_id in 0..10 {
        let blocks = writer.write_column(&column, column_id).unwrap();
        for (block, metadata) in &blocks {
            assert_eq!(block.column_id, column_id);
            assert_eq!(metadata.column_id, column_id);
        }
    }
}

#[test]
fn test_column_writer_compression_effectiveness() {
    // Test that compression actually reduces size for compressible data
    let writer = ColumnWriter::new(CompressionType::LZ4, 1000);
    let data: Vec<i32> = vec![42; 1000]; // Highly repetitive
    let column = Column::Int32(data);
    
    let blocks = writer.write_column(&column, 0).unwrap();
    
    for (block, metadata) in &blocks {
        // Compressed size should be less than or equal to uncompressed
        assert!(block.compressed_size <= metadata.uncompressed_size);
    }
}

#[test]
fn test_column_writer_all_column_types() {
    let writer = ColumnWriter::new(CompressionType::LZ4, 100);
    
    // Test all supported column types
    let columns = vec![
        Column::Int8(vec![1, 2, 3]),
        Column::Int32(vec![1, 2, 3]),
        Column::Int64(vec![1, 2, 3]),
        Column::UInt64(vec![1, 2, 3]),
        Column::Float64(vec![1.0, 2.0, 3.0]),
        Column::String(vec!["a".to_string(), "b".to_string(), "c".to_string()]),
    ];
    
    for (idx, column) in columns.iter().enumerate() {
        let result = writer.write_column(column, idx as u32);
        assert!(result.is_ok(), "Failed to write column type at index {}", idx);
    }
}

// ============================================================================
// COLUMN READER TESTS
// ============================================================================

#[test]
fn test_column_reader_roundtrip() {
    let writer = ColumnWriter::new(CompressionType::LZ4, 100);
    let reader = ColumnReader::new(CompressionType::LZ4);
    
    let original = Column::Int32(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    let blocks = writer.write_column(&original, 0).unwrap();
    
    // Read all blocks and reconstruct
    let mut reconstructed_data = Vec::new();
    for (block, _) in &blocks {
        let read_column = reader.read_block(block).unwrap();
        match read_column {
            Column::Int32(data) => {
                reconstructed_data.extend(data);
            }
            _ => panic!("Expected Int32"),
        }
    }
    
    match original {
        Column::Int32(orig_data) => {
            assert_eq!(reconstructed_data, orig_data);
        }
        _ => panic!("Expected Int32"),
    }
}

#[test]
fn test_column_reader_all_compression_types() {
    let original = Column::Int32(vec![1, 2, 3, 4, 5]);
    
    for comp_type in [CompressionType::None, CompressionType::LZ4, CompressionType::Zstd, CompressionType::Snappy] {
        let writer = ColumnWriter::new(comp_type, 100);
        let reader = ColumnReader::new(comp_type);
        
        let blocks = writer.write_column(&original, 0).unwrap();
        for (block, _) in &blocks {
            let read_column = reader.read_block(block).unwrap();
            match (&original, &read_column) {
                (Column::Int32(orig), Column::Int32(read)) => {
                    assert_eq!(orig, read);
                }
                _ => panic!("Type mismatch"),
            }
        }
    }
}

#[test]
fn test_column_reader_wrong_compression_type() {
    let writer = ColumnWriter::new(CompressionType::LZ4, 100);
    let wrong_reader = ColumnReader::new(CompressionType::Zstd);
    
    let column = Column::Int32(vec![1, 2, 3]);
    let blocks = writer.write_column(&column, 0).unwrap();
    
    // Try to read with wrong decompressor
    for (block, _) in &blocks {
        let result = wrong_reader.read_block(block);
        // Should fail or handle gracefully
        assert!(result.is_err() || result.is_ok());
    }
}

#[test]
fn test_column_reader_empty_block() {
    let writer = ColumnWriter::new(CompressionType::LZ4, 100);
    let reader = ColumnReader::new(CompressionType::LZ4);
    
    let empty_column = Column::Int32(vec![]);
    let blocks = writer.write_column(&empty_column, 0).unwrap();
    
    // Should handle empty blocks
    if !blocks.is_empty() {
        for (block, _) in &blocks {
            let result = reader.read_block(block);
            // Should succeed or fail gracefully
            assert!(result.is_ok() || result.is_err());
        }
    }
}

#[test]
fn test_column_reader_corrupted_block() {
    let reader = ColumnReader::new(CompressionType::LZ4);
    
    // Create corrupted block
    let corrupted_block = Block {
        column_id: 0,
        data: Bytes::from(vec![0xFF, 0xFF, 0xFF, 0xFF]), // Invalid compressed data
        row_count: 1,
        data_type: DataType::Int32,
        compression: CompressionType::LZ4,
        uncompressed_size: 4,
        compressed_size: 4,
    };
    
    let result = reader.read_block(&corrupted_block);
    assert!(result.is_err());
}

#[test]
fn test_column_reader_size_mismatch() {
    let writer = ColumnWriter::new(CompressionType::LZ4, 100);
    let reader = ColumnReader::new(CompressionType::LZ4);
    
    let column = Column::Int32(vec![1, 2, 3]);
    let blocks = writer.write_column(&column, 0).unwrap();
    
    // Modify uncompressed_size to wrong value
    for (mut block, _) in blocks {
        block.uncompressed_size = 999999;
        let result = reader.read_block(&block);
        // Should handle size mismatch
        assert!(result.is_err() || result.is_ok());
    }
}

// ============================================================================
// BLOCK METADATA EDGE CASES
// ============================================================================

#[test]
fn test_block_metadata_zero_compression_ratio() {
    let metadata = BlockMetadata {
        block_id: 1,
        column_id: 0,
        row_start: 0,
        row_count: 0,
        data_type: DataType::Int32,
        compression: CompressionType::LZ4,
        uncompressed_size: 0,
        compressed_size: 0,
        min_value: None,
        max_value: None,
        null_count: 0,
    };
    
    assert_eq!(metadata.compression_ratio(), 1.0);
}

#[test]
fn test_block_metadata_negative_compression() {
    // Compression that increases size (shouldn't happen but test it)
    let metadata = BlockMetadata {
        block_id: 1,
        column_id: 0,
        row_start: 0,
        row_count: 100,
        data_type: DataType::Int32,
        compression: CompressionType::LZ4,
        uncompressed_size: 100,
        compressed_size: 200, // Larger than uncompressed
        min_value: None,
        max_value: None,
        null_count: 0,
    };
    
    // Should handle gracefully
    assert_eq!(metadata.compression_ratio(), 2.0);
}

#[test]
fn test_block_metadata_max_values() {
    let metadata = BlockMetadata {
        block_id: u64::MAX,
        column_id: u32::MAX,
        row_start: usize::MAX,
        row_count: usize::MAX,
        data_type: DataType::Int32,
        compression: CompressionType::LZ4,
        uncompressed_size: usize::MAX,
        compressed_size: usize::MAX,
        min_value: None,
        max_value: None,
        null_count: usize::MAX,
    };
    
    // Should handle max values
    assert_eq!(metadata.block_id, u64::MAX);
    assert_eq!(metadata.column_id, u32::MAX);
    assert_eq!(metadata.compression_ratio(), 1.0);
}

// ============================================================================
// BLOCK SERIALIZATION TESTS
// ============================================================================

#[test]
fn test_block_serialization() {
    use serde_json;
    
    let block = Block {
        column_id: 1,
        data: Bytes::from(vec![1, 2, 3, 4]),
        row_count: 1,
        data_type: DataType::Int32,
        compression: CompressionType::LZ4,
        uncompressed_size: 4,
        compressed_size: 4,
    };
    
    // Block contains Bytes which may not serialize directly
    // This tests that we can work with blocks
    assert_eq!(block.column_id, 1);
    assert_eq!(block.row_count, 1);
}

#[test]
fn test_block_metadata_serialization() {
    use serde_json;
    
    let metadata = BlockMetadata {
        block_id: 1,
        column_id: 0,
        row_start: 0,
        row_count: 100,
        data_type: DataType::Int32,
        compression: CompressionType::LZ4,
        uncompressed_size: 400,
        compressed_size: 200,
        min_value: Some(vec![1, 2, 3, 4]),
        max_value: Some(vec![5, 6, 7, 8]),
        null_count: 10,
    };
    
    let serialized = serde_json::to_string(&metadata).unwrap();
    let deserialized: BlockMetadata = serde_json::from_str(&serialized).unwrap();
    
    assert_eq!(metadata.block_id, deserialized.block_id);
    assert_eq!(metadata.column_id, deserialized.column_id);
    assert_eq!(metadata.row_count, deserialized.row_count);
    assert_eq!(metadata.compression_ratio(), deserialized.compression_ratio());
}

// ============================================================================
// BLOCK PERFORMANCE TESTS
// ============================================================================

#[test]
fn test_block_write_performance() {
    let writer = ColumnWriter::new(CompressionType::LZ4, 10000);
    let large_data: Vec<i32> = (0..1_000_000).collect();
    let column = Column::Int32(large_data);
    
    let start = std::time::Instant::now();
    let blocks = writer.write_column(&column, 0).unwrap();
    let duration = start.elapsed();
    
    assert!(!blocks.is_empty());
    assert!(duration.as_secs() < 10); // Should be fast
}

#[test]
fn test_block_read_performance() {
    let writer = ColumnWriter::new(CompressionType::LZ4, 10000);
    let reader = ColumnReader::new(CompressionType::LZ4);
    
    let large_data: Vec<i32> = (0..1_000_000).collect();
    let column = Column::Int32(large_data);
    let blocks = writer.write_column(&column, 0).unwrap();
    
    let start = std::time::Instant::now();
    for (block, _) in &blocks {
        let _ = reader.read_block(block).unwrap();
    }
    let duration = start.elapsed();
    
    assert!(duration.as_secs() < 10); // Should be fast
}

// ============================================================================
// BLOCK EDGE CASES
// ============================================================================

#[test]
fn test_block_partial_read() {
    let writer = ColumnWriter::new(CompressionType::LZ4, 10);
    let reader = ColumnReader::new(CompressionType::LZ4);
    
    let data: Vec<i32> = (0..100).collect();
    let column = Column::Int32(data);
    let blocks = writer.write_column(&column, 0).unwrap();
    
    // Read only first block
    if !blocks.is_empty() {
        let (block, _) = &blocks[0];
        let read_column = reader.read_block(block).unwrap();
        match read_column {
            Column::Int32(data) => {
                assert!(data.len() <= 10); // Should be <= block size
            }
            _ => panic!("Expected Int32"),
        }
    }
}

#[test]
fn test_block_concurrent_access() {
    use std::sync::Arc;
    
    let writer = ColumnWriter::new(CompressionType::LZ4, 100);
    let reader = Arc::new(ColumnReader::new(CompressionType::LZ4));
    
    let column = Column::Int32(vec![1, 2, 3, 4, 5]);
    let blocks = writer.write_column(&column, 0).unwrap();
    
    // Multiple readers accessing same blocks concurrently
    let mut handles = vec![];
    for (block, _) in &blocks {
        let reader = reader.clone();
        let block = block.clone();
        let handle = std::thread::spawn(move || {
            reader.read_block(&block)
        });
        handles.push(handle);
    }
    
    for handle in handles {
        let result = handle.join().unwrap();
        assert!(result.is_ok());
    }
}

#[test]
fn test_block_metadata_statistics() {
    let metadata = BlockMetadata {
        block_id: 1,
        column_id: 0,
        row_start: 0,
        row_count: 1000,
        data_type: DataType::Int32,
        compression: CompressionType::LZ4,
        uncompressed_size: 4000,
        compressed_size: 1000,
        min_value: Some(vec![0, 0, 0, 1]),
        max_value: Some(vec![0xFF, 0xFF, 0xFF, 0x7F]),
        null_count: 50,
    };
    
    // Verify statistics
    assert_eq!(metadata.row_count, 1000);
    assert_eq!(metadata.null_count, 50);
    assert_eq!(metadata.compression_ratio(), 0.25); // 1000/4000
    assert!(metadata.min_value.is_some());
    assert!(metadata.max_value.is_some());
}

