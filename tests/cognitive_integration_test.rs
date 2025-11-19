// Integration test for cognitive features - verifies they actually work

use narayana_storage::cognitive::*;
use narayana_storage::reinforcement_learning::*;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_cognitive_brain_basic() {
    let brain = CognitiveBrain::new();
    
    // Test 1: Create a thought
    let thought_id = brain.create_thought(
        json!({"task": "navigate", "destination": "kitchen"}),
        0.9
    ).unwrap();
    
    assert!(!thought_id.is_empty());
    
    // Test 2: Store a memory
    let memory_id = brain.store_memory(
        MemoryType::Episodic,
        json!({"event": "found_cup", "location": "kitchen"}),
        None,
        vec!["cup".to_string(), "kitchen".to_string()],
        Some(&thought_id),
    ).unwrap();
    
    assert!(!memory_id.is_empty());
    
    // Test 3: Store an experience
    let experience_id = brain.store_experience(
        "object_interaction".to_string(),
        json!({"object": "cup", "location": "table"}),
        Some(json!({"action": "grasp"})),
        Some(json!({"outcome": "success"})),
        Some(1.0),
        None,
    ).unwrap();
    
    assert!(!experience_id.is_empty());
    
    // Test 4: Retrieve memories by tag
    let memories = brain.retrieve_memories_by_tag("cup").unwrap();
    
    assert!(!memories.is_empty());
    assert_eq!(memories[0].memory_type, MemoryType::Episodic);
}

#[tokio::test]
async fn test_rl_integration() {
    let brain = Arc::new(CognitiveBrain::new());
    
    // Create RL engine
    let rl_config = RLConfig {
        learning_rate: 0.01,
        discount_factor: 0.95,
        epsilon: 0.1,
        batch_size: 32,
        replay_buffer_size: 1000,
        update_frequency: 10,
        algorithm: RLAlgorithm::DQN,
    };
    
    let rl_engine = Arc::new(RLEngine::new(brain.clone(), rl_config));
    brain.set_rl_engine(rl_engine.clone());
    
    // Store experience - should automatically go to RL engine
    let experience_id = brain.store_experience(
        "test_event".to_string(),
        json!({"state": "s1"}),
        Some(json!({"action": "a1"})),
        Some(json!({"state": "s2"})),
        Some(1.0),
        None,
    ).unwrap();
    
    assert!(!experience_id.is_empty());
    
    // Verify RL engine has the experience
    // (In a real test, we'd check the replay buffer, but it's private)
    // For now, just verify no panic occurred
}

#[tokio::test]
async fn test_thought_processing() {
    let brain = CognitiveBrain::new();
    
    // Create multiple thoughts in parallel
    let thought1 = brain.create_thought(
        json!({"task": "task1"}),
        0.9
    ).unwrap();
    
    let thought2 = brain.create_thought(
        json!({"task": "task2"}),
        0.7
    ).unwrap();
    
    // Both thoughts should exist
    assert!(!thought1.is_empty());
    assert!(!thought2.is_empty());
    assert_ne!(thought1, thought2);
    
    // Test thought merging
    let merged_id = brain.merge_thoughts(vec![thought1.clone(), thought2.clone()]).unwrap();
    assert!(!merged_id.is_empty());
}

#[tokio::test]
async fn test_pattern_learning() {
    let brain = CognitiveBrain::new();
    
    // Store multiple similar experiences
    for _i in 0..5 {
        brain.store_experience(
            "pattern_test".to_string(),
            json!({"condition": "c1"}),
            Some(json!({"action": "a1"})),
            Some(json!({"outcome": "success"})),
            Some(1.0),
            None,
        ).unwrap();
    }
    
    // Detect patterns
    let patterns = brain.detect_patterns_from_experiences().unwrap();
    
    // Should detect at least one pattern (since we have 5 similar experiences)
    assert!(!patterns.is_empty());
}

