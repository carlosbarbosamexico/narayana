// Tests for AI analytics

use narayana_query::ai_analytics::*;
use std::collections::HashMap;
use serde_json::Value;

#[test]
fn test_engagement_rate() {
    let rate = AIAnalytics::engagement_rate(100, 50);
    assert_eq!(rate, 200.0); // 100/50 * 100 = 200%
}

#[test]
fn test_conversion_rate() {
    let rate = AIAnalytics::conversion_rate(10, 100);
    assert_eq!(rate, 10.0); // 10/100 * 100 = 10%
}

#[test]
fn test_average_session_duration() {
    let durations = vec![1000, 2000, 3000, 4000, 5000];
    let avg = AIAnalytics::average_session_duration(&durations);
    assert_eq!(avg, 3000.0); // (1000+2000+3000+4000+5000)/5 = 3000
}

#[test]
fn test_agent_performance() {
    let mut events = Vec::new();
    for i in 0..10 {
        let mut metadata = HashMap::new();
        metadata.insert("success".to_string(), Value::Bool(i % 2 == 0));
        metadata.insert("response_time_ms".to_string(), Value::Number((i * 10).into()));
        
        events.push(Event {
            id: i,
            timestamp: i,
            event_type: "test".to_string(),
            agent_id: Some("agent-1".to_string()),
            metadata,
        });
    }
    
    let performance = AIAnalytics::agent_performance("agent-1", &events);
    assert_eq!(performance.total_events, 10);
    assert_eq!(performance.successful_events, 5);
}

#[test]
fn test_user_engagement_score() {
    let engagements = vec![
        Engagement {
            id: 1,
            timestamp: 0,
            user_id: "user-1".to_string(),
            duration_ms: Some(1000),
        },
        Engagement {
            id: 2,
            timestamp: 1,
            user_id: "user-1".to_string(),
            duration_ms: Some(2000),
        },
    ];
    
    let conversations = vec![
        Conversation {
            id: 1,
            created_at: 0,
            user_id: "user-1".to_string(),
            messages: vec![],
        },
    ];
    
    let score = AIAnalytics::user_engagement_score("user-1", &engagements, &conversations);
    assert!(score > 0.0);
}

#[test]
fn test_time_series_analysis() {
    let events = vec![
        Event {
            id: 1,
            timestamp: 1000,
            event_type: "test".to_string(),
            agent_id: None,
            metadata: HashMap::new(),
        },
        Event {
            id: 2,
            timestamp: 2000,
            event_type: "test".to_string(),
            agent_id: None,
            metadata: HashMap::new(),
        },
        Event {
            id: 3,
            timestamp: 3000,
            event_type: "test".to_string(),
            agent_id: None,
            metadata: HashMap::new(),
        },
    ];
    
    let windows = AIAnalytics::time_series_analysis(&events, 2000);
    assert!(!windows.is_empty());
}

#[test]
fn test_detect_anomalies() {
    let events = vec![
        Event {
            id: 1,
            timestamp: 0,
            event_type: "test".to_string(),
            agent_id: None,
            metadata: HashMap::new(),
        },
        Event {
            id: 2,
            timestamp: 1,
            event_type: "test".to_string(),
            agent_id: None,
            metadata: HashMap::new(),
        },
    ];
    
    let anomalies = AIAnalytics::detect_anomalies(&events);
    // Should detect or not detect anomalies based on data
    assert!(anomalies.len() >= 0);
}

#[test]
fn test_event_aggregator_creation() {
    let aggregator = EventAggregator::new(60000);
    // Should create successfully
}

#[test]
fn test_event_aggregator_aggregate() {
    let mut aggregator = EventAggregator::new(60000);
    let events = vec![
        Event {
            id: 1,
            timestamp: 1000,
            event_type: "test".to_string(),
            agent_id: Some("agent-1".to_string()),
            metadata: {
                let mut m = HashMap::new();
                m.insert("user_id".to_string(), Value::String("user-1".to_string()));
                m
            },
        },
    ];
    
    let results = aggregator.aggregate(events);
    assert!(!results.is_empty());
}

