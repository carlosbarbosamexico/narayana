use dashmap::DashMap;
use std::sync::Arc;
use std::hash::Hash;
use std::time::{Duration, Instant};

struct CacheEntry<V> {
    value: V,
    inserted_at: Instant,
    last_accessed: Arc<parking_lot::RwLock<Instant>>,
}

pub struct LRUCache<K, V> {
    cache: DashMap<K, CacheEntry<V>>,
    max_size: usize,
    ttl: Option<Duration>,
}

impl<K, V> LRUCache<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: DashMap::new(),
            max_size,
            ttl: None,
        }
    }

    pub fn with_ttl(max_size: usize, ttl: Duration) -> Self {
        Self {
            cache: DashMap::new(),
            max_size,
            ttl: Some(ttl),
        }
    }

    pub fn get(&self, key: &K) -> Option<V> {
        if let Some(mut entry) = self.cache.get_mut(key) {
            // Check TTL
            if let Some(ttl) = self.ttl {
                if entry.inserted_at.elapsed() > ttl {
                    self.cache.remove(key);
                    return None;
                }
            }

            *entry.last_accessed.write() = Instant::now();
            Some(entry.value.clone())
        } else {
            None
        }
    }

    pub fn insert(&self, key: K, value: V) {
        // Evict if needed
        if self.cache.len() >= self.max_size {
            self.evict_lru();
        }

        let now = Instant::now();
        self.cache.insert(
            key,
            CacheEntry {
                value,
                inserted_at: now,
                last_accessed: Arc::new(parking_lot::RwLock::new(now)),
            },
        );
    }

    pub fn remove(&self, key: &K) -> Option<V> {
        self.cache.remove(key).map(|(_, entry)| entry.value)
    }

    pub fn clear(&self) {
        self.cache.clear();
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    fn evict_lru(&self) {
        // Simple eviction: remove oldest entry
        // In production, this would use a proper LRU data structure
        if let Some(oldest_key) = self.cache.iter().min_by_key(|entry| {
            *entry.last_accessed.read()
        }).map(|entry| entry.key().clone()) {
            self.cache.remove(&oldest_key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_cache_insert_get() {
        let cache = LRUCache::new(10);
        cache.insert("key1", "value1");
        cache.insert("key2", "value2");
        
        assert_eq!(cache.get(&"key1"), Some("value1"));
        assert_eq!(cache.get(&"key2"), Some("value2"));
        assert_eq!(cache.get(&"key3"), None);
    }

    #[test]
    fn test_cache_eviction() {
        let cache = LRUCache::new(2);
        cache.insert("key1", "value1");
        cache.insert("key2", "value2");
        cache.insert("key3", "value3"); // Should evict key1
        
        assert_eq!(cache.get(&"key1"), None);
        assert_eq!(cache.get(&"key2"), Some("value2"));
        assert_eq!(cache.get(&"key3"), Some("value3"));
    }

    #[test]
    fn test_cache_ttl() {
        let cache = LRUCache::with_ttl(10, Duration::from_millis(100));
        cache.insert("key1", "value1");
        
        assert_eq!(cache.get(&"key1"), Some("value1"));
        
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(cache.get(&"key1"), None);
    }

    #[test]
    fn test_cache_remove() {
        let cache = LRUCache::new(10);
        cache.insert("key1", "value1");
        assert_eq!(cache.remove(&"key1"), Some("value1"));
        assert_eq!(cache.get(&"key1"), None);
    }

    #[test]
    fn test_cache_clear() {
        let cache = LRUCache::new(10);
        cache.insert("key1", "value1");
        cache.insert("key2", "value2");
        assert_eq!(cache.len(), 2);
        
        cache.clear();
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.get(&"key1"), None);
    }

    #[test]
    fn test_cache_len() {
        let cache = LRUCache::new(10);
        assert_eq!(cache.len(), 0);
        cache.insert("key1", "value1");
        assert_eq!(cache.len(), 1);
        cache.insert("key2", "value2");
        assert_eq!(cache.len(), 2);
    }
}
