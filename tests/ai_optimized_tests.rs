// Tests for AI-optimized features

use narayana_storage::ai_optimized::*;
use narayana_core::types::TableId;

#[tokio::test]
async fn test_event_streamer_creation() {
    let streamer = EventStreamer::new(1000, 100);
    // Should create successfully
}

#[tokio::test]
async fn test_event_streamer_stream() {
    let streamer = EventStreamer::new(1000, 100);
    let event = Event {
        id: 1,
        timestamp: 0,
        event_type: "test".to_string(),
        agent_id: Some("agent-1".to_string()),
        session_id: Some("session-1".to_string()),
        user_id: Some("user-1".to_string()),
        metadata: std::collections::HashMap::new(),
        payload: b"data".to_vec(),
    };
    
    streamer.stream(event).await.unwrap();
}

#[tokio::test]
async fn test_event_streamer_stream_batch() {
    let streamer = EventStreamer::new(1000, 100);
    let events = vec![
        Event {
            id: 1,
            timestamp: 0,
            event_type: "test".to_string(),
            agent_id: None,
            session_id: None,
            user_id: None,
            metadata: std::collections::HashMap::new(),
            payload: b"data1".to_vec(),
        },
        Event {
            id: 2,
            timestamp: 1,
            event_type: "test".to_string(),
            agent_id: None,
            session_id: None,
            user_id: None,
            metadata: std::collections::HashMap::new(),
            payload: b"data2".to_vec(),
        },
    ];
    
    streamer.stream_batch(events).await.unwrap();
}

#[tokio::test]
async fn test_event_streamer_flush() {
    let streamer = EventStreamer::new(1000, 100);
    let count = streamer.flush().await.unwrap();
    assert_eq!(count, 0); // Empty buffer
}

#[test]
fn test_conversation_manager_creation() {
    let manager = ConversationManager::new();
    // Should create successfully
}

#[test]
fn test_conversation_manager_create_conversation() {
    let manager = ConversationManager::new();
    let conversation = manager.create_conversation(
        "agent-1".to_string(),
        "user-1".to_string(),
        "session-1".to_string(),
    );
    
    assert_eq!(conversation.agent_id, "agent-1");
    assert_eq!(conversation.user_id, "user-1");
    assert_eq!(conversation.session_id, "session-1");
}

#[test]
fn test_conversation_manager_add_message() {
    let manager = ConversationManager::new();
    let conversation = manager.create_conversation(
        "agent-1".to_string(),
        "user-1".to_string(),
        "session-1".to_string(),
    );
    
    let message = Message {
        id: 1,
        timestamp: 0,
        role: MessageRole::User,
        content: "Hello".to_string(),
        embeddings: None,
        metadata: std::collections::HashMap::new(),
    };
    
    manager.add_message(conversation.id, message).unwrap();
    let retrieved = manager.get_conversation(conversation.id).unwrap();
    assert_eq!(retrieved.messages.len(), 1);
}

#[test]
fn test_engagement_tracker_creation() {
    let tracker = EngagementTracker::new();
    // Should create successfully
}

#[test]
fn test_engagement_tracker_track() {
    let tracker = EngagementTracker::new();
    let engagement = Engagement {
        id: 1,
        timestamp: 0,
        agent_id: "agent-1".to_string(),
        user_id: "user-1".to_string(),
        engagement_type: EngagementType::Click,
        duration_ms: Some(1000),
        metadata: std::collections::HashMap::new(),
    };
    
    tracker.track(engagement).unwrap();
    let metrics = tracker.get_metrics();
    assert_eq!(metrics.total_engagements, 1);
}

#[test]
fn test_engagement_tracker_conversion() {
    let tracker = EngagementTracker::new();
    let engagement = Engagement {
        id: 1,
        timestamp: 0,
        agent_id: "agent-1".to_string(),
        user_id: "user-1".to_string(),
        engagement_type: EngagementType::Conversion,
        duration_ms: Some(5000),
        metadata: std::collections::HashMap::new(),
    };
    
    tracker.track(engagement).unwrap();
    let metrics = tracker.get_metrics();
    assert_eq!(metrics.conversions, 1);
}

#[test]
fn test_transaction_processor_creation() {
    let processor = TransactionProcessor::new(10);
    // Should create successfully
}

#[test]
fn test_transaction_processor_process() {
    let processor = TransactionProcessor::new(10);
    let transaction = Transaction {
        id: 1,
        timestamp: 0,
        agent_id: "agent-1".to_string(),
        transaction_type: "test".to_string(),
        data: b"data".to_vec(),
    };
    
    processor.process(transaction);
    // Should process without blocking
}

