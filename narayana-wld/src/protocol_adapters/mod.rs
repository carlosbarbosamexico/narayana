//! Protocol adapters for different communication protocols

pub mod http_adapter;
pub mod websocket_adapter;

use crate::event_transformer::{WorldEvent, WorldAction};
use narayana_core::Error;
use async_trait::async_trait;
use tokio::sync::broadcast;

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
}

pub use http_adapter::HttpAdapter;
pub use websocket_adapter::WebSocketAdapter;

