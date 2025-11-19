// Advanced indexing - skip indexes, bloom filters, min-max indexes
// Way beyond ClickHouse capabilities

use narayana_core::{Error, Result, schema::DataType};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use bytes::Bytes;

/// Skip index for fast range queries
pub struct SkipIndex {
    pub column_id: u32,
    pub block_size: usize,
    pub min_values: Vec<Bytes>, // Min value per block
    pub max_values: Vec<Bytes>, // Max value per block
    pub row_offsets: Vec<usize>, // Row offset per block
}

impl SkipIndex {
    pub fn new(column_id: u32, block_size: usize) -> Self {
        Self {
            column_id,
            block_size,
            min_values: Vec::new(),
            max_values: Vec::new(),
            row_offsets: Vec::new(),
        }
    }

    /// Add block to skip index
    /// SECURITY: Validates parameters to prevent DoS
    pub fn add_block(&mut self, min_value: Bytes, max_value: Bytes, row_offset: usize) -> Result<()> {
        // SECURITY: Limit value sizes
        const MAX_VALUE_SIZE: usize = 10 * 1024 * 1024; // 10MB max
        if min_value.len() > MAX_VALUE_SIZE {
            return Err(Error::Storage(format!(
                "Min value size {} exceeds maximum allowed {} bytes",
                min_value.len(), MAX_VALUE_SIZE
            )));
        }
        if max_value.len() > MAX_VALUE_SIZE {
            return Err(Error::Storage(format!(
                "Max value size {} exceeds maximum allowed {} bytes",
                max_value.len(), MAX_VALUE_SIZE
            )));
        }
        
        // SECURITY: Validate min <= max
        if min_value > max_value {
            return Err(Error::Storage("Min value must be <= max value".to_string()));
        }
        
        // SECURITY: Limit number of blocks to prevent DoS
        const MAX_BLOCKS: usize = 10_000_000; // 10M blocks max
        if self.min_values.len() >= MAX_BLOCKS {
            return Err(Error::Storage(format!(
                "Number of blocks {} exceeds maximum allowed {}",
                self.min_values.len(), MAX_BLOCKS
            )));
        }
        
        self.min_values.push(min_value);
        self.max_values.push(max_value);
        self.row_offsets.push(row_offset);
        Ok(())
    }

    /// Find blocks that might contain value (for range queries)
    /// SECURITY: Validates query parameters and limits results
    pub fn find_blocks(&self, min: &Bytes, max: &Bytes) -> Result<Vec<usize>> {
        // SECURITY: Validate query parameters
        const MAX_QUERY_SIZE: usize = 1024 * 1024; // 1MB max
        if min.len() > MAX_QUERY_SIZE || max.len() > MAX_QUERY_SIZE {
            return Err(Error::Storage(format!(
                "Query size exceeds maximum allowed {} bytes",
                MAX_QUERY_SIZE
            )));
        }
        
        // SECURITY: Validate min <= max
        if min > max {
            return Err(Error::Storage("Min query must be <= max query".to_string()));
        }
        
        // SECURITY: Limit result size to prevent DoS
        const MAX_RESULTS: usize = 1_000_000; // 1M results max
        let mut result = Vec::new();
        
        for (i, (block_min, block_max)) in self.min_values.iter().zip(self.max_values.iter()).enumerate() {
            // Check if ranges overlap
            if block_max >= min && block_min <= max {
                result.push(i);
                
                // SECURITY: Early exit if too many results
                if result.len() > MAX_RESULTS {
                    return Err(Error::Storage(format!(
                        "Found {} matching blocks, exceeds maximum allowed {}",
                        result.len(), MAX_RESULTS
                    )));
                }
            }
        }
        Ok(result)
    }
}

/// Bloom filter for fast membership tests
pub struct BloomFilter {
    bits: Vec<u8>,
    pub hash_count: usize,
    pub bit_count: usize,
}

