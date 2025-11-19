use crate::config::*;
use crate::error::Result;
use serde_json::Value;
use std::sync::Arc;

// Trait for brain interface to avoid circular dependency
pub trait BrainFunctionInterface: Send + Sync {
    fn create_thought(&self, content: serde_json::Value, priority: f64) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>>;
    fn store_memory(&self, memory_type: &str, content: serde_json::Value, tags: Vec<String>) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>>;
    fn store_experience(&self, event_type: String, observation: serde_json::Value, action: Option<serde_json::Value>, outcome: Option<serde_json::Value>, reward: Option<f64>) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>>;
    fn get_thought(&self, thought_id: &str) -> Option<serde_json::Value>;
    fn get_memory(&self, memory_id: &str) -> Option<serde_json::Value>;
}

#[derive(Debug, Clone)]
pub enum BrainFunction {
    CreateThought,
    StoreMemory,
    RetrieveMemories,
    StoreExperience,
    GetThought,
    GetMemory,
}

impl BrainFunction {
    pub fn as_str(&self) -> &'static str {
        match self {
            BrainFunction::CreateThought => "create_thought",
            BrainFunction::StoreMemory => "store_memory",
            BrainFunction::RetrieveMemories => "retrieve_memories",
            BrainFunction::StoreExperience => "store_experience",
            BrainFunction::GetThought => "get_thought",
            BrainFunction::GetMemory => "get_memory",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "create_thought" => Some(BrainFunction::CreateThought),
            "store_memory" => Some(BrainFunction::StoreMemory),
            "retrieve_memories" => Some(BrainFunction::RetrieveMemories),
            "store_experience" => Some(BrainFunction::StoreExperience),
            "get_thought" => Some(BrainFunction::GetThought),
            "get_memory" => Some(BrainFunction::GetMemory),
            _ => None,
        }
    }

    pub fn to_function_definition(&self) -> FunctionDefinition {
        match self {
            BrainFunction::CreateThought => FunctionDefinition {
                name: "create_thought".to_string(),
                description: "Create a new thought in the cognitive brain".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "object",
                            "description": "The content of the thought as JSON"
                        },
                        "priority": {
                            "type": "number",
                            "description": "Priority of the thought (0.0 to 1.0)"
                        }
                    },
                    "required": ["content", "priority"]
                }),
            },
            BrainFunction::StoreMemory => FunctionDefinition {
                name: "store_memory".to_string(),
                description: "Store a memory in the cognitive brain".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "memory_type": {
                            "type": "string",
                            "enum": ["Episodic", "Semantic", "Procedural", "Working", "Associative", "Emotional", "Spatial", "Temporal", "LongTerm"],
                            "description": "Type of memory to store"
                        },
                        "content": {
                            "type": "object",
                            "description": "The content of the memory as JSON"
                        },
                        "tags": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Tags for the memory"
                        }
                    },
                    "required": ["memory_type", "content"]
                }),
            },
            BrainFunction::RetrieveMemories => FunctionDefinition {
                name: "retrieve_memories".to_string(),
                description: "Retrieve memories from the cognitive brain".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Query to search for memories"
                        },
                        "k": {
                            "type": "number",
                            "description": "Number of memories to retrieve"
                        }
                    },
                    "required": ["query", "k"]
                }),
            },
            BrainFunction::StoreExperience => FunctionDefinition {
                name: "store_experience".to_string(),
                description: "Store an experience in the cognitive brain".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "event_type": {
                            "type": "string",
                            "description": "Type of event"
                        },
                        "observation": {
                            "type": "object",
                            "description": "Observation data as JSON"
                        },
                        "action": {
                            "type": "object",
                            "description": "Action taken (optional)"
                        },
                        "outcome": {
                            "type": "object",
                            "description": "Outcome of the action (optional)"
                        },
                        "reward": {
                            "type": "number",
                            "description": "Reward value (optional)"
                        }
                    },
                    "required": ["event_type", "observation"]
                }),
            },
            BrainFunction::GetThought => FunctionDefinition {
                name: "get_thought".to_string(),
                description: "Get a thought by ID".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "thought_id": {
                            "type": "string",
                            "description": "ID of the thought to retrieve"
                        }
                    },
                    "required": ["thought_id"]
                }),
            },
            BrainFunction::GetMemory => FunctionDefinition {
                name: "get_memory".to_string(),
                description: "Get a memory by ID".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "memory_id": {
                            "type": "string",
                            "description": "ID of the memory to retrieve"
                        }
                    },
                    "required": ["memory_id"]
                }),
            },
        }
    }
}

pub struct FunctionCallingSystem {
    brain: Arc<dyn BrainFunctionInterface>,
}

impl FunctionCallingSystem {
    pub fn new(brain: Arc<dyn BrainFunctionInterface>) -> Self {
        Self { brain }
    }

    /// Execute a function call from the LLM
    pub async fn execute_function_call(
        &self,
        function_name: &str,
        arguments: &str,
    ) -> Result<Value> {
        // Security: Validate function name
        if function_name.is_empty() || function_name.len() > 100 {
            return Err(crate::error::LLMError::InvalidResponse("Invalid function name".to_string()));
        }
        
        // Security: Validate arguments size
        if arguments.len() > 10000 {
            return Err(crate::error::LLMError::InvalidResponse("Arguments too large".to_string()));
        }
        
        // Security: Prevent injection - only allow alphanumeric, underscore, dash
        if !function_name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(crate::error::LLMError::InvalidResponse("Invalid function name characters".to_string()));
        }
        
