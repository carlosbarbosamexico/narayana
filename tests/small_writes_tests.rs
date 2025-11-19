// Tests for small writes optimization

use narayana_storage::small_writes::*;
use narayana_core::types::TableId;
use bytes::Bytes;

#[tokio::test]
async fn test_small_write_buffer_creation() {
    let buffer = SmallWriteBuffer::new(1000, 100);
    // Should create successfully
}

#[tokio::test]
async fn test_small_write_buffer_write() {
    let buffer = SmallWriteBuffer::new(1000, 100);
    let table_id = TableId(1);
    let row = Row {
        data: vec![Bytes::from(b"test".to_vec())],
    };
    
    buffer.write(table_id, row).await.unwrap();
}

#[tokio::test]
async fn test_small_write_buffer_write_batch() {
    let buffer = SmallWriteBuffer::new(1000, 100);
    let table_id = TableId(1);
    let rows = vec![
        Row { data: vec![Bytes::from(b"row1".to_vec())] },
        Row { data: vec![Bytes::from(b"row2".to_vec())] },
    ];
    
    buffer.write_batch(table_id, rows).await.unwrap();
}

#[tokio::test]
async fn test_small_write_buffer_flush() {
    let buffer = SmallWriteBuffer::new(1000, 100);
    let table_id = TableId(1);
    let count = buffer.flush_table(table_id).await.unwrap();
    assert_eq!(count, 0); // Empty buffer
}

#[tokio::test]
async fn test_small_write_buffer_flush_all() {
    let buffer = SmallWriteBuffer::new(1000, 100);
    let results = buffer.flush_all().await.unwrap();
    assert_eq!(results.len(), 0); // No tables
}

#[test]
fn test_concurrent_write_handler_creation() {
    let handler = ConcurrentWriteHandler::new(10);
    // Should create successfully
}

#[test]
fn test_concurrent_write_handler_write() {
    let handler = ConcurrentWriteHandler::new(10);
    let table_id = TableId(1);
    let row = Row {
        data: vec![Bytes::from(b"test".to_vec())],
    };
    
    handler.write(table_id, row);
    // Should write without blocking
}

