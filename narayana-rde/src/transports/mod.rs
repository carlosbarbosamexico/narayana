// Transport layer for event delivery

pub mod http;
pub mod websocket;
pub mod grpc;
pub mod sse;

/// Transport type
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportType {
    /// HTTP webhook delivery
    Webhook,
    /// WebSocket real-time delivery
    WebSocket,
    /// gRPC streaming
    Grpc,
    /// Server-Sent Events
    Sse,
}

impl std::fmt::Display for TransportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportType::Webhook => write!(f, "webhook"),
            TransportType::WebSocket => write!(f, "websocket"),
            TransportType::Grpc => write!(f, "grpc"),
            TransportType::Sse => write!(f, "sse"),
        }
    }
}

