use async_trait::async_trait;
use narayana_core::{Error, Result, schema::Schema, types::TableId, column::Column};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

use crate::block::BlockMetadata;
use crate::writer::ColumnWriter;
use crate::reader::ColumnReader;

#[async_trait]
pub trait ColumnStore: Send + Sync {
    /// Create a new table with the given schema
    async fn create_table(&self, table_id: TableId, schema: Schema) -> Result<()>;

    /// Write columns to a table
    async fn write_columns(&self, table_id: TableId, columns: Vec<Column>) -> Result<()>;

    /// Read columns from a table
    async fn read_columns(
        &self,
        table_id: TableId,
        column_ids: Vec<u32>,
        row_start: usize,
        row_count: usize,
    ) -> Result<Vec<Column>>;

    /// Get table schema
    async fn get_schema(&self, table_id: TableId) -> Result<Schema>;

    /// Get block metadata for a column
    async fn get_block_metadata(
        &self,
        table_id: TableId,
        column_id: u32,
    ) -> Result<Vec<BlockMetadata>>;

    /// Delete a table
    async fn delete_table(&self, table_id: TableId) -> Result<()>;
}

pub struct InMemoryColumnStore {
    tables: Arc<RwLock<HashMap<TableId, TableMetadata>>>,
}

struct TableMetadata {
    schema: Schema,
    columns: HashMap<u32, Vec<Column>>,
    block_metadata: HashMap<u32, Vec<BlockMetadata>>,
}

