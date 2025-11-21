//! Integration tests for narayana-spk

use narayana_spk::{SpeechConfig, SpeechAdapter, SpeechSynthesizer};
use narayana_spk::config::{TtsEngine, VoiceConfig};
use narayana_wld::world_broker::WorldBroker;
use narayana_wld::protocol_adapters::ProtocolAdapter;
use narayana_wld::event_transformer::WorldAction;
use narayana_storage::cognitive::CognitiveBrain;
use narayana_storage::conscience_persistent_loop::ConsciencePersistentLoop;
use narayana_wld::config::WorldBrokerConfig;
use std::sync::Arc;
use serde_json::json;

#[tokio::test]
async fn test_speech_config_default() {
    let config = SpeechConfig::default();
    assert!(!config.enabled); // Off by default
    assert_eq!(config.engine, TtsEngine::Native);
    assert_eq!(config.rate, 150);
    assert!(config.validate().is_ok());
}

#[tokio::test]
async fn test_speech_config_validation() {
    let mut config = SpeechConfig::default();
    config.enabled = true;
    config.rate = 600; // Too high
    assert!(config.validate().is_err());
    
    config.rate = 150;
    config.volume = 1.5; // Too high
    assert!(config.validate().is_err());
    
    config.volume = 0.8;
    assert!(config.validate().is_ok());
}

#[tokio::test]
async fn test_speech_adapter_creation() {
    let config = SpeechConfig::default();
    let adapter = SpeechAdapter::new(config);
    assert!(adapter.is_ok());
}

#[tokio::test]
async fn test_speech_adapter_protocol_name() {
    let config = SpeechConfig::default();
    let adapter = SpeechAdapter::new(config).unwrap();
    assert_eq!(adapter.protocol_name(), "speech");
}

#[tokio::test]
async fn test_speech_adapter_start_stop() {
    let config = SpeechConfig::default();
    let adapter = SpeechAdapter::new(config).unwrap();
    
    // Create a minimal broker handle for testing using WorldBroker
    // This ensures we have a properly constructed handle
    let brain = Arc::new(CognitiveBrain::new());
    let cpl = Arc::new(ConsciencePersistentLoop::new(brain.clone(), Default::default()));
    let broker_config = WorldBrokerConfig::default();
    let _broker = WorldBroker::new(brain.clone(), cpl.clone(), broker_config).unwrap();
    
    // Create handle manually by accessing broker internals
    // Since fields are private, we'll use a workaround: create handle from broker's start method
    // For now, just test that stop works without requiring start
    let result = adapter.stop().await;
    assert!(result.is_ok());
    
    // Note: Full start/stop test would require a public constructor for WorldBrokerHandle
    // or access to broker's internal handle creation
}

#[tokio::test]
async fn test_voice_config_default() {
    let voice = VoiceConfig::default();
    assert_eq!(voice.language, "en-US");
    assert!(voice.name.is_none());
}

#[tokio::test]
async fn test_speech_adapter_send_action_disabled() {
    let config = SpeechConfig::default(); // disabled by default
    let adapter = SpeechAdapter::new(config).unwrap();
    
    let action = WorldAction::ActuatorCommand {
        target: "speech".to_string(),
        command: json!({"text": "Hello"}),
    };
    
    // Should not error even if disabled
    let result = adapter.send_action(action).await;
    assert!(result.is_ok());
}

