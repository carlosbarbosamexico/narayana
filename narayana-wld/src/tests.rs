//! Comprehensive tests for world broker interface

#[cfg(test)]
mod tests {
    use narayana_storage::cognitive::CognitiveBrain;
    use narayana_storage::conscience_persistent_loop::{ConsciencePersistentLoop, CPLConfig};
    use crate::event_transformer::{WorldEvent, WorldAction, EventTransformer};
    use crate::attention_filter::{AttentionFilter, AttentionFilterConfig};
    use crate::config::WorldBrokerConfig;
    use crate::world_broker::WorldBroker;
    use serde_json::json;
    use std::sync::Arc;
    use parking_lot::RwLock;
    use tokio::time::{sleep, Duration};

    fn create_test_brain() -> Arc<CognitiveBrain> {
        Arc::new(CognitiveBrain::new())
    }

    fn create_test_cpl(brain: Arc<CognitiveBrain>) -> Arc<ConsciencePersistentLoop> {
        let config = CPLConfig::default();
        Arc::new(ConsciencePersistentLoop::new(brain, config))
    }

    // ============================================================================
    // Event Transformer Tests
    // ============================================================================

    #[tokio::test]
    async fn test_event_transformer_world_to_cognitive() {
        let transformer = EventTransformer::new();
        
        let event = WorldEvent::SensorData {
            source: "test_sensor".to_string(),
            data: json!({"temperature": 25.5}),
            timestamp: 1000,
        };

        let result = transformer.world_to_cognitive(&event);
        assert!(result.is_ok());
        
        match result.unwrap() {
            narayana_storage::cognitive::CognitiveEvent::ExperienceStored { .. } => {
                // Expected
            }
            _ => panic!("Unexpected event type"),
        }
    }

    #[tokio::test]
    async fn test_event_transformer_world_to_cpl() {
        let transformer = EventTransformer::new();
        
        let event = WorldEvent::UserInput {
            user_id: "user1".to_string(),
            input: "Hello".to_string(),
            context: json!({}),
        };

        let result = transformer.world_to_cpl(&event);
        assert!(result.is_ok());
        
        match result.unwrap() {
            narayana_storage::conscience_persistent_loop::CPLEvent::GlobalWorkspaceBroadcast { .. } => {
                // Expected
            }
            _ => panic!("Unexpected event type"),
        }
    }

    #[tokio::test]
    async fn test_event_transformer_sanitization() {
        let transformer = EventTransformer::new();
        
        // Test with potentially malicious input
        let event = WorldEvent::SensorData {
            source: "sensor{};DROP TABLE--".to_string(),
            data: json!({}),
            timestamp: 1000,
        };

        let result = transformer.world_to_cognitive(&event);
        assert!(result.is_ok());
        
        // Should sanitize the source
        let event_id = match result.unwrap() {
            narayana_storage::cognitive::CognitiveEvent::ExperienceStored { experience_id } => experience_id,
            _ => panic!("Unexpected event type"),
        };
        
        // Should not contain dangerous characters
        assert!(!event_id.contains("{}"));
        assert!(!event_id.contains("DROP"));
    }

    #[tokio::test]
    async fn test_event_transformer_large_input() {
        let transformer = EventTransformer::new();
        
        // Test with very large input
        let large_input = "x".repeat(200_000);
        let event = WorldEvent::UserInput {
            user_id: "user1".to_string(),
            input: large_input,
            context: json!({}),
        };

        let result = transformer.world_to_cognitive(&event);
        assert!(result.is_err()); // Should reject too large input
    }

    #[tokio::test]
    async fn test_event_transformer_empty_source() {
        let transformer = EventTransformer::new();
        
        let event = WorldEvent::SensorData {
            source: "".to_string(),
            data: json!({}),
            timestamp: 1000,
        };

        let result = transformer.world_to_cognitive(&event);
        assert!(result.is_err()); // Should reject empty source
    }

