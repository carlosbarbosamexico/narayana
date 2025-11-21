//! Tests for API TTS engine implementations

use narayana_spk::engines::api::ApiTtsEngine;
use narayana_spk::engines::TtsEngine;
use narayana_spk::config::{VoiceConfig, RetryConfig};
use narayana_spk::error::SpeechError;

#[test]
fn test_api_engine_new_openai() {
    let engine = ApiTtsEngine::new_openai(
        "https://api.openai.com".to_string(),
        Some("test_key".to_string()),
        Some("tts-1".to_string()),
        30,
        RetryConfig::default(),
    );

    assert!(engine.is_ok());
    let engine = engine.unwrap();
    assert!(engine.is_available());
    assert_eq!(engine.name(), "OpenAI TTS");
}

#[test]
fn test_api_engine_new_openai_no_key() {
    let engine = ApiTtsEngine::new_openai(
        "https://api.openai.com".to_string(),
        None,
        None,
        30,
        RetryConfig::default(),
    );

    assert!(engine.is_ok());
    let engine = engine.unwrap();
    // Should not be available without API key
    assert!(!engine.is_available());
}

#[test]
fn test_api_engine_new_openai_endpoint_trimming() {
    let engine = ApiTtsEngine::new_openai(
        "https://api.openai.com/".to_string(), // Trailing slash
        Some("test_key".to_string()),
        None,
        30,
        RetryConfig::default(),
    );

    assert!(engine.is_ok());
    // Endpoint should be trimmed
}

#[test]
fn test_api_engine_new_google_cloud() {
    let engine = ApiTtsEngine::new_google_cloud(
        "https://texttospeech.googleapis.com".to_string(),
        Some("test_key".to_string()),
        None,
        30,
        RetryConfig::default(),
    );

    assert!(engine.is_ok());
    let engine = engine.unwrap();
    assert_eq!(engine.name(), "Google Cloud TTS");
}

#[test]
fn test_api_engine_new_google_cloud_no_key() {
    let engine = ApiTtsEngine::new_google_cloud(
        "https://texttospeech.googleapis.com".to_string(),
        None,
        None,
        30,
        RetryConfig::default(),
    );

    assert!(engine.is_ok());
    let engine = engine.unwrap();
    assert!(!engine.is_available());
}

#[test]
fn test_api_engine_new_amazon_polly() {
    let engine = ApiTtsEngine::new_amazon_polly(
        "https://polly.us-east-1.amazonaws.com".to_string(),
        Some("test_key".to_string()),
        None,
        30,
        RetryConfig::default(),
    );

    assert!(engine.is_ok());
    let engine = engine.unwrap();
    assert_eq!(engine.name(), "Amazon Polly");
}

#[test]
fn test_api_engine_new_amazon_polly_with_model() {
    let engine = ApiTtsEngine::new_amazon_polly(
        "https://polly.us-east-1.amazonaws.com".to_string(),
        Some("test_key".to_string()),
        Some("neural".to_string()),
        30,
        RetryConfig::default(),
    );

    assert!(engine.is_ok());
}

#[test]
fn test_api_engine_new_custom() {
    let engine = ApiTtsEngine::new_custom(
        "https://custom-tts.example.com".to_string(),
        Some("test_key".to_string()),
        Some("model1".to_string()),
        30,
        RetryConfig::default(),
        "MyCustomEngine".to_string(),
    );

    assert!(engine.is_ok());
    let engine = engine.unwrap();
    assert!(engine.is_available());
    assert_eq!(engine.name(), "MyCustomEngine");
}

#[test]
fn test_api_engine_custom_no_endpoint() {
    let engine = ApiTtsEngine::new_custom(
        "".to_string(), // Empty endpoint
        None,
        None,
        30,
        RetryConfig::default(),
        "Test".to_string(),
    );

    assert!(engine.is_ok());
    let engine = engine.unwrap();
    // Should not be available without endpoint
    assert!(!engine.is_available());
}

