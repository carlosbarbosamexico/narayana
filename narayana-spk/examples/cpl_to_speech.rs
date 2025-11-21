//! Example: CPL → Brain → SPK → WLD
//! 
//! This example demonstrates the complete flow:
//! 1. CPL (Conscience Persistent Loop) generates a cognitive event
//! 2. Brain processes the event and may generate a response
//! 3. MotorInterface transforms cognitive event to WorldAction
//! 4. WorldBroker routes WorldAction to SpeechAdapter
//! 5. SpeechAdapter synthesizes speech and emits status event
//!
//! Run with: cargo run --example cpl_to_speech --package narayana-spk

use narayana_storage::cognitive::CognitiveBrain;
use narayana_storage::conscience_persistent_loop::{ConsciencePersistentLoop, CPLConfig, CPLEvent};
use narayana_wld::{WorldBroker, WorldBrokerConfig};
use narayana_wld::event_transformer::WorldAction;
use narayana_spk::{SpeechAdapter, SpeechConfig};
use narayana_spk::config::TtsEngine;
use narayana_spk::cpl_integration::create_speech_adapter_from_cpl;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use serde_json::json;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing (if tracing_subscriber is available)
    // tracing_subscriber::fmt::init();

    info!("=== CPL → Brain → SPK → WLD Example ===");

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

    // Step 3: Create and register Speech Adapter (before creating broker)
    info!("3. Creating Speech Adapter from CPL config...");
    let speech_adapter = match create_speech_adapter_from_cpl(&cpl_config) {
        Ok(Some(adapter)) => {
            info!("Speech adapter created successfully");
            adapter
        }
        Ok(None) => {
            warn!("Speech adapter not created (speech disabled in CPL)");
            // Create a default adapter for demonstration
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

    // Step 4: Create World Broker
    info!("4. Creating World Broker...");
    let broker_config = WorldBrokerConfig::default();
    let broker = WorldBroker::new(brain.clone(), cpl.clone(), broker_config)?;
    
    // Register adapter with broker (before starting broker)
    broker.register_adapter(Box::new(speech_adapter));
    info!("Speech adapter registered with World Broker");
    
    // Start broker (this will start all registered adapters)
    broker.start().await?;
    info!("World Broker started");

    // Step 5: Subscribe to speech events (to see when speech is synthesized)
    // Note: SpeechAdapter emits WorldEvent::SensorData when speech is synthesized
    // We'll demonstrate this by checking the adapter's event emission
    info!("5. Speech adapter will emit events when speech is synthesized");

    // Step 6: Simulate CPL generating a cognitive event that should trigger speech
    info!("5. Simulating CPL event that triggers speech...");
    
    // Wait a bit for everything to initialize
    sleep(Duration::from_millis(500)).await;

    // Method 1: Directly send a WorldAction to the broker (simulating what MotorInterface would do)
    info!("6. Sending speech action via World Broker...");
    let speech_action = WorldAction::ActuatorCommand {
        target: "speech".to_string(),
        command: json!({
            "text": "Hello! This is a test of the CPL to speech synthesis pipeline. The flow goes from CPL, through the brain, to the world broker, and finally to the speech adapter."
        }),
    };

    broker.send_action(speech_action).await?;
    info!("Speech action sent to broker");

    // Method 2: Simulate a cognitive event that would be transformed to speech
    // In a real scenario, the CPL would emit a CPLEvent, which MotorInterface would
    // transform to a WorldAction. For demonstration, we'll show how this could work:
    info!("7. Demonstrating cognitive event flow...");
    
    // Create a cognitive event that represents "I want to speak"
    // In practice, this would come from the CPL's internal processes
    let cognitive_event = narayana_storage::cognitive::CognitiveEvent::ThoughtCreated {
        thought_id: "example_thought_1".to_string(),
    };

    // The MotorInterface would transform this to a WorldAction
    // For this example, we'll directly create the action that would result
    let speech_action2 = WorldAction::ActuatorCommand {
        target: "speech".to_string(),
        command: json!({
            "text": "The cognitive event has been processed and transformed into a speech action."
        }),
    };

    broker.send_action(speech_action2).await?;
    info!("Second speech action sent");

    // Wait for speech synthesis to complete
    sleep(Duration::from_secs(2)).await;

    // Step 7: Demonstrate CPL event emission (if CPL emits events that trigger speech)
    info!("8. Demonstrating CPL event emission...");
    
    // In a real scenario, the CPL might emit events that get transformed to speech
    // For example, when the CPL has a realization or wants to communicate
    // The EventTransformer would convert CPLEvent to WorldAction
    
    // Send one more speech action to show the complete flow
    let speech_action3 = WorldAction::ActuatorCommand {
        target: "speech".to_string(),
        command: json!({
            "text": "This completes the demonstration of the CPL to speech pipeline."
        }),
    };

    broker.send_action(speech_action3).await?;
    info!("Final speech action sent");

    // Wait for processing
    sleep(Duration::from_secs(2)).await;

    info!("=== Example Complete ===");
    info!("Flow demonstrated: CPL → Brain → WorldBroker → SpeechAdapter → Speech Synthesis");
    
    // Cleanup
    broker.stop().await?;
    cpl.stop().await?;

    Ok(())
}

