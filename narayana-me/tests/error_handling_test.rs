//! Error handling tests for narayana-me

use narayana_me::{AvatarConfig, AvatarBroker, AvatarProviderType, Expression, Gesture, Emotion};
use narayana_me::error::AvatarError;

#[tokio::test]
async fn test_invalid_config_errors() {
    let mut config = AvatarConfig::default();
    
    // Test various invalid configurations
    config.expression_sensitivity = -1.0;
    assert!(AvatarBroker::new(config.clone()).is_err());
    
    config.expression_sensitivity = 2.0;
    assert!(AvatarBroker::new(config.clone()).is_err());
    
    config.expression_sensitivity = 0.5;
    config.animation_speed = 0.0;
    assert!(AvatarBroker::new(config.clone()).is_err());
    
    config.animation_speed = 3.0;
    assert!(AvatarBroker::new(config.clone()).is_err());
    
    config.animation_speed = 1.0;
    config.websocket_port = Some(0);
    assert!(AvatarBroker::new(config.clone()).is_err());
    
    // Port 65536 is out of range for u16, so we can't test it directly
    // The type system prevents creating such a config
}

#[tokio::test]
async fn test_start_stream_without_initialization() {
    let mut config = AvatarConfig::default();
    config.enabled = true;
    config.provider = AvatarProviderType::BeyondPresence;
    let broker = AvatarBroker::new(config).unwrap();
    
    // Try to start stream without initialization
    let result = broker.start_stream().await;
    assert!(result.is_err());
    assert!(format!("{}", result.unwrap_err()).contains("not initialized"));
}

#[tokio::test]
async fn test_set_expression_without_initialization() {
    let mut config = AvatarConfig::default();
    config.enabled = true;
    config.provider = AvatarProviderType::BeyondPresence;
    let broker = AvatarBroker::new(config).unwrap();
    
    // Try to set expression without initialization
    let result = broker.set_expression(Expression::Happy, 0.8).await;
    assert!(result.is_err());
    assert!(format!("{}", result.unwrap_err()).contains("not initialized"));
}

#[tokio::test]
async fn test_set_gesture_without_initialization() {
    let mut config = AvatarConfig::default();
    config.enabled = true;
    config.provider = AvatarProviderType::BeyondPresence;
    let broker = AvatarBroker::new(config).unwrap();
    
    // Try to set gesture without initialization
    let result = broker.set_gesture(Gesture::Wave, 1000).await;
    assert!(result.is_err());
    assert!(format!("{}", result.unwrap_err()).contains("not initialized"));
}

#[tokio::test]
async fn test_send_audio_without_initialization() {
    let mut config = AvatarConfig::default();
    config.enabled = true;
    config.provider = AvatarProviderType::BeyondPresence;
    let broker = AvatarBroker::new(config).unwrap();
    
    // Try to send audio without initialization
    let result = broker.send_audio(vec![0, 1, 2, 3]).await;
    assert!(result.is_err());
    assert!(format!("{}", result.unwrap_err()).contains("not initialized"));
}

#[tokio::test]
async fn test_oversized_audio_rejection() {
    let mut config = AvatarConfig::default();
    config.enabled = true;
    config.enable_lip_sync = true;
    config.provider = AvatarProviderType::BeyondPresence;
    let broker = AvatarBroker::new(config).unwrap();
    
    // Create oversized audio (10MB + 1 byte)
    let oversized_audio = vec![0; 10 * 1024 * 1024 + 1];
    let result = broker.send_audio(oversized_audio).await;
    assert!(result.is_err());
    assert!(format!("{}", result.unwrap_err()).contains("too large"));
}

#[tokio::test]
async fn test_invalid_intensity_values() {
    let mut config = AvatarConfig::default();
    config.enabled = true;
    config.provider = AvatarProviderType::BeyondPresence;
    let broker = AvatarBroker::new(config).unwrap();
    
    // Test NaN
    let result = broker.set_expression(Expression::Happy, f64::NAN).await;
    assert!(result.is_err());
    
    // Test Infinity
    let result = broker.set_expression(Expression::Happy, f64::INFINITY).await;
    assert!(result.is_err());
    
    // Test negative infinity
    let result = broker.set_expression(Expression::Happy, f64::NEG_INFINITY).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_invalid_emotion_intensity() {
    let mut config = AvatarConfig::default();
    config.enabled = true;
    config.provider = AvatarProviderType::BeyondPresence;
    let broker = AvatarBroker::new(config).unwrap();
    
    // Test NaN emotion intensity
    let _result = broker.update_emotion(Emotion::Joy, f64::NAN).await;
    // Should handle gracefully (clamp or error)
}

#[tokio::test]
async fn test_unsupported_provider() {
    // All providers are now implemented, so this test checks that providers are accessible
    let mut config = AvatarConfig::default();
    config.enabled = true;
    config.provider = AvatarProviderType::LiveAvatar;
    let broker = AvatarBroker::new(config).unwrap();
    
    // Try to initialize (will fail without API key, but structure should be OK)
    let result = broker.initialize().await;
    // Should either succeed or fail with API error, not "not yet implemented"
    if let Err(e) = &result {
        let error_msg = format!("{}", e);
        assert!(!error_msg.contains("not yet implemented"), "Provider should be implemented, got: {}", error_msg);
    }
}

#[tokio::test]
async fn test_provider_not_enabled_feature() {
    // This test would require building without the feature flag
    // For now, we test that the error message is correct
    let mut config = AvatarConfig::default();
    config.enabled = true;
    config.provider = AvatarProviderType::BeyondPresence;
    
    #[cfg(not(feature = "beyond-presence"))]
    {
        let broker = AvatarBroker::new(config).unwrap();
        let result = broker.initialize().await;
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("not enabled"));
    }
    
    #[cfg(feature = "beyond-presence")]
    {
        // With feature enabled, this test is skipped
        // (would require API key to actually initialize)
    }
}

