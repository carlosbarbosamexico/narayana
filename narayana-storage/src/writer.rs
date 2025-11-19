use narayana_core::{Error, Result, column::Column, schema::DataType, types::CompressionType};
use crate::block::{Block, BlockMetadata};
use crate::compression::{create_compressor, Compressor};
use bytes::{Bytes, BytesMut};
use bincode;

pub struct ColumnWriter {
    compression: CompressionType,
    block_size: usize,
}

impl ColumnWriter {
    pub fn new(compression: CompressionType, block_size: usize) -> Self {
        Self {
            compression,
            block_size,
        }
    }

    pub fn write_column(&self, column: &Column, column_id: u32) -> Result<Vec<(Block, BlockMetadata)>> {
        let compressor = create_compressor(self.compression);
        let mut blocks = Vec::new();
        let mut row_offset = 0;

        match column {
            Column::Int8(data) => {
                let chunks = data.chunks(self.block_size);
                for chunk in chunks {
                    let (block, metadata) = self.write_chunk(
                        chunk,
                        &*compressor,
                        column_id,
                        row_offset,
                        DataType::Int8,
                    )?;
                    blocks.push((block, metadata));
                    row_offset += chunk.len();
                }
            }
            Column::Int32(data) => {
                let chunks = data.chunks(self.block_size);
                for chunk in chunks {
                    let (block, metadata) = self.write_chunk(
                        chunk,
                        &*compressor,
                        column_id,
                        row_offset,
                        DataType::Int32,
                    )?;
                    blocks.push((block, metadata));
                    row_offset += chunk.len();
                }
            }
            Column::Int64(data) => {
                let chunks = data.chunks(self.block_size);
                for chunk in chunks {
                    let (block, metadata) = self.write_chunk(
                        chunk,
                        &*compressor,
                        column_id,
                        row_offset,
                        DataType::Int64,
                    )?;
                    blocks.push((block, metadata));
                    row_offset += chunk.len();
                }
            }
            Column::UInt64(data) => {
                let chunks = data.chunks(self.block_size);
                for chunk in chunks {
                    let (block, metadata) = self.write_chunk(
                        chunk,
                        &*compressor,
                        column_id,
                        row_offset,
                        DataType::UInt64,
                    )?;
                    blocks.push((block, metadata));
                    row_offset += chunk.len();
                }
            }
            Column::Float64(data) => {
                let chunks = data.chunks(self.block_size);
                for chunk in chunks {
                    let (block, metadata) = self.write_chunk(
                        chunk,
                        &*compressor,
                        column_id,
                        row_offset,
                        DataType::Float64,
                    )?;
                    blocks.push((block, metadata));
                    row_offset += chunk.len();
                }
            }
            Column::Boolean(data) => {
                let chunks = data.chunks(self.block_size);
                for chunk in chunks {
                    // Convert booleans to u8 (0 or 1) for storage
                    let u8_data: Vec<u8> = chunk.iter().map(|&b| if b { 1u8 } else { 0u8 }).collect();
                    let (block, metadata) = self.write_chunk(
                        &u8_data,
                        &*compressor,
                        column_id,
                        row_offset,
                        DataType::Boolean,
                    )?;
                    blocks.push((block, metadata));
                    row_offset += chunk.len();
                }
            }
            Column::String(data) => {
                let chunks = data.chunks(self.block_size);
                for chunk in chunks {
                    let serialized = bincode::serialize(chunk)
                        .map_err(|e| Error::Serialization(format!("Failed to serialize: {}", e)))?;
                    let compressed = compressor.compress(&serialized)?;
                    
                    let block = Block {
                        column_id,
                        data: Bytes::from(compressed.clone()),
                        row_count: chunk.len(),
                        data_type: DataType::String,
                        compression: self.compression,
                        uncompressed_size: serialized.len(),
                        compressed_size: compressed.len(),
                    };

                    let metadata = BlockMetadata {
                        block_id: blocks.len() as u64,
                        column_id,
                        row_start: row_offset,
                        row_count: chunk.len(),
                        data_type: DataType::String,
                        compression: self.compression,
                        uncompressed_size: serialized.len(),
                        compressed_size: compressed.len(),
                        min_value: None,
                        max_value: None,
                        null_count: 0,
                    };

                    blocks.push((block, metadata));
                    row_offset += chunk.len();
                }
            }
            _ => {
                return Err(Error::Storage("Unsupported column type for writing".to_string()));
            }
        }

        Ok(blocks)
    }

