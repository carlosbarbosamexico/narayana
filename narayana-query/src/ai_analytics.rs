// AI analytics for events, conversations, engagements - real-time insights

use narayana_core::column::Column;
use serde_json::Value;
use std::collections::HashMap;

/// Real-time analytics for AI agents
pub struct AIAnalytics;

impl AIAnalytics {
    /// Calculate engagement rate
    pub fn engagement_rate(total_engagements: u64, total_users: u64) -> f64 {
        if total_users == 0 {
            return 0.0;
        }
        (total_engagements as f64 / total_users as f64) * 100.0
    }

    /// Calculate conversion rate
    pub fn conversion_rate(conversions: u64, total_engagements: u64) -> f64 {
        if total_engagements == 0 {
            return 0.0;
        }
        (conversions as f64 / total_engagements as f64) * 100.0
    }

    /// Calculate average session duration
    pub fn average_session_duration(durations: &[u64]) -> f64 {
        if durations.is_empty() {
            return 0.0;
        }
        durations.iter().sum::<u64>() as f64 / durations.len() as f64
    }

    /// Calculate agent performance metrics
    pub fn agent_performance(
        agent_id: &str,
        events: &[Event],
    ) -> AgentPerformance {
        let total_events = events.len();
        let successful_events = events.iter()
            .filter(|e| e.metadata.get("success").and_then(|v| v.as_bool()).unwrap_or(false))
            .count();
        
        let average_response_time = events.iter()
            .filter_map(|e| e.metadata.get("response_time_ms").and_then(|v| v.as_u64()))
            .sum::<u64>() as f64 / total_events as f64;

        AgentPerformance {
            agent_id: agent_id.to_string(),
            total_events,
            successful_events,
            success_rate: (successful_events as f64 / total_events as f64) * 100.0,
            average_response_time_ms: average_response_time,
        }
    }

    /// Calculate user engagement score
    pub fn user_engagement_score(
        user_id: &str,
        engagements: &[Engagement],
        conversations: &[Conversation],
    ) -> f64 {
        let engagement_count = engagements.len() as f64;
        let conversation_count = conversations.len() as f64;
        let total_duration: u64 = engagements.iter()
            .filter_map(|e| e.duration_ms)
            .sum();
        
        // Weighted score
        (engagement_count * 0.4) + (conversation_count * 0.3) + (total_duration as f64 / 1000.0 * 0.3)
    }

    /// Time-series analysis for events
    pub fn time_series_analysis(events: &[Event], window_size: usize) -> Vec<TimeWindow> {
        if events.is_empty() {
            return Vec::new();
        }

        let mut sorted_events = events.to_vec();
        sorted_events.sort_by_key(|e| e.timestamp);

        let mut windows = Vec::new();
        let mut current_window_start = sorted_events[0].timestamp;
        let mut current_window_events = Vec::new();
        
        // Store last timestamp before moving sorted_events
        let last_timestamp = sorted_events.last().map(|e| e.timestamp);

        for event in sorted_events {
            if event.timestamp - current_window_start > window_size as i64 * 1000 {
                // New window
                if !current_window_events.is_empty() {
                    windows.push(TimeWindow {
                        start: current_window_start,
                        end: event.timestamp,
                        event_count: current_window_events.len(),
                        events: current_window_events.clone(),
                    });
                }
                current_window_start = event.timestamp;
                current_window_events.clear();
            }
            current_window_events.push(event);
        }

        // Add last window
        if !current_window_events.is_empty() {
            if let Some(last_ts) = last_timestamp {
                windows.push(TimeWindow {
                    start: current_window_start,
                    end: last_ts,
                    event_count: current_window_events.len(),
                    events: current_window_events,
                });
            }
        }

        windows
    }

