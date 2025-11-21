//! CPL integration for audio settings

use crate::config::AudioConfig;
use crate::error::AudioError;
use narayana_core::Error;
use narayana_storage::conscience_persistent_loop::CPLConfig;
use serde_json;

/// Extract audio config from CPL config
pub fn audio_config_from_cpl(cpl_config: &CPLConfig) -> Option<AudioConfig> {
    if !cpl_config.enable_audio {
        return None; // Audio disabled in CPL
    }

    // If CPL has explicit audio config, use it
    if let Some(ref audio_json) = cpl_config.audio_config {
        match serde_json::from_value::<AudioConfig>(audio_json.clone()) {
            Ok(mut config) => {
                // Validate the parsed config
                if let Err(e) = config.validate() {
                    tracing::warn!("Invalid CPL audio config: {}, using defaults", e);
                    // Fall back to default with enabled=true
                    let mut default_config = AudioConfig::default();
                    default_config.enabled = true;
                    Some(default_config)
                } else {
                    config.enabled = true; // Ensure enabled if CPL says so
                    Some(config)
                }
            }
            Err(e) => {
                tracing::warn!("Failed to parse CPL audio config: {}", e);
                // Fall back to default with enabled=true
                let mut config = AudioConfig::default();
                config.enabled = true;
                Some(config)
            }
        }
    } else {
        // Use default config but enable it
        let mut config = AudioConfig::default();
        config.enabled = true;
        Some(config)
    }
}

/// Create audio adapter from CPL config
pub fn create_audio_adapter_from_cpl(
    cpl_config: &CPLConfig,
) -> Result<Option<crate::audio_adapter::AudioAdapter>, Error> {
    if let Some(audio_config) = audio_config_from_cpl(cpl_config) {
        match crate::audio_adapter::AudioAdapter::new(audio_config) {
            Ok(adapter) => Ok(Some(adapter)),
            Err(e) => {
                tracing::warn!("Failed to create audio adapter from CPL config: {}", e);
                Ok(None)
            }
        }
    } else {
        Ok(None)
    }
}

