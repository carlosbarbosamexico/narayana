use narayana_core::{Error, Result, types::CompressionType};
use bytes::{Bytes, BytesMut};

pub trait Compressor: Send + Sync {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>>;
    fn compression_type(&self) -> CompressionType;
}

pub trait Decompressor: Send + Sync {
    fn decompress(&self, data: &[u8], output_len: usize) -> Result<Vec<u8>>;
}

pub struct Lz4Compressor;

impl Compressor for Lz4Compressor {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Use FAST compression mode with level 4 (HIGH_COMPRESSION doesn't exist in lz4 crate)
        lz4::block::compress(data, Some(lz4::block::CompressionMode::FAST(4)), true)
            .map_err(|e| Error::Serialization(format!("LZ4 compression failed: {}", e)))
    }

    fn compression_type(&self) -> CompressionType {
        CompressionType::LZ4
    }
}

impl Decompressor for Lz4Compressor {
    fn decompress(&self, data: &[u8], output_len: usize) -> Result<Vec<u8>> {
        // SECURITY: Prevent compression bomb attacks - limit decompressed size
        const MAX_DECOMPRESSED_SIZE: usize = 100 * 1024 * 1024; // 100MB max
        if output_len > MAX_DECOMPRESSED_SIZE {
            return Err(Error::Deserialization(format!(
                "Decompressed size {} exceeds maximum allowed size {}",
                output_len, MAX_DECOMPRESSED_SIZE
            )));
        }
        
        // Try decompression - if output_len is provided, use it as hint
        // The lz4 crate with frame header (true) should handle size automatically
        // But we can provide output_len as a capacity hint
        let result = if output_len > 0 {
            // Try with output_len first
            lz4::block::decompress(data, Some(output_len.try_into().unwrap_or(i32::MAX)))
                .or_else(|_| {
                    // If that fails, try without size hint (frame header should contain it)
                    lz4::block::decompress(data, None)
                })
        } else {
            lz4::block::decompress(data, None)
        };
        
        result.map_err(|e| Error::Deserialization(format!("LZ4 decompression failed: {}", e)))
    }
}

pub struct ZstdCompressor {
    level: i32,
}

impl ZstdCompressor {
    pub fn new(level: i32) -> Self {
        Self { level }
    }
}

impl Compressor for ZstdCompressor {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        zstd::encode_all(data, self.level)
            .map_err(|e| Error::Serialization(format!("Zstd compression failed: {}", e)))
    }

    fn compression_type(&self) -> CompressionType {
        CompressionType::Zstd
    }
}

impl Decompressor for ZstdCompressor {
    fn decompress(&self, data: &[u8], output_len: usize) -> Result<Vec<u8>> {
        // SECURITY: Prevent compression bomb attacks - limit decompressed size
        const MAX_DECOMPRESSED_SIZE: usize = 100 * 1024 * 1024; // 100MB max
        if output_len > MAX_DECOMPRESSED_SIZE {
            return Err(Error::Deserialization(format!(
                "Decompressed size {} exceeds maximum allowed size {}",
                output_len, MAX_DECOMPRESSED_SIZE
            )));
        }
        // SECURITY: Also check actual decompressed size after decompression
        let decompressed = zstd::decode_all(data)
            .map_err(|e| Error::Deserialization(format!("Zstd decompression failed: {}", e)))?;
        if decompressed.len() > MAX_DECOMPRESSED_SIZE {
            return Err(Error::Deserialization(format!(
                "Decompressed data size {} exceeds maximum allowed size {}",
                decompressed.len(), MAX_DECOMPRESSED_SIZE
            )));
        }
        Ok(decompressed)
    }
}

pub struct SnappyCompressor;

impl Compressor for SnappyCompressor {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Snappy compression using snap crate
        let mut encoder = snap::raw::Encoder::new();
        encoder.compress_vec(data)
            .map_err(|e| Error::Serialization(format!("Snappy compression failed: {:?}", e)))
    }

    fn compression_type(&self) -> CompressionType {
        CompressionType::Snappy
    }
}

