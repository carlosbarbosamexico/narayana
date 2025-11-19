use crate::config::*;
use crate::error::Result;
use crate::manager::LLMManager;
use std::sync::Arc;

// Trait for brain interface to avoid circular dependency
#[async_trait::async_trait]
pub trait BrainInterface: Send + Sync {
    async fn retrieve_memories_semantic(
        &self,
        query_embedding: &[f32],
        k: usize,
        memory_type: Option<&str>,
        thought_id: Option<&str>,
    ) -> std::result::Result<Vec<Memory>, Box<dyn std::error::Error + Send + Sync>>;
    
    fn get_memory(&self, memory_id: &str) -> Option<Memory>;
}

#[derive(Debug, Clone)]
pub struct Memory {
    pub id: String,
    pub content: serde_json::Value,
}

pub struct RAGSystem {
    brain: Arc<dyn BrainInterface>,
}

impl RAGSystem {
    pub fn new(brain: Arc<dyn BrainInterface>) -> Self {
        Self { brain }
    }

    /// Retrieve and generate: RAG pattern
    /// Retrieves relevant memories and uses them as context for LLM generation
    pub async fn retrieve_and_generate(
        &self,
        llm_manager: &LLMManager,
        query: &str,
        k_memories: usize,
    ) -> Result<String> {
        // Input validation
        if query.is_empty() {
            return Err(crate::error::LLMError::InvalidResponse("Query cannot be empty".to_string()));
        }
        
        if query.len() > 10000 {
            return Err(crate::error::LLMError::InvalidResponse("Query too long (max 10000 chars)".to_string()));
        }
        
        // Limit k_memories to prevent DoS
        let k_memories = k_memories.min(100);
        
        // Generate embedding for query
        let query_embedding = llm_manager
            .generate_embedding(query, None)
            .await?;
        
        // Validate embedding size
        if query_embedding.is_empty() || query_embedding.len() > 10000 {
            return Err(crate::error::LLMError::InvalidResponse("Invalid embedding size".to_string()));
        }

        // Retrieve relevant memories
        let memories = self
            .brain
            .retrieve_memories_semantic(&query_embedding, k_memories, None, None)
            .await
            .map_err(|e| crate::error::LLMError::BrainIntegration(e.to_string()))?;

        // Build context from memories
        let context = self.build_memory_context(&memories);

        // Generate response with context
        let prompt = format!(
            "Based on the following context:\n\n{}\n\nAnswer this question: {}",
            context, query
        );

        let response = llm_manager
            .chat(vec![Message {
                role: MessageRole::User,
                content: prompt,
            }], None)
            .await?;

        Ok(response)
    }

    /// Summarize multiple memories
    pub async fn summarize_memories(
        &self,
        llm_manager: &LLMManager,
        memory_ids: &[String],
    ) -> Result<String> {
        // Input validation
        if memory_ids.is_empty() {
            return Err(crate::error::LLMError::InvalidResponse("Memory IDs cannot be empty".to_string()));
        }
        
        if memory_ids.len() > 100 {
            return Err(crate::error::LLMError::InvalidResponse("Too many memory IDs (max 100)".to_string()));
        }
        
        let mut memory_contents = Vec::new();

        for id in memory_ids.iter().take(100) {
            // Validate ID format
            if id.is_empty() || id.len() > 200 {
                continue;
            }
            
            if let Some(memory) = self.brain.get_memory(id) {
                let content_str = serde_json::to_string(&memory.content).unwrap_or_default();
                // Limit individual memory content size
                let truncated = if content_str.len() > 10000 {
                    format!("{}...", &content_str[..10000])
                } else {
                    content_str
                };
                memory_contents.push(format!(
                    "Memory {}: {}",
                    id,
                    truncated
                ));
            }
        }
        
        if memory_contents.is_empty() {
            return Err(crate::error::LLMError::InvalidResponse("No valid memories found".to_string()));
        }

        let prompt = format!(
            "Summarize the following memories into a coherent summary:\n\n{}",
            memory_contents.join("\n\n")
        );

        let response = llm_manager
            .chat(vec![Message {
                role: MessageRole::User,
                content: prompt,
            }], None)
            .await?;

        Ok(response)
    }

    /// Enhance memory with additional context
    pub async fn enhance_memory_with_context(
        &self,
        llm_manager: &LLMManager,
        memory_id: &str,
        additional_context: &str,
    ) -> Result<String> {
        // Input validation
        if memory_id.is_empty() || memory_id.len() > 200 {
            return Err(crate::error::LLMError::InvalidResponse("Invalid memory ID".to_string()));
        }
        
        if additional_context.len() > 100000 {
            return Err(crate::error::LLMError::InvalidResponse("Additional context too large".to_string()));
        }
        
        let memory = self.brain.get_memory(memory_id).ok_or_else(|| {
            crate::error::LLMError::BrainIntegration(format!(
                "Memory {} not found",
                memory_id
            ))
        })?;

        let memory_content = serde_json::to_string(&memory.content).unwrap_or_default();

        let prompt = format!(
            "Given this memory:\n{}\n\nAnd this additional context:\n{}\n\nEnhance and update the memory with the new context.",
            memory_content, additional_context
        );

        let response = llm_manager
            .chat(vec![Message {
                role: MessageRole::User,
                content: prompt,
            }], None)
            .await?;

        Ok(response)
    }

    fn build_memory_context(&self, memories: &[Memory]) -> String {
        memories
            .iter()
            .enumerate()
            .map(|(i, mem)| {
                format!(
                    "[Memory {}]: {}",
                    i + 1,
                    serde_json::to_string(&mem.content).unwrap_or_default()
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

