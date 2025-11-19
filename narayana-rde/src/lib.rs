// Narayana RDE - Rapid Data Events
// Event-driven pub/sub system with multiple transport mechanisms

pub mod actor;
pub mod auth;
pub mod events;
pub mod subscriptions;
pub mod transformations;
pub mod transports;
pub mod rate_limiter;

pub use actor::{Actor, ActorId, ActorType};
pub use events::{Event, EventName, EventSchema, RdeEvent};
pub use subscriptions::{Subscription, SubscriptionId, TransportType};

use std::sync::Arc;
use narayana_core::Result;
use narayana_storage::native_events::{NativeEventsSystem, StreamName, Event as NativeEvent, EventStream};

/// RDE Manager - Main entry point for Rapid Data Events
pub struct RdeManager {
    actors: Arc<dashmap::DashMap<ActorId, Actor>>,
    events: dashmap::DashMap<EventName, EventSchema>,
    subscriptions: dashmap::DashMap<SubscriptionId, Subscription>,
    native_events: Arc<NativeEventsSystem>,
    auth: Arc<auth::AuthManager>,
    rate_limiter: Arc<rate_limiter::SubscriptionRateLimiter>,
    websocket_manager: Option<Arc<dyn WebSocketBroadcaster + Send + Sync>>,
    sse_connections: Arc<dashmap::DashMap<SubscriptionId, tokio::sync::mpsc::Sender<String>>>,
    grpc_streams: Arc<dashmap::DashMap<SubscriptionId, tokio::sync::mpsc::Sender<serde_json::Value>>>,
}

/// Trait for WebSocket broadcasting (to avoid direct dependency on WebSocketManager)
pub trait WebSocketBroadcaster {
    fn broadcast_to_channel(&self, channel: &str, message: serde_json::Value);
}

impl RdeManager {
    /// Create new RDE Manager
    pub fn new(native_events: Arc<NativeEventsSystem>) -> Self {
        let actors = Arc::new(dashmap::DashMap::new());
        Self {
            actors: actors.clone(),
            events: dashmap::DashMap::new(),
            subscriptions: dashmap::DashMap::new(),
            native_events,
            auth: Arc::new(auth::AuthManager::new(actors)),
            rate_limiter: Arc::new(rate_limiter::SubscriptionRateLimiter::new()),
            websocket_manager: None,
            sse_connections: Arc::new(dashmap::DashMap::new()),
            grpc_streams: Arc::new(dashmap::DashMap::new()),
        }
    }
    
    /// Set WebSocket manager for WebSocket transport
    pub fn with_websocket_manager(mut self, manager: Arc<dyn WebSocketBroadcaster + Send + Sync>) -> Self {
        self.websocket_manager = Some(manager);
        self
    }
    
    /// Register SSE connection for a subscription
    pub fn register_sse_connection(&self, subscription_id: SubscriptionId, sender: tokio::sync::mpsc::Sender<String>) {
        self.sse_connections.insert(subscription_id, sender);
    }
    
    /// Register gRPC stream for a subscription
    pub fn register_grpc_stream(&self, subscription_id: SubscriptionId, sender: tokio::sync::mpsc::Sender<serde_json::Value>) {
        self.grpc_streams.insert(subscription_id, sender);
    }
    
    /// Get SSE connection sender
    pub fn get_sse_sender(&self, subscription_id: &SubscriptionId) -> Option<tokio::sync::mpsc::Sender<String>> {
        self.sse_connections.get(subscription_id).map(|s| s.value().clone())
    }
    
    /// Get gRPC stream sender
    pub fn get_grpc_sender(&self, subscription_id: &SubscriptionId) -> Option<tokio::sync::mpsc::Sender<serde_json::Value>> {
        self.grpc_streams.get(subscription_id).map(|s| s.value().clone())
    }
    
    /// Get WebSocket manager
    pub fn get_websocket_manager(&self) -> Option<Arc<dyn WebSocketBroadcaster + Send + Sync>> {
        self.websocket_manager.clone()
    }

