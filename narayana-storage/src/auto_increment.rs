// Auto-increment feature - ClickHouse missing feature

use narayana_core::{Error, Result, types::TableId};
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Auto-increment manager
pub struct AutoIncrementManager {
    sequences: Arc<RwLock<HashMap<String, Arc<AtomicU64>>>>,
}

impl AutoIncrementManager {
    pub fn new() -> Self {
        Self {
            sequences: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create auto-increment sequence
    pub fn create_sequence(&self, name: String, start: u64) {
        let mut sequences = self.sequences.write();
        sequences.insert(name, Arc::new(AtomicU64::new(start)));
    }

    /// Get next value from sequence
    pub fn next(&self, name: &str) -> Result<u64> {
        let sequences = self.sequences.read();
        if let Some(seq) = sequences.get(name) {
            Ok(seq.fetch_add(1, Ordering::SeqCst))
        } else {
            Err(Error::Storage(format!("Sequence '{}' not found", name)))
        }
    }

    /// Get current value (without incrementing)
    pub fn current(&self, name: &str) -> Result<u64> {
        let sequences = self.sequences.read();
        if let Some(seq) = sequences.get(name) {
            Ok(seq.load(Ordering::SeqCst))
        } else {
            Err(Error::Storage(format!("Sequence '{}' not found", name)))
        }
    }

    /// Set sequence value
    pub fn set(&self, name: &str, value: u64) -> Result<()> {
        let sequences = self.sequences.read();
        if let Some(seq) = sequences.get(name) {
            seq.store(value, Ordering::SeqCst);
            Ok(())
        } else {
            Err(Error::Storage(format!("Sequence '{}' not found", name)))
        }
    }

    /// Reset sequence
    pub fn reset(&self, name: &str, start: u64) -> Result<()> {
        self.set(name, start)
    }
}

/// Table-level auto-increment
pub struct TableAutoIncrement {
    manager: AutoIncrementManager,
}

impl TableAutoIncrement {
    pub fn new() -> Self {
        Self {
            manager: AutoIncrementManager::new(),
        }
    }

    /// Enable auto-increment for table column
    pub fn enable(&self, table_id: TableId, column: &str, start: u64) {
        let sequence_name = format!("table_{}_{}", table_id.0, column);
        self.manager.create_sequence(sequence_name, start);
    }

    /// Get next value for table column
    pub fn next(&self, table_id: TableId, column: &str) -> Result<u64> {
        let sequence_name = format!("table_{}_{}", table_id.0, column);
        self.manager.next(&sequence_name)
    }

    /// Get current value for table column
    pub fn current(&self, table_id: TableId, column: &str) -> Result<u64> {
        let sequence_name = format!("table_{}_{}", table_id.0, column);
        self.manager.current(&sequence_name)
    }
}

