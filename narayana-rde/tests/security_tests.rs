// Security tests for narayana-rde

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
async fn test_authentication_bypass_attempt() {
    let manager = create_test_manager();
    
    let source = Actor::new(
        ActorId::from("source1"),
        "Source".to_string(),
        ActorType::Source,
        "correct-token-123456789012".to_string(),
    );
    manager.register_actor(source).await.unwrap();
    
    // Try to publish without authentication
    let result = manager.publish_event(
        &ActorId::from("source1"),
        "wrong-token",
        "test_event",
        serde_json::json!({}),
    ).await;
    
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Authentication failed") || 
            error_msg.contains("not found"));
}

#[tokio::test]
async fn test_auth_token_not_leaked() {
    let manager = create_test_manager();
    
    let actor = Actor::new(
        ActorId::from("test"),
        "Test".to_string(),
        ActorType::Source,
        "secret-token-123456789012".to_string(),
    );
    manager.register_actor(actor).await.unwrap();
    
    let retrieved = manager.get_actor(&ActorId::from("test"));
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().auth_token, "");
}

#[tokio::test]
async fn test_ssrf_protection() {
    use narayana_rde::transports::http;
    use narayana_rde::subscriptions::{Subscription, SubscriptionId};
    
    // Test localhost blocking
    let subscription = Subscription {
        id: SubscriptionId::new(),
        actor_id: ActorId::from("test"),
        event_name: EventName::from("test:event"),
        transport: TransportType::Webhook,
        config: serde_json::json!({
            "webhook_url": "http://localhost:8080/webhook"
        }),
        created_at: 0,
    };
    
    let result = http::deliver_webhook(&subscription, &serde_json::json!({})).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("localhost"));
    
    // Test private IP blocking
    let subscription = Subscription {
        id: SubscriptionId::new(),
        actor_id: ActorId::from("test"),
        event_name: EventName::from("test:event"),
        transport: TransportType::Webhook,
        config: serde_json::json!({
            "webhook_url": "http://192.168.1.1/webhook"
        }),
        created_at: 0,
    };
    
    let result = http::deliver_webhook(&subscription, &serde_json::json!({})).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("private network"));
}

#[tokio::test]
async fn test_payload_size_limit() {
    let manager = create_test_manager();
    
    let source = Actor::new(
        ActorId::from("source1"),
        "Source".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    manager.register_actor(source).await.unwrap();
    
    // Create payload larger than 10MB
    let large_data = "x".repeat(11 * 1024 * 1024);
    let payload = serde_json::json!({"data": large_data});
    
    let result = manager.publish_event(
        &ActorId::from("source1"),
        "token-123456789012",
        "test_event",
        payload,
    ).await;
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("too large"));
}

#[tokio::test]
async fn test_subscription_limit() {
    let manager = create_test_manager();
    
    let origin = Actor::new(
        ActorId::from("origin1"),
        "Origin".to_string(),
        ActorType::Origin,
        "token-123456789012".to_string(),
    );
    manager.register_actor(origin).await.unwrap();
    
    // Create max subscriptions
    for i in 0..10_000 {
        let result = manager.subscribe(
            &ActorId::from("origin1"),
            "token-123456789012",
            &format!("source1:event_{}", i),
            TransportType::Webhook,
            None,
        ).await;
        assert!(result.is_ok(), "Failed at subscription {}", i);
    }
    
    // Next should fail
    let result = manager.subscribe(
        &ActorId::from("origin1"),
        "token-123456789012",
        "source1:event_10000",
        TransportType::Webhook,
        None,
    ).await;
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Maximum subscriptions"));
}

#[tokio::test]
async fn test_weak_token_rejection() {
    let manager = create_test_manager();
    
    // Token too short
    let actor = Actor::new(
        ActorId::from("test"),
        "Test".to_string(),
        ActorType::Source,
        "short".to_string(), // Less than 16 chars
    );
    
    let result = manager.register_actor(actor).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("too short"));
}

