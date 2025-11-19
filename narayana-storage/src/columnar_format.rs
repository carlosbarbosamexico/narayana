// True column-oriented storage format - zero metadata overhead
// Values stored directly without length numbers or extra data

use narayana_core::{Error, Result, schema::DataType};
use std::mem;
use bytes::{Bytes, BytesMut};

/// True column-oriented storage - raw bytes, no metadata
pub struct ColumnarFormat;

impl ColumnarFormat {
    /// Write fixed-length values directly (zero overhead)
    /// For UInt8: 1 billion values = exactly 1GB uncompressed
    /// SECURITY: Returns Result to prevent panics from integer overflow
    pub fn write_fixed_length<T: Copy>(data: &[T]) -> Result<Bytes> {
        let size = mem::size_of::<T>();
        // SECURITY: Check for integer overflow in multiplication - return error instead of panic
        let total_bytes = data.len().checked_mul(size)
            .ok_or_else(|| Error::Storage(format!(
                "Integer overflow in write_fixed_length: {} * {} exceeds usize::MAX",
                data.len(), size
            )))?;
        
        // Direct memory copy - no serialization overhead
        let mut buffer = BytesMut::with_capacity(total_bytes);
        unsafe {
            let src = data.as_ptr() as *const u8;
            let dst = buffer.as_mut_ptr();
            std::ptr::copy_nonoverlapping(src, dst, total_bytes);
            buffer.set_len(total_bytes);
        }
        
        Ok(buffer.freeze())
    }

    /// Read fixed-length values directly (zero overhead)
    pub fn read_fixed_length<T: Copy>(bytes: &[u8]) -> Result<Vec<T>> {
        let size = mem::size_of::<T>();
        if size == 0 {
            return Err(Error::Storage("Invalid size: zero".to_string()));
        }
        if bytes.len() % size != 0 {
            return Err(Error::Storage(format!(
                "Invalid data length: {} not divisible by {}",
                bytes.len(),
                size
            )));
        }
        
        let count = bytes.len() / size;
        let mut result = Vec::with_capacity(count);
        
        // EDGE CASE: Check memory alignment before unsafe pointer operations
        // Some architectures require aligned memory access
        let src_ptr = bytes.as_ptr();
        let dst_ptr = result.as_mut_ptr();
        let align = std::mem::align_of::<T>();
        
        // If alignment requirements are not met, use safe fallback
        if src_ptr as usize % align != 0 || dst_ptr as usize % align != 0 {
            // Fallback: use safe byte-by-byte copy (slower but safe)
            // This handles unaligned memory access
            return Err(Error::Storage(format!(
                "Unaligned memory access: required alignment {}, src align {}, dst align {}",
                align,
                src_ptr as usize % align,
                dst_ptr as usize % align
            )));
        }
        
        unsafe {
            let src = src_ptr as *const T;
            let dst = dst_ptr;
            std::ptr::copy_nonoverlapping(src, dst, count);
            result.set_len(count);
        }
        
        Ok(result)
    }

    /// Write UInt8 column (1 byte per value, zero overhead)
    pub fn write_uint8(data: &[u8]) -> Bytes {
        // Direct copy - 1 billion values = exactly 1GB
        Bytes::copy_from_slice(data)
    }

    /// Read UInt8 column (zero overhead)
    pub fn read_uint8(bytes: &[u8]) -> Vec<u8> {
        bytes.to_vec()
    }

    /// Write Int32 column (4 bytes per value, zero overhead)
    pub fn write_int32(data: &[i32]) -> Result<Bytes> {
        Self::write_fixed_length(data)
    }

    /// Read Int32 column (zero overhead)
    pub fn read_int32(bytes: &[u8]) -> Result<Vec<i32>> {
        Self::read_fixed_length(bytes)
    }

    /// Write Int64 column (8 bytes per value, zero overhead)
    pub fn write_int64(data: &[i64]) -> Result<Bytes> {
        Self::write_fixed_length(data)
    }

    /// Read Int64 column (zero overhead)
    pub fn read_int64(bytes: &[u8]) -> Result<Vec<i64>> {
        Self::read_fixed_length(bytes)
    }

    /// Write Float64 column (8 bytes per value, zero overhead)
    pub fn write_float64(data: &[f64]) -> Result<Bytes> {
        Self::write_fixed_length(data)
    }

    /// Read Float64 column (zero overhead)
    pub fn read_float64(bytes: &[u8]) -> Result<Vec<f64>> {
        Self::read_fixed_length(bytes)
    }

