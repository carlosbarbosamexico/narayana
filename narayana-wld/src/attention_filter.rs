//! Attention filter with salience computation
//! 
//! Based on cognitive science models:
//! - Itti & Koch (2001): Novelty detection
//! - Posner (1980): Urgency and temporal constraints
//! - Desimone & Duncan (1995): Relevance and attention
//! - Friston (2010): Prediction error

use crate::event_transformer::WorldEvent;
use narayana_core::Error;
use narayana_storage::cognitive::CognitiveBrain;
use serde_json::Value as JsonValue;
use std::collections::VecDeque;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::warn;

/// Attention filter with salience computation
pub struct AttentionFilter {
    brain: Arc<CognitiveBrain>,
    config: AttentionFilterConfig,
    event_history: Arc<RwLock<VecDeque<EventHistoryEntry>>>,
    predictions: Arc<RwLock<PredictionModel>>,
}

#[derive(Debug, Clone)]
struct EventHistoryEntry {
    event_type: String,
    timestamp: u64,
    salience: f64,
}

struct PredictionModel {
    event_type_counts: std::collections::HashMap<String, usize>,
    last_event_type: Option<String>,
    total_events: usize,
}

#[derive(Debug, Clone)]
pub struct AttentionFilterConfig {
    pub novelty_weight: f64,
    pub urgency_weight: f64,
    pub relevance_weight: f64,
    pub magnitude_weight: f64,
    pub prediction_error_weight: f64,
    pub salience_threshold: f64,
    pub context_window_size: usize,
}

impl Default for AttentionFilterConfig {
    fn default() -> Self {
        Self {
            novelty_weight: 0.2,
            urgency_weight: 0.2,
            relevance_weight: 0.2,
            magnitude_weight: 0.1,
            prediction_error_weight: 0.3,
            salience_threshold: 0.5,
            context_window_size: 100,
        }
    }
}

impl AttentionFilter {
    pub fn new(brain: Arc<CognitiveBrain>, config: AttentionFilterConfig) -> Self {
        Self {
            brain,
            config,
            event_history: Arc::new(RwLock::new(VecDeque::new())),
            predictions: Arc::new(RwLock::new(PredictionModel {
                event_type_counts: std::collections::HashMap::new(),
                last_event_type: None,
                total_events: 0,
            })),
        }
    }

    /// Compute salience for a world event
    pub fn compute_salience(&self, event: &WorldEvent) -> Result<f64, Error> {
        let event_type = self.get_event_type(event);
        let timestamp = self.get_timestamp(event);
        
        // Compute individual factors
        let novelty = self.compute_novelty(&event_type)?;
        let urgency = self.compute_urgency(event, timestamp)?;
        let relevance = self.compute_relevance(event)?;
        let magnitude = self.compute_magnitude(event)?;
        let prediction_error = self.compute_prediction_error(&event_type)?;

        // Validate inputs are finite numbers (not NaN or Infinity)
        let novelty = if novelty.is_finite() { novelty.clamp(0.0, 1.0) } else { 0.0 };
        let urgency = if urgency.is_finite() { urgency.clamp(0.0, 1.0) } else { 0.0 };
        let relevance = if relevance.is_finite() { relevance.clamp(0.0, 1.0) } else { 0.0 };
        let magnitude = if magnitude.is_finite() { magnitude.clamp(0.0, 1.0) } else { 0.0 };
        let prediction_error = if prediction_error.is_finite() { prediction_error.clamp(0.0, 1.0) } else { 0.0 };
        
        // Weighted combination with bounds checking
        let salience = 
            self.config.novelty_weight * novelty +
            self.config.urgency_weight * urgency +
            self.config.relevance_weight * relevance +
            self.config.magnitude_weight * magnitude +
            self.config.prediction_error_weight * prediction_error;
        
        // Clamp final salience to valid range and ensure it's finite
        let salience = if salience.is_finite() {
            salience.clamp(0.0, 1.0)
        } else {
            warn!("Computed salience is not finite, using default 0.5");
            0.5
        };

        // Update history
        {
            let mut history = self.event_history.write();
            history.push_back(EventHistoryEntry {
                event_type: event_type.clone(),
                timestamp,
                salience,
            });
            if history.len() > self.config.context_window_size {
                history.pop_front();
            }
        }

        // Update prediction model
        {
            let mut model = self.predictions.write();
            *model.event_type_counts.entry(event_type.clone()).or_insert(0) += 1;
            model.last_event_type = Some(event_type);
            
            // Prevent integer overflow
            model.total_events = model.total_events.saturating_add(1);
            
            // Prevent unbounded growth of event_type_counts
            const MAX_EVENT_TYPES: usize = 10_000;
            if model.event_type_counts.len() > MAX_EVENT_TYPES {
                // Remove least frequent event types
                let mut entries: Vec<_> = model.event_type_counts.iter().map(|(k, v)| (k.clone(), *v)).collect();
                entries.sort_by_key(|(_, count)| *count);
                let to_remove = entries.len().saturating_sub(MAX_EVENT_TYPES);
                let keys_to_remove: Vec<_> = entries.iter().take(to_remove).map(|(key, _)| key.clone()).collect();
                for key in keys_to_remove {
                    model.event_type_counts.remove(&key);
                }
            }
        }

        Ok(salience)
    }

    /// Check if event should be routed to Global Workspace
    pub fn should_route_to_workspace(&self, event: &WorldEvent) -> Result<bool, Error> {
        let salience = self.compute_salience(event)?;
        Ok(salience >= self.config.salience_threshold)
    }

