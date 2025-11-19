pub mod config;
pub mod error;
pub mod manager;
pub mod providers;
pub mod rag;
pub mod function_calling;
pub mod reasoning;
pub mod planning;
pub mod cache;

#[cfg(test)]
mod manager_tests;
#[cfg(test)]
mod cache_tests;
#[cfg(test)]
mod providers_tests;
#[cfg(test)]
mod function_calling_tests;
#[cfg(test)]
mod reasoning_tests;
#[cfg(test)]
mod planning_tests;

pub use config::*;
pub use error::*;
pub use manager::LLMManager;
pub use providers::Provider;
pub use rag::Memory as RAGMemory;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_enum() {
        assert_eq!(Provider::OpenAI.env_var_name(), "OPENAI_API_KEY");
        assert_eq!(Provider::Anthropic.env_var_name(), "ANTHROPIC_API_KEY");
        assert_eq!(Provider::Google.env_var_name(), "GOOGLE_API_KEY");
        assert_eq!(Provider::Cohere.env_var_name(), "COHERE_API_KEY");
    }

    #[test]
    fn test_message_role_hash() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();
        
        MessageRole::User.hash(&mut hasher1);
        MessageRole::User.hash(&mut hasher2);
        
        assert_eq!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn test_llm_config_default() {
        let config = LLMConfig::default();
        assert_eq!(config.temperature, 0.7);
        assert!(config.enable_caching);
        assert_eq!(config.cache_ttl_seconds, 3600);
    }

    #[test]
    fn test_llm_manager_creation() {
        let _manager = LLMManager::new();
        // Should create without panicking
        assert!(true); // Manager created successfully
    }

    #[test]
    fn test_cache_creation() {
        use crate::cache::ResponseCache;
        let _cache = ResponseCache::new(100);
        // Should create without panicking
        assert!(true);
    }

    #[test]
    fn test_cache_operations() {
        use crate::cache::ResponseCache;
        let cache = ResponseCache::new(10);
        
        // Test set and get
        cache.set("test_key", "test_value".to_string(), 3600);
        let value = cache.get("test_key");
        assert_eq!(value, Some("test_value".to_string()));
        
        // Test cache miss
        let miss = cache.get("nonexistent");
        assert_eq!(miss, None);
    }

    #[test]
    fn test_provider_from_str() {
        assert_eq!(Provider::from_str("openai"), Some(Provider::OpenAI));
        assert_eq!(Provider::from_str("anthropic"), Some(Provider::Anthropic));
        assert_eq!(Provider::from_str("google"), Some(Provider::Google));
        assert_eq!(Provider::from_str("cohere"), Some(Provider::Cohere));
        assert_eq!(Provider::from_str("invalid"), None);
    }

    #[test]
    fn test_brain_function_from_str() {
        use crate::function_calling::BrainFunction;
        
        assert!(BrainFunction::from_str("create_thought").is_some());
        assert!(BrainFunction::from_str("store_memory").is_some());
        assert!(BrainFunction::from_str("retrieve_memories").is_some());
        assert!(BrainFunction::from_str("store_experience").is_some());
        assert!(BrainFunction::from_str("get_thought").is_some());
        assert!(BrainFunction::from_str("get_memory").is_some());
        assert!(BrainFunction::from_str("invalid").is_none());
    }
}
