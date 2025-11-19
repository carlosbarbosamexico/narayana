// True column-oriented storage - optimized for CPU efficiency
// No metadata overhead, direct memory layout, compact storage

use narayana_core::{Error, Result, schema::DataType};
use bytes::{Bytes, BytesMut};
use std::mem;

/// True column-oriented block - raw values only
pub struct TrueColumnarBlock {
    pub data: Bytes,
    pub data_type: DataType,
    pub row_count: usize,
}

impl TrueColumnarBlock {
    /// Create block from raw data (zero overhead)
    pub fn new(data: Bytes, data_type: DataType, row_count: usize) -> Self {
        Self {
            data,
            data_type,
            row_count,
        }
    }

    /// Get exact size (for validation)
    pub fn expected_size(&self) -> usize {
        match self.data_type {
            DataType::UInt8 => self.row_count * 1,
            DataType::Int8 => self.row_count * 1,
            DataType::Int16 => self.row_count * 2,
            DataType::Int32 => self.row_count * 4,
            DataType::Int64 => self.row_count * 8,
            DataType::UInt16 => self.row_count * 2,
            DataType::UInt32 => self.row_count * 4,
            DataType::UInt64 => self.row_count * 8,
            DataType::Float32 => self.row_count * 4,
            DataType::Float64 => self.row_count * 8,
            DataType::Boolean => (self.row_count + 7) / 8,
            DataType::Timestamp => self.row_count * 8,
            DataType::Date => self.row_count * 4,
            _ => 0, // Variable length
        }
    }

    /// Verify block integrity
    pub fn verify(&self) -> bool {
        let expected = self.expected_size();
        if expected > 0 {
            self.data.len() == expected
        } else {
            true // Variable length, can't verify
        }
    }
}

/// True column-oriented writer (zero overhead)
pub struct TrueColumnarWriter;

impl TrueColumnarWriter {
    /// Write UInt8 column (1 billion values = exactly 1GB)
    pub fn write_uint8(data: &[u8]) -> TrueColumnarBlock {
        // Direct copy - no serialization, no metadata
        TrueColumnarBlock {
            data: Bytes::copy_from_slice(data),
            data_type: DataType::UInt8,
            row_count: data.len(),
        }
    }

    /// Write Int32 column (4 bytes per value)
    /// SECURITY: Returns Result to prevent panics from integer overflow
    pub fn write_int32(data: &[i32]) -> Result<TrueColumnarBlock> {
        let size = mem::size_of::<i32>();
        // SECURITY: Check for integer overflow - return error instead of panic
        let total_bytes = data.len().checked_mul(size)
            .ok_or_else(|| Error::Storage(format!(
                "Integer overflow in write_int32: {} * {} exceeds usize::MAX",
                data.len(), size
            )))?;
        let bytes = unsafe {
            std::slice::from_raw_parts(
                data.as_ptr() as *const u8,
                total_bytes
            )
        };
        Ok(TrueColumnarBlock {
            data: Bytes::copy_from_slice(bytes),
            data_type: DataType::Int32,
            row_count: data.len(),
        })
    }

    /// Write Int64 column (8 bytes per value)
    /// SECURITY: Returns Result to prevent panics from integer overflow
    pub fn write_int64(data: &[i64]) -> Result<TrueColumnarBlock> {
        let size = mem::size_of::<i64>();
        // SECURITY: Check for integer overflow - return error instead of panic
        let total_bytes = data.len().checked_mul(size)
            .ok_or_else(|| Error::Storage(format!(
                "Integer overflow in write_int64: {} * {} exceeds usize::MAX",
                data.len(), size
            )))?;
        let bytes = unsafe {
            std::slice::from_raw_parts(
                data.as_ptr() as *const u8,
                total_bytes
            )
        };
        Ok(TrueColumnarBlock {
            data: Bytes::copy_from_slice(bytes),
            data_type: DataType::Int64,
            row_count: data.len(),
        })
    }

    /// Write Float64 column (8 bytes per value)
    /// SECURITY: Returns Result to prevent panics from integer overflow
    pub fn write_float64(data: &[f64]) -> Result<TrueColumnarBlock> {
        let size = mem::size_of::<f64>();
        // SECURITY: Check for integer overflow - return error instead of panic
        let total_bytes = data.len().checked_mul(size)
            .ok_or_else(|| Error::Storage(format!(
                "Integer overflow in write_float64: {} * {} exceeds usize::MAX",
                data.len(), size
            )))?;
        let bytes = unsafe {
            std::slice::from_raw_parts(
                data.as_ptr() as *const u8,
                total_bytes
            )
        };
        Ok(TrueColumnarBlock {
            data: Bytes::copy_from_slice(bytes),
            data_type: DataType::Float64,
            row_count: data.len(),
        })
    }

    /// Write Boolean column (1 bit per value, packed)
    pub fn write_boolean(data: &[bool]) -> TrueColumnarBlock {
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
        
        TrueColumnarBlock {
            data: buffer.freeze(),
            data_type: DataType::Boolean,
            row_count: data.len(),
        }
    }
}

/// True column-oriented reader (zero overhead)
pub struct TrueColumnarReader;

impl TrueColumnarReader {
    /// Read UInt8 column (direct memory access)
    pub fn read_uint8(block: &TrueColumnarBlock) -> Result<Vec<u8>> {
        if block.data_type != DataType::UInt8 {
            return Err(Error::Storage("Type mismatch".to_string()));
        }
        Ok(block.data.to_vec())
    }

