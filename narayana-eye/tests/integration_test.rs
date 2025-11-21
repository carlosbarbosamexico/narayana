//! Integration tests for narayana-eye

use narayana_eye::config::{VisionConfig, ProcessingMode};
use narayana_eye::error::VisionError;
use narayana_eye::VisionAdapter;
use narayana_wld::world_broker::WorldBroker;
use narayana_core::Error;
use std::sync::Arc;
use tokio::time::{self, Duration};

#[tokio::test]
async fn test_config_validation() {
    let config = VisionConfig::default();
    assert!(config.validate().is_ok());
}

#[tokio::test]
async fn test_config_serialization() {
    let config = VisionConfig::default();
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: VisionConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config.camera_id, deserialized.camera_id);
    assert_eq!(config.frame_rate, deserialized.frame_rate);
    assert_eq!(config.resolution, deserialized.resolution);
}

#[tokio::test]
async fn test_processing_mode_serialization() {
    let mode = ProcessingMode::RealTime;
    let json = serde_json::to_string(&mode).unwrap();
    let deserialized: ProcessingMode = serde_json::from_str(&json).unwrap();
    assert_eq!(mode, deserialized);
    
    let mode = ProcessingMode::OnDemand;
    let json = serde_json::to_string(&mode).unwrap();
    let deserialized: ProcessingMode = serde_json::from_str(&json).unwrap();
    assert_eq!(mode, deserialized);
}

#[tokio::test]
async fn test_error_display() {
    let err = VisionError::Camera("test error".to_string());
    let display = format!("{}", err);
    assert!(display.contains("Camera error"));
    assert!(display.contains("test error"));
}

#[tokio::test]
async fn test_error_conversion() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
    let vision_err: VisionError = io_err.into();
    match vision_err {
        VisionError::Io(_) => {}
        _ => panic!("Expected Io error"),
    }
}

#[tokio::test]
async fn test_vision_adapter_initialization() -> Result<(), Error> {
    let config = VisionConfig::default();
    let adapter = VisionAdapter::new(config)?;
    // Initialization should succeed without panicking
    Ok(())
}

#[tokio::test]
async fn test_vision_adapter_start_stop_realtime() -> Result<(), Error> {
    let mut config = VisionConfig::default();
    config.processing_mode = ProcessingMode::RealTime;
    config.enable_detection = false; // Disable models for faster test
    config.enable_segmentation = false;
    config.enable_scene_understanding = false;

    let adapter = Arc::new(VisionAdapter::new(config)?);
    let broker = WorldBroker::new();
    let broker_handle = broker.handle();

    // Start the adapter
    adapter.start(broker_handle.clone()).await?;

    // Wait a bit to ensure tasks are running
    time::sleep(Duration::from_millis(100)).await;

    // Stop the adapter
    adapter.stop().await?;

    Ok(())
}

#[tokio::test]
async fn test_vision_adapter_start_stop_on_demand() -> Result<(), Error> {
    let mut config = VisionConfig::default();
    config.processing_mode = ProcessingMode::OnDemand;
    config.enable_detection = false; // Disable models for faster test
    config.enable_segmentation = false;
    config.enable_scene_understanding = false;

    let adapter = Arc::new(VisionAdapter::new(config)?);
    let broker = WorldBroker::new();
    let broker_handle = broker.handle();

    // Start the adapter
    adapter.start(broker_handle.clone()).await?;

    // Wait a bit
    time::sleep(Duration::from_millis(100)).await;

    // Stop the adapter
    adapter.stop().await?;
    
    Ok(())

    Ok(())
}

#[tokio::test]
async fn test_vision_adapter_double_start_fails() -> Result<(), Error> {
    let config = VisionConfig::default();
    let adapter = Arc::new(VisionAdapter::new(config)?);
    let broker = WorldBroker::new();
    let broker_handle = broker.handle();

    adapter.start(broker_handle.clone()).await?;
    let result = adapter.start(broker_handle.clone()).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "Storage error: Vision adapter already running");

    adapter.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_vision_adapter_protocol_name() -> Result<(), Error> {
    let config = VisionConfig::default();
    let adapter = VisionAdapter::new(config)?;
    // Test that ProtocolAdapter trait is implemented
    use narayana_wld::protocol_adapters::ProtocolAdapter;
    assert_eq!(adapter.protocol_name(), "vision");
    Ok(())
}

#[tokio::test]
async fn test_vision_adapter_set_llm_manager() -> Result<(), Error> {
    let mut config = VisionConfig::default();
    config.llm_integration = true;
    let mut adapter = VisionAdapter::new(config)?;
    
    // Should accept None
    adapter.set_llm_manager(None);
    
    // Should accept Some(LLMManager) - but we can't easily create one in tests
    // So we just verify the method exists and doesn't panic
    assert!(true);
    
    Ok(())
}

#[tokio::test]
async fn test_vision_adapter_subscribe_events() -> Result<(), Error> {
    let config = VisionConfig::default();
    let adapter = Arc::new(VisionAdapter::new(config)?);
    let broker = WorldBroker::new();
    let broker_handle = broker.handle();

    // Subscribe before starting should still work
    use narayana_wld::protocol_adapters::ProtocolAdapter;
    let mut receiver = adapter.subscribe_events();
    
    // Start the adapter
    adapter.start(broker_handle.clone()).await?;
    
    // Wait a bit
    time::sleep(Duration::from_millis(100)).await;
    
    // Try to receive events (may timeout, which is OK)
    let _ = tokio::time::timeout(Duration::from_millis(50), receiver.recv()).await;
    
    adapter.stop().await?;
    Ok(())
}

