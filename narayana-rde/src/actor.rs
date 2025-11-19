// Actor management for RDE

use serde::{Deserialize, Serialize};
use std::fmt;

/// Actor ID (any string identifier)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActorId(pub String);

impl From<String> for ActorId {
    fn from(s: String) -> Self {
        ActorId(s)
    }
}

impl From<&str> for ActorId {
    fn from(s: &str) -> Self {
        ActorId(s.to_string())
    }
}

impl fmt::Display for ActorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Actor type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActorType {
    /// Source actor - publishes events
    Source,
    /// Origin actor - receives/subscribes to events
    Origin,
}

/// Actor - represents a system that publishes or receives events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    pub id: ActorId,
    pub name: String,
    pub actor_type: ActorType,
    pub auth_token: String,
    pub created_at: u64,
    pub metadata: serde_json::Value,
}

impl Actor {
    /// Create a new actor
    pub fn new(
        id: impl Into<ActorId>,
        name: String,
        actor_type: ActorType,
        auth_token: String,
    ) -> Self {
        Self {
            id: id.into(),
            name,
            actor_type,
            auth_token,
            created_at: chrono::Utc::now().timestamp() as u64,
            metadata: serde_json::json!({}),
        }
    }

    /// Verify authentication token (constant-time comparison to prevent timing attacks)
    pub fn verify_token(&self, token: &str) -> bool {
        use sha2::{Sha256, Digest};
        
        // Use hash comparison for constant-time comparison
        let mut hasher = Sha256::new();
        hasher.update(self.auth_token.as_bytes());
        let expected_hash = hasher.finalize();
        
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let provided_hash = hasher.finalize();
        
        expected_hash == provided_hash
    }
}

