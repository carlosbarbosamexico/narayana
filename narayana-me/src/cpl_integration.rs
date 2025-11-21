//! CPL integration for avatar settings

use crate::config::AvatarConfig;
use narayana_core::Error;
use narayana_storage::conscience_persistent_loop::CPLConfig;
use serde_json;

/// Extract avatar config from CPL config
/// This allows CPL settings to cascade to the avatar adapter
pub fn avatar_config_from_cpl(cpl_config: &CPLConfig) -> Option<AvatarConfig> {
    if !cpl_config.enable_avatar {
        return None; // Avatar disabled in CPL
    }

    // If CPL has explicit avatar config, use it
    if let Some(ref avatar_json) = cpl_config.avatar_config {
        // Validate JSON size to prevent DoS
        let json_size = serde_json::to_string(avatar_json)
            .map(|s| s.len())
            .unwrap_or(0);
        const MAX_AVATAR_CONFIG_SIZE: usize = 100_000; // 100KB max
        if json_size > MAX_AVATAR_CONFIG_SIZE {
            tracing::warn!("CPL avatar config too large ({} bytes, max {} bytes), using defaults", json_size, MAX_AVATAR_CONFIG_SIZE);
            let mut config = AvatarConfig::default();
            config.enabled = true;
            return Some(config);
        }
        
        match serde_json::from_value::<AvatarConfig>(avatar_json.clone()) {
            Ok(mut config) => {
                // Validate the parsed config
                if let Err(e) = config.validate() {
                    tracing::warn!("Invalid CPL avatar config: {}, using defaults", e);
                    // Fall back to default with enabled=true
                    let mut default_config = AvatarConfig::default();
                    default_config.enabled = true;
                    Some(default_config)
                } else {
                    config.enabled = true; // Ensure enabled if CPL says so
                    Some(config)
                }
            }
            Err(e) => {
                tracing::warn!("Failed to parse CPL avatar config: {}", e);
                // Fall back to default with enabled=true
                let mut config = AvatarConfig::default();
                config.enabled = true;
                Some(config)
            }
        }
    } else {
        // Use default config but enable it
        let mut config = AvatarConfig::default();
        config.enabled = true;
        Some(config)
    }
}

/// Create avatar adapter from CPL config
pub fn create_avatar_adapter_from_cpl(
    cpl_config: &CPLConfig,
) -> Result<Option<crate::avatar_adapter::AvatarAdapter>, Error> {
    if let Some(avatar_config) = avatar_config_from_cpl(cpl_config) {
        match crate::avatar_adapter::AvatarAdapter::new(avatar_config) {
            Ok(adapter) => Ok(Some(adapter)),
            Err(e) => {
                tracing::warn!("Failed to create avatar adapter from CPL config: {}", e);
                Ok(None)
            }
        }
    } else {
        Ok(None)
    }
}