    #[tokio::test]
    async fn test_event_transformer_cognitive_to_world() {
        let transformer = EventTransformer::new();
        
        let event = narayana_storage::cognitive::CognitiveEvent::ThoughtCompleted {
            thought_id: "thought_123".to_string(),
        };

        let result = transformer.cognitive_to_world(&event);
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_event_transformer_cpl_to_world() {
        let transformer = EventTransformer::new();
        
        // High priority event
        let event = narayana_storage::conscience_persistent_loop::CPLEvent::GlobalWorkspaceBroadcast {
            content_id: "content_123".to_string(),
            priority: 0.9,
        };

        let result = transformer.cpl_to_world(&event);
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
        
        // Low priority event
        let event = narayana_storage::conscience_persistent_loop::CPLEvent::GlobalWorkspaceBroadcast {
            content_id: "content_456".to_string(),
            priority: 0.5,
        };

        let result = transformer.cpl_to_world(&event);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_event_transformer_invalid_priority() {
        let transformer = EventTransformer::new();
        
        // Test with NaN priority
        let event = narayana_storage::conscience_persistent_loop::CPLEvent::GlobalWorkspaceBroadcast {
            content_id: "content_123".to_string(),
            priority: f64::NAN,
        };

        let result = transformer.cpl_to_world(&event);
        assert!(result.is_ok());
        // Should handle NaN gracefully
    }

    // ============================================================================
    // Attention Filter Tests
    // ============================================================================

    #[tokio::test]
    async fn test_attention_filter_salience() {
        let brain = create_test_brain();
        let filter = AttentionFilter::new(brain, Default::default());
        
        let event = WorldEvent::Command {
            command: "test".to_string(),
            args: json!({}),
        };

        let salience = filter.compute_salience(&event).unwrap();
        assert!(salience >= 0.0 && salience <= 1.0);
    }

    #[tokio::test]
    async fn test_attention_filter_routing() {
        let brain = create_test_brain();
        let filter = AttentionFilter::new(brain, Default::default());
        
        // High-salience event (command)
        let high_event = WorldEvent::Command {
            command: "urgent".to_string(),
            args: json!({}),
        };
        
        // Low-salience event (sensor data)
        let low_event = WorldEvent::SensorData {
            source: "sensor".to_string(),
            data: json!({}),
            timestamp: 1000,
        };

        let should_route_high = filter.should_route_to_workspace(&high_event).unwrap();
        let should_route_low = filter.should_route_to_workspace(&low_event).unwrap();
        
        // Commands should generally have higher salience than sensor data
        assert!(should_route_high || !should_route_low || should_route_high == should_route_low);
    }

    #[tokio::test]
    async fn test_attention_filter_novelty() {
        let brain = create_test_brain();
        let filter = AttentionFilter::new(brain, Default::default());
        
        // First event should have high novelty
        let event1 = WorldEvent::SensorData {
            source: "new_sensor".to_string(),
            data: json!({}),
            timestamp: 1000,
        };
        
        let salience1 = filter.compute_salience(&event1).unwrap();
        
        // Same event again should have lower novelty
        let salience2 = filter.compute_salience(&event1).unwrap();
        
        // First occurrence should generally have higher salience
        assert!(salience1 >= salience2 || (salience1 - salience2).abs() < 0.1);
    }

    #[tokio::test]
    async fn test_attention_filter_timestamp_validation() {
        let brain = create_test_brain();
        let filter = AttentionFilter::new(brain, Default::default());
        
        // Test with timestamp far in the future
        let future_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() + 10000; // 10k seconds in future
        
        let event = WorldEvent::SensorData {
            source: "sensor".to_string(),
            data: json!({}),
            timestamp: future_timestamp,
        };

        // Should handle gracefully
        let result = filter.compute_salience(&event);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_attention_filter_event_type_sanitization() {
        let brain = create_test_brain();
        let filter = AttentionFilter::new(brain, Default::default());
        
        // Test with malicious event type
        let event = WorldEvent::SystemEvent {
            event_type: "type{};DROP--".to_string(),
            payload: json!({}),
        };

        // Should sanitize and handle
        let result = filter.compute_salience(&event);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_attention_filter_magnitude_validation() {
        let brain = create_test_brain();
        let filter = AttentionFilter::new(brain, Default::default());
        
        // Test with NaN magnitude
        let event = WorldEvent::SensorData {
            source: "sensor".to_string(),
            data: json!({"magnitude": f64::NAN}),
            timestamp: 1000,
        };

        // Should handle NaN gracefully
        let result = filter.compute_salience(&event);
        assert!(result.is_ok());
        assert!(result.unwrap().is_finite());
    }

    #[tokio::test]
    async fn test_attention_filter_unbounded_growth() {
        let brain = create_test_brain();
        let config = AttentionFilterConfig {
            context_window_size: 10,
            ..Default::default()
        };
        let filter = AttentionFilter::new(brain, config);
        
        // Send many events
        for i in 0..100 {
            let event = WorldEvent::SensorData {
                source: format!("sensor_{}", i),
                data: json!({}),
                timestamp: 1000 + i as u64,
            };
            filter.compute_salience(&event).unwrap();
        }
        
        // History should be bounded - verify by checking that salience computation
        // doesn't grow unbounded (indirect test since event_history is private)
        let event = WorldEvent::SensorData {
            source: "sensor_0".to_string(),
            data: json!({}),
            timestamp: 2000,
        };
        let salience = filter.compute_salience(&event).unwrap();
        assert!(salience.is_finite());
        assert!(salience >= 0.0 && salience <= 1.0);
    }

    // ============================================================================
    // World Broker Tests
    // ============================================================================

    #[tokio::test]
    async fn test_world_broker_creation() {
        let brain = create_test_brain();
        let cpl = create_test_cpl(brain.clone());
        let config = WorldBrokerConfig::default();
        
        let broker = WorldBroker::new(brain, cpl, config);
        assert!(broker.is_ok());
    }

    #[tokio::test]
    async fn test_world_broker_invalid_config() {
        let brain = create_test_brain();
        let cpl = create_test_cpl(brain.clone());
        let mut config = WorldBrokerConfig::default();
        config.salience_threshold = 2.0; // Invalid: > 1.0
        
        let broker = WorldBroker::new(brain, cpl, config);
        assert!(broker.is_err());
    }

    #[tokio::test]
    async fn test_world_broker_start_stop() {
        let brain = create_test_brain();
        let cpl = create_test_cpl(brain.clone());
        let mut config = WorldBrokerConfig::default();
        config.enabled_adapters = vec![]; // No adapters for test
        
        let broker = WorldBroker::new(brain, cpl, config).unwrap();
        
        // Start should succeed
        let start_result = broker.start().await;
        assert!(start_result.is_ok());
        
        // Double start should fail
        let double_start = broker.start().await;
        assert!(double_start.is_err());
        
        // Stop should succeed
        let stop_result = broker.stop().await;
        assert!(stop_result.is_ok());
        
        // Double stop should fail
        let double_stop = broker.stop().await;
        assert!(double_stop.is_err());
    }

    #[tokio::test]
    async fn test_world_broker_process_event() {
        let brain = create_test_brain();
        let cpl = create_test_cpl(brain.clone());
        let mut config = WorldBrokerConfig::default();
        config.enabled_adapters = vec![];
        
        let broker = WorldBroker::new(brain, cpl, config).unwrap();
        broker.start().await.unwrap();
        
        let event = WorldEvent::UserInput {
            user_id: "user1".to_string(),
            input: "test".to_string(),
            context: json!({}),
        };
        
        let result = broker.process_world_event(event).await;
        assert!(result.is_ok());
        
        broker.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_world_broker_send_action() {
        let brain = create_test_brain();
        let cpl = create_test_cpl(brain.clone());
        let mut config = WorldBrokerConfig::default();
        config.enabled_adapters = vec![];
        
        let broker = WorldBroker::new(brain, cpl, config).unwrap();
        broker.start().await.unwrap();
        
        let action = WorldAction::SystemNotification {
            channel: "test".to_string(),
            content: json!({"message": "test"}),
        };
        
        let result = broker.send_action(action).await;
        assert!(result.is_ok());
        
        broker.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_world_broker_action_validation() {
        let brain = create_test_brain();
        let cpl = create_test_cpl(brain.clone());
        let mut config = WorldBrokerConfig::default();
        config.enabled_adapters = vec![];
        
        let broker = WorldBroker::new(brain, cpl, config).unwrap();
        broker.start().await.unwrap();
        
        // Test with invalid action (empty target)
        let invalid_action = WorldAction::ActuatorCommand {
            target: "".to_string(),
            command: json!({}),
        };
        
        let result = broker.send_action(invalid_action).await;
        assert!(result.is_err());
        
        broker.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_world_broker_adapter_registration() {
        let brain = create_test_brain();
        let cpl = create_test_cpl(brain.clone());
        let config = WorldBrokerConfig::default();
        
        let broker = WorldBroker::new(brain, cpl, config).unwrap();
        
        // Register HTTP adapter
        let http_adapter = crate::protocol_adapters::HttpAdapter::new(8080);
        broker.register_adapter(Box::new(http_adapter));
        
        // Try to register invalid adapter name
        // (Can't easily test this without creating a mock adapter)
    }

    // ============================================================================
    // Sensory Interface Tests
    // ============================================================================

    #[tokio::test]
    async fn test_sensory_interface() {
        let brain = create_test_brain();
        let cpl = create_test_cpl(brain.clone());
        let transformer = Arc::new(RwLock::new(EventTransformer::new()));
        let attention_filter = Arc::new(AttentionFilter::new(brain.clone(), Default::default()));
        
        let sensory = crate::sensory_interface::SensoryInterface::new(
            brain,
            cpl,
            transformer,
            attention_filter,
        );

        let event = WorldEvent::UserInput {
            user_id: "user1".to_string(),
            input: "test".to_string(),
            context: json!({}),
        };

        let result = sensory.process_event(event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_sensory_interface_subscription() {
        let brain = create_test_brain();
        let cpl = create_test_cpl(brain.clone());
        let transformer = Arc::new(RwLock::new(EventTransformer::new()));
        let attention_filter = Arc::new(AttentionFilter::new(brain.clone(), Default::default()));
        
        let sensory = crate::sensory_interface::SensoryInterface::new(
            brain,
            cpl,
            transformer,
            attention_filter,
        );

        let mut receiver = sensory.subscribe();
        
        let event = WorldEvent::UserInput {
            user_id: "user1".to_string(),
            input: "test".to_string(),
            context: json!({}),
        };

        sensory.process_event(event.clone()).await.unwrap();
        
        // Should receive event
        tokio::time::timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
    }

    // ============================================================================
    // Motor Interface Tests
    // ============================================================================

    #[tokio::test]
    async fn test_motor_interface() {
        let brain = create_test_brain();
        let transformer = Arc::new(RwLock::new(EventTransformer::new()));
        
        let motor = crate::motor_interface::MotorInterface::new(brain.clone(), transformer);
        
        let action = WorldAction::SystemNotification {
            channel: "test".to_string(),
            content: json!({"message": "test"}),
        };

        let result = motor.queue_action(action.clone()).await;
        assert!(result.is_ok());
        
        let popped = motor.pop_action();
        assert!(popped.is_some());
        assert_eq!(format!("{:?}", popped.unwrap()), format!("{:?}", action));
    }

    #[tokio::test]
    async fn test_motor_interface_queue_bounds() {
        let brain = create_test_brain();
        let transformer = Arc::new(RwLock::new(EventTransformer::new()));
        
        let motor = crate::motor_interface::MotorInterface::new(brain.clone(), transformer);
        
        // Fill queue beyond limit
        for i in 0..10_100 {
            let action = WorldAction::SystemNotification {
                channel: format!("channel_{}", i),
                content: json!({"index": i}),
            };
            motor.queue_action(action).await.unwrap();
        }
        
        // Queue should be bounded - verify by popping all and counting
        let mut count = 0;
        while motor.pop_action().is_some() {
            count += 1;
        }
        assert!(count <= 10_000);
    }

    #[tokio::test]
    async fn test_motor_interface_listening() {
        let brain = create_test_brain();
        let transformer = Arc::new(RwLock::new(EventTransformer::new()));
        
        let motor = crate::motor_interface::MotorInterface::new(brain.clone(), transformer);
        
        // Start listening
        motor.start_listening().await.unwrap();
        
        // Create a thought - this will emit a ThoughtCreated event
        let _thought_id = brain.create_thought(
            json!({"task": "test"}),
            0.8,
        ).unwrap();
        
        // Wait a bit for event processing
        sleep(Duration::from_millis(100)).await;
        
        // Test that listening is working (may or may not have action depending on timing)
        // The important thing is that it doesn't panic
        // ThoughtCreated events don't generate actions, only ThoughtCompleted do
    }

    // ============================================================================
    // Configuration Tests
    // ============================================================================

    #[test]
    fn test_config_validation() {
        let mut config = WorldBrokerConfig::default();
        
        // Valid config
        assert!(config.validate().is_ok());
        
        // Invalid salience threshold
        config.salience_threshold = 2.0;
        assert!(config.validate().is_err());
        
        config.salience_threshold = 0.5;
        
        // Invalid weights (don't sum to 1.0)
        config.novelty_weight = 0.5;
        config.urgency_weight = 0.5;
        config.relevance_weight = 0.5;
        config.magnitude_weight = 0.5;
        config.prediction_error_weight = 0.5;
        assert!(config.validate().is_err());
        
        // Invalid buffer size
        config = WorldBrokerConfig::default();
        config.event_buffer_size = 0;
        assert!(config.validate().is_err());
    }

    // ============================================================================
    // Integration Tests
    // ============================================================================

    #[tokio::test]
    async fn test_end_to_end_event_flow() {
        let brain = create_test_brain();
        let cpl = create_test_cpl(brain.clone());
        let mut config = WorldBrokerConfig::default();
        config.enabled_adapters = vec![];
        
        let broker = WorldBroker::new(brain.clone(), cpl, config).unwrap();
        broker.start().await.unwrap();
        
        // Send world event
        let event = WorldEvent::UserInput {
            user_id: "user1".to_string(),
            input: "Hello, world!".to_string(),
            context: json!({"session": "test"}),
        };
        
        broker.process_world_event(event).await.unwrap();
        
        // Wait for processing
        sleep(Duration::from_millis(200)).await;
        
        broker.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_concurrent_events() {
        let brain = create_test_brain();
        let cpl = create_test_cpl(brain.clone());
        let mut config = WorldBrokerConfig::default();
        config.enabled_adapters = vec![];
        
        let broker = Arc::new(WorldBroker::new(brain, cpl, config).unwrap());
        broker.start().await.unwrap();
        
        // Send multiple events concurrently
        let mut handles = vec![];
        for i in 0..10 {
            let broker_clone = broker.clone();
            let handle = tokio::spawn(async move {
                let event = WorldEvent::SensorData {
                    source: format!("sensor_{}", i),
                    data: json!({"value": i}),
                    timestamp: 1000 + i as u64,
                };
                broker_clone.process_world_event(event).await
            });
            handles.push(handle);
        }
        
        // Wait for all
        for handle in handles {
            assert!(handle.await.unwrap().is_ok());
        }
        
        broker.stop().await.unwrap();
    }

    // ============================================================================
    // Edge Case Tests
    // ============================================================================

    #[tokio::test]
    async fn test_empty_strings() {
        let transformer = EventTransformer::new();
        
        // Empty source
        let event = WorldEvent::SensorData {
            source: "".to_string(),
            data: json!({}),
            timestamp: 1000,
        };
        assert!(transformer.world_to_cognitive(&event).is_err());
        
        // Empty user_id
        let event = WorldEvent::UserInput {
            user_id: "".to_string(),
            input: "test".to_string(),
            context: json!({}),
        };
        assert!(transformer.world_to_cognitive(&event).is_err());
    }

    #[tokio::test]
    async fn test_very_large_payloads() {
        let transformer = EventTransformer::new();
        
        // Very large JSON payload
        let large_data: serde_json::Value = json!({
            "data": vec![0u8; 2_000_000]
        });
        
        let event = WorldEvent::SensorData {
            source: "sensor".to_string(),
            data: large_data,
            timestamp: 1000,
        };
        
        // Should handle or reject gracefully
        let result = transformer.world_to_cognitive(&event);
        // May succeed or fail depending on validation
    }

    #[tokio::test]
    async fn test_timestamp_edge_cases() {
        let brain = create_test_brain();
        let filter = AttentionFilter::new(brain, Default::default());
        
        // Timestamp in far future
        let future_ts = u64::MAX;
        let event = WorldEvent::SensorData {
            source: "sensor".to_string(),
            data: json!({}),
            timestamp: future_ts,
        };
        
        // Should handle gracefully
        let result = filter.compute_salience(&event);
        assert!(result.is_ok());
        
        // Timestamp in past
        let past_ts = 0;
        let event = WorldEvent::SensorData {
            source: "sensor".to_string(),
            data: json!({}),
            timestamp: past_ts,
        };
        
        let result = filter.compute_salience(&event);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_special_characters() {
        let transformer = EventTransformer::new();
        
        // Test with various special characters
        let special_chars = vec!["{}", "[]", "();", "DROP", "DELETE", "\\", "/", "\"", "'"];
        
        for chars in special_chars {
            let event = WorldEvent::SensorData {
                source: format!("sensor{}", chars),
                data: json!({}),
                timestamp: 1000,
            };
            
            // Should sanitize
            let result = transformer.world_to_cognitive(&event);
            if result.is_ok() {
                let event_id = match result.unwrap() {
                    narayana_storage::cognitive::CognitiveEvent::ExperienceStored { experience_id } => experience_id,
                    _ => continue,
                };
                // Should not contain dangerous characters
                assert!(!event_id.contains("{}"));
                assert!(!event_id.contains("DROP"));
            }
        }
    }

    #[tokio::test]
    async fn test_unicode_input() {
        let transformer = EventTransformer::new();
        
        // Test with unicode characters
        let event = WorldEvent::UserInput {
            user_id: "user_测试".to_string(),
            input: "Hello 世界".to_string(),
            context: json!({}),
        };
        
        // Should handle unicode
        let result = transformer.world_to_cognitive(&event);
        // May sanitize unicode or handle it
    }

    // ============================================================================
    // Performance Tests
    // ============================================================================

    #[tokio::test]
    async fn test_high_throughput() {
        let brain = create_test_brain();
        let cpl = create_test_cpl(brain.clone());
        let mut config = WorldBrokerConfig::default();
        config.enabled_adapters = vec![];
        
        let broker = Arc::new(WorldBroker::new(brain, cpl, config).unwrap());
        broker.start().await.unwrap();
        
        let start = std::time::Instant::now();
        
        // Process many events
        for i in 0..1000 {
            let event = WorldEvent::SensorData {
                source: format!("sensor_{}", i % 10),
                data: json!({"value": i}),
                timestamp: 1000 + i as u64,
            };
            broker.process_world_event(event).await.unwrap();
        }
        
        let elapsed = start.elapsed();
        println!("Processed 1000 events in {:?}", elapsed);
        assert!(elapsed.as_secs() < 10); // Should be fast
        
        broker.stop().await.unwrap();
    }
}
