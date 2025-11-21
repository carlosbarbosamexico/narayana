// WebSocket handler for real-time communication

use narayana_api::websocket::{ConnectionId, WsMessage};
use crate::websocket_manager::WebSocketManager;
use crate::websocket_bridge::WebSocketBridge;
use crate::security::TokenManager;
use narayana_storage::{ColumnStore, database_manager::DatabaseManager};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::Response,
};
use std::sync::Arc;
use tokio::sync::mpsc;
use futures_util::{SinkExt, StreamExt};
use tracing::{info, warn, error, debug};
use uuid::Uuid;
use serde::Deserialize;

/// WebSocket state
#[derive(Clone)]
pub struct WebSocketState {
    pub manager: Arc<WebSocketManager>,
    pub bridge: Arc<WebSocketBridge>,
    pub token_manager: Arc<TokenManager>,
    pub storage: Arc<dyn ColumnStore>,
    pub db_manager: Arc<DatabaseManager>,
}

/// Query parameters for WebSocket connection
#[derive(Deserialize)]
pub struct WsQueryParams {
    pub token: Option<String>,
}

/// WebSocket upgrade handler
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsQueryParams>,
    State(state): State<Arc<WebSocketState>>,
) -> Response {
    // Validate token if provided
    let user_id = if let Some(token) = &params.token {
        // Verify token and extract user_id
        match state.token_manager.verify_token(token) {
            Ok(claims) => Some(claims.sub),
            Err(e) => {
                warn!("Invalid WebSocket token: {}", e);
                None
            }
        }
    } else {
        None
    };

    ws.on_upgrade(move |socket| handle_socket(socket, state, user_id))
}

/// Handle WebSocket connection
pub(crate) async fn handle_socket(
    socket: WebSocket,
    state: Arc<WebSocketState>,
    user_id: Option<String>,
) {
    let connection_id = Uuid::new_v4().to_string();
    info!("WebSocket connection established: {} (user: {:?})", connection_id, user_id);

    // Create channel for sending messages to this connection
    let (tx, mut rx) = mpsc::unbounded_channel::<WsMessage>();

    // Register connection with manager
    if let Err(e) = state.manager.register_connection(
        connection_id.clone(),
        user_id.clone(),
        tx.clone(),
    ) {
        error!("Failed to register WebSocket connection {}: {}", connection_id, e);
        return;
    }

    // Split socket into sender and receiver using futures_util
    let (mut sender, mut receiver) = socket.split();

    // Spawn task to send messages from manager to client
    let connection_id_clone = connection_id.clone();
    let manager_clone = state.manager.clone();
    let send_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            let json = match message.to_json() {
                Ok(json) => json,
                Err(e) => {
                    error!("Failed to serialize WebSocket message: {}", e);
                    continue;
                }
            };

            if let Err(e) = sender.send(Message::Text(json)).await {
                warn!("Failed to send WebSocket message to {}: {}", connection_id_clone, e);
                break;
            }
        }
    });

    // Handle incoming messages from client
    let connection_id_clone2 = connection_id.clone();
    let manager_clone2 = state.manager.clone();
    let storage_clone = state.storage.clone();
    let db_manager_clone = state.db_manager.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Err(e) = handle_message(&text, &connection_id_clone2, &manager_clone2, storage_clone.clone(), db_manager_clone.clone()).await {
                        error!("Error handling message from {}: {}", connection_id_clone2, e);
                    }
                }
                Ok(Message::Binary(_)) => {
                    warn!("Received binary message from {}, ignoring", connection_id_clone2);
                }
                Ok(Message::Close(_)) => {
                    debug!("WebSocket connection {} closed by client", connection_id_clone2);
                    break;
                }
                Ok(Message::Ping(_data)) => {
                    // Respond to ping with pong
                    // Note: axum handles this automatically, but we can also send a pong message
                    debug!("Received ping from {}", connection_id_clone2);
                }
                Ok(Message::Pong(_)) => {
                    debug!("Received pong from {}", connection_id_clone2);
                }
                Err(e) => {
                    error!("WebSocket error from {}: {}", connection_id_clone2, e);
                    break;
                }
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {
            debug!("Send task completed for connection {}", connection_id);
        }
        _ = recv_task => {
            debug!("Receive task completed for connection {}", connection_id);
        }
    }

    // Clean up connection
    state.manager.unregister_connection(&connection_id);
    info!("WebSocket connection closed: {}", connection_id);
}

