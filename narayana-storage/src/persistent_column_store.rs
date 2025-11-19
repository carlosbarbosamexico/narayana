// Real Persistent Columnar Storage
// Actually writes to disk with compression, indexing, and proper block management

use async_trait::async_trait;
use narayana_core::{Error, Result, schema::Schema, types::{TableId, CompressionType}, column::Column};
use parking_lot::RwLock;
use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};
use bytes::Bytes;
use bincode;

use crate::block::{Block, BlockMetadata};
use crate::writer::ColumnWriter;
use crate::reader::ColumnReader;
use crate::index::{Index, BTreeIndex};

/// Persistent columnar store that actually writes to disk
pub struct PersistentColumnStore {
    data_dir: PathBuf,
    tables: Arc<RwLock<HashMap<TableId, TableMetadata>>>,
    block_writer: ColumnWriter,
    block_reader: ColumnReader,
    indexes: Arc<RwLock<HashMap<(TableId, u32), Box<dyn Index + Send + Sync>>>>,
    compression: CompressionType,
}

#[derive(Clone)]
struct TableMetadata {
    schema: Schema,
    column_files: HashMap<u32, PathBuf>, // column_id -> file path
    block_metadata: HashMap<u32, Vec<BlockMetadata>>,
    row_count: usize,
}

