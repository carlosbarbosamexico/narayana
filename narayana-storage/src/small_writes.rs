// Optimized handling of frequent small writes - ClickHouse limitation

use narayana_core::{Error, Result, types::TableId};
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use crossbeam::queue::SegQueue;
use bytes::Bytes;

/// Write buffer for small writes - batches them efficiently
pub struct SmallWriteBuffer {
    buffers: Arc<RwLock<HashMap<TableId, WriteBuffer>>>,
    batch_size: usize,
    flush_interval_ms: u64,
}

struct WriteBuffer {
    rows: Vec<Row>,
    last_flush: std::time::Instant,
}

#[derive(Debug, Clone)]
pub struct Row {
    pub data: Vec<Bytes>,
}

impl SmallWriteBuffer {
    pub fn new(batch_size: usize, flush_interval_ms: u64) -> Self {
        Self {
            buffers: Arc::new(RwLock::new(HashMap::new())),
            batch_size,
            flush_interval_ms,
        }
    }

    /// Write single row (buffered, non-blocking)
    /// SECURITY: Prevent unbounded Vec growth
    pub async fn write(&self, table_id: TableId, row: Row) -> Result<()> {
        // SECURITY: Limit buffer size to prevent memory exhaustion
        const MAX_BUFFER_SIZE: usize = 10_000_000; // Maximum rows per buffer
        
        let should_flush = {
            let mut buffers = self.buffers.write();
            let buffer = buffers.entry(table_id).or_insert_with(|| WriteBuffer {
                rows: Vec::new(),
                last_flush: std::time::Instant::now(),
            });

            // SECURITY: If buffer is too large, force flush to prevent unbounded growth
            if buffer.rows.len() >= MAX_BUFFER_SIZE {
                // Remove 10% of oldest entries (FIFO eviction)
                let remove_count = MAX_BUFFER_SIZE / 10;
                buffer.rows.drain(0..remove_count);
            }

            buffer.rows.push(row);
            let len = buffer.rows.len();
            len >= self.batch_size
        };

        // Auto-flush if batch size reached
        if should_flush {
            self.flush_table(table_id).await?;
        }

        Ok(())
    }

    /// Write multiple rows (batch)
    /// SECURITY: Prevent unbounded Vec growth and DoS with large batches
    pub async fn write_batch(&self, table_id: TableId, rows: Vec<Row>) -> Result<()> {
        // SECURITY: Limit batch size to prevent DoS
        const MAX_BATCH_SIZE: usize = 1_000_000; // Maximum rows per batch
        const MAX_BUFFER_SIZE: usize = 10_000_000; // Maximum rows per buffer
        
        // SECURITY: Reject batches that are too large
        if rows.len() > MAX_BATCH_SIZE {
            return Err(narayana_core::Error::Storage(format!(
                "Batch size {} exceeds maximum {}", rows.len(), MAX_BATCH_SIZE
            )));
        }
        
        let should_flush = {
            let mut buffers = self.buffers.write();
            let buffer = buffers.entry(table_id).or_insert_with(|| WriteBuffer {
                rows: Vec::new(),
                last_flush: std::time::Instant::now(),
            });

            // SECURITY: Check if adding rows would exceed buffer limit
            let current_len = buffer.rows.len();
            if current_len.saturating_add(rows.len()) > MAX_BUFFER_SIZE {
                // Remove enough entries to make room (FIFO eviction)
                let target_len = MAX_BUFFER_SIZE.saturating_sub(rows.len());
                if current_len > target_len {
                    let remove_count = current_len - target_len;
                    buffer.rows.drain(0..remove_count);
                }
            }

            buffer.rows.extend(rows);
            buffer.rows.len() >= self.batch_size
        };

        // Auto-flush if batch size reached
        if should_flush {
            self.flush_table(table_id).await?;
        }

        Ok(())
    }

    /// Flush buffer for table
    pub async fn flush_table(&self, table_id: TableId) -> Result<usize> {
        let mut buffers = self.buffers.write();
        if let Some(buffer) = buffers.get_mut(&table_id) {
            let count = buffer.rows.len();
            if count > 0 {
                // In production, would write to storage
                buffer.rows.clear();
                buffer.last_flush = std::time::Instant::now();
            }
            Ok(count)
        } else {
            Ok(0)
        }
    }

