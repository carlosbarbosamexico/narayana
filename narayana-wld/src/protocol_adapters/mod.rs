//! Protocol adapters for different communication protocols

pub mod http_adapter;
pub mod websocket_adapter;

use crate::event_transformer::{WorldEvent, WorldAction};
use narayana_core::Error;
use async_trait::async_trait;
use tokio::sync::broadcast;
use serde::{Deserialize, Serialize};

// Minimal types for default implementations to avoid cyclic dependency with narayana-cns
// These match narayana_cns types but are defined locally
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ComponentId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInfo {
    pub id: ComponentId,
    pub name: String,
}

/// Trait for protocol adapters
#[async_trait]
pub trait ProtocolAdapter: Send + Sync {
    /// Get the protocol name
    fn protocol_name(&self) -> &str;

    /// Start the adapter
    async fn start(&self, broker: crate::world_broker::WorldBrokerHandle) -> Result<(), Error>;

    /// Stop the adapter
    async fn stop(&self) -> Result<(), Error>;

    /// Send action to external system
    async fn send_action(&self, action: WorldAction) -> Result<(), Error>;

    /// Subscribe to incoming events
    fn subscribe_events(&self) -> broadcast::Receiver<WorldEvent>;
    
    // CNS enhancement: Component registration methods
    // Default implementations for backward compatibility
    
    /// Register a component (CNS enhancement)
    /// Default implementation returns error - adapters should override if they support registration
    async fn register_component(
        &self,
        _component: ComponentInfo,
    ) -> Result<ComponentId, Error> {
        Err(Error::Storage(format!(
            "Adapter '{}' does not support component registration",
            self.protocol_name()
        )))
    }
    
    /// Unregister a component (CNS enhancement)
    /// Default implementation returns error - adapters should override if they support registration
    async fn unregister_component(
        &self,
        _component_id: &ComponentId,
    ) -> Result<(), Error> {
        Err(Error::Storage(format!(
            "Adapter '{}' does not support component unregistration",
            self.protocol_name()
        )))
    }
    
    /// Get all registered components (CNS enhancement)
    /// Default implementation returns empty vector
    async fn get_components(&self) -> Result<Vec<ComponentInfo>, Error> {
        Ok(Vec::new())
    }
    
    /// Check if component is available (CNS enhancement)
    /// Default implementation returns false
    async fn component_available(
        &self,
        _component_id: &ComponentId,
    ) -> Result<bool, Error> {
        Ok(false)
    }
    
    /// Update component availability status (CNS enhancement)
    /// Default implementation is no-op
    async fn set_component_available(
        &self,
        _component_id: &ComponentId,
        _available: bool,
    ) -> Result<(), Error> {
        Ok(())
    }
}

pub use http_adapter::HttpAdapter;
pub use websocket_adapter::WebSocketAdapter;