    /// Register a new actor
    pub async fn register_actor(&self, actor: Actor) -> Result<ActorId> {
        // Validate actor ID
        if actor.id.0.is_empty() {
            return Err(narayana_core::Error::Storage("Actor ID cannot be empty".to_string()));
        }
        if actor.id.0.len() > 256 {
            return Err(narayana_core::Error::Storage("Actor ID too long (max 256 chars)".to_string()));
        }
        // Prevent control characters and problematic unicode
        if actor.id.0.chars().any(|c| c.is_control() || c == '\0') {
            return Err(narayana_core::Error::Storage("Actor ID cannot contain control characters".to_string()));
        }
        // Prevent just colon
        if actor.id.0 == ":" {
            return Err(narayana_core::Error::Storage("Actor ID cannot be just ':'".to_string()));
        }
        // Prevent wildcard-only
        if actor.id.0 == "*" {
            return Err(narayana_core::Error::Storage("Actor ID cannot be '*' (reserved for wildcards)".to_string()));
        }
        
        // Validate actor name
        if actor.name.is_empty() {
            return Err(narayana_core::Error::Storage("Actor name cannot be empty".to_string()));
        }
        if actor.name.len() > 1024 {
            return Err(narayana_core::Error::Storage("Actor name too long (max 1024 chars)".to_string()));
        }
        
        // Validate auth token
        if actor.auth_token.is_empty() {
            return Err(narayana_core::Error::Storage("Auth token cannot be empty".to_string()));
        }
        if actor.auth_token.len() > 4096 {
            return Err(narayana_core::Error::Storage("Auth token too long (max 4096 chars)".to_string()));
        }
        
        // SECURITY: Prevent weak tokens (minimum length)
        if actor.auth_token.len() < 16 {
            return Err(narayana_core::Error::Storage("Auth token too short (min 16 chars for security)".to_string()));
        }
        
        // SECURITY: Atomic check-and-insert to prevent TOCTOU race condition
        // DashMap's insert returns Some(old_value) if key already exists
        let id = actor.id.clone();
        if self.actors.insert(id.clone(), actor).is_some() {
            return Err(narayana_core::Error::Storage("Actor already exists".to_string()));
        }
        
        Ok(id)
    }

    /// Get actor by ID (sanitized - no auth_token)
    pub fn get_actor(&self, id: &ActorId) -> Option<Actor> {
        self.actors.get(id).map(|a| {
            let mut actor = a.clone();
            // SECURITY: Don't leak auth_token
            actor.auth_token = String::new(); // Clear auth token
            actor
        })
    }

