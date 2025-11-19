// gRPC streaming transport

use crate::subscriptions::Subscription;
use narayana_core::Result;
use tokio::sync::mpsc;

/// Deliver event via gRPC streaming
pub async fn deliver_grpc(
    subscription: &Subscription,
    payload: &serde_json::Value,
    sender: Option<mpsc::Sender<serde_json::Value>>,
) -> Result<()> {
    // Create gRPC message with metadata
    let grpc_message = serde_json::json!({
        "subscription_id": subscription.id.0,
        "event_name": subscription.event_name.0,
        "payload": payload,
        "timestamp": chrono::Utc::now().timestamp(),
    });
    
    // Send to gRPC stream if available
    if let Some(sender) = sender {
        sender.send(grpc_message).await
            .map_err(|e| narayana_core::Error::Storage(format!("Failed to send gRPC message: {}", e)))?;
        Ok(())
    } else {
        // If no gRPC stream, log warning but don't fail
        tracing::warn!("gRPC stream not available for subscription {}", subscription.id.0);
        Ok(())
    }
}