impl PersistentColumnStore {
    pub fn new(data_dir: impl AsRef<Path>, compression: CompressionType) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        
        // Create data directory if it doesn't exist
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| Error::Storage(format!("Failed to create data directory: {}", e)))?;

        Ok(Self {
            data_dir,
            tables: Arc::new(RwLock::new(HashMap::new())),
            block_writer: ColumnWriter::new(compression, 64 * 1024), // 64KB blocks
            block_reader: ColumnReader::new(compression),
            indexes: Arc::new(RwLock::new(HashMap::new())),
            compression,
        })
    }

    fn table_dir(&self, table_id: &TableId) -> PathBuf {
        self.data_dir.join(format!("table_{}", table_id.0))
    }

    fn column_file_path(&self, table_id: &TableId, column_id: u32, block_id: u64) -> PathBuf {
        self.table_dir(table_id).join(format!("col_{}_block_{}.dat", column_id, block_id))
    }

    fn metadata_file_path(&self, table_id: &TableId) -> PathBuf {
        self.table_dir(table_id).join("metadata.bin")
    }

    async fn save_table_metadata(&self, table_id: &TableId, metadata: &TableMetadata) -> Result<()> {
        let metadata_path = self.metadata_file_path(table_id);
        if let Some(parent) = metadata_path.parent() {
            fs::create_dir_all(parent).await
                .map_err(|e| Error::Storage(format!("Failed to create table directory: {}", e)))?;
        }

        // Serialize metadata (excluding file paths which are derived)
        let serializable = SerializableTableMetadata {
            schema: metadata.schema.clone(),
            block_metadata: metadata.block_metadata.clone(),
            row_count: metadata.row_count,
        };

        let bytes = bincode::serialize(&serializable)
            .map_err(|e| Error::Serialization(format!("Failed to serialize metadata: {}", e)))?;

        // ATOMIC WRITE: Write to temp file, sync, then rename
        let temp_path = metadata_path.with_extension("bin.tmp");
        {
            let mut file = fs::File::create(&temp_path).await
                .map_err(|e| Error::Storage(format!("Failed to create metadata temp file: {}", e)))?;
            file.write_all(&bytes).await
                .map_err(|e| {
                    // Cleanup temp file on error
                    let _ = std::fs::remove_file(&temp_path);
                    Error::Storage(format!("Failed to write metadata: {}", e))
                })?;
            // CRITICAL: Sync to ensure metadata is on disk
            file.sync_all().await
                .map_err(|e| {
                    // Cleanup temp file on error
                    let _ = std::fs::remove_file(&temp_path);
                    Error::Storage(format!("Failed to sync metadata: {}", e))
                })?;
        }
        
        // Atomic rename
        fs::rename(&temp_path, &metadata_path).await
            .map_err(|e| {
                // Cleanup temp file on error
                let _ = std::fs::remove_file(&temp_path);
                Error::Storage(format!("Failed to rename metadata file: {}", e))
            })?;

        Ok(())
    }

    async fn load_table_metadata(&self, table_id: &TableId) -> Result<Option<TableMetadata>> {
        let metadata_path = self.metadata_file_path(table_id);
        
        if !metadata_path.exists() {
            return Ok(None);
        }

        let bytes = fs::read(&metadata_path).await
            .map_err(|e| Error::Storage(format!("Failed to read metadata: {}", e)))?;

        // SECURITY: Handle deserialization errors gracefully - return None if metadata is corrupted
        let serializable: SerializableTableMetadata = match bincode::deserialize(&bytes) {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to deserialize metadata for table {}: {}. Skipping corrupted metadata.", table_id.0, e);
                return Ok(None); // Return None instead of error to allow startup
            }
        };

        // Reconstruct column files from block metadata
        let mut column_files = HashMap::new();
        for (column_id, blocks) in &serializable.block_metadata {
            if let Some(first_block) = blocks.first() {
                let file_path = self.column_file_path(table_id, *column_id, first_block.block_id);
                column_files.insert(*column_id, file_path);
            }
        }

        Ok(Some(TableMetadata {
            schema: serializable.schema,
            column_files,
            block_metadata: serializable.block_metadata,
            row_count: serializable.row_count,
        }))
    }

    async fn write_block_to_disk(&self, table_id: &TableId, column_id: u32, block: &Block, metadata: &BlockMetadata) -> Result<()> {
        let file_path = self.column_file_path(table_id, column_id, metadata.block_id);
        
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).await
                .map_err(|e| Error::Storage(format!("Failed to create directory: {}", e)))?;
        }

        // ATOMIC WRITE: Write to temp file first, then rename (prevents corruption)
        let temp_path = file_path.with_extension("tmp");
        
        // Write block data to temp file
        {
            let mut file = fs::File::create(&temp_path).await
                .map_err(|e| Error::Storage(format!("Failed to create temp file: {}", e)))?;
            file.write_all(&block.data).await
                .map_err(|e| {
                    // Cleanup temp file on error
                    let _ = std::fs::remove_file(&temp_path);
                    Error::Storage(format!("Failed to write block data: {}", e))
                })?;
            // CRITICAL: Sync to ensure data is on disk before rename
            file.sync_all().await
                .map_err(|e| {
                    // Cleanup temp file on error
                    let _ = std::fs::remove_file(&temp_path);
                    Error::Storage(format!("Failed to sync block data: {}", e))
                })?;
        }
        
        // Atomic rename (POSIX guarantees this is atomic)
        fs::rename(&temp_path, &file_path).await
            .map_err(|e| {
                // Cleanup temp file on error
                let _ = std::fs::remove_file(&temp_path);
                Error::Storage(format!("Failed to rename temp file: {}", e))
            })?;

        // Write block metadata with atomic write
        let metadata_path = file_path.with_extension("meta");
        let metadata_temp_path = metadata_path.with_extension("meta.tmp");
        let metadata_bytes = bincode::serialize(metadata)
            .map_err(|e| Error::Serialization(format!("Failed to serialize block metadata: {}", e)))?;
        
        {
            let mut file = fs::File::create(&metadata_temp_path).await
                .map_err(|e| Error::Storage(format!("Failed to create metadata temp file: {}", e)))?;
            file.write_all(&metadata_bytes).await
                .map_err(|e| {
                    // Cleanup temp file on error
                    let _ = std::fs::remove_file(&metadata_temp_path);
                    Error::Storage(format!("Failed to write metadata: {}", e))
                })?;
            // CRITICAL: Sync metadata to disk
            file.sync_all().await
                .map_err(|e| {
                    // Cleanup temp file on error
                    let _ = std::fs::remove_file(&metadata_temp_path);
                    Error::Storage(format!("Failed to sync metadata: {}", e))
                })?;
        }
        
        // Atomic rename for metadata
        fs::rename(&metadata_temp_path, &metadata_path).await
            .map_err(|e| {
                // Cleanup temp file on error
                let _ = std::fs::remove_file(&metadata_temp_path);
                Error::Storage(format!("Failed to rename metadata temp file: {}", e))
            })?;

        Ok(())
    }

    async fn read_block_from_disk(&self, table_id: &TableId, column_id: u32, block_id: u64) -> Result<Option<(Block, BlockMetadata)>> {
        let file_path = self.column_file_path(table_id, column_id, block_id);
        
        if !file_path.exists() {
            return Ok(None);
        }

        // Read block data
        let data = fs::read(&file_path).await
            .map_err(|e| Error::Storage(format!("Failed to read block: {}", e)))?;

        // Read block metadata
        let metadata_path = file_path.with_extension("meta");
        let metadata_bytes = fs::read(&metadata_path).await
            .map_err(|e| Error::Storage(format!("Failed to read block metadata: {}", e)))?;
        let metadata: BlockMetadata = bincode::deserialize(&metadata_bytes)
            .map_err(|e| Error::Deserialization(format!("Failed to deserialize block metadata: {}", e)))?;

        let block = Block {
            column_id,
            data: Bytes::from(data),
            row_count: metadata.row_count,
            data_type: metadata.data_type.clone(),
            compression: metadata.compression,
            uncompressed_size: metadata.uncompressed_size,
            compressed_size: metadata.compressed_size,
        };

        Ok(Some((block, metadata)))
    }

    async fn update_index(&self, table_id: TableId, column_id: u32, block_metadata: &BlockMetadata) -> Result<()> {
        let key = (table_id, column_id);
        let mut indexes = self.indexes.write();
        
        if !indexes.contains_key(&key) {
            indexes.insert(key.clone(), Box::new(BTreeIndex::new()));
        }

        if let Some(index) = indexes.get_mut(&key) {
            // Index by min/max values for range queries
            if let Some(ref min_val) = block_metadata.min_value {
                index.insert(min_val.clone(), block_metadata.block_id)?;
            }
        }

        Ok(())
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SerializableTableMetadata {
    schema: Schema,
    block_metadata: HashMap<u32, Vec<BlockMetadata>>,
    row_count: usize,
}

#[async_trait]
impl crate::column_store::ColumnStore for PersistentColumnStore {
    async fn create_table(&self, table_id: TableId, schema: Schema) -> Result<()> {
        let metadata = {
            let mut tables = self.tables.write();
            if tables.contains_key(&table_id) {
                return Err(Error::Storage(format!("Table {} already exists", table_id.0)));
            }

            let metadata = TableMetadata {
                schema: schema.clone(),
                column_files: HashMap::new(),
                block_metadata: HashMap::new(),
                row_count: 0,
            };

            tables.insert(table_id.clone(), metadata.clone());
            metadata
        };
        self.save_table_metadata(&table_id, &metadata).await?;

        info!("Created persistent table {}", table_id.0);
        Ok(())
    }

    async fn write_columns(&self, table_id: TableId, columns: Vec<Column>) -> Result<()> {
        // Prepare all blocks first
        let mut all_blocks_data = Vec::new();
        for (idx, column) in columns.into_iter().enumerate() {
            let column_id = idx as u32;
            let blocks = self.block_writer.write_column(&column, column_id)?;
            all_blocks_data.push((column_id, blocks, column.len()));
        }
        
        // Process each column
        for (column_id, blocks, column_len) in all_blocks_data {
            for (block, metadata) in blocks {
                // Write to disk (outside of lock)
                self.write_block_to_disk(&table_id, column_id, &block, &metadata).await?;
                
                // Update table metadata (acquire lock)
                {
                    let mut tables = self.tables.write();
                    let table = tables
                        .get_mut(&table_id)
                        .ok_or_else(|| Error::Storage(format!("Table {} not found", table_id.0)))?;
                    
                    table.block_metadata
                        .entry(column_id)
                        .or_insert_with(Vec::new)
                        .push(metadata.clone());
                    
                    // Update column file path
                    if let Some(first_block) = table.block_metadata.get(&column_id)
                        .and_then(|blocks| blocks.first()) {
                        let file_path = self.column_file_path(&table_id, column_id, first_block.block_id);
                        table.column_files.insert(column_id, file_path);
                    }
                    
                    // Update row count
                    table.row_count = table.row_count.max(column_len);
                }
                
                // Update index (outside of lock)
                self.update_index(table_id.clone(), column_id, &metadata).await?;
            }
        }
        
        // Save updated metadata (outside of lock)
        {
            let metadata = {
                let tables = self.tables.read();
                tables.get(&table_id)
                    .ok_or_else(|| Error::Storage(format!("Table {} not found", table_id.0)))?
                    .clone()
            };
            self.save_table_metadata(&table_id, &metadata).await?;
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
        // Collect block metadata first (inside lock)
        let blocks_to_read: Vec<(u32, Vec<BlockMetadata>)> = {
            let tables = self.tables.read();
            let table = tables
                .get(&table_id)
                .ok_or_else(|| Error::Storage(format!("Table {} not found", table_id.0)))?;

            column_ids.iter()
                .filter_map(|&column_id| {
                    table.block_metadata.get(&column_id)
                        .map(|blocks| {
                            let relevant_blocks: Vec<BlockMetadata> = blocks.iter()
                                .filter(|block_meta| {
                                    let row_end = block_meta.row_start + block_meta.row_count;
                                    row_start < row_end && (row_start + row_count) > block_meta.row_start
                                })
                                .cloned()
                                .collect();
                            (column_id, relevant_blocks)
                        })
                })
                .collect()
        };

        // Read blocks from disk (outside of lock)
        let mut result = Vec::new();
        for (column_id, blocks_metadata) in blocks_to_read {
            let mut column_data: Option<Column> = None;
            
            for block_meta in blocks_metadata {
                // Read block from disk
                if let Some((block, _)) = self.read_block_from_disk(&table_id, column_id, block_meta.block_id).await? {
                    // Decompress and read column data
                    let decompressed = self.block_reader.read_block(&block)?;
                    
                    // Merge with existing column data
                    column_data = match column_data.take() {
                        None => Some(decompressed),
                        Some(existing) => {
                            match existing.append(&decompressed) {
                                Ok(merged) => Some(merged),
                                Err(e) => {
                                    warn!("Failed to append column data: {}", e);
                                    Some(existing) // Keep existing on error
                                }
                            }
                        }
                    };
                }
            }
            
            if let Some(col) = column_data {
                // Slice to requested range
                match col.slice(row_start, row_count) {
                    Ok(sliced) => result.push(sliced),
                    Err(e) => {
                        warn!("Failed to slice column: {}", e);
                        // Return full column if slice fails
                        result.push(col);
                    }
                }
            }
        }

        Ok(result)
    }

    async fn get_schema(&self, table_id: TableId) -> Result<Schema> {
        // Try to load from memory first
        {
            let tables = self.tables.read();
            if let Some(table) = tables.get(&table_id) {
                return Ok(table.schema.clone());
            }
        }

        // Load from disk if not in memory
        if let Some(metadata) = self.load_table_metadata(&table_id).await? {
            let mut tables = self.tables.write();
            tables.insert(table_id.clone(), metadata.clone());
            Ok(metadata.schema)
        } else {
            Err(Error::Storage(format!("Table {} not found", table_id.0)))
        }
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
        // Remove from tables first (inside lock)
        {
            let mut tables = self.tables.write();
            if !tables.contains_key(&table_id) {
                return Err(Error::Storage(format!("Table {} not found", table_id.0)));
            }
            tables.remove(&table_id);
        }
        
        // Delete table directory (outside of lock)
        let table_dir = self.table_dir(&table_id);
        if table_dir.exists() {
            fs::remove_dir_all(&table_dir).await
                .map_err(|e| Error::Storage(format!("Failed to delete table directory: {}", e)))?;
        }

        // Remove indexes (acquire lock)
        {
            let mut indexes = self.indexes.write();
            indexes.retain(|(tid, _), _| *tid != table_id);
        }

        info!("Deleted persistent table {}", table_id.0);
        Ok(())
    }
}

impl PersistentColumnStore {
    /// Load all tables from disk on startup
    pub async fn load_all_tables(&self) -> Result<()> {
        if !self.data_dir.exists() {
            return Ok(());
        }

        // CRITICAL: Clean up any orphaned temp files from previous crashes
        self.cleanup_temp_files().await?;

        // Scan for table directories
        let mut entries = fs::read_dir(&self.data_dir).await
            .map_err(|e| Error::Storage(format!("Failed to read data directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| Error::Storage(format!("Failed to read directory entry: {}", e)))? {
            
            let path = entry.path();
            if path.is_dir() {
                let dir_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .ok_or_else(|| Error::Storage("Invalid directory name".to_string()))?;
                
                if dir_name.starts_with("table_") {
                    let table_id_str = dir_name.strip_prefix("table_")
                        .ok_or_else(|| Error::Storage("Invalid table directory name".to_string()))?;
                    let table_id = table_id_str.parse::<u64>()
                        .map_err(|_| Error::Storage("Invalid table ID".to_string()))
                        .map(|id| TableId(id))?;
                    
                    // SECURITY: Handle deserialization errors gracefully - skip corrupted tables
                    match self.load_table_metadata(&table_id).await {
                        Ok(Some(metadata)) => {
                            let mut tables = self.tables.write();
                            tables.insert(table_id, metadata);
                        }
                        Ok(None) => {
                            // No metadata file, skip
                        }
                        Err(e) => {
                            // Log error but continue - don't fail startup due to corrupted metadata
                            warn!("Warning: Failed to load metadata for table {}: {}. Skipping.", table_id.0, e);
                        }
                    }
                }
            }
        }

        info!("Loaded {} tables from disk", self.tables.read().len());
        Ok(())
    }

    /// Clean up orphaned temp files from previous crashes
    async fn cleanup_temp_files(&self) -> Result<()> {
        if !self.data_dir.exists() {
            return Ok(());
        }

        let mut cleaned = 0;
        let mut entries = fs::read_dir(&self.data_dir).await
            .map_err(|e| Error::Storage(format!("Failed to read data directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| Error::Storage(format!("Failed to read directory entry: {}", e)))? {
            
            let path = entry.path();
            if path.is_dir() {
                // Scan table directories for temp files
                if let Ok(mut table_entries) = fs::read_dir(&path).await {
                    while let Ok(Some(table_entry)) = table_entries.next_entry().await {
                        let file_path = table_entry.path();
                        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                            if ext == "tmp" || ext == "meta.tmp" || ext == "bin.tmp" {
                                // Orphaned temp file - remove it
                                if let Err(e) = fs::remove_file(&file_path).await {
                                    warn!("Failed to remove orphaned temp file {:?}: {}", file_path, e);
                                } else {
                                    cleaned += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        if cleaned > 0 {
            info!("Cleaned up {} orphaned temp files", cleaned);
        }

        Ok(())
    }
}

