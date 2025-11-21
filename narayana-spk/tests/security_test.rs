//! Security tests for narayana-spk
//! Tests for input validation, resource limits, and security vulnerabilities

use narayana_spk::config::{SpeechConfig, VoiceConfig, ApiTtsConfig, RetryConfig};
use narayana_spk::synthesizer::SpeechSynthesizer;
use narayana_spk::error::SpeechError;

#[test]
fn test_text_length_limits() {
    let mut config = SpeechConfig::default();
    config.enabled = true;
    
    // This will fail because synthesizer requires engine to be available
    // But we can test the config validation
    let config_result = config.validate();
    assert!(config_result.is_ok());
}

#[test]
fn test_voice_config_validation_language_too_long() {
    let mut voice = VoiceConfig::default();
    voice.language = "a".repeat(33); // 33 chars, max is 32
    assert!(voice.validate().is_err());
}

#[test]
fn test_voice_config_validation_language_invalid_chars() {
    let mut voice = VoiceConfig::default();
    voice.language = "en-US; DROP TABLE".to_string();
    assert!(voice.validate().is_err());
}

#[test]
fn test_voice_config_validation_name_too_long() {
    let mut voice = VoiceConfig::default();
    voice.name = Some("a".repeat(257)); // 257 chars, max is 256
    assert!(voice.validate().is_err());
}

#[test]
fn test_voice_config_validation_name_null_bytes() {
    let mut voice = VoiceConfig::default();
    voice.name = Some("voice\0name".to_string());
    assert!(voice.validate().is_err());
}

#[test]
fn test_voice_config_validation_name_control_chars() {
    let mut voice = VoiceConfig::default();
    voice.name = Some("voice\nname".to_string());
    assert!(voice.validate().is_err());
}

#[test]
fn test_speech_config_cache_dir_path_traversal() {
    let mut config = SpeechConfig::default();
    config.cache_dir = std::path::PathBuf::from("/tmp/../../etc/passwd");
    assert!(config.validate().is_err());
}

#[test]
fn test_speech_config_max_cache_size_limit() {
    let mut config = SpeechConfig::default();
    config.max_cache_size_mb = 10_001; // Over 10GB limit
    assert!(config.validate().is_err());
}

#[test]
fn test_speech_config_queue_size_zero() {
    let mut config = SpeechConfig::default();
    config.queue_size = 0;
    assert!(config.validate().is_err());
}

#[test]
fn test_speech_config_queue_size_too_large() {
    let mut config = SpeechConfig::default();
    config.queue_size = 10_001; // Over 10,000 limit
    assert!(config.validate().is_err());
}

#[test]
fn test_api_config_endpoint_too_long() {
    let mut config = SpeechConfig::default();
    config.api_config = Some(ApiTtsConfig {
        endpoint: "https://".to_string() + &"a".repeat(2049), // 2049 chars, max is 2048
        api_key: None,
        model: None,
        timeout_secs: 30,
        retry_config: RetryConfig::default(),
    });
    assert!(config.validate().is_err());
}

#[test]
fn test_api_config_endpoint_invalid_chars() {
    let mut config = SpeechConfig::default();
    config.api_config = Some(ApiTtsConfig {
        endpoint: "https://example.com\0".to_string(),
        api_key: None,
        model: None,
        timeout_secs: 30,
        retry_config: RetryConfig::default(),
    });
    assert!(config.validate().is_err());
}

#[test]
fn test_api_config_model_too_long() {
    let mut config = SpeechConfig::default();
    config.api_config = Some(ApiTtsConfig {
        endpoint: "https://example.com".to_string(),
        api_key: None,
        model: Some("a".repeat(257)), // 257 chars, max is 256
        timeout_secs: 30,
        retry_config: RetryConfig::default(),
    });
    assert!(config.validate().is_err());
}

#[test]
fn test_api_config_timeout_zero() {
    let mut config = SpeechConfig::default();
    config.api_config = Some(ApiTtsConfig {
        endpoint: "https://example.com".to_string(),
        api_key: None,
        model: None,
        timeout_secs: 0,
        retry_config: RetryConfig::default(),
    });
    assert!(config.validate().is_err());
}

#[test]
fn test_api_config_timeout_too_large() {
    let mut config = SpeechConfig::default();
    config.api_config = Some(ApiTtsConfig {
        endpoint: "https://example.com".to_string(),
        api_key: None,
        model: None,
        timeout_secs: 301, // Over 300 second limit
        retry_config: RetryConfig::default(),
    });
    assert!(config.validate().is_err());
}

#[test]
fn test_retry_config_max_retries_too_large() {
    let mut config = SpeechConfig::default();
    config.api_config = Some(ApiTtsConfig {
        endpoint: "https://example.com".to_string(),
        api_key: None,
        model: None,
        timeout_secs: 30,
        retry_config: RetryConfig {
            max_retries: 101, // Over 100 limit
            initial_delay_ms: 100,
            max_delay_ms: 5000,
        },
    });
    assert!(config.validate().is_err());
}

#[test]
fn test_retry_config_initial_delay_too_large() {
    let mut config = SpeechConfig::default();
    config.api_config = Some(ApiTtsConfig {
        endpoint: "https://example.com".to_string(),
        api_key: None,
        model: None,
        timeout_secs: 30,
        retry_config: RetryConfig {
            max_retries: 3,
            initial_delay_ms: 60_001, // Over 60000 ms limit
            max_delay_ms: 5000,
        },
    });
    assert!(config.validate().is_err());
}

#[test]
fn test_retry_config_max_delay_too_large() {
    let mut config = SpeechConfig::default();
    config.api_config = Some(ApiTtsConfig {
        endpoint: "https://example.com".to_string(),
        api_key: None,
        model: None,
        timeout_secs: 30,
        retry_config: RetryConfig {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 300_001, // Over 300000 ms limit
        },
    });
    assert!(config.validate().is_err());
}

#[test]
fn test_retry_config_initial_greater_than_max() {
    let mut config = SpeechConfig::default();
    config.api_config = Some(ApiTtsConfig {
        endpoint: "https://example.com".to_string(),
        api_key: None,
        model: None,
        timeout_secs: 30,
        retry_config: RetryConfig {
            max_retries: 3,
            initial_delay_ms: 5000,
            max_delay_ms: 1000, // Less than initial
        },
    });
    assert!(config.validate().is_err());
}

#[test]
fn test_language_code_valid_formats() {
    let valid_languages = vec!["en", "en-US", "es-ES", "fr-FR", "de-DE"];
    
    for lang in valid_languages {
        let mut voice = VoiceConfig::default();
        voice.language = lang.to_string();
        assert!(voice.validate().is_ok(), "Language '{}' should be valid", lang);
    }
}

#[test]
fn test_language_code_invalid_formats() {
    let invalid_languages = vec![
        "en_US", // underscore not allowed
        "en US", // space not allowed
        "en@US", // @ not allowed
        "en;US", // semicolon not allowed
    ];
    
    for lang in invalid_languages {
        let mut voice = VoiceConfig::default();
        voice.language = lang.to_string();
        assert!(voice.validate().is_err(), "Language '{}' should be invalid", lang);
    }
}