    /// Detect anomalies in event stream
    pub fn detect_anomalies(events: &[Event]) -> Vec<Anomaly> {
        let mut anomalies = Vec::new();
        
        if events.len() < 2 {
            return anomalies;
        }

        // Calculate baseline statistics
        let event_counts: Vec<usize> = events.iter()
            .map(|_| 1)
            .collect();
        
        let mean = event_counts.iter().sum::<usize>() as f64 / event_counts.len() as f64;
        let variance: f64 = event_counts.iter()
            .map(|&x| (x as f64 - mean).powi(2))
            .sum::<f64>() / event_counts.len() as f64;
        let stddev = variance.sqrt();

        // Detect outliers (3 sigma rule)
        for (i, event) in events.iter().enumerate() {
            let deviation = (1.0 - mean).abs() / stddev;
            if deviation > 3.0 {
                anomalies.push(Anomaly {
                    event_id: event.id,
                    timestamp: event.timestamp,
                    deviation,
                    description: format!("Event count deviation: {:.2} sigma", deviation),
                });
            }
        }

        anomalies
    }
}

/// Event structure (from storage)
#[derive(Debug, Clone)]
pub struct Event {
    pub id: u64,
    pub timestamp: i64,
    pub event_type: String,
    pub agent_id: Option<String>,
    pub metadata: HashMap<String, Value>,
}

/// Engagement structure (from storage)
#[derive(Debug, Clone)]
pub struct Engagement {
    pub id: u64,
    pub timestamp: i64,
    pub user_id: String,
    pub duration_ms: Option<u64>,
}

/// Conversation structure (from storage)
#[derive(Debug, Clone)]
pub struct Conversation {
    pub id: u64,
    pub created_at: i64,
    pub user_id: String,
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id: u64,
    pub timestamp: i64,
    pub content: String,
}

/// Agent performance metrics
#[derive(Debug, Clone)]
pub struct AgentPerformance {
    pub agent_id: String,
    pub total_events: usize,
    pub successful_events: usize,
    pub success_rate: f64,
    pub average_response_time_ms: f64,
}

/// Time window for analysis
#[derive(Debug, Clone)]
pub struct TimeWindow {
    pub start: i64,
    pub end: i64,
    pub event_count: usize,
    pub events: Vec<Event>,
}

/// Anomaly detection result
#[derive(Debug, Clone)]
pub struct Anomaly {
    pub event_id: u64,
    pub timestamp: i64,
    pub deviation: f64,
    pub description: String,
}

/// Real-time event aggregator
pub struct EventAggregator {
    windows: Vec<TimeWindow>,
    window_size_ms: u64,
}

impl EventAggregator {
    pub fn new(window_size_ms: u64) -> Self {
        Self {
            windows: Vec::new(),
            window_size_ms,
        }
    }

    /// Aggregate events in real-time
    pub fn aggregate(&mut self, events: Vec<Event>) -> Vec<AggregateResult> {
        let windows = AIAnalytics::time_series_analysis(&events, self.window_size_ms as usize);
        
        windows.iter().map(|window| {
            AggregateResult {
                window_start: window.start,
                window_end: window.end,
                event_count: window.event_count,
                unique_users: self.count_unique_users(&window.events),
                unique_agents: self.count_unique_agents(&window.events),
            }
        }).collect()
    }

    fn count_unique_users(&self, events: &[Event]) -> usize {
        use std::collections::HashSet;
        events.iter()
            .filter_map(|e| e.metadata.get("user_id").and_then(|v| v.as_str()))
            .collect::<HashSet<_>>()
            .len()
    }

    fn count_unique_agents(&self, events: &[Event]) -> usize {
        use std::collections::HashSet;
        events.iter()
            .filter_map(|e| e.agent_id.as_ref())
            .collect::<HashSet<_>>()
            .len()
    }
}

#[derive(Debug, Clone)]
pub struct AggregateResult {
    pub window_start: i64,
    pub window_end: i64,
    pub event_count: usize,
    pub unique_users: usize,
    pub unique_agents: usize,
}

