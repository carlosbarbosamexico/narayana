// Real Advanced Indexing: Skip Lists, Bloom Filters, Min-Max Indexes

use narayana_core::{Error, Result};
use std::collections::BTreeMap;
use parking_lot::RwLock;
use std::sync::Arc;
use rand::Rng;

/// Skip list node for fast range queries
struct SkipListNode {
    key: Vec<u8>,
    value: u64,
    forward: RwLock<Vec<Option<Arc<SkipListNode>>>>,
}

impl SkipListNode {
    fn new(key: Vec<u8>, value: u64, level: usize) -> Result<Self> {
        // SECURITY: Check for integer overflow in level + 1
        let forward_size = level.checked_add(1)
            .ok_or_else(|| Error::Storage(format!(
                "Integer overflow in SkipListNode::new: level {} + 1 exceeds usize::MAX",
                level
            )))?;
        
        Ok(Self {
            key,
            value,
            forward: RwLock::new(vec![None; forward_size]),
        })
    }
}

/// Skip list index for O(log n) range queries
pub struct SkipListIndex {
    head: Arc<SkipListNode>,
    level: RwLock<usize>,
    max_level: usize,
    probability: f64,
}

impl SkipListIndex {
    /// Create a new skip list index
    /// SECURITY: Validates max_level to prevent DoS attacks
    pub fn new(max_level: usize) -> Result<Self> {
        // SECURITY: Prevent DoS by limiting max level
        const MAX_ALLOWED_LEVEL: usize = 32; // Reasonable upper bound
        if max_level > MAX_ALLOWED_LEVEL {
            return Err(Error::Storage(format!(
                "max_level {} exceeds maximum allowed {}",
                max_level, MAX_ALLOWED_LEVEL
            )));
        }
        if max_level == 0 {
            return Err(Error::Storage("max_level must be greater than 0".to_string()));
        }
        
        let head = Arc::new(SkipListNode::new(vec![], 0, max_level)?);
        Ok(Self {
            head,
            level: RwLock::new(0),
            max_level,
            probability: 0.5,
        })
    }

    fn random_level(&self) -> usize {
        let mut level = 0;
        let mut rng = rand::thread_rng();
        while rng.gen::<f64>() < self.probability && level < self.max_level {
            level += 1;
        }
        level
    }

