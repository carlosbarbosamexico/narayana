//! Basic example of using narayana-eye

use narayana_eye::{VisionAdapter, VisionConfig, ProcessingMode};
use narayana_wld::{WorldBroker, WorldBrokerConfig};
use narayana_storage::cognitive::CognitiveBrain;
use narayana_storage::conscience_persistent_loop::{ConsciencePersistentLoop, CPLConfig};
use std::sync::Arc;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing (optional)
    // tracing_subscriber::fmt::init();

    // Create cognitive brain and CPL
    let brain = Arc::new(CognitiveBrain::new());
    let cpl_config = CPLConfig::default();
    let cpl = Arc::new(ConsciencePersistentLoop::new(brain.clone(), cpl_config));

    // Create vision configuration
    let vision_config = VisionConfig {
        camera_id: 0,
        frame_rate: 30,
        resolution: (640, 480),
        enable_detection: true,
        enable_segmentation: false,
        enable_tracking: true,
        enable_scene_understanding: true,
        llm_integration: false, // Set to true to enable LLM descriptions
        model_path: PathBuf::from("./models"),
        processing_mode: ProcessingMode::RealTime,
    };

    // Create vision adapter
    let mut vision_adapter = VisionAdapter::new(vision_config)
        .map_err(|e| format!("Failed to create vision adapter: {}", e))?;

    // Optional: Enable LLM integration
    // let llm_manager = Arc::new(LLMManager::new());
    // vision_adapter.set_llm_manager(Some(llm_manager));

    // Create world broker
    let mut config = WorldBrokerConfig::default();
    config.enabled_adapters = vec!["vision".to_string()];
    
    let broker = WorldBroker::new(brain, cpl, config)
        .map_err(|e| format!("Failed to create world broker: {}", e))?;

    // Register vision adapter
    broker.register_adapter(Box::new(vision_adapter));

    // Start broker (this will start vision processing)
    broker.start().await
        .map_err(|e| format!("Failed to start broker: {}", e))?;

    println!("Vision system started! Processing frames...");
    println!("Press Ctrl+C to stop");

    // Keep running
    tokio::signal::ctrl_c().await?;
    
    // Stop broker
    broker.stop().await
        .map_err(|e| format!("Failed to stop broker: {}", e))?;

    println!("Vision system stopped");
    Ok(())
}

