use async_trait::async_trait;
use crate::config::*;
use crate::error::{LLMError, Result};
use crate::providers::trait_impl::Provider as ProviderTrait;
use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use parking_lot::RwLock;

pub struct AnthropicProvider {
    api_key: Arc<RwLock<Option<String>>>,
    client: Client,
    base_url: String,
}

impl AnthropicProvider {
    pub fn new() -> Self {
        Self {
            api_key: Arc::new(RwLock::new(None)),
            client: Client::new(),
            base_url: "https://api.anthropic.com/v1".to_string(),
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
            .ok_or_else(|| LLMError::MissingApiKey("Anthropic".to_string()))
    }
}

#[async_trait]
impl ProviderTrait for AnthropicProvider {
    fn name(&self) -> &'static str {
        "anthropic"
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
                    "claude-3-opus-20240229".to_string()
                } else {
                    sanitized
                }
            })
            .unwrap_or_else(|| "claude-3-opus-20240229".to_string());

        // Convert messages to Anthropic format
        let mut messages = Vec::new();
        let mut system = None;

        for msg in &request.messages {
            match msg.role {
                MessageRole::System => {
                    system = Some(msg.content.clone());
                }
                MessageRole::User => {
                    messages.push(json!({
                        "role": "user",
                        "content": msg.content
                    }));
                }
                MessageRole::Assistant => {
                    messages.push(json!({
                        "role": "assistant",
                        "content": msg.content
                    }));
                }
                MessageRole::Tool => {
                    // Anthropic doesn't have a direct "tool" role, map to user
                    messages.push(json!({
                        "role": "user",
                        "content": format!("Tool output: {}", msg.content)
                    }));
                }
                MessageRole::Function => {
                    // Anthropic doesn't have a direct "function" role, map to user
                    messages.push(json!({
                        "role": "user",
                        "content": format!("Function output: {}", msg.content)
                    }));
                }
            }
        }

        // Validate and limit max_tokens
        let max_tokens = request.max_tokens
            .map(|t| t.min(4096))
            .unwrap_or(4096);
        
        let mut body = json!({
            "model": model,
            "messages": messages,
            "max_tokens": max_tokens,
        });

        if let Some(sys) = system {
            body["system"] = json!(sys);
        }

        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp.clamp(0.0, 1.0)); // Anthropic temperature range
        }

        // Handle tools if provided
        if let Some(tools) = request.tools {
            body["tools"] = json!(tools);
        }

        // Validate base_url
        if !self.base_url.starts_with("https://") {
            return Err(LLMError::InvalidResponse("Invalid base URL".to_string()));
        }
        
        let response = self
            .client
            .post(&format!("{}/messages", self.base_url))
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
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

        let content = json["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let usage = json.get("usage").and_then(|u| {
            Some(Usage {
                prompt_tokens: u["input_tokens"].as_u64()? as u32,
                completion_tokens: u["output_tokens"].as_u64()? as u32,
                total_tokens: (u["input_tokens"].as_u64()? + u["output_tokens"].as_u64()?) as u32,
            })
        });

        // Extract tool calls if present
        let tool_calls = json["content"]
            .as_array()
            .and_then(|content| {
                let mut calls = Vec::new();
                for item in content {
                    if item["type"].as_str() == Some("tool_use") {
                        calls.push(ToolCall {
                            id: item["id"].as_str()?.to_string(),
                            r#type: "function".to_string(),
                            function: FunctionCall {
                                name: item["name"].as_str()?.to_string(),
                                arguments: item["input"].to_string(),
                            },
                        });
                    }
                }
                if calls.is_empty() {
                    None
                } else {
                    Some(calls)
                }
            });

        Ok(ChatResponse {
            content,
            model: json["model"].as_str().unwrap_or(&model).to_string(),
            usage,
            finish_reason: json["stop_reason"].as_str().map(|s| s.to_string()),
            function_calls: None,
            tool_calls,
        })
    }

    async fn embeddings(&self, _request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        Err(LLMError::Provider(
            "Anthropic does not provide embeddings API".to_string(),
        ))
    }

    fn supports_function_calling(&self) -> bool {
        false
    }

    fn supports_tool_use(&self) -> bool {
        true
    }

    fn available_models(&self) -> Vec<String> {
        vec![
            "claude-3-opus-20240229".to_string(),
            "claude-3-sonnet-20240229".to_string(),
            "claude-3-haiku-20240307".to_string(),
        ]
    }
}

