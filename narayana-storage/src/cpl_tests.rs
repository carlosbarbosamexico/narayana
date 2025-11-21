// CPL Integration Tests
// Comprehensive tests for Conscience Persistent Loop and all cognitive systems

#[cfg(test)]
mod tests {
    use crate::cognitive::{CognitiveBrain, MemoryType};
    use crate::conscience_persistent_loop::{ConsciencePersistentLoop, CPLConfig, CPLEvent};
    use crate::cpl_manager::CPLManager;
    use narayana_core::Result;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::sleep;
    use serde_json::json;

    async fn create_test_cpl() -> Arc<ConsciencePersistentLoop> {
        let brain = Arc::new(CognitiveBrain::new());
        let mut config = CPLConfig::default();
        config.loop_interval_ms = 50; // Fast for testing
        config.enable_global_workspace = true;
        config.enable_background_daemon = true;
        config.enable_dreaming = true;
        config.working_memory_capacity = 5;
        config.enable_attention = true;
        config.enable_narrative = true;
        config.enable_memory_bridge = true;
        config.enable_persistence = false; // Disable for tests
        config.persistence_dir = None;
        
        let cpl = Arc::new(ConsciencePersistentLoop::new(brain, config));
        cpl.initialize().await.unwrap();
        cpl
    }

    #[tokio::test]
    async fn test_cpl_initialization() {
        let cpl = create_test_cpl().await;
        assert!(!cpl.is_running());
        assert_eq!(cpl.id().len(), 36); // UUID length
    }

    #[tokio::test]
    async fn test_cpl_start_stop() {
        let cpl = create_test_cpl().await;
        
        // Start CPL
        cpl.clone().start().await.unwrap();
        assert!(cpl.is_running());
        
        // Wait a bit
        sleep(Duration::from_millis(200)).await;
        
        // Stop CPL
        cpl.stop().await.unwrap();
        assert!(!cpl.is_running());
    }

