//! Configuration for the World Broker

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for the World Broker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldBrokerConfig {
    /// Enabled protocol adapters
    pub enabled_adapters: Vec<String>,
    
    /// Salience threshold for routing to Global Workspace (0.0-1.0)
    pub salience_threshold: f64,
    
    /// Event buffer size for incoming events
    pub event_buffer_size: usize,
    
    /// Enable attention filter
    pub enable_attention_filter: bool,
    
    /// Adapter-specific configurations
    pub adapter_configs: HashMap<String, serde_json::Value>,
    
    /// Prediction error weight in salience computation
    pub prediction_error_weight: f64,
    
    /// Novelty weight in salience computation
    pub novelty_weight: f64,
    
    /// Urgency weight in salience computation
    pub urgency_weight: f64,
    
    /// Relevance weight in salience computation
    pub relevance_weight: f64,
    
    /// Magnitude weight in salience computation
    pub magnitude_weight: f64,
    
    /// Enable predictive processing
    pub enable_predictive_processing: bool,
    
    /// Context window size for event history
    pub context_window_size: usize,
}

impl Default for WorldBrokerConfig {
    fn default() -> Self {
        Self {
            enabled_adapters: vec!["http".to_string(), "websocket".to_string()],
            salience_threshold: 0.5,
            event_buffer_size: 1000,
            enable_attention_filter: true,
            adapter_configs: HashMap::new(),
            prediction_error_weight: 0.3,
            novelty_weight: 0.2,
            urgency_weight: 0.2,
            relevance_weight: 0.2,
            magnitude_weight: 0.1,
            enable_predictive_processing: true,
            context_window_size: 100,
        }
    }
}

impl WorldBrokerConfig {
    /// Validate configuration values
    pub fn validate(&self) -> Result<(), String> {
        // Validate salience threshold
        if !(0.0..=1.0).contains(&self.salience_threshold) {
            return Err("salience_threshold must be between 0.0 and 1.0".to_string());
        }

        // Validate weights sum to approximately 1.0
        let weight_sum = self.novelty_weight + self.urgency_weight + 
                        self.relevance_weight + self.magnitude_weight + 
                        self.prediction_error_weight;
        if (weight_sum - 1.0).abs() > 0.01 {
            return Err(format!("Attention filter weights must sum to ~1.0, got {}", weight_sum));
        }

        // Validate weights are non-negative
        if self.novelty_weight < 0.0 || self.urgency_weight < 0.0 ||
           self.relevance_weight < 0.0 || self.magnitude_weight < 0.0 ||
           self.prediction_error_weight < 0.0 {
            return Err("All attention filter weights must be non-negative".to_string());
        }

        // Validate buffer size
        if self.event_buffer_size == 0 {
            return Err("event_buffer_size must be > 0".to_string());
        }

        // Validate context window size
        if self.context_window_size == 0 {
            return Err("context_window_size must be > 0".to_string());
        }

        Ok(())
    }
}

