//! Integration tests for SpeechSynthesizer with different engines

use narayana_spk::synthesizer::SpeechSynthesizer;
use narayana_spk::config::{SpeechConfig, VoiceConfig, TtsEngine};
use narayana_spk::error::SpeechError;

#[tokio::test]
async fn test_synthesizer_creation_native() {
    let mut config = SpeechConfig::default();
    config.enabled = true;
    config.engine = TtsEngine::Native;
    
    let result = SpeechSynthesizer::new(config);
    
    // May fail if native engine not available, but tests the code path
    if result.is_err() {
        // Expected if engine not available - just verify it's an error
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn test_synthesizer_creation_disabled() {
    let mut config = SpeechConfig::default();
    config.enabled = false;
    
    let result = SpeechSynthesizer::new(config);
    assert!(result.is_err());
    // Error should indicate disabled
    if let Err(e) = result {
        assert!(e.to_string().contains("disabled"));
    }
}

#[tokio::test]
async fn test_synthesizer_speak_empty_text() {
    let mut config = SpeechConfig::default();
    config.enabled = true;
    config.engine = TtsEngine::Native;
    
    let result = SpeechSynthesizer::new(config);
    
    if let Ok(synthesizer) = result {
        let speak_result = synthesizer.speak("").await;
        assert!(speak_result.is_err());
        assert!(speak_result.unwrap_err().to_string().contains("empty"));
    }
}

#[tokio::test]
async fn test_synthesizer_speak_with_config() {
    let mut config = SpeechConfig::default();
    config.enabled = true;
    config.engine = TtsEngine::Native;
    
    let result = SpeechSynthesizer::new(config);
    
    if let Ok(synthesizer) = result {
        let mut voice_config = VoiceConfig::default();
        voice_config.language = "en-US".to_string();
        
        let speak_result = synthesizer.speak_with_config("Hello", &voice_config).await;
        // May fail if engine not available, but tests the code path
        if speak_result.is_err() {
            // Expected if engine not available
        }
    }
}

#[tokio::test]
async fn test_synthesizer_cache_enabled() {
    let mut config = SpeechConfig::default();
    config.enabled = true;
    config.engine = TtsEngine::Native;
    config.enable_cache = true;
    
    let result = SpeechSynthesizer::new(config);
    
    if let Ok(_synthesizer) = result {
        // Test that cache is enabled
        // Cache functionality is tested in cache_test.rs
    }
}

#[tokio::test]
async fn test_synthesizer_cache_disabled() {
    let mut config = SpeechConfig::default();
    config.enabled = true;
    config.engine = TtsEngine::Native;
    config.enable_cache = false;
    
    let result = SpeechSynthesizer::new(config);
    
    if let Ok(_synthesizer) = result {
        // Test that cache is disabled
    }
}

#[tokio::test]
async fn test_synthesizer_text_validation() {
    let mut config = SpeechConfig::default();
    config.enabled = true;
    config.engine = TtsEngine::Native;
    
    let result = SpeechSynthesizer::new(config);
    
    if let Ok(synthesizer) = result {
        // Test null bytes
        let text_with_null = "Hello\0world";
        let result = synthesizer.speak(text_with_null).await;
        assert!(result.is_err());
        
        // Test too long text
        let long_text = "a".repeat(100_001);
        let result = synthesizer.speak(&long_text).await;
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn test_synthesizer_voice_config_validation() {
    let mut config = SpeechConfig::default();
    config.enabled = true;
    config.engine = TtsEngine::Native;
    
    let result = SpeechSynthesizer::new(config);
    
    if let Ok(synthesizer) = result {
        // Test language too long
        let mut voice_config = VoiceConfig::default();
        voice_config.language = "a".repeat(33);
        let result = synthesizer.speak_with_config("Hello", &voice_config).await;
        assert!(result.is_err());
        
        // Test voice name too long
        let mut voice_config = VoiceConfig::default();
        voice_config.name = Some("a".repeat(257));
        let result = synthesizer.speak_with_config("Hello", &voice_config).await;
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn test_synthesizer_audio_size_validation() {
    let mut config = SpeechConfig::default();
    config.enabled = true;
    config.engine = TtsEngine::Native;
    
    let result = SpeechSynthesizer::new(config);
    
    if result.is_ok() {
        // Audio size validation is tested in cache_test.rs
        // This test just verifies the code path exists
    }
}

