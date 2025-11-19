use async_trait::async_trait;
use crate::config::*;
use crate::error::{LLMError, Result};
use crate::providers::trait_impl::Provider as ProviderTrait;
use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use parking_lot::RwLock;

pub struct GoogleProvider {
    api_key: Arc<RwLock<Option<String>>>,
    client: Client,
    base_url: String,
}

impl GoogleProvider {
    pub fn new() -> Self {
        Self {
            api_key: Arc::new(RwLock::new(None)),
            client: Client::new(),
            base_url: "https://generativelanguage.googleapis.com/v1".to_string(),
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
            .ok_or_else(|| LLMError::MissingApiKey("Google".to_string()))
    }
}

#[async_trait]
impl ProviderTrait for GoogleProvider {
    fn name(&self) -> &'static str {
        "google"
    }

    fn has_api_key(&self) -> bool {
        self.api_key.read().is_some()
    }

    fn set_api_key(&mut self, key: String) {
        *self.api_key.write() = Some(key);
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let api_key = self.get_api_key()?;
        let model = request.model.unwrap_or_else(|| "gemini-pro".to_string());

        // Convert messages to Google format
        let contents: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| {
                json!({
                    "role": match m.role {
                        MessageRole::System => "user", // Google doesn't have system role
                        MessageRole::User => "user",
                        MessageRole::Assistant => "model",
                        MessageRole::Tool => "tool",
                        MessageRole::Function => "function",
                    },
                    "parts": [{"text": m.content}]
                })
            })
            .collect();

        let mut body = json!({
            "contents": contents,
        });

        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp.clamp(0.0, 2.0)); // Google temperature range
        }

        if let Some(max_tokens) = request.max_tokens {
            body["maxOutputTokens"] = json!(max_tokens.min(8192)); // Limit max tokens
        }

        // Validate base_url to prevent SSRF
        if !self.base_url.starts_with("https://") {
            return Err(LLMError::InvalidResponse("Invalid base URL".to_string()));
        }
        
        // URL encode model name to prevent injection
        let model_encoded = urlencoding::encode(&model);
        let url = format!("{}/models/{}:generateContent?key={}", self.base_url, model_encoded, api_key);

        let response = self
            .client
            .post(&url)
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

        let content = json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let usage = json.get("usageMetadata").and_then(|u| {
            Some(Usage {
                prompt_tokens: u["promptTokenCount"].as_u64()? as u32,
                completion_tokens: u["candidatesTokenCount"].as_u64()? as u32,
                total_tokens: u["totalTokenCount"].as_u64()? as u32,
            })
        });

        Ok(ChatResponse {
            content,
            model: model.clone(),
            usage,
            finish_reason: json["candidates"][0]["finishReason"].as_str().map(|s| s.to_string()),
            function_calls: None,
            tool_calls: None,
        })
    }

    async fn embeddings(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        let api_key = self.get_api_key()?;
        let model = request.model.unwrap_or_else(|| "embedding-001".to_string());

        // Validate and limit input
        if request.input.is_empty() {
            return Err(LLMError::InvalidResponse("Input cannot be empty".to_string()));
        }
        
        // Limit batch size
        let input_limited: Vec<&String> = request.input.iter().take(100).collect();
        
        let body = json!({
            "model": format!("models/{}", model),
            "content": {
                "parts": input_limited.iter().map(|text| {
                    json!({"text": text})
                }).collect::<Vec<_>>()
            }
        });

        // Validate base_url
        if !self.base_url.starts_with("https://") {
            return Err(LLMError::InvalidResponse("Invalid base URL".to_string()));
        }
        
        // URL encode model name to prevent injection
        let model_encoded = urlencoding::encode(&model);
        let url = format!("{}/models/{}:embedContent?key={}", self.base_url, model_encoded, api_key);

        let response = self
            .client
            .post(&url)
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

        // Google embeddings API returns a single embedding, not an array
        // This needs to be fixed - the API structure is different
        let embedding_array = json["embedding"]["values"]
            .as_array()
            .ok_or_else(|| LLMError::InvalidResponse("No embedding values".to_string()))?;
        
        if embedding_array.is_empty() {
            return Err(LLMError::InvalidResponse("Empty embedding".to_string()));
        }
        
        if embedding_array.len() > 10000 {
            return Err(LLMError::InvalidResponse("Embedding dimension too large".to_string()));
        }
        
        let embedding: std::result::Result<Vec<f32>, LLMError> = embedding_array
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
        
        let embedding = embedding?;
        let embeddings = vec![embedding]; // Wrap in Vec for consistency

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
        false
    }

    fn available_models(&self) -> Vec<String> {
        vec![
            "gemini-pro".to_string(),
            "gemini-pro-vision".to_string(),
        ]
    }
}

