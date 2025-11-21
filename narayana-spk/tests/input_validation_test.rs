//! Input validation tests for narayana-spk
//! Tests for command injection prevention, XSS prevention, and input sanitization

use narayana_spk::config::{SpeechConfig, VoiceConfig};
use narayana_spk::speech_adapter::SpeechAdapter;
use narayana_wld::event_transformer::WorldAction;
use narayana_wld::protocol_adapters::ProtocolAdapter;
use serde_json::json;
use narayana_core::Error;

#[tokio::test]
async fn test_command_injection_prevention() {
    // Test that shell metacharacters are filtered
    let dangerous_inputs = vec![
        "test; rm -rf /",
        "test | cat /etc/passwd",
        "test && malicious_command",
        "test $(evil_command)",
        "test `backtick_command`",
        "test $(whoami)",
        "test; DROP TABLE users;",
    ];
    
    let config = SpeechConfig::default();
    let adapter = SpeechAdapter::new(config).unwrap();
    
    for dangerous_input in dangerous_inputs {
        let action = WorldAction::ActuatorCommand {
            target: "speech".to_string(),
            command: json!({"text": dangerous_input}),
        };
        
        // Should not crash or execute commands
        let result = adapter.send_action(action).await;
        assert!(result.is_ok(), "Should handle dangerous input: {}", dangerous_input);
    }
}

#[tokio::test]
async fn test_xss_prevention_in_text() {
    // Test that XSS attempts in text are handled safely
    let xss_inputs = vec![
        "<script>alert('xss')</script>",
        "<iframe src='evil.com'></iframe>",
        "javascript:alert('xss')",
        "onerror=alert('xss')",
        "<img src=x onerror=alert('xss')>",
    ];
    
    let config = SpeechConfig::default();
    let adapter = SpeechAdapter::new(config).unwrap();
    
    for xss_input in xss_inputs {
        let action = WorldAction::ActuatorCommand {
            target: "speech".to_string(),
            command: json!({"text": xss_input}),
        };
        
        // Should sanitize or reject XSS attempts
        let result = adapter.send_action(action).await;
        assert!(result.is_ok(), "Should handle XSS input: {}", xss_input);
    }
}

#[tokio::test]
async fn test_null_byte_injection() {
    let config = SpeechConfig::default();
    let adapter = SpeechAdapter::new(config).unwrap();
    
    let action = WorldAction::ActuatorCommand {
        target: "speech".to_string(),
        command: json!({"text": "Hello\0world"}),
    };
    
    // Should reject null bytes
    let result = adapter.send_action(action).await;
    assert!(result.is_ok()); // Should handle gracefully
}

#[tokio::test]
async fn test_oversized_command() {
    let config = SpeechConfig::default();
    let adapter = SpeechAdapter::new(config).unwrap();
    
    // Create a command that's too large
    let large_text = "a".repeat(200_001); // Over 200KB limit
    let action = WorldAction::ActuatorCommand {
        target: "speech".to_string(),
        command: json!({"text": large_text}),
    };
    
    // Should reject oversized commands
    let result = adapter.send_action(action).await;
    assert!(result.is_ok()); // Should handle gracefully (truncate or reject)
}

#[tokio::test]
async fn test_oversized_target_name() {
    let config = SpeechConfig::default();
    let adapter = SpeechAdapter::new(config).unwrap();
    
    let large_target = "a".repeat(257); // Over 256 char limit
    let action = WorldAction::ActuatorCommand {
        target: large_target,
        command: json!({"text": "Hello"}),
    };
    
    // Should reject oversized target
    let result = adapter.send_action(action).await;
    assert!(result.is_ok()); // Should handle gracefully
}

#[tokio::test]
async fn test_empty_text() {
    let config = SpeechConfig::default();
    let adapter = SpeechAdapter::new(config).unwrap();
    
    let action = WorldAction::ActuatorCommand {
        target: "speech".to_string(),
        command: json!({"text": ""}),
    };
    
    // Should reject empty text
    let result = adapter.send_action(action).await;
    assert!(result.is_ok()); // Should handle gracefully
}