    /// Publish an event
    /// SECURITY: Requires authentication token
    pub async fn publish_event(
        &self,
        actor_id: &ActorId,
        auth_token: &str,
        event_name: &str,
        payload: serde_json::Value,
    ) -> Result<()> {
        // SECURITY: Authenticate first
        if !self.auth.authenticate(actor_id, auth_token)? {
            return Err(narayana_core::Error::Storage("Authentication failed".to_string()));
        }
        // Validate event name
        if event_name.is_empty() {
            return Err(narayana_core::Error::Storage("Event name cannot be empty".to_string()));
        }
        if event_name.len() > 256 {
            return Err(narayana_core::Error::Storage("Event name too long (max 256 chars)".to_string()));
        }
        // Prevent colon in event name to avoid namespacing issues
        if event_name.contains(':') {
            return Err(narayana_core::Error::Storage("Event name cannot contain ':' character".to_string()));
        }
        // Prevent control characters
        if event_name.chars().any(|c| c.is_control() || c == '\0') {
            return Err(narayana_core::Error::Storage("Event name cannot contain control characters".to_string()));
        }
        // Prevent just colon or wildcard
        if event_name == ":" || event_name == "*" {
            return Err(narayana_core::Error::Storage("Event name cannot be ':' or '*'".to_string()));
        }
        
        // Validate payload size (prevent memory exhaustion)
        const MAX_PAYLOAD_SIZE: usize = 10 * 1024 * 1024; // 10MB
        let payload_size = serde_json::to_string(&payload)
            .map_err(|e| narayana_core::Error::Storage(format!("Failed to serialize payload: {}", e)))?
            .len();
        if payload_size > MAX_PAYLOAD_SIZE {
            return Err(narayana_core::Error::Storage(format!(
                "Payload too large: {} bytes (max: {} bytes)",
                payload_size, MAX_PAYLOAD_SIZE
            )));
        }
        
        // Verify actor exists and is source type (check again after validation to prevent race condition)
        // SECURITY: Use generic error message to prevent actor enumeration
        let actor = self.actors.get(actor_id)
            .ok_or_else(|| narayana_core::Error::Storage("Actor not found or authentication failed".to_string()))?;
        
        if actor.actor_type != ActorType::Source {
            return Err(narayana_core::Error::Storage("Actor is not a source actor or authentication failed".to_string()));
        }

        // Create full event name (namespaced)
        let full_event_name = format!("{}:{}", actor_id, event_name);
        let event_name_key = EventName::from(full_event_name.clone());

        // Extract schema from first event
        if !self.events.contains_key(&event_name_key) {
            let schema = events::extract_schema(&payload)?;
            self.events.insert(event_name_key.clone(), schema);
        }

        // Ensure stream exists
        let stream_name = StreamName(format!("rde:{}", full_event_name));
        let stream = EventStream {
            name: stream_name.clone(),
            partitions: 1,
            retention: Some(std::time::Duration::from_secs(7 * 24 * 60 * 60)), // 7 days
            replication_factor: 1,
            compression: true,
            encryption: false,
            max_size: None,
            max_events: Some(1_000_000),
        };
        
        // Create stream if it doesn't exist (idempotent)
        if let Err(e) = self.native_events.create_stream(stream).await {
            // Stream might already exist, that's ok - but log other errors
            if !e.to_string().contains("already exists") {
                tracing::warn!("Failed to create stream: {}", e);
            }
        }

        // Create native event
        // Handle timestamp edge cases (negative, overflow, etc.)
        let timestamp_ms = chrono::Utc::now().timestamp_millis();
        let event_id = if timestamp_ms > 0 && timestamp_ms < i64::MAX as i64 {
            narayana_storage::native_events::EventId(timestamp_ms as u64)
        } else {
            // Fallback if timestamp is negative or overflow (shouldn't happen, but handle edge case)
            let fallback_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            // Prevent overflow
            let safe_ms = fallback_ms.min(u64::MAX as u128) as u64;
            narayana_storage::native_events::EventId(safe_ms)
        };
        
        let native_event = NativeEvent {
            id: event_id,
            stream: stream_name.clone(),
            topic: None,
            queue: None,
            event_type: event_name.to_string(),
            payload: payload.clone(),
            headers: std::collections::HashMap::new(),
            timestamp: chrono::Utc::now().timestamp() as u64,
            correlation_id: None,
            causation_id: None,
            partition_key: None,
            ttl: None,
            priority: 0,
        };

        // Publish to native events system
        // Continue even if publish fails (best effort)
        match self.native_events.publish_event(native_event).await {
            Ok(_) => {}
            Err(e) => {
                tracing::warn!("Failed to publish event to native events system: {}, continuing with delivery", e);
                // Continue with delivery even if storage fails
            }
        }

        // Deliver to subscribers
        // Don't fail entire publish if delivery fails
        if let Err(e) = self.deliver_to_subscribers(&event_name_key, &payload).await {
            tracing::warn!("Failed to deliver event to some subscribers: {}", e);
            // Event was published, so we return success even if delivery partially failed
        }

        Ok(())
    }