    /// Read Int32 column (direct memory access)
    pub fn read_int32(block: &TrueColumnarBlock) -> Result<Vec<i32>> {
        if block.data_type != DataType::Int32 {
            return Err(Error::Storage("Type mismatch".to_string()));
        }
        
        let size = mem::size_of::<i32>();
        if size == 0 {
            return Err(Error::Storage("Invalid size: zero".to_string()));
        }
        if block.data.len() % size != 0 {
            return Err(Error::Storage("Invalid data length".to_string()));
        }
        
        let count = block.data.len() / size;
        let mut result = Vec::with_capacity(count);
        
        unsafe {
            let src = block.data.as_ptr() as *const i32;
            let dst = result.as_mut_ptr();
            std::ptr::copy_nonoverlapping(src, dst, count);
            result.set_len(count);
        }
        
        Ok(result)
    }

    /// Read Int64 column (direct memory access)
    pub fn read_int64(block: &TrueColumnarBlock) -> Result<Vec<i64>> {
        if block.data_type != DataType::Int64 {
            return Err(Error::Storage("Type mismatch".to_string()));
        }
        
        let size = mem::size_of::<i64>();
        if size == 0 {
            return Err(Error::Storage("Invalid size: zero".to_string()));
        }
        if block.data.len() % size != 0 {
            return Err(Error::Storage("Invalid data length".to_string()));
        }
        
        let count = block.data.len() / size;
        let mut result = Vec::with_capacity(count);
        
        unsafe {
            let src = block.data.as_ptr() as *const i64;
            let dst = result.as_mut_ptr();
            std::ptr::copy_nonoverlapping(src, dst, count);
            result.set_len(count);
        }
        
        Ok(result)
    }

    /// Read Float64 column (direct memory access)
    pub fn read_float64(block: &TrueColumnarBlock) -> Result<Vec<f64>> {
        if block.data_type != DataType::Float64 {
            return Err(Error::Storage("Type mismatch".to_string()));
        }
        
        let size = mem::size_of::<f64>();
        if size == 0 {
            return Err(Error::Storage("Invalid size: zero".to_string()));
        }
        if block.data.len() % size != 0 {
            return Err(Error::Storage("Invalid data length".to_string()));
        }
        
        let count = block.data.len() / size;
        let mut result = Vec::with_capacity(count);
        
        unsafe {
            let src = block.data.as_ptr() as *const f64;
            let dst = result.as_mut_ptr();
            std::ptr::copy_nonoverlapping(src, dst, count);
            result.set_len(count);
        }
        
        Ok(result)
    }

    /// Read Boolean column (bit-packed)
    pub fn read_boolean(block: &TrueColumnarBlock) -> Result<Vec<bool>> {
        if block.data_type != DataType::Boolean {
            return Err(Error::Storage("Type mismatch".to_string()));
        }
        
        let mut result = Vec::with_capacity(block.row_count);
        let expected_byte_count = (block.row_count + 7) / 8;
        for i in 0..block.row_count {
            let byte_idx = i / 8;
            let bit_idx = i % 8;
            // Safe bounds check - prevent index out of bounds
            if byte_idx < block.data.len() && byte_idx < expected_byte_count {
                result.push((block.data[byte_idx] & (1 << bit_idx)) != 0);
            } else {
                // Out of bounds - default to false
                result.push(false);
            }
        }
        
        Ok(result)
    }
}

/// CPU-optimized column operations
pub struct CpuOptimizedOps;

impl CpuOptimizedOps {
    /// SIMD-optimized sum for UInt8
    pub fn sum_uint8(data: &[u8]) -> u64 {
        // Use parallel sum for CPU efficiency
        use rayon::prelude::*;
        data.par_iter().map(|&x| x as u64).sum()
    }

    /// SIMD-optimized sum for Int32
    pub fn sum_int32(data: &[i32]) -> i64 {
        use rayon::prelude::*;
        data.par_iter().map(|&x| x as i64).sum()
    }

    /// SIMD-optimized sum for Int64
    pub fn sum_int64(data: &[i64]) -> i64 {
        use rayon::prelude::*;
        data.par_iter().sum()
    }

    /// SIMD-optimized filter
    pub fn filter_uint8(data: &[u8], mask: &[bool]) -> Vec<u8> {
        use rayon::prelude::*;
        data.par_iter()
            .zip(mask.par_iter())
            .filter_map(|(&val, &keep)| if keep { Some(val) } else { None })
            .collect()
    }

    /// SIMD-optimized comparison
    pub fn compare_eq_int32(data: &[i32], value: i32) -> Vec<bool> {
        use rayon::prelude::*;
        data.par_iter().map(|&x| x == value).collect()
    }
}

/// Memory layout optimizer for cache efficiency
pub struct MemoryLayoutOptimizer;

impl MemoryLayoutOptimizer {
    /// Align data for optimal CPU cache usage
    pub fn align_for_cache(data: &mut Vec<u8>, alignment: usize) {
        let remainder = data.len() % alignment;
        if remainder != 0 {
            let padding = alignment - remainder;
            data.resize(data.len() + padding, 0);
        }
    }

    /// Ensure data is cache-line aligned (64 bytes)
    pub fn align_cache_line(data: &mut Vec<u8>) {
        Self::align_for_cache(data, 64);
    }
}