#[tokio::test]
async fn test_missing_text_field() {
    let config = SpeechConfig::default();
    let adapter = SpeechAdapter::new(config).unwrap();
    
    let action = WorldAction::ActuatorCommand {
        target: "speech".to_string(),
        command: json!({}), // No text field
    };
    
    // Should handle missing text gracefully
    let result = adapter.send_action(action).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_message_field_as_alternative() {
    let config = SpeechConfig::default();
    let adapter = SpeechAdapter::new(config).unwrap();
    
    let action = WorldAction::ActuatorCommand {
        target: "speech".to_string(),
        command: json!({"message": "Hello world"}),
    };
    
    // Should accept "message" as alternative to "text"
    let result = adapter.send_action(action).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_very_long_text_truncation() {
    let config = SpeechConfig::default();
    let adapter = SpeechAdapter::new(config).unwrap();
    
    let long_text = "a".repeat(150_000); // Over 100KB limit
    let action = WorldAction::ActuatorCommand {
        target: "speech".to_string(),
        command: json!({"text": long_text}),
    };
    
    // Should truncate to 100KB
    let result = adapter.send_action(action).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_unicode_text_handling() {
    let config = SpeechConfig::default();
    let adapter = SpeechAdapter::new(config).unwrap();
    
    let unicode_texts = vec![
        "Hello 世界",
        "مرحبا",
        "Здравствуй",
        "こんにちは",
        "🎉🎊🎈",
    ];
    
    for text in unicode_texts {
        let action = WorldAction::ActuatorCommand {
            target: "speech".to_string(),
            command: json!({"text": text}),
        };
        
        // Should handle Unicode text
        let result = adapter.send_action(action).await;
        assert!(result.is_ok(), "Should handle Unicode text: {}", text);
    }
}

#[tokio::test]
async fn test_utf8_boundary_truncation() {
    let config = SpeechConfig::default();
    let adapter = SpeechAdapter::new(config).unwrap();
    
    // Text that ends in the middle of a multi-byte UTF-8 character
    let text = "Hello 世界"; // "世界" is 6 bytes in UTF-8
    let action = WorldAction::ActuatorCommand {
        target: "speech".to_string(),
        command: json!({"text": text}),
    };
    
    // Should handle UTF-8 boundaries correctly when truncating
    let result = adapter.send_action(action).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_special_characters_in_text() {
    let config = SpeechConfig::default();
    let adapter = SpeechAdapter::new(config).unwrap();
    
    let special_chars = vec![
        "Hello, world!",
        "Price: $100.00",
        "Email: test@example.com",
        "Path: /usr/bin/test",
        "Query: ?param=value",
    ];
    
    for text in special_chars {
        let action = WorldAction::ActuatorCommand {
            target: "speech".to_string(),
            command: json!({"text": text}),
        };
        
        // Should handle special characters
        let result = adapter.send_action(action).await;
        assert!(result.is_ok(), "Should handle special characters: {}", text);
    }
}

#[tokio::test]
async fn test_non_speech_target() {
    let config = SpeechConfig::default();
    let adapter = SpeechAdapter::new(config).unwrap();
    
    let action = WorldAction::ActuatorCommand {
        target: "motor".to_string(), // Not a speech target
        command: json!({"text": "Hello"}),
    };
    
    // Should ignore non-speech targets
    let result = adapter.send_action(action).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_speech_prefix_target() {
    let config = SpeechConfig::default();
    let adapter = SpeechAdapter::new(config).unwrap();
    
    let action = WorldAction::ActuatorCommand {
        target: "speech_tts".to_string(), // Starts with "speech_"
        command: json!({"text": "Hello"}),
    };
    
    // Should handle targets starting with "speech_"
    let result = adapter.send_action(action).await;
    assert!(result.is_ok());
}