impl BloomFilter {
    /// Create a new bloom filter
    /// SECURITY: Validates parameters to prevent DoS attacks
    pub fn new(capacity: usize, false_positive_rate: f64) -> Result<Self> {
        // SECURITY: Validate false_positive_rate
        if false_positive_rate <= 0.0 || false_positive_rate >= 1.0 {
            return Err(Error::Storage(format!(
                "False positive rate must be between 0 and 1, got {}",
                false_positive_rate
            )));
        }
        
        // SECURITY: Prevent DoS by limiting capacity
        const MAX_CAPACITY: usize = 1_000_000_000; // 1B elements max
        if capacity > MAX_CAPACITY {
            return Err(Error::Storage(format!(
                "Capacity {} exceeds maximum allowed {}",
                capacity, MAX_CAPACITY
            )));
        }
        if capacity == 0 {
            return Err(Error::Storage("Capacity must be greater than 0".to_string()));
        }
        
        // Calculate optimal bit count and hash count
        // SECURITY: Check for overflow in calculations
        let bit_count_f64 = -(capacity as f64 * false_positive_rate.ln()) / (2.0_f64.ln().powi(2));
        if bit_count_f64.is_infinite() || bit_count_f64.is_nan() {
            return Err(Error::Storage("Invalid bit count calculation".to_string()));
        }
        
        let bit_count = bit_count_f64.ceil() as usize;
        
        // SECURITY: Limit bit count to prevent DoS
        const MAX_BIT_COUNT: usize = 10_000_000_000; // ~1.25GB max
        if bit_count > MAX_BIT_COUNT {
            return Err(Error::Storage(format!(
                "Calculated bit count {} exceeds maximum allowed {}",
                bit_count, MAX_BIT_COUNT
            )));
        }
        
        let hash_count_f64 = (bit_count as f64 / capacity as f64) * 2.0_f64.ln();
        let hash_count = hash_count_f64.ceil() as usize;
        
        // SECURITY: Limit hash count
        const MAX_HASH_COUNT: usize = 50;
        if hash_count > MAX_HASH_COUNT {
            return Err(Error::Storage(format!(
                "Calculated hash count {} exceeds maximum allowed {}",
                hash_count, MAX_HASH_COUNT
            )));
        }
        
        // SECURITY: Check for integer overflow in byte calculation
        let byte_count = (bit_count.checked_add(7))
            .and_then(|n| n.checked_div(8))
            .ok_or_else(|| Error::Storage("Integer overflow in byte count calculation".to_string()))?;
        
        Ok(Self {
            bits: vec![0; byte_count],
            hash_count,
            bit_count,
        })
    }

    /// Add value to bloom filter
    /// SECURITY: Validates value size and bounds
    pub fn add(&mut self, value: &[u8]) -> Result<()> {
        // SECURITY: Limit value size to prevent DoS
        const MAX_VALUE_SIZE: usize = 10 * 1024 * 1024; // 10MB max
        if value.len() > MAX_VALUE_SIZE {
            return Err(Error::Storage(format!(
                "Value size {} exceeds maximum allowed {} bytes",
                value.len(), MAX_VALUE_SIZE
            )));
        }
        
        for i in 0..self.hash_count {
            let hash = self.hash(value, i);
            let bit_index = hash % self.bit_count;
            let byte_index = bit_index / 8;
            let bit_offset = bit_index % 8;
            
            // SECURITY: Bounds check to prevent out-of-bounds access
            if byte_index >= self.bits.len() {
                return Err(Error::Storage(format!(
                    "Byte index {} out of bounds (bits len: {})",
                    byte_index, self.bits.len()
                )));
            }
            
            // SECURITY: Validate bit_offset is within byte (0-7)
            if bit_offset > 7 {
                return Err(Error::Storage(format!(
                    "Bit offset {} out of range (must be 0-7)",
                    bit_offset
                )));
            }
            
            self.bits[byte_index] |= 1 << bit_offset;
        }
        Ok(())
    }

