// Error recovery and resilience tests for narayana-rde

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
async fn test_publish_continues_on_delivery_failure() {
    let manager = create_test_manager();
    
    let source = Actor::new(
        ActorId::from("source1"),
        "Source".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    let origin = Actor::new(
        ActorId::from("origin1"),
        "Origin".to_string(),
        ActorType::Origin,
        "token-123456789012".to_string(),
    );
    
    manager.register_actor(source).await.unwrap();
    manager.register_actor(origin).await.unwrap();
    
    // Subscribe with invalid webhook URL (will fail delivery)
    manager.subscribe(
        &ActorId::from("origin1"),
        "token-123456789012",
        "source1:test_event",
        TransportType::Webhook,
        Some(serde_json::json!({
            "webhook_url": "http://localhost:9999/invalid" // Will fail SSRF check or network
        })),
    ).await.unwrap();
    
    // Publish event - should succeed even if delivery fails
    let result = manager.publish_event(
        &ActorId::from("source1"),
        "token-123456789012",
        "test_event",
        serde_json::json!({"data": "test"}),
    ).await;
    
    // Publish should succeed (delivery failure is logged but doesn't fail publish)
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_multiple_subscribers_partial_failure() {
    let manager = create_test_manager();
    
    let source = Actor::new(
        ActorId::from("source1"),
        "Source".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    
    manager.register_actor(source).await.unwrap();
    
    // Register multiple origins
    for i in 1..=3 {
        let origin = Actor::new(
            ActorId::from(format!("origin{}", i)),
            format!("Origin {}", i),
            ActorType::Origin,
            format!("token-origin{}-123456789012", i),
        );
        manager.register_actor(origin).await.unwrap();
        
        // Some with valid URLs, some with invalid
        let webhook_url = if i == 2 {
            "http://localhost:9999/invalid" // Invalid - will fail
        } else {
            "https://example.com/webhook"
        };
        
        manager.subscribe(
            &ActorId::from(format!("origin{}", i)),
            &format!("token-origin{}-123456789012", i),
            "source1:test_event",
            TransportType::Webhook,
            Some(serde_json::json!({
                "webhook_url": webhook_url
            })),
        ).await.unwrap();
    }
    
    // Publish event - should succeed even if one subscriber fails
    let result = manager.publish_event(
        &ActorId::from("source1"),
        "token-123456789012",
        "test_event",
        serde_json::json!({"data": "test"}),
    ).await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_transformation_failure_fallback() {
    let manager = create_test_manager();
    
    let source = Actor::new(
        ActorId::from("source1"),
        "Source".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    let origin = Actor::new(
        ActorId::from("origin1"),
        "Origin".to_string(),
        ActorType::Origin,
        "token-123456789012".to_string(),
    );
    
    manager.register_actor(source).await.unwrap();
    manager.register_actor(origin).await.unwrap();
    
    // Subscribe with transformation that will fail
    manager.subscribe(
        &ActorId::from("origin1"),
        "token-123456789012",
        "source1:test_event",
        TransportType::Webhook,
        Some(serde_json::json!({
            "webhook_url": "https://example.com/webhook",
            "output_config": {
                "transforms": [
                    {
                        "type": "field",
                        "source": "nonexistent_field",
                        "target": "new_field"
                    }
                ]
            }
        })),
    ).await.unwrap();
    
    // Publish event - transformation should fail but use original payload
    let result = manager.publish_event(
        &ActorId::from("source1"),
        "token-123456789012",
        "test_event",
        serde_json::json!({"existing_field": "value"}),
    ).await;
    
    // Should succeed (transformation failure falls back to original)
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_native_events_failure_continues() {
    let manager = create_test_manager();
    
    let source = Actor::new(
        ActorId::from("source1"),
        "Source".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    manager.register_actor(source).await.unwrap();
    
    // Publish event - even if native events system fails, delivery should continue
    let result = manager.publish_event(
        &ActorId::from("source1"),
        "token-123456789012",
        "test_event",
        serde_json::json!({"data": "test"}),
    ).await;
    
    // Should succeed (native events failure is logged but doesn't fail publish)
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_invalid_auth_token_handling() {
    let manager = create_test_manager();
    
    let source = Actor::new(
        ActorId::from("source1"),
        "Source".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    manager.register_actor(source).await.unwrap();
    
    // Try various invalid tokens
    let invalid_tokens = vec![
        "",
        "short",
        "wrong-token",
        "token-123456789012-wrong",
        "token-123456789012\n", // With control character
    ];
    
    for token in invalid_tokens {
        let result = manager.publish_event(
            &ActorId::from("source1"),
            token,
            "test_event",
            serde_json::json!({}),
        ).await;
        
        assert!(result.is_err(), "Should fail with invalid token: {:?}", token);
    }
}

#[tokio::test]
async fn test_invalid_actor_id_handling() {
    let manager = create_test_manager();
    
    let source = Actor::new(
        ActorId::from("source1"),
        "Source".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    manager.register_actor(source).await.unwrap();
    
    // Try publishing with non-existent actor
    let result = manager.publish_event(
        &ActorId::from("nonexistent"),
        "token-123456789012",
        "test_event",
        serde_json::json!({}),
    ).await;
    
    assert!(result.is_err());
}

#[tokio::test]
async fn test_invalid_event_name_handling() {
    let manager = create_test_manager();
    
    let source = Actor::new(
        ActorId::from("source1"),
        "Source".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    manager.register_actor(source).await.unwrap();
    
    // Try various invalid event names
    let invalid_names = vec![
        "",
        ":",
        "*",
        "event:name", // Contains colon
        "event\nname", // Contains control character
    ];
    
    for name in invalid_names {
        let result = manager.publish_event(
            &ActorId::from("source1"),
            "token-123456789012",
            name,
            serde_json::json!({}),
        ).await;
        
        assert!(result.is_err(), "Should fail with invalid event name: {:?}", name);
    }
}

#[tokio::test]
async fn test_recovery_after_error() {
    let manager = create_test_manager();
    
    let source = Actor::new(
        ActorId::from("source1"),
        "Source".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    manager.register_actor(source).await.unwrap();
    
    // First publish with invalid event name (should fail)
    let result1 = manager.publish_event(
        &ActorId::from("source1"),
        "token-123456789012",
        "", // Invalid
        serde_json::json!({}),
    ).await;
    assert!(result1.is_err());
    
    // Then publish with valid event name (should succeed)
    let result2 = manager.publish_event(
        &ActorId::from("source1"),
        "token-123456789012",
        "valid_event",
        serde_json::json!({"data": "test"}),
    ).await;
    assert!(result2.is_ok());
}



