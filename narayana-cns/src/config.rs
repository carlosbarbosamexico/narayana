//! Configuration for narayana-cns

use crate::safety::SafetyLevel;
use serde::{Deserialize, Serialize};

/// CNS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CnsConfig {
    /// Default safety level
    pub default_safety_level: SafetyLevel,
    /// Heartbeat timeout in seconds
    pub heartbeat_timeout_secs: u64,
    /// Enable capability-based routing
    pub enable_capability_routing: bool,
    /// Enable load balancing
    pub enable_load_balancing: bool,
    /// Maximum action queue size
    pub max_action_queue_size: usize,
    /// Enable emergency stop
    pub enable_emergency_stop: bool,
    /// Action timeout in milliseconds
    pub action_timeout_ms: u64,
}

impl Default for CnsConfig {
    fn default() -> Self {
        Self {
            default_safety_level: SafetyLevel::Production,
            heartbeat_timeout_secs: 5,
            enable_capability_routing: true,
            enable_load_balancing: true,
            max_action_queue_size: 1000,
            enable_emergency_stop: true,
            action_timeout_ms: 5000,
        }
    }
}

impl CnsConfig {
    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.heartbeat_timeout_secs == 0 {
            return Err("Heartbeat timeout must be greater than 0".to_string());
        }
        
        if self.max_action_queue_size == 0 {
            return Err("Max action queue size must be greater than 0".to_string());
        }
        
        if self.action_timeout_ms == 0 {
            return Err("Action timeout must be greater than 0".to_string());
        }
        
        Ok(())
    }
}

