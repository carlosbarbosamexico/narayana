// Transformations using existing TransformEngine

use crate::subscriptions::Subscription;
use narayana_core::{TransformEngine, OutputConfig, Error, Result};

/// Apply transformation to event payload
pub fn apply_transformation(
    subscription: &Subscription,
    payload: &serde_json::Value,
) -> Result<serde_json::Value> {
    // Check if transformation is configured
    if let Some(output_config_json) = subscription.config.get("output_config") {
        // SECURITY: Validate it's an object before deserialization
        if !output_config_json.is_object() {
            return Err(Error::Storage("output_config must be an object".to_string()));
        }
        
        // SECURITY: Limit depth and size of config to prevent deserialization attacks
        let config_str = serde_json::to_string(output_config_json)
            .map_err(|e| Error::Storage(format!("Failed to serialize config: {}", e)))?;
        if config_str.len() > 100_000 {
            return Err(Error::Storage("output_config too large (max 100KB)".to_string()));
        }
        
        // Parse OutputConfig from JSON
        let output_config: OutputConfig = serde_json::from_value(output_config_json.clone())
            .map_err(|e| Error::Storage(format!("Failed to parse output_config: {}", e)))?;
        
        // Apply transformation using existing TransformEngine
        let transformed = TransformEngine::apply_config(payload.clone(), &output_config)
            .map_err(|e| Error::Storage(format!("Transformation failed: {}", e)))?;
        
        Ok(transformed)
    } else {
        // No transformation, return original
        Ok(payload.clone())
    }
}

