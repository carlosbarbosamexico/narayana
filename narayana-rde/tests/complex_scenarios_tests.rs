// Complex scenario tests for narayana-rde

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
async fn test_multi_actor_multi_event_workflow() {
    let manager = create_test_manager();
    
    // Register multiple sources
    for i in 1..=5 {
        let source = Actor::new(
            ActorId::from(format!("source{}", i)),
            format!("Source {}", i),
            ActorType::Source,
            format!("token-source{}-123456789012", i),
        );
        manager.register_actor(source).await.unwrap();
    }
    
    // Register multiple origins
    for i in 1..=3 {
        let origin = Actor::new(
            ActorId::from(format!("origin{}", i)),
            format!("Origin {}", i),
            ActorType::Origin,
            format!("token-origin{}-123456789012", i),
        );
        manager.register_actor(origin).await.unwrap();
    }
    
    // Each origin subscribes to different events from different sources
    manager.subscribe(
        &ActorId::from("origin1"),
        "token-origin1-123456789012",
        "source1:order_created",
        TransportType::Webhook,
        None,
    ).await.unwrap();
    
    manager.subscribe(
        &ActorId::from("origin2"),
        "token-origin2-123456789012",
        "source2:payment_received",
        TransportType::Webhook,
        None,
    ).await.unwrap();
    
    manager.subscribe(
        &ActorId::from("origin3"),
        "token-origin3-123456789012",
        "source3:shipment_created",
        TransportType::Webhook,
        None,
    ).await.unwrap();
    
    // Publish events from different sources
    manager.publish_event(
        &ActorId::from("source1"),
        "token-source1-123456789012",
        "order_created",
        serde_json::json!({"order_id": "123"}),
    ).await.unwrap();
    
    manager.publish_event(
        &ActorId::from("source2"),
        "token-source2-123456789012",
        "payment_received",
        serde_json::json!({"payment_id": "456"}),
    ).await.unwrap();
    
    manager.publish_event(
        &ActorId::from("source3"),
        "token-source3-123456789012",
        "shipment_created",
        serde_json::json!({"shipment_id": "789"}),
    ).await.unwrap();
}

#[tokio::test]
async fn test_event_chain_workflow() {
    let manager = create_test_manager();
    
    // Source publishes order_created
    let source = Actor::new(
        ActorId::from("shopify"),
        "Shopify".to_string(),
        ActorType::Source,
        "token-shopify-123456789012".to_string(),
    );
    manager.register_actor(source).await.unwrap();
    
    // Origin subscribes and processes, then publishes new event
    let mut origin1 = Actor::new(
        ActorId::from("processor"),
        "Processor".to_string(),
        ActorType::Origin,
        "token-processor-123456789012".to_string(),
    );
    origin1.metadata = serde_json::json!({"allow_wildcard_subscriptions": true});
    manager.register_actor(origin1).await.unwrap();
    
    // Another origin subscribes to processed events
    let origin2 = Actor::new(
        ActorId::from("warehouse"),
        "Warehouse".to_string(),
        ActorType::Origin,
        "token-warehouse-123456789012".to_string(),
    );
    manager.register_actor(origin2).await.unwrap();
    
    // Warehouse subscribes to processed orders
    manager.subscribe(
        &ActorId::from("warehouse"),
        "token-warehouse-123456789012",
        "processor:order_processed",
        TransportType::Webhook,
        None,
    ).await.unwrap();
    
    // Processor subscribes to all order_created events
    manager.subscribe(
        &ActorId::from("processor"),
        "token-processor-123456789012",
        "order_created",
        TransportType::Webhook,
        None,
    ).await.unwrap();
    
    // Publish initial event
    manager.publish_event(
        &ActorId::from("shopify"),
        "token-shopify-123456789012",
        "order_created",
        serde_json::json!({"order_id": "12345"}),
    ).await.unwrap();
}

#[tokio::test]
async fn test_subscription_to_multiple_events() {
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
    
    // Subscribe to multiple different events
    let events = vec!["order_created", "order_updated", "order_cancelled", "order_shipped"];
    
    for event in &events {
        manager.subscribe(
            &ActorId::from("origin1"),
            "token-123456789012",
            &format!("source1:{}", event),
            TransportType::Webhook,
            None,
        ).await.unwrap();
    }
    
    // Publish all events
    for event in &events {
        manager.publish_event(
            &ActorId::from("source1"),
            "token-123456789012",
            event,
            serde_json::json!({"event": event}),
        ).await.unwrap();
    }
}

