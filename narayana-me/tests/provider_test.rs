//! Tests for all avatar providers

use narayana_me::{AvatarConfig, AvatarBroker, AvatarProviderType};

#[tokio::test]
async fn test_live_avatar_provider() {
    let mut config = AvatarConfig::default();
    config.enabled = true;
    config.provider = AvatarProviderType::LiveAvatar;
    
    let broker = AvatarBroker::new(config).unwrap();
    
    // Try to initialize (will fail if API key not set, but structure should be OK)
    let result = broker.initialize().await;
    // Either succeeds or fails with API error, but should not panic
    if let Err(e) = &result {
        let error_msg = format!("{}", e);
        assert!(error_msg.contains("API") || error_msg.contains("not set"), "Unexpected error: {}", error_msg);
    }
}

#[tokio::test]
async fn test_ready_player_me_provider() {
    let mut config = AvatarConfig::default();
    config.enabled = true;
    config.provider = AvatarProviderType::ReadyPlayerMe;
    
    let broker = AvatarBroker::new(config).unwrap();
    
    // Try to initialize (will fail if API key not set, but structure should be OK)
    let result = broker.initialize().await;
    // Either succeeds or fails with API error, but should not panic
    if let Err(e) = &result {
        let error_msg = format!("{}", e);
        assert!(error_msg.contains("API") || error_msg.contains("not set"), "Unexpected error: {}", error_msg);
    }
}

#[tokio::test]
async fn test_avatar_sdk_provider() {
    let mut config = AvatarConfig::default();
    config.enabled = true;
    config.provider = AvatarProviderType::AvatarSDK;
    
    let broker = AvatarBroker::new(config).unwrap();
    
    // Try to initialize (will fail if API key not set, but structure should be OK)
    let result = broker.initialize().await;
    // Either succeeds or fails with API error, but should not panic
    if let Err(e) = &result {
        let error_msg = format!("{}", e);
        assert!(error_msg.contains("API") || error_msg.contains("not set"), "Unexpected error: {}", error_msg);
    }
}

#[tokio::test]
async fn test_open_avatar_chat_provider() {
    let mut config = AvatarConfig::default();
    config.enabled = true;
    config.provider = AvatarProviderType::OpenAvatarChat;
    
    let broker = AvatarBroker::new(config).unwrap();
    
    // Try to initialize (API key optional for open-source, but structure should be OK)
    let result = broker.initialize().await;
    // Either succeeds or fails with API error, but should not panic
    if let Err(e) = &result {
        let error_msg = format!("{}", e);
        assert!(error_msg.contains("API") || error_msg.contains("not set"), "Unexpected error: {}", error_msg);
    }
}

#[tokio::test]
async fn test_all_provider_types() {
    let providers = vec![
        AvatarProviderType::BeyondPresence,
        AvatarProviderType::LiveAvatar,
        AvatarProviderType::ReadyPlayerMe,
        AvatarProviderType::AvatarSDK,
        AvatarProviderType::OpenAvatarChat,
    ];
    
    for provider in providers {
        let mut config = AvatarConfig::default();
        config.enabled = true;
        config.provider = provider.clone();
        
        // Should create broker successfully
        let broker = AvatarBroker::new(config);
        assert!(broker.is_ok(), "Failed to create broker for {:?}", provider);
        
        // Initialize may fail without API keys, but should not panic
        let broker = broker.unwrap();
        let _ = broker.initialize().await; // Ignore result (may fail without API keys)
    }
}

