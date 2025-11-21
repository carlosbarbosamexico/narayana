use async_trait::async_trait;
use crate::config::*;
use crate::error::Result;

#[async_trait]
pub trait Provider: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &'static str;

    /// Check if API key is set
    fn has_api_key(&self) -> bool;

    /// Set API key
    fn set_api_key(&mut self, key: String);

    /// Chat completion
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse>;

    /// Generate embeddings
    async fn embeddings(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse>;

    /// Check if provider supports function calling
    fn supports_function_calling(&self) -> bool;

    /// Check if provider supports tool use
    fn supports_tool_use(&self) -> bool;

    /// Get available models
    fn available_models(&self) -> Vec<String>;
}



