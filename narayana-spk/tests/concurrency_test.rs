//! Concurrency and thread safety tests for narayana-spk
//! Tests for race conditions, deadlocks, and concurrent access

use narayana_spk::config::SpeechConfig;
use narayana_spk::speech_adapter::SpeechAdapter;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[test]
fn test_config_thread_safety() {
    // Test that SpeechConfig can be safely shared across threads
    let config = Arc::new(SpeechConfig::default());
    let mut handles = vec![];
    
    for i in 0..10 {
        let config_clone = config.clone();
        let handle = thread::spawn(move || {
            // Read config from multiple threads
            let _rate = config_clone.rate;
            let _volume = config_clone.volume;
            let _result = config_clone.validate();
            i // Return something to prevent optimization
        });
        handles.push(handle);
    }
    
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results.len(), 10);
}

#[test]
fn test_adapter_creation_thread_safety() {
    // Test that adapter creation is thread-safe
    let config = SpeechConfig::default();
    let mut handles = vec![];
    
    for _ in 0..5 {
        let config_clone = config.clone();
        let handle = thread::spawn(move || {
            // Create adapters from multiple threads
            SpeechAdapter::new(config_clone)
        });
        handles.push(handle);
    }
    
    for handle in handles {
        let result = handle.join().unwrap();
        assert!(result.is_ok());
    }
}

#[test]
fn test_concurrent_validation() {
    // Test concurrent config validation
    let config = Arc::new(SpeechConfig::default());
    let mut handles = vec![];
    
    for _ in 0..20 {
        let config_clone = config.clone();
        let handle = thread::spawn(move || {
            config_clone.validate()
        });
        handles.push(handle);
    }
    
    for handle in handles {
        let result = handle.join().unwrap();
        assert!(result.is_ok());
    }
}

#[test]
fn test_no_deadlock_on_concurrent_access() {
    // Test that concurrent access doesn't cause deadlocks
    let config = Arc::new(SpeechConfig::default());
    let mut handles = vec![];
    
    for _ in 0..10 {
        let config_clone = config.clone();
        let handle = thread::spawn(move || {
            // Perform various operations
            let _rate = config_clone.rate;
            thread::sleep(Duration::from_millis(1));
            let _volume = config_clone.volume;
            thread::sleep(Duration::from_millis(1));
            let _result = config_clone.validate();
        });
        handles.push(handle);
    }
    
    // All threads should complete without deadlock
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_arc_clone_performance() {
    // Test that Arc cloning is efficient
    let config = Arc::new(SpeechConfig::default());
    
    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let _cloned = config.clone();
    }
    let duration = start.elapsed();
    
    // Arc cloning should be very fast (just reference counting)
    assert!(duration.as_millis() < 100, "Arc cloning took too long: {:?}", duration);
}

#[test]
fn test_send_sync_traits() {
    // Test that types implement Send and Sync
    use std::marker::{Send, Sync};
    
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    
    // These should compile (meaning types are Send/Sync)
    assert_send::<SpeechConfig>();
    assert_sync::<SpeechConfig>();
    
    // Arc types should also be Send/Sync
    assert_send::<Arc<SpeechConfig>>();
    assert_sync::<Arc<SpeechConfig>>();
}

#[test]
fn test_concurrent_error_handling() {
    // Test that errors are handled correctly in concurrent scenarios
    let mut config = SpeechConfig::default();
    config.rate = 600; // Invalid
    
    let config = Arc::new(config);
    let mut handles = vec![];
    
    for _ in 0..5 {
        let config_clone = config.clone();
        let handle = thread::spawn(move || {
            config_clone.validate()
        });
        handles.push(handle);
    }
    
    // All threads should get the same error
    for handle in handles {
        let result = handle.join().unwrap();
        assert!(result.is_err());
    }
}


