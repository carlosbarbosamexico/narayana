// Tests for transaction engine

use narayana_storage::transaction_engine::*;

#[test]
fn test_lock_free_queue_creation() {
    let queue = LockFreeTransactionQueue::new();
    // Should create successfully
}

#[test]
fn test_lock_free_queue_push() {
    let queue = LockFreeTransactionQueue::new();
    let transaction = Transaction {
        id: 1,
        operations: vec![],
    };
    queue.push(transaction);
    assert_eq!(queue.len(), 1);
}

#[test]
fn test_lock_free_queue_pop() {
    let queue = LockFreeTransactionQueue::new();
    let transaction = Transaction {
        id: 1,
        operations: vec![],
    };
    queue.push(transaction);
    let popped = queue.pop();
    assert!(popped.is_some());
    assert_eq!(popped.unwrap().id, 1);
}

#[test]
fn test_batch_processor_creation() {
    let processor = BatchProcessor::new(100);
    // Should create successfully
}

#[test]
fn test_batch_processor_process() {
    let processor = BatchProcessor::new(100);
    let transactions = vec![
        Transaction { id: 1, operations: vec![] },
        Transaction { id: 2, operations: vec![] },
    ];
    processor.process(transactions);
    // Should process successfully
}

#[test]
fn test_write_optimized_wal_creation() {
    let wal = WriteOptimizedWAL::new();
    // Should create successfully
}

#[test]
fn test_write_optimized_wal_append() {
    let mut wal = WriteOptimizedWAL::new();
    let entry = WALEntry {
        transaction_id: 1,
        operation: "INSERT".to_string(),
        data: b"data".to_vec(),
    };
    wal.append(entry);
    assert_eq!(wal.len(), 1);
}

#[test]
fn test_lock_free_counter_creation() {
    let counter = LockFreeCounter::new();
    assert_eq!(counter.get(), 0);
}

#[test]
fn test_lock_free_counter_increment() {
    let counter = LockFreeCounter::new();
    counter.increment();
    assert_eq!(counter.get(), 1);
}

#[test]
fn test_lock_free_counter_add() {
    let counter = LockFreeCounter::new();
    counter.add(5);
    assert_eq!(counter.get(), 5);
}

#[test]
fn test_hot_path_cache_creation() {
    let cache = HotPathCache::new(100);
    // Should create successfully
}

#[test]
fn test_hot_path_cache_get_set() {
    let cache = HotPathCache::new(100);
    cache.set("key-1", "value-1");
    let value = cache.get("key-1");
    assert_eq!(value, Some("value-1".to_string()));
}

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