    /// Flush all buffers
    pub async fn flush_all(&self) -> Result<HashMap<TableId, usize>> {
        let mut results = HashMap::new();
        let buffers = self.buffers.read();
        let table_ids: Vec<TableId> = buffers.keys().cloned().collect();
        drop(buffers);

        for table_id in table_ids {
            let count = self.flush_table(table_id).await?;
            results.insert(table_id, count);
        }

        Ok(results)
    }

    /// Start auto-flush background task
    /// SECURITY: Collect keys before iteration to prevent iterator invalidation
    pub async fn start_auto_flush(&self) {
        let buffers = self.buffers.clone();
        let interval_ms = self.flush_interval_ms;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(interval_ms));
            loop {
                interval.tick().await;
                // SECURITY: Collect keys before iteration to prevent iterator invalidation
                let table_ids: Vec<TableId> = {
                    let buffers = buffers.read();
                    buffers.keys().cloned().collect()
                };
                
                // Process each table separately
                for table_id in table_ids {
                    let should_flush = {
                        let mut buffers = buffers.write();
                        if let Some(buffer) = buffers.get_mut(&table_id) {
                            let should = buffer.last_flush.elapsed().as_millis() as u64 >= interval_ms
                                && !buffer.rows.is_empty();
                            if should {
                                buffer.rows.clear();
                                buffer.last_flush = std::time::Instant::now();
                            }
                            should
                        } else {
                            false
                        }
                    };
                    
                    // If flush was needed, trigger actual flush (in production)
                    if should_flush {
                        // In production, would call flush_table here
                        // For now, rows are already cleared above
                    }
                }
            }
        });
    }
}

/// High-concurrency write handler
pub struct ConcurrentWriteHandler {
    queues: Arc<RwLock<HashMap<TableId, Arc<SegQueue<Row>>>>>,
    workers: usize,
}

impl ConcurrentWriteHandler {
    pub fn new(workers: usize) -> Self {
        Self {
            queues: Arc::new(RwLock::new(HashMap::new())),
            workers,
        }
    }

    /// Write row (non-blocking, high-concurrency)
    /// SECURITY: Fixed race condition using double-check-lock pattern
    pub fn write(&self, table_id: TableId, row: Row) {
        // Fast path: check if queue exists
        {
            let queues = self.queues.read();
            if let Some(queue) = queues.get(&table_id) {
                queue.push(row);
                return; // Fast path - queue exists, we're done
            }
        }
        
        // Slow path: queue doesn't exist, need to create it
        // Use write lock to ensure atomic creation
        let mut queues = self.queues.write();
        
        // Double-check: another thread might have created it while we waited
        if let Some(queue) = queues.get(&table_id) {
            queue.push(row);
        } else {
            // Create new queue and insert the row
            let queue = Arc::new(SegQueue::new());
            queue.push(row);
            queues.insert(table_id, queue);
        }
    }

    /// Start worker threads
    /// SECURITY: Fixed unbounded loop to prevent DoS
    pub async fn start_workers(&self) {
        const MAX_ROWS_PER_ITERATION: usize = 100; // Prevent DoS
        
        for _ in 0..self.workers {
            let queues = self.queues.clone();
            tokio::spawn(async move {
                loop {
                    // Collect queue references to avoid holding lock across await
                    let queues_to_process: Vec<_> = {
                        let queues_read = queues.read();
                        queues_read.iter().map(|(id, queue)| (*id, queue.clone())).collect()
                    };
                    
                    for (table_id, queue) in queues_to_process {
                        // Process limited number of rows per iteration to prevent DoS
                        let mut processed = 0;
                        while processed < MAX_ROWS_PER_ITERATION {
                            if let Some(row) = queue.pop() {
                                // Process row (in production, would write to storage)
                                let _ = table_id;
                                let _ = row;
                                processed += 1;
                            } else {
                                break; // Queue empty
                            }
                        }
                    }
                    
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }
            });
        }
    }
}