/// Handle incoming WebSocket message
async fn handle_message(
    text: &str,
    connection_id: &ConnectionId,
    manager: &Arc<WebSocketManager>,
    storage: Arc<dyn ColumnStore>,
    db_manager: Arc<DatabaseManager>,
) -> Result<(), String> {
    // Update activity timestamp
    manager.update_activity(connection_id);

    // Parse message
    let message = match WsMessage::from_json(text) {
        Ok(msg) => msg,
        Err(e) => {
            error!("Failed to parse WebSocket message from {}: {}", connection_id, e);
            let error_msg = WsMessage::error("parse_error", &format!("Invalid message format: {}", e));
            if let Ok(json) = error_msg.to_json() {
                // We can't send directly here, but the error is logged
                // The connection will be cleaned up if it's broken
            }
            return Err(format!("Parse error: {}", e));
        }
    };

    match message {
        WsMessage::Subscribe { channel, filter } => {
            debug!("Connection {} subscribing to channel: {}", connection_id, channel);
            match manager.subscribe(connection_id, channel.clone(), filter) {
                Ok(_) => {
                    let subscribed_msg = WsMessage::Subscribed { channel: channel.clone() };
                    if !manager.send_to_connection(connection_id, subscribed_msg) {
                        warn!("Failed to send subscribed confirmation to {}", connection_id);
                    }
                }
                Err(e) => {
                    error!("Failed to subscribe {} to channel {}: {}", connection_id, channel, e);
                    let error_msg = WsMessage::error("subscribe_error", &e);
                    manager.send_to_connection(connection_id, error_msg);
                }
            }
        }
        WsMessage::Unsubscribe { channel } => {
            debug!("Connection {} unsubscribing from channel: {}", connection_id, channel);
            match manager.unsubscribe(connection_id, &channel) {
                Ok(_) => {
                    let unsubscribed_msg = WsMessage::Unsubscribed { channel: channel.clone() };
                    if !manager.send_to_connection(connection_id, unsubscribed_msg) {
                        warn!("Failed to send unsubscribed confirmation to {}", connection_id);
                    }
                }
                Err(e) => {
                    error!("Failed to unsubscribe {} from channel {}: {}", connection_id, channel, e);
                    let error_msg = WsMessage::error("unsubscribe_error", &e);
                    manager.send_to_connection(connection_id, error_msg);
                }
            }
        }
        WsMessage::Ping { id } => {
            debug!("Received ping from {} (id: {:?})", connection_id, id);
            let pong_msg = WsMessage::Pong { id };
            if !manager.send_to_connection(connection_id, pong_msg) {
                warn!("Failed to send pong to {}", connection_id);
            }
        }
        WsMessage::Query { query, params } => {
            debug!("Executing query from connection {}: {}", connection_id, query);
            
            // Parse query - for now, support simple table queries
            // Format: "table:<table_id>" or "table_name:<name>" or SQL-like queries
            let query_result = if query.starts_with("table:") {
                // Simple table query: table:<table_id>
                let table_id_str = query.trim_start_matches("table:");
                if let Ok(table_id) = table_id_str.parse::<u64>() {
                    execute_table_query(table_id, params, storage, db_manager).await
                } else {
                    Err(format!("Invalid table ID: {}", table_id_str))
                }
            } else if query.starts_with("table_name:") {
                // Query by table name
                let table_name = query.trim_start_matches("table_name:");
                execute_table_query_by_name(table_name, params, storage, db_manager).await
            } else {
                // For now, treat as table name
                execute_table_query_by_name(&query, params, storage, db_manager).await
            };

            match query_result {
                Ok(result) => {
                    let result_msg = WsMessage::QueryResult {
                        result: serde_json::json!(result),
                        query_id: None,
                    };
                    if !manager.send_to_connection(connection_id, result_msg) {
                        warn!("Failed to send query result to {}", connection_id);
                    }
                }
                Err(e) => {
                    error!("Query execution failed for connection {}: {}", connection_id, e);
                    let error_msg = WsMessage::error("query_error", &format!("Query execution failed: {}", e));
                    manager.send_to_connection(connection_id, error_msg);
                }
            }
        }
        _ => {
            warn!("Unexpected message type from connection {}: {:?}", connection_id, message);
            let error_msg = WsMessage::error("invalid_message", "Unexpected message type");
            manager.send_to_connection(connection_id, error_msg);
        }
    }

    Ok(())
}

