use async_trait::async_trait;
use crate::config::*;
use crate::error::{LLMError, Result};
use crate::providers::trait_impl::Provider as ProviderTrait;
use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use parking_lot::RwLock;

pub struct OpenAIProvider {
    api_key: Arc<RwLock<Option<String>>>,
    client: Client,
    base_url: String,
}

impl OpenAIProvider {
    pub fn new() -> Self {
        Self {
            api_key: Arc::new(RwLock::new(None)),
            client: Client::new(),
            base_url: "https://api.openai.com/v1".to_string(),
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
            .ok_or_else(|| LLMError::MissingApiKey("OpenAI".to_string()))
    }
}

#[async_trait]
impl ProviderTrait for OpenAIProvider {
    fn name(&self) -> &'static str {
        "openai"
    }

    fn has_api_key(&self) -> bool {
        self.api_key.read().is_some()
    }

    fn set_api_key(&mut self, key: String) {
        *self.api_key.write() = Some(key);
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let api_key = self.get_api_key()?;
        
        // Validate and sanitize model name to prevent injection
        let model = request.model
            .as_ref()
            .map(|m| {
                // Sanitize model name - only allow alphanumeric, dash, underscore, dot
                let sanitized: String = m.chars()
                    .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
                    .take(100) // Limit length
                    .collect();
                if sanitized.is_empty() {
                    "gpt-4".to_string()
                } else {
                    sanitized
                }
            })
            .unwrap_or_else(|| "gpt-4".to_string());

        // Validate and limit max_tokens to prevent excessive API usage
        let max_tokens = request.max_tokens
            .map(|t| t.min(4096)) // Limit to 4096 tokens
            .unwrap_or(2000);
        
        let mut body = json!({
            "model": model,
            "messages": request.messages.iter().map(|m| {
                json!({
                    "role": match m.role {
                        MessageRole::System => "system",
                        MessageRole::User => "user",
                        MessageRole::Assistant => "assistant",
                        MessageRole::Tool => "tool",
                        MessageRole::Function => "function",
                    },
                    "content": m.content
                })
            }).collect::<Vec<_>>(),
            "temperature": request.temperature.unwrap_or(0.7).clamp(0.0, 2.0), // Clamp temperature
            "max_tokens": max_tokens,
        });

        if let Some(functions) = request.functions {
            body["functions"] = json!(functions);
        }

        // Validate base_url to prevent SSRF
        if !self.base_url.starts_with("https://") {
            return Err(LLMError::InvalidResponse("Invalid base URL".to_string()));
        }
        
        // Sanitize API key in logs (never log full key)
        let api_key_prefix = if api_key.len() > 8 {
            &api_key[..8]
        } else {
            "***"
        };
        tracing::debug!("Making request to OpenAI with key {}...", api_key_prefix);
        
        // Add timeout and size limits
        let url = format!("{}/chat/completions", self.base_url);
        let response = self
            .client
            .post(&url)
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
            // Limit error message size to prevent DoS
            let error_msg = if text.len() > 500 {
                format!("HTTP {}: {}", status, &text[..500])
            } else {
                format!("HTTP {}: {}", status, text)
            };
            return Err(LLMError::InvalidResponse(error_msg));
        }

        let json: serde_json::Value = response.json().await?;
        
        // Validate response structure
        let choices = json.get("choices").and_then(|c| c.as_array()).ok_or_else(|| {
            LLMError::InvalidResponse("Invalid response format: no choices array".to_string())
        })?;
        
        if choices.is_empty() {
            return Err(LLMError::InvalidResponse("No choices in response".to_string()));
        }
        
        let choice = choices[0].as_object().ok_or_else(|| {
            LLMError::InvalidResponse("Invalid choice format".to_string())
        })?;

        let content = choice["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let usage = json.get("usage").and_then(|u| {
            Some(Usage {
                prompt_tokens: u["prompt_tokens"].as_u64()? as u32,
                completion_tokens: u["completion_tokens"].as_u64()? as u32,
                total_tokens: u["total_tokens"].as_u64()? as u32,
            })
        });

        let function_calls = choice["message"]
            .get("function_call")
            .and_then(|fc| {
                Some(vec![FunctionCall {
                    name: fc["name"].as_str()?.to_string(),
                    arguments: fc["arguments"].as_str()?.to_string(),
                }])
            });

        Ok(ChatResponse {
            content,
            model: json["model"].as_str().unwrap_or(&model).to_string(),
            usage,
            finish_reason: choice["finish_reason"].as_str().map(|s| s.to_string()),
            function_calls,
            tool_calls: None,
        })
    }

    async fn embeddings(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        let api_key = self.get_api_key()?;
        let model = request.model.unwrap_or_else(|| "text-embedding-ada-002".to_string());

        // Validate input size
        if request.input.is_empty() {
            return Err(LLMError::InvalidResponse("Input cannot be empty".to_string()));
        }
        
        // Limit batch size to prevent excessive API usage
        let input_limited: Vec<String> = request.input.iter().take(100).cloned().collect();
        
        let body = json!({
            "model": model,
            "input": input_limited,
        });

        let response = self
            .client
            .post(&format!("{}/embeddings", self.base_url))
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

        let data_array = json["data"]
            .as_array()
            .ok_or_else(|| LLMError::InvalidResponse("No data in response".to_string()))?;
        
        if data_array.is_empty() {
            return Err(LLMError::InvalidResponse("Empty embeddings response".to_string()));
        }
        
        // Limit number of embeddings to prevent memory exhaustion
        let embeddings: Vec<Vec<f32>> = data_array
            .iter()
            .take(100) // Limit batch size
            .map(|item| {
                item["embedding"]
                    .as_array()
                    .ok_or_else(|| LLMError::InvalidResponse("Invalid embedding format".to_string()))
                    .and_then(|emb| {
                        if emb.is_empty() {
                            Err(LLMError::InvalidResponse("Empty embedding".to_string()))
                        } else if emb.len() > 10000 {
                            Err(LLMError::InvalidResponse("Embedding dimension too large".to_string()))
                        } else {
                            // Validate all values are finite numbers
                            let vec: std::result::Result<Vec<f32>, LLMError> = emb.iter()
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
                        }
                    })
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
        
        if embeddings.is_empty() {
            return Err(LLMError::InvalidResponse("No valid embeddings in response".to_string()));
        }

        let usage = json.get("usage").and_then(|u| {
            Some(Usage {
                prompt_tokens: u["prompt_tokens"].as_u64()? as u32,
                completion_tokens: 0,
                total_tokens: u["total_tokens"].as_u64()? as u32,
            })
        });

        Ok(EmbeddingResponse {
            embeddings,
            model: json["model"].as_str().unwrap_or(&model).to_string(),
            usage,
        })
    }

    fn supports_function_calling(&self) -> bool {
        true
    }

    fn supports_tool_use(&self) -> bool {
        true
    }

    fn available_models(&self) -> Vec<String> {
        vec![
            "gpt-4".to_string(),
            "gpt-4-turbo-preview".to_string(),
            "gpt-3.5-turbo".to_string(),
            "gpt-3.5-turbo-16k".to_string(),
        ]
    }
}

