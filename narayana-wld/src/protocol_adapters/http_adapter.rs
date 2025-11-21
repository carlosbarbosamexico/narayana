//! HTTP/REST protocol adapter

use crate::event_transformer::{WorldEvent, WorldAction};
use crate::world_broker::WorldBrokerHandle;
use narayana_core::Error;
use async_trait::async_trait;
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde_json::{json, Value as JsonValue};
use std::sync::Arc;
use tokio::sync::broadcast;
use parking_lot::RwLock;
use tracing::{info, warn, error};

/// HTTP adapter for REST API communication
pub struct HttpAdapter {
    port: u16,
    event_sender: Arc<RwLock<Option<broadcast::Sender<WorldEvent>>>>,
    is_running: Arc<RwLock<bool>>,
}

impl HttpAdapter {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            event_sender: Arc::new(RwLock::new(None)),
            is_running: Arc::new(RwLock::new(false)),
        }
    }
}

#[async_trait]
impl crate::protocol_adapters::ProtocolAdapter for HttpAdapter {
    fn protocol_name(&self) -> &str {
        "http"
    }

    async fn start(&self, broker: WorldBrokerHandle) -> Result<(), Error> {
        if *self.is_running.read() {
            return Err(Error::Storage("HTTP adapter already running".to_string()));
        }

        let (sender, _) = broadcast::channel(1000);
        *self.event_sender.write() = Some(sender.clone());
        *self.is_running.write() = true;

        let port = self.port;
        let broker_clone = broker.clone();

        // Create router
        let app = Router::new()
            .route("/world/events", post(handle_event))
            .route("/world/health", get(health_check))
            .with_state(HttpAdapterState {
                event_sender: sender,
                broker: broker_clone,
            });

        // Start server
        let addr = format!("0.0.0.0:{}", port);
        info!("Starting HTTP adapter on {}", addr);

        tokio::spawn(async move {
            let listener = match tokio::net::TcpListener::bind(&addr).await {
                Ok(l) => l,
                Err(e) => {
                    error!("Failed to bind HTTP adapter on {}: {}", addr, e);
                    return;
                }
            };
            
            if let Err(e) = axum::serve(listener, app).await {
                error!("HTTP adapter server error: {}", e);
            }
        });

        Ok(())
    }

    async fn stop(&self) -> Result<(), Error> {
        *self.is_running.write() = false;
        *self.event_sender.write() = None;
        info!("HTTP adapter stopped");
        Ok(())
    }

    async fn send_action(&self, _action: WorldAction) -> Result<(), Error> {
        // HTTP adapter typically receives events, not sends actions
        // Actions would be sent via HTTP client to external systems
        warn!("HTTP adapter send_action called - not implemented for server mode");
        Ok(())
    }

    fn subscribe_events(&self) -> broadcast::Receiver<WorldEvent> {
        self.event_sender.read()
            .as_ref()
            .map(|s| s.subscribe())
            .unwrap_or_else(|| {
                // Return a closed receiver if not started
                let (_, receiver) = broadcast::channel(1);
                receiver
            })
    }
}

#[derive(Clone)]
struct HttpAdapterState {
    event_sender: broadcast::Sender<WorldEvent>,
    broker: WorldBrokerHandle,
}

