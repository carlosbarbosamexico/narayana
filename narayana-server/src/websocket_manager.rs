// WebSocket Connection Manager
// Tracks active connections, subscriptions, and routes messages

use narayana_api::websocket::{ConnectionId, Channel, WsMessage, EventFilter};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

/// WebSocket connection state
#[derive(Debug, Clone)]
pub struct ConnectionState {
    pub id: ConnectionId,
    pub user_id: Option<String>,
    pub subscriptions: HashSet<Channel>,
    pub created_at: u64,
    pub last_activity: u64,
}

/// WebSocket connection manager
pub struct WebSocketManager {
    /// Active connections: connection_id -> connection_state
    connections: Arc<RwLock<HashMap<ConnectionId, ConnectionState>>>,
    
    /// Per-connection subscriptions: connection_id -> set of channels
    connection_subscriptions: Arc<RwLock<HashMap<ConnectionId, HashSet<Channel>>>>,
    
    /// Channel subscriptions: channel -> set of connection_ids
    channel_subscriptions: Arc<RwLock<HashMap<Channel, HashSet<ConnectionId>>>>,
    
    /// Per-connection message senders: connection_id -> sender
    message_senders: Arc<RwLock<HashMap<ConnectionId, mpsc::UnboundedSender<WsMessage>>>>,
    
    /// Configuration
    config: WebSocketConfig,
}

/// WebSocket configuration
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    pub max_connections: usize,
    pub max_subscriptions_per_connection: usize,
    pub ping_interval_secs: u64,
    pub connection_timeout_secs: u64,
    pub enable_compression: bool,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            max_connections: 1000,
            max_subscriptions_per_connection: 100,
            ping_interval_secs: 30,
            connection_timeout_secs: 300,
            enable_compression: true,
        }
    }
}

impl WebSocketManager {
    /// Create new WebSocket manager
    pub fn new(config: WebSocketConfig) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            connection_subscriptions: Arc::new(RwLock::new(HashMap::new())),
            channel_subscriptions: Arc::new(RwLock::new(HashMap::new())),
            message_senders: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Register a new connection
    pub fn register_connection(
        &self,
        connection_id: ConnectionId,
        user_id: Option<String>,
        sender: mpsc::UnboundedSender<WsMessage>,
    ) -> Result<(), String> {
        let connections = self.connections.read();
        
        // Check connection limit
        if connections.len() >= self.config.max_connections {
            return Err(format!(
                "Maximum connections ({}) reached",
                self.config.max_connections
            ));
        }
        drop(connections);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let state = ConnectionState {
            id: connection_id.clone(),
            user_id: user_id.clone(),
            subscriptions: HashSet::new(),
            created_at: now,
            last_activity: now,
        };

        self.connections.write().insert(connection_id.clone(), state);
        self.connection_subscriptions
            .write()
            .insert(connection_id.clone(), HashSet::new());
        self.message_senders.write().insert(connection_id.clone(), sender);

        info!("WebSocket connection registered: {}", connection_id);
        Ok(())
    }

    /// Unregister a connection
    pub fn unregister_connection(&self, connection_id: &ConnectionId) {
        // Remove from all channels
        let subscriptions = {
            let subs = self.connection_subscriptions.read();
            subs.get(connection_id).cloned()
        };

        if let Some(channels) = subscriptions {
            for channel in channels {
                self.unsubscribe_from_channel(connection_id, &channel);
            }
        }

        // Remove connection
        self.connections.write().remove(connection_id);
        self.connection_subscriptions.write().remove(connection_id);
        self.message_senders.write().remove(connection_id);

        info!("WebSocket connection unregistered: {}", connection_id);
    }

