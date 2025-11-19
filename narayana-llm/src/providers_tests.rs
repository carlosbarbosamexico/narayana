#[cfg(test)]
mod providers_tests {
    use crate::providers::*;
    use crate::config::*;
    use crate::error::LLMError;

    #[test]
    fn test_openai_provider_creation() {
        let provider = openai::OpenAIProvider::new();
        assert_eq!(provider.name(), "openai");
        assert!(!provider.has_api_key());
    }

    #[test]
    fn test_openai_provider_with_key() {
        let provider = openai::OpenAIProvider::with_api_key("sk-test123".to_string());
        assert!(provider.has_api_key());
    }

    #[test]
    fn test_anthropic_provider_creation() {
        let provider = anthropic::AnthropicProvider::new();
        assert_eq!(provider.name(), "anthropic");
        assert!(!provider.has_api_key());
    }

    #[test]
    fn test_anthropic_provider_with_key() {
        let provider = anthropic::AnthropicProvider::with_api_key("sk-test123".to_string());
        assert!(provider.has_api_key());
    }

    #[test]
    fn test_google_provider_creation() {
        let provider = google::GoogleProvider::new();
        assert_eq!(provider.name(), "google");
        assert!(!provider.has_api_key());
    }

    #[test]
    fn test_google_provider_with_key() {
        let provider = google::GoogleProvider::with_api_key("test-key".to_string());
        assert!(provider.has_api_key());
    }

    #[test]
    fn test_cohere_provider_creation() {
        let provider = cohere::CohereProvider::new();
        assert_eq!(provider.name(), "cohere");
        assert!(!provider.has_api_key());
    }

    #[test]
    fn test_cohere_provider_with_key() {
        let provider = cohere::CohereProvider::with_api_key("test-key".to_string());
        assert!(provider.has_api_key());
    }

    #[test]
    fn test_provider_supports_function_calling() {
        let openai = openai::OpenAIProvider::new();
        assert!(openai.supports_function_calling());
        
        let anthropic = anthropic::AnthropicProvider::new();
        assert!(!anthropic.supports_function_calling());
        
        let google = google::GoogleProvider::new();
        assert!(!google.supports_function_calling());
        
        let cohere = cohere::CohereProvider::new();
        assert!(!cohere.supports_function_calling());
    }

    #[test]
    fn test_provider_supports_tool_use() {
        let openai = openai::OpenAIProvider::new();
        assert!(openai.supports_tool_use());
        
        let anthropic = anthropic::AnthropicProvider::new();
        assert!(anthropic.supports_tool_use());
        
        let google = google::GoogleProvider::new();
        assert!(!google.supports_tool_use());
        
        let cohere = cohere::CohereProvider::new();
        assert!(cohere.supports_tool_use());
    }

    #[test]
    fn test_provider_available_models() {
        let openai = openai::OpenAIProvider::new();
        let models = openai.available_models();
        assert!(!models.is_empty());
        // Check that at least one known model is present
        assert!(models.iter().any(|m| m.contains("gpt")));
        
        let anthropic = anthropic::AnthropicProvider::new();
        let models = anthropic.available_models();
        assert!(!models.is_empty());
        assert!(models.iter().any(|m| m.contains("claude")));
        
        let google = google::GoogleProvider::new();
        let models = google.available_models();
        assert!(!models.is_empty());
        
        let cohere = cohere::CohereProvider::new();
        let models = cohere.available_models();
        assert!(!models.is_empty());
    }

    #[test]
    fn test_provider_set_api_key() {
        let mut openai = openai::OpenAIProvider::new();
        assert!(!openai.has_api_key());
        
        openai.set_api_key("sk-test123".to_string());
        assert!(openai.has_api_key());
    }

    #[tokio::test]
    async fn test_provider_chat_without_key() {
        let openai = openai::OpenAIProvider::new();
        let request = ChatRequest {
            messages: vec![Message {
                role: MessageRole::User,
                content: "test".to_string(),
            }],
            model: None,
            temperature: None,
            max_tokens: None,
            functions: None,
            tools: None,
        };
        
        let result = openai.chat(request).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            LLMError::MissingApiKey(_) => {}
            _ => panic!("Expected MissingApiKey error"),
        }
    }
}

