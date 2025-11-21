//! Basic audio capture example
//! Demonstrates how to capture audio from the system microphone and analyze it

use narayana_sc::*;
use narayana_wld::{WorldBroker, WorldBrokerConfig};
use narayana_storage::conscience_persistent_loop::{ConsciencePersistentLoop, CPLConfig};
use narayana_storage::cognitive::CognitiveBrain;
use tokio::time::{sleep, Duration};
use tracing::{info, error};
use tracing_subscriber;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting basic audio capture example...");

    // Create audio configuration
    let mut audio_config = AudioConfig::default();
    audio_config.enabled = true;
    audio_config.sample_rate = 44100;
    audio_config.channels = 1;
    audio_config.buffer_size = 4096;
    
    // Enable analysis features
    audio_config.analysis.enable_fft = true;
    audio_config.analysis.enable_energy = true;
    audio_config.analysis.enable_zcr = true;
    audio_config.analysis.enable_spectral = true;
    audio_config.analysis.enable_pitch = true;
    
    // Enable advanced features
    audio_config.capture.noise_reduction = true;
    audio_config.capture.agc = true;
    
    // Validate configuration
    audio_config.validate()
        .map_err(|e| format!("Invalid audio config: {}", e))?;

    info!("Audio configuration validated");

    // Create a simple world broker for event handling
    // Note: In a real scenario, you'd have a CPL and Brain
    // For this example, we'll create minimal required components
    let brain = Arc::new(CognitiveBrain::new());
    let cpl_config = CPLConfig::default();
    let cpl = Arc::new(ConsciencePersistentLoop::new(
        "example-cpl".to_string(),
        brain.clone(),
        cpl_config,
    )?);
    
    let broker_config = WorldBrokerConfig::default();
    let world_broker = WorldBroker::new(brain, cpl, broker_config)
        .map_err(|e| format!("Failed to create world broker: {}", e))?;
    
    // Create audio adapter
    let audio_adapter = match AudioAdapter::new(audio_config) {
        Ok(adapter) => {
            info!("Audio adapter created successfully");
            adapter
        }
        Err(e) => {
            error!("Failed to create audio adapter: {}", e);
            error!("Note: This may fail if no audio device is available");
            return Err(format!("Audio adapter creation failed: {}", e).into());
        }
    };
    
    // Subscribe to events BEFORE registering (so we have a reference)
    let mut event_receiver = audio_adapter.subscribe_events();
    
    // Register audio adapter
    world_broker.register_adapter(Box::new(audio_adapter));

    info!("Audio adapter registered with world broker");

    // Start the world broker
    world_broker.start().await
        .map_err(|e| format!("Failed to start world broker: {}", e))?;

    info!("World broker started. Capturing audio for 10 seconds...");

    // Spawn a task to handle events
    let event_handle = tokio::spawn(async move {
        while let Ok(event) = event_receiver.recv().await {
            match event {
                narayana_wld::WorldEvent::SensorData { source, data, .. } if source == "audio" => {
                    info!("Received audio event: {:?}", data);
                    
                    // Extract analysis results if available
                    if let Some(analysis) = data.get("analysis") {
                        if let Some(energy) = analysis.get("energy").and_then(|v| v.as_f64()) {
                            info!("Audio energy: {:.2}", energy);
                        }
                        if let Some(pitch) = analysis.get("pitch").and_then(|v| v.as_f64()) {
                            info!("Detected pitch: {:.2} Hz", pitch);
                        }
                        if let Some(frequencies) = analysis.get("dominant_frequencies") {
                            if let Some(freqs) = frequencies.as_array() {
                                info!("Dominant frequencies: {:?}", freqs);
                            }
                        }
                    }
                    
                    // Extract voice-to-text if available
                    if let Some(text) = data.get("text") {
                        if let Some(text_str) = text.as_str() {
                            info!("Voice-to-text: {}", text_str);
                        }
                    }
                }
                _ => {
                    // Ignore other events
                }
            }
        }
    });

    // Run for 10 seconds
    sleep(Duration::from_secs(10)).await;

    info!("Stopping audio capture...");

    // Stop the world broker
    world_broker.stop().await
        .map_err(|e| format!("Failed to stop world broker: {}", e))?;

    // Wait for event handler to finish
    let _ = event_handle.await;

    info!("Audio capture example completed");

    Ok(())
}

