use crate::config::*;
use crate::error::{LLMError, Result};
use crate::providers::trait_impl::Provider as ProviderTrait;
use crate::providers::{openai::OpenAIProvider, anthropic::AnthropicProvider, google::GoogleProvider, cohere::CohereProvider};
use crate::rag::{RAGSystem, BrainInterface};
use crate::function_calling::{FunctionCallingSystem, BrainFunction, BrainFunctionInterface};
use crate::reasoning::ReasoningSystem;
use crate::planning::PlanningSystem;
use crate::cache::ResponseCache;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use std::env;

pub struct LLMManager {
    providers: Arc<RwLock<HashMap<Provider, ProviderBox>>>,
    default_provider: Arc<RwLock<Option<Provider>>>,
    config: Arc<RwLock<LLMConfig>>,
    rag: Option<RAGSystem>,
    function_calling: Option<FunctionCallingSystem>,
    reasoning: ReasoningSystem,
    planning: PlanningSystem,
    cache: Arc<ResponseCache>,
}

enum ProviderBox {
    OpenAI(OpenAIProvider),
    Anthropic(AnthropicProvider),
    Google(GoogleProvider),
    Cohere(CohereProvider),
}

#[async_trait::async_trait]
impl ProviderTrait for ProviderBox {
    fn name(&self) -> &'static str {
        match self {
            ProviderBox::OpenAI(p) => p.name(),
            ProviderBox::Anthropic(p) => p.name(),
            ProviderBox::Google(p) => p.name(),
            ProviderBox::Cohere(p) => p.name(),
        }
    }

    fn has_api_key(&self) -> bool {
        match self {
            ProviderBox::OpenAI(p) => p.has_api_key(),
            ProviderBox::Anthropic(p) => p.has_api_key(),
            ProviderBox::Google(p) => p.has_api_key(),
            ProviderBox::Cohere(p) => p.has_api_key(),
        }
    }

    fn set_api_key(&mut self, key: String) {
        match self {
            ProviderBox::OpenAI(p) => p.set_api_key(key),
            ProviderBox::Anthropic(p) => p.set_api_key(key),
            ProviderBox::Google(p) => p.set_api_key(key),
            ProviderBox::Cohere(p) => p.set_api_key(key),
        }
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        match self {
            ProviderBox::OpenAI(p) => p.chat(request).await,
            ProviderBox::Anthropic(p) => p.chat(request).await,
            ProviderBox::Google(p) => p.chat(request).await,
            ProviderBox::Cohere(p) => p.chat(request).await,
        }
    }

    async fn embeddings(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        match self {
            ProviderBox::OpenAI(p) => p.embeddings(request).await,
            ProviderBox::Anthropic(p) => p.embeddings(request).await,
            ProviderBox::Google(p) => p.embeddings(request).await,
            ProviderBox::Cohere(p) => p.embeddings(request).await,
        }
    }

    fn supports_function_calling(&self) -> bool {
        match self {
            ProviderBox::OpenAI(p) => p.supports_function_calling(),
            ProviderBox::Anthropic(p) => p.supports_function_calling(),
            ProviderBox::Google(p) => p.supports_function_calling(),
            ProviderBox::Cohere(p) => p.supports_function_calling(),
        }
    }

    fn supports_tool_use(&self) -> bool {
        match self {
            ProviderBox::OpenAI(p) => p.supports_tool_use(),
            ProviderBox::Anthropic(p) => p.supports_tool_use(),
            ProviderBox::Google(p) => p.supports_tool_use(),
            ProviderBox::Cohere(p) => p.supports_tool_use(),
        }
    }

    fn available_models(&self) -> Vec<String> {
        match self {
            ProviderBox::OpenAI(p) => p.available_models(),
            ProviderBox::Anthropic(p) => p.available_models(),
            ProviderBox::Google(p) => p.available_models(),
            ProviderBox::Cohere(p) => p.available_models(),
        }
    }
}

impl LLMManager {
    pub fn new() -> Self {
        let mut manager = Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
            default_provider: Arc::new(RwLock::new(None)),
            config: Arc::new(RwLock::new(LLMConfig::default())),
            rag: None,
            function_calling: None,
            reasoning: ReasoningSystem::new(),
            planning: PlanningSystem::new(),
            cache: Arc::new(ResponseCache::new(1000)),
        };

        // Initialize providers from environment variables
        manager.initialize_from_env();