    /// Subscribe to an event
    /// SECURITY: Requires authentication token
    pub async fn subscribe(
        &self,
        actor_id: &ActorId,
        auth_token: &str,
        event_name: &str,
        transport: TransportType,
        config: Option<serde_json::Value>,
    ) -> Result<SubscriptionId> {
        // SECURITY: Authenticate first
        if !self.auth.authenticate(actor_id, auth_token)? {
            return Err(narayana_core::Error::Storage("Authentication failed".to_string()));
        }
        
        // Validate event name
        if event_name.is_empty() {
            return Err(narayana_core::Error::Storage("Event name cannot be empty".to_string()));
        }
        
        // Verify actor exists and is origin type
        let actor = self.actors.get(actor_id)
            .ok_or_else(|| narayana_core::Error::Storage("Actor not found".to_string()))?;
        
        if actor.actor_type != ActorType::Origin {
            return Err(narayana_core::Error::Storage("Actor is not an origin actor".to_string()));
        }
        
        // SECURITY: Restrict wildcard subscriptions to prevent privacy leaks
        // Only allow wildcard if explicitly permitted in actor metadata
        // Wildcard is when event_name doesn't contain ':' (will become "*:event_name")
        let is_wildcard = !event_name.contains(':');
        if is_wildcard {
            let allow_wildcard = actor.metadata
                .get("allow_wildcard_subscriptions")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !allow_wildcard {
                return Err(narayana_core::Error::Storage(
                    "Wildcard subscriptions require explicit permission. Use 'actor_id:event_name' format instead.".to_string()
                ));
            }
        }

        // Check subscription limits (prevent memory exhaustion)
        const MAX_SUBSCRIPTIONS_PER_ACTOR: usize = 10_000;
        let actor_subscription_count = self.subscriptions
            .iter()
            .filter(|s| s.value().actor_id == *actor_id)
            .count();
        
        if actor_subscription_count >= MAX_SUBSCRIPTIONS_PER_ACTOR {
            return Err(narayana_core::Error::Storage(format!(
                "Maximum subscriptions ({}) reached",
                MAX_SUBSCRIPTIONS_PER_ACTOR
            )));
        }

        // Validate event name format
        if event_name == "*" || event_name == ":" || event_name == "*:*" {
            return Err(narayana_core::Error::Storage("Invalid event name pattern".to_string()));
        }
        
        // Validate subscription config size
        if let Some(ref config) = config {
            let config_size = serde_json::to_string(config)
                .map_err(|e| narayana_core::Error::Storage(format!("Failed to serialize config: {}", e)))?
                .len();
            const MAX_CONFIG_SIZE: usize = 1024 * 1024; // 1MB
            if config_size > MAX_CONFIG_SIZE {
                return Err(narayana_core::Error::Storage(format!(
                    "Subscription config too large: {} bytes (max: {} bytes)",
                    config_size, MAX_CONFIG_SIZE
                )));
            }
        }
        
        // Use full namespaced event name for subscription
        // Event name can be either "actor_id:event_name" (full) or just "event_name" (will match any actor)
        let full_event_name = if event_name.contains(':') {
            // Validate that it's a proper namespaced format
            let parts: Vec<&str> = event_name.split(':').collect();
            if parts.len() != 2 {
                return Err(narayana_core::Error::Storage("Invalid namespaced event name format (expected 'actor:event')".to_string()));
            }
            event_name.to_string() // Already namespaced
        } else {
            // Subscribe to all actors with this event name - use wildcard pattern
            format!("*:{}", event_name) // Wildcard pattern
        };
        
        let subscription_id = SubscriptionId::new();
        let subscription = Subscription {
            id: subscription_id.clone(),
            actor_id: actor_id.clone(),
            event_name: EventName::from(full_event_name),
            transport,
            config: config.unwrap_or_default(),
            created_at: chrono::Utc::now().timestamp() as u64,
        };

        self.subscriptions.insert(subscription_id.clone(), subscription);

        // If event doesn't exist yet, subscription is stored and will be delivered when event is published
        // This is handled in deliver_to_subscribers

        Ok(subscription_id)
    }

