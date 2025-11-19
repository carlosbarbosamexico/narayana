// Event publishing and schema management

use serde::{Deserialize, Serialize};
use std::fmt;
use narayana_core::Result;

/// Event name (full namespaced: actor_id:event_name)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventName(pub String);

impl From<String> for EventName {
    fn from(s: String) -> Self {
        EventName(s)
    }
}

impl From<&str> for EventName {
    fn from(s: &str) -> Self {
        EventName(s.to_string())
    }
}

impl fmt::Display for EventName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Event schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSchema {
    pub fields: Vec<SchemaField>,
    pub extracted_at: u64,
}

/// Schema field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaField {
    pub name: String,
    pub field_type: String, // "string", "number", "boolean", "object", "array"
    pub required: bool,
}

/// RDE Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdeEvent {
    pub actor_id: String,
    pub event_name: String,
    pub full_event_name: EventName,
    pub payload: serde_json::Value,
    pub schema: Option<EventSchema>,
    pub timestamp: u64,
}

/// Extract schema from JSON payload
pub fn extract_schema(payload: &serde_json::Value) -> Result<EventSchema> {
    let mut fields = Vec::new();
    
    // Limit total number of fields to prevent DoS
    const MAX_FIELDS: usize = 10_000;

    match payload {
        serde_json::Value::Object(obj) => {
            // Limit object size
            if obj.len() > MAX_FIELDS {
                return Err(narayana_core::Error::Storage(format!(
                    "Payload has too many fields: {} (max: {})",
                    obj.len(), MAX_FIELDS
                )));
            }
            
            for (key, value) in obj {
                // Limit field name length to prevent DoS
                if key.len() > 1024 {
                    continue; // Skip extremely long field names
                }
                
                // Prevent control characters in field names
                if key.chars().any(|c| c.is_control() || c == '\0') {
                    continue; // Skip invalid field names
                }
                
                let field_type = match value {
                    serde_json::Value::String(_) => "string",
                    serde_json::Value::Number(_) => "number",
                    serde_json::Value::Bool(_) => "boolean",
                    serde_json::Value::Object(_) => "object",
                    serde_json::Value::Array(_) => "array",
                    serde_json::Value::Null => "null",
                };

                fields.push(SchemaField {
                    name: key.clone(),
                    field_type: field_type.to_string(),
                    required: !value.is_null(),
                });
            }
        }
        serde_json::Value::Array(arr) => {
            // Limit array size for schema extraction
            if arr.len() > 1000 {
                // For very large arrays, just indicate it's an array
                fields.push(SchemaField {
                    name: "[array]".to_string(),
                    field_type: "array".to_string(),
                    required: true,
                });
            } else if let Some(first) = arr.first() {
                // Try to infer schema from first element
                if let serde_json::Value::Object(obj) = first {
                    for (key, value) in obj {
                        if key.len() > 1024 || fields.len() >= MAX_FIELDS {
                            break;
                        }
                        let field_type = match value {
                            serde_json::Value::String(_) => "string",
                            serde_json::Value::Number(_) => "number",
                            serde_json::Value::Bool(_) => "boolean",
                            serde_json::Value::Object(_) => "object",
                            serde_json::Value::Array(_) => "array",
                            serde_json::Value::Null => "null",
                        };
                        fields.push(SchemaField {
                            name: key.clone(),
                            field_type: field_type.to_string(),
                            required: !value.is_null(),
                        });
                    }
                } else {
                    // Primitive array
                    let field_type = match first {
                        serde_json::Value::String(_) => "string",
                        serde_json::Value::Number(_) => "number",
                        serde_json::Value::Bool(_) => "boolean",
                        serde_json::Value::Null => "null",
                        _ => "unknown",
                    };
                    fields.push(SchemaField {
                        name: "[array]".to_string(),
                        field_type: format!("array<{}>", field_type),
                        required: true,
                    });
                }
            } else {
                // Empty array
                fields.push(SchemaField {
                    name: "[array]".to_string(),
                    field_type: "array".to_string(),
                    required: true,
                });
            }
        }
        _ => {
            // For primitives, create a single field
            let field_type = match payload {
                serde_json::Value::String(_) => "string",
                serde_json::Value::Number(_) => "number",
                serde_json::Value::Bool(_) => "boolean",
                serde_json::Value::Null => "null",
                _ => "unknown",
            };
            fields.push(SchemaField {
                name: "[value]".to_string(),
                field_type: field_type.to_string(),
                required: !payload.is_null(),
            });
        }
    }

    Ok(EventSchema {
        fields,
        extracted_at: chrono::Utc::now().timestamp() as u64,
    })
}

/// Event type alias for compatibility
pub type Event = RdeEvent;

