use lru::LruCache;
use parking_lot::RwLock;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::Arc;

#[derive(Debug, Clone)]
struct CacheEntry {
    response: String,
    timestamp: u64,
    ttl: u64,
}

pub struct ResponseCache {
    cache: Arc<RwLock<LruCache<u64, CacheEntry>>>,
}

impl ResponseCache {
    pub fn new(capacity: usize) -> Self {
        // Limit capacity to prevent memory exhaustion
        let capacity = capacity.min(10000).max(1);
        let capacity = NonZeroUsize::new(capacity)
            .expect("Capacity should be at least 1");
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(capacity))),
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        // Validate key size to prevent DoS
        if key.len() > 10000 {
            return None;
        }
        
        let hash = self.hash_key(key);
        let mut cache = self.cache.write();
        
        if let Some(entry) = cache.get(&hash) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            // Check for overflow in timestamp calculation
            if entry.timestamp > now {
                // Clock went backwards or entry is from future - invalidate
                cache.pop(&hash);
                return None;
            }
            
            let age = now.saturating_sub(entry.timestamp);
            if age < entry.ttl {
                return Some(entry.response.clone());
            } else {
                cache.pop(&hash);
            }
        }
        None
    }

    pub fn set(&self, key: &str, response: String, ttl: u64) {
        // Validate inputs
        if key.len() > 10000 {
            tracing::warn!("Cache key too long, skipping");
            return;
        }
        
        // Limit response size to prevent memory exhaustion
        if response.len() > 1_000_000 {
            tracing::warn!("Cache response too large ({} bytes), skipping", response.len());
            return;
        }
        
        // Limit TTL to prevent stale cache entries
        let ttl = ttl.min(86400 * 7); // Max 7 days
        
        let hash = self.hash_key(key);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let entry = CacheEntry {
            response,
            timestamp: now,
            ttl,
        };
        
        self.cache.write().put(hash, entry);
    }

    fn hash_key(&self, key: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    pub fn clear(&self) {
        self.cache.write().clear();
    }
}