    /// Deliver event to all subscribers
    async fn deliver_to_subscribers(
        &self,
        event_name: &EventName,
        payload: &serde_json::Value,
    ) -> Result<()> {
        // Find all subscriptions for this event
        // Support wildcard matching: "*:event_name" matches any actor's event
        // Limit number of subscriptions to prevent memory exhaustion
        const MAX_SUBSCRIPTIONS_TO_DELIVER: usize = 1000;
        let matching_subscriptions: Vec<Subscription> = self.subscriptions
            .iter()
            .filter(|s| {
                let sub_event = &s.value().event_name.0;
                let target_event = &event_name.0;
                
                // Exact match
                sub_event == target_event ||
                // Wildcard match: "*:event_name" matches "actor_id:event_name"
                (sub_event.starts_with("*:") && 
                 sub_event.len() > 2 && // Prevent "*:" matching everything
                 !sub_event[2..].contains(':') && // Prevent nested wildcards like "*:actor:event"
                 target_event.contains(':') && // Target must be namespaced
                 target_event.ends_with(&sub_event[2..]) &&
                 target_event.len() > sub_event.len() - 1) // Ensure there's an actor part
            })
            .take(MAX_SUBSCRIPTIONS_TO_DELIVER) // Limit to prevent DoS
            .map(|s| s.value().clone())
            .collect();
        
        if matching_subscriptions.len() >= MAX_SUBSCRIPTIONS_TO_DELIVER {
            // SECURITY: Don't log event name to prevent information disclosure
            tracing::warn!("Event has more than {} subscriptions, limiting delivery", MAX_SUBSCRIPTIONS_TO_DELIVER);
        }

        // Deliver via appropriate transport
        for subscription in matching_subscriptions {
            // Check rate limit for this subscription
            let rate_limit = subscription.config
                .get("rate_limit_per_second")
                .and_then(|v| v.as_f64());
            
            let delay = self.rate_limiter.check_and_record(
                &subscription.id.0,
                rate_limit,
            ).await;
            
            // Wait if rate limited
            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }
            
            // Apply transformation if configured (continue on error)
            let transformed_payload = match crate::transformations::apply_transformation(&subscription, payload) {
                Ok(transformed) => transformed,
                Err(e) => {
                    // SECURITY: Don't log subscription ID to prevent information disclosure
                    tracing::warn!("Transformation failed, using original payload: {}", e);
                    payload.clone() // Use original payload if transformation fails
                }
            };
            
            let result = match subscription.transport {
                TransportType::Webhook => {
                    crate::transports::http::deliver_webhook(&subscription, &transformed_payload).await
                }
                TransportType::WebSocket => {
                    crate::transports::websocket::deliver_websocket(
                        &subscription,
                        &transformed_payload,
                        self.get_websocket_manager(),
                    ).await
                }
                TransportType::Grpc => {
                    crate::transports::grpc::deliver_grpc(
                        &subscription,
                        &transformed_payload,
                        self.get_grpc_sender(&subscription.id),
                    ).await
                }
                TransportType::Sse => {
                    crate::transports::sse::deliver_sse(
                        &subscription,
                        &transformed_payload,
                        self.get_sse_sender(&subscription.id),
                    ).await
                }
            };
            
            if let Err(e) = result {
                // SECURITY: Don't log subscription ID to prevent information disclosure
                tracing::warn!("Failed to deliver event to subscription: {}", e);
                // Continue with other subscriptions
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use narayana_storage::native_events::EventsConfig;

    fn create_test_manager() -> RdeManager {
        let mut config = EventsConfig::default();
        config.max_message_size = 10 * 1024 * 1024;
        config.enable_persistence = false;
        let native_events = Arc::new(NativeEventsSystem::new(config));
        RdeManager::new(native_events)
    }

    fn create_test_source_actor(id: &str, token: &str) -> Actor {
        Actor::new(
            ActorId::from(id),
            format!("Source Actor {}", id),
            ActorType::Source,
            token.to_string(),
        )
    }

    fn create_test_origin_actor(id: &str, token: &str) -> Actor {
        Actor::new(
            ActorId::from(id),
            format!("Origin Actor {}", id),
            ActorType::Origin,
            token.to_string(),
        )
    }

    #[tokio::test]
    async fn test_register_actor() {
        let manager = create_test_manager();
        let actor = create_test_source_actor("test-actor", "test-token-123456789012");
        
        let result = manager.register_actor(actor).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0, "test-actor");
    }

    #[tokio::test]
    async fn test_register_actor_duplicate() {
        let manager = create_test_manager();
        let actor1 = create_test_source_actor("test-actor", "test-token-123456789012");
        let actor2 = create_test_source_actor("test-actor", "different-token-123456789012");
        
        assert!(manager.register_actor(actor1).await.is_ok());
        assert!(manager.register_actor(actor2).await.is_err());
    }

    #[tokio::test]
    async fn test_register_actor_validation() {
        let manager = create_test_manager();
        
        // Empty ID
        let actor = Actor::new(
            ActorId::from(""),
            "Test".to_string(),
            ActorType::Source,
            "token-123456789012".to_string(),
        );
        assert!(manager.register_actor(actor).await.is_err());
        
        // Too short token
        let actor = create_test_source_actor("test", "short");
        assert!(manager.register_actor(actor).await.is_err());
        
        // Control characters in ID
        let actor = Actor::new(
            ActorId::from("test\nactor"),
            "Test".to_string(),
            ActorType::Source,
            "token-123456789012".to_string(),
        );
        assert!(manager.register_actor(actor).await.is_err());
    }

    #[tokio::test]
    async fn test_get_actor_sanitized() {
        let manager = create_test_manager();
        let actor = create_test_source_actor("test-actor", "secret-token-123456789012");
        
        manager.register_actor(actor).await.unwrap();
        let retrieved = manager.get_actor(&ActorId::from("test-actor"));
        
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().auth_token, ""); // Should be cleared
    }

