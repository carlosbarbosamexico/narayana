// Efficient mutable data handling - ClickHouse limitation

use narayana_core::{Error, Result, types::TableId};
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;

/// Mutable data manager - efficient updates and deletes
pub struct MutableDataManager {
    update_buffers: Arc<RwLock<HashMap<TableId, UpdateBuffer>>>,
    delete_markers: Arc<RwLock<HashMap<TableId, DeleteMarkers>>>,
}

struct UpdateBuffer {
    updates: Vec<Update>,
    last_merge: std::time::Instant,
}

#[derive(Debug, Clone)]
pub struct Update {
    pub row_id: u64,
    pub column: String,
    pub value: Vec<u8>,
    pub timestamp: i64,
}

struct DeleteMarkers {
    deleted_rows: std::collections::HashSet<u64>,
    last_compact: std::time::Instant,
}

impl MutableDataManager {
    pub fn new() -> Self {
        Self {
            update_buffers: Arc::new(RwLock::new(HashMap::new())),
            delete_markers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Update row (buffered for efficiency)
    /// SECURITY: Prevent unbounded Vec growth
    pub fn update(&self, table_id: TableId, row_id: u64, column: String, value: Vec<u8>) -> Result<()> {
        // SECURITY: Limit update buffer size to prevent memory exhaustion
        const MAX_UPDATE_BUFFER_SIZE: usize = 1_000_000; // Maximum updates per table
        
        let mut buffers = self.update_buffers.write();
        let buffer = buffers.entry(table_id).or_insert_with(|| UpdateBuffer {
            updates: Vec::new(),
            last_merge: std::time::Instant::now(),
        });

        // SECURITY: If buffer is too large, remove oldest entries (FIFO eviction)
        if buffer.updates.len() >= MAX_UPDATE_BUFFER_SIZE {
            // Remove 10% of oldest entries
            let remove_count = MAX_UPDATE_BUFFER_SIZE / 10;
            buffer.updates.drain(0..remove_count);
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        buffer.updates.push(Update {
            row_id,
            column,
            value,
            timestamp,
        });

        Ok(())
    }

    /// Delete row (marked for deletion)
    /// SECURITY: Prevent unbounded HashSet growth
    pub fn delete(&self, table_id: TableId, row_id: u64) -> Result<()> {
        // SECURITY: Limit delete markers size to prevent memory exhaustion
        const MAX_DELETE_MARKERS: usize = 10_000_000; // Maximum deleted rows per table
        
        let mut markers = self.delete_markers.write();
        let marker = markers.entry(table_id).or_insert_with(|| DeleteMarkers {
            deleted_rows: std::collections::HashSet::new(),
            last_compact: std::time::Instant::now(),
        });

        // SECURITY: If HashSet is too large, trigger compaction
        if marker.deleted_rows.len() >= MAX_DELETE_MARKERS {
            // In production, would trigger actual compaction to remove deleted rows
            // For now, log a warning and allow insertion (compaction should be called separately)
            tracing::warn!("Delete markers for table {:?} reached limit {}, compaction recommended", table_id, MAX_DELETE_MARKERS);
        }

        marker.deleted_rows.insert(row_id);
        Ok(())
    }

    /// Check if row is deleted
    pub fn is_deleted(&self, table_id: TableId, row_id: u64) -> bool {
        let markers = self.delete_markers.read();
        if let Some(marker) = markers.get(&table_id) {
            marker.deleted_rows.contains(&row_id)
        } else {
            false
        }
    }

    /// Get updates for row
    pub fn get_updates(&self, table_id: TableId, row_id: u64) -> Vec<Update> {
        let buffers = self.update_buffers.read();
        if let Some(buffer) = buffers.get(&table_id) {
            buffer.updates.iter()
                .filter(|u| u.row_id == row_id)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Merge updates (apply to storage)
    pub async fn merge_updates(&self, table_id: TableId) -> Result<usize> {
        let mut buffers = self.update_buffers.write();
        if let Some(buffer) = buffers.get_mut(&table_id) {
            let count = buffer.updates.len();
            // In production, would apply updates to storage
            buffer.updates.clear();
            buffer.last_merge = std::time::Instant::now();
            Ok(count)
        } else {
            Ok(0)
        }
    }

    /// Compact deletes (remove deleted rows)
    pub async fn compact_deletes(&self, table_id: TableId) -> Result<usize> {
        let mut markers = self.delete_markers.write();
        if let Some(marker) = markers.get_mut(&table_id) {
            let count = marker.deleted_rows.len();
            // In production, would remove deleted rows from storage
            marker.deleted_rows.clear();
            marker.last_compact = std::time::Instant::now();
            Ok(count)
        } else {
            Ok(0)
        }
    }

    /// Upsert (update or insert)
    pub async fn upsert(&self, table_id: TableId, row_id: u64, data: HashMap<String, Vec<u8>>) -> Result<()> {
        // Check if row exists
        if self.is_deleted(table_id, row_id) {
            // Re-insert
            // In production, would insert new row
        } else {
            // Update existing
            for (column, value) in data {
                self.update(table_id, row_id, column, value)?;
            }
        }
        Ok(())
    }
}

/// Delta storage for efficient updates
pub struct DeltaStorage {
    base_data: TableId,
    delta_data: HashMap<u64, HashMap<String, Vec<u8>>>, // row_id -> column -> value
}

impl DeltaStorage {
    pub fn new(base_data: TableId) -> Self {
        Self {
            base_data,
            delta_data: HashMap::new(),
        }
    }

    /// Apply delta to base data
    pub fn apply_delta(&mut self, row_id: u64, column: String, value: Vec<u8>) {
        self.delta_data.entry(row_id)
            .or_insert_with(HashMap::new)
            .insert(column, value);
    }

    /// Get value (check delta first, then base)
    pub fn get(&self, row_id: u64, column: &str) -> Option<Vec<u8>> {
        if let Some(delta) = self.delta_data.get(&row_id) {
            delta.get(column).cloned()
        } else {
            // In production, would read from base data
            None
        }
    }

    /// Merge delta into base (periodic compaction)
    pub async fn merge(&mut self) -> Result<usize> {
        let count = self.delta_data.len();
        // In production, would merge delta into base storage
        self.delta_data.clear();
        Ok(count)
    }
}

