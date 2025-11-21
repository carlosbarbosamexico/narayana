//! narayana-me: 3D Virtual Avatar for CPL
//! 
//! Provides realistic 3D avatar rendering with:
//! - Pluggable avatar providers (Beyond Presence, LiveAvatar, Ready Player Me, etc.)
//! - Real-time lip sync and facial expressions
//! - Integration with narayana-wld for CPL-controlled avatars
//! - WebSocket bridge for web client streaming
//! - Configurable and off by default

pub mod error;
pub mod config;
pub mod avatar_broker;
pub mod providers;
pub mod avatar_adapter;
pub mod cpl_integration;
pub mod bridge;
pub mod multimodal;

pub use error::AvatarError;
pub use config::{AvatarConfig, AvatarProviderType, Expression, Gesture, Emotion};
pub use avatar_broker::{AvatarBroker, AvatarProvider, AvatarStream};
pub use avatar_adapter::AvatarAdapter;
pub use cpl_integration::{avatar_config_from_cpl, create_avatar_adapter_from_cpl};
pub use bridge::AvatarBridge; // Export bridge for external use
pub use multimodal::MultimodalManager; // Export multimodal manager for external use