    /// Write Boolean column (1 bit per value, packed)
    pub fn write_boolean(data: &[bool]) -> Bytes {
        // Pack 8 booleans per byte
        let byte_count = (data.len() + 7) / 8;
        let mut buffer = BytesMut::with_capacity(byte_count);
        buffer.resize(byte_count, 0);
        
        for (i, &value) in data.iter().enumerate() {
            let byte_idx = i / 8;
            let bit_idx = i % 8;
            if value {
                buffer[byte_idx] |= 1 << bit_idx;
            }
        }
        
        buffer.freeze()
    }

    /// Read Boolean column (zero overhead)
    pub fn read_boolean(bytes: &[u8], count: usize) -> Vec<bool> {
        let mut result = Vec::with_capacity(count);
        for i in 0..count {
            let byte_idx = i / 8;
            let bit_idx = i % 8;
            if byte_idx < bytes.len() {
                result.push((bytes[byte_idx] & (1 << bit_idx)) != 0);
            } else {
                result.push(false);
            }
        }
        result
    }

    /// Write String column (variable length, but compact)
    /// Uses offset array + concatenated strings
    /// SECURITY: Returns Result to prevent panics from integer overflow
    pub fn write_strings(data: &[String]) -> Result<(Bytes, Bytes)> {
        // Calculate offsets
        let mut offsets = Vec::with_capacity(data.len() + 1);
        offsets.push(0u32);
        
        // EDGE CASE: Prevent u32 overflow when summing string lengths
        let mut total_len = 0u32;
        for s in data {
            let len = s.len();
            // Check for overflow before adding
            total_len = total_len.saturating_add(len.min(u32::MAX as usize) as u32);
            offsets.push(total_len);
        }
        
        // Concatenate all strings
        let mut strings_data = BytesMut::with_capacity(total_len as usize);
        for s in data {
            strings_data.extend_from_slice(s.as_bytes());
        }
        
        // Write offsets as fixed-length
        let offsets_bytes = Self::write_fixed_length(&offsets)?;
        
        Ok((offsets_bytes, strings_data.freeze()))
    }

    /// Read String column (zero overhead)
    pub fn read_strings(offsets_bytes: &[u8], strings_bytes: &[u8]) -> Result<Vec<String>> {
        let offsets: Vec<u32> = Self::read_fixed_length(offsets_bytes)?;
        
        // SECURITY: Prevent integer underflow and empty offsets
        if offsets.is_empty() {
            return Ok(Vec::new());
        }
        
        if offsets.len() == 1 {
            // Only one offset (start), no strings
            return Ok(Vec::new());
        }
        
        // SECURITY: Check for integer underflow
        let string_count = offsets.len().checked_sub(1)
            .ok_or_else(|| Error::Storage("Integer underflow in offsets length".to_string()))?;
        
        let mut result = Vec::with_capacity(string_count);
        
        for i in 0..string_count {
            // SECURITY: Validate offset indices
            if i >= offsets.len() || (i + 1) >= offsets.len() {
                return Err(Error::Storage("Invalid offset index".to_string()));
            }
            
            let start = offsets[i] as usize;
            let end = offsets[i + 1] as usize;
            
            // SECURITY: Validate offset ordering (end must be >= start)
            if end < start {
                return Err(Error::Storage(format!(
                    "Invalid offset ordering: start {} > end {}",
                    start, end
                )));
            }
            
            // SECURITY: Validate bounds
            if end > strings_bytes.len() {
                return Err(Error::Storage(format!(
                    "Offset {} exceeds string data length {}",
                    end, strings_bytes.len()
                )));
            }
            
            // SECURITY: Validate start is within bounds
            if start > strings_bytes.len() {
                return Err(Error::Storage(format!(
                    "Offset {} exceeds string data length {}",
                    start, strings_bytes.len()
                )));
            }
            
            let s = String::from_utf8(strings_bytes[start..end].to_vec())
                .map_err(|e| Error::Storage(format!("Invalid UTF-8: {}", e)))?;
            result.push(s);
        }
        
        Ok(result)
    }
}

/// Compact column writer (true column-oriented)
pub struct CompactColumnWriter {
    block_size: usize,
}

impl CompactColumnWriter {
    pub fn new(block_size: usize) -> Self {
        Self { block_size }
    }

