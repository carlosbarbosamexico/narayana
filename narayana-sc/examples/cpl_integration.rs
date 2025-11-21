//! CPL integration example
//! Demonstrates how to integrate audio capture with the Conscience Persistent Loop

use narayana_sc::*;
use narayana_storage::conscience_persistent_loop::CPLConfig;
use narayana_wld::WorldBroker;
use tokio::time::{sleep, Duration};
use tracing::{info, error};
use tracing_subscriber;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting CPL integration example...");

    // Create CPL configuration with audio enabled
    let mut cpl_config = CPLConfig::default();
    cpl_config.enable_audio = true;
    
    // Configure audio settings via JSON
    cpl_config.audio_config = Some(json!({
        "enabled": true,
        "sample_rate": 44100,
        "channels": 1,
        "buffer_size": 4096,
        "capture": {
            "noise_reduction": true,
            "agc": true,
            "low_latency": true,
            "buffer_strategy": "ring",
            "ring_buffer_size": 8192
        },
        "analysis": {
            "enable_fft": true,
            "enable_energy": true,
            "enable_zcr": true,
            "enable_spectral": true,
            "enable_pitch": true,
            "fft_window_size": 2048,
            "analysis_interval_ms": 100
        },
        "enable_llm_vtt": false
    }));

    info!("CPL configuration created with audio enabled");

    // Extract audio config from CPL config
    let audio_config = audio_config_from_cpl(&cpl_config);
    
    match audio_config {
        Some(config) => {
            info!("Audio configuration extracted from CPL");
            info!("  Enabled: {}", config.enabled);
            info!("  Sample rate: {} Hz", config.sample_rate);
            info!("  Channels: {}", config.channels);
            info!("  Buffer size: {}", config.buffer_size);
            
            // Create audio adapter from CPL config
            match create_audio_adapter_from_cpl(&cpl_config) {
                Ok(Some(adapter)) => {
                    info!("Audio adapter created from CPL config");
                    
                    // Create world broker
                    let mut world_broker = WorldBroker::new();
                    
                    // Register audio adapter
                    world_broker.register_adapter(Box::new(adapter))
                        .map_err(|e| format!("Failed to register adapter: {}", e))?;
                    
                    info!("Audio adapter registered with world broker");
                    
                    // Start world broker
                    world_broker.start().await
                        .map_err(|e| format!("Failed to start world broker: {}", e))?;
                    
                    info!("World broker started. Listening for audio events...");
                    
                    // Subscribe to events
                    let mut event_receiver = world_broker.subscribe_events();
                    
                    // Handle events for 10 seconds
                    let start = std::time::Instant::now();
                    while start.elapsed() < Duration::from_secs(10) {
                        tokio::select! {
                            event_result = event_receiver.recv() => {
                                match event_result {
                                    Ok(event) => {
                                        match event {
                                            narayana_wld::WorldEvent::SensorData { source, data, .. } if source == "audio" => {
                                                info!("Audio event received");
                                                
                                                // Process audio analysis data
                                                if let Some(analysis) = data.get("analysis") {
                                                    if let Some(energy) = analysis.get("energy").and_then(|v| v.as_f64()) {
                                                        if energy > 0.01 {
                                                            info!("  Energy: {:.4}", energy);
                                                        }
                                                    }
                                                    
                                                    if let Some(pitch) = analysis.get("pitch").and_then(|v| v.as_f64()) {
                                                        info!("  Pitch: {:.2} Hz", pitch);
                                                    }
                                                }
                                                
                                                // Process voice-to-text
                                                if let Some(text) = data.get("text") {
                                                    if let Some(text_str) = text.as_str() {
                                                        info!("  Voice: {}", text_str);
                                                    }
                                                }
                                            }
                                            _ => {
                                                // Ignore other events
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        // Channel closed or lagged
                                        break;
                                    }
                                }
                            }
                            _ = sleep(Duration::from_millis(100)) => {
                                // Timeout for select
                            }
                        }
                    }
                    
                    // Stop world broker
                    world_broker.stop().await
                        .map_err(|e| format!("Failed to stop world broker: {}", e))?;
                    
                    info!("CPL integration example completed");
                }
                Ok(None) => {
                    info!("Audio adapter not created (audio disabled or invalid config)");
                }
                Err(e) => {
                    error!("Failed to create audio adapter: {}", e);
                    return Err(e.into());
                }
            }
        }
        None => {
            info!("No audio configuration extracted (audio disabled in CPL)");
        }
    }

    Ok(())
}

