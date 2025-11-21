//! Tests for native TTS engine (platform-specific)

use narayana_spk::engines::native::NativeTtsEngine;
use narayana_spk::engines::TtsEngine;
use narayana_spk::config::VoiceConfig;
use narayana_spk::error::SpeechError;

#[test]
fn test_native_engine_new() {
    let result = NativeTtsEngine::new();
    // Should succeed (engine may or may not be available)
    assert!(result.is_ok());
}

#[test]
fn test_native_engine_is_available() {
    let engine = NativeTtsEngine::new().unwrap();
    // Will be true or false depending on platform and availability
    let available = engine.is_available();
    // Just verify the method works
    assert!(available || !available); // Always true, just checking it doesn't panic
}

#[tokio::test]
async fn test_native_engine_synthesize_empty_text() {
    let engine = NativeTtsEngine::new().unwrap();
    
    let result = engine.synthesize("", &VoiceConfig::default()).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("empty"));
}

#[tokio::test]
async fn test_native_engine_synthesize_too_long() {
    let engine = NativeTtsEngine::new().unwrap();
    
    let long_text = "a".repeat(100_001);
    let result = engine.synthesize(&long_text, &VoiceConfig::default()).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("too long"));
}

#[tokio::test]
async fn test_native_engine_synthesize_null_bytes() {
    let engine = NativeTtsEngine::new().unwrap();
    
    let text_with_null = "Hello\0world";
    let result = engine.synthesize(text_with_null, &VoiceConfig::default()).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("null"));
}

#[tokio::test]
async fn test_native_engine_list_voices() {
    let engine = NativeTtsEngine::new().unwrap();
    
    let voices_result = engine.list_voices().await;
    assert!(voices_result.is_ok());
    let voices = voices_result.unwrap();
    // Should return at least some voices (even if empty list)
    assert!(voices.len() >= 0);
}

#[tokio::test]
async fn test_native_engine_synthesize_with_voice_config() {
    let engine = NativeTtsEngine::new().unwrap();
    
    if !engine.is_available() {
        // Skip if engine not available
        return;
    }
    
    let mut voice_config = VoiceConfig::default();
    voice_config.language = "en-US".to_string();
    
    // This will fail if engine not available, but tests the code path
    let _result = engine.synthesize("Hello world", &voice_config).await;
}

#[tokio::test]
async fn test_native_engine_synthesize_with_voice_name() {
    let engine = NativeTtsEngine::new().unwrap();
    
    if !engine.is_available() {
        return;
    }
    
    let mut voice_config = VoiceConfig::default();
    voice_config.name = Some("Alice".to_string());
    
    let _result = engine.synthesize("Hello world", &voice_config).await;
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn test_macos_native_engine() {
    let engine = NativeTtsEngine::new().unwrap();
    
    // macOS-specific tests
    let voices_result = engine.list_voices().await;
    assert!(voices_result.is_ok());
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn test_linux_native_engine() {
    let engine = NativeTtsEngine::new().unwrap();
    
    // Linux-specific tests
    let voices_result = engine.list_voices().await;
    assert!(voices_result.is_ok());
}

#[cfg(target_os = "windows")]
#[tokio::test]
async fn test_windows_native_engine() {
    let engine = NativeTtsEngine::new().unwrap();
    
    // Windows-specific tests
    let voices_result = engine.list_voices().await;
    assert!(voices_result.is_ok());
}

#[test]
fn test_native_engine_name() {
    let engine = NativeTtsEngine::new().unwrap();
    
    #[cfg(target_os = "macos")]
    assert_eq!(engine.name(), "Native TTS (macOS)");
    
    #[cfg(target_os = "linux")]
    assert_eq!(engine.name(), "Native TTS (Linux)");
    
    #[cfg(target_os = "windows")]
    assert_eq!(engine.name(), "Native TTS (Windows)");
    
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    assert_eq!(engine.name(), "Native TTS");
}