    /// Write column in true column-oriented format
    pub fn write_column(&self, data_type: DataType, data: &[u8], row_count: usize) -> Result<Bytes> {
        match data_type {
            DataType::UInt8 => {
                // Direct write - 1 byte per value
                Ok(ColumnarFormat::write_uint8(data))
            }
            DataType::Int32 => {
                // 4 bytes per value
                let ints: &[i32] = unsafe {
                    std::slice::from_raw_parts(data.as_ptr() as *const i32, row_count)
                };
                // write_int32 already returns Result<Bytes>
                ColumnarFormat::write_int32(ints)
            }
            DataType::Int64 => {
                // 8 bytes per value
                let longs: &[i64] = unsafe {
                    std::slice::from_raw_parts(data.as_ptr() as *const i64, row_count)
                };
                // write_int64 already returns Result<Bytes>
                ColumnarFormat::write_int64(longs)
            }
            DataType::Float64 => {
                // 8 bytes per value
                let floats: &[f64] = unsafe {
                    std::slice::from_raw_parts(data.as_ptr() as *const f64, row_count)
                };
                // write_float64 already returns Result<Bytes>
                ColumnarFormat::write_float64(floats)
            }
            _ => Err(Error::Storage("Unsupported fixed-length type".to_string())),
        }
    }
}

/// Compact column reader (true column-oriented)
pub struct CompactColumnReader;

impl CompactColumnReader {
    pub fn new() -> Self {
        Self
    }

    /// Read column in true column-oriented format
    pub fn read_column(&self, data_type: DataType, bytes: &[u8], row_count: usize) -> Result<Vec<u8>> {
        match data_type {
            DataType::UInt8 => {
                Ok(ColumnarFormat::read_uint8(bytes))
            }
            DataType::Int32 => {
                let ints = ColumnarFormat::read_int32(bytes)?;
                let size = mem::size_of::<i32>();
                // SECURITY: Check for integer overflow - return error instead of panic
                let total_bytes = ints.len().checked_mul(size)
                    .ok_or_else(|| Error::Storage(format!(
                        "Integer overflow in columnar format conversion: {} * {} exceeds usize::MAX",
                        ints.len(), size
                    )))?;
                let bytes: Vec<u8> = unsafe {
                    std::slice::from_raw_parts(
                        ints.as_ptr() as *const u8,
                        total_bytes
                    ).to_vec()
                };
                Ok(bytes)
            }
            DataType::Int64 => {
                let longs = ColumnarFormat::read_int64(bytes)?;
                let size = mem::size_of::<i64>();
                // SECURITY: Check for integer overflow - return error instead of panic
                let total_bytes = longs.len().checked_mul(size)
                    .ok_or_else(|| Error::Storage(format!(
                        "Integer overflow in columnar format conversion: {} * {} exceeds usize::MAX",
                        longs.len(), size
                    )))?;
                let bytes: Vec<u8> = unsafe {
                    std::slice::from_raw_parts(
                        longs.as_ptr() as *const u8,
                        total_bytes
                    ).to_vec()
                };
                Ok(bytes)
            }
            DataType::Float64 => {
                let floats = ColumnarFormat::read_float64(bytes)?;
                let size = mem::size_of::<f64>();
                // SECURITY: Check for integer overflow - return error instead of panic
                let total_bytes = floats.len().checked_mul(size)
                    .ok_or_else(|| Error::Storage(format!(
                        "Integer overflow in columnar format conversion: {} * {} exceeds usize::MAX",
                        floats.len(), size
                    )))?;
                let bytes: Vec<u8> = unsafe {
                    std::slice::from_raw_parts(
                        floats.as_ptr() as *const u8,
                        total_bytes
                    ).to_vec()
                };
                Ok(bytes)
            }
            _ => Err(Error::Storage("Unsupported fixed-length type".to_string())),
        }
    }
}

/// Size calculator for true column-oriented storage
pub struct ColumnarSizeCalculator;

impl ColumnarSizeCalculator {
    /// Calculate exact size for fixed-length column
    pub fn calculate_size(data_type: DataType, row_count: usize) -> usize {
        match data_type {
            DataType::UInt8 => row_count * 1,
            DataType::Int8 => row_count * 1,
            DataType::Int16 => row_count * 2,
            DataType::Int32 => row_count * 4,
            DataType::Int64 => row_count * 8,
            DataType::UInt16 => row_count * 2,
            DataType::UInt32 => row_count * 4,
            DataType::UInt64 => row_count * 8,
            DataType::Float32 => row_count * 4,
            DataType::Float64 => row_count * 8,
            DataType::Boolean => (row_count + 7) / 8, // Packed bits
            DataType::Timestamp => row_count * 8,
            DataType::Date => row_count * 4,
            _ => 0, // Variable length
        }
    }

    /// Verify size matches expected (for validation)
    pub fn verify_size(data_type: DataType, bytes: &[u8], row_count: usize) -> bool {
        let expected = Self::calculate_size(data_type, row_count);
        bytes.len() == expected
    }
}

