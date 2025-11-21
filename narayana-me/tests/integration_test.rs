//! Integration tests for narayana-me

use narayana_me::{AvatarConfig, AvatarBroker, AvatarProviderType, Expression, Gesture, Emotion};
use narayana_core::Error;

#[tokio::test]
async fn test_avatar_config_validation() {
    let config = AvatarConfig::default();
    assert!(!config.enabled);
    assert!(config.validate().is_ok());
}

#[tokio::test]
async fn test_avatar_config_validation_edge_cases() {
    // Test valid boundaries
    let mut config = AvatarConfig::default();
    config.expression_sensitivity = 0.0;
    assert!(config.validate().is_ok());
    
    config.expression_sensitivity = 1.0;
    assert!(config.validate().is_ok());
    
    config.animation_speed = 0.5;
    assert!(config.validate().is_ok());
    
    config.animation_speed = 2.0;
    assert!(config.validate().is_ok());
    
    // Test invalid boundaries
    config.expression_sensitivity = -0.1;
    assert!(config.validate().is_err());
    
    config.expression_sensitivity = 1.1;
    assert!(config.validate().is_err());
    
    config.animation_speed = 0.4;
    assert!(config.validate().is_err());
    
    config.animation_speed = 2.1;
    assert!(config.validate().is_err());
}

#[tokio::test]
async fn test_avatar_config_port_validation() {
    let mut config = AvatarConfig::default();
    
    // Valid ports
    config.websocket_port = Some(8080);
    assert!(config.validate().is_ok());
    
    config.websocket_port = Some(65535);
    assert!(config.validate().is_ok());
    
    // Invalid ports
    config.websocket_port = Some(0);
    assert!(config.validate().is_err());
    
    // Port 65536 is out of range for u16, so we can't test it directly
    // But the validation should catch it if somehow it got through
}

#[tokio::test]
async fn test_avatar_config_avatar_id_validation() {
    let mut config = AvatarConfig::default();
    
    // Valid avatar IDs
    config.avatar_id = Some("valid_avatar_123".to_string());
    assert!(config.validate().is_ok());
    
    config.avatar_id = Some("avatar-with-dashes".to_string());
    assert!(config.validate().is_ok());
    
    // Invalid avatar IDs
    config.avatar_id = Some("".to_string());
    assert!(config.validate().is_err());
    
    config.avatar_id = Some("a".repeat(257));
    assert!(config.validate().is_err());
    
    config.avatar_id = Some("invalid\nid".to_string());
    assert!(config.validate().is_err());
}

#[tokio::test]
async fn test_avatar_config_provider_config_validation() {
    let mut config = AvatarConfig::default();
    
    // Valid provider config
    config.provider_config = Some(serde_json::json!({
        "key": "value",
        "nested": {
            "inner": "data"
        }
    }));
    assert!(config.validate().is_ok());
    
    // Invalid: not an object
    config.provider_config = Some(serde_json::json!([]));
    assert!(config.validate().is_err());
    
    config.provider_config = Some(serde_json::json!("string"));
    assert!(config.validate().is_err());
    
    // Too deeply nested
    let mut deep_json = serde_json::json!({});
    let mut current = &mut deep_json;
    for _ in 0..35 {
        *current = serde_json::json!({ "nested": {} });
        current = current.get_mut("nested").unwrap();
    }
    config.provider_config = Some(deep_json);
    assert!(config.validate().is_err());
}

#[tokio::test]
async fn test_avatar_broker_creation() {
    let config = AvatarConfig::default();
    let broker = AvatarBroker::new(config);
    assert!(broker.is_ok());
}

#[tokio::test]
async fn test_avatar_broker_with_invalid_config() {
    let mut config = AvatarConfig::default();
    config.expression_sensitivity = 2.0; // Invalid: should be 0.0-1.0
    let broker = AvatarBroker::new(config);
    assert!(broker.is_err());
}

#[tokio::test]
async fn test_avatar_broker_initialization_disabled() {
    let config = AvatarConfig::default();
    assert!(!config.enabled);
    let broker = AvatarBroker::new(config).unwrap();
    let result = broker.initialize().await;
    assert!(result.is_ok()); // Should succeed even when disabled
}

