// Comprehensive WebSocket Tests
// Tests for WebSocket connection management, subscriptions, security, and edge cases

use narayana_api::websocket::{ConnectionId, Channel, WsMessage, EventFilter};
use narayana_server::websocket_manager::{WebSocketManager, WebSocketConfig, ConnectionState};
use std::sync::Arc;
use tokio::sync::mpsc;
use std::time::Duration;
use tokio::time::sleep;

// ============================================================================
// Connection Management Tests
// ============================================================================

#[tokio::test]
async fn test_connection_registration() {
    let config = WebSocketConfig::default();
    let manager = WebSocketManager::new(config);
    
    let connection_id = "test-conn-1".to_string();
    let (tx, _rx) = mpsc::unbounded_channel();
    
    let result = manager.register_connection(
        connection_id.clone(),
        Some("user-1".to_string()),
        tx,
    );
    
    assert!(result.is_ok());
    assert_eq!(manager.connection_count(), 1);
    
    let state = manager.get_connection_state(&connection_id);
    assert!(state.is_some());
    assert_eq!(state.unwrap().user_id, Some("user-1".to_string()));
}

#[tokio::test]
async fn test_connection_unregistration() {
    let config = WebSocketConfig::default();
    let manager = WebSocketManager::new(config);
    
    let connection_id = "test-conn-1".to_string();
    let (tx, _rx) = mpsc::unbounded_channel();
    
    manager.register_connection(connection_id.clone(), None, tx).unwrap();
    assert_eq!(manager.connection_count(), 1);
    
    manager.unregister_connection(&connection_id);
    assert_eq!(manager.connection_count(), 0);
    
    let state = manager.get_connection_state(&connection_id);
    assert!(state.is_none());
}

#[tokio::test]
async fn test_connection_limit() {
    let mut config = WebSocketConfig::default();
    config.max_connections = 2;
    let manager = WebSocketManager::new(config);
    
    let (tx1, _rx1) = mpsc::unbounded_channel();
    let (tx2, _rx2) = mpsc::unbounded_channel();
    let (tx3, _rx3) = mpsc::unbounded_channel();
    
    assert!(manager.register_connection("conn-1".to_string(), None, tx1).is_ok());
    assert!(manager.register_connection("conn-2".to_string(), None, tx2).is_ok());
    
    // Third connection should fail
    let result = manager.register_connection("conn-3".to_string(), None, tx3);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Maximum connections"));
}

#[tokio::test]
async fn test_connection_activity_update() {
    let config = WebSocketConfig::default();
    let manager = WebSocketManager::new(config);
    
    let connection_id = "test-conn-1".to_string();
    let (tx, _rx) = mpsc::unbounded_channel();
    
    manager.register_connection(connection_id.clone(), None, tx).unwrap();
    
    let initial_state = manager.get_connection_state(&connection_id).unwrap();
    let initial_activity = initial_state.last_activity;
    
    sleep(Duration::from_millis(10)).await;
    manager.update_activity(&connection_id);
    
    let updated_state = manager.get_connection_state(&connection_id).unwrap();
    assert!(updated_state.last_activity > initial_activity);
}

// ============================================================================
// Subscription Tests
// ============================================================================

#[tokio::test]
async fn test_subscribe_to_channel() {
    let config = WebSocketConfig::default();
    let manager = WebSocketManager::new(config);
    
    let connection_id = "test-conn-1".to_string();
    let (tx, _rx) = mpsc::unbounded_channel();
    
    manager.register_connection(connection_id.clone(), Some("user-1".to_string()), tx).unwrap();
    
    let channel = "brain:thoughts".to_string();
    let result = manager.subscribe(&connection_id, channel.clone(), None);
    
    assert!(result.is_ok());
    
    let channels = manager.get_connection_channels(&connection_id, &connection_id).unwrap();
    assert!(channels.contains(&channel));
}