/// Execute a table query by table ID
async fn execute_table_query(
    table_id: u64,
    params: Option<serde_json::Value>,
    storage: Arc<dyn ColumnStore>,
    db_manager: Arc<DatabaseManager>,
) -> Result<serde_json::Value, String> {
    use narayana_core::types::TableId;
    
    let table_id = TableId(table_id);
    
    // Parse params for limit, offset, columns
    let limit = params
        .as_ref()
        .and_then(|p| p.get("limit"))
        .and_then(|l| l.as_u64())
        .map(|l| l as usize)
        .unwrap_or(1000)
        .min(10000); // Max 10k rows
    
    let offset = params
        .as_ref()
        .and_then(|p| p.get("offset"))
        .and_then(|o| o.as_u64())
        .map(|o| o as usize)
        .unwrap_or(0);
    
    // Get column indices from params or default to all
    let column_indices: Vec<u32> = if let Some(cols_param) = params.as_ref().and_then(|p| p.get("columns")).and_then(|c| c.as_array()) {
        // Use provided column indices
        cols_param
            .iter()
            .filter_map(|v| v.as_u64().map(|i| i as u32))
            .collect()
    } else {
        // Default to all columns - get schema first
        match storage.get_schema(table_id).await {
            Ok(schema) => (0..schema.fields.len() as u32).collect(),
            Err(_) => vec![0, 1], // Fallback to first two columns
        }
    };
    
    // Read columns from storage
    match storage.read_columns(table_id, column_indices.clone(), offset, limit).await {
        Ok(columns) => {
            let row_count = columns.first().map(|c| c.len()).unwrap_or(0);
            
            // Convert columns to JSON
            let json_columns: Vec<serde_json::Value> = columns
                .iter()
                .filter_map(|col| serde_json::to_value(col).ok())
                .collect();
            
            Ok(serde_json::json!({
                "columns": json_columns,
                "row_count": row_count,
                "table_id": table_id.0,
            }))
        }
        Err(e) => Err(format!("Failed to query table: {}", e))
    }
}

/// Execute a table query by table name
async fn execute_table_query_by_name(
    table_name: &str,
    params: Option<serde_json::Value>,
    storage: Arc<dyn ColumnStore>,
    db_manager: Arc<DatabaseManager>,
) -> Result<serde_json::Value, String> {
    use narayana_core::types::TableId;
    
    // Find table ID by name - iterate through all databases and their tables
    let databases = db_manager.list_databases();
    let mut table_id = None;
    for db in databases {
        if let Ok(tables) = db_manager.list_tables(db.id) {
            if let Some(found) = tables.iter().find(|t| t.name == table_name) {
                table_id = Some(found.table_id);
                break;
            }
        }
    }
    let table_id = table_id.ok_or_else(|| format!("Table '{}' not found", table_name))?;
    
    execute_table_query(table_id.0, params, storage, db_manager).await
}
