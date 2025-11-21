//! narayana-cns: Central Nervous System for component registration and routing
//! 
//! Provides:
//! - Dynamic component registration and discovery
//! - Capability-based action routing
//! - Safety interlock and validation
//! - Pluggable transport layer abstraction
//! - Integration with WorldBroker and CPL

pub mod error;
pub mod component;
pub mod capability;
pub mod registry;
pub mod safety;
pub mod router;
pub mod transport;
pub mod cns;
pub mod config;

pub use error::CnsError;
pub use component::{ComponentInfo, ComponentId, ComponentType, ComponentState};
pub use capability::{Capability, StructuredCapability, CapabilityMatcher};
pub use registry::ComponentRegistry;
pub use safety::{SafetyValidator, SafetyLimits, SafetyLevel, SafetyRule};
pub use router::ActionRouter;
pub use transport::{Transport, TransportConfig, TransportRegistry};
pub use cns::CentralNervousSystem;
pub use config::CnsConfig;

