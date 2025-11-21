//! CPL integration for speech settings

use crate::config::SpeechConfig;
use crate::error::SpeechError;
use narayana_core::Error;
use narayana_storage::conscience_persistent_loop::CPLConfig;
use serde_json;

/// Extract speech config from CPL config
/// This allows CPL settings to cascade to the speech adapter
pub fn speech_config_from_cpl(cpl_config: &CPLConfig) -> Option<SpeechConfig> {
    if !cpl_config.enable_speech {
        return None; // Speech disabled in CPL
    }

    // If CPL has explicit speech config, use it
    if let Some(ref speech_json) = cpl_config.speech_config {
        match serde_json::from_value::<SpeechConfig>(speech_json.clone()) {
            Ok(mut config) => {
                // Validate the parsed config
                if let Err(e) = config.validate() {
                    tracing::warn!("Invalid CPL speech config: {}, using defaults", e);
                    // Fall back to default with enabled=true
                    let mut default_config = SpeechConfig::default();
                    default_config.enabled = true;
                    Some(default_config)
                } else {
                    config.enabled = true; // Ensure enabled if CPL says so
                    Some(config)
                }
            }
            Err(e) => {
                tracing::warn!("Failed to parse CPL speech config: {}", e);
                // Fall back to default with enabled=true
                let mut config = SpeechConfig::default();
                config.enabled = true;
                Some(config)
            }
        }
    } else {
        // Use default config but enable it
        let mut config = SpeechConfig::default();
        config.enabled = true;
        Some(config)
    }
}

/// Create speech adapter from CPL config
pub fn create_speech_adapter_from_cpl(
    cpl_config: &CPLConfig,
) -> Result<Option<crate::speech_adapter::SpeechAdapter>, Error> {
    if let Some(speech_config) = speech_config_from_cpl(cpl_config) {
        match crate::speech_adapter::SpeechAdapter::new(speech_config) {
            Ok(adapter) => Ok(Some(adapter)),
            Err(e) => {
                tracing::warn!("Failed to create speech adapter from CPL config: {}", e);
                Ok(None)
            }
        }
    } else {
        Ok(None)
    }
}

