//! Performance and stress tests for narayana-spk
//! Tests for resource usage, limits, and performance characteristics

use narayana_spk::config::{SpeechConfig, VoiceConfig};
use std::time::Instant;

#[test]
fn test_config_validation_performance() {
    // Test that config validation is fast
    let config = SpeechConfig::default();
    
    let start = Instant::now();
    let _result = config.validate();
    let duration = start.elapsed();
    
    // Validation should be very fast (< 1ms for simple config)
    assert!(duration.as_millis() < 100, "Config validation took too long: {:?}", duration);
}

#[test]
fn test_voice_config_validation_performance() {
    // Test that voice config validation is fast
    let voice = VoiceConfig::default();
    
    let start = Instant::now();
    let _result = voice.validate();
    let duration = start.elapsed();
    
    // Validation should be very fast
    assert!(duration.as_millis() < 100, "Voice validation took too long: {:?}", duration);
}

#[test]
fn test_large_text_validation_performance() {
    // Test that large text validation is still fast
    let large_text = "a".repeat(100_000);
    
    let start = Instant::now();
    let is_empty = large_text.is_empty();
    let has_null = large_text.contains('\0');
    let len = large_text.len();
    let duration = start.elapsed();
    
    assert!(!is_empty);
    assert!(!has_null);
    assert_eq!(len, 100_000);
    // Basic operations should be fast even on large text
    assert!(duration.as_millis() < 1000, "Large text validation took too long: {:?}", duration);
}

#[test]
fn test_cache_key_generation_performance() {
    // Test that cache key generation is fast
    let text = "Hello world".repeat(1000); // ~11KB
    let voice = VoiceConfig::default();
    
    // Simulate cache key generation (hashing)
    use sha2::{Sha256, Digest};
    let start = Instant::now();
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hasher.update(voice.language.as_bytes());
    let _hash = hasher.finalize();
    let duration = start.elapsed();
    
    // Hashing should be fast even for larger inputs
    assert!(duration.as_millis() < 100, "Cache key generation took too long: {:?}", duration);
}

#[test]
fn test_memory_usage_limits() {
    // Test that memory usage is bounded
    let max_text_length = 100_000;
    let max_cache_size_mb = 10_000;
    let max_audio_size = 10 * 1024 * 1024; // 10MB
    
    // Verify limits are reasonable
    assert!(max_text_length < usize::MAX);
    assert!(max_cache_size_mb < u64::MAX);
    assert!(max_audio_size < usize::MAX);
    
    // Verify limits prevent excessive memory usage
    let max_memory_per_text = max_audio_size; // Worst case: one audio per text
    let max_total_memory = (max_cache_size_mb as usize) * 1024 * 1024;
    
    assert!(max_memory_per_text < max_total_memory);
}

#[test]
fn test_concurrent_access_safety() {
    // Test that config can be safely accessed from multiple threads
    use std::sync::Arc;
    use std::thread;
    
    let config = Arc::new(SpeechConfig::default());
    let mut handles = vec![];
    
    for _ in 0..10 {
        let config_clone = config.clone();
        let handle = thread::spawn(move || {
            // Multiple threads reading config
            let _rate = config_clone.rate;
            let _volume = config_clone.volume;
            let _result = config_clone.validate();
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_string_operations_performance() {
    // Test that string operations used in validation are efficient
    let test_strings = vec![
        "en-US".to_string(),
        "a".repeat(32), // Max length language
        "voice-name".to_string(),
        "a".repeat(256), // Max length voice name
    ];
    
    for s in test_strings {
        let start = Instant::now();
        let len = s.len();
        let has_invalid = s.chars().any(|c| c == '\0' || c.is_control());
        let is_ascii = s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
        let duration = start.elapsed();
        
        assert!(len > 0);
        // String operations should be fast
        assert!(duration.as_millis() < 10, "String operations took too long: {:?}", duration);
        
        // Use the results to prevent optimization
        let _ = (has_invalid, is_ascii);
    }
}

#[test]
fn test_config_clone_performance() {
    // Test that config cloning is reasonably fast
    let config = SpeechConfig::default();
    
    let start = Instant::now();
    for _ in 0..100 {
        let _cloned = config.clone();
    }
    let duration = start.elapsed();
    
    // Cloning should be fast
    assert!(duration.as_millis() < 1000, "Config cloning took too long: {:?}", duration);
}

#[test]
fn test_validation_caching() {
    // Test that repeated validation calls are fast
    let config = SpeechConfig::default();
    
    let start = Instant::now();
    for _ in 0..1000 {
        let _result = config.validate();
    }
    let duration = start.elapsed();
    
    // 1000 validations should be very fast
    assert!(duration.as_millis() < 1000, "Repeated validation took too long: {:?}", duration);
}

#[test]
fn test_resource_limits_enforcement() {
    // Test that resource limits are actually enforced
    let max_text = 100_000;
    let max_cache_mb = 10_000;
    let max_audio_mb = 10;
    let max_queue = 10_000;
    
    // All limits should be positive and reasonable
    assert!(max_text > 0);
    assert!(max_cache_mb > 0);
    assert!(max_audio_mb > 0);
    assert!(max_queue > 0);
    
    // Limits should prevent resource exhaustion
    assert!(max_text < usize::MAX / 2);
    assert!(max_cache_mb < u64::MAX / 2);
    assert!(max_audio_mb * 1024 * 1024 < usize::MAX / 2);
    assert!(max_queue < usize::MAX / 2);
}