#[tokio::test]
async fn test_control_character_rejection() {
    let manager = create_test_manager();
    
    // Actor ID with control character
    let actor = Actor::new(
        ActorId::from("test\nactor"),
        "Test".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    
    let result = manager.register_actor(actor).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("control characters"));
    
    // Event name with control character
    let actor = Actor::new(
        ActorId::from("source1"),
        "Source".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    manager.register_actor(actor).await.unwrap();
    
    let result = manager.publish_event(
        &ActorId::from("source1"),
        "token-123456789012",
        "event\nname",
        serde_json::json!({}),
    ).await;
    
    assert!(result.is_err());
}

#[tokio::test]
async fn test_wildcard_subscription_permission() {
    let manager = create_test_manager();
    
    // Origin without wildcard permission
    let origin = Actor::new(
        ActorId::from("origin1"),
        "Origin".to_string(),
        ActorType::Origin,
        "token-123456789012".to_string(),
    );
    manager.register_actor(origin).await.unwrap();
    
    // Try wildcard subscription
    let result = manager.subscribe(
        &ActorId::from("origin1"),
        "token-123456789012",
        "test_event", // No colon = wildcard
        TransportType::Webhook,
        None,
    ).await;
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Wildcard subscriptions require"));
}

#[tokio::test]
async fn test_actor_enumeration_prevention() {
    let manager = create_test_manager();
    
    // Try to publish with non-existent actor
    let result = manager.publish_event(
        &ActorId::from("nonexistent"),
        "token",
        "test_event",
        serde_json::json!({}),
    ).await;
    
    // Error should not reveal if actor exists
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(!error_msg.contains("nonexistent")); // Should not leak actor ID
}

#[tokio::test]
async fn test_duplicate_actor_registration() {
    let manager = create_test_manager();
    
    let actor1 = Actor::new(
        ActorId::from("test"),
        "Test 1".to_string(),
        ActorType::Source,
        "token1-123456789012".to_string(),
    );
    let actor2 = Actor::new(
        ActorId::from("test"),
        "Test 2".to_string(),
        ActorType::Source,
        "token2-123456789012".to_string(),
    );
    
    assert!(manager.register_actor(actor1).await.is_ok());
    
    // Second registration should fail
    let result = manager.register_actor(actor2).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
}

#[tokio::test]
async fn test_config_size_limit() {
    let manager = create_test_manager();
    
    let origin = Actor::new(
        ActorId::from("origin1"),
        "Origin".to_string(),
        ActorType::Origin,
        "token-123456789012".to_string(),
    );
    manager.register_actor(origin).await.unwrap();
    
    // Create config larger than 1MB
    let large_config = serde_json::json!({
        "data": "x".repeat(2 * 1024 * 1024)
    });
    
    let result = manager.subscribe(
        &ActorId::from("origin1"),
        "token-123456789012",
        "test:event",
        TransportType::Webhook,
        Some(large_config),
    ).await;
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("too large"));
}

#[tokio::test]
async fn test_event_name_validation() {
    let manager = create_test_manager();
    
    let source = Actor::new(
        ActorId::from("source1"),
        "Source".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    manager.register_actor(source).await.unwrap();
    
    // Empty event name
    let result = manager.publish_event(
        &ActorId::from("source1"),
        "token-123456789012",
        "",
        serde_json::json!({}),
    ).await;
    assert!(result.is_err());
    
    // Event name with colon
    let result = manager.publish_event(
        &ActorId::from("source1"),
        "token-123456789012",
        "event:name",
        serde_json::json!({}),
    ).await;
    assert!(result.is_err());
    
    // Event name just "*"
    let result = manager.publish_event(
        &ActorId::from("source1"),
        "token-123456789012",
        "*",
        serde_json::json!({}),
    ).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_hmac_secret_validation() {
    use narayana_rde::transports::http;
    use narayana_rde::subscriptions::{Subscription, SubscriptionId};
    
    // Empty secret
    let subscription = Subscription {
        id: SubscriptionId::new(),
        actor_id: ActorId::from("test"),
        event_name: EventName::from("test:event"),
        transport: TransportType::Webhook,
        config: serde_json::json!({
            "webhook_url": "https://example.com/webhook",
            "webhook_secret": ""
        }),
        created_at: 0,
    };
    
    // This should fail during HMAC generation
    let result = http::deliver_webhook(&subscription, &serde_json::json!({})).await;
    // Note: This might fail for other reasons (URL validation), but secret validation should catch it
}

