// Tests for network sync

use narayana_storage::network_sync::*;
use narayana_storage::quantum_sync::{VectorClock, EntangledState};
use narayana_core::types::TableId;

#[test]
fn test_sync_message_state_vector() {
    let states = vec![EntangledState::new("node-1".to_string())];
    let message = SyncMessage::StateVector { states };
    // Should create successfully
}

#[test]
fn test_sync_message_delta() {
    let message = SyncMessage::Delta {
        table_id: TableId(1),
        delta: b"delta data".to_vec(),
    };
    // Should create successfully
}

#[test]
fn test_sync_message_merge() {
    let state = EntangledState::new("node-1".to_string());
    let message = SyncMessage::Merge {
        table_id: TableId(1),
        state,
    };
    // Should create successfully
}

#[test]
fn test_sync_message_heartbeat() {
    let message = SyncMessage::Heartbeat {
        node_id: "node-1".to_string(),
        timestamp: 1234567890,
    };
    // Should create successfully
}

#[test]
fn test_sync_transport_creation() {
    use std::sync::Arc;
    let sync_manager = Arc::new(narayana_storage::quantum_sync::QuantumSyncManager::new("node-1".to_string()));
    let transport = SyncTransport::new(sync_manager);
    // Should create successfully
}

#[tokio::test]
async fn test_sync_transport_send_state_vector() {
    use std::sync::Arc;
    let sync_manager = Arc::new(narayana_storage::quantum_sync::QuantumSyncManager::new("node-1".to_string()));
    let transport = SyncTransport::new(sync_manager);
    let result = transport.send_state_vector("node-2").await;
    // Should send successfully
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_sync_transport_receive_message() {
    use std::sync::Arc;
    let sync_manager = Arc::new(narayana_storage::quantum_sync::QuantumSyncManager::new("node-1".to_string()));
    let transport = SyncTransport::new(sync_manager);
    let message = SyncMessage::Heartbeat {
        node_id: "node-2".to_string(),
        timestamp: 1234567890,
    };
    let result = transport.receive_message(message).await;
    assert!(result.is_ok());
}

#[test]
fn test_efficient_broadcast_creation() {
    use std::sync::Arc;
    let sync_manager = Arc::new(narayana_storage::quantum_sync::QuantumSyncManager::new("node-1".to_string()));
    let broadcast = EfficientBroadcast::new(sync_manager);
    // Should create successfully
}

#[tokio::test]
async fn test_efficient_broadcast_broadcast() {
    use std::sync::Arc;
    let sync_manager = Arc::new(narayana_storage::quantum_sync::QuantumSyncManager::new("node-1".to_string()));
    let broadcast = EfficientBroadcast::new(sync_manager);
    let event = SyncEvent {
        table_id: TableId(1),
        operation: SyncOperation::Update,
        vector_clock: VectorClock::new("node-1".to_string()),
        data: b"data".to_vec(),
    };
    let result = broadcast.broadcast(event).await;
    assert!(result.is_ok());
}

