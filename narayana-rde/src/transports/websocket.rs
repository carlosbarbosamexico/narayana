// WebSocket transport

use crate::subscriptions::Subscription;
use crate::WebSocketBroadcaster;
use narayana_core::Result;
use narayana_api::websocket::WsMessage;
use std::sync::Arc;
use tracing;

/// Deliver event via WebSocket
pub async fn deliver_websocket(
    subscription: &Subscription,
    payload: &serde_json::Value,
    websocket_manager: Option<Arc<dyn WebSocketBroadcaster + Send + Sync>>,
) -> Result<()> {
    let channel = format!("rde:events:{}", subscription.event_name.0);
    
    // Create WebSocket message
    let message = serde_json::json!({
        "channel": channel.clone(),
        "event": subscription.event_name.0.clone(),
        "payload": payload,
        "timestamp": chrono::Utc::now().timestamp(),
    });

    // Broadcast via WebSocket manager if available
    if let Some(manager) = websocket_manager {
        manager.broadcast_to_channel(&channel, message);
        Ok(())
    } else {
        // If no WebSocket manager, log warning but don't fail
        tracing::warn!("WebSocket manager not available for subscription {}", subscription.id.0);
        Ok(())
    }
}