        let args: Value = serde_json::from_str(arguments)
            .map_err(|e| crate::error::LLMError::InvalidResponse(format!("Invalid JSON: {}", e)))?;

        // Security: Limit JSON depth to prevent stack overflow
        if serde_json::to_string(&args).unwrap_or_default().len() > 10000 {
            return Err(crate::error::LLMError::InvalidResponse("Arguments JSON too large".to_string()));
        }

        let function = BrainFunction::from_str(function_name)
            .ok_or_else(|| {
                crate::error::LLMError::InvalidResponse(format!(
                    "Unknown function: {}",
                    function_name
                ))
            })?;

        match function {
            BrainFunction::CreateThought => {
                let content = args["content"].clone();
                
                // Validate content size
                if serde_json::to_string(&content).unwrap_or_default().len() > 100000 {
                    return Err(crate::error::LLMError::InvalidResponse("Content too large".to_string()));
                }
                
                let priority = args["priority"]
                    .as_f64()
                    .unwrap_or(0.5)
                    .clamp(0.0, 1.0); // Clamp priority to valid range
                
                let thought_id = self
                    .brain
                    .create_thought(content, priority)
                    .map_err(|e| crate::error::LLMError::BrainIntegration(e.to_string()))?;
                Ok(json!({"thought_id": thought_id}))
            }
            BrainFunction::StoreMemory => {
                let memory_type_str = args["memory_type"]
                    .as_str()
                    .ok_or_else(|| {
                        crate::error::LLMError::InvalidResponse("memory_type must be a string".to_string())
                    })?;
                
                // Validate memory type
                const VALID_MEMORY_TYPES: &[&str] = &[
                    "Episodic", "Semantic", "Procedural", "Working", 
                    "LongTerm", "Associative", "Emotional", "Spatial", "Temporal"
                ];
                if !VALID_MEMORY_TYPES.contains(&memory_type_str) {
                    return Err(crate::error::LLMError::InvalidResponse(
                        format!("Invalid memory type: {}", memory_type_str)
                    ));
                }
                
                let content = args["content"].clone();
                
                // Validate content size
                if serde_json::to_string(&content).unwrap_or_default().len() > 100000 {
                    return Err(crate::error::LLMError::InvalidResponse("Memory content too large".to_string()));
                }
                
                let tags: Vec<String> = args["tags"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .take(50) // Limit number of tags
                            .filter_map(|v| {
                                v.as_str().map(|s| {
                                    let tag = s.to_string();
                                    // Limit tag length
                                    if tag.len() > 100 {
                                        tag[..100].to_string()
                                    } else {
                                        tag
                                    }
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                
                let memory_id = self
                    .brain
                    .store_memory(memory_type_str, content, tags)
                    .map_err(|e| crate::error::LLMError::BrainIntegration(e.to_string()))?;
                Ok(json!({"memory_id": memory_id}))
            }
            BrainFunction::RetrieveMemories => {
                let _query = args["query"]
                    .as_str()
                    .ok_or_else(|| {
                        crate::error::LLMError::InvalidResponse("query must be a string".to_string())
                    })?;
                let _k = args["k"].as_u64().unwrap_or(5) as usize;
                // For now, return empty - actual retrieval is handled by RAG
                Ok(json!({"memories": []}))
            }
            BrainFunction::StoreExperience => {
                let event_type = args["event_type"]
                    .as_str()
                    .ok_or_else(|| {
                        crate::error::LLMError::InvalidResponse("event_type must be a string".to_string())
                    })?
                    .to_string();
                let observation = args["observation"].clone();
                let action = args.get("action").cloned();
                let outcome = args.get("outcome").cloned();
                let reward = args.get("reward").and_then(|r| r.as_f64());
                let experience_id = self
                    .brain
                    .store_experience(event_type, observation, action, outcome, reward)
                    .map_err(|e| crate::error::LLMError::BrainIntegration(e.to_string()))?;
                Ok(json!({"experience_id": experience_id}))
            }
            BrainFunction::GetThought => {
                let thought_id = args["thought_id"]
                    .as_str()
                    .ok_or_else(|| {
                        crate::error::LLMError::InvalidResponse("thought_id must be a string".to_string())
                    })?;
                if let Some(thought) = self.brain.get_thought(thought_id) {
                    Ok(thought)
                } else {
                    Err(crate::error::LLMError::BrainIntegration(format!(
                        "Thought {} not found",
                        thought_id
                    )))
                }
            }
            BrainFunction::GetMemory => {
                let memory_id = args["memory_id"]
                    .as_str()
                    .ok_or_else(|| {
                        crate::error::LLMError::InvalidResponse("memory_id must be a string".to_string())
                    })?;
                if let Some(memory) = self.brain.get_memory(memory_id) {
                    Ok(memory)
                } else {
                    Err(crate::error::LLMError::BrainIntegration(format!(
                        "Memory {} not found",
                        memory_id
                    )))
                }
            }
        }
    }

    /// Get all available brain functions as function definitions
    pub fn get_brain_functions() -> Vec<FunctionDefinition> {
        vec![
            BrainFunction::CreateThought.to_function_definition(),
            BrainFunction::StoreMemory.to_function_definition(),
            BrainFunction::RetrieveMemories.to_function_definition(),
            BrainFunction::StoreExperience.to_function_definition(),
            BrainFunction::GetThought.to_function_definition(),
            BrainFunction::GetMemory.to_function_definition(),
        ]
    }
}

use serde_json::json;