async fn handle_event(
    State(state): State<HttpAdapterState>,
    Json(payload): Json<JsonValue>,
) -> Result<Json<JsonValue>, StatusCode> {
    // Validate payload size to prevent DoS
    let payload_str = serde_json::to_string(&payload).unwrap_or_default();
    if payload_str.len() > 10_000_000 {
        warn!("Payload too large: {} bytes", payload_str.len());
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    // Parse event from JSON
    let event = match parse_event_from_json(&payload) {
        Ok(e) => e,
        Err(e) => {
            warn!("Failed to parse event: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // Send to broker
    if let Err(e) = state.broker.process_world_event(event.clone()).await {
        warn!("Failed to process event: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Also send to event channel (non-blocking)
    if state.event_sender.send(event).is_err() {
        warn!("Event channel full, message dropped");
    }

    Ok(Json(json!({"status": "ok"})))
}

async fn health_check() -> Json<JsonValue> {
    Json(json!({"status": "healthy", "protocol": "http"}))
}

fn parse_event_from_json(payload: &JsonValue) -> Result<WorldEvent, Error> {
    let event_type = payload.get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Storage("Missing 'type' field".to_string()))?;

    // Validate event type
    if event_type.is_empty() || event_type.len() > 64 {
        return Err(Error::Storage("Invalid event type".to_string()));
    }

    match event_type {
        "sensor" => {
            let source = payload.get("source")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Storage("Missing 'source' field".to_string()))?
                .to_string();
            
            // Validate source
            if source.is_empty() || source.len() > 256 {
                return Err(Error::Storage("Invalid sensor source".to_string()));
            }
            
            let data = payload.get("data")
                .cloned()
                .unwrap_or(json!({}));
            
            // Validate data size
            let data_str = serde_json::to_string(&data).unwrap_or_default();
            if data_str.len() > 1_000_000 {
                return Err(Error::Storage("Sensor data too large".to_string()));
            }
            
            let timestamp = payload.get("timestamp")
                .and_then(|v| v.as_u64())
                .unwrap_or_else(|| {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                });
            
            // Validate timestamp is reasonable
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            const MAX_FUTURE_SKEW: u64 = 3600; // 1 hour
            const MAX_PAST_AGE: u64 = 31536000; // 1 year
            let validated_timestamp = if timestamp > now + MAX_FUTURE_SKEW || 
                                       now.saturating_sub(timestamp) > MAX_PAST_AGE {
                warn!("Invalid timestamp {}, using current time", timestamp);
                now
            } else {
                timestamp
            };

            Ok(WorldEvent::SensorData { source, data, timestamp: validated_timestamp })
        }
        "user_input" => {
            let user_id = payload.get("user_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Storage("Missing 'user_id' field".to_string()))?
                .to_string();
            
            // Validate user_id
            if user_id.is_empty() || user_id.len() > 256 {
                return Err(Error::Storage("Invalid user_id".to_string()));
            }
            
            let input = payload.get("input")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Storage("Missing 'input' field".to_string()))?
                .to_string();
            
            // Validate input size
            if input.len() > 100_000 {
                return Err(Error::Storage("User input too large".to_string()));
            }
            
            let context = payload.get("context")
                .cloned()
                .unwrap_or(json!({}));
            
            // Validate context size
            let context_str = serde_json::to_string(&context).unwrap_or_default();
            if context_str.len() > 1_000_000 {
                return Err(Error::Storage("Context too large".to_string()));
            }

            Ok(WorldEvent::UserInput { user_id, input, context })
        }
        "system" => {
            let event_type = payload.get("event_type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Storage("Missing 'event_type' field".to_string()))?
                .to_string();
            
            // Validate event_type
            if event_type.is_empty() || event_type.len() > 256 {
                return Err(Error::Storage("Invalid event_type".to_string()));
            }
            
            let event_payload = payload.get("payload")
                .cloned()
                .unwrap_or(json!({}));
            
            // Validate payload size
            let payload_str = serde_json::to_string(&event_payload).unwrap_or_default();
            if payload_str.len() > 1_000_000 {
                return Err(Error::Storage("System event payload too large".to_string()));
            }

            Ok(WorldEvent::SystemEvent { event_type, payload: event_payload })
        }
        "command" => {
            let command = payload.get("command")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Storage("Missing 'command' field".to_string()))?
                .to_string();
            
            // Validate command
            if command.is_empty() || command.len() > 256 {
                return Err(Error::Storage("Invalid command".to_string()));
            }
            
            let args = payload.get("args")
                .cloned()
                .unwrap_or(json!({}));
            
            // Validate args size
            let args_str = serde_json::to_string(&args).unwrap_or_default();
            if args_str.len() > 1_000_000 {
                return Err(Error::Storage("Command args too large".to_string()));
            }

            Ok(WorldEvent::Command { command, args })
        }
        _ => Err(Error::Storage(format!("Unknown event type: {}", event_type))),
    }
}

