// WebSocket API for real-time communication

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use narayana_core::Result;

/// WebSocket message protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    // Client -> Server messages
    #[serde(rename = "subscribe")]
    Subscribe {
        channel: String,
        filter: Option<EventFilter>,
    },
    #[serde(rename = "unsubscribe")]
    Unsubscribe {
        channel: String,
    },
    #[serde(rename = "query")]
    Query {
        query: String,
        params: Option<JsonValue>,
    },
    #[serde(rename = "ping")]
    Ping {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    
    // Server -> Client messages
    #[serde(rename = "event")]
    Event {
        channel: String,
        event: JsonValue,
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<u64>,
    },
    #[serde(rename = "query_result")]
    QueryResult {
        result: JsonValue,
        #[serde(skip_serializing_if = "Option::is_none")]
        query_id: Option<String>,
    },
    #[serde(rename = "error")]
    Error {
        code: String,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    #[serde(rename = "pong")]
    Pong {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    #[serde(rename = "subscribed")]
    Subscribed {
        channel: String,
    },
    #[serde(rename = "unsubscribed")]
    Unsubscribed {
        channel: String,
    },
}

/// Event filter for selective subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventFilter {
    /// Filter by event type (exact match)
    #[serde(rename = "event_type")]
    EventType(String),
    /// Filter by event type pattern (regex)
    #[serde(rename = "event_type_pattern")]
    EventTypePattern(String),
    /// Filter by header (key-value match)
    #[serde(rename = "header")]
    Header {
        key: String,
        value: String,
    },
    /// Filter by payload field (JSONPath)
    #[serde(rename = "payload_field")]
    PayloadField {
        path: String,
        value: JsonValue,
    },
    /// Combined filter (AND logic)
    #[serde(rename = "and")]
    And(Vec<EventFilter>),
    /// Combined filter (OR logic)
    #[serde(rename = "or")]
    Or(Vec<EventFilter>),
    /// Negated filter (NOT logic)
    #[serde(rename = "not")]
    Not(Box<EventFilter>),
}

impl WsMessage {
    /// Parse message from JSON string
    pub fn from_json(json: &str) -> std::result::Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize message to JSON string
    pub fn to_json(&self) -> std::result::Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Create error message
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        WsMessage::Error {
            code: code.into(),
            message: message.into(),
            request_id: None,
        }
    }

    /// Create error message with request ID
    pub fn error_with_id(
        code: impl Into<String>,
        message: impl Into<String>,
        request_id: impl Into<String>,
    ) -> Self {
        WsMessage::Error {
            code: code.into(),
            message: message.into(),
            request_id: Some(request_id.into()),
        }
    }

    /// Create event message
    pub fn event(channel: impl Into<String>, event: JsonValue) -> Self {
        WsMessage::Event {
            channel: channel.into(),
            event,
            timestamp: None,
        }
    }

    /// Create event message with timestamp
    pub fn event_with_timestamp(
        channel: impl Into<String>,
        event: JsonValue,
        timestamp: u64,
    ) -> Self {
        WsMessage::Event {
            channel: channel.into(),
            event,
            timestamp: Some(timestamp),
        }
    }
}

/// WebSocket connection ID
pub type ConnectionId = String;

/// WebSocket channel name
pub type Channel = String;