    fn write_chunk<T: Copy>(
        &self,
        chunk: &[T],
        compressor: &dyn Compressor,
        column_id: u32,
        row_start: usize,
        data_type: DataType,
    ) -> Result<(Block, BlockMetadata)> {
        // True column-oriented: direct memory copy, no serialization overhead
        use std::mem;
        let size = mem::size_of::<T>();
        // Check for integer overflow in multiplication
        let total_bytes = chunk.len().checked_mul(size)
            .ok_or_else(|| Error::Storage(format!("Integer overflow: {} * {}", chunk.len(), size)))?;
        let raw_bytes = unsafe {
            std::slice::from_raw_parts(
                chunk.as_ptr() as *const u8,
                total_bytes
            )
        };
        let compressed = compressor.compress(raw_bytes)?;

        let uncompressed_size = raw_bytes.len();
        
        let block = Block {
            column_id,
            data: Bytes::from(compressed.clone()),
            row_count: chunk.len(),
            data_type: data_type.clone(),
            compression: self.compression,
            uncompressed_size,
            compressed_size: compressed.len(),
        };

        let metadata = BlockMetadata {
            block_id: 0,
            column_id,
            row_start,
            row_count: chunk.len(),
            data_type,
            compression: self.compression,
            uncompressed_size,
            compressed_size: compressed.len(),
            min_value: None,
            max_value: None,
            null_count: 0,
        };

        Ok((block, metadata))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use narayana_core::types::CompressionType;

    #[test]
    fn test_write_int32_column() {
        let writer = ColumnWriter::new(CompressionType::LZ4, 100);
        let column = narayana_core::column::Column::Int32(vec![1, 2, 3, 4, 5]);
        let blocks = writer.write_column(&column, 0).unwrap();
        assert!(!blocks.is_empty());
    }

    #[test]
    fn test_write_int64_column() {
        let writer = ColumnWriter::new(CompressionType::Zstd, 100);
        let column = narayana_core::column::Column::Int64(vec![1, 2, 3]);
        let blocks = writer.write_column(&column, 0).unwrap();
        assert!(!blocks.is_empty());
    }

    #[test]
    fn test_write_string_column() {
        let writer = ColumnWriter::new(CompressionType::Snappy, 100);
        let column = narayana_core::column::Column::String(vec!["hello".to_string(), "world".to_string()]);
        let blocks = writer.write_column(&column, 0).unwrap();
        assert!(!blocks.is_empty());
    }

    #[test]
    fn test_write_large_column() {
        let writer = ColumnWriter::new(CompressionType::LZ4, 10);
        let data: Vec<i32> = (0..100).collect();
        let column = narayana_core::column::Column::Int32(data);
        let blocks = writer.write_column(&column, 0).unwrap();
        assert!(blocks.len() > 1); // Should create multiple blocks
    }

    #[test]
    fn test_block_metadata() {
        let writer = ColumnWriter::new(CompressionType::None, 100);
        let column = narayana_core::column::Column::Int32(vec![1, 2, 3]);
        let blocks = writer.write_column(&column, 0).unwrap();
        
        for (block, metadata) in blocks {
            assert_eq!(block.column_id, 0);
            assert_eq!(block.row_count, metadata.row_count);
            assert_eq!(block.compression, metadata.compression);
        }
    }
}
