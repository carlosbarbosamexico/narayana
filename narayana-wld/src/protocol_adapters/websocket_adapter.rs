//! WebSocket protocol adapter

use crate::event_transformer::{WorldEvent, WorldAction};
use crate::world_broker::WorldBrokerHandle;
use narayana_core::Error;
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use tokio::sync::broadcast;
use parking_lot::RwLock;
use tracing::{info, warn, debug};

/// WebSocket adapter for real-time bidirectional communication
pub struct WebSocketAdapter {
    path: String,
    event_sender: Arc<RwLock<Option<broadcast::Sender<WorldEvent>>>>,
    is_running: Arc<RwLock<bool>>,
}

impl WebSocketAdapter {
    pub fn new(path: String) -> Self {
        Self {
            path,
            event_sender: Arc::new(RwLock::new(None)),
            is_running: Arc::new(RwLock::new(false)),
        }
    }
}

#[async_trait]
impl crate::protocol_adapters::ProtocolAdapter for WebSocketAdapter {
    fn protocol_name(&self) -> &str {
        "websocket"
    }

    async fn start(&self, _broker: WorldBrokerHandle) -> Result<(), Error> {
        if *self.is_running.read() {
            return Err(Error::Storage("WebSocket adapter already running".to_string()));
        }

        let (sender, _) = broadcast::channel(1000);
        *self.event_sender.write() = Some(sender.clone());
        *self.is_running.write() = true;

        info!("WebSocket adapter started on path: {}", self.path);
        Ok(())
    }

    async fn stop(&self) -> Result<(), Error> {
        *self.is_running.write() = false;
        *self.event_sender.write() = None;
        info!("WebSocket adapter stopped");
        Ok(())
    }

    async fn send_action(&self, action: WorldAction) -> Result<(), Error> {
        // Actions are sent through WebSocket connections
        // This would be handled by individual connection handlers
        debug!("WebSocket adapter send_action: {:?}", action);
        Ok(())
    }

    fn subscribe_events(&self) -> broadcast::Receiver<WorldEvent> {
        self.event_sender.read()
            .as_ref()
            .map(|s| s.subscribe())
            .unwrap_or_else(|| {
                let (_, receiver) = broadcast::channel(1);
                receiver
            })
    }
}

// WebSocket handling would be implemented when integrated with HTTP server
// For now, this is a placeholder adapter

fn parse_event_from_json(payload: &JsonValue) -> Result<WorldEvent, Error> {
    // Similar to HTTP adapter parsing
    let event_type = payload.get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Storage("Missing 'type' field".to_string()))?;

    match event_type {
        "sensor" => {
            let source = payload.get("source")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Storage("Missing 'source' field".to_string()))?
                .to_string();
            let data = payload.get("data").cloned().unwrap_or(JsonValue::Object(serde_json::Map::new()));
            let timestamp = payload.get("timestamp")
                .and_then(|v| v.as_u64())
                .unwrap_or_else(|| {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                });
            Ok(WorldEvent::SensorData { source, data, timestamp })
        }
        "user_input" => {
            let user_id = payload.get("user_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Storage("Missing 'user_id' field".to_string()))?
                .to_string();
            let input = payload.get("input")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Storage("Missing 'input' field".to_string()))?
                .to_string();
            let context = payload.get("context").cloned().unwrap_or(JsonValue::Object(serde_json::Map::new()));
            Ok(WorldEvent::UserInput { user_id, input, context })
        }
        "system" => {
            let event_type = payload.get("event_type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Storage("Missing 'event_type' field".to_string()))?
                .to_string();
            let payload_val = payload.get("payload").cloned().unwrap_or(JsonValue::Object(serde_json::Map::new()));
            Ok(WorldEvent::SystemEvent { event_type, payload: payload_val })
        }
        "command" => {
            let command = payload.get("command")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Storage("Missing 'command' field".to_string()))?
                .to_string();
            let args = payload.get("args").cloned().unwrap_or(JsonValue::Object(serde_json::Map::new()));
            Ok(WorldEvent::Command { command, args })
        }
        _ => Err(Error::Storage(format!("Unknown event type: {}", event_type))),
    }
}