#[tokio::test]
async fn test_wildcard_subscription_multiple_sources() {
    let manager = create_test_manager();
    
    // Register multiple sources
    for i in 1..=5 {
        let source = Actor::new(
            ActorId::from(format!("shop{}", i)),
            format!("Shop {}", i),
            ActorType::Source,
            format!("token-shop{}-123456789012", i),
        );
        manager.register_actor(source).await.unwrap();
    }
    
    // Register origin with wildcard permission
    let mut origin = Actor::new(
        ActorId::from("aggregator"),
        "Aggregator".to_string(),
        ActorType::Origin,
        "token-aggregator-123456789012".to_string(),
    );
    origin.metadata = serde_json::json!({"allow_wildcard_subscriptions": true});
    manager.register_actor(origin).await.unwrap();
    
    // Subscribe to all shops' order_created events
    manager.subscribe(
        &ActorId::from("aggregator"),
        "token-aggregator-123456789012",
        "order_created", // Wildcard - matches any shop
        TransportType::Webhook,
        None,
    ).await.unwrap();
    
    // Publish from all shops
    for i in 1..=5 {
        manager.publish_event(
            &ActorId::from(format!("shop{}", i)),
            &format!("token-shop{}-123456789012", i),
            "order_created",
            serde_json::json!({"shop_id": i, "order_id": format!("order_{}", i)}),
        ).await.unwrap();
    }
}

#[tokio::test]
async fn test_event_with_complex_nested_payload() {
    let manager = create_test_manager();
    
    let source = Actor::new(
        ActorId::from("source1"),
        "Source".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    manager.register_actor(source).await.unwrap();
    
    // Complex nested payload
    let payload = serde_json::json!({
        "order": {
            "id": "12345",
            "customer": {
                "id": "67890",
                "name": "John Doe",
                "address": {
                    "street": "123 Main St",
                    "city": "New York",
                    "zip": "10001",
                    "country": "USA"
                },
                "preferences": {
                    "language": "en",
                    "currency": "USD"
                }
            },
            "items": [
                {
                    "product_id": "prod1",
                    "quantity": 2,
                    "price": 29.99
                },
                {
                    "product_id": "prod2",
                    "quantity": 1,
                    "price": 49.99
                }
            ],
            "totals": {
                "subtotal": 109.97,
                "tax": 8.80,
                "shipping": 5.99,
                "total": 124.76
            },
            "metadata": {
                "source": "web",
                "campaign": "summer_sale",
                "tags": ["urgent", "vip"]
            }
        },
        "timestamp": "2024-01-15T10:30:00Z",
        "version": "1.0"
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
async fn test_rapid_event_publishing() {
    let manager = create_test_manager();
    
    let source = Actor::new(
        ActorId::from("source1"),
        "Source".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    manager.register_actor(source).await.unwrap();
    
    // Publish many events rapidly
    for i in 0..100 {
        let result = manager.publish_event(
            &ActorId::from("source1"),
            "token-123456789012",
            "test_event",
            serde_json::json!({"index": i, "data": format!("event_{}", i)}),
        ).await;
        assert!(result.is_ok(), "Failed at event {}", i);
    }
}

#[tokio::test]
async fn test_subscription_after_multiple_events() {
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
    
    // Publish multiple events first
    for i in 0..10 {
        manager.publish_event(
            &ActorId::from("source1"),
            "token-123456789012",
            "test_event",
            serde_json::json!({"index": i}),
        ).await.unwrap();
    }
    
    // Then subscribe (should work even though events were published before)
    let result = manager.subscribe(
        &ActorId::from("origin1"),
        "token-123456789012",
        "source1:test_event",
        TransportType::Webhook,
        None,
    ).await;
    
    assert!(result.is_ok());
    
    // Publish new event - subscription should receive it
    manager.publish_event(
        &ActorId::from("source1"),
        "token-123456789012",
        "test_event",
        serde_json::json!({"index": 11}),
    ).await.unwrap();
}