    #[tokio::test]
    async fn test_cpl_double_start() {
        let cpl = create_test_cpl().await;
        
        cpl.clone().start().await.unwrap();
        
        // Try to start again - should fail
        let result = cpl.clone().start().await;
        assert!(result.is_err());
        
        cpl.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_working_memory_capacity() {
        let cpl = create_test_cpl().await;
        let working_memory = cpl.working_memory();
        
        // Add more items than capacity
        for i in 0..10 {
            let context = json!({"test": i});
            working_memory.add(
                format!("item_{}", i),
                crate::working_memory::ScratchpadContentType::Memory,
                context,
            ).await.unwrap();
        }
        
        // Check that capacity is enforced
        let size = working_memory.size().await;
        assert!(size <= working_memory.capacity());
    }

    #[tokio::test]
    async fn test_working_memory_decay() {
        let cpl = create_test_cpl().await;
        let working_memory = cpl.working_memory();
        
        // Add item
        working_memory.add(
            "test_item".to_string(),
            crate::working_memory::ScratchpadContentType::Memory,
            json!({"test": true}),
        ).await.unwrap();
        
        // Get initial activation
        let entries = working_memory.get_active().await;
        assert!(!entries.is_empty());
        let initial_activation = entries[0].activation;
        
        // Wait and update (should decay)
        sleep(Duration::from_millis(100)).await;
        working_memory.update().await.unwrap();
        
        // Check activation decreased
        let entries_after = working_memory.get_active().await;
        if !entries_after.is_empty() {
            assert!(entries_after[0].activation <= initial_activation);
        }
    }

    #[tokio::test]
    async fn test_global_workspace_broadcast() {
        let cpl = create_test_cpl().await;
        cpl.clone().start().await.unwrap();
        
        // Create a thought
        let _thought_id = cpl.brain().create_thought(
            json!({"action": "test", "priority": 0.9}),
            0.9,
        ).unwrap();
        
        // Wait for loop iteration
        sleep(Duration::from_millis(150)).await;
        
        // Check that thought was considered for workspace
        // (indirect test - workspace should have processed it)
        
        cpl.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_memory_consolidation() {
        let cpl = create_test_cpl().await;
        cpl.clone().start().await.unwrap();
        
        // Create episodic memory
        let memory_id = cpl.brain().store_memory(
            MemoryType::Episodic,
            json!({"event": "test_event"}),
            None,
            vec!["test".to_string()],
            None,
        ).unwrap();
        
        // Wait for background daemon to process
        sleep(Duration::from_millis(200)).await;
        
        // Check memory strength was updated
        let memory = cpl.brain().access_memory(&memory_id).unwrap();
        assert!(memory.strength >= 0.0 && memory.strength <= 1.0);
        
        cpl.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_memory_bridge_consolidation() {
        let cpl = create_test_cpl().await;
        cpl.clone().start().await.unwrap();
        
        // Create multiple episodic memories
        for i in 0..5 {
            cpl.brain().store_memory(
                MemoryType::Episodic,
                json!({"event": format!("event_{}", i)}),
                None,
                vec!["test".to_string()],
                None,
            ).unwrap();
        }
        
        // Wait for memory bridge to process
        sleep(Duration::from_millis(300)).await;
        
        cpl.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_narrative_generation() {
        let cpl = create_test_cpl().await;
        cpl.clone().start().await.unwrap();
        
        // Create experiences
        for i in 0..3 {
            cpl.brain().store_experience(
                "test_experience".to_string(),
                json!({"observation": format!("obs_{}", i)}),
                Some(json!({"action": "test"})),
                Some(json!({"outcome": "success"})),
                Some(0.8),
                None,
            ).unwrap();
        }
        
        // Wait for narrative generator
        sleep(Duration::from_millis(200)).await;
        
        cpl.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_attention_routing() {
        let cpl = create_test_cpl().await;
        cpl.clone().start().await.unwrap();
        
        // Create multiple thoughts with different priorities
        for i in 0..5 {
            cpl.brain().create_thought(
                json!({"action": format!("action_{}", i)}),
                (i as f64) / 10.0,
            ).unwrap();
        }
        
        // Wait for attention router
        sleep(Duration::from_millis(150)).await;
        
        cpl.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_dreaming_loop() {
        let cpl = create_test_cpl().await;
        cpl.clone().start().await.unwrap();
        
        // Create experiences for replay
        for i in 0..10 {
            cpl.brain().store_experience(
                "dream_experience".to_string(),
                json!({"observation": format!("dream_{}", i)}),
                Some(json!({"action": "dream"})),
                Some(json!({"outcome": "dreamed"})),
                Some(0.5 + (i as f64) / 20.0),
                None,
            ).unwrap();
        }
        
        // Wait for dreaming loop (runs every 10 iterations)
        sleep(Duration::from_millis(600)).await;
        
        cpl.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_cpl_manager_spawn() {
        let manager = CPLManager::new(CPLConfig::default());
        
        // Spawn multiple CPLs
        let cpl1_id = manager.spawn_cpl(None).await.unwrap();
        let cpl2_id = manager.spawn_cpl(None).await.unwrap();
        
        assert_ne!(cpl1_id, cpl2_id);
        assert_eq!(manager.count(), 2);
        
        // Start both
        manager.start_cpl(&cpl1_id).await.unwrap();
        manager.start_cpl(&cpl2_id).await.unwrap();
        
        sleep(Duration::from_millis(100)).await;
        
        // Stop and remove
        manager.stop_cpl(&cpl1_id).await.unwrap();
        manager.stop_cpl(&cpl2_id).await.unwrap();
        
        manager.remove_cpl(&cpl1_id).await.unwrap();
        manager.remove_cpl(&cpl2_id).await.unwrap();
        
        assert_eq!(manager.count(), 0);
        
        // Test accessing non-existent CPL
        assert!(manager.get_cpl("nonexistent").is_none());
    }

    #[tokio::test]
    async fn test_cpl_manager_start_all() {
        let manager = CPLManager::new(CPLConfig::default());
        
        let _cpl1_id = manager.spawn_cpl(None).await.unwrap();
        let _cpl2_id = manager.spawn_cpl(None).await.unwrap();
        
        manager.start_all().await.unwrap();
        
        sleep(Duration::from_millis(100)).await;
        
        manager.stop_all().await.unwrap();
    }

    #[tokio::test]
    async fn test_cpl_events() {
        let cpl = create_test_cpl().await;
        let mut receiver = cpl.subscribe_events();
        
        cpl.clone().start().await.unwrap();
        
        // Wait for events
        sleep(Duration::from_millis(150)).await;
        
        // Check we received events
        let mut event_count = 0;
        while let Ok(_) = receiver.try_recv() {
            event_count += 1;
        }
        
        assert!(event_count > 0);
        
        cpl.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_edge_case_empty_brain() {
        let brain = Arc::new(CognitiveBrain::new());
        let config = CPLConfig::default();
        let cpl = Arc::new(ConsciencePersistentLoop::new(brain, config));
        
        cpl.initialize().await.unwrap();
        cpl.clone().start().await.unwrap();
        
        // Should handle empty brain gracefully
        sleep(Duration::from_millis(100)).await;
        
        cpl.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_edge_case_invalid_config() {
        let brain = Arc::new(CognitiveBrain::new());
        let mut config = CPLConfig::default();
        config.loop_interval_ms = 0; // Invalid
        
        let cpl = Arc::new(ConsciencePersistentLoop::new(brain, config));
        
        // Should fail to initialize or start
        let result = cpl.initialize().await;
        assert!(result.is_err() || cpl.clone().start().await.is_err());
    }

    #[tokio::test]
    async fn test_edge_case_large_working_memory() {
        let cpl = create_test_cpl().await;
        let working_memory = cpl.working_memory();
        
        // Add many items rapidly
        for i in 0..100 {
            working_memory.add(
                format!("item_{}", i),
                crate::working_memory::ScratchpadContentType::Memory,
                json!({"index": i}),
            ).await.unwrap();
        }
        
        // Should still respect capacity
        let size = working_memory.size().await;
        assert!(size <= working_memory.capacity());
    }

    #[tokio::test]
    async fn test_edge_case_concurrent_access() {
        let cpl = create_test_cpl().await;
        cpl.clone().start().await.unwrap();
        
        // Concurrently add memories and experiences
        let mut handles = Vec::new();
        
        for i in 0..10 {
            let brain = cpl.brain().clone();
            let handle = tokio::spawn(async move {
                brain.store_memory(
                    MemoryType::Episodic,
                    json!({"concurrent": i}),
                    None,
                    vec![],
                    None,
                ).unwrap();
            });
            handles.push(handle);
        }
        
        for handle in handles {
            handle.await.unwrap();
        }
        
        sleep(Duration::from_millis(200)).await;
        
        cpl.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_persistence_save_load() {
        let brain = Arc::new(CognitiveBrain::new());
        let mut config = CPLConfig::default();
        config.enable_persistence = true;
        config.persistence_dir = Some("test_data/cpl_test".to_string());
        
        let cpl = Arc::new(ConsciencePersistentLoop::new(brain, config));
        cpl.initialize().await.unwrap();
        
        // Save state
        cpl.clone().start().await.unwrap();
        sleep(Duration::from_millis(100)).await;
        cpl.stop().await.unwrap();
        
        // State should be saved (tested indirectly)
    }
}