impl InMemoryColumnStore {
    pub fn new() -> Self {
        Self {
            tables: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl ColumnStore for InMemoryColumnStore {
    async fn create_table(&self, table_id: TableId, schema: Schema) -> Result<()> {
        let mut tables = self.tables.write();
        if tables.contains_key(&table_id) {
            return Err(Error::Storage(format!("Table {} already exists", table_id.0)));
        }

        tables.insert(
            table_id,
            TableMetadata {
                schema,
                columns: HashMap::new(),
                block_metadata: HashMap::new(),
            },
        );

        info!("Created table {}", table_id.0);
        Ok(())
    }

    async fn write_columns(&self, table_id: TableId, columns: Vec<Column>) -> Result<()> {
        let mut tables = self.tables.write();
        let table = tables
            .get_mut(&table_id)
            .ok_or_else(|| Error::Storage(format!("Table {} not found", table_id.0)))?;

        // Optimized: batch all column writes, avoid repeated HashMap lookups
        for (idx, column) in columns.into_iter().enumerate() {
            let column_id = idx as u32;
            // Use get_mut instead of entry for better performance on hot path
            if let Some(col_vec) = table.columns.get_mut(&column_id) {
                col_vec.push(column);
            } else {
                table.columns.insert(column_id, vec![column]);
            }
        }

        Ok(())
    }

    async fn read_columns(
        &self,
        table_id: TableId,
        column_ids: Vec<u32>,
        row_start: usize,
        row_count: usize,
    ) -> Result<Vec<Column>> {
        let tables = self.tables.read();
        let table = tables
            .get(&table_id)
            .ok_or_else(|| Error::Storage(format!("Table {} not found", table_id.0)))?;

        let mut result = Vec::new();
        for column_id in column_ids {
            if let Some(columns) = table.columns.get(&column_id) {
                // Optimized merge: pre-allocate and copy directly (no repeated clones!)
                if columns.is_empty() {
                    continue;
                }
                
                // Calculate total size first
                let total_size: usize = columns.iter().map(|c| c.len()).sum();
                
                // Merge efficiently based on column type
                let merged_column = match &columns[0] {
                    Column::Int64(_) => {
                        let mut merged = Vec::with_capacity(total_size);
                        for col in columns.iter() {
                            if let Column::Int64(vals) = col {
                                merged.extend_from_slice(vals);
                            }
                        }
                        Column::Int64(merged)
                    }
                    Column::Int32(_) => {
                        let mut merged = Vec::with_capacity(total_size);
                        for col in columns.iter() {
                            if let Column::Int32(vals) = col {
                                merged.extend_from_slice(vals);
                            }
                        }
                        Column::Int32(merged)
                    }
                    Column::Int16(_) => {
                        let mut merged = Vec::with_capacity(total_size);
                        for col in columns.iter() {
                            if let Column::Int16(vals) = col {
                                merged.extend_from_slice(vals);
                            }
                        }
                        Column::Int16(merged)
                    }
                    Column::Int8(_) => {
                        let mut merged = Vec::with_capacity(total_size);
                        for col in columns.iter() {
                            if let Column::Int8(vals) = col {
                                merged.extend_from_slice(vals);
                            }
                        }
                        Column::Int8(merged)
                    }
                    Column::UInt64(_) => {
                        let mut merged = Vec::with_capacity(total_size);
                        for col in columns.iter() {
                            if let Column::UInt64(vals) = col {
                                merged.extend_from_slice(vals);
                            }
                        }
                        Column::UInt64(merged)
                    }
                    Column::UInt32(_) => {
                        let mut merged = Vec::with_capacity(total_size);
                        for col in columns.iter() {
                            if let Column::UInt32(vals) = col {
                                merged.extend_from_slice(vals);
                            }
                        }
                        Column::UInt32(merged)
                    }
                    Column::UInt16(_) => {
                        let mut merged = Vec::with_capacity(total_size);
                        for col in columns.iter() {
                            if let Column::UInt16(vals) = col {
                                merged.extend_from_slice(vals);
                            }
                        }
                        Column::UInt16(merged)
                    }
                    Column::UInt8(_) => {
                        let mut merged = Vec::with_capacity(total_size);
                        for col in columns.iter() {
                            if let Column::UInt8(vals) = col {
                                merged.extend_from_slice(vals);
                            }
                        }
                        Column::UInt8(merged)
                    }
                    Column::Float64(_) => {
                        let mut merged = Vec::with_capacity(total_size);
                        for col in columns.iter() {
                            if let Column::Float64(vals) = col {
                                merged.extend_from_slice(vals);
                            }
                        }
                        Column::Float64(merged)
                    }
                    Column::Float32(_) => {
                        let mut merged = Vec::with_capacity(total_size);
                        for col in columns.iter() {
                            if let Column::Float32(vals) = col {
                                merged.extend_from_slice(vals);
                            }
                        }
                        Column::Float32(merged)
                    }
                    Column::Boolean(_) => {
                        let mut merged = Vec::with_capacity(total_size);
                        for col in columns.iter() {
                            if let Column::Boolean(vals) = col {
                                merged.extend_from_slice(vals);
                            }
                        }
                        Column::Boolean(merged)
                    }
                    Column::String(_) => {
                        let mut merged = Vec::with_capacity(total_size);
                        for col in columns.iter() {
                            if let Column::String(vals) = col {
                                merged.extend_from_slice(vals);
                            }
                        }
                        Column::String(merged)
                    }
                    Column::Binary(_) => {
                        let mut merged = Vec::with_capacity(total_size);
                        for col in columns.iter() {
                            if let Column::Binary(vals) = col {
                                merged.extend_from_slice(vals);
                            }
                        }
                        Column::Binary(merged)
                    }
                    Column::Timestamp(_) => {
                        let mut merged = Vec::with_capacity(total_size);
                        for col in columns.iter() {
                            if let Column::Timestamp(vals) = col {
                                merged.extend_from_slice(vals);
                            }
                        }
                        Column::Timestamp(merged)
                    }
                    Column::Date(_) => {
                        let mut merged = Vec::with_capacity(total_size);
                        for col in columns.iter() {
                            if let Column::Date(vals) = col {
                                merged.extend_from_slice(vals);
                            }
                        }
                        Column::Date(merged)
                    }
                };
                
                // Slice to requested range
                if row_start > 0 || row_count < merged_column.len() {
                    match merged_column.slice(row_start, row_count) {
                        Ok(sliced) => result.push(sliced),
                        Err(e) => {
                            warn!("Failed to slice column: {}", e);
                            result.push(merged_column); // Return full column if slice fails
                        }
                    }
                } else {
                    result.push(merged_column);
                }
            }
        }

        Ok(result)
    }

    async fn get_schema(&self, table_id: TableId) -> Result<Schema> {
        let tables = self.tables.read();
        let table = tables
            .get(&table_id)
            .ok_or_else(|| Error::Storage(format!("Table {} not found", table_id.0)))?;

        Ok(table.schema.clone())
    }

    async fn get_block_metadata(
        &self,
        table_id: TableId,
        column_id: u32,
    ) -> Result<Vec<BlockMetadata>> {
        let tables = self.tables.read();
        let table = tables
            .get(&table_id)
            .ok_or_else(|| Error::Storage(format!("Table {} not found", table_id.0)))?;

        Ok(table
            .block_metadata
            .get(&column_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn delete_table(&self, table_id: TableId) -> Result<()> {
        let mut tables = self.tables.write();
        tables.remove(&table_id);
        info!("Deleted table {}", table_id.0);
        Ok(())
    }
}

