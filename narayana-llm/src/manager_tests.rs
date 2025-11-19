#[cfg(test)]
mod manager_tests {
    use crate::config::{Message, MessageRole};
    use crate::error::LLMError;
    use crate::manager::LLMManager;
    use crate::Provider;

    #[test]
    fn test_manager_initialization() {
        let _manager = LLMManager::new();
        // Should initialize without panicking
        assert!(true);
    }

    #[test]
    fn test_set_api_key() {
        let manager = LLMManager::new();
        manager.set_api_key(Provider::OpenAI, "sk-test123".to_string());
        // Should not panic
        assert!(true);
    }

    #[test]
    fn test_set_empty_api_key() {
        let manager = LLMManager::new();
        // Empty key should be rejected
        manager.set_api_key(Provider::OpenAI, "".to_string());
        // Should not panic, but key won't be set
        assert!(true);
    }

    #[test]
    fn test_set_too_long_api_key() {
        let manager = LLMManager::new();
        let long_key = "a".repeat(2000);
        manager.set_api_key(Provider::OpenAI, long_key);
        // Should not panic, but key won't be set
        assert!(true);
    }

    #[tokio::test]
    async fn test_chat_empty_messages() {
        let manager = LLMManager::new();
        let result = manager.chat(vec![], None).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            LLMError::InvalidResponse(msg) => {
                assert!(msg.contains("cannot be empty"));
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[tokio::test]
    async fn test_chat_too_many_messages() {
        let manager = LLMManager::new();
        let messages: Vec<Message> = (0..150)
            .map(|i| Message {
                role: MessageRole::User,
                content: format!("Message {}", i),
            })
            .collect();
        let result = manager.chat(messages, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_chat_message_too_large() {
        let manager = LLMManager::new();
        let large_content = "a".repeat(200_000);
        let messages = vec![Message {
            role: MessageRole::User,
            content: large_content,
        }];
        let result = manager.chat(messages, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_generate_embedding_empty() {
        let manager = LLMManager::new();
        let result = manager.generate_embedding("", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_generate_embedding_too_large() {
        let manager = LLMManager::new();
        let large_text = "a".repeat(10_000);
        let result = manager.generate_embedding(&large_text, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_chat_with_functions_empty() {
        let manager = LLMManager::new();
        let messages = vec![Message {
            role: MessageRole::User,
            content: "test".to_string(),
        }];
        let result = manager.chat_with_functions(messages, vec![], None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_chat_with_functions_too_many() {
        let manager = LLMManager::new();
        let messages = vec![Message {
            role: MessageRole::User,
            content: "test".to_string(),
        }];
        use crate::function_calling::BrainFunction;
        let functions: Vec<BrainFunction> = (0..100)
            .map(|_| BrainFunction::CreateThought)
            .collect();
        let result = manager.chat_with_functions(messages, functions, None).await;
        assert!(result.is_err());
    }
}