#[tokio::test]
async fn test_avatar_broker_idempotent_initialization() {
    let mut config = AvatarConfig::default();
    config.enabled = true;
    config.provider = AvatarProviderType::BeyondPresence;
    let broker = AvatarBroker::new(config).unwrap();
    
    // First initialization
    let result1 = broker.initialize().await;
    // Should either succeed or fail with API error (not panic)
    
    // Second initialization (should be idempotent)
    let result2 = broker.initialize().await;
    // Should not error on second call
    assert!(result2.is_ok() || format!("{}", result1.unwrap_err()).contains("API"));
}

#[tokio::test]
async fn test_expression_mapping() {
    use narayana_me::Emotion;
    
    let emotion = Emotion::Joy;
    let expression = emotion.to_expression();
    assert!(matches!(expression, Expression::Happy));
    
    let emotion = Emotion::Sadness;
    let expression = emotion.to_expression();
    assert!(matches!(expression, Expression::Sad));
    
    let emotion = Emotion::Anger;
    let expression = emotion.to_expression();
    assert!(matches!(expression, Expression::Angry));
    
    let emotion = Emotion::Surprise;
    let expression = emotion.to_expression();
    assert!(matches!(expression, Expression::Surprised));
    
    let emotion = Emotion::Thinking;
    let expression = emotion.to_expression();
    assert!(matches!(expression, Expression::Thinking));
    
    let emotion = Emotion::Recognition;
    let expression = emotion.to_expression();
    assert!(matches!(expression, Expression::Recognition));
    
    let emotion = Emotion::Neutral;
    let expression = emotion.to_expression();
    assert!(matches!(expression, Expression::Neutral));
}

#[tokio::test]
async fn test_avatar_broker_set_expression_disabled() {
    let mut config = AvatarConfig::default();
    config.enabled = false;
    let broker = AvatarBroker::new(config).unwrap();
    
    // Should succeed even when disabled (no-op)
    let _result = broker.set_expression(Expression::Happy, 0.8).await;
    // Either succeeds or fails because not initialized, but should not panic
}

