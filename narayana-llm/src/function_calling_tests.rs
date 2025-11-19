#[cfg(test)]
mod function_calling_tests {
    use crate::function_calling::*;
    use crate::error::LLMError;
    use serde_json::json;

    // Mock brain implementation for testing
    struct MockBrain;

    impl crate::function_calling::BrainFunctionInterface for MockBrain {
        fn create_thought(&self, _content: serde_json::Value, priority: f64) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>> {
            assert!(priority >= 0.0 && priority <= 1.0);
            Ok("thought-123".to_string())
        }
        
        fn store_memory(&self, memory_type: &str, _content: serde_json::Value, _tags: Vec<String>) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>> {
            assert!(!memory_type.is_empty());
            Ok("memory-123".to_string())
        }
        
        fn store_experience(&self, event_type: String, _observation: serde_json::Value, _action: Option<serde_json::Value>, _outcome: Option<serde_json::Value>, _reward: Option<f64>) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>> {
            assert!(!event_type.is_empty());
            Ok("experience-123".to_string())
        }
        
        fn get_thought(&self, thought_id: &str) -> Option<serde_json::Value> {
            if thought_id == "existing-thought" {
                Some(json!({"id": thought_id, "content": "test"}))
            } else {
                None
            }
        }
        
        fn get_memory(&self, memory_id: &str) -> Option<serde_json::Value> {
            if memory_id == "existing-memory" {
                Some(json!({"id": memory_id, "content": "test"}))
            } else {
                None
            }
        }
    }

    #[test]
    fn test_brain_function_from_str() {
        assert!(BrainFunction::from_str("create_thought").is_some());
        assert!(BrainFunction::from_str("store_memory").is_some());
        assert!(BrainFunction::from_str("retrieve_memories").is_some());
        assert!(BrainFunction::from_str("store_experience").is_some());
        assert!(BrainFunction::from_str("get_thought").is_some());
        assert!(BrainFunction::from_str("get_memory").is_some());
        assert!(BrainFunction::from_str("invalid").is_none());
    }

    #[test]
    fn test_brain_function_to_function_definition() {
        let func = BrainFunction::CreateThought;
        let def = func.to_function_definition();
        assert_eq!(def.name, "create_thought");
        assert!(!def.description.is_empty());
    }

    #[test]
    fn test_function_calling_system_creation() {
        let brain = std::sync::Arc::new(MockBrain);
        let _system = FunctionCallingSystem::new(brain);
        // Should not panic
        assert!(true);
    }

    #[test]
    fn test_get_brain_functions() {
        let functions = FunctionCallingSystem::get_brain_functions();
        assert!(!functions.is_empty());
        assert!(functions.len() >= 6);
    }

    #[tokio::test]
    async fn test_execute_function_call_invalid_name() {
        let brain = std::sync::Arc::new(MockBrain);
        let system = FunctionCallingSystem::new(brain);
        
        let result = system.execute_function_call("", "{}").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_function_call_invalid_json() {
        let brain = std::sync::Arc::new(MockBrain);
        let system = FunctionCallingSystem::new(brain);
        
        let result = system.execute_function_call("create_thought", "invalid json").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_function_call_too_large() {
        let brain = std::sync::Arc::new(MockBrain);
        let system = FunctionCallingSystem::new(brain);
        
        let large_args = "a".repeat(20_000);
        let result = system.execute_function_call("create_thought", &large_args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_function_call_create_thought() {
        let brain = std::sync::Arc::new(MockBrain);
        let system = FunctionCallingSystem::new(brain);
        
        let args = json!({
            "content": {"task": "test"},
            "priority": 0.8
        });
        
        let result = system.execute_function_call("create_thought", &serde_json::to_string(&args).unwrap()).await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert!(value.get("thought_id").is_some());
    }

    #[tokio::test]
    async fn test_execute_function_call_store_memory() {
        let brain = std::sync::Arc::new(MockBrain);
        let system = FunctionCallingSystem::new(brain);
        
        let args = json!({
            "memory_type": "Episodic",
            "content": {"event": "test"},
            "tags": ["test"]
        });
        
        let result = system.execute_function_call("store_memory", &serde_json::to_string(&args).unwrap()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_function_call_invalid_memory_type() {
        let brain = std::sync::Arc::new(MockBrain);
        let system = FunctionCallingSystem::new(brain);
        
        let args = json!({
            "memory_type": "InvalidType",
            "content": {"event": "test"}
        });
        
        let result = system.execute_function_call("store_memory", &serde_json::to_string(&args).unwrap()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_function_call_get_thought() {
        let brain = std::sync::Arc::new(MockBrain);
        let system = FunctionCallingSystem::new(brain);
        
        let args = json!({
            "thought_id": "existing-thought"
        });
        
        let result = system.execute_function_call("get_thought", &serde_json::to_string(&args).unwrap()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_function_call_get_thought_not_found() {
        let brain = std::sync::Arc::new(MockBrain);
        let system = FunctionCallingSystem::new(brain);
        
        let args = json!({
            "thought_id": "nonexistent"
        });
        
        let result = system.execute_function_call("get_thought", &serde_json::to_string(&args).unwrap()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_function_call_priority_clamping() {
        let brain = std::sync::Arc::new(MockBrain);
        let system = FunctionCallingSystem::new(brain);
        
        // Test priority > 1.0 (should be clamped)
        let args = json!({
            "content": {"task": "test"},
            "priority": 2.0
        });
        
        let result = system.execute_function_call("create_thought", &serde_json::to_string(&args).unwrap()).await;
        // Should succeed with clamped priority
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_function_call_tags_limit() {
        let brain = std::sync::Arc::new(MockBrain);
        let system = FunctionCallingSystem::new(brain);
        
        // Test with many tags (should be limited)
        let tags: Vec<String> = (0..100).map(|i| format!("tag{}", i)).collect();
        let args = json!({
            "memory_type": "Episodic",
            "content": {"event": "test"},
            "tags": tags
        });
        
        let result = system.execute_function_call("store_memory", &serde_json::to_string(&args).unwrap()).await;
        // Should succeed with limited tags
        assert!(result.is_ok());
    }
}

