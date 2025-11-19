// Event subscriptions with multiple transport support

use crate::actor::ActorId;
use crate::events::EventName;
pub use crate::transports::TransportType;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Subscription ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubscriptionId(pub String);

impl SubscriptionId {
    /// Create new subscription ID
    pub fn new() -> Self {
        SubscriptionId(Uuid::new_v4().to_string())
    }
}

impl Default for SubscriptionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Subscription - represents an actor subscribing to an event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: SubscriptionId,
    pub actor_id: ActorId,
    pub event_name: EventName,
    pub transport: TransportType,
    pub config: serde_json::Value, // Transport-specific config (webhook_url, etc.)
    pub created_at: u64,
}

impl Subscription {
    /// Create new subscription
    pub fn new(
        actor_id: ActorId,
        event_name: EventName,
        transport: TransportType,
        config: serde_json::Value,
    ) -> Self {
        Self {
            id: SubscriptionId::new(),
            actor_id,
            event_name,
            transport,
            config,
            created_at: chrono::Utc::now().timestamp() as u64,
        }
    }
}