#[tokio::test]
async fn test_subscribe_duplicate() {
    let config = WebSocketConfig::default();
    let manager = WebSocketManager::new(config);
    
    let connection_id = "test-conn-1".to_string();
    let (tx, _rx) = mpsc::unbounded_channel();
    
    manager.register_connection(connection_id.clone(), Some("user-1".to_string()), tx).unwrap();
    
    let channel = "brain:thoughts".to_string();
    
    // First subscription
    assert!(manager.subscribe(&connection_id, channel.clone(), None).is_ok());
    
    // Duplicate subscription (should be idempotent)
    assert!(manager.subscribe(&connection_id, channel.clone(), None).is_ok());
    
    let channels = manager.get_connection_channels(&connection_id, &connection_id).unwrap();
    assert_eq!(channels.len(), 1); // Still only one subscription
}

#[tokio::test]
async fn test_subscribe_limit() {
    let mut config = WebSocketConfig::default();
    config.max_subscriptions_per_connection = 2;
    let manager = WebSocketManager::new(config);
    
    let connection_id = "test-conn-1".to_string();
    let (tx, _rx) = mpsc::unbounded_channel();
    
    manager.register_connection(connection_id.clone(), Some("user-1".to_string()), tx).unwrap();
    
    assert!(manager.subscribe(&connection_id, "channel-1".to_string(), None).is_ok());
    assert!(manager.subscribe(&connection_id, "channel-2".to_string(), None).is_ok());
    
    // Third subscription should fail
    let result = manager.subscribe(&connection_id, "channel-3".to_string(), None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Maximum subscriptions"));
}

#[tokio::test]
async fn test_unsubscribe_from_channel() {
    let config = WebSocketConfig::default();
    let manager = WebSocketManager::new(config);
    
    let connection_id = "test-conn-1".to_string();
    let (tx, _rx) = mpsc::unbounded_channel();
    
    manager.register_connection(connection_id.clone(), Some("user-1".to_string()), tx).unwrap();
    
    let channel = "brain:thoughts".to_string();
    manager.subscribe(&connection_id, channel.clone(), None).unwrap();
    
    let result = manager.unsubscribe(&connection_id, &channel);
    assert!(result.is_ok());
    
    let channels = manager.get_connection_channels(&connection_id, &connection_id).unwrap();
    assert!(!channels.contains(&channel));
}

#[tokio::test]
async fn test_unsubscribe_nonexistent() {
    let config = WebSocketConfig::default();
    let manager = WebSocketManager::new(config);
    
    let connection_id = "test-conn-1".to_string();
    let (tx, _rx) = mpsc::unbounded_channel();
    
    manager.register_connection(connection_id.clone(), None, tx).unwrap();
    
    // Unsubscribe from channel we never subscribed to (should be idempotent)
    let result = manager.unsubscribe(&connection_id, &"nonexistent".to_string());
    assert!(result.is_ok());
}

// ============================================================================
// Authorization Tests
// ============================================================================

#[tokio::test]
async fn test_subscribe_public_channel_unauthorized() {
    let config = WebSocketConfig::default();
    let manager = WebSocketManager::new(config);
    
    let connection_id = "test-conn-1".to_string();
    let (tx, _rx) = mpsc::unbounded_channel();
    
    // Register without user_id (unauthenticated)
    manager.register_connection(connection_id.clone(), None, tx).unwrap();
    
    // Public channels should be accessible
    assert!(manager.subscribe(&connection_id, "brain:thoughts".to_string(), None).is_ok());
}

#[tokio::test]
async fn test_subscribe_database_channel_requires_auth() {
    let config = WebSocketConfig::default();
    let manager = WebSocketManager::new(config);
    
    let connection_id = "test-conn-1".to_string();
    let (tx, _rx) = mpsc::unbounded_channel();
    
    // Register without user_id (unauthenticated)
    manager.register_connection(connection_id.clone(), None, tx).unwrap();
    
    // Database channels require authentication
    let result = manager.subscribe(&connection_id, "db:test:events".to_string(), None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unauthorized"));
}

#[tokio::test]
async fn test_subscribe_database_channel_authenticated() {
    let config = WebSocketConfig::default();
    let manager = WebSocketManager::new(config);
    
    let connection_id = "test-conn-1".to_string();
    let (tx, _rx) = mpsc::unbounded_channel();
    
    // Register with user_id (authenticated)
    manager.register_connection(connection_id.clone(), Some("user-1".to_string()), tx).unwrap();
    
    // Database channels should be accessible when authenticated
    assert!(manager.subscribe(&connection_id, "db:test:events".to_string(), None).is_ok());
}

#[tokio::test]
async fn test_subscribe_unknown_channel_denied() {
    let config = WebSocketConfig::default();
    let manager = WebSocketManager::new(config);
    
    let connection_id = "test-conn-1".to_string();
    let (tx, _rx) = mpsc::unbounded_channel();
    
    manager.register_connection(connection_id.clone(), Some("user-1".to_string()), tx).unwrap();
    
    // Unknown channel types should be denied
    let result = manager.subscribe(&connection_id, "unknown:channel".to_string(), None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unauthorized"));
}

// ============================================================================
// Channel Validation Tests
// ============================================================================

#[tokio::test]
async fn test_subscribe_empty_channel() {
    let config = WebSocketConfig::default();
    let manager = WebSocketManager::new(config);
    
    let connection_id = "test-conn-1".to_string();
    let (tx, _rx) = mpsc::unbounded_channel();
    
    manager.register_connection(connection_id.clone(), Some("user-1".to_string()), tx).unwrap();
    
    let result = manager.subscribe(&connection_id, "".to_string(), None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("empty"));
}

#[tokio::test]
async fn test_subscribe_too_long_channel() {
    let config = WebSocketConfig::default();
    let manager = WebSocketManager::new(config);
    
    let connection_id = "test-conn-1".to_string();
    let (tx, _rx) = mpsc::unbounded_channel();
    
    manager.register_connection(connection_id.clone(), Some("user-1".to_string()), tx).unwrap();
    
    let long_channel = "a".repeat(257); // Exceeds 256 char limit
    let result = manager.subscribe(&connection_id, long_channel, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("too long"));
}

#[tokio::test]
async fn test_subscribe_invalid_characters() {
    let config = WebSocketConfig::default();
    let manager = WebSocketManager::new(config);
    
    let connection_id = "test-conn-1".to_string();
    let (tx, _rx) = mpsc::unbounded_channel();
    
    manager.register_connection(connection_id.clone(), Some("user-1".to_string()), tx).unwrap();
    
    // Test null byte
    let result = manager.subscribe(&connection_id, "channel\0name".to_string(), None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid characters"));
    
    // Test newline
    let result = manager.subscribe(&connection_id, "channel\nname".to_string(), None);
    assert!(result.is_err());
    
    // Test carriage return
    let result = manager.subscribe(&connection_id, "channel\rname".to_string(), None);
    assert!(result.is_err());
}

// ============================================================================
// Message Broadcasting Tests
// ============================================================================

#[tokio::test]
async fn test_broadcast_to_channel() {
    let config = WebSocketConfig::default();
    let manager = WebSocketManager::new(config);
    
    let connection_id1 = "conn-1".to_string();
    let connection_id2 = "conn-2".to_string();
    let (tx1, mut rx1) = mpsc::unbounded_channel();
    let (tx2, mut rx2) = mpsc::unbounded_channel();
    
    manager.register_connection(connection_id1.clone(), Some("user-1".to_string()), tx1).unwrap();
    manager.register_connection(connection_id2.clone(), Some("user-2".to_string()), tx2).unwrap();
    
    let channel = "brain:thoughts".to_string();
    manager.subscribe(&connection_id1, channel.clone(), None).unwrap();
    manager.subscribe(&connection_id2, channel.clone(), None).unwrap();
    
    let message = WsMessage::event(channel.clone(), serde_json::json!({"type": "test"}));
    let count = manager.broadcast_to_channel(&channel, message);
    
    assert_eq!(count, 2);
    
    // Check messages were received
    tokio::select! {
        _ = rx1.recv() => {},
        _ = tokio::time::sleep(Duration::from_millis(100)) => panic!("Message not received on conn-1"),
    }
    
    tokio::select! {
        _ = rx2.recv() => {},
        _ = tokio::time::sleep(Duration::from_millis(100)) => panic!("Message not received on conn-2"),
    }
}

#[tokio::test]
async fn test_broadcast_to_empty_channel() {
    let config = WebSocketConfig::default();
    let manager = WebSocketManager::new(config);
    
    let channel = "empty:channel".to_string();
    let message = WsMessage::event(channel.clone(), serde_json::json!({"type": "test"}));
    
    let count = manager.broadcast_to_channel(&channel, message);
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_send_to_connection() {
    let config = WebSocketConfig::default();
    let manager = WebSocketManager::new(config);
    
    let connection_id = "test-conn-1".to_string();
    let (tx, mut rx) = mpsc::unbounded_channel();
    
    manager.register_connection(connection_id.clone(), None, tx).unwrap();
    
    let message = WsMessage::Ping { id: Some("test-id".to_string()) };
    let result = manager.send_to_connection(&connection_id, message);
    
    assert!(result);
    
    // Check message was received
    tokio::select! {
        _ = rx.recv() => {},
        _ = tokio::time::sleep(Duration::from_millis(100)) => panic!("Message not received"),
    }
}

#[tokio::test]
async fn test_send_to_nonexistent_connection() {
    let config = WebSocketConfig::default();
    let manager = WebSocketManager::new(config);
    
    let message = WsMessage::Ping { id: None };
    let result = manager.send_to_connection(&"nonexistent".to_string(), message);
    
    assert!(!result);
}

// ============================================================================
// Message Serialization Tests
// ============================================================================

#[test]
fn test_message_serialization() {
    let message = WsMessage::Subscribe {
        channel: "test:channel".to_string(),
        filter: None,
    };
    
    let json = message.to_json().unwrap();
    assert!(json.contains("subscribe"));
    assert!(json.contains("test:channel"));
    
    let parsed = WsMessage::from_json(&json).unwrap();
    match parsed {
        WsMessage::Subscribe { channel, .. } => {
            assert_eq!(channel, "test:channel");
        }
        _ => panic!("Wrong message type"),
    }
}

#[test]
fn test_message_error_creation() {
    let error = WsMessage::error("TEST_ERROR", "Test error message");
    
    match error {
        WsMessage::Error { code, message, .. } => {
            assert_eq!(code, "TEST_ERROR");
            assert_eq!(message, "Test error message");
        }
        _ => panic!("Wrong message type"),
    }
}

#[test]
fn test_message_event_creation() {
    let event = WsMessage::event("test:channel", serde_json::json!({"data": "test"}));
    
    match event {
        WsMessage::Event { channel, event, .. } => {
            assert_eq!(channel, "test:channel");
            assert_eq!(event["data"], "test");
        }
        _ => panic!("Wrong message type"),
    }
}

#[test]
fn test_message_event_with_timestamp() {
    let timestamp = 1234567890;
    let event = WsMessage::event_with_timestamp(
        "test:channel",
        serde_json::json!({"data": "test"}),
        timestamp,
    );
    
    match event {
        WsMessage::Event { channel, timestamp: Some(ts), .. } => {
            assert_eq!(channel, "test:channel");
            assert_eq!(ts, timestamp);
        }
        _ => panic!("Wrong message type or missing timestamp"),
    }
}

// ============================================================================
// Security Tests
// ============================================================================

#[tokio::test]
async fn test_channel_name_injection_prevention() {
    let config = WebSocketConfig::default();
    let manager = WebSocketManager::new(config);
    
    let connection_id = "test-conn-1".to_string();
    let (tx, _rx) = mpsc::unbounded_channel();
    
    manager.register_connection(connection_id.clone(), Some("user-1".to_string()), tx).unwrap();
    
    // Test various injection attempts
    let malicious_channels = vec![
        "db:test:events\nDROP TABLE",
        "db:test:events; DELETE",
        "db:test:events\0injection",
        "db:test:events\r\ninjection",
    ];
    
    for channel in malicious_channels {
        let result = manager.subscribe(&connection_id, channel.to_string(), None);
        assert!(result.is_err(), "Should reject malicious channel: {}", channel);
    }
}

#[tokio::test]
async fn test_broadcast_subscriber_limit() {
    let config = WebSocketConfig::default();
    let manager = WebSocketManager::new(config);
    
    let channel = "test:channel".to_string();
    
    // Create many connections and subscribe them
    for i in 0..15_000 {
        let connection_id = format!("conn-{}", i);
        let (tx, _rx) = mpsc::unbounded_channel();
        manager.register_connection(connection_id.clone(), Some("user-1".to_string()), tx).unwrap();
        manager.subscribe(&connection_id, channel.clone(), None).unwrap();
    }
    
    let message = WsMessage::event(channel.clone(), serde_json::json!({"type": "test"}));
    let count = manager.broadcast_to_channel(&channel, message);
    
    // Should be limited to MAX_BROADCAST_SUBSCRIBERS (10,000)
    assert!(count <= 10_000);
}

// ============================================================================
// Cleanup Tests
// ============================================================================

#[tokio::test]
async fn test_cleanup_stale_connections() {
    let mut config = WebSocketConfig::default();
    config.connection_timeout_secs = 1; // 1 second timeout
    let manager = WebSocketManager::new(config);
    
    let connection_id = "test-conn-1".to_string();
    let (tx, _rx) = mpsc::unbounded_channel();
    
    manager.register_connection(connection_id.clone(), None, tx).unwrap();
    assert_eq!(manager.connection_count(), 1);
    
    // Wait for timeout
    sleep(Duration::from_secs(2)).await;
    
    let cleaned = manager.cleanup_stale_connections();
    assert_eq!(cleaned, 1);
    assert_eq!(manager.connection_count(), 0);
}

#[tokio::test]
async fn test_cleanup_preserves_active_connections() {
    let mut config = WebSocketConfig::default();
    config.connection_timeout_secs = 1;
    let manager = WebSocketManager::new(config);
    
    let connection_id = "test-conn-1".to_string();
    let (tx, _rx) = mpsc::unbounded_channel();
    
    manager.register_connection(connection_id.clone(), None, tx).unwrap();
    
    // Update activity before timeout
    sleep(Duration::from_millis(500)).await;
    manager.update_activity(&connection_id);
    
    // Wait past original timeout
    sleep(Duration::from_secs(1)).await;
    
    let cleaned = manager.cleanup_stale_connections();
    assert_eq!(cleaned, 0); // Should not clean active connection
    assert_eq!(manager.connection_count(), 1);
}

#[tokio::test]
async fn test_unsubscribe_cleans_empty_channels() {
    let config = WebSocketConfig::default();
    let manager = WebSocketManager::new(config);
    
    let connection_id = "test-conn-1".to_string();
    let (tx, _rx) = mpsc::unbounded_channel();
    
    manager.register_connection(connection_id.clone(), Some("user-1".to_string()), tx).unwrap();
    
    let channel = "brain:thoughts".to_string();
    manager.subscribe(&connection_id, channel.clone(), None).unwrap();
    
    assert_eq!(manager.channel_subscription_count(&channel), 1);
    
    manager.unsubscribe(&connection_id, &channel).unwrap();
    
    // Channel should be cleaned up (empty channels removed)
    assert_eq!(manager.channel_subscription_count(&channel), 0);
}

// ============================================================================
// Concurrent Operations Tests
// ============================================================================

#[tokio::test]
async fn test_concurrent_subscriptions() {
    let config = WebSocketConfig::default();
    let manager = Arc::new(WebSocketManager::new(config));
    
    let connection_id = "test-conn-1".to_string();
    let (tx, _rx) = mpsc::unbounded_channel();
    
    manager.register_connection(connection_id.clone(), Some("user-1".to_string()), tx).unwrap();
    
    // Concurrent subscriptions
    let mut handles = vec![];
    for i in 0..10 {
        let manager_clone = manager.clone();
        let conn_id = connection_id.clone();
        let channel = format!("channel-{}", i);
        
        handles.push(tokio::spawn(async move {
            manager_clone.subscribe(&conn_id, channel, None)
        }));
    }
    
    for handle in handles {
        assert!(handle.await.unwrap().is_ok());
    }
    
    let channels = manager.get_connection_channels(&connection_id, &connection_id).unwrap();
    assert_eq!(channels.len(), 10);
}

#[tokio::test]
async fn test_concurrent_broadcasts() {
    let config = WebSocketConfig::default();
    let manager = Arc::new(WebSocketManager::new(config));
    
    let channel = "test:channel".to_string();
    
    // Create multiple connections
    for i in 0..5 {
        let connection_id = format!("conn-{}", i);
        let (tx, _rx) = mpsc::unbounded_channel();
        manager.register_connection(connection_id.clone(), Some("user-1".to_string()), tx).unwrap();
        manager.subscribe(&connection_id, channel.clone(), None).unwrap();
    }
    
    // Concurrent broadcasts
    let mut handles = vec![];
    for _ in 0..10 {
        let manager_clone = manager.clone();
        let channel_clone = channel.clone();
        
        handles.push(tokio::spawn(async move {
            let message = WsMessage::event(channel_clone.clone(), serde_json::json!({"type": "test"}));
            manager_clone.broadcast_to_channel(&channel_clone, message)
        }));
    }
    
    for handle in handles {
        let count = handle.await.unwrap();
        assert_eq!(count, 5); // Should reach all 5 connections
    }
}

// ============================================================================
// Edge Cases Tests
// ============================================================================

#[tokio::test]
async fn test_get_connection_channels_unauthorized() {
    let config = WebSocketConfig::default();
    let manager = WebSocketManager::new(config);
    
    let connection_id1 = "conn-1".to_string();
    let connection_id2 = "conn-2".to_string();
    let (tx, _rx) = mpsc::unbounded_channel();
    
    manager.register_connection(connection_id1.clone(), None, tx).unwrap();
    manager.subscribe(&connection_id1, "channel-1".to_string(), None).unwrap();
    
    // connection_id2 should not be able to see connection_id1's channels
    let result = manager.get_connection_channels(&connection_id1, &connection_id2);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unauthorized"));
}

#[tokio::test]
async fn test_dead_connection_cleanup_during_broadcast() {
    let config = WebSocketConfig::default();
    let manager = WebSocketManager::new(config);
    
    let connection_id = "test-conn-1".to_string();
    let (tx, _rx) = mpsc::unbounded_channel();
    
    manager.register_connection(connection_id.clone(), Some("user-1".to_string()), tx).unwrap();
    
    let channel = "brain:thoughts".to_string();
    manager.subscribe(&connection_id, channel.clone(), None).unwrap();
    
    // Drop the receiver to simulate dead connection
    drop(_rx);
    
    // Broadcast should clean up dead connection
    let message = WsMessage::event(channel.clone(), serde_json::json!({"type": "test"}));
    let count = manager.broadcast_to_channel(&channel, message);
    
    assert_eq!(count, 0); // Should not send to dead connection
    assert_eq!(manager.connection_count(), 0); // Dead connection should be cleaned up
}

#[tokio::test]
async fn test_multiple_subscriptions_same_channel() {
    let config = WebSocketConfig::default();
    let manager = WebSocketManager::new(config);
    
    let connection_id = "test-conn-1".to_string();
    let (tx, _rx) = mpsc::unbounded_channel();
    
    manager.register_connection(connection_id.clone(), Some("user-1".to_string()), tx).unwrap();
    
    let channel = "brain:thoughts".to_string();
    
    // Subscribe multiple times (should be idempotent)
    manager.subscribe(&connection_id, channel.clone(), None).unwrap();
    manager.subscribe(&connection_id, channel.clone(), None).unwrap();
    manager.subscribe(&connection_id, channel.clone(), None).unwrap();
    
    let channels = manager.get_connection_channels(&connection_id, &connection_id).unwrap();
    assert_eq!(channels.len(), 1); // Should still be only one subscription
}

#[test]
fn test_message_json_nesting_depth() {
    // Test that deeply nested JSON is rejected
    let mut deep_json = "{".to_string();
    for _ in 0..200 {
        deep_json.push_str("\"nested\": {");
    }
    for _ in 0..200 {
        deep_json.push('}');
    }
    
    // This should fail due to nesting depth limit (128)
    let result = WsMessage::from_json(&deep_json);
    assert!(result.is_err());
}

#[test]
fn test_message_empty_string() {
    let result = WsMessage::from_json("");
    assert!(result.is_err());
}

#[test]
fn test_message_invalid_json() {
    let result = WsMessage::from_json("not json");
    assert!(result.is_err());
}

#[test]
fn test_message_not_object() {
    // JSON array should be rejected (must be object)
    let result = WsMessage::from_json("[1, 2, 3]");
    assert!(result.is_err());
}




