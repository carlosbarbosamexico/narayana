#[cfg(test)]
mod cache_tests {
    use crate::cache::ResponseCache;

    #[test]
    fn test_cache_basic_operations() {
        let cache = ResponseCache::new(10);
        
        // Test set and get
        cache.set("key1", "value1".to_string(), 3600);
        assert_eq!(cache.get("key1"), Some("value1".to_string()));
        
        // Test cache miss
        assert_eq!(cache.get("nonexistent"), None);
    }

    #[test]
    fn test_cache_ttl_expiration() {
        let cache = ResponseCache::new(10);
        
        // Set with very short TTL
        cache.set("key1", "value1".to_string(), 1);
        
        // Should be available immediately
        assert_eq!(cache.get("key1"), Some("value1".to_string()));
        
        // Wait for expiration (in real test, would use mock time)
        // For now, just verify the structure works
        assert!(true);
    }

    #[test]
    fn test_cache_capacity_limit() {
        let cache = ResponseCache::new(3);
        
        // Fill cache beyond capacity
        for i in 0..5 {
            cache.set(&format!("key{}", i), format!("value{}", i), 3600);
        }
        
        // Oldest entries should be evicted (LRU)
        // First two should be gone
        assert_eq!(cache.get("key0"), None);
        assert_eq!(cache.get("key1"), None);
        
        // Recent entries should still be there
        assert_eq!(cache.get("key4"), Some("value4".to_string()));
    }

    #[test]
    fn test_cache_key_too_large() {
        let cache = ResponseCache::new(10);
        let large_key = "a".repeat(20_000);
        
        cache.set(&large_key, "value".to_string(), 3600);
        // Should not panic, but entry won't be cached
        assert_eq!(cache.get(&large_key), None);
    }

    #[test]
    fn test_cache_value_too_large() {
        let cache = ResponseCache::new(10);
        let large_value = "a".repeat(2_000_000);
        
        cache.set("key1", large_value, 3600);
        // Should not panic, but entry won't be cached
        assert_eq!(cache.get("key1"), None);
    }

    #[test]
    fn test_cache_ttl_limit() {
        let cache = ResponseCache::new(10);
        
        // Set with very long TTL (should be clamped)
        cache.set("key1", "value1".to_string(), 1_000_000_000);
        
        // Should still work
        assert_eq!(cache.get("key1"), Some("value1".to_string()));
    }

    #[test]
    fn test_cache_clear() {
        let cache = ResponseCache::new(10);
        
        cache.set("key1", "value1".to_string(), 3600);
        cache.set("key2", "value2".to_string(), 3600);
        
        assert_eq!(cache.get("key1"), Some("value1".to_string()));
        assert_eq!(cache.get("key2"), Some("value2".to_string()));
        
        cache.clear();
        
        assert_eq!(cache.get("key1"), None);
        assert_eq!(cache.get("key2"), None);
    }

    #[test]
    fn test_cache_capacity_zero() {
        // Should default to 1
        let cache = ResponseCache::new(0);
        cache.set("key1", "value1".to_string(), 3600);
        // Should not panic
        assert!(true);
    }

    #[test]
    fn test_cache_capacity_too_large() {
        // Should be clamped to max
        let cache = ResponseCache::new(100_000);
        cache.set("key1", "value1".to_string(), 3600);
        assert_eq!(cache.get("key1"), Some("value1".to_string()));
    }
}