#[tokio::test]
async fn test_avatar_broker_set_gesture_disabled() {
    let mut config = AvatarConfig::default();
    config.enabled = false;
    config.enable_gestures = false;
    let broker = AvatarBroker::new(config).unwrap();
    
    // Should succeed even when disabled (no-op)
    let result = broker.set_gesture(Gesture::Wave, 1000).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_avatar_broker_send_audio_disabled() {
    let mut config = AvatarConfig::default();
    config.enabled = false;
    config.enable_lip_sync = false;
    let broker = AvatarBroker::new(config).unwrap();
    
    // Should succeed even when disabled (no-op)
    let result = broker.send_audio(vec![0, 1, 2, 3]).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_avatar_broker_send_empty_audio() {
    let config = AvatarConfig::default();
    let broker = AvatarBroker::new(config).unwrap();
    
    // Empty audio should be accepted
    let result = broker.send_audio(vec![]).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_avatar_broker_expression_intensity_clamping() {
    let mut config = AvatarConfig::default();
    config.enabled = true;
    config.provider = AvatarProviderType::BeyondPresence;
    config.expression_sensitivity = 0.5;
    let broker = AvatarBroker::new(config).unwrap();
    
    // Test intensity clamping
    // Even with extreme values, should not panic
    let _ = broker.set_expression(Expression::Happy, -100.0).await;
    let _ = broker.set_expression(Expression::Happy, 100.0).await;
    let _ = broker.set_expression(Expression::Happy, f64::NAN).await;
    let _ = broker.set_expression(Expression::Happy, f64::INFINITY).await;
}

#[tokio::test]
async fn test_avatar_broker_gesture_duration_limits() {
    let mut config = AvatarConfig::default();
    config.enabled = true;
    config.provider = AvatarProviderType::BeyondPresence;
    let broker = AvatarBroker::new(config).unwrap();
    
    // Very long duration should be clamped
    let _result = broker.set_gesture(Gesture::Wave, 1_000_000_000).await;
    // Should not panic, either succeeds or fails gracefully
}

#[tokio::test]
async fn test_avatar_broker_update_emotion() {
    let mut config = AvatarConfig::default();
    config.enabled = true;
    config.provider = AvatarProviderType::BeyondPresence;
    let broker = AvatarBroker::new(config).unwrap();
    
    // Test emotion update (maps to expression)
    let _result = broker.update_emotion(Emotion::Joy, 0.8).await;
    // Should not panic
}

#[tokio::test]
async fn test_avatar_broker_get_client_url_no_stream() {
    let config = AvatarConfig::default();
    let broker = AvatarBroker::new(config).unwrap();
    
    // No stream started, should return None
    let url = broker.get_client_url().await;
    assert!(url.is_none());
}

#[tokio::test]
async fn test_avatar_broker_stop_stream_no_stream() {
    let config = AvatarConfig::default();
    let broker = AvatarBroker::new(config).unwrap();
    
    // Stopping non-existent stream should succeed (idempotent)
    let result = broker.stop_stream().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_avatar_broker_idempotent_stop_stream() {
    let config = AvatarConfig::default();
    let broker = AvatarBroker::new(config).unwrap();
    
    // Stop multiple times should be idempotent
    let result1 = broker.stop_stream().await;
    assert!(result1.is_ok());
    
    let result2 = broker.stop_stream().await;
    assert!(result2.is_ok());
}

#[tokio::test]
async fn test_expression_serialization() {
    // Test that expressions can be serialized/deserialized
    let expr = Expression::Happy;
    let json = serde_json::to_string(&expr).unwrap();
    let deserialized: Expression = serde_json::from_str(&json).unwrap();
    assert_eq!(expr, deserialized);
    
    let custom_expr = Expression::Custom("my_custom_expression".to_string());
    let json = serde_json::to_string(&custom_expr).unwrap();
    let deserialized: Expression = serde_json::from_str(&json).unwrap();
    assert_eq!(custom_expr, deserialized);
}

#[tokio::test]
async fn test_gesture_serialization() {
    // Test that gestures can be serialized/deserialized
    let gesture = Gesture::Wave;
    let json = serde_json::to_string(&gesture).unwrap();
    let deserialized: Gesture = serde_json::from_str(&json).unwrap();
    assert_eq!(gesture, deserialized);
    
    let custom_gesture = Gesture::Custom("my_custom_gesture".to_string());
    let json = serde_json::to_string(&custom_gesture).unwrap();
    let deserialized: Gesture = serde_json::from_str(&json).unwrap();
    assert_eq!(custom_gesture, deserialized);
}

#[tokio::test]
async fn test_emotion_serialization() {
    // Test that emotions can be serialized/deserialized
    let emotion = Emotion::Joy;
    let json = serde_json::to_string(&emotion).unwrap();
    let deserialized: Emotion = serde_json::from_str(&json).unwrap();
    assert_eq!(emotion, deserialized);
}

#[tokio::test]
async fn test_avatar_config_serialization() {
    // Test full config serialization
    let mut config = AvatarConfig::default();
    config.enabled = true;
    config.expression_sensitivity = 0.8;
    config.avatar_id = Some("test_avatar".to_string());
    config.provider_config = Some(serde_json::json!({ "key": "value" }));
    
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: AvatarConfig = serde_json::from_str(&json).unwrap();
    
    assert_eq!(config.enabled, deserialized.enabled);
    assert_eq!(config.expression_sensitivity, deserialized.expression_sensitivity);
    assert_eq!(config.avatar_id, deserialized.avatar_id);
}

#[tokio::test]
async fn test_provider_type_serialization() {
    // Test all provider types can be serialized
    let providers = vec![
        AvatarProviderType::BeyondPresence,
        AvatarProviderType::LiveAvatar,
        AvatarProviderType::ReadyPlayerMe,
        AvatarProviderType::AvatarSDK,
        AvatarProviderType::OpenAvatarChat,
    ];
    
    for provider in providers {
        let json = serde_json::to_string(&provider).unwrap();
        let deserialized: AvatarProviderType = serde_json::from_str(&json).unwrap();
        assert_eq!(provider, deserialized);
    }
}