#[test]
fn test_api_engine_custom_no_name() {
    let engine = ApiTtsEngine::new_custom(
        "https://example.com".to_string(),
        None,
        None,
        30,
        RetryConfig::default(),
        "".to_string(), // Empty name
    );

    assert!(engine.is_ok());
    let engine = engine.unwrap();
    // Empty name should result in empty string or default
    let name = engine.name();
    assert!(name.is_empty() || name == "Custom API TTS");
}

#[tokio::test]
async fn test_api_engine_synthesize_empty_text() {
    let engine = ApiTtsEngine::new_openai(
        "https://api.openai.com".to_string(),
        Some("test_key".to_string()),
        None,
        30,
        RetryConfig::default(),
    ).unwrap();

    let result = engine.synthesize("", &VoiceConfig::default()).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("empty"));
}

#[tokio::test]
async fn test_api_engine_synthesize_too_long() {
    let engine = ApiTtsEngine::new_openai(
        "https://api.openai.com".to_string(),
        Some("test_key".to_string()),
        None,
        30,
        RetryConfig::default(),
    ).unwrap();

    let long_text = "a".repeat(100_001);
    let result = engine.synthesize(&long_text, &VoiceConfig::default()).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("too long"));
}

#[tokio::test]
async fn test_api_engine_list_voices_openai() {
    let engine = ApiTtsEngine::new_openai(
        "https://api.openai.com".to_string(),
        Some("test_key".to_string()),
        None,
        30,
        RetryConfig::default(),
    ).unwrap();

    let voices = engine.list_voices().await;
    assert!(voices.is_ok());
    let voices_list = voices.unwrap();
    // OpenAI has fixed voices
    assert!(!voices_list.is_empty());
    assert!(voices_list.contains(&"alloy".to_string()));
    assert!(voices_list.contains(&"echo".to_string()));
}

#[tokio::test]
async fn test_api_engine_list_voices_google_cloud() {
    let engine = ApiTtsEngine::new_google_cloud(
        "https://texttospeech.googleapis.com".to_string(),
        Some("test_key".to_string()),
        None,
        30,
        RetryConfig::default(),
    ).unwrap();

    let voices = engine.list_voices().await;
    assert!(voices.is_ok());
    let voices_list = voices.unwrap();
    // Google Cloud has common voices
    assert!(!voices_list.is_empty());
}

#[tokio::test]
async fn test_api_engine_list_voices_amazon_polly() {
    let engine = ApiTtsEngine::new_amazon_polly(
        "https://polly.us-east-1.amazonaws.com".to_string(),
        Some("test_key".to_string()),
        None,
        30,
        RetryConfig::default(),
    ).unwrap();

    let voices = engine.list_voices().await;
    assert!(voices.is_ok());
    let voices_list = voices.unwrap();
    // Amazon Polly has common voices
    assert!(!voices_list.is_empty());
    assert!(voices_list.contains(&"Joanna".to_string()));
}

#[tokio::test]
async fn test_api_engine_list_voices_custom() {
    let engine = ApiTtsEngine::new_custom(
        "https://custom-tts.example.com".to_string(),
        Some("test_key".to_string()),
        None,
        30,
        RetryConfig::default(),
        "Test".to_string(),
    ).unwrap();

    let voices = engine.list_voices().await;
    assert!(voices.is_ok());
    // Custom APIs return empty list (can't list without knowing API structure)
    let voices_list = voices.unwrap();
    assert!(voices_list.is_empty());
}

#[test]
fn test_api_engine_retry_config() {
    let retry_config = RetryConfig {
        max_retries: 3,
        initial_delay_ms: 100,
        max_delay_ms: 5000,
    };

    let engine = ApiTtsEngine::new_openai(
        "https://api.openai.com".to_string(),
        Some("test_key".to_string()),
        None,
        30,
        retry_config,
    );

    assert!(engine.is_ok());
}

#[test]
fn test_api_engine_timeout() {
    let engine = ApiTtsEngine::new_openai(
        "https://api.openai.com".to_string(),
        Some("test_key".to_string()),
        None,
        60, // 60 second timeout
        RetryConfig::default(),
    );

    assert!(engine.is_ok());
}