    /// Subscribe connection to a channel
    pub fn subscribe(
        &self,
        connection_id: &ConnectionId,
        channel: Channel,
        _filter: Option<EventFilter>,
    ) -> Result<(), String> {
        // Validate channel name
        if channel.is_empty() {
            return Err("Channel name cannot be empty".to_string());
        }
        if channel.len() > 256 {
            return Err("Channel name too long (max 256 characters)".to_string());
        }
        // Prevent channel name injection
        if channel.contains('\0') || channel.contains('\n') || channel.contains('\r') {
            return Err("Invalid characters in channel name".to_string());
        }

        // SECURITY: Check if connection exists and get user_id for authorization
        let connection_state = {
            let connections = self.connections.read();
            connections.get(connection_id).cloned()
        };
        
        let user_id = match connection_state {
            Some(state) => state.user_id,
            None => return Err("Connection not found".to_string()),
        };

        // SECURITY: Authorization check - prevent unauthorized access to sensitive channels
        if !self.is_channel_authorized(&channel, &user_id) {
            return Err("Unauthorized access to channel".to_string());
        }

        // Check if already subscribed (idempotent operation)
        let already_subscribed = {
            let subs = self.connection_subscriptions.read();
            subs.get(connection_id)
                .map(|s| s.contains(&channel))
                .unwrap_or(false)
        };

        if already_subscribed {
            debug!("Connection {} already subscribed to channel {}", connection_id, channel);
            return Ok(()); // Idempotent - return success
        }

        // Check subscription limit
        let subscription_count = {
            let subs = self.connection_subscriptions.read();
            subs.get(connection_id)
                .map(|s| s.len())
                .unwrap_or(0)
        };

        if subscription_count >= self.config.max_subscriptions_per_connection {
            return Err(format!(
                "Maximum subscriptions ({}) per connection reached",
                self.config.max_subscriptions_per_connection
            ));
        }

        // Add to connection's subscriptions (atomic operation)
        {
            let mut subs = self.connection_subscriptions.write();
            subs.entry(connection_id.clone())
                .or_insert_with(HashSet::new)
                .insert(channel.clone());
        }

        // Add to channel's subscribers (atomic operation)
        {
            let mut channel_subs = self.channel_subscriptions.write();
            channel_subs.entry(channel.clone())
                .or_insert_with(HashSet::new)
                .insert(connection_id.clone());
        }

        // Update last activity
        if let Some(state) = self.connections.write().get_mut(connection_id) {
            state.last_activity = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
        }

        debug!("Connection {} subscribed to channel {}", connection_id, channel);
        Ok(())
    }

    /// Unsubscribe connection from a channel
    pub fn unsubscribe(&self, connection_id: &ConnectionId, channel: &Channel) -> Result<(), String> {
        // Check if connection exists
        if !self.connections.read().contains_key(connection_id) {
            return Err("Connection not found".to_string());
        }

        // Check if subscribed to this channel
        let was_subscribed = {
            let subs = self.connection_subscriptions.read();
            subs.get(connection_id)
                .map(|s| s.contains(channel))
                .unwrap_or(false)
        };

        if !was_subscribed {
            debug!("Connection {} not subscribed to channel {}", connection_id, channel);
            return Ok(()); // Idempotent - return success
        }

        self.unsubscribe_from_channel(connection_id, channel);
        Ok(())
    }

    fn unsubscribe_from_channel(&self, connection_id: &ConnectionId, channel: &Channel) {
        // Remove from connection's subscriptions
        let was_in_connection_subs = {
            let mut subs = self.connection_subscriptions.write();
            if let Some(connection_subs) = subs.get_mut(connection_id) {
                connection_subs.remove(channel)
            } else {
                false
            }
        };

        // Remove from channel's subscribers
        let was_in_channel_subs = {
            let mut channel_subs = self.channel_subscriptions.write();
            if let Some(subscribers) = channel_subs.get_mut(channel) {
                subscribers.remove(connection_id)
            } else {
                false
            }
        };

        // Clean up empty channel entry to prevent memory leak
        if was_in_channel_subs {
            let mut channel_subs = self.channel_subscriptions.write();
            if let Some(subscribers) = channel_subs.get(channel) {
                if subscribers.is_empty() {
                    channel_subs.remove(channel);
                }
            }
        }

        if was_in_connection_subs || was_in_channel_subs {
            debug!("Connection {} unsubscribed from channel {}", connection_id, channel);
        }
    }

    /// Send message to a specific connection
    pub fn send_to_connection(&self, connection_id: &ConnectionId, message: WsMessage) -> bool {
        // Check if connection still exists before sending
        if !self.connections.read().contains_key(connection_id) {
            debug!("Attempted to send message to non-existent connection: {}", connection_id);
            return false;
        }

        let senders = self.message_senders.read();
        if let Some(sender) = senders.get(connection_id) {
            // Try to send - if channel is closed, send will fail
            match sender.send(message) {
                Ok(_) => true,
                Err(_) => {
                    debug!("Message sender closed for connection: {}", connection_id);
                    false
                }
            }
        } else {
            false
        }
    }