impl Decompressor for SnappyCompressor {
    fn decompress(&self, data: &[u8], output_len: usize) -> Result<Vec<u8>> {
        // SECURITY: Prevent compression bomb attacks - limit decompressed size
        const MAX_DECOMPRESSED_SIZE: usize = 100 * 1024 * 1024; // 100MB max
        if output_len > MAX_DECOMPRESSED_SIZE {
            return Err(Error::Deserialization(format!(
                "Decompressed size {} exceeds maximum allowed size {}",
                output_len, MAX_DECOMPRESSED_SIZE
            )));
        }
        // Snappy decompression using snap crate
        let mut decoder = snap::raw::Decoder::new();
        let decompressed = decoder.decompress_vec(data)
            .map_err(|e| Error::Deserialization(format!("Snappy decompression failed: {:?}", e)))?;
        // SECURITY: Also check actual decompressed size after decompression
        if decompressed.len() > MAX_DECOMPRESSED_SIZE {
            return Err(Error::Deserialization(format!(
                "Decompressed data size {} exceeds maximum allowed size {}",
                decompressed.len(), MAX_DECOMPRESSED_SIZE
            )));
        }
        Ok(decompressed)
    }
}

pub struct NoOpCompressor;

impl Compressor for NoOpCompressor {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }

    fn compression_type(&self) -> CompressionType {
        CompressionType::None
    }
}

impl Decompressor for NoOpCompressor {
    fn decompress(&self, data: &[u8], _output_len: usize) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }
}

pub fn create_compressor(compression_type: CompressionType) -> Box<dyn Compressor> {
    match compression_type {
        CompressionType::None => Box::new(NoOpCompressor),
        CompressionType::LZ4 => Box::new(Lz4Compressor),
        CompressionType::Zstd => Box::new(ZstdCompressor::new(3)),
        CompressionType::Snappy => Box::new(SnappyCompressor),
    }
}

pub fn create_decompressor(compression_type: CompressionType) -> Box<dyn Decompressor> {
    match compression_type {
        CompressionType::None => Box::new(NoOpCompressor),
        CompressionType::LZ4 => Box::new(Lz4Compressor),
        CompressionType::Zstd => Box::new(ZstdCompressor::new(3)),
        CompressionType::Snappy => Box::new(SnappyCompressor),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use narayana_core::types::CompressionType;

    fn test_compression_roundtrip(comp_type: CompressionType) {
        let compressor = create_compressor(comp_type);
        let decompressor = create_decompressor(comp_type);
        
        let original = b"test data for compression roundtrip";
        let compressed = compressor.compress(original).unwrap();
        let decompressed = decompressor.decompress(&compressed, original.len()).unwrap();
        
        assert_eq!(original, decompressed.as_slice());
    }

    #[test]
    fn test_lz4_compression() {
        test_compression_roundtrip(CompressionType::LZ4);
    }

    #[test]
    fn test_zstd_compression() {
        test_compression_roundtrip(CompressionType::Zstd);
    }

    #[test]
    fn test_snappy_compression() {
        test_compression_roundtrip(CompressionType::Snappy);
    }

    #[test]
    fn test_noop_compression() {
        test_compression_roundtrip(CompressionType::None);
    }

    #[test]
    fn test_compression_types() {
        assert_eq!(create_compressor(CompressionType::LZ4).compression_type(), CompressionType::LZ4);
        assert_eq!(create_compressor(CompressionType::Zstd).compression_type(), CompressionType::Zstd);
        assert_eq!(create_compressor(CompressionType::Snappy).compression_type(), CompressionType::Snappy);
        assert_eq!(create_compressor(CompressionType::None).compression_type(), CompressionType::None);
    }

    #[test]
    fn test_compression_empty_data() {
        let compressor = create_compressor(CompressionType::LZ4);
        let decompressor = create_decompressor(CompressionType::LZ4);
        
        let original = b"";
        let compressed = compressor.compress(original).unwrap();
        let decompressed = decompressor.decompress(&compressed, 0).unwrap();
        
        assert_eq!(original, decompressed.as_slice());
    }

    #[test]
    fn test_compression_large_data() {
        let compressor = create_compressor(CompressionType::LZ4);
        let decompressor = create_decompressor(CompressionType::LZ4);
        
        let original: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
        let compressed = compressor.compress(&original).unwrap();
        let decompressed = decompressor.decompress(&compressed, original.len()).unwrap();
        
        assert_eq!(original, decompressed);
    }
}
