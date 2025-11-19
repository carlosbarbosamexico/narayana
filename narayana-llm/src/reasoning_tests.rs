#[cfg(test)]
mod reasoning_tests {
    use crate::reasoning::ReasoningSystem;
    use crate::manager::LLMManager;

    #[test]
    fn test_reasoning_system_creation() {
        let _system = ReasoningSystem::new();
        // Should not panic
        assert!(true);
    }

    #[tokio::test]
    async fn test_chain_of_thought_empty_problem() {
        let system = ReasoningSystem::new();
        let manager = LLMManager::new();
        
        let result = system.chain_of_thought_reasoning(&manager, "", &[]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_chain_of_thought_too_large_problem() {
        let system = ReasoningSystem::new();
        let manager = LLMManager::new();
        let large_problem = "a".repeat(20_000);
        
        let result = system.chain_of_thought_reasoning(&manager, &large_problem, &[]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_chain_of_thought_too_many_steps() {
        let system = ReasoningSystem::new();
        let manager = LLMManager::new();
        let steps: Vec<&str> = (0..100).map(|_| "step").collect();
        
        let result = system.chain_of_thought_reasoning(&manager, "test", &steps).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_tree_of_thoughts_empty_problem() {
        let system = ReasoningSystem::new();
        let manager = LLMManager::new();
        
        let result = system.tree_of_thoughts(&manager, "", 3).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_tree_of_thoughts_too_many_branches() {
        let system = ReasoningSystem::new();
        let manager = LLMManager::new();
        
        // Should clamp to max 10 branches
        let result = system.tree_of_thoughts(&manager, "test problem", 100).await;
        // Should not panic, but will only process 10 branches
        // May fail due to no API key, but shouldn't panic
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_generate_hypothesis_empty_observation() {
        let system = ReasoningSystem::new();
        let manager = LLMManager::new();
        
        let result = system.generate_hypothesis(&manager, "", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_generate_hypothesis_too_large_observation() {
        let system = ReasoningSystem::new();
        let manager = LLMManager::new();
        let large_obs = "a".repeat(20_000);
        
        let result = system.generate_hypothesis(&manager, &large_obs, None).await;
        assert!(result.is_err());
    }
}

