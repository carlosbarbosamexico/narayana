//! Event transformation between world events and cognitive events

use narayana_core::Error;
use narayana_storage::cognitive::CognitiveEvent;
use narayana_storage::conscience_persistent_loop::CPLEvent;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::warn;

/// External world events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorldEvent {
    /// Sensor data from external systems
    SensorData {
        source: String,
        data: JsonValue,
        timestamp: u64,
    },
    /// User input/interaction
    UserInput {
        user_id: String,
        input: String,
        context: JsonValue,
    },
    /// System-level events
    SystemEvent {
        event_type: String,
        payload: JsonValue,
    },
    /// Commands from external systems
    Command {
        command: String,
        args: JsonValue,
    },
}

/// External world actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorldAction {
    /// Actuator commands
    ActuatorCommand {
        target: String,
        command: JsonValue,
    },
    /// User responses
    UserResponse {
        user_id: String,
        message: String,
    },
    /// System notifications
    SystemNotification {
        channel: String,
        content: JsonValue,
    },
    /// Data transmission
    DataTransmission {
        destination: String,
        data: JsonValue,
    },
}

/// Event transformer for bidirectional conversion
pub struct EventTransformer {
    context: JsonValue,
}

impl EventTransformer {
    pub fn new() -> Self {
        Self {
            context: JsonValue::Object(serde_json::Map::new()),
        }
    }

    /// Transform world event to cognitive event
    pub fn world_to_cognitive(&self, event: &WorldEvent) -> Result<CognitiveEvent, Error> {
        // Validate and sanitize inputs
        match event {
            WorldEvent::SensorData { source, data, .. } => {
                // Sanitize source to prevent format string attacks
                let sanitized_source = sanitize_identifier(source);
                if sanitized_source.is_empty() {
                    return Err(Error::Storage("Invalid sensor source".to_string()));
                }
                
                // Store as experience
                Ok(CognitiveEvent::ExperienceStored {
                    experience_id: format!("sensor_{}_{}", sanitized_source, 
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs()),
                })
            }
            WorldEvent::UserInput { user_id, input, .. } => {
                // Validate input length to prevent DoS
                if input.len() > 100_000 {
                    return Err(Error::Storage("User input too large".to_string()));
                }
                
                let sanitized_user_id = sanitize_identifier(user_id);
                if sanitized_user_id.is_empty() {
                    return Err(Error::Storage("Invalid user_id".to_string()));
                }
                
                // Create thought from user input
                Ok(CognitiveEvent::ThoughtCreated {
                    thought_id: format!("user_input_{}_{}", sanitized_user_id,
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs()),
                })
            }
            WorldEvent::SystemEvent { event_type, .. } => {
                let sanitized_type = sanitize_identifier(event_type);
                if sanitized_type.is_empty() {
                    return Err(Error::Storage("Invalid event_type".to_string()));
                }
                
                // System events become thoughts
                Ok(CognitiveEvent::ThoughtCreated {
                    thought_id: format!("system_{}_{}", sanitized_type,
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs()),
                })
            }
            WorldEvent::Command { command, .. } => {
                let sanitized_command = sanitize_identifier(command);
                if sanitized_command.is_empty() {
                    return Err(Error::Storage("Invalid command".to_string()));
                }
                
                // Commands become thoughts
                Ok(CognitiveEvent::ThoughtCreated {
                    thought_id: format!("cmd_{}_{}", sanitized_command,
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs()),
                })
            }
        }
    }

    /// Transform world event to CPL event
    pub fn world_to_cpl(&self, event: &WorldEvent) -> Result<CPLEvent, Error> {
        match event {
            WorldEvent::SensorData { .. } => {
                Ok(CPLEvent::BackgroundProcessCompleted {
                    process_type: "sensor_processing".to_string(),
                })
            }
            WorldEvent::UserInput { .. } | WorldEvent::SystemEvent { .. } | WorldEvent::Command { .. } => {
                Ok(CPLEvent::GlobalWorkspaceBroadcast {
                    content_id: format!("world_event_{}", 
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs()),
                    priority: 0.7, // Default priority for world events
                })
            }
        }
    }

    /// Transform cognitive event to world action
    pub fn cognitive_to_world(&self, event: &CognitiveEvent) -> Result<Option<WorldAction>, Error> {
        match event {
            CognitiveEvent::ThoughtCompleted { thought_id } => {
                // Completed thoughts might trigger responses
                Ok(Some(WorldAction::SystemNotification {
                    channel: "thoughts".to_string(),
                    content: serde_json::json!({
                        "thought_id": thought_id,
                        "status": "completed"
                    }),
                }))
            }
            _ => Ok(None), // Most cognitive events don't directly map to world actions
        }
    }

    /// Transform CPL event to world action
    pub fn cpl_to_world(&self, event: &CPLEvent) -> Result<Option<WorldAction>, Error> {
        match event {
            CPLEvent::GlobalWorkspaceBroadcast { content_id, priority } => {
                // Validate priority is in valid range and finite
                let priority = if priority.is_finite() {
                    priority.clamp(0.0, 1.0)
                } else {
                    warn!("Invalid priority value (non-finite), using 0.5");
                    0.5
                };
                
                // Sanitize content_id
                let sanitized_id = sanitize_identifier(content_id);
                if sanitized_id.is_empty() {
                    return Err(Error::Storage("Invalid content_id".to_string()));
                }
                
                if priority > 0.8 {
                    // High-priority broadcasts might trigger notifications
                    Ok(Some(WorldAction::SystemNotification {
                        channel: "global_workspace".to_string(),
                        content: serde_json::json!({
                            "content_id": sanitized_id,
                            "priority": priority
                        }),
                    }))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    /// Update context
    pub fn update_context(&mut self, context: JsonValue) {
        self.context = context;
    }
}

impl Default for EventTransformer {
    fn default() -> Self {
        Self::new()
    }
}

/// Sanitize identifier to prevent format string attacks and injection
fn sanitize_identifier(s: &str) -> String {
    // Remove any characters that could be used in format strings or injection
    s.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '.')
        .take(256) // Limit length
        .collect()
}

