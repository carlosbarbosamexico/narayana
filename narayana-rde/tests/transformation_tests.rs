// Transformation tests for narayana-rde

use narayana_rde::*;
use narayana_rde::subscriptions::{Subscription, SubscriptionId};
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
async fn test_transformation_field_mapping() {
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
    
    // Subscribe with transformation config
    let output_config = serde_json::json!({
        "transforms": [
            {
                "type": "field",
                "source": "order_id",
                "target": "id"
            },
            {
                "type": "field",
                "source": "customer_name",
                "target": "customer"
            }
        ]
    });
    
    manager.subscribe(
        &ActorId::from("origin1"),
        "token-123456789012",
        "source1:order_created",
        TransportType::Webhook,
        Some(serde_json::json!({
            "webhook_url": "https://example.com/webhook",
            "output_config": output_config
        })),
    ).await.unwrap();
    
    // Publish event with original field names
    let payload = serde_json::json!({
        "order_id": "12345",
        "customer_name": "John Doe",
        "total": 99.99
    });
    
    let result = manager.publish_event(
        &ActorId::from("source1"),
        "token-123456789012",
        "order_created",
        payload,
    ).await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_transformation_without_config() {
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
    
    // Subscribe without transformation
    manager.subscribe(
        &ActorId::from("origin1"),
        "token-123456789012",
        "source1:order_created",
        TransportType::Webhook,
        Some(serde_json::json!({
            "webhook_url": "https://example.com/webhook"
        })),
    ).await.unwrap();
    
    // Publish event
    let payload = serde_json::json!({
        "order_id": "12345",
        "customer": "John Doe"
    });
    
    let result = manager.publish_event(
        &ActorId::from("source1"),
        "token-123456789012",
        "order_created",
        payload,
    ).await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_transformation_invalid_config() {
    use narayana_rde::transformations;
    use narayana_rde::subscriptions::Subscription;
    
    let subscription = Subscription {
        id: SubscriptionId::new(),
        actor_id: ActorId::from("test"),
        event_name: EventName::from("test:event"),
        transport: TransportType::Webhook,
        config: serde_json::json!({
            "output_config": "not an object" // Invalid - should be object
        }),
        created_at: 0,
    };
    
    let payload = serde_json::json!({"data": "test"});
    let result = transformations::apply_transformation(&subscription, &payload);
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("must be an object"));
}

#[tokio::test]
async fn test_transformation_large_config() {
    use narayana_rde::transformations;
    use narayana_rde::subscriptions::Subscription;
    
    // Create config larger than 100KB
    let large_data = "x".repeat(101 * 1024);
    let subscription = Subscription {
        id: SubscriptionId::new(),
        actor_id: ActorId::from("test"),
        event_name: EventName::from("test:event"),
        transport: TransportType::Webhook,
        config: serde_json::json!({
            "output_config": {
                "data": large_data
            }
        }),
        created_at: 0,
    };
    
    let payload = serde_json::json!({"data": "test"});
    let result = transformations::apply_transformation(&subscription, &payload);
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("too large"));
}

#[tokio::test]
async fn test_transformation_fallback_on_error() {
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
    
    // Subscribe with invalid transformation (will fail but should use original)
    let output_config = serde_json::json!({
        "transforms": [
            {
                "type": "field",
                "source": "nonexistent_field",
                "target": "new_field"
            }
        ]
    });
    
    manager.subscribe(
        &ActorId::from("origin1"),
        "token-123456789012",
        "source1:order_created",
        TransportType::Webhook,
        Some(serde_json::json!({
            "webhook_url": "https://example.com/webhook",
            "output_config": output_config
        })),
    ).await.unwrap();
    
    // Publish event - transformation should fail but delivery should continue
    let payload = serde_json::json!({
        "order_id": "12345",
        "customer": "John Doe"
    });
    
    let result = manager.publish_event(
        &ActorId::from("source1"),
        "token-123456789012",
        "order_created",
        payload,
    ).await;
    
    // Should succeed even if transformation fails (falls back to original)
    assert!(result.is_ok());
}

