//! Tests for speech configuration

use narayana_spk::config::{SpeechConfig, VoiceConfig, TtsEngine, ApiTtsConfig, RetryConfig};

#[test]
fn test_speech_config_default() {
    let config = SpeechConfig::default();
    assert!(!config.enabled);
    assert_eq!(config.engine, TtsEngine::Native);
    assert_eq!(config.rate, 150);
    assert_eq!(config.volume, 0.8);
    assert_eq!(config.pitch, 0.0);
}

#[test]
fn test_speech_config_validation_rate() {
    let mut config = SpeechConfig::default();
    config.rate = 600; // Too high
    assert!(config.validate().is_err());
    
    config.rate = 0;
    assert!(config.validate().is_ok());
    
    config.rate = 500;
    assert!(config.validate().is_ok());
}

#[test]
fn test_speech_config_validation_volume() {
    let mut config = SpeechConfig::default();
    config.volume = 1.5; // Too high
    assert!(config.validate().is_err());
    
    config.volume = -0.1; // Too low
    assert!(config.validate().is_err());
    
    config.volume = 0.5;
    assert!(config.validate().is_ok());
}

#[test]
fn test_speech_config_validation_pitch() {
    let mut config = SpeechConfig::default();
    config.pitch = 1.5; // Too high
    assert!(config.validate().is_err());
    
    config.pitch = -1.5; // Too low
    assert!(config.validate().is_err());
    
    config.pitch = 0.0;
    assert!(config.validate().is_ok());
}

#[test]
fn test_speech_config_validation_api_endpoint() {
    let mut config = SpeechConfig::default();
    config.api_config = Some(ApiTtsConfig {
        endpoint: "http://example.com".to_string(), // Not HTTPS
        api_key: None,
        model: None,
        timeout_secs: 30,
        retry_config: RetryConfig::default(),
    });
    assert!(config.validate().is_err());
    
    config.api_config.as_mut().unwrap().endpoint = "https://example.com".to_string();
    assert!(config.validate().is_ok());
}

#[test]
fn test_voice_config_default() {
    let voice = VoiceConfig::default();
    assert_eq!(voice.language, "en-US");
    assert!(voice.name.is_none());
    assert!(voice.gender.is_none());
    assert!(voice.age.is_none());
}