    pub fn insert(&self, key: Vec<u8>, value: u64) -> Result<()> {
        // SECURITY: Validate key size to prevent DoS
        const MAX_KEY_SIZE: usize = 1024 * 1024; // 1MB max key size
        if key.len() > MAX_KEY_SIZE {
            return Err(Error::Storage(format!(
                "Key size {} exceeds maximum allowed {} bytes",
                key.len(), MAX_KEY_SIZE
            )));
        }
        
        // SECURITY: Check for integer overflow in max_level + 1
        let update_size = self.max_level.checked_add(1)
            .ok_or_else(|| Error::Storage("Integer overflow in update vector size".to_string()))?;
        
        let mut update = vec![Arc::clone(&self.head); update_size];
        let mut current = Arc::clone(&self.head);
        
        // Find insertion point
        let current_level = *self.level.read();
        // SECURITY: Validate level is within bounds
        if current_level > self.max_level {
            return Err(Error::Storage("Current level exceeds max_level".to_string()));
        }
        
        for i in (0..=current_level).rev() {
            loop {
                let forward = current.forward.read();
                if i >= forward.len() {
                    return Err(Error::Storage(format!(
                        "Forward index {} out of bounds (len: {})",
                        i, forward.len()
                    )));
                }
                
                let should_continue = if let Some(ref next) = forward[i] {
                    if next.key.as_slice() < key.as_slice() {
                        let next_clone = Arc::clone(next);
                        drop(forward);
                        current = next_clone;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };
                
                if !should_continue {
                    break;
                }
            }
            update[i] = Arc::clone(&current);
        }

        // Insert new node
        let new_level = self.random_level();
        let current_level = *self.level.read();
        
        // SECURITY: Validate new_level is within bounds
        if new_level > self.max_level {
            return Err(Error::Storage(format!(
                "Random level {} exceeds max_level {}",
                new_level, self.max_level
            )));
        }
        
        if new_level > current_level {
            // SECURITY: Check for integer overflow
            let start_level = current_level.checked_add(1)
                .ok_or_else(|| Error::Storage("Integer overflow in level calculation".to_string()))?;
            
            for i in start_level..=new_level {
                // SECURITY: Validate index is within bounds
                if i >= update.len() {
                    return Err(Error::Storage(format!(
                        "Update index {} out of bounds (len: {})",
                        i, update.len()
                    )));
                }
                update[i] = Arc::clone(&self.head);
            }
            *self.level.write() = new_level;
        }

        // Create node with forward array initialized
        let mut forward = vec![None; new_level + 1];
        let mut update_forwards = Vec::new();
        
        for i in 0..=new_level {
            // SECURITY: Bounds check for update array
            if i >= update.len() {
                return Err(Error::Storage(format!(
                    "Update index {} out of bounds (len: {})",
                    i, update.len()
                )));
            }
            
            // Get the old forward value from update node
            let old_forward = update[i].forward.write()[i].take();
            forward[i] = old_forward;
            update_forwards.push((i, Arc::clone(&update[i])));
        }
        
        let new_node = Arc::new(SkipListNode {
            key: key.clone(),
            value,
            forward: RwLock::new(forward),
        });
        
        // Update forward pointers in update array
        for (i, update_node) in update_forwards {
            update_node.forward.write()[i] = Some(Arc::clone(&new_node));
        }

        Ok(())
    }

    pub fn lookup(&self, key: &[u8]) -> Result<Option<u64>> {
        // SECURITY: Validate key size to prevent DoS
        const MAX_KEY_SIZE: usize = 1024 * 1024; // 1MB max key size
        if key.len() > MAX_KEY_SIZE {
            return Err(Error::Storage(format!(
                "Key size {} exceeds maximum allowed {} bytes",
                key.len(), MAX_KEY_SIZE
            )));
        }
        
        // SECURITY: Limit traversal depth to prevent infinite loops
        const MAX_TRAVERSAL_DEPTH: usize = 1_000_000; // 1M nodes max
        let mut traversal_count = 0;
        
        let mut current = Arc::clone(&self.head);
        let current_level = *self.level.read();
        
        // SECURITY: Validate level is within bounds
        if current_level > self.max_level {
            return Err(Error::Storage("Current level exceeds max_level".to_string()));
        }
        
        for i in (0..=current_level).rev() {
            loop {
                let forward = current.forward.read();
                // SECURITY: Validate forward array bounds
                if i >= forward.len() {
                    return Err(Error::Storage(format!(
                        "Forward index {} out of bounds (len: {})",
                        i, forward.len()
                    )));
                }
                
                let should_continue = if let Some(ref next) = forward[i] {
                    // SECURITY: Prevent infinite loops
                    traversal_count += 1;
                    if traversal_count > MAX_TRAVERSAL_DEPTH {
                        return Err(Error::Storage(format!(
                            "Traversal depth {} exceeds maximum allowed {}",
                            traversal_count, MAX_TRAVERSAL_DEPTH
                        )));
                    }
                    
                    if next.key.as_slice() < key {
                        let next_clone = Arc::clone(next);
                        drop(forward);
                        current = next_clone;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };
                
                if !should_continue {
                    break;
                }
            }
        }

        // SECURITY: Validate forward[0] bounds
        let forward = current.forward.read();
        if forward.len() == 0 {
            return Ok(None);
        }
        
        if let Some(ref next) = forward[0] {
            if next.key.as_slice() == key {
                return Ok(Some(next.value));
            }
        }

        Ok(None)
    }

    pub fn range_scan(&self, start: &[u8], end: &[u8]) -> Result<Vec<u64>> {
        // SECURITY: Validate query parameters
        const MAX_QUERY_SIZE: usize = 1024 * 1024; // 1MB max
        if start.len() > MAX_QUERY_SIZE || end.len() > MAX_QUERY_SIZE {
            return Err(Error::Storage(format!(
                "Query size exceeds maximum allowed {} bytes",
                MAX_QUERY_SIZE
            )));
        }
        
        // SECURITY: Validate start <= end
        if start > end {
            return Err(Error::Storage("Start key must be <= end key".to_string()));
        }
        
        // SECURITY: Limit result size to prevent DoS
        const MAX_RESULTS: usize = 1_000_000; // 1M results max
        const MAX_TRAVERSAL_DEPTH: usize = 2_000_000; // 2M nodes max for traversal
        
        let mut results = Vec::new();
        let mut current = Arc::clone(&self.head);
        let mut traversal_count = 0;
        
        let current_level = *self.level.read();
        
        // SECURITY: Validate level is within bounds
        if current_level > self.max_level {
            return Err(Error::Storage("Current level exceeds max_level".to_string()));
        }
        
        // Find start position
        for i in (0..=current_level).rev() {
            loop {
                let forward = current.forward.read();
                // SECURITY: Validate forward array bounds
                if i >= forward.len() {
                    return Err(Error::Storage(format!(
                        "Forward index {} out of bounds (len: {})",
                        i, forward.len()
                    )));
                }
                
                let should_continue = if let Some(ref next) = forward[i] {
                    // SECURITY: Prevent infinite loops
                    traversal_count += 1;
                    if traversal_count > MAX_TRAVERSAL_DEPTH {
                        return Err(Error::Storage(format!(
                            "Traversal depth {} exceeds maximum allowed {}",
                            traversal_count, MAX_TRAVERSAL_DEPTH
                        )));
                    }
                    
                    if next.key.as_slice() < start {
                        let next_clone = Arc::clone(next);
                        drop(forward);
                        current = next_clone;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };
                
                if !should_continue {
                    break;
                }
            }
        }

        // Traverse from start to end
        loop {
            let forward = current.forward.read();
            // SECURITY: Validate forward[0] exists
            if forward.len() == 0 {
                return Ok(results);
            }
            
            let should_continue = if let Some(ref next) = forward[0] {
                // SECURITY: Prevent infinite loops
                traversal_count += 1;
                if traversal_count > MAX_TRAVERSAL_DEPTH {
                    return Err(Error::Storage(format!(
                        "Traversal depth {} exceeds maximum allowed {}",
                        traversal_count, MAX_TRAVERSAL_DEPTH
                    )));
                }
                
                if next.key.as_slice() > end {
                    false
                } else {
                    if next.key.as_slice() >= start {
                        results.push(next.value);
                        
                        // SECURITY: Early exit if too many results
                        if results.len() > MAX_RESULTS {
                            return Err(Error::Storage(format!(
                                "Range scan returned {} results, exceeds maximum allowed {}",
                                results.len(), MAX_RESULTS
                            )));
                        }
                    }
                    let next_clone = Arc::clone(next);
                    drop(forward);
                    current = next_clone;
                    true
                }
            } else {
                false
            };
            
            if !should_continue {
                break;
            }
        }

        Ok(results)
    }
}

/// Bloom filter for fast membership tests
pub struct BloomFilter {
    bits: Arc<RwLock<Vec<bool>>>,
    size: usize,
    hash_count: usize,
}

impl BloomFilter {
    /// Create a new bloom filter
    /// SECURITY: Validates parameters to prevent DoS attacks
    pub fn new(size: usize, hash_count: usize) -> Result<Self> {
        // SECURITY: Prevent DoS by limiting size
        const MAX_SIZE: usize = 100_000_000; // 100M bits max (~12.5MB)
        if size > MAX_SIZE {
            return Err(Error::Storage(format!(
                "Bloom filter size {} exceeds maximum allowed {}",
                size, MAX_SIZE
            )));
        }
        if size == 0 {
            return Err(Error::Storage("Bloom filter size must be greater than 0".to_string()));
        }
        
        // SECURITY: Limit hash count to prevent DoS
        const MAX_HASH_COUNT: usize = 20; // Reasonable upper bound
        if hash_count > MAX_HASH_COUNT {
            return Err(Error::Storage(format!(
                "Hash count {} exceeds maximum allowed {}",
                hash_count, MAX_HASH_COUNT
            )));
        }
        if hash_count == 0 {
            return Err(Error::Storage("Hash count must be greater than 0".to_string()));
        }
        
        Ok(Self {
            bits: Arc::new(RwLock::new(vec![false; size])),
            size,
            hash_count,
        })
    }

    fn hash(&self, item: &[u8], seed: u64) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        // SECURITY: Limit item size to prevent DoS
        const MAX_ITEM_SIZE: usize = 10 * 1024 * 1024; // 10MB max
        if item.len() > MAX_ITEM_SIZE {
            // Truncate to prevent DoS, but log warning
            // In production, would use proper logging
            let truncated = &item[..MAX_ITEM_SIZE.min(item.len())];
            let mut hasher = DefaultHasher::new();
            truncated.hash(&mut hasher);
            seed.hash(&mut hasher);
            (hasher.finish() as usize) % self.size
        } else {
        let mut hasher = DefaultHasher::new();
        item.hash(&mut hasher);
        seed.hash(&mut hasher);
        (hasher.finish() as usize) % self.size
    }
    }

    /// Insert item into bloom filter
    /// SECURITY: Validates item size to prevent DoS
    pub fn insert(&self, item: &[u8]) -> Result<()> {
        // SECURITY: Limit item size to prevent DoS
        const MAX_ITEM_SIZE: usize = 10 * 1024 * 1024; // 10MB max
        if item.len() > MAX_ITEM_SIZE {
            return Err(Error::Storage(format!(
                "Item size {} exceeds maximum allowed {} bytes",
                item.len(), MAX_ITEM_SIZE
            )));
        }
        
        let mut bits = self.bits.write();
        for i in 0..self.hash_count {
            let index = self.hash(item, i as u64);
            
            // SECURITY: Bounds check to prevent out-of-bounds access
            if index >= bits.len() {
                return Err(Error::Storage(format!(
                    "Hash index {} out of bounds (bits len: {})",
                    index, bits.len()
                )));
            }
            
            bits[index] = true;
        }
        Ok(())
    }

    /// Check if item might be in bloom filter
    /// SECURITY: Validates item size to prevent DoS
    pub fn might_contain(&self, item: &[u8]) -> Result<bool> {
        // SECURITY: Limit item size to prevent DoS
        const MAX_ITEM_SIZE: usize = 10 * 1024 * 1024; // 10MB max
        if item.len() > MAX_ITEM_SIZE {
            return Err(Error::Storage(format!(
                "Item size {} exceeds maximum allowed {} bytes",
                item.len(), MAX_ITEM_SIZE
            )));
        }
        
        let bits = self.bits.read();
        for i in 0..self.hash_count {
            let index = self.hash(item, i as u64);
            
            // SECURITY: Bounds check to prevent out-of-bounds access
            if index >= bits.len() {
                return Err(Error::Storage(format!(
                    "Hash index {} out of bounds (bits len: {})",
                    index, bits.len()
                )));
            }
            
            if !bits[index] {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub fn clear(&self) {
        let mut bits = self.bits.write();
        bits.fill(false);
    }
}

/// Min-max index for range pruning
pub struct MinMaxIndex {
    min_values: RwLock<BTreeMap<u64, Vec<u8>>>, // block_id -> min_value
    max_values: RwLock<BTreeMap<u64, Vec<u8>>>, // block_id -> max_value
}

impl MinMaxIndex {
    pub fn new() -> Self {
        Self {
            min_values: RwLock::new(BTreeMap::new()),
            max_values: RwLock::new(BTreeMap::new()),
        }
    }

    /// Insert min/max values for a block
    /// SECURITY: Validates value sizes to prevent DoS
    pub fn insert(&self, block_id: u64, min_value: Vec<u8>, max_value: Vec<u8>) -> Result<()> {
        // SECURITY: Limit value size to prevent DoS
        const MAX_VALUE_SIZE: usize = 10 * 1024 * 1024; // 10MB max per value
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
        
        self.min_values.write().insert(block_id, min_value);
        self.max_values.write().insert(block_id, max_value);
        Ok(())
    }

    /// Check if block can contain the query range
    /// SECURITY: Validates query parameters to prevent DoS
    pub fn can_contain(&self, block_id: u64, min_query: &[u8], max_query: &[u8]) -> Result<bool> {
        // SECURITY: Validate query parameter sizes
        const MAX_QUERY_SIZE: usize = 10 * 1024 * 1024; // 10MB max
        if min_query.len() > MAX_QUERY_SIZE {
            return Err(Error::Storage(format!(
                "Min query size {} exceeds maximum allowed {} bytes",
                min_query.len(), MAX_QUERY_SIZE
            )));
        }
        if max_query.len() > MAX_QUERY_SIZE {
            return Err(Error::Storage(format!(
                "Max query size {} exceeds maximum allowed {} bytes",
                max_query.len(), MAX_QUERY_SIZE
            )));
        }
        
        // SECURITY: Validate min_query <= max_query
        if min_query > max_query {
            return Err(Error::Storage("Min query must be <= max query".to_string()));
        }
        
        let min_values = self.min_values.read();
        let max_values = self.max_values.read();
        
        if let (Some(min_val), Some(max_val)) = (min_values.get(&block_id), max_values.get(&block_id)) {
            // Check if ranges overlap
            Ok(max_val.as_slice() >= min_query && min_val.as_slice() <= max_query)
        } else {
            Ok(false)
        }
    }

    /// Get min/max range for a block
    /// SECURITY: Validates value sizes before cloning to prevent DoS
    pub fn get_range(&self, block_id: u64) -> Result<Option<(Vec<u8>, Vec<u8>)>> {
        let min_values = self.min_values.read();
        let max_values = self.max_values.read();
        
        if let (Some(min_val), Some(max_val)) = (min_values.get(&block_id), max_values.get(&block_id)) {
            // SECURITY: Validate value sizes before cloning
            const MAX_VALUE_SIZE: usize = 10 * 1024 * 1024; // 10MB max per value
            if min_val.len() > MAX_VALUE_SIZE {
                return Err(Error::Storage(format!(
                    "Min value size {} exceeds maximum allowed {} bytes",
                    min_val.len(), MAX_VALUE_SIZE
                )));
            }
            if max_val.len() > MAX_VALUE_SIZE {
                return Err(Error::Storage(format!(
                    "Max value size {} exceeds maximum allowed {} bytes",
                    max_val.len(), MAX_VALUE_SIZE
                )));
            }
            
            Ok(Some((min_val.clone(), max_val.clone())))
        } else {
            Ok(None)
        }
    }
}

/// Composite index that uses multiple index types
pub struct CompositeIndex {
    btree: crate::index::BTreeIndex,
    skip_list: Option<SkipListIndex>,
    bloom: Option<BloomFilter>,
    min_max: Option<MinMaxIndex>,
}

impl CompositeIndex {
    /// Create a new composite index
    /// SECURITY: Validates all sub-index creation
    pub fn new(use_skip_list: bool, use_bloom: bool, use_min_max: bool) -> Result<Self> {
        Ok(Self {
            btree: crate::index::BTreeIndex::new(),
            skip_list: if use_skip_list {
                Some(SkipListIndex::new(16)?)
            } else {
                None
            },
            bloom: if use_bloom {
                Some(BloomFilter::new(10000, 3)?)
            } else {
                None
            },
            min_max: if use_min_max {
                Some(MinMaxIndex::new())
            } else {
                None
            },
        })
    }

    pub fn insert(&mut self, key: Vec<u8>, value: u64) -> Result<()> {
        // SECURITY: Validate key before inserting into all indexes
        const MAX_KEY_SIZE: usize = 1024 * 1024; // 1MB max key size
        if key.len() > MAX_KEY_SIZE {
            return Err(Error::Storage(format!(
                "Key size {} exceeds maximum allowed {} bytes",
                key.len(), MAX_KEY_SIZE
            )));
        }
        
        crate::index::Index::insert(&mut self.btree, key.clone(), value)?;
        
        if let Some(ref skip_list) = self.skip_list {
            skip_list.insert(key.clone(), value)?;
        }
        
        if let Some(ref bloom) = self.bloom {
            bloom.insert(&key)?;
        }

        Ok(())
    }

    pub fn lookup(&self, key: &[u8]) -> Result<Option<u64>> {
        // Use bloom filter for fast negative check
        if let Some(ref bloom) = self.bloom {
            // SECURITY: Handle Result from might_contain
            match bloom.might_contain(key) {
                Ok(false) => return Ok(None),
                Ok(true) => {}, // Continue lookup
                Err(e) => return Err(e), // Propagate validation error
            }
        }

        // Use skip list if available (faster for range queries)
        if let Some(ref skip_list) = self.skip_list {
            skip_list.lookup(key)
        } else {
            crate::index::Index::lookup(&self.btree, key)
        }
    }

    pub fn range_scan(&self, start: &[u8], end: &[u8]) -> Result<Vec<u64>> {
        // SECURITY: Validate range query parameters
        const MAX_QUERY_SIZE: usize = 1024 * 1024; // 1MB max
        if start.len() > MAX_QUERY_SIZE || end.len() > MAX_QUERY_SIZE {
            return Err(Error::Storage(format!(
                "Query size exceeds maximum allowed {} bytes",
                MAX_QUERY_SIZE
            )));
        }
        
        // SECURITY: Validate start <= end
        if start > end {
            return Err(Error::Storage("Start key must be <= end key".to_string()));
        }
        
        // SECURITY: Limit result size to prevent DoS
        const MAX_RESULTS: usize = 1_000_000; // 1M results max
        
        let results = if let Some(ref skip_list) = self.skip_list {
            skip_list.range_scan(start, end)?
        } else {
            crate::index::Index::range_scan(&self.btree, start, end)?
        };
        
        if results.len() > MAX_RESULTS {
            return Err(Error::Storage(format!(
                "Range scan returned {} results, exceeds maximum allowed {}",
                results.len(), MAX_RESULTS
            )));
        }
        
        Ok(results)
    }
}

impl crate::index::Index for CompositeIndex {
    fn insert(&mut self, key: Vec<u8>, value: u64) -> Result<()> {
        self.insert(key, value)
    }

    fn lookup(&self, key: &[u8]) -> Result<Option<u64>> {
        self.lookup(key)
    }

    fn range_scan(&self, start: &[u8], end: &[u8]) -> Result<Vec<u64>> {
        self.range_scan(start, end)
    }
}

