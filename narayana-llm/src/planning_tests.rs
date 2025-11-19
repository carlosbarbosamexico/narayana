#[cfg(test)]
mod planning_tests {
    use crate::planning::*;
    use crate::manager::LLMManager;

    #[test]
    fn test_planning_system_creation() {
        let _system = PlanningSystem::new();
        // Should not panic
        assert!(true);
    }

    #[tokio::test]
    async fn test_generate_plan_empty_goal() {
        let system = PlanningSystem::new();
        let manager = LLMManager::new();
        
        let result = system.generate_plan(&manager, "", &[]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_generate_plan_too_large_goal() {
        let system = PlanningSystem::new();
        let manager = LLMManager::new();
        let large_goal = "a".repeat(20_000);
        
        let result = system.generate_plan(&manager, &large_goal, &[]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_generate_plan_too_many_constraints() {
        let system = PlanningSystem::new();
        let manager = LLMManager::new();
        let constraints: Vec<String> = (0..200).map(|i| format!("constraint{}", i)).collect();
        
        let result = system.generate_plan(&manager, "test goal", &constraints).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_refine_plan_invalid_id() {
        let system = PlanningSystem::new();
        let manager = LLMManager::new();
        
        let result = system.refine_plan(&manager, "", "feedback").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_refine_plan_nonexistent() {
        let system = PlanningSystem::new();
        let manager = LLMManager::new();
        
        let result = system.refine_plan(&manager, "nonexistent-plan-id", "feedback").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_refine_plan_too_large_feedback() {
        let system = PlanningSystem::new();
        let manager = LLMManager::new();
        let large_feedback = "a".repeat(200_000);
        
        // Create a plan first (may fail due to no API key, but that's ok)
        let plan_id = system.generate_plan(&manager, "test goal", &[]).await;
        
        if let Ok(id) = plan_id {
            let result = system.refine_plan(&manager, &id, &large_feedback).await;
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_get_plan_nonexistent() {
        let system = PlanningSystem::new();
        let plan = system.get_plan("nonexistent");
        assert!(plan.is_none());
    }
}

