// Edge case tests for narayana-rde

use narayana_rde::*;
use narayana_storage::native_events::{EventsConfig, NativeEventsSystem};
use std::sync::Arc;

fn create_test_manager() -> RdeManager {
    let mut config = EventsConfig::default();
    config.max_message_size = 10 * 1024 * 1024;
    config.enable_persistence = false;
    let native_events = Arc::new(NativeEventsSystem::new(config));
    RdeManager::new(native_events)
}

#[tokio::test]
async fn test_empty_payload() {
    let manager = create_test_manager();
    
    let source = Actor::new(
        ActorId::from("source1"),
        "Source".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    manager.register_actor(source).await.unwrap();
    
    // Empty object
    let result = manager.publish_event(
        &ActorId::from("source1"),
        "token-123456789012",
        "test_event",
        serde_json::json!({}),
    ).await;
    assert!(result.is_ok());
    
    // Null
    let result = manager.publish_event(
        &ActorId::from("source1"),
        "token-123456789012",
        "test_event2",
        serde_json::json!(null),
    ).await;
    assert!(result.is_ok());
    
    // Empty array
    let result = manager.publish_event(
        &ActorId::from("source1"),
        "token-123456789012",
        "test_event3",
        serde_json::json!([]),
    ).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_unicode_in_actor_id() {
    let manager = create_test_manager();
    
    // Valid unicode
    let actor = Actor::new(
        ActorId::from("café"),
        "Café".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    assert!(manager.register_actor(actor).await.is_ok());
    
    // Control characters should fail
    let actor = Actor::new(
        ActorId::from("test\u{0000}actor"),
        "Test".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    assert!(manager.register_actor(actor).await.is_err());
}

#[tokio::test]
async fn test_max_length_actor_id() {
    let manager = create_test_manager();
    
    // Exactly 256 chars - should work
    let long_id = "a".repeat(256);
    let actor = Actor::new(
        ActorId::from(long_id.clone()),
        "Test".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    assert!(manager.register_actor(actor).await.is_ok());
    
    // 257 chars - should fail
    let long_id = "a".repeat(257);
    let actor = Actor::new(
        ActorId::from(long_id),
        "Test".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    assert!(manager.register_actor(actor).await.is_err());
}

#[tokio::test]
async fn test_max_length_event_name() {
    let manager = create_test_manager();
    
    let source = Actor::new(
        ActorId::from("source1"),
        "Source".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    manager.register_actor(source).await.unwrap();
    
    // Exactly 256 chars - should work
    let long_name = "a".repeat(256);
    let result = manager.publish_event(
        &ActorId::from("source1"),
        "token-123456789012",
        &long_name,
        serde_json::json!({}),
    ).await;
    assert!(result.is_ok());
    
    // 257 chars - should fail
    let long_name = "a".repeat(257);
    let result = manager.publish_event(
        &ActorId::from("source1"),
        "token-123456789012",
        &long_name,
        serde_json::json!({}),
    ).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_nested_json_payload() {
    let manager = create_test_manager();
    
    let source = Actor::new(
        ActorId::from("source1"),
        "Source".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    manager.register_actor(source).await.unwrap();
    
    // Deeply nested JSON - build it from the inside out
    let mut nested = serde_json::json!({});
    for i in (0..10).rev() {
        nested = serde_json::json!({
            "level": i,
            "nested": nested
        });
    }
    let payload = nested;
    
    let result = manager.publish_event(
        &ActorId::from("source1"),
        "token-123456789012",
        "nested_event",
        payload,
    ).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_large_array_payload() {
    let manager = create_test_manager();
    
    let source = Actor::new(
        ActorId::from("source1"),
        "Source".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    manager.register_actor(source).await.unwrap();
    
    // Large array (but under 10MB)
    let array: Vec<serde_json::Value> = (0..10000)
        .map(|i| serde_json::json!({"index": i, "value": i * 2}))
        .collect();
    
    let payload = serde_json::json!(array);
    
    let result = manager.publish_event(
        &ActorId::from("source1"),
        "token-123456789012",
        "array_event",
        payload,
    ).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_special_characters_in_payload() {
    let manager = create_test_manager();
    
    let source = Actor::new(
        ActorId::from("source1"),
        "Source".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    manager.register_actor(source).await.unwrap();
    
    let payload = serde_json::json!({
        "special": "!@#$%^&*()_+-=[]{}|;':\",./<>?",
        "unicode": "你好世界 🌍",
        "newlines": "line1\nline2\rline3",
        "quotes": "\"double\" 'single'",
        "backslash": "\\path\\to\\file"
    });
    
    let result = manager.publish_event(
        &ActorId::from("source1"),
        "token-123456789012",
        "special_event",
        payload,
    ).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_multiple_events_same_name() {
    let manager = create_test_manager();
    
    let source = Actor::new(
        ActorId::from("source1"),
        "Source".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    manager.register_actor(source).await.unwrap();
    
    // Publish same event multiple times
    for i in 0..10 {
        let result = manager.publish_event(
            &ActorId::from("source1"),
            "token-123456789012",
            "repeated_event",
            serde_json::json!({"iteration": i}),
        ).await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_schema_extraction_various_types() {
    let manager = create_test_manager();
    
    let source = Actor::new(
        ActorId::from("source1"),
        "Source".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    manager.register_actor(source).await.unwrap();
    
    // Test schema extraction for different payload types
    let test_cases = vec![
        ("object", serde_json::json!({"key": "value"})),
        ("array", serde_json::json!([1, 2, 3])),
        ("string", serde_json::json!("string")),
        ("number", serde_json::json!(42)),
        ("boolean", serde_json::json!(true)),
        ("null", serde_json::json!(null)),
    ];
    
    for (name, payload) in test_cases {
        let result = manager.publish_event(
            &ActorId::from("source1"),
            "token-123456789012",
            &format!("test_{}", name),
            payload,
        ).await;
        assert!(result.is_ok(), "Failed for type: {}", name);
    }
}

#[tokio::test]
async fn test_reserved_actor_id_rejection() {
    let manager = create_test_manager();
    
    // Try to register with "*"
    let actor = Actor::new(
        ActorId::from("*"),
        "Test".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    assert!(manager.register_actor(actor).await.is_err());
    
    // Try to register with ":"
    let actor = Actor::new(
        ActorId::from(":"),
        "Test".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    assert!(manager.register_actor(actor).await.is_err());
}

#[tokio::test]
async fn test_invalid_namespaced_event_format() {
    let manager = create_test_manager();
    
    let origin = Actor::new(
        ActorId::from("origin1"),
        "Origin".to_string(),
        ActorType::Origin,
        "token-123456789012".to_string(),
    );
    manager.register_actor(origin).await.unwrap();
    
    // Invalid format - multiple colons
    let result = manager.subscribe(
        &ActorId::from("origin1"),
        "token-123456789012",
        "actor:event:extra",
        TransportType::Webhook,
        None,
    ).await;
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid namespaced"));
}

#[tokio::test]
async fn test_concurrent_subscription_race() {
    let manager = create_test_manager();
    
    let origin = Actor::new(
        ActorId::from("origin1"),
        "Origin".to_string(),
        ActorType::Origin,
        "token-123456789012".to_string(),
    );
    manager.register_actor(origin).await.unwrap();
    
    // Try to subscribe to same event multiple times (sequential)
    // All should succeed (multiple subscriptions to same event are allowed)
    for _ in 0..10 {
        let result = manager.subscribe(
            &ActorId::from("origin1"),
            "token-123456789012",
            "source1:same_event",
            TransportType::Webhook,
            None,
        ).await;
        assert!(result.is_ok());
    }
}