        manager
    }

    /// Initialize with brain interface for RAG and function calling
    pub fn with_brain<B: BrainInterface + BrainFunctionInterface + 'static>(brain: Arc<B>) -> Self {
        let mut manager = Self::new();
        manager.rag = Some(RAGSystem::new(brain.clone()));
        manager.function_calling = Some(FunctionCallingSystem::new(brain));
        manager
    }

    fn initialize_from_env(&mut self) {
        // Try to load API keys from environment
        for provider in [Provider::OpenAI, Provider::Anthropic, Provider::Google, Provider::Cohere] {
            if let Ok(key) = env::var(provider.env_var_name()) {
                self.set_api_key(provider, key);
            }
        }

        // Set default provider if available
        if self.providers.read().contains_key(&Provider::Cohere) {
            *self.default_provider.write() = Some(Provider::Cohere);
        } else if self.providers.read().contains_key(&Provider::OpenAI) {
            *self.default_provider.write() = Some(Provider::OpenAI);
        }
    }

    /// Set API key for a provider
    pub fn set_api_key(&self, provider: Provider, key: String) {
        // Validate API key format
        if key.is_empty() {
            tracing::warn!("Empty API key provided for {:?}", provider);
            return;
        }
        
        if key.len() > 1000 {
            tracing::warn!("API key too long for {:?}", provider);
            return;
        }
        
        // Basic format validation
        match provider {
            Provider::OpenAI | Provider::Anthropic | Provider::Cohere => {
                if !key.starts_with("sk-") && !key.starts_with("Bearer ") {
                    // Allow other formats but log warning
                    tracing::debug!("API key format may be invalid for {:?}", provider);
                }
            }
            Provider::Google => {
                // Google API keys are typically longer alphanumeric strings
                if key.len() < 20 {
                    tracing::warn!("Google API key seems too short");
                }
            }
        }
        
        let mut providers = self.providers.write();
        
        match provider {
            Provider::OpenAI => {
                let mut p = OpenAIProvider::new();
                p.set_api_key(key);
                providers.insert(provider, ProviderBox::OpenAI(p));
            }
            Provider::Anthropic => {
                let mut p = AnthropicProvider::new();
                p.set_api_key(key);
                providers.insert(provider, ProviderBox::Anthropic(p));
            }
            Provider::Google => {
                let mut p = GoogleProvider::new();
                p.set_api_key(key);
                providers.insert(provider, ProviderBox::Google(p));
            }
            Provider::Cohere => {
                let mut p = CohereProvider::new();
                p.set_api_key(key);
                providers.insert(provider, ProviderBox::Cohere(p));
            }
        }

        // Set as default if no default is set
        if self.default_provider.read().is_none() {
            *self.default_provider.write() = Some(provider);
        }
    }

    /// Get the provider to use (default or specified)
    fn get_provider(&self, provider: Option<Provider>) -> Result<Provider> {
        let provider = provider.or_else(|| *self.default_provider.read());
        
        provider.ok_or_else(|| {
            LLMError::MissingApiKey("No provider configured".to_string())
        })
    }

    /// Chat completion
    pub async fn chat(
        &self,
        messages: Vec<Message>,
        provider: Option<Provider>,
    ) -> Result<String> {
        // Input validation and security checks
        if messages.is_empty() {
            return Err(LLMError::InvalidResponse("Messages cannot be empty".to_string()));
        }
        
        if messages.len() > 100 {
            return Err(LLMError::InvalidResponse("Too many messages (max 100)".to_string()));
        }
        
        // Validate message content size (check for overflow)
        let total_size: usize = messages.iter()
            .map(|m| m.content.len())
            .try_fold(0usize, |acc, len| {
                acc.checked_add(len)
                    .ok_or_else(|| LLMError::InvalidResponse("Message size calculation overflow".to_string()))
            })
            .map_err(|e| e)?;
            
        if total_size > 1_000_000 {
            return Err(LLMError::InvalidResponse("Total message content too large (max 1MB)".to_string()));
        }
        
        // Sanitize messages - check for suspicious patterns
        for msg in &messages {
            if msg.content.len() > 100_000 {
                return Err(LLMError::InvalidResponse("Individual message too large (max 100KB)".to_string()));
            }
        }
        
        let config = self.config.read();
        
        // Create cache key more safely (avoid Debug format which could be huge)
        // Use a hash-based approach instead
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        for msg in &messages {
            msg.role.hash(&mut hasher);
            msg.content.hash(&mut hasher);
        }
        let cache_key = format!("chat:{}", hasher.finish());
        
        if config.enable_caching {
            if let Some(cached) = self.cache.get(&cache_key) {
                return Ok(cached);
            }
        }

        let provider = self.get_provider(provider)?;
        let providers = self.providers.read();
        let provider_box = providers
            .get(&provider)
            .ok_or_else(|| LLMError::MissingApiKey(format!("Provider {:?} not configured", provider)))?;
        
        let request = ChatRequest {
            messages,
            model: config.default_model.clone(),
            temperature: Some(config.temperature),
            max_tokens: config.max_tokens,
            functions: None,
            tools: None,
        };

        let response = provider_box.chat(request).await?;
        let content = response.content;

        if config.enable_caching {
            self.cache.set(&cache_key, content.clone(), config.cache_ttl_seconds);
        }

        Ok(content)
    }

    /// Generate embeddings
    pub async fn generate_embedding(
        &self,
        text: &str,
        provider: Option<Provider>,
    ) -> Result<Vec<f32>> {
        // Input validation
        if text.is_empty() {
            return Err(LLMError::InvalidResponse("Text cannot be empty".to_string()));
        }
        
        if text.len() > 8_000 {
            return Err(LLMError::InvalidResponse("Text too long for embedding (max 8000 chars)".to_string()));
        }
        
        let provider = self.get_provider(provider)?;
        let providers = self.providers.read();
        let provider_box = providers
            .get(&provider)
            .ok_or_else(|| LLMError::MissingApiKey(format!("Provider {:?} not configured", provider)))?;
        
        let request = EmbeddingRequest {
            input: vec![text.to_string()],
            model: None,
        };

        let response = provider_box.embeddings(request).await?;
        response.embeddings
            .into_iter()
            .next()
            .ok_or_else(|| LLMError::InvalidResponse("No embedding returned".to_string()))
    }

    /// Generate thought with RAG
    pub async fn generate_thought(
        &self,
        prompt: &str,
        _context: Option<&str>,
        k_memories: usize,
    ) -> Result<String> {
        self.rag
            .as_ref()
            .ok_or_else(|| LLMError::BrainIntegration("RAG system not initialized".to_string()))?
            .retrieve_and_generate(self, prompt, k_memories)
            .await
    }

    /// Chain of thought reasoning
    pub async fn chain_of_thought_reasoning(
        &self,
        problem: &str,
        steps: &[&str],
    ) -> Result<String> {
        self.reasoning
            .chain_of_thought_reasoning(self, problem, steps)
            .await
    }

    /// Tree of thoughts
    pub async fn tree_of_thoughts(
        &self,
        problem: &str,
        branches: usize,
    ) -> Result<Vec<String>> {
        self.reasoning.tree_of_thoughts(self, problem, branches).await
    }

    /// Generate hypothesis
    pub async fn generate_hypothesis(
        &self,
        observation: &str,
        context: Option<&str>,
    ) -> Result<String> {
        self.reasoning
            .generate_hypothesis(self, observation, context)
            .await
    }

    /// Generate plan
    pub async fn generate_plan(
        &self,
        goal: &str,
        constraints: &[String],
    ) -> Result<String> {
        self.planning.generate_plan(self, goal, constraints).await
    }

    /// Chat with function calling
    pub async fn chat_with_functions(
        &self,
        messages: Vec<Message>,
        functions: Vec<BrainFunction>,
        provider: Option<Provider>,
    ) -> Result<String> {
        let provider = self.get_provider(provider)?;
        let providers = self.providers.read();
        let provider_box = providers
            .get(&provider)
            .ok_or_else(|| LLMError::MissingApiKey(format!("Provider {:?} not configured", provider)))?;
        
        let function_defs: Vec<FunctionDefinition> = functions
            .iter()
            .map(|f| f.to_function_definition())
            .collect();

        // Clone messages early to avoid move issues
        let messages_clone = messages.clone();
        
        let mut request = ChatRequest {
            messages,
            model: self.config.read().default_model.clone(),
            temperature: Some(self.config.read().temperature),
            max_tokens: self.config.read().max_tokens,
            functions: Some(function_defs),
            tools: None,
        };

        // Convert functions to tools if provider supports it
        if provider_box.supports_tool_use() {
            let tools: Vec<ToolDefinition> = request
                .functions
                .as_ref()
                .unwrap()
                .iter()
                .map(|f| ToolDefinition {
                    r#type: "function".to_string(),
                    function: f.clone(),
                })
                .collect();
            request.tools = Some(tools);
            request.functions = None;
        }

        let response = provider_box.chat(request).await?;

        // Handle function calls if present (limit to prevent infinite loops)
        // Note: This is a simplified version - in production, track depth across calls
        #[allow(dead_code)]
        const MAX_FUNCTION_CALL_DEPTH: usize = 10;
        const MAX_FUNCTION_CALLS_PER_RESPONSE: usize = 5;
        
        if let Some(function_calls) = response.function_calls {
            let function_calling = self.function_calling.as_ref().ok_or_else(|| {
                LLMError::BrainIntegration("Function calling not initialized".to_string())
            })?;
            
            // Limit number of function calls per response
            for call in function_calls.iter().take(MAX_FUNCTION_CALLS_PER_RESPONSE) {
                // Validate function name to prevent injection
                if call.name.is_empty() || call.name.len() > 100 {
                    return Err(LLMError::InvalidResponse("Invalid function name".to_string()));
                }
                
                // Validate function name characters
                if !call.name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                    return Err(LLMError::InvalidResponse("Invalid function name characters".to_string()));
                }
                
                // Validate arguments size
                if call.arguments.len() > 10000 {
                    return Err(LLMError::InvalidResponse("Function arguments too large".to_string()));
                }
                
                // Validate JSON structure before parsing
                if let Err(e) = serde_json::from_str::<serde_json::Value>(&call.arguments) {
                    return Err(LLMError::InvalidResponse(format!("Invalid function arguments JSON: {}", e)));
                }
                
                let result = function_calling
                    .execute_function_call(&call.name, &call.arguments)
                    .await?;
                
                // Limit result size to prevent huge responses
                let result_str = serde_json::to_string(&result)?;
                if result_str.len() > 100000 {
                    return Err(LLMError::InvalidResponse("Function result too large".to_string()));
                }
                
                // Continue conversation with function result
                let mut new_messages = messages_clone.clone();
                
                // Limit message history to prevent context explosion
                if new_messages.len() > 50 {
                    new_messages = new_messages[new_messages.len() - 50..].to_vec();
                }
                
                new_messages.push(Message {
                    role: MessageRole::Assistant,
                    content: response.content.clone(),
                });
                new_messages.push(Message {
                    role: MessageRole::User,
                    content: format!("Function result: {}", result_str),
                });

                // Recursive call - in production, should track depth
                return self.chat(new_messages, Some(provider)).await;
            }
        }

        if let Some(tool_calls) = response.tool_calls {
            let function_calling = self.function_calling.as_ref().ok_or_else(|| {
                LLMError::BrainIntegration("Function calling not initialized".to_string())
            })?;
            
            // Limit number of tool calls per response
            for call in tool_calls.iter().take(MAX_FUNCTION_CALLS_PER_RESPONSE) {
                // Validate function name
                if call.function.name.is_empty() || call.function.name.len() > 100 {
                    return Err(LLMError::InvalidResponse("Invalid tool function name".to_string()));
                }
                
                // Validate function name characters
                if !call.function.name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                    return Err(LLMError::InvalidResponse("Invalid tool function name characters".to_string()));
                }
                
                // Validate arguments size
                if call.function.arguments.len() > 10000 {
                    return Err(LLMError::InvalidResponse("Tool arguments too large".to_string()));
                }
                
                // Validate JSON structure
                if let Err(e) = serde_json::from_str::<serde_json::Value>(&call.function.arguments) {
                    return Err(LLMError::InvalidResponse(format!("Invalid tool arguments JSON: {}", e)));
                }
                
                let result = function_calling
                    .execute_function_call(&call.function.name, &call.function.arguments)
                    .await?;
                
                // Limit result size
                let result_str = serde_json::to_string(&result)?;
                if result_str.len() > 100000 {
                    return Err(LLMError::InvalidResponse("Tool result too large".to_string()));
                }
                
                // Continue conversation with tool result
                let mut new_messages = messages_clone.clone();
                
                // Limit message history
                if new_messages.len() > 50 {
                    new_messages = new_messages[new_messages.len() - 50..].to_vec();
                }
                
                new_messages.push(Message {
                    role: MessageRole::Assistant,
                    content: response.content.clone(),
                });
                new_messages.push(Message {
                    role: MessageRole::User,
                    content: format!("Tool result: {}", result_str),
                });

                return self.chat(new_messages, Some(provider)).await;
            }
        }

        Ok(response.content)
    }

    /// Get RAG system
    pub fn rag(&self) -> Option<&RAGSystem> {
        self.rag.as_ref()
    }

    /// Get function calling system
    pub fn function_calling(&self) -> Option<&FunctionCallingSystem> {
        self.function_calling.as_ref()
    }

    /// Get reasoning system
    pub fn reasoning(&self) -> &ReasoningSystem {
        &self.reasoning
    }

    /// Get planning system
    pub fn planning(&self) -> &PlanningSystem {
        &self.planning
    }
}