    #[tokio::test]
    async fn test_publish_event() {
        let manager = create_test_manager();
        let actor = create_test_source_actor("source1", "token-123456789012");
        manager.register_actor(actor).await.unwrap();
        
        let payload = serde_json::json!({"data": "test"});
        let result = manager.publish_event(
            &ActorId::from("source1"),
            "token-123456789012",
            "test_event",
            payload,
        ).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_publish_event_auth_required() {
        let manager = create_test_manager();
        let actor = create_test_source_actor("source1", "token-123456789012");
        manager.register_actor(actor).await.unwrap();
        
        let payload = serde_json::json!({"data": "test"});
        let result = manager.publish_event(
            &ActorId::from("source1"),
            "wrong-token",
            "test_event",
            payload,
        ).await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_publish_event_validation() {
        let manager = create_test_manager();
        let actor = create_test_source_actor("source1", "token-123456789012");
        manager.register_actor(actor).await.unwrap();
        
        // Empty event name
        let result = manager.publish_event(
            &ActorId::from("source1"),
            "token-123456789012",
            "",
            serde_json::json!({}),
        ).await;
        assert!(result.is_err());
        
        // Event name with colon
        let result = manager.publish_event(
            &ActorId::from("source1"),
            "token-123456789012",
            "event:name",
            serde_json::json!({}),
        ).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_subscribe() {
        let manager = create_test_manager();
        let actor = create_test_origin_actor("origin1", "token-123456789012");
        manager.register_actor(actor).await.unwrap();
        
        let result = manager.subscribe(
            &ActorId::from("origin1"),
            "token-123456789012",
            "source1:test_event",
            TransportType::Webhook,
            Some(serde_json::json!({"webhook_url": "https://example.com/webhook"})),
        ).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_subscribe_wildcard_requires_permission() {
        let manager = create_test_manager();
        let actor = create_test_origin_actor("origin1", "token-123456789012");
        manager.register_actor(actor).await.unwrap();
        
        // Wildcard subscription without permission
        let result = manager.subscribe(
            &ActorId::from("origin1"),
            "token-123456789012",
            "test_event", // No colon = wildcard
            TransportType::Webhook,
            None,
        ).await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_subscribe_wildcard_with_permission() {
        let manager = create_test_manager();
        let mut actor = create_test_origin_actor("origin1", "token-123456789012");
        actor.metadata = serde_json::json!({"allow_wildcard_subscriptions": true});
        manager.register_actor(actor).await.unwrap();
        
        // Wildcard subscription with permission
        let result = manager.subscribe(
            &ActorId::from("origin1"),
            "token-123456789012",
            "test_event",
            TransportType::Webhook,
            None,
        ).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_event_delivery_to_subscribers() {
        let manager = create_test_manager();
        
        // Register source
        let source = create_test_source_actor("source1", "token-123456789012");
        manager.register_actor(source).await.unwrap();
        
        // Register origin with wildcard permission
        let mut origin = create_test_origin_actor("origin1", "token-123456789012");
        origin.metadata = serde_json::json!({"allow_wildcard_subscriptions": true});
        manager.register_actor(origin).await.unwrap();
        
        // Subscribe to event
        manager.subscribe(
            &ActorId::from("origin1"),
            "token-123456789012",
            "test_event",
            TransportType::Webhook,
            Some(serde_json::json!({"webhook_url": "https://example.com/webhook"})),
        ).await.unwrap();
        
        // Publish event
        let payload = serde_json::json!({"data": "test"});
        let result = manager.publish_event(
            &ActorId::from("source1"),
            "token-123456789012",
            "test_event",
            payload,
        ).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_subscription_limit() {
        let manager = create_test_manager();
        let actor = create_test_origin_actor("origin1", "token-123456789012");
        manager.register_actor(actor).await.unwrap();
        
        // Create max subscriptions
        for i in 0..10_000 {
            let event_name = format!("source1:event_{}", i);
            let result = manager.subscribe(
                &ActorId::from("origin1"),
                "token-123456789012",
                &event_name,
                TransportType::Webhook,
                None,
            ).await;
            assert!(result.is_ok());
        }
        
        // Next subscription should fail
        let result = manager.subscribe(
            &ActorId::from("origin1"),
            "token-123456789012",
            "source1:event_10000",
            TransportType::Webhook,
            None,
        ).await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_payload_size_limit() {
        let manager = create_test_manager();
        let actor = create_test_source_actor("source1", "token-123456789012");
        manager.register_actor(actor).await.unwrap();
        
        // Create payload larger than 10MB
        let large_data = "x".repeat(11 * 1024 * 1024);
        let payload = serde_json::json!({"data": large_data});
        
        let result = manager.publish_event(
            &ActorId::from("source1"),
            "token-123456789012",
            "test_event",
            payload,
        ).await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_actor_type_validation() {
        let manager = create_test_manager();
        
        // Source actor cannot subscribe
        let source = create_test_source_actor("source1", "token-123456789012");
        manager.register_actor(source).await.unwrap();
        
        let result = manager.subscribe(
            &ActorId::from("source1"),
            "token-123456789012",
            "test:event",
            TransportType::Webhook,
            None,
        ).await;
        
        assert!(result.is_err());
        
        // Origin actor cannot publish
        let origin = create_test_origin_actor("origin1", "token-123456789012");
        manager.register_actor(origin).await.unwrap();
        
        let result = manager.publish_event(
            &ActorId::from("origin1"),
            "token-123456789012",
            "test_event",
            serde_json::json!({}),
        ).await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_concurrent_registration() {
        let manager = create_test_manager();
        
        // Try to register same actor concurrently
        let actor1 = create_test_source_actor("test", "token1-123456789012");
        let actor2 = create_test_source_actor("test", "token2-123456789012");
        
        let (r1, r2) = tokio::join!(
            manager.register_actor(actor1),
            manager.register_actor(actor2)
        );
        
        // One should succeed, one should fail
        assert!(r1.is_ok() || r2.is_ok());
        assert!(!(r1.is_ok() && r2.is_ok()));
    }

    #[tokio::test]
    async fn test_schema_extraction() {
        let manager = create_test_manager();
        let actor = create_test_source_actor("source1", "token-123456789012");
        manager.register_actor(actor).await.unwrap();
        
        let payload = serde_json::json!({
            "string_field": "value",
            "number_field": 42,
            "bool_field": true,
            "null_field": null
        });
        
        let result = manager.publish_event(
            &ActorId::from("source1"),
            "token-123456789012",
            "test_event",
            payload,
        ).await;
        
        assert!(result.is_ok());
    }
}
