//! World Broker Interface (narayana-wld)
//! 
//! A broker interface that mediates bidirectional communication between
//! Conscience Persistent Loops (CPL) and the external world.
//! 
//! Based on cognitive architecture principles:
//! - Global Workspace Theory (Baars, 1988)
//! - Predictive Processing / Active Inference (Friston, 2010)
//! - Embodied Cognition (Varela, Thompson, Rosch, 1991)
//! - Attention Mechanisms (Desimone & Duncan, 1995)

pub mod world_broker;
pub mod sensory_interface;
pub mod motor_interface;
pub mod event_transformer;
pub mod attention_filter;
pub mod config;
pub mod protocol_adapters;

pub use world_broker::{WorldBroker, WorldBrokerHandle};
pub use config::WorldBrokerConfig;
pub use event_transformer::{WorldEvent, WorldAction, EventTransformer};
pub use attention_filter::AttentionFilter;
pub use sensory_interface::SensoryInterface;
pub use motor_interface::MotorInterface;
pub use protocol_adapters::{ProtocolAdapter, HttpAdapter, WebSocketAdapter};

#[cfg(test)]
mod tests;
