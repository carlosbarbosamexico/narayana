//! Component types and definitions

use crate::capability::Capability;
use crate::safety::SafetyLimits;
use crate::transport::TransportConfig;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Unique component identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ComponentId(pub String);

impl ComponentId {
    pub fn new(id: String) -> Self {
        Self(id)
    }
    
    pub fn generate() -> Self {
        use uuid::Uuid;
        Self(Uuid::new_v4().to_string())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ComponentId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ComponentId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Component type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComponentType {
    /// Actuator - can perform actions
    Actuator,
    /// Sensor - provides data
    Sensor,
    /// Hybrid - both actuator and sensor
    Hybrid,
}

/// Component state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComponentState {
    /// Component is available and ready
    Available,
    /// Component is busy/processing
    Busy,
    /// Component is unavailable/offline
    Unavailable,
    /// Component is in error state
    Error(String),
    /// Component is in maintenance mode
    Maintenance,
}

impl Default for ComponentState {
    fn default() -> Self {
        ComponentState::Unavailable
    }
}

/// Component information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInfo {
    /// Unique component identifier
    pub id: ComponentId,
    /// Human-readable component name
    pub name: String,
    /// Component type
    pub component_type: ComponentType,
    /// Capabilities this component provides
    pub capabilities: Vec<Capability>,
    /// Transport configuration
    pub transport: TransportConfig,
    /// Additional metadata
    pub metadata: HashMap<String, JsonValue>,
    /// Safety limits (if applicable)
    pub safety_limits: Option<SafetyLimits>,
    /// Component version
    pub version: String,
    /// Current state
    #[serde(skip)]
    pub state: ComponentState,
    /// Registration timestamp
    pub registered_at: u64,
    /// Last heartbeat timestamp
    pub last_heartbeat: u64,
}

impl ComponentInfo {
    /// Create new component info
    pub fn new(
        id: ComponentId,
        name: String,
        component_type: ComponentType,
        capabilities: Vec<Capability>,
        transport: TransportConfig,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        Self {
            id,
            name,
            component_type,
            capabilities,
            transport,
            metadata: HashMap::new(),
            safety_limits: None,
            version: "1.0.0".to_string(),
            state: ComponentState::Available,
            registered_at: now,
            last_heartbeat: now,
        }
    }
    
    /// Check if component is available
    pub fn is_available(&self) -> bool {
        matches!(self.state, ComponentState::Available)
    }
    
    /// Check if component has a specific capability
    pub fn has_capability(&self, capability: &Capability) -> bool {
        self.capabilities.iter().any(|c| c.matches(capability))
    }
    
    /// Update heartbeat timestamp
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }
    
    /// Check if component is healthy (heartbeat within timeout)
    pub fn is_healthy(&self, heartbeat_timeout_secs: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        now.saturating_sub(self.last_heartbeat) <= heartbeat_timeout_secs
    }
}

