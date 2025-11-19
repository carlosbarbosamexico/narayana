// Brain wrapper to implement LLM traits for CognitiveBrain
// This avoids circular dependencies by implementing the traits in the server crate

use narayana_llm::{BrainInterface, BrainFunctionInterface, RAGMemory};
use narayana_storage::cognitive::{CognitiveBrain, Memory as StorageMemory, MemoryType};
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;

/// Wrapper that implements both BrainInterface and BrainFunctionInterface
pub struct BrainWrapper {
    brain: Arc<CognitiveBrain>,
}

impl BrainWrapper {
    pub fn new(brain: Arc<CognitiveBrain>) -> Self {
        Self { brain }
    }
}

#[async_trait]
impl BrainInterface for BrainWrapper {
    async fn retrieve_memories_semantic(
        &self,
        query_embedding: &[f32],
        k: usize,
        memory_type: Option<&str>,
        _thought_id: Option<&str>,
    ) -> std::result::Result<Vec<RAGMemory>, Box<dyn std::error::Error + Send + Sync>> {
        let memory_type_enum = memory_type.and_then(|s| {
            match s {
                "Episodic" => Some(MemoryType::Episodic),
                "Semantic" => Some(MemoryType::Semantic),
                "Procedural" => Some(MemoryType::Procedural),
                "Working" => Some(MemoryType::Working),
                "LongTerm" => Some(MemoryType::LongTerm),
                "Associative" => Some(MemoryType::Associative),
                "Emotional" => Some(MemoryType::Emotional),
                "Spatial" => Some(MemoryType::Spatial),
                "Temporal" => Some(MemoryType::Temporal),
                _ => None,
            }
        });
        
        let memories = self.brain
            .retrieve_memories_semantic(query_embedding, k, memory_type_enum, None)
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn std::error::Error + Send + Sync>)?;
        
        Ok(memories.into_iter().map(|m| RAGMemory {
            id: m.id,
            content: m.content,
        }).collect())
    }
    
    fn get_memory(&self, memory_id: &str) -> Option<RAGMemory> {
        let memories = self.brain.memories.read();
        memories.get(memory_id).map(|m| RAGMemory {
            id: m.id.clone(),
            content: m.content.clone(),
        })
    }
}

impl BrainFunctionInterface for BrainWrapper {
    fn create_thought(&self, content: Value, priority: f64) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>> {
        self.brain
            .create_thought(content, priority.clamp(0.0, 1.0))
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn std::error::Error + Send + Sync>)
    }
    
    fn store_memory(&self, memory_type: &str, content: Value, tags: Vec<String>) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let memory_type_enum = match memory_type {
            "Episodic" => MemoryType::Episodic,
            "Semantic" => MemoryType::Semantic,
            "Procedural" => MemoryType::Procedural,
            "Working" => MemoryType::Working,
            "LongTerm" => MemoryType::LongTerm,
            "Associative" => MemoryType::Associative,
            "Emotional" => MemoryType::Emotional,
            "Spatial" => MemoryType::Spatial,
            "Temporal" => MemoryType::Temporal,
            _ => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("Invalid memory type: {}", memory_type)))),
        };
        
        self.brain
            .store_memory(memory_type_enum, content, None, tags, None)
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn std::error::Error + Send + Sync>)
    }
    
    fn store_experience(&self, event_type: String, observation: Value, action: Option<Value>, outcome: Option<Value>, reward: Option<f64>) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>> {
        self.brain
            .store_experience(event_type, observation, action, outcome, reward, None)
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn std::error::Error + Send + Sync>)
    }
    
    fn get_thought(&self, thought_id: &str) -> Option<Value> {
        let thoughts = self.brain.thoughts.read();
        thoughts.get(thought_id).map(|t| {
            serde_json::json!({
                "id": t.id,
                "content": t.content,
                "state": format!("{:?}", t.state),
                "priority": t.priority,
                "created_at": t.created_at,
                "updated_at": t.updated_at,
            })
        })
    }
    
    fn get_memory(&self, memory_id: &str) -> Option<Value> {
        let memories = self.brain.memories.read();
        memories.get(memory_id).map(|m| {
            serde_json::json!({
                "id": m.id,
                "content": m.content,
                "memory_type": format!("{:?}", m.memory_type),
                "strength": m.strength,
                "tags": m.tags,
            })
        })
    }
}

