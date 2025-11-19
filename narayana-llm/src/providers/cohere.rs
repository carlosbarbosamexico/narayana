use async_trait::async_trait;
use crate::config::*;
use crate::error::{LLMError, Result};
use crate::providers::trait_impl::Provider as ProviderTrait;
use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use parking_lot::RwLock;

pub struct CohereProvider {
    api_key: Arc<RwLock<Option<String>>>,
    client: Client,
    base_url: String,
}

impl CohereProvider {
    pub fn new() -> Self {
        Self {
            api_key: Arc::new(RwLock::new(None)),
            client: Client::new(),
            base_url: "https://api.cohere.ai/v1".to_string(),
        }
    }

    pub fn with_api_key(api_key: String) -> Self {
        let mut provider = Self::new();
        provider.set_api_key(api_key);
        provider
    }

    fn get_api_key(&self) -> Result<String> {
        self.api_key
            .read()
            .as_ref()
            .cloned()
            .ok_or_else(|| LLMError::MissingApiKey("Cohere".to_string()))
    }
}

#[async_trait]
impl ProviderTrait for CohereProvider {
    fn name(&self) -> &'static str {
        "cohere"
    }

    fn has_api_key(&self) -> bool {
        self.api_key.read().is_some()
    }

    fn set_api_key(&mut self, key: String) {
        *self.api_key.write() = Some(key);
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let api_key = self.get_api_key()?;
        
        // Validate and sanitize model name
        let model = request.model
            .as_ref()
            .map(|m| {
                let sanitized: String = m.chars()
                    .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
                    .take(100)
                    .collect();
                if sanitized.is_empty() {
                    "command-r-plus".to_string()
                } else {
                    sanitized
                }
            })
            .unwrap_or_else(|| "command-r-plus".to_string());

        // Cohere chat format - combine messages into a single chat history
        let mut chat_history = Vec::new();
        let mut message = String::new();

        for msg in &request.messages {
            match msg.role {
                MessageRole::System => {
                    // Cohere doesn't have system messages, prepend to first user message
                    if message.is_empty() {
                        message = format!("System: {}\n\n", msg.content);
                    }
                }
                MessageRole::User => {
                    if !message.is_empty() {
                        message.push_str(&msg.content);
                    } else {
                        message = msg.content.clone();
                    }
                }
                MessageRole::Assistant => {
                    if !message.is_empty() {
                        chat_history.push(json!({
                            "role": "USER",
                            "message": message
                        }));
                        message = String::new();
                    }
                    chat_history.push(json!({
                        "role": "CHATBOT",
                        "message": msg.content
                    }));
                }
                MessageRole::Tool | MessageRole::Function => {
                    // Cohere doesn't have direct tool/function roles, map to user
                    if !message.is_empty() {
                        message.push_str(&format!("\nTool/Function output: {}", msg.content));
                    } else {
                        message = format!("Tool/Function output: {}", msg.content);
                    }
                }
            }
        }

        // Add final user message if exists
        if !message.is_empty() {
            chat_history.push(json!({
                "role": "USER",
                "message": message
            }));
        }

        // Limit chat history size to prevent excessive context
        let chat_history_vec: Vec<serde_json::Value> = if chat_history.len() > 20 {
            chat_history[chat_history.len() - 20..].to_vec()
        } else {
            chat_history.clone()
        };
        
        let current_message = chat_history_vec.last()
            .and_then(|h| h["message"].as_str())
            .unwrap_or("")
            .to_string();
        
        let history_for_body: Vec<&serde_json::Value> = if chat_history_vec.len() > 1 {
            chat_history_vec[..chat_history_vec.len() - 1].iter().collect()
        } else {
            vec![]
        };
        
        let mut body = json!({
            "model": model,
            "message": current_message,
            "chat_history": history_for_body,
            "temperature": request.temperature.unwrap_or(0.7).clamp(0.0, 1.0),
        });

        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = json!(max_tokens.min(4096)); // Limit max tokens
        }

        // Validate base_url
        if !self.base_url.starts_with("https://") {
            return Err(LLMError::InvalidResponse("Invalid base URL".to_string()));
        }
        
        let response = self
            .client
            .post(&format!("{}/chat", self.base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(120))
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        
        // Handle rate limiting
        if status == 429 {
            return Err(LLMError::RateLimit);
        }
        
        // Handle authentication errors
        if status == 401 || status == 403 {
            return Err(LLMError::AuthenticationFailed);
        }
        
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            let error_msg = if text.len() > 500 {
                format!("HTTP {}: {}", status, &text[..500])
            } else {
                format!("HTTP {}: {}", status, text)
            };
            return Err(LLMError::InvalidResponse(error_msg));
        }

        let json: serde_json::Value = response.json().await?;

        let content = json["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let usage = json.get("meta").and_then(|m| {
            m.get("tokens").and_then(|t| {
                Some(Usage {
                    prompt_tokens: t["input_tokens"].as_u64()? as u32,
                    completion_tokens: t["output_tokens"].as_u64()? as u32,
                    total_tokens: (t["input_tokens"].as_u64()? + t["output_tokens"].as_u64()?) as u32,
                })
            })
        });

        Ok(ChatResponse {
            content,
            model: json["generation_id"].as_str().unwrap_or(&model).to_string(),
            usage,
            finish_reason: json["finish_reason"].as_str().map(|s| s.to_string()),
            function_calls: None,
            tool_calls: None,
        })
    }

    async fn embeddings(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        let api_key = self.get_api_key()?;
        let model = request.model.unwrap_or_else(|| "embed-english-v3.0".to_string());

        // Validate and limit input
        if request.input.is_empty() {
            return Err(LLMError::InvalidResponse("Input cannot be empty".to_string()));
        }
        
        // Limit batch size
        let input_limited: Vec<String> = request.input.iter().take(100).cloned().collect();
        
        let body = json!({
            "model": model,
            "texts": input_limited,
            "input_type": "search_document",
        });

        let response = self
            .client
            .post(&format!("{}/embed", self.base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(LLMError::InvalidResponse(format!(
                "HTTP {}: {}",
                status, text
            )));
        }

        let json: serde_json::Value = response.json().await?;

        let embeddings_array = json["embeddings"]
            .as_array()
            .ok_or_else(|| LLMError::InvalidResponse("No embeddings in response".to_string()))?;
        
        if embeddings_array.is_empty() {
            return Err(LLMError::InvalidResponse("Empty embeddings response".to_string()));
        }
        
        // Limit number of embeddings
        let embeddings: std::result::Result<Vec<Vec<f32>>, LLMError> = embeddings_array
            .iter()
            .take(100)
            .map(|emb| {
                let emb_array = emb.as_array()
                    .ok_or_else(|| LLMError::InvalidResponse("Invalid embedding format".to_string()))?;
                
                if emb_array.is_empty() {
                    return Err(LLMError::InvalidResponse("Empty embedding vector".to_string()));
                }
                
                if emb_array.len() > 10000 {
                    return Err(LLMError::InvalidResponse("Embedding dimension too large".to_string()));
                }
                
                let vec: std::result::Result<Vec<f32>, LLMError> = emb_array
                    .iter()
                    .map(|v| {
                        let val = v.as_f64()
                            .ok_or_else(|| LLMError::InvalidResponse("Non-numeric embedding value".to_string()))?;
                        if !val.is_finite() {
                            return Err(LLMError::InvalidResponse("Non-finite embedding value".to_string()));
                        }
                        Ok(val as f32)
                    })
                    .collect();
                vec
            })
            .collect();
        
        let embeddings = embeddings?;

        Ok(EmbeddingResponse {
            embeddings,
            model: model.clone(),
            usage: None,
        })
    }

    fn supports_function_calling(&self) -> bool {
        false
    }

    fn supports_tool_use(&self) -> bool {
        true // Command R+ supports tool use
    }

    fn available_models(&self) -> Vec<String> {
        vec![
            "command-r-plus".to_string(),
            "command-r".to_string(),
            "command".to_string(),
        ]
    }
}

