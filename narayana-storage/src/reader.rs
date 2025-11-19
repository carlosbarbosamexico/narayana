use narayana_core::{Error, Result, column::Column, schema::DataType, types::CompressionType};
use crate::block::Block;
use crate::compression::{create_decompressor, Decompressor};
use bincode;

pub struct ColumnReader {
    compression: CompressionType,
}

impl ColumnReader {
    pub fn new(compression: CompressionType) -> Self {
        Self { compression }
    }

    pub fn read_block(&self, block: &Block) -> Result<Column> {
        let decompressor = create_decompressor(block.compression);
        let decompressed = decompressor.decompress(&block.data, block.uncompressed_size)?;

        // True column-oriented: direct memory access, no deserialization overhead
        use std::mem;
        
        match &block.data_type {
            DataType::UInt8 => {
                Ok(Column::UInt8(decompressed))
            }
            DataType::Int8 => {
                // SECURITY: i8 is 1 byte, so alignment is always satisfied
                // No alignment check needed for single-byte types
                // SECURITY: Limit count to prevent memory exhaustion
                const MAX_COUNT: usize = 1_000_000_000; // 1 billion max
                let count = decompressed.len();
                if count > MAX_COUNT {
                    return Err(Error::Deserialization(format!(
                        "Count {} exceeds maximum allowed {}",
                        count, MAX_COUNT
                    )));
                }
                let mut data = Vec::with_capacity(count);
                unsafe {
                    let src = decompressed.as_ptr() as *const i8;
                    let dst = data.as_mut_ptr();
                    std::ptr::copy_nonoverlapping(src, dst, count);
                    data.set_len(count);
                }
                Ok(Column::Int8(data))
            }
            DataType::Int32 => {
                let size = mem::size_of::<i32>();
                if size == 0 {
                    return Err(Error::Deserialization("Invalid size: zero".to_string()));
                }
                if decompressed.len() % size != 0 {
                    return Err(Error::Deserialization("Invalid data length".to_string()));
                }
                let count = decompressed.len() / size;
                let mut data = Vec::with_capacity(count);
                
                // SECURITY: Check memory alignment before unsafe pointer operations
                let src_ptr = decompressed.as_ptr();
                let dst_ptr = data.as_mut_ptr();
                let align = mem::align_of::<i32>();
                
                if src_ptr as usize % align != 0 || dst_ptr as usize % align != 0 {
                    return Err(Error::Deserialization(format!(
                        "Unaligned memory access: required alignment {}, src align {}, dst align {}",
                        align,
                        src_ptr as usize % align,
                        dst_ptr as usize % align
                    )));
                }
                
                unsafe {
                    let src = src_ptr as *const i32;
                    let dst = dst_ptr;
                    std::ptr::copy_nonoverlapping(src, dst, count);
                    data.set_len(count);
                }
                Ok(Column::Int32(data))
            }
            DataType::Int64 => {
                let size = mem::size_of::<i64>();
                if size == 0 {
                    return Err(Error::Deserialization("Invalid size: zero".to_string()));
                }
                if decompressed.len() % size != 0 {
                    return Err(Error::Deserialization("Invalid data length".to_string()));
                }
                let count = decompressed.len() / size;
                let mut data = Vec::with_capacity(count);
                
                // SECURITY: Check memory alignment before unsafe pointer operations
                let src_ptr = decompressed.as_ptr();
                let dst_ptr = data.as_mut_ptr();
                let align = mem::align_of::<i64>();
                
                if src_ptr as usize % align != 0 || dst_ptr as usize % align != 0 {
                    return Err(Error::Deserialization(format!(
                        "Unaligned memory access: required alignment {}, src align {}, dst align {}",
                        align,
                        src_ptr as usize % align,
                        dst_ptr as usize % align
                    )));
                }
                
                unsafe {
                    let src = src_ptr as *const i64;
                    let dst = dst_ptr;
                    std::ptr::copy_nonoverlapping(src, dst, count);
                    data.set_len(count);
                }
                Ok(Column::Int64(data))
            }
            DataType::UInt64 => {
                let size = mem::size_of::<u64>();
                if size == 0 {
                    return Err(Error::Deserialization("Invalid size: zero".to_string()));
                }
                if decompressed.len() % size != 0 {
                    return Err(Error::Deserialization("Invalid data length".to_string()));
                }
                let count = decompressed.len() / size;
                let mut data = Vec::with_capacity(count);
                
                // SECURITY: Check memory alignment before unsafe pointer operations
                let src_ptr = decompressed.as_ptr();
                let dst_ptr = data.as_mut_ptr();
                let align = mem::align_of::<u64>();
                
                if src_ptr as usize % align != 0 || dst_ptr as usize % align != 0 {
                    return Err(Error::Deserialization(format!(
                        "Unaligned memory access: required alignment {}, src align {}, dst align {}",
                        align,
                        src_ptr as usize % align,
                        dst_ptr as usize % align
                    )));
                }
                
                unsafe {
                    let src = src_ptr as *const u64;
                    let dst = dst_ptr;
                    std::ptr::copy_nonoverlapping(src, dst, count);
                    data.set_len(count);
                }
                Ok(Column::UInt64(data))
            }
            DataType::Float64 => {
                let size = mem::size_of::<f64>();
                if size == 0 {
                    return Err(Error::Deserialization("Invalid size: zero".to_string()));
                }
                if decompressed.len() % size != 0 {
                    return Err(Error::Deserialization("Invalid data length".to_string()));
                }
                let count = decompressed.len() / size;
                let mut data = Vec::with_capacity(count);
                
                // SECURITY: Check memory alignment before unsafe pointer operations
                let src_ptr = decompressed.as_ptr();
                let dst_ptr = data.as_mut_ptr();
                let align = mem::align_of::<f64>();
                
                if src_ptr as usize % align != 0 || dst_ptr as usize % align != 0 {
                    return Err(Error::Deserialization(format!(
                        "Unaligned memory access: required alignment {}, src align {}, dst align {}",
                        align,
                        src_ptr as usize % align,
                        dst_ptr as usize % align
                    )));
                }
                
                unsafe {
                    let src = src_ptr as *const f64;
                    let dst = dst_ptr;
                    std::ptr::copy_nonoverlapping(src, dst, count);
                    data.set_len(count);
                }
                Ok(Column::Float64(data))
            }
            DataType::Boolean => {
                // Boolean was stored as u8 (0 or 1), convert back to bool
                // SECURITY: Validate decompressed data length matches expected row count
                let expected_size = block.row_count;
                if decompressed.len() != expected_size {
                    return Err(Error::Deserialization(format!(
                        "Boolean column size mismatch: expected {} bytes ({} rows), got {} bytes",
                        expected_size, expected_size, decompressed.len()
                    )));
                }
                // Convert u8 (0 or 1) to bool
                let bool_data: Vec<bool> = decompressed.iter().map(|&b| b != 0).collect();
                Ok(Column::Boolean(bool_data))
            }
            DataType::String => {
                // Strings still need deserialization (variable length)
                let data: Vec<String> = bincode::deserialize(&decompressed)
                    .map_err(|e| Error::Deserialization(format!("Failed to deserialize: {}", e)))?;
                Ok(Column::String(data))
            }
            _ => Err(Error::Deserialization("Unsupported data type for reading".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use narayana_core::types::CompressionType;
    use crate::writer::ColumnWriter;
    use narayana_core::column::Column;

    #[test]
    fn test_read_write_roundtrip() {
        let writer = ColumnWriter::new(CompressionType::LZ4, 100);
        let reader = ColumnReader::new(CompressionType::LZ4);
        
        let original = Column::Int32(vec![1, 2, 3, 4, 5]);
        let blocks = writer.write_column(&original, 0).unwrap();
        
        for (block, _) in blocks {
            let read_column = reader.read_block(&block).unwrap();
            match (&original, &read_column) {
                (Column::Int32(orig), Column::Int32(read)) => {
                    assert_eq!(orig, read);
                }
                _ => {
                    eprintln!("Type mismatch in test: expected Int32, got different type");
                    panic!("Type mismatch in test");
                }
            }
        }
    }

    #[test]
    fn test_read_string_column() {
        let writer = ColumnWriter::new(CompressionType::Snappy, 100);
        let reader = ColumnReader::new(CompressionType::Snappy);
        
        let original = Column::String(vec!["hello".to_string(), "world".to_string()]);
        let blocks = writer.write_column(&original, 0).unwrap();
        
        for (block, _) in blocks {
            let read_column = reader.read_block(&block).unwrap();
            match read_column {
                Column::String(data) => {
                    assert_eq!(data.len(), 2);
                }
                _ => {
                    eprintln!("Type mismatch in test: expected String column, got different type");
                    panic!("Expected String column in test");
                }
            }
        }
    }
}