    /// Check if value might be in set
    /// SECURITY: Validates value size and bounds
    pub fn might_contain(&self, value: &[u8]) -> Result<bool> {
        // SECURITY: Limit value size to prevent DoS
        const MAX_VALUE_SIZE: usize = 10 * 1024 * 1024; // 10MB max
        if value.len() > MAX_VALUE_SIZE {
            return Err(Error::Storage(format!(
                "Value size {} exceeds maximum allowed {} bytes",
                value.len(), MAX_VALUE_SIZE
            )));
        }
        
        for i in 0..self.hash_count {
            let hash = self.hash(value, i);
            let bit_index = hash % self.bit_count;
            let byte_index = bit_index / 8;
            let bit_offset = bit_index % 8;
            
            // SECURITY: Bounds check to prevent out-of-bounds access
            if byte_index >= self.bits.len() {
                return Err(Error::Storage(format!(
                    "Byte index {} out of bounds (bits len: {})",
                    byte_index, self.bits.len()
                )));
            }
            
            // SECURITY: Validate bit_offset is within byte (0-7)
            if bit_offset > 7 {
                return Err(Error::Storage(format!(
                    "Bit offset {} out of range (must be 0-7)",
                    bit_offset
                )));
            }
            
            if (self.bits[byte_index] & (1 << bit_offset)) == 0 {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn hash(&self, value: &[u8], seed: usize) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        // SECURITY: Limit value size to prevent DoS in hashing
        const MAX_HASH_VALUE_SIZE: usize = 10 * 1024 * 1024; // 10MB max
        let value_to_hash = if value.len() > MAX_HASH_VALUE_SIZE {
            &value[..MAX_HASH_VALUE_SIZE]
        } else {
            value
        };
        
        let mut hasher = DefaultHasher::new();
        value_to_hash.hash(&mut hasher);
        seed.hash(&mut hasher);
        hasher.finish() as usize
    }
}

/// Min-max index for range queries
pub struct MinMaxIndex {
    pub column_id: u32,
    pub block_min: Option<Bytes>,
    pub block_max: Option<Bytes>,
    pub global_min: Option<Bytes>,
    pub global_max: Option<Bytes>,
}

impl MinMaxIndex {
    pub fn new(column_id: u32) -> Self {
        Self {
            column_id,
            block_min: None,
            block_max: None,
            global_min: None,
            global_max: None,
        }
    }

    /// Update with new value
    /// SECURITY: Validates value size to prevent DoS
    pub fn update(&mut self, value: Bytes) -> Result<()> {
        // SECURITY: Limit value size to prevent DoS
        const MAX_VALUE_SIZE: usize = 10 * 1024 * 1024; // 10MB max
        if value.len() > MAX_VALUE_SIZE {
            return Err(Error::Storage(format!(
                "Value size {} exceeds maximum allowed {} bytes",
                value.len(), MAX_VALUE_SIZE
            )));
        }
        
        if let Some(ref min) = self.global_min {
            if value < *min {
                self.global_min = Some(value.clone());
            }
        } else {
            self.global_min = Some(value.clone());
        }

        if let Some(ref max) = self.global_max {
            if value > *max {
                self.global_max = Some(value.clone());
            }
        } else {
            self.global_max = Some(value.clone());
        }
        
        Ok(())
    }

    /// Check if range might contain value
    /// SECURITY: Validates query parameters to prevent DoS
    pub fn might_contain(&self, min: &Bytes, max: &Bytes) -> Result<bool> {
        // SECURITY: Validate query parameter sizes
        const MAX_QUERY_SIZE: usize = 10 * 1024 * 1024; // 10MB max
        if min.len() > MAX_QUERY_SIZE {
            return Err(Error::Storage(format!(
                "Min query size {} exceeds maximum allowed {} bytes",
                min.len(), MAX_QUERY_SIZE
            )));
        }
        if max.len() > MAX_QUERY_SIZE {
            return Err(Error::Storage(format!(
                "Max query size {} exceeds maximum allowed {} bytes",
                max.len(), MAX_QUERY_SIZE
            )));
        }
        
        // SECURITY: Validate min <= max
        if min > max {
            return Err(Error::Storage("Min query must be <= max query".to_string()));
        }
        
        if let (Some(ref global_min), Some(ref global_max)) = (&self.global_min, &self.global_max) {
            // Check if ranges overlap
            Ok(global_max >= min && global_min <= max)
        } else {
            Ok(true) // No data yet, assume might contain
        }
    }
}

/// Index manager
pub struct AdvancedIndexManager {
    skip_indexes: std::sync::Arc<parking_lot::RwLock<HashMap<u32, SkipIndex>>>,
    bloom_filters: std::sync::Arc<parking_lot::RwLock<HashMap<u32, BloomFilter>>>,
    min_max_indexes: std::sync::Arc<parking_lot::RwLock<HashMap<u32, MinMaxIndex>>>,
}

impl AdvancedIndexManager {
    pub fn new() -> Self {
        Self {
            skip_indexes: std::sync::Arc::new(parking_lot::RwLock::new(HashMap::new())),
            bloom_filters: std::sync::Arc::new(parking_lot::RwLock::new(HashMap::new())),
            min_max_indexes: std::sync::Arc::new(parking_lot::RwLock::new(HashMap::new())),
        }
    }

    /// Create skip index for column
    pub fn create_skip_index(&self, column_id: u32, block_size: usize) {
        let mut indexes = self.skip_indexes.write();
        indexes.insert(column_id, SkipIndex::new(column_id, block_size));
    }

    /// Create bloom filter for column
    /// SECURITY: Validates parameters and handles errors
    pub fn create_bloom_filter(&self, column_id: u32, capacity: usize, false_positive_rate: f64) -> Result<()> {
        let mut filters = self.bloom_filters.write();
        let filter = BloomFilter::new(capacity, false_positive_rate)?;
        filters.insert(column_id, filter);
        Ok(())
    }

    /// Create min-max index for column
    pub fn create_min_max_index(&self, column_id: u32) {
        let mut indexes = self.min_max_indexes.write();
        indexes.insert(column_id, MinMaxIndex::new(column_id));
    }

    /// Check if value might exist (using bloom filter)
    /// SECURITY: Returns Result to handle validation errors
    pub fn might_contain(&self, column_id: u32, value: &[u8]) -> Result<bool> {
        let filters = self.bloom_filters.read();
        if let Some(filter) = filters.get(&column_id) {
            filter.might_contain(value)
        } else {
            Ok(true) // No filter, assume might contain
        }
    }

    /// Find blocks for range query (using skip index)
    /// SECURITY: Returns Result to handle validation errors
    pub fn find_blocks_for_range(&self, column_id: u32, min: &Bytes, max: &Bytes) -> Result<Vec<usize>> {
        let indexes = self.skip_indexes.read();
        if let Some(index) = indexes.get(&column_id) {
            index.find_blocks(min, max)
        } else {
            Ok(vec![]) // No index, return empty
        }
    }
}

