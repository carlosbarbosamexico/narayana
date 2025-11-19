// Authentication for RDE

use crate::actor::{Actor, ActorId};
use dashmap::DashMap;
use std::sync::Arc;
use narayana_core::Result;

/// Authentication Manager
pub struct AuthManager {
    actors: Arc<DashMap<ActorId, Actor>>,
}

impl AuthManager {
    /// Create new auth manager with shared actors map
    pub fn new(actors: Arc<DashMap<ActorId, Actor>>) -> Self {
        Self { actors }
    }

    /// Authenticate actor by token
    pub fn authenticate(&self, actor_id: &ActorId, token: &str) -> Result<bool> {
        if let Some(actor) = self.actors.get(actor_id) {
            Ok(actor.verify_token(token))
        } else {
            Ok(false)
        }
    }
}


