// Server-Sent Events transport

use crate::subscriptions::Subscription;
use narayana_core::Result;
use tokio::sync::mpsc;

/// Deliver event via SSE
pub async fn deliver_sse(
    subscription: &Subscription,
    payload: &serde_json::Value,
    sender: Option<mpsc::Sender<String>>,
) -> Result<()> {
    // Format SSE message according to SSE spec
    // Format: "data: <json>\n\n"
    let sse_message = format!(
        "event: {}\ndata: {}\n\n",
        subscription.event_name.0,
        serde_json::to_string(payload)
            .map_err(|e| narayana_core::Error::Storage(format!("Failed to serialize SSE payload: {}", e)))?
    );
    
    // Send to SSE connection if available
    if let Some(sender) = sender {
        sender.send(sse_message).await
            .map_err(|e| narayana_core::Error::Storage(format!("Failed to send SSE message: {}", e)))?;
        Ok(())
    } else {
        // If no SSE connection, log warning but don't fail
        tracing::warn!("SSE connection not available for subscription {}", subscription.id.0);
        Ok(())
    }
}

