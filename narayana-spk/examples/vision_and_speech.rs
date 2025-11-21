//! Example: Vision + Speech Integration
//! 
//! This example demonstrates the complete flow:
//! 1. Vision adapter captures frames and detects objects
//! 2. Vision events are emitted to World Broker
//! 3. Brain/CPL processes vision events
//! 4. Speech adapter synthesizes descriptions of what was seen
//!
//! Run with: cargo run --example vision_and_speech --package narayana-spk

use narayana_storage::cognitive::CognitiveBrain;
use narayana_storage::conscience_persistent_loop::{ConsciencePersistentLoop, CPLConfig};
use narayana_wld::{WorldBroker, WorldBrokerConfig};
use narayana_wld::event_transformer::{WorldEvent, WorldAction};
use narayana_spk::{SpeechAdapter, SpeechConfig};
use narayana_spk::cpl_integration::create_speech_adapter_from_cpl;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use serde_json::json;
use tracing::{info, warn, debug};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing if not already initialized
    let _ = tracing_subscriber::fmt::try_init();
    
    info!("=== Vision + Speech Integration Example ===");

    // Step 1: Create Cognitive Brain
    info!("1. Creating Cognitive Brain...");
    let brain = Arc::new(CognitiveBrain::new());

    // Step 2: Create CPL with speech enabled
    info!("2. Creating CPL with speech enabled...");
    let mut cpl_config = CPLConfig::default();
    cpl_config.enable_speech = true;
    cpl_config.speech_config = Some(json!({
        "enabled": true,
        "engine": "Native",
        "rate": 150,
        "volume": 0.8,
        "voice": {
            "language": "en-US"
        }
    }));

    let cpl = Arc::new(ConsciencePersistentLoop::new(brain.clone(), cpl_config.clone()));
    let cpl_clone = cpl.clone();
    cpl_clone.initialize().await?;
    cpl_clone.start().await?;
    info!("CPL initialized and started");

    // Step 3: Note - Using simulated vision events for testing
    info!("3. Using simulated vision events (no camera required for testing)...");

    // Step 4: Create Speech Adapter
    info!("5. Creating Speech Adapter...");
    let speech_adapter = match create_speech_adapter_from_cpl(&cpl_config) {
        Ok(Some(adapter)) => {
            info!("Speech adapter created from CPL config");
            adapter
        }
        Ok(None) => {
            warn!("Speech adapter not created from CPL, using default");
            let mut config = SpeechConfig::default();
            config.enabled = true;
            SpeechAdapter::new(config)?
        }
        Err(e) => {
            warn!("Failed to create speech adapter from CPL: {}, using default", e);
            let mut config = SpeechConfig::default();
            config.enabled = true;
            SpeechAdapter::new(config)?
        }
    };

    // Step 5: Create World Broker
    info!("6. Creating World Broker...");
    let broker_config = WorldBrokerConfig::default();
    let broker = WorldBroker::new(brain.clone(), cpl.clone(), broker_config)?;
    
    // Register speech adapter
    broker.register_adapter(Box::new(speech_adapter));
    info!("Speech adapter registered with World Broker");
    info!("Note: Using simulated vision events for testing (no camera required)");
    
    // Start broker
    broker.start().await?;
    info!("World Broker started");

    // Step 6: Set up vision-to-speech processing
    info!("7. Setting up vision-to-speech pipeline...");
    
    // Helper function to process vision events and trigger speech
    async fn process_vision_for_speech(
        broker: &WorldBroker,
        event: WorldEvent,
    ) -> Result<(), narayana_core::Error> {
        if let WorldEvent::SensorData { source, data, .. } = event {
            // Check if this is a vision event (vision adapter uses "camera_0" or similar)
            if source.starts_with("camera_") || source == "vision" {
                debug!("Vision event received from source: {}", source);
                
                // Extract detection information
                if let Some(detections) = data.get("detections").and_then(|d| d.as_array()) {
                    if !detections.is_empty() {
                        // Build description from detections
                        let mut description = String::from("I can see ");
                        let mut object_names = Vec::new();
                        
                        // Limit to first 5 detections to avoid overly long descriptions
                        for (i, det) in detections.iter().take(5).enumerate() {
                            if let Some(obj_name) = det.get("class_name").and_then(|n| n.as_str()) {
                                if i > 0 {
                                    if i == detections.len().min(5) - 1 {
                                        description.push_str(" and ");
                                    } else {
                                        description.push_str(", ");
                                    }
                                }
                                description.push_str(obj_name);
                                object_names.push(obj_name);
                            }
                        }
                        
                        if !object_names.is_empty() {
                            description.push('.');
                            
                            info!("📢 Speaking: {}", description);
                            
                            // Send speech action through broker
                            let speech_action = WorldAction::ActuatorCommand {
                                target: "speech".to_string(),
                                command: json!({
                                    "text": description
                                }),
                            };
                            
                            broker.send_action(speech_action).await?;
                        }
                    }
                }
                
                // Check for scene description
                if let Some(scene_desc) = data.get("scene_description").and_then(|d| d.as_str()) {
                    if !scene_desc.is_empty() {
                        let description = format!("Scene description: {}.", scene_desc);
                        info!("📢 Speaking scene description: {}", description);
                        
                        let speech_action = WorldAction::ActuatorCommand {
                            target: "speech".to_string(),
                            command: json!({
                                "text": description
                            }),
                        };
                        
                        broker.send_action(speech_action).await?;
                    }
                }
            }
        }
        Ok(())
    }

    // Step 7: Wait for initialization
    info!("8. Waiting for initialization...");
    sleep(Duration::from_millis(500)).await;

    // Step 8: Demonstrate manual speech trigger based on vision
    info!("9. Demonstrating manual vision-to-speech trigger...");
    
    // Simulate a vision event that would trigger speech
    let vision_event = WorldEvent::SensorData {
        source: "vision".to_string(),
        data: json!({
            "detections": [
                {
                    "class_name": "person",
                    "confidence": 0.95,
                    "bbox": [100, 100, 200, 300]
                },
                {
                    "class_name": "laptop",
                    "confidence": 0.87,
                    "bbox": [300, 200, 400, 350]
                },
                {
                    "class_name": "cup",
                    "confidence": 0.72,
                    "bbox": [500, 300, 550, 400]
                }
            ],
            "timestamp": chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64
        }),
        timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
    };

    info!("Sending simulated vision event...");
    broker.process_world_event(vision_event.clone()).await?;
    
    // Process the event to trigger speech
    process_vision_for_speech(&broker, vision_event).await?;
    
    sleep(Duration::from_secs(2)).await;

    // Step 9: Demonstrate scene description
    info!("10. Demonstrating scene description...");
    
    let scene_event = WorldEvent::SensorData {
        source: "vision".to_string(),
        data: json!({
            "scene_description": "A person is working at a desk with a laptop and a cup nearby. The scene appears to be an office environment.",
            "timestamp": chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64
        }),
        timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
    };

    info!("Sending scene description event...");
    broker.process_world_event(scene_event.clone()).await?;
    
    // Process the scene event to trigger speech
    process_vision_for_speech(&broker, scene_event).await?;
    
    sleep(Duration::from_secs(2)).await;

    // Step 10: Demonstrate direct speech action (what the vision processor would send)
    info!("11. Demonstrating direct speech action...");
    
    let speech_action = WorldAction::ActuatorCommand {
        target: "speech".to_string(),
        command: json!({
            "text": "I can see a person, a laptop, and a cup. This appears to be an office workspace."
        }),
    };

    broker.send_action(speech_action).await?;
    info!("Speech action sent");

    // Wait for processing
    sleep(Duration::from_secs(3)).await;

    info!("=== Example Complete ===");
    info!("Flow demonstrated:");
    info!("  1. Vision captures frames and detects objects");
    info!("  2. Vision events emitted to World Broker");
    info!("  3. Brain/CPL processes vision events");
    info!("  4. Speech synthesizes descriptions of what was seen");
    
    // Cleanup
    broker.stop().await?;
    cpl.stop().await?;

    Ok(())
}

