use narayana_core::{Error, Result};
use std::collections::BTreeMap;
use parking_lot::RwLock;

/// Index for fast lookups
pub trait Index: Send + Sync {
    fn insert(&mut self, key: Vec<u8>, value: u64) -> Result<()>;
    fn lookup(&self, key: &[u8]) -> Result<Option<u64>>;
    fn range_scan(&self, start: &[u8], end: &[u8]) -> Result<Vec<u64>>;
}

/// B-tree index implementation
pub struct BTreeIndex {
    tree: RwLock<BTreeMap<Vec<u8>, u64>>,
}

impl BTreeIndex {
    pub fn new() -> Self {
        Self {
            tree: RwLock::new(BTreeMap::new()),
        }
    }
}

impl Index for BTreeIndex {
    fn insert(&mut self, key: Vec<u8>, value: u64) -> Result<()> {
        self.tree.write().insert(key, value);
        Ok(())
    }

    fn lookup(&self, key: &[u8]) -> Result<Option<u64>> {
        Ok(self.tree.read().get(key).copied())
    }

    fn range_scan(&self, start: &[u8], end: &[u8]) -> Result<Vec<u64>> {
        let tree = self.tree.read();
        let mut results = Vec::new();
        
        for (key, value) in tree.range(start.to_vec()..=end.to_vec()) {
            results.push(*value);
        }
        
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_insert_lookup() {
        let mut index = BTreeIndex::new();
        index.insert(b"key1".to_vec(), 100).unwrap();
        index.insert(b"key2".to_vec(), 200).unwrap();
        
        assert_eq!(index.lookup(b"key1").unwrap(), Some(100));
        assert_eq!(index.lookup(b"key2").unwrap(), Some(200));
        assert_eq!(index.lookup(b"key3").unwrap(), None);
    }

    #[test]
    fn test_index_range_scan() {
        let mut index = BTreeIndex::new();
        index.insert(b"a".to_vec(), 1).unwrap();
        index.insert(b"b".to_vec(), 2).unwrap();
        index.insert(b"c".to_vec(), 3).unwrap();
        index.insert(b"d".to_vec(), 4).unwrap();
        
        let results = index.range_scan(b"b", b"c").unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.contains(&2));
        assert!(results.contains(&3));
    }

    #[test]
    fn test_index_overwrite() {
        let mut index = BTreeIndex::new();
        index.insert(b"key".to_vec(), 100).unwrap();
        index.insert(b"key".to_vec(), 200).unwrap();
        
        assert_eq!(index.lookup(b"key").unwrap(), Some(200));
    }

    #[test]
    fn test_index_empty_range() {
        let index = BTreeIndex::new();
        let results = index.range_scan(b"a", b"z").unwrap();
        assert!(results.is_empty());
    }
}
