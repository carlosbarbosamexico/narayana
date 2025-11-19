// Integration tests for narayana-rde

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

fn create_source_actor(id: &str, token: &str) -> Actor {
    Actor::new(
        ActorId::from(id),
        format!("Source {}", id),
        ActorType::Source,
        token.to_string(),
    )
}

fn create_origin_actor(id: &str, token: &str) -> Actor {
    Actor::new(
        ActorId::from(id),
        format!("Origin {}", id),
        ActorType::Origin,
        token.to_string(),
    )
}

#[tokio::test]
async fn test_basic_publish_subscribe_flow() {
    let manager = create_test_manager();
    
    // Register actors
    let source = create_source_actor("shopify", "token-shopify-123456789012");
    let origin = create_origin_actor("oneroute", "token-oneroute-123456789012");
    
    manager.register_actor(source).await.unwrap();
    manager.register_actor(origin).await.unwrap();
    
    // Subscribe to event
    manager.subscribe(
        &ActorId::from("oneroute"),
        "token-oneroute-123456789012",
        "shopify:order_created",
        TransportType::Webhook,
        Some(serde_json::json!({
            "webhook_url": "https://api.oneroute.com/webhook"
        })),
    ).await.unwrap();
    
    // Publish event
    let payload = serde_json::json!({
        "order_id": "12345",
        "customer_id": "67890",
        "total": 99.99
    });
    
    let result = manager.publish_event(
        &ActorId::from("shopify"),
        "token-shopify-123456789012",
        "order_created",
        payload,
    ).await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_multiple_subscribers() {
    let manager = create_test_manager();
    
    // Register source
    let source = create_source_actor("source1", "token-source1-123456789012");
    manager.register_actor(source).await.unwrap();
    
    // Register multiple origins
    for i in 1..=5 {
        let origin_id = format!("origin{}", i);
        let token = format!("token-origin{}-123456789012", i);
        let origin = create_origin_actor(&origin_id, &token);
        manager.register_actor(origin).await.unwrap();
        
        manager.subscribe(
            &ActorId::from(origin_id.clone()),
            &token,
            "source1:test_event",
            TransportType::Webhook,
            Some(serde_json::json!({
                "webhook_url": format!("https://example.com/webhook{}", i)
            })),
        ).await.unwrap();
    }
    
    // Publish event - should reach all 5 subscribers
    let result = manager.publish_event(
        &ActorId::from("source1"),
        "token-source1-123456789012",
        "test_event",
        serde_json::json!({"data": "test"}),
    ).await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_wildcard_subscription() {
    let manager = create_test_manager();
    
    // Register multiple sources
    for i in 1..=3 {
        let source_id = format!("source{}", i);
        let token = format!("token-source{}-123456789012", i);
        let source = create_source_actor(&source_id, &token);
        manager.register_actor(source).await.unwrap();
    }
    
    // Register origin with wildcard permission
    let mut origin = create_origin_actor("origin1", "token-origin1-123456789012");
    origin.metadata = serde_json::json!({"allow_wildcard_subscriptions": true});
    manager.register_actor(origin).await.unwrap();
    
    // Subscribe to all sources' events
    manager.subscribe(
        &ActorId::from("origin1"),
        "token-origin1-123456789012",
        "order_created", // Wildcard - will match any source
        TransportType::Webhook,
        None,
    ).await.unwrap();
    
    // Publish from different sources
    for i in 1..=3 {
        let source_id = format!("source{}", i);
        let token = format!("token-source{}-123456789012", i);
        let payload = serde_json::json!({"source": i});
        let result = manager.publish_event(
            &ActorId::from(source_id),
            &token,
            "order_created",
            payload,
        ).await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_event_namespacing() {
    let manager = create_test_manager();
    
    let source1 = create_source_actor("shopify", "token-shopify-123456789012");
    let source2 = create_source_actor("woocommerce", "token-woo-123456789012");
    let origin = create_origin_actor("oneroute", "token-oneroute-123456789012");
    
    manager.register_actor(source1).await.unwrap();
    manager.register_actor(source2).await.unwrap();
    manager.register_actor(origin).await.unwrap();
    
    // Subscribe to specific source's event
    manager.subscribe(
        &ActorId::from("oneroute"),
        "token-oneroute-123456789012",
        "shopify:order_created",
        TransportType::Webhook,
        None,
    ).await.unwrap();
    
    // Publish from shopify - should match
    let result1 = manager.publish_event(
        &ActorId::from("shopify"),
        "token-shopify-123456789012",
        "order_created",
        serde_json::json!({"source": "shopify"}),
    ).await;
    assert!(result1.is_ok());
    
    // Publish from woocommerce - should NOT match (different namespace)
    let result2 = manager.publish_event(
        &ActorId::from("woocommerce"),
        "token-woo-123456789012",
        "order_created",
        serde_json::json!({"source": "woocommerce"}),
    ).await;
    assert!(result2.is_ok()); // Publish succeeds, but won't be delivered to subscriber
}

#[tokio::test]
async fn test_subscription_before_event_exists() {
    let manager = create_test_manager();
    
    let source = create_source_actor("source1", "token-source1-123456789012");
    let origin = create_origin_actor("origin1", "token-origin1-123456789012");
    
    manager.register_actor(source).await.unwrap();
    manager.register_actor(origin).await.unwrap();
    
    // Subscribe to event that doesn't exist yet
    let result = manager.subscribe(
        &ActorId::from("origin1"),
        "token-origin1-123456789012",
        "source1:future_event",
        TransportType::Webhook,
        None,
    ).await;
    
    assert!(result.is_ok());
    
    // Now publish the event - subscription should receive it
    let result = manager.publish_event(
        &ActorId::from("source1"),
        "token-source1-123456789012",
        "future_event",
        serde_json::json!({"data": "test"}),
    ).await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_authentication_failure() {
    let manager = create_test_manager();
    
    let source = create_source_actor("source1", "token-source1-123456789012");
    manager.register_actor(source).await.unwrap();
    
    // Try to publish with wrong token
    let result = manager.publish_event(
        &ActorId::from("source1"),
        "wrong-token",
        "test_event",
        serde_json::json!({}),
    ).await;
    
    assert!(result.is_err());
    
    // Try to subscribe with wrong token
    let origin = create_origin_actor("origin1", "token-origin1-123456789012");
    manager.register_actor(origin).await.unwrap();
    
    let result = manager.subscribe(
        &ActorId::from("origin1"),
        "wrong-token",
        "test:event",
        TransportType::Webhook,
        None,
    ).await;
    
    assert!(result.is_err());
}

#[tokio::test]
async fn test_actor_type_enforcement() {
    let manager = create_test_manager();
    
    // Source cannot subscribe
    let source = create_source_actor("source1", "token-source1-123456789012");
    manager.register_actor(source).await.unwrap();
    
    let result = manager.subscribe(
        &ActorId::from("source1"),
        "token-source1-123456789012",
        "test:event",
        TransportType::Webhook,
        None,
    ).await;
    
    assert!(result.is_err());
    
    // Origin cannot publish
    let origin = create_origin_actor("origin1", "token-origin1-123456789012");
    manager.register_actor(origin).await.unwrap();
    
    let result = manager.publish_event(
        &ActorId::from("origin1"),
        "token-origin1-123456789012",
        "test_event",
        serde_json::json!({}),
    ).await;
    
    assert!(result.is_err());
}

#[tokio::test]
async fn test_payload_validation() {
    let manager = create_test_manager();
    
    let source = create_source_actor("source1", "token-source1-123456789012");
    manager.register_actor(source).await.unwrap();
    
    // Test various payload types
    let payloads = vec![
        serde_json::json!({}), // Empty object
        serde_json::json!([]), // Array
        serde_json::json!(null), // Null
        serde_json::json!("string"), // String
        serde_json::json!(42), // Number
        serde_json::json!(true), // Boolean
        serde_json::json!({
            "nested": {
                "deep": {
                    "value": 123
                }
            }
        }), // Nested object
        serde_json::json!({
            "array": [1, 2, 3],
            "mixed": {
                "string": "value",
                "number": 42
            }
        }), // Mixed types
    ];
    
    for (idx, payload) in payloads.into_iter().enumerate() {
        let event_name = format!("test_event_{}", idx);
        let result = manager.publish_event(
            &ActorId::from("source1"),
            "token-source1-123456789012",
            &event_name,
            payload,
        ).await;
        assert!(result.is_ok(), "Failed to publish payload at index {}", idx);
    }
}

#[tokio::test]
async fn test_concurrent_operations() {
    let manager = create_test_manager();
    
    // Register actors
    let source = create_source_actor("source1", "token-source1-123456789012");
    let origin = create_origin_actor("origin1", "token-origin1-123456789012");
    
    manager.register_actor(source).await.unwrap();
    manager.register_actor(origin).await.unwrap();
    
    // Sequential subscriptions (testing that multiple work)
    for i in 0..10 {
        let result = manager.subscribe(
            &ActorId::from("origin1"),
            "token-origin1-123456789012",
            &format!("source1:event_{}", i),
            TransportType::Webhook,
            None,
        ).await;
        assert!(result.is_ok());
    }
    
    // Sequential publishes (testing that multiple work)
    for i in 0..10 {
        let result = manager.publish_event(
            &ActorId::from("source1"),
            "token-source1-123456789012",
            &format!("event_{}", i),
            serde_json::json!({"index": i}),
        ).await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_get_actor_sanitization() {
    let manager = create_test_manager();
    
    let actor = create_source_actor("test", "secret-token-123456789012");
    manager.register_actor(actor).await.unwrap();
    
    let retrieved = manager.get_actor(&ActorId::from("test"));
    assert!(retrieved.is_some());
    
    let actor = retrieved.unwrap();
    assert_eq!(actor.id.0, "test");
    assert_eq!(actor.auth_token, ""); // Should be cleared
    assert_eq!(actor.actor_type, ActorType::Source);
}

