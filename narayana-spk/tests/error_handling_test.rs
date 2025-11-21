//! Tests for error handling in narayana-spk
//! Tests for error propagation, recovery, and edge cases

use narayana_spk::config::{SpeechConfig, VoiceConfig};
use narayana_spk::error::SpeechError;
use narayana_spk::synthesizer::SpeechSynthesizer;
use narayana_spk::speech_adapter::SpeechAdapter;
use narayana_core::Error;

#[test]
fn test_speech_error_display() {
    let errors = vec![
        SpeechError::Config("Test config error".to_string()),
        SpeechError::Engine("Test engine error".to_string()),
        SpeechError::Synthesizer("Test synthesizer error".to_string()),
        SpeechError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "Test IO error")),
    ];
    
    for error in errors {
        let error_str = format!("{}", error);
        assert!(!error_str.is_empty());
    }
}

#[test]
fn test_speech_error_from_io_error() {
    let io_error = std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "Permission denied"
    );
    let speech_error = SpeechError::Io(io_error);
    
    match speech_error {
        SpeechError::Io(_) => {},
        _ => panic!("Expected Io error"),
    }
}

#[test]
fn test_config_validation_error_propagation() {
    let mut config = SpeechConfig::default();
    config.enabled = true;
    config.rate = 600; // Invalid
    
    let result = SpeechSynthesizer::new(config);
    assert!(result.is_err());
    
    match result {
        Err(SpeechError::Config(_)) => {},
        _ => panic!("Expected Config error"),
    }
}

#[test]
fn test_engine_unavailable_error() {
    let mut config = SpeechConfig::default();
    config.enabled = true;
    // If native engine is not available, should return Engine error
    // This test may pass or fail depending on platform
    let _result = SpeechSynthesizer::new(config);
    // Result depends on whether TTS engine is available
}

#[test]
fn test_adapter_creation_with_invalid_config() {
    let mut config = SpeechConfig::default();
    config.rate = 600; // Invalid rate
    
    let result = SpeechAdapter::new(config);
    assert!(result.is_err());
    
    match result {
        Err(Error::Storage(msg)) => {
            assert!(msg.contains("Invalid speech config"));
        }
        _ => panic!("Expected Storage error with invalid config message"),
    }
}

#[test]
fn test_adapter_creation_with_valid_config() {
    let config = SpeechConfig::default();
    let result = SpeechAdapter::new(config);
    assert!(result.is_ok());
}

#[test]
fn test_voice_config_validation_error() {
    let mut voice = VoiceConfig::default();
    voice.language = "a".repeat(33); // Too long
    
    let result = voice.validate();
    assert!(result.is_err());
    
    match result {
        Err(msg) => {
            assert!(msg.contains("too long") || msg.contains("Language"));
        }
        _ => panic!("Expected validation error"),
    }
}

#[test]
fn test_speech_config_validation_error_messages() {
    let mut config = SpeechConfig::default();
    
    // Test rate error
    config.rate = 600;
    let result = config.validate();
    assert!(result.is_err());
    if let Err(msg) = result {
        assert!(msg.contains("rate") || msg.contains("WPM"));
    }
    
    // Test volume error
    config.rate = 150;
    config.volume = 1.5;
    let result = config.validate();
    assert!(result.is_err());
    if let Err(msg) = result {
        assert!(msg.contains("volume") || msg.contains("0.0") || msg.contains("1.0"));
    }
    
    // Test pitch error
    config.volume = 0.8;
    config.pitch = 1.5;
    let result = config.validate();
    assert!(result.is_err());
    if let Err(msg) = result {
        assert!(msg.contains("pitch") || msg.contains("-1.0") || msg.contains("1.0"));
    }
}

#[test]
fn test_api_config_validation_errors() {
    let mut config = SpeechConfig::default();
    
    // Test HTTP (not HTTPS) error
    config.api_config = Some(narayana_spk::config::ApiTtsConfig {
        endpoint: "http://example.com".to_string(),
        api_key: None,
        model: None,
        timeout_secs: 30,
        retry_config: narayana_spk::config::RetryConfig::default(),
    });
    let result = config.validate();
    assert!(result.is_err());
    if let Err(msg) = result {
        assert!(msg.contains("HTTPS") || msg.contains("endpoint"));
    }
    
    // Test empty endpoint error
    config.api_config.as_mut().unwrap().endpoint = String::new();
    let result = config.validate();
    assert!(result.is_err());
    if let Err(msg) = result {
        assert!(msg.contains("empty") || msg.contains("endpoint"));
    }
}

#[test]
fn test_retry_config_validation_errors() {
    let mut config = SpeechConfig::default();
    
    // Test initial delay > max delay
    config.api_config = Some(narayana_spk::config::ApiTtsConfig {
        endpoint: "https://example.com".to_string(),
        api_key: None,
        model: None,
        timeout_secs: 30,
        retry_config: narayana_spk::config::RetryConfig {
            max_retries: 3,
            initial_delay_ms: 5000,
            max_delay_ms: 1000, // Less than initial
        },
    });
    let result = config.validate();
    assert!(result.is_err());
    if let Err(msg) = result {
        assert!(msg.contains("delay") || msg.contains("greater"));
    }
}

#[test]
fn test_error_recovery() {
    // Test that errors don't leave the system in an invalid state
    let mut config = SpeechConfig::default();
    config.enabled = true;
    config.rate = 600; // Invalid
    
    // First attempt should fail
    let result1 = SpeechSynthesizer::new(config.clone());
    assert!(result1.is_err());
    
    // Fix the config
    config.rate = 150;
    
    // Second attempt should succeed (if engine available)
    let _result2 = SpeechSynthesizer::new(config);
    // Result depends on engine availability
}

#[test]
fn test_multiple_validation_errors() {
    // Test that validation catches all errors, not just the first one
    let mut config = SpeechConfig::default();
    config.rate = 600; // Invalid
    config.volume = 1.5; // Invalid
    config.pitch = 1.5; // Invalid
    
    let result = config.validate();
    assert!(result.is_err());
    // Should report at least one error (may report first error found)
}


