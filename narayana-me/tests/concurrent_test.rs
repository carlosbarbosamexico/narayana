//! Concurrent access tests for narayana-me

use narayana_me::{AvatarConfig, AvatarBroker, AvatarProviderType, Expression, Gesture};
use std::sync::Arc;
use tokio::task;

#[tokio::test]
async fn test_concurrent_broker_creation() {
    let mut handles = vec![];
    
    for i in 0..10 {
        let handle = task::spawn(async move {
            let mut config = AvatarConfig::default();
            config.websocket_port = Some(8080 + i);
            AvatarBroker::new(config)
        });
        handles.push(handle);
    }
    
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_concurrent_expression_updates() {
    let config = AvatarConfig::default();
    let broker = Arc::new(AvatarBroker::new(config).unwrap());
    let mut handles = vec![];
    
    for i in 0..10 {
        let broker_clone = broker.clone();
        let handle = task::spawn(async move {
            let expression = match i % 4 {
                0 => Expression::Happy,
                1 => Expression::Sad,
                2 => Expression::Angry,
                _ => Expression::Neutral,
            };
            broker_clone.set_expression(expression, 0.8).await
        });
        handles.push(handle);
    }
    
    for handle in handles {
        let _result = handle.await.unwrap();
        // Should not panic, even if provider not initialized
    }
}

#[tokio::test]
async fn test_concurrent_gesture_updates() {
    let config = AvatarConfig::default();
    let broker = Arc::new(AvatarBroker::new(config).unwrap());
    let mut handles = vec![];
    
    for i in 0..10 {
        let broker_clone = broker.clone();
        let handle = task::spawn(async move {
            let gesture = match i % 4 {
                0 => Gesture::Wave,
                1 => Gesture::Nod,
                2 => Gesture::Shake,
                _ => Gesture::ThumbsUp,
            };
            broker_clone.set_gesture(gesture, 1000 * (i as u64 + 1)).await
        });
        handles.push(handle);
    }
    
    for handle in handles {
        let result = handle.await.unwrap();
        // Should not panic
    }
}

#[tokio::test]
async fn test_concurrent_get_client_url() {
    let config = AvatarConfig::default();
    let broker = Arc::new(AvatarBroker::new(config).unwrap());
    let mut handles = vec![];
    
    for _ in 0..20 {
        let broker_clone = broker.clone();
        let handle = task::spawn(async move {
            broker_clone.get_client_url().await
        });
        handles.push(handle);
    }
    
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_none()); // No stream started
    }
}

#[tokio::test]
async fn test_concurrent_stop_stream() {
    let config = AvatarConfig::default();
    let broker = Arc::new(AvatarBroker::new(config).unwrap());
    let mut handles = vec![];
    
    // Multiple concurrent stops should be idempotent
    for _ in 0..10 {
        let broker_clone = broker.clone();
        let handle = task::spawn(async move {
            broker_clone.stop_stream().await
        });
        handles.push(handle);
    }
    
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

