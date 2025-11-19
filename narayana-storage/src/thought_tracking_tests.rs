#[cfg(test)]
mod thought_tracking_tests {
    use crate::cognitive::{CognitiveBrain, MemoryType, MemoryAccessType};
    use crate::thought_serialization::{ThoughtReplaySystem, TimelineEventType};
    use crate::thought_kernel::{ThoughtKernel, ThoughtContext};
    use serde_json::json;
    use std::sync::Arc;
    use std::collections::HashMap;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_memory_access_tracking() {
        let brain = Arc::new(CognitiveBrain::new());
        let thought_id = brain.create_thought(json!({"task": "test"}), 1.0).unwrap();
        
        // Store memory with thought_id to track access
        let memory_id = brain.store_memory(
            MemoryType::Episodic,
            json!({"event": "test"}),
            Some(vec![0.1, 0.2, 0.3]),
            vec!["test".to_string()],
            Some(&thought_id),
        ).unwrap();
        
        // Retrieve memory to track read access
        let query_embedding = vec![0.1, 0.2, 0.3];
        brain.retrieve_memories_semantic(&query_embedding, 1, None, Some(&thought_id)).unwrap();
        
        // Check that memory accesses were tracked
        let thoughts = brain.thoughts.read();
        let thought = thoughts.get(&thought_id).unwrap();
        
        assert!(!thought.memory_accesses.is_empty());
        assert!(thought.memory_accesses.iter().any(|acc| acc.memory_id == memory_id));
        assert!(thought.memory_accesses.iter().any(|acc| acc.access_type == MemoryAccessType::Write));
        assert!(thought.memory_accesses.iter().any(|acc| acc.access_type == MemoryAccessType::Read));
    }

    #[test]
    fn test_spawned_thought_tracking() {
        let brain = Arc::new(CognitiveBrain::new());
        let kernel = ThoughtKernel::new(brain.clone());
        
        let parent_id = brain.create_thought(json!({"task": "parent"}), 1.0).unwrap();
        
        let ctx = ThoughtContext {
            content: json!({"task": "child"}),
            priority: 0.8,
            parent_thought_id: Some(parent_id.clone()),
            deadline: None,
            gpu_required: false,
            context: HashMap::new(),
            shared_memory_id: None,
        };
        
        // Spawn child thought
        let _child_result = tokio::runtime::Runtime::new().unwrap().block_on(
            kernel.spawn_thought(ctx, |_ctx, _input| {
                Ok(json!({"result": "done"}))
            })
        ).unwrap();
        
        // Check that parent tracks spawned child
        let thoughts = brain.thoughts.read();
        let parent = thoughts.get(&parent_id).unwrap();
        
        assert!(!parent.spawned_thoughts.is_empty());
    }

    #[test]
    fn test_causality_chain_building() {
        let brain = Arc::new(CognitiveBrain::new());
        let replay = ThoughtReplaySystem::new(brain.clone());
        
        // Create parent thought
        let parent_id = brain.create_thought(json!({"task": "parent"}), 1.0).unwrap();
        
        // Create child thought with association
        let child_id = brain.create_thought(json!({"task": "child"}), 0.8).unwrap();
        
        // Create association
        brain.create_association(&parent_id, &child_id).unwrap();
        
        // Store memory accessed by child
        let memory_id = brain.store_memory(
            MemoryType::Semantic,
            json!({"data": "test"}),
            Some(vec![0.1, 0.2]),
            vec![],
            Some(&child_id),
        ).unwrap();
        
        // Get child thought and build causality chain
        let thoughts = brain.thoughts.read();
        let child = thoughts.get(&child_id).unwrap().clone();
        drop(thoughts);
        
        // Serialize and get trace, which includes causality chain
        let trace = replay.serialize_thought_trace(&child_id).unwrap();
        let chain = trace.causality_chain;
        
        // Should have links for association, memory access
        assert!(!chain.is_empty());
        assert!(chain.iter().any(|link| link.from == parent_id && link.to == child_id));
        assert!(chain.iter().any(|link| link.to == memory_id));
    }

    #[test]
    fn test_timeline_reconstruction() {
        let brain = Arc::new(CognitiveBrain::new());
        let replay = ThoughtReplaySystem::new(brain.clone());
        
        let thought_id = brain.create_thought(json!({"task": "test"}), 1.0).unwrap();
        
        // Store memory to create memory access event
        brain.store_memory(
            MemoryType::Episodic,
            json!({"event": "test"}),
            Some(vec![0.1, 0.2]),
            vec![],
            Some(&thought_id),
        ).unwrap();
        
        // Get thought and build timeline
        let thoughts = brain.thoughts.read();
        let thought = thoughts.get(&thought_id).unwrap().clone();
        drop(thoughts);
        
        // Serialize to get timeline
        let trace = replay.serialize_thought_trace(&thought_id).unwrap();
        let timeline = trace.timeline;
        
        // Should have created, memory accessed, and completed events
        assert!(timeline.len() >= 2);
        assert!(timeline.iter().any(|e| matches!(e.event_type, TimelineEventType::ThoughtCreated)));
        assert!(timeline.iter().any(|e| matches!(e.event_type, TimelineEventType::MemoryAccessed)));
        
        // Timeline should be sorted by timestamp
        for i in 1..timeline.len() {
            assert!(timeline[i-1].timestamp <= timeline[i].timestamp);
        }
    }
}

