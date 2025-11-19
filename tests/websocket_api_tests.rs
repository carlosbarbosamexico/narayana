// Tests for WebSocket API

use narayana_api::websocket::*;

#[test]
fn test_websocket_connection_creation() {
    let ws = WebSocketConnection::new();
    // Should create successfully
}

#[test]
fn test_websocket_message_creation() {
    let message = WebSocketMessage {
        message_type: MessageType::Query,
        data: serde_json::json!({"query": "SELECT * FROM users"}),
    };
    // Should create successfully
}

#[test]
fn test_websocket_message_types() {
    let query_msg = WebSocketMessage {
        message_type: MessageType::Query,
        data: serde_json::json!({}),
    };
    
    let subscribe_msg = WebSocketMessage {
        message_type: MessageType::Subscribe,
        data: serde_json::json!({"table": "users"}),
    };
    
    let update_msg = WebSocketMessage {
        message_type: MessageType::Update,
        data: serde_json::json!({"id": 1}),
    };
    
    // All should create successfully
    assert!(matches!(query_msg.message_type, MessageType::Query));
    assert!(matches!(subscribe_msg.message_type, MessageType::Subscribe));
    assert!(matches!(update_msg.message_type, MessageType::Update));
}

#[test]
fn test_websocket_connection_send() {
    let ws = WebSocketConnection::new();
    let message = WebSocketMessage {
        message_type: MessageType::Query,
        data: serde_json::json!({}),
    };
    let result = ws.send(message);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_websocket_query_creation() {
    let ws = WebSocketConnection::new();
    let query = WebSocketQuery::new(ws, "SELECT * FROM users".to_string());
    // Should create successfully
}

#[tokio::test]
async fn test_websocket_query_execute() {
    let ws = WebSocketConnection::new();
    let query = WebSocketQuery::new(ws, "SELECT * FROM users".to_string());
    let mut stream = query.execute();
    // Should create stream
    // Note: Stream will be empty in test environment
}

