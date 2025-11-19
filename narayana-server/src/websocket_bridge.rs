// WebSocket Event Bridge
// Bridges existing event systems to WebSocket connections

use narayana_api::websocket::{Channel, WsMessage};
use narayana_storage::{
    cognitive::{CognitiveBrain, CognitiveEvent},
    native_events::{Event, StreamName, TopicName},
    sensory_streams::{SensoryStreamManager, StreamEvent},
};
use crate::websocket_manager::WebSocketManager;
use std::sync::Arc;
use tokio::task::JoinHandle;
use parking_lot::RwLock;
use tracing::{info, warn, error, debug};
use serde_json::json;

/// WebSocket event bridge
pub struct WebSocketBridge {
    manager: Arc<WebSocketManager>,
    brain: Arc<CognitiveBrain>,
    // event_manager: Option<Arc<EventManager>>, // EventManager not available
    stream_manager: Option<Arc<SensoryStreamManager>>,
    handles: Arc<parking_lot::RwLock<Vec<JoinHandle<()>>>>,
}

// WebSocketManager is defined in websocket_manager.rs

impl WebSocketBridge {
    /// Create new WebSocket bridge
    pub fn new(
        manager: Arc<WebSocketManager>,
        brain: Arc<CognitiveBrain>,
        // event_manager: Option<Arc<EventManager>>, // EventManager not available
        stream_manager: Option<Arc<SensoryStreamManager>>,
    ) -> Self {
        Self {
            manager,
            brain,
            // event_manager,
            stream_manager,
            handles: Arc::new(parking_lot::RwLock::new(Vec::new())),
        }
    }

    /// Start all event bridges
    pub fn start(&mut self) {
        info!("Starting WebSocket event bridges...");

        // Bridge cognitive events
        self.start_cognitive_bridge();

        // Bridge native events if available
        // if self.event_manager.is_some() {
        if false { // EventManager not available
            self.start_native_events_bridge();
        }

        // Bridge stream events if available
        if self.stream_manager.is_some() {
            self.start_stream_events_bridge();
        }

        // Start periodic stats broadcaster
        self.start_stats_broadcaster();

        info!("WebSocket event bridges started");
    }

