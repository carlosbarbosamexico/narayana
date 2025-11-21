//! Tests for CPL integration

use narayana_spk::cpl_integration::speech_config_from_cpl;
use narayana_storage::conscience_persistent_loop::CPLConfig;
use serde_json::json;

#[test]
fn test_speech_config_from_cpl_disabled() {
    let mut cpl_config = CPLConfig::default();
    cpl_config.enable_speech = false;
    
    let result = speech_config_from_cpl(&cpl_config);
    assert!(result.is_none());
}

#[test]
fn test_speech_config_from_cpl_enabled() {
    let mut cpl_config = CPLConfig::default();
    cpl_config.enable_speech = true;
    
    let result = speech_config_from_cpl(&cpl_config);
    assert!(result.is_some());
    let config = result.unwrap();
    assert!(config.enabled);
}

#[test]
fn test_speech_config_from_cpl_with_json() {
    let mut cpl_config = CPLConfig::default();
    cpl_config.enable_speech = true;
    use serde_json::json;
    cpl_config.speech_config = Some(json!({
        "rate": 200,
        "volume": 0.9,
        "voice": {
            "language": "es-ES"
        }
    }));
    
    let result = speech_config_from_cpl(&cpl_config);
    assert!(result.is_some());
    let config = result.unwrap();
    assert!(config.enabled);
    assert_eq!(config.rate, 200);
    assert_eq!(config.volume, 0.9);
    assert_eq!(config.voice.language, "es-ES");
}