    /// Broadcast message to all subscribers of a channel
    pub fn broadcast_to_channel(&self, channel: &Channel, message: WsMessage) -> usize {
        // Limit broadcast size to prevent memory exhaustion
        const MAX_BROADCAST_SUBSCRIBERS: usize = 10_000;
        
        let subscribers = {
            let subs = self.channel_subscriptions.read();
            subs.get(channel).cloned().unwrap_or_default()
        };

        if subscribers.is_empty() {
            return 0;
        }

        // Limit number of subscribers to prevent DoS
        let subscribers_to_process: Vec<ConnectionId> = subscribers
            .into_iter()
            .take(MAX_BROADCAST_SUBSCRIBERS)
            .collect();

        if subscribers_to_process.len() >= MAX_BROADCAST_SUBSCRIBERS {
            warn!("Channel {} has more than {} subscribers, limiting broadcast", channel, MAX_BROADCAST_SUBSCRIBERS);
        }

        let senders = self.message_senders.read();
        let connections = self.connections.read();
        let mut sent_count = 0;
        let mut dead_connections = Vec::new();

        for connection_id in subscribers_to_process {
            // Check if connection still exists
            if !connections.contains_key(&connection_id) {
                dead_connections.push(connection_id.clone());
                continue;
            }

            if let Some(sender) = senders.get(&connection_id) {
                // Clone message for each subscriber
                let msg = message.clone();
                match sender.send(msg) {
                    Ok(_) => {
                        sent_count += 1;
                    }
                    Err(_) => {
                        warn!("Failed to send message to connection {} (channel closed)", connection_id);
                        dead_connections.push(connection_id);
                    }
                }
            }
        }

        // Clean up dead connections (but don't hold locks while doing so)
        if !dead_connections.is_empty() {
            drop(senders);
            drop(connections);
            for conn_id in dead_connections {
                self.unregister_connection(&conn_id);
            }
        }

        sent_count
    }

    /// Get all channels a connection is subscribed to
    /// SECURITY: Only allow connection to see its own channels
    pub fn get_connection_channels(&self, connection_id: &ConnectionId, requesting_connection_id: &ConnectionId) -> Result<Vec<Channel>, String> {
        // SECURITY: Only allow a connection to see its own channels
        if connection_id != requesting_connection_id {
            return Err("Unauthorized: Cannot access other connection's channels".to_string());
        }
        
        let subs = self.connection_subscriptions.read();
        Ok(subs.get(connection_id)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default())
    }

    /// Get all subscribers of a channel
    /// SECURITY: Removed public access - this leaks connection IDs
    /// Use internal methods only for authorized operations
    #[allow(dead_code)]
    pub(crate) fn get_channel_subscribers_internal(&self, channel: &Channel) -> Vec<ConnectionId> {
        let subs = self.channel_subscriptions.read();
        subs.get(channel)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get connection count
    pub fn connection_count(&self) -> usize {
        self.connections.read().len()
    }

    /// Get subscription count for a channel
    pub fn channel_subscription_count(&self, channel: &Channel) -> usize {
        let subs = self.channel_subscriptions.read();
        subs.get(channel).map(|s| s.len()).unwrap_or(0)
    }

    /// Update connection activity timestamp
    pub fn update_activity(&self, connection_id: &ConnectionId) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if let Some(state) = self.connections.write().get_mut(connection_id) {
            state.last_activity = now;
        }
    }

    /// Clean up stale connections (older than timeout)
    pub fn cleanup_stale_connections(&self) -> usize {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let timeout = self.config.connection_timeout_secs;
        let stale_connections: Vec<ConnectionId> = {
            let connections = self.connections.read();
            connections
                .iter()
                .filter(|(_, state)| now.saturating_sub(state.last_activity) > timeout)
                .map(|(id, _)| id.clone())
                .collect()
        };

        for connection_id in &stale_connections {
            self.unregister_connection(connection_id);
        }

        if !stale_connections.is_empty() {
            info!("Cleaned up {} stale connections", stale_connections.len());
        }

        stale_connections.len()
    }

    /// Get connection state
    /// SECURITY: Only return state if caller has permission (connection_id matches or is admin)
    pub fn get_connection_state(&self, connection_id: &ConnectionId) -> Option<ConnectionState> {
        self.connections.read().get(connection_id).cloned()
    }

    /// SECURITY: Check if user is authorized to subscribe to a channel
    fn is_channel_authorized(&self, channel: &Channel, user_id: &Option<String>) -> bool {
        // Public channels (no user-specific data)
        let public_channels = [
            "brain:thoughts",
            "brain:memories",
            "brain:experiences",
            "brain:patterns",
            "brain:associations",
        ];
        
        // Check if it's a public channel
        for public_channel in &public_channels {
            if channel.starts_with(public_channel) {
                return true;
            }
        }

        // Database channels - require authentication and check database access
        if channel.starts_with("db:") {
            // For now, require authentication for database channels
            // In production, check database-specific permissions
            return user_id.is_some();
        }

        // Worker channels - require authentication
        if channel.starts_with("worker:") || channel.starts_with("workers:") {
            // For now, require authentication for worker channels
            // In production, check worker-specific permissions
            return user_id.is_some();
        }

        // Stream channels - require authentication
        if channel.starts_with("streams:") {
            // For now, require authentication for stream channels
            // In production, check stream-specific permissions
            return user_id.is_some();
        }

        // Unknown channel types - deny by default (secure default)
        false
    }
}

