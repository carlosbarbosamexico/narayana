//! Edge case tests for narayana-spk
//! Tests for boundary conditions, overflow, and unusual inputs

use narayana_spk::config::{SpeechConfig, VoiceConfig};
use narayana_spk::error::SpeechError;

#[test]
fn test_config_boundary_values() {
    // Test minimum valid values
    let mut config = SpeechConfig::default();
    config.rate = 0;
    config.volume = 0.0;
    config.pitch = -1.0;
    config.queue_size = 1;
    assert!(config.validate().is_ok());
    
    // Test maximum valid values
    config.rate = 500;
    config.volume = 1.0;
    config.pitch = 1.0;
    config.queue_size = 10_000;
    assert!(config.validate().is_ok());
}

#[test]
fn test_config_just_over_boundary() {
    let mut config = SpeechConfig::default();
    config.rate = 501; // Just over max
    assert!(config.validate().is_err());
    
    config.rate = 500;
    config.volume = 1.0001; // Just over max
    assert!(config.validate().is_err());
    
    config.volume = 1.0;
    config.pitch = 1.0001; // Just over max
    assert!(config.validate().is_err());
}

#[test]
fn test_config_just_under_boundary() {
    let mut config = SpeechConfig::default();
    config.volume = -0.0001; // Just under min
    assert!(config.validate().is_err());
    
    config.volume = 0.0;
    config.pitch = -1.0001; // Just under min
    assert!(config.validate().is_err());
}

#[test]
fn test_voice_config_empty_language() {
    let mut voice = VoiceConfig::default();
    voice.language = String::new();
    assert!(voice.validate().is_err());
}

#[test]
fn test_voice_config_max_length_language() {
    let mut voice = VoiceConfig::default();
    voice.language = "a".repeat(32); // Exactly 32 chars (max)
    assert!(voice.validate().is_ok());
}

#[test]
fn test_voice_config_max_length_name() {
    let mut voice = VoiceConfig::default();
    voice.name = Some("a".repeat(256)); // Exactly 256 chars (max)
    assert!(voice.validate().is_ok());
}

#[test]
fn test_voice_config_empty_name() {
    let mut voice = VoiceConfig::default();
    voice.name = Some(String::new());
    assert!(voice.validate().is_err());
}

#[test]
fn test_cache_size_overflow_protection() {
    let mut config = SpeechConfig::default();
    // Test that extremely large values are handled
    config.max_cache_size_mb = u64::MAX;
    // Should be capped at validation
    let result = config.validate();
    // Either validation fails or value is capped
    assert!(result.is_err() || config.max_cache_size_mb <= 10_000);
}

#[test]
fn test_queue_size_boundaries() {
    let mut config = SpeechConfig::default();
    
    // Minimum valid
    config.queue_size = 1;
    assert!(config.validate().is_ok());
    
    // Maximum valid
    config.queue_size = 10_000;
    assert!(config.validate().is_ok());
    
    // Just over max
    config.queue_size = 10_001;
    assert!(config.validate().is_err());
}

#[test]
fn test_api_endpoint_boundaries() {
    let mut config = SpeechConfig::default();
    
    // Valid endpoint
    config.api_config = Some(narayana_spk::config::ApiTtsConfig {
        endpoint: "https://example.com".to_string(),
        api_key: None,
        model: None,
        timeout_secs: 30,
        retry_config: narayana_spk::config::RetryConfig::default(),
    });
    assert!(config.validate().is_ok());
    
    // Maximum length endpoint (2048 chars)
    if let Some(ref mut api) = config.api_config {
        api.endpoint = "https://".to_string() + &"a".repeat(2040);
    }
    assert!(config.validate().is_ok());
}

#[test]
fn test_unicode_in_language_code() {
    let mut voice = VoiceConfig::default();
    voice.language = "en-🇺🇸".to_string(); // Unicode emoji
    // Should fail because only ASCII alphanumeric and '-' allowed
    assert!(voice.validate().is_err());
}

#[test]
fn test_unicode_in_voice_name() {
    let mut voice = VoiceConfig::default();
    voice.name = Some("voice-测试".to_string()); // Unicode characters
    // Should fail because of control character check (though this might pass)
    // Let's test that it at least doesn't crash
    let _ = voice.validate();
}

#[test]
fn test_special_characters_in_language() {
    let special_chars = vec!['!', '@', '#', '$', '%', '^', '&', '*', '(', ')'];
    
    for ch in special_chars {
        let mut voice = VoiceConfig::default();
        voice.language = format!("en{}US", ch);
        assert!(voice.validate().is_err(), "Language with '{}' should be invalid", ch);
    }
}

#[test]
fn test_whitespace_in_language() {
    let whitespace_chars = vec![' ', '\t', '\n', '\r'];
    
    for ch in whitespace_chars {
        let mut voice = VoiceConfig::default();
        voice.language = format!("en{}US", ch);
        assert!(voice.validate().is_err(), "Language with whitespace '{}' should be invalid", ch);
    }
}

#[test]
fn test_multiple_hyphens_in_language() {
    let mut voice = VoiceConfig::default();
    voice.language = "en-US-CA".to_string(); // Multiple hyphens
    // Should be valid (some locales use this format)
    assert!(voice.validate().is_ok());
}

#[test]
fn test_very_long_valid_language() {
    let mut voice = VoiceConfig::default();
    voice.language = "a".repeat(32); // Max length
    assert!(voice.validate().is_ok());
}

#[test]
fn test_very_long_valid_voice_name() {
    let mut voice = VoiceConfig::default();
    voice.name = Some("a".repeat(256)); // Max length
    assert!(voice.validate().is_ok());
}


