use narayana_core::{types::CompressionType, schema::DataType};
use serde::{Deserialize, Serialize};
use bytes::Bytes;

/// A block of columnar data
// Note: Block is not serializable because Bytes doesn't implement Serialize/Deserialize
// Use BlockMetadata for serialization instead
#[derive(Debug, Clone)]
pub struct Block {
    pub column_id: u32,
    pub data: Bytes,
    pub row_count: usize,
    pub data_type: DataType,
    pub compression: CompressionType,
    pub uncompressed_size: usize,
    pub compressed_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockMetadata {
    // Note: Block.data (Bytes) is not serializable - skip in serialization
    pub block_id: u64,
    pub column_id: u32,
    pub row_start: usize,
    pub row_count: usize,
    pub data_type: DataType,
    pub compression: CompressionType,
    pub uncompressed_size: usize,
    pub compressed_size: usize,
    pub min_value: Option<Vec<u8>>,
    pub max_value: Option<Vec<u8>>,
    pub null_count: usize,
}

impl BlockMetadata {
    pub fn compression_ratio(&self) -> f64 {
        if self.uncompressed_size == 0 {
            return 1.0;
        }
        self.compressed_size as f64 / self.uncompressed_size as f64
    }
}