    fn get_event_type(&self, event: &WorldEvent) -> String {
        match event {
            WorldEvent::SensorData { source, .. } => {
                // Sanitize source to prevent injection
                let sanitized = source.chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '.')
                    .take(128)
                    .collect::<String>();
                format!("sensor:{}", sanitized)
            }
            WorldEvent::UserInput { .. } => "user:input".to_string(),
            WorldEvent::SystemEvent { event_type, .. } => {
                // Sanitize event_type
                let sanitized = event_type.chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '.')
                    .take(128)
                    .collect::<String>();
                format!("system:{}", sanitized)
            }
            WorldEvent::Command { command, .. } => {
                // Sanitize command
                let sanitized = command.chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '.')
                    .take(128)
                    .collect::<String>();
                format!("command:{}", sanitized)
            }
        }
    }

    fn get_timestamp(&self, event: &WorldEvent) -> u64 {
        match event {
            WorldEvent::SensorData { timestamp, .. } => {
                // Validate timestamp is reasonable (not in far future, not too old)
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                
                // Allow timestamps up to 1 hour in the future (clock skew) and up to 1 year in the past
                const MAX_FUTURE_SKEW: u64 = 3600; // 1 hour
                const MAX_PAST_AGE: u64 = 31536000; // 1 year
                
                if *timestamp > now + MAX_FUTURE_SKEW {
                    warn!("Timestamp {} is too far in the future, using current time", timestamp);
                    now
                } else if now.saturating_sub(*timestamp) > MAX_PAST_AGE {
                    warn!("Timestamp {} is too old, using current time", timestamp);
                    now
                } else {
                    *timestamp
                }
            }
            WorldEvent::UserInput { .. } | WorldEvent::SystemEvent { .. } | WorldEvent::Command { .. } => {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            }
        }
    }

    /// Compute novelty: deviation from expected patterns
    fn compute_novelty(&self, event_type: &str) -> Result<f64, Error> {
        let history = self.event_history.read();
        
        // Count occurrences in recent history
        let recent_count = history.iter()
            .rev()
            .take(10)
            .filter(|e| e.event_type == event_type)
            .count();

        // Novelty is inverse of frequency
        // Prevent division issues and ensure valid range
        let denominator = 1.0 + recent_count as f64;
        if denominator == 0.0 {
            return Ok(1.0); // Should never happen, but defensive
        }
        let novelty = 1.0 / denominator;
        Ok(novelty.clamp(0.0, 1.0))
    }

    /// Compute urgency: temporal constraints
    fn compute_urgency(&self, event: &WorldEvent, timestamp: u64) -> Result<f64, Error> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Check for time-sensitive indicators in event
        let time_delta = now.saturating_sub(timestamp);
        
        // Urgency decreases with age
        let urgency = if time_delta < 1 {
            1.0
        } else if time_delta < 5 {
            0.8
        } else if time_delta < 60 {
            0.5
        } else {
            0.2
        };

        // Commands and user input are typically urgent
        let type_urgency = match event {
            WorldEvent::Command { .. } => 0.9,
            WorldEvent::UserInput { .. } => 0.8,
            WorldEvent::SystemEvent { .. } => 0.5,
            WorldEvent::SensorData { .. } => 0.3,
        };

        Ok((urgency + type_urgency) / 2.0)
    }

    /// Compute relevance: match with current cognitive state
    fn compute_relevance(&self, event: &WorldEvent) -> Result<f64, Error> {
        // For now, use heuristics based on event type
        // In a full implementation, this would query the cognitive brain
        // for current goals, working memory contents, etc.
        
        let base_relevance = match event {
            WorldEvent::UserInput { .. } => 0.9, // User input is always relevant
            WorldEvent::Command { .. } => 0.8,   // Commands are relevant
            WorldEvent::SystemEvent { .. } => 0.6,
            WorldEvent::SensorData { .. } => 0.4, // Sensor data varies
        };

        Ok(base_relevance)
    }

    /// Compute magnitude: event importance/severity
    fn compute_magnitude(&self, event: &WorldEvent) -> Result<f64, Error> {
        // Extract magnitude from event payload if available
        let magnitude = match event {
            WorldEvent::SensorData { data, .. } => {
                // Check for magnitude indicators in sensor data
                if let Some(val) = data.get("magnitude").and_then(|v| v.as_f64()) {
                    if val.is_finite() {
                        (val / 100.0).min(1.0).max(0.0) // Normalize to 0-1
                    } else {
                        warn!("Non-finite magnitude value in sensor data");
                        0.5 // Default for invalid values
                    }
                } else {
                    0.5 // Default
                }
            }
            WorldEvent::UserInput { .. } => 0.8,
            WorldEvent::Command { .. } => 0.9,
            WorldEvent::SystemEvent { .. } => 0.6,
        };

        Ok(magnitude)
    }

    /// Compute prediction error: deviation from predicted event
    fn compute_prediction_error(&self, event_type: &str) -> Result<f64, Error> {
        // Validate event_type
        if event_type.is_empty() || event_type.len() > 512 {
            return Err(Error::Storage("Invalid event_type".to_string()));
        }
        
        let model = self.predictions.read();
        
        // First event has maximum prediction error
        if model.total_events == 0 {
            return Ok(1.0);
        }

        // Predict next event based on frequency
        let predicted_type = model.last_event_type.as_ref()
            .map(|t| t.clone())
            .unwrap_or_else(|| "unknown".to_string());

        // Prediction error is high if event differs from prediction
        let error = if event_type == predicted_type {
            0.2 // Low error if prediction was correct
        } else {
            // Higher error if event type is rare
            let count = model.event_type_counts.get(event_type).copied().unwrap_or(0);
            // Safe division - we already checked total_events > 0
            let frequency = count as f64 / model.total_events as f64;
            (1.0 - frequency).clamp(0.0, 1.0) // Clamp to valid range
        };

        Ok(error)
    }
}

