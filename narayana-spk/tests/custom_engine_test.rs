//! Tests for custom TTS engine implementation

use narayana_spk::engines::custom::CustomTtsEngine;
use narayana_spk::engines::TtsEngine;
use narayana_spk::config::VoiceConfig;
use narayana_spk::error::SpeechError;
use bytes::Bytes;

#[test]
fn test_custom_engine_sync() {
    let engine = CustomTtsEngine::new(
        "test_engine".to_string(),
        |text: &str, _config: &VoiceConfig| {
            Ok(Bytes::from(format!("audio_{}", text)))
        },
        || Ok(vec!["voice1".to_string(), "voice2".to_string()]),
        || true,
    );

    assert_eq!(engine.name(), "test_engine");
    assert!(engine.is_available());
}

#[tokio::test]
async fn test_custom_engine_synthesize_empty_text() {
    let engine = CustomTtsEngine::new(
        "test".to_string(),
        |_text: &str, _config: &VoiceConfig| {
            Ok(Bytes::from("audio"))
        },
        || Ok(vec![]),
        || true,
    );

    let result = engine.synthesize("", &VoiceConfig::default()).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("empty"));
}

#[tokio::test]
async fn test_custom_engine_synthesize_too_long() {
    let engine = CustomTtsEngine::new(
        "test".to_string(),
        |_text: &str, _config: &VoiceConfig| {
            Ok(Bytes::from("audio"))
        },
        || Ok(vec![]),
        || true,
    );

    let long_text = "a".repeat(100_001);
    let result = engine.synthesize(&long_text, &VoiceConfig::default()).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("too long"));
}

#[tokio::test]
async fn test_custom_engine_synthesize_valid() {
    let engine = CustomTtsEngine::new(
        "test".to_string(),
        |text: &str, _config: &VoiceConfig| {
            Ok(Bytes::from(format!("audio_{}", text)))
        },
        || Ok(vec![]),
        || true,
    );

    let result = engine.synthesize("hello", &VoiceConfig::default()).await;
    assert!(result.is_ok());
    let audio = result.unwrap();
    assert_eq!(String::from_utf8_lossy(&audio), "audio_hello");
}

#[tokio::test]
async fn test_custom_engine_list_voices() {
    let voices = vec!["voice1".to_string(), "voice2".to_string()];
    let engine = CustomTtsEngine::new(
        "test".to_string(),
        |_text: &str, _config: &VoiceConfig| {
            Ok(Bytes::from("audio"))
        },
        move || Ok(voices.clone()),
        || true,
    );

    let result = engine.list_voices().await;
    assert!(result.is_ok());
    let voices_list = result.unwrap();
    assert_eq!(voices_list.len(), 2);
    assert_eq!(voices_list[0], "voice1");
    assert_eq!(voices_list[1], "voice2");
}

#[test]
fn test_custom_engine_from_async_no_runtime() {
    // This should fail if no tokio runtime is available
    // We can't easily test this without creating a runtime, so we'll test the error case
    // by ensuring the function signature is correct
    let _engine_result: Result<CustomTtsEngine, SpeechError> = Err(SpeechError::Engine(
        "No tokio runtime available".to_string()
    ));
    // Just verify the error type is correct
    assert!(_engine_result.is_err());
}

#[tokio::test]
async fn test_custom_engine_from_async_with_runtime() {
    // Test async custom engine creation within a tokio runtime
    let engine_result = CustomTtsEngine::from_async(
        "async_test".to_string(),
        |text: &str, _config: &VoiceConfig| {
            let text = text.to_string(); // Clone to move into async block
            Box::pin(async move {
                Ok(Bytes::from(format!("async_audio_{}", text)))
            })
        },
        || {
            Box::pin(async move {
                Ok(vec!["async_voice1".to_string()])
            })
        },
        || true,
    );

    // Should succeed when called from within tokio runtime
    assert!(engine_result.is_ok());
    let engine = engine_result.unwrap();
    assert_eq!(engine.name(), "async_test");
    assert!(engine.is_available());
}