    /// Bridge cognitive brain events
    fn start_cognitive_bridge(&mut self) {
        let manager = self.manager.clone();
        let mut receiver = self.brain.subscribe();

        let handle = tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(event) => {
                        let channel = match &event {
                            CognitiveEvent::ThoughtCreated { thought_id: _ } => {
                                "brain:thoughts".to_string()
                            }
                            CognitiveEvent::ThoughtCompleted { thought_id: _ } => {
                                "brain:thoughts".to_string()
                            }
                            CognitiveEvent::MemoryFormed { memory_id: _, memory_type: _ } => {
                                "brain:memories".to_string()
                            }
                            CognitiveEvent::ExperienceStored { experience_id: _ } => {
                                "brain:experiences".to_string()
                            }
                            CognitiveEvent::PatternLearned { pattern_id: _ } => {
                                "brain:patterns".to_string()
                            }
                            CognitiveEvent::AssociationCreated { from: _, to: _ } => {
                                "brain:associations".to_string()
                            }
                            CognitiveEvent::MemoryRetrieved { memory_id: _ } => {
                                "brain:memories".to_string()
                            }
                            CognitiveEvent::ThoughtMerged { from: _, to: _ } => {
                                "brain:thoughts".to_string()
                            }
                            CognitiveEvent::ThoughtDiscarded { thought_id: _ } => {
                                "brain:thoughts".to_string()
                            }
                        };

                        let event_json = match serde_json::to_value(&event) {
                            Ok(json) => json!({
                                "type": format!("{:?}", event),
                                "data": json,
                            }),
                            Err(e) => {
                                error!("Failed to serialize cognitive event: {}", e);
                                continue;
                            }
                        };

                        let message = WsMessage::event_with_timestamp(
                            channel.clone(),
                            event_json,
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs(),
                        );

                        // Validate message can be serialized before broadcasting
                        if message.to_json().is_ok() {
                            let count = manager.broadcast_to_channel(&channel, message);
                            if count > 0 {
                                debug!("Broadcasted cognitive event to {} connections", count);
                            }
                        } else {
                            error!("Failed to serialize cognitive event message");
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        warn!("Cognitive event receiver closed, stopping bridge");
                        break;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!("Cognitive event receiver lagged, skipped {} events", skipped);
                        // Continue processing
                    }
                }
            }
        });

        self.handles.write().push(handle);
    }

    /// Bridge native events system
    fn start_native_events_bridge(&mut self) {
        // Note: This is a simplified bridge. In production, you'd want to
        // subscribe to specific streams/topics based on WebSocket subscriptions
        let manager = self.manager.clone();
        // EventManager not available
        warn!("EventManager not available, skipping native events bridge");
        return;

        // For now, we'll create a bridge that listens for events on common channels
        // In production, this would be more dynamic based on active subscriptions
        let handle = tokio::spawn(async move {
            // This is a placeholder - in production you'd subscribe to EventManager
            // and route events to appropriate WebSocket channels
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                // Event routing would happen here
            }
        });

        self.handles.write().push(handle);
    }

    /// Bridge sensory stream events
    fn start_stream_events_bridge(&mut self) {
        let manager = self.manager.clone();
        let stream_manager = self.stream_manager.as_ref().unwrap().clone();
        let mut receiver = stream_manager.subscribe();

        let handle = tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(event) => {
                        // Validate and sanitize stream_id
                        let (channel, timestamp) = match &event {
                            StreamEvent::DataReceived { stream_id, timestamp } => {
                                let stream_id_safe: String = stream_id
                                    .chars()
                                    .filter(|c| !c.is_control() && *c != ':' && *c != '/' && *c != '\\')
                                    .take(256)
                                    .collect();
                                if stream_id_safe.is_empty() {
                                    warn!("Stream ID became empty after sanitization");
                                    continue;
                                }
                                (format!("streams:{}:data", stream_id_safe), *timestamp)
                            }
                            StreamEvent::StreamStarted { stream_id } => {
                                let stream_id_safe: String = stream_id
                                    .chars()
                                    .filter(|c| !c.is_control() && *c != ':' && *c != '/' && *c != '\\')
                                    .take(256)
                                    .collect();
                                if stream_id_safe.is_empty() {
                                    warn!("Stream ID became empty after sanitization");
                                    continue;
                                }
                                let ts = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs();
                                (format!("streams:{}:events", stream_id_safe), ts)
                            }
                            StreamEvent::StreamStopped { stream_id } => {
                                let stream_id_safe: String = stream_id
                                    .chars()
                                    .filter(|c| !c.is_control() && *c != ':' && *c != '/' && *c != '\\')
                                    .take(256)
                                    .collect();
                                if stream_id_safe.is_empty() {
                                    warn!("Stream ID became empty after sanitization");
                                    continue;
                                }
                                let ts = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs();
                                (format!("streams:{}:events", stream_id_safe), ts)
                            }
                            StreamEvent::Error { stream_id, error: _ } => {
                                let stream_id_safe: String = stream_id
                                    .chars()
                                    .filter(|c| !c.is_control() && *c != ':' && *c != '/' && *c != '\\')
                                    .take(256)
                                    .collect();
                                if stream_id_safe.is_empty() {
                                    warn!("Stream ID became empty after sanitization");
                                    continue;
                                }
                                let ts = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs();
                                (format!("streams:{}:events", stream_id_safe), ts)
                            }
                        };

                        // StreamEvent doesn't implement Serialize, create JSON manually
                        let event_json = json!({
                            "type": "stream_event",
                            "timestamp": std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs(),
                        });

                        let message = WsMessage::event_with_timestamp(
                            channel.clone(),
                            event_json,
                            timestamp,
                        );

                        // Validate message can be serialized before broadcasting
                        if message.to_json().is_ok() {
                            let count = manager.broadcast_to_channel(&channel, message);
                            if count > 0 {
                                debug!("Broadcasted stream event to {} connections", count);
                            }
                        } else {
                            error!("Failed to serialize stream event message");
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        warn!("Stream event receiver closed, stopping bridge");
                        break;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!("Stream event receiver lagged, skipped {} events", skipped);
                        // Continue processing
                    }
                }
            }
        });

        self.handles.write().push(handle);
    }

    /// Broadcast database event
    pub fn broadcast_database_event(
        &self,
        db_name: &str,
        table_name: Option<&str>,
        event_type: &str,
        data: serde_json::Value,
    ) {
        // Validate inputs to prevent channel name injection
        if db_name.is_empty() {
            warn!("Attempted to broadcast database event with empty db_name");
            return;
        }
        if db_name.len() > 256 {
            warn!("Database name too long: {} (max 256)", db_name.len());
            return;
        }
        // Sanitize db_name - remove invalid characters
        let db_name_safe: String = db_name
            .chars()
            .filter(|c| !c.is_control() && *c != ':' && *c != '/' && *c != '\\')
            .take(256)
            .collect();
        
        if db_name_safe.is_empty() {
            warn!("Database name became empty after sanitization");
            return;
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Broadcast to database-level channel
        let db_channel = format!("db:{}:events", db_name_safe);
        let message = WsMessage::event_with_timestamp(
            db_channel.clone(),
            json!({
                "type": event_type,
                "database": db_name_safe,
                "data": data,
            }),
            timestamp,
        );
        // Validate message can be serialized before broadcasting
        if message.to_json().is_ok() {
            self.manager.broadcast_to_channel(&db_channel, message);
        } else {
            error!("Failed to serialize database event message");
        }

        // Broadcast to table-level channel if table specified
        if let Some(table) = table_name {
            // Validate and sanitize table name
            if table.is_empty() {
                warn!("Attempted to broadcast database event with empty table_name");
                return;
            }
            if table.len() > 256 {
                warn!("Table name too long: {} (max 256)", table.len());
                return;
            }
            let table_safe: String = table
                .chars()
                .filter(|c| !c.is_control() && *c != ':' && *c != '/' && *c != '\\')
                .take(256)
                .collect();
            
            if table_safe.is_empty() {
                warn!("Table name became empty after sanitization");
                return;
            }

            // Sanitize event_type
            let event_type_safe: String = event_type
                .chars()
                .filter(|c| !c.is_control() && *c != ':' && *c != '/' && *c != '\\')
                .take(256)
                .collect();

            let table_channel = format!("db:{}:table:{}:events", db_name_safe, table_safe);
            let message = WsMessage::event_with_timestamp(
                table_channel.clone(),
                json!({
                    "type": event_type,
                    "database": db_name_safe,
                    "table": table_safe,
                    "data": data,
                }),
                timestamp,
            );
            // Validate message can be serialized
            if message.to_json().is_ok() {
                self.manager.broadcast_to_channel(&table_channel, message);
            } else {
                error!("Failed to serialize table event message");
            }

            // Broadcast to specific event type channel
            let event_channel = format!("db:{}:table:{}:{}", db_name_safe, table_safe, event_type_safe);
            let message = WsMessage::event_with_timestamp(
                event_channel.clone(),
                json!({
                    "type": event_type,
                    "database": db_name_safe,
                    "table": table_safe,
                    "data": data,
                }),
                timestamp,
            );
            // Validate message can be serialized
            if message.to_json().is_ok() {
                self.manager.broadcast_to_channel(&event_channel, message);
            } else {
                error!("Failed to serialize event type channel message");
            }
        }
    }

    /// Broadcast worker event
    pub fn broadcast_worker_event(
        &self,
        worker_id: Option<&str>,
        event_type: &str,
        data: serde_json::Value,
    ) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Broadcast to general workers channel
        let workers_channel = "workers:execution".to_string();
        let message = WsMessage::event_with_timestamp(
            workers_channel.clone(),
            json!({
                "type": event_type,
                "data": data,
            }),
            timestamp,
        );
        // Validate message can be serialized
        if message.to_json().is_ok() {
            self.manager.broadcast_to_channel(&workers_channel, message);
        } else {
            error!("Failed to serialize worker event message");
            return;
        }

        // Broadcast to worker-specific channel if worker_id provided
        if let Some(worker) = worker_id {
            // Validate and sanitize worker_id
            if worker.is_empty() {
                warn!("Attempted to broadcast worker event with empty worker_id");
                return;
            }
            if worker.len() > 256 {
                warn!("Worker ID too long: {} (max 256)", worker.len());
                return;
            }
            let worker_safe: String = worker
                .chars()
                .filter(|c| !c.is_control() && *c != ':' && *c != '/' && *c != '\\')
                .take(256)
                .collect();
            
            if worker_safe.is_empty() {
                warn!("Worker ID became empty after sanitization");
                return;
            }

            let worker_channel = format!("worker:{}:events", worker_safe);
            let message = WsMessage::event_with_timestamp(
                worker_channel.clone(),
                json!({
                    "type": event_type,
                    "worker_id": worker_safe,
                    "data": data,
                }),
                timestamp,
            );
            // Validate message can be serialized
            if message.to_json().is_ok() {
                self.manager.broadcast_to_channel(&worker_channel, message);
            } else {
                error!("Failed to serialize worker-specific event message");
            }
        }
    }

    /// Start periodic stats broadcaster
    fn start_stats_broadcaster(&mut self) {
        let manager = self.manager.clone();
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
            
            loop {
                interval.tick().await;
                
                // Get stats from atomic counters
                use crate::http::{TOTAL_QUERIES, TOTAL_ROWS_READ, TOTAL_ROWS_INSERTED, TOTAL_QUERY_TIME_MS};
                use std::sync::atomic::Ordering;
                
                let total_queries = TOTAL_QUERIES.load(Ordering::Relaxed);
                let total_rows_read = TOTAL_ROWS_READ.load(Ordering::Relaxed);
                let total_rows_inserted = TOTAL_ROWS_INSERTED.load(Ordering::Relaxed);
                let total_query_time = TOTAL_QUERY_TIME_MS.load(Ordering::Relaxed);
                
                let avg_latency_ms = if total_queries > 0 {
                    total_query_time as f64 / total_queries as f64
                } else {
                    0.0
                };
                
                let stats_data = serde_json::json!({
                    "type": "stats_update",
                    "data": {
                        "total_queries": total_queries,
                        "avg_latency_ms": avg_latency_ms,
                        "total_rows_read": total_rows_read,
                        "total_rows_inserted": total_rows_inserted,
                    }
                });
                
                let message = WsMessage::event_with_timestamp(
                    "system:stats".to_string(),
                    stats_data,
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                );
                
                if message.to_json().is_ok() {
                    let channel = "system:stats".to_string();
                    let count = manager.broadcast_to_channel(&channel, message);
                    if count > 0 {
                        debug!("Broadcasted stats update to {} connections", count);
                    }
                }
            }
        });
        
        self.handles.write().push(handle);
    }

    /// Shutdown all bridges
    pub fn shutdown(&self) {
        info!("Shutting down WebSocket event bridges...");
        let handles = self.handles.write();
        for handle in handles.iter() {
            handle.abort();
        }
        info!("WebSocket event bridges shut down");
    }
}

