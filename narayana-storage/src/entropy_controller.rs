// Entropy Controller - Dynamic Entropy Management for Arrow of Time
// Implements configurable entropy as a variable with multiple policies
// Based on EntroPIC: Entropy Stabilization with Proportional-Integral Control
// References:
// - EntroPIC: https://arxiv.org/abs/2511.15248
// - Preventing Attention Entropy Collapse: https://arxiv.org/abs/2303.06296

use crate::cognitive::Experience;
use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

/// Entropy adjustment policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntropyPolicy {
    /// Keep entropy constant at initial value
    Fixed,
    /// Linear increase/decrease over time
    Linear,
    /// Exponential increase/decrease
    Exponential,
    /// Adapt based on performance metrics
    Adaptive,
    /// Adjust based on conditions
    Conditional,
}

/// Condition for conditional entropy adjustment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntropyCondition {
    /// Metric name (e.g., "performance", "complexity", "loss")
    pub metric: String,
    /// Comparison operator ("gt", "lt", "eq", "gte", "lte")
    pub operator: String,
    /// Threshold value
    pub threshold: f64,
    /// How much to adjust entropy when condition is met
    pub entropy_adjustment: f64,
}

/// Entropy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntropyConfig {
    /// Initial entropy value (0.0 to 1.0, or None for auto-calculation)
    pub initial_entropy: Option<f64>,
    /// Entropy adjustment policy
    pub entropy_policy: EntropyPolicy,
    /// Enable dynamic entropy adjustment during training
    pub enable_dynamic_entropy: bool,
    /// Enable conditional entropy adjustment
    pub enable_conditional_entropy: bool,
    /// Entropy stabilization (EntroPIC-like control)
    pub enable_entropy_stabilization: bool,
    /// Target entropy value for stabilization (if using stabilization)
    pub target_entropy: Option<f64>,
    /// Entropy adjustment rate (for linear/exponential policies)
    pub entropy_adjustment_rate: f64,
    /// Conditions for conditional entropy adjustment
    pub entropy_conditions: Vec<EntropyCondition>,
}

impl Default for EntropyConfig {
    fn default() -> Self {
        Self {
            initial_entropy: None, // Auto-calculate
            entropy_policy: EntropyPolicy::Fixed,
            enable_dynamic_entropy: false,
            enable_conditional_entropy: false,
            enable_entropy_stabilization: false,
            target_entropy: None,
            entropy_adjustment_rate: 0.01,
            entropy_conditions: Vec::new(),
        }
    }
}

/// Entropy history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EntropyHistoryEntry {
    timestamp: u64,
    entropy: f64,
    source: String, // "calculated", "set", "adjusted", "conditional"
}

/// Entropy Controller - Manages dynamic entropy for Arrow of Time system
pub struct EntropyController {
    /// Current entropy value (0.0 to 1.0)
    current_entropy: Arc<RwLock<f64>>,
    /// Configuration
    config: EntropyConfig,
    /// Entropy history for tracking
    history: Arc<RwLock<VecDeque<EntropyHistoryEntry>>>,
    /// PI controller state for entropy stabilization (EntroPIC-inspired)
    pi_controller: Arc<RwLock<PIController>>,
    /// Training iteration count (for linear/exponential policies)
    iteration: Arc<RwLock<u64>>,
    /// Performance metrics cache (for adaptive/conditional policies)
    metrics_cache: Arc<RwLock<HashMap<String, f64>>>,
}

/// Proportional-Integral controller for entropy stabilization
#[derive(Debug, Clone)]
struct PIController {
    kp: f64, // Proportional gain
    ki: f64, // Integral gain
    integral: f64, // Integral term
    last_error: f64, // Last error for derivative (if needed)
}

impl PIController {
    fn new(kp: f64, ki: f64) -> Self {
        Self {
            kp,
            ki,
            integral: 0.0,
            last_error: 0.0,
        }
    }

    /// Update PI controller and return adjustment
    fn update(&mut self, error: f64) -> f64 {
        self.integral += error;
        // Clamp integral to prevent windup
        self.integral = self.integral.clamp(-1.0, 1.0);
        
        let adjustment = self.kp * error + self.ki * self.integral;
        self.last_error = error;
        adjustment
    }

    fn reset(&mut self) {
        self.integral = 0.0;
        self.last_error = 0.0;
    }
}

use std::collections::HashMap;

impl EntropyController {
    /// Create new entropy controller
    pub fn new(config: EntropyConfig) -> Self {
        let initial_entropy = config.initial_entropy.unwrap_or(0.5); // Default to 0.5 if not set
        
        // Validate initial entropy
        let initial_entropy = initial_entropy.clamp(0.0, 1.0);
        
        let pi_controller = if config.enable_entropy_stabilization {
            PIController::new(0.1, 0.01) // Default PI gains
        } else {
            PIController::new(0.0, 0.0)
        };

        let controller = Self {
            current_entropy: Arc::new(RwLock::new(initial_entropy)),
            config,
            history: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            pi_controller: Arc::new(RwLock::new(pi_controller)),
            iteration: Arc::new(RwLock::new(0)),
            metrics_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        // Record initial entropy
        controller.record_entropy(initial_entropy, "initialized".to_string());

        info!("EntropyController initialized with entropy: {:.3}", initial_entropy);
        controller
    }

    /// Get current entropy value
    pub fn get_entropy(&self) -> f64 {
        *self.current_entropy.read()
    }

    /// Set entropy value directly (runtime adjustment)
    pub fn set_entropy(&self, entropy: f64) -> Result<()> {
        // SECURITY: Validate input
        if entropy.is_nan() || entropy.is_infinite() {
            return Err(Error::Storage("Invalid entropy value (NaN or Infinity)".to_string()));
        }
        
        let entropy = entropy.clamp(0.0, 1.0);

        *self.current_entropy.write() = entropy;
        self.record_entropy(entropy, "set".to_string());
        
        debug!("Entropy set to: {:.3}", entropy);
        Ok(())
    }

    /// Calculate entropy from experience
    pub fn calculate_experience_entropy(&self, experience: &Experience) -> Result<f64> {
        // Use Shannon entropy: H(X) = -Σ p(x) log p(x)
        
        // If experience has embedding, calculate entropy from embedding distribution
        if let Some(ref embedding) = experience.embedding {
            return Ok(self.calculate_embedding_entropy(embedding));
        }

        // Otherwise, calculate from observation/context data
        let mut entropy = 0.0;
        
        // Calculate entropy from observation JSON
        if let Some(obs_entropy) = self.calculate_json_entropy(&experience.observation) {
            entropy += obs_entropy * 0.5; // Weight observation
        }

        // Calculate entropy from patterns
        if !experience.patterns.is_empty() {
            let pattern_entropy = self.calculate_pattern_entropy(&experience.patterns);
            entropy += pattern_entropy * 0.3; // Weight patterns
        }

        // Calculate entropy from context
        if !experience.context.is_empty() {
            let context_entropy = self.calculate_context_entropy(&experience.context);
            entropy += context_entropy * 0.2; // Weight context
        }

        // Normalize to 0.0-1.0
        Ok(entropy.clamp(0.0, 1.0))
    }

    /// Calculate entropy from embedding vector
    fn calculate_embedding_entropy(&self, embedding: &[f32]) -> f64 {
        if embedding.is_empty() {
            return 0.0;
        }

        // Discretize embedding values into bins for entropy calculation
        const BINS: usize = 20;
        let mut bins = vec![0; BINS];
        let min_val = embedding.iter().copied().fold(f32::INFINITY, f32::min);
        let max_val = embedding.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let range = (max_val - min_val).max(1e-6);

        for &val in embedding {
            // EDGE CASE: Prevent overflow in bin calculation
            let bin_f = ((val - min_val) / range * (BINS - 1) as f32);
            // EDGE CASE: Check for NaN/Infinity before casting
            let bin = if bin_f.is_finite() && bin_f >= 0.0 {
                bin_f as usize
            } else {
                0 // Default to first bin if invalid
            };
            let bin = bin.min(BINS - 1);
            bins[bin] += 1;
        }

        // Calculate Shannon entropy
        // EDGE CASE: Prevent precision loss for very large embeddings
        let total = embedding.len();
        if total == 0 {
            return 0.0;
        }
        let total_f64 = total as f64;
        // EDGE CASE: Check for overflow in conversion
        if total_f64.is_infinite() || total_f64.is_nan() {
            return 0.0;
        }
        
        let mut entropy = 0.0;
        for &count in &bins {
            if count > 0 {
                // EDGE CASE: Prevent division by zero (already checked, but be safe)
                if total_f64 > 0.0 {
                    let p = count as f64 / total_f64;
                    // EDGE CASE: Validate probability
                    if p > 0.0 && p <= 1.0 {
                        let term = p * p.ln();
                        // EDGE CASE: Check for NaN (ln(0) = -inf, but we check p > 0)
                        if term.is_finite() {
                            entropy -= term;
                        }
                    }
                }
            }
        }

        // Normalize by max entropy (ln(BINS))
        // EDGE CASE: Check for NaN/Infinity before division
        if !entropy.is_finite() {
            return 0.0;
        }
        let max_entropy = (BINS as f64).ln();
        if max_entropy > 0.0 {
            let normalized = entropy / max_entropy;
            if normalized.is_finite() {
                normalized.clamp(0.0, 1.0)
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// Calculate entropy from JSON value
    fn calculate_json_entropy(&self, value: &serde_json::Value) -> Option<f64> {
        match value {
            serde_json::Value::String(s) => {
                // Calculate entropy from string character distribution
                let mut char_counts = HashMap::new();
                for ch in s.chars() {
                    *char_counts.entry(ch).or_insert(0) += 1;
                }
                Some(self.calculate_shannon_entropy(&char_counts))
            }
            serde_json::Value::Array(arr) => {
                // SECURITY: Limit array size to prevent DoS
                const MAX_ARRAY_SIZE: usize = 10000;
                let arr = if arr.len() > MAX_ARRAY_SIZE {
                    &arr[..MAX_ARRAY_SIZE]
                } else {
                    arr
                };
                
                // Average entropy of array elements
                let entropies: Vec<f64> = arr.iter()
                    .filter_map(|v| self.calculate_json_entropy(v))
                    .collect();
                if entropies.is_empty() {
                    None
                } else {
                    // EDGE CASE: Prevent division by zero (already checked, but be safe)
                    let len = entropies.len() as f64;
                    if len > 0.0 {
                        let avg = entropies.iter().sum::<f64>() / len;
                        // EDGE CASE: Check for NaN/Infinity
                        if avg.is_finite() {
                            Some(avg)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
            }
            serde_json::Value::Object(obj) => {
                // SECURITY: Limit object size to prevent DoS
                const MAX_OBJECT_SIZE: usize = 10000;
                let values_iter: Box<dyn Iterator<Item = &serde_json::Value>> = if obj.len() > MAX_OBJECT_SIZE {
                    // Take first MAX_OBJECT_SIZE entries
                    Box::new(obj.values().take(MAX_OBJECT_SIZE))
                } else {
                    Box::new(obj.values())
                };
                
                // Average entropy of object values
                let entropies: Vec<f64> = values_iter
                    .filter_map(|v| self.calculate_json_entropy(v))
                    .collect();
                if entropies.is_empty() {
                    None
                } else {
                    // EDGE CASE: Prevent division by zero
                    let len = entropies.len() as f64;
                    if len > 0.0 {
                        let avg = entropies.iter().sum::<f64>() / len;
                        // EDGE CASE: Check for NaN/Infinity
                        if avg.is_finite() {
                            Some(avg)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
            }
            _ => None, // Numbers, booleans, null have low/no entropy
        }
    }

    /// Calculate entropy from patterns
    fn calculate_pattern_entropy(&self, patterns: &[crate::cognitive::Pattern]) -> f64 {
        if patterns.is_empty() {
            return 0.0;
        }

        // Calculate entropy based on pattern diversity
        let mut type_counts = HashMap::new();
        for pattern in patterns {
            *type_counts.entry(&pattern.pattern_type).or_insert(0) += 1;
        }

        self.calculate_shannon_entropy(&type_counts)
    }

    /// Calculate entropy from context
    fn calculate_context_entropy(&self, context: &HashMap<String, serde_json::Value>) -> f64 {
        if context.is_empty() {
            return 0.0;
        }

        // Calculate entropy from context key diversity and value complexity
        let key_entropy = (context.len() as f64).ln() / 10.0; // Normalized by reasonable max
        let value_entropies: Vec<f64> = context.values()
            .filter_map(|v| self.calculate_json_entropy(v))
            .collect();
        
        let avg_value_entropy = if value_entropies.is_empty() {
            0.0
        } else {
            value_entropies.iter().sum::<f64>() / value_entropies.len() as f64
        };

        (key_entropy + avg_value_entropy).clamp(0.0, 1.0)
    }

    /// Calculate Shannon entropy from counts
    fn calculate_shannon_entropy<T>(&self, counts: &HashMap<T, usize>) -> f64
    where
        T: std::hash::Hash + Eq,
    {
        // EDGE CASE: Empty counts
        if counts.is_empty() {
            return 0.0;
        }

        let total: usize = counts.values().sum();
        // EDGE CASE: Prevent division by zero
        if total == 0 {
            return 0.0;
        }

        let mut entropy = 0.0;
        for &count in counts.values() {
            if count > 0 {
                let p = count as f64 / total as f64;
                // EDGE CASE: Check for valid probability (0 < p <= 1)
                if p > 0.0 && p <= 1.0 {
                    let term = p * p.ln();
                    // EDGE CASE: Check for NaN (ln(0) = -inf, but we check p > 0)
                    if term.is_finite() {
                        entropy -= term;
                    }
                }
            }
        }

        // EDGE CASE: Check for NaN/Infinity
        if !entropy.is_finite() {
            return 0.0;
        }

        // Normalize by max entropy (ln(n))
        let n = counts.len() as f64;
        if n > 1.0 {
            let normalized = entropy / n.ln();
            // EDGE CASE: Check for NaN/Infinity after normalization
            if normalized.is_finite() {
                normalized.clamp(0.0, 1.0)
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// Update entropy based on policy (called during training iterations)
    pub fn update_entropy(&self) -> Result<()> {
        if !self.config.enable_dynamic_entropy {
            return Ok(()); // Dynamic entropy disabled
        }

        let current = *self.current_entropy.read();
        let iteration = {
            let mut iter = self.iteration.write();
            *iter += 1;
            *iter
        };

        let new_entropy = match self.config.entropy_policy {
            EntropyPolicy::Fixed => current, // No change
            EntropyPolicy::Linear => {
                // Linear increase: entropy = initial + rate * iteration
                let initial = self.config.initial_entropy.unwrap_or(0.5);
                let rate = self.config.entropy_adjustment_rate;
                (initial + rate * iteration as f64).clamp(0.0, 1.0)
            }
            EntropyPolicy::Exponential => {
                // Exponential: entropy = initial * (1 + rate)^iteration
                let initial = self.config.initial_entropy.unwrap_or(0.5);
                let rate = self.config.entropy_adjustment_rate;
                (initial * (1.0 + rate).powi(iteration as i32)).clamp(0.0, 1.0)
            }
            EntropyPolicy::Adaptive => {
                // Adapt based on performance metrics
                self.calculate_adaptive_entropy(current)?
            }
            EntropyPolicy::Conditional => {
                // Apply conditional adjustments
                self.apply_conditional_entropy(current)?
            }
        };

        // Apply entropy stabilization if enabled
        let final_entropy = if self.config.enable_entropy_stabilization {
            self.apply_entropy_stabilization(new_entropy)?
        } else {
            new_entropy
        };

        if (final_entropy - current).abs() > 1e-6 {
            *self.current_entropy.write() = final_entropy;
            self.record_entropy(final_entropy, "policy_update".to_string());
            debug!("Entropy updated to {:.3} (policy: {:?})", final_entropy, self.config.entropy_policy);
        }

        Ok(())
    }

    /// Calculate adaptive entropy based on performance metrics
    fn calculate_adaptive_entropy(&self, current: f64) -> Result<f64> {
        let metrics = self.metrics_cache.read();
        
        // If no metrics available, return current
        if metrics.is_empty() {
            return Ok(current);
        }

        // Calculate entropy adjustment based on performance
        // Higher performance -> increase entropy (more exploration)
        // Lower performance -> decrease entropy (more exploitation)
        let performance = metrics.get("performance").copied().unwrap_or(0.5);
        let loss = metrics.get("loss").copied().unwrap_or(0.5);
        
        // Adjust entropy: high performance -> increase, low loss -> increase
        let adjustment = (performance - 0.5) * 0.1 - (loss - 0.5) * 0.1;
        // EDGE CASE: Validate adjustment
        let adjustment = if adjustment.is_finite() {
            adjustment.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        
        let new_entropy = (current + adjustment).clamp(0.0, 1.0);
        // EDGE CASE: Final validation
        if new_entropy.is_finite() {
            Ok(new_entropy)
        } else {
            Ok(current) // Return current if calculation fails
        }
    }

    /// Apply conditional entropy adjustments
    fn apply_conditional_entropy(&self, current: f64) -> Result<f64> {
        if !self.config.enable_conditional_entropy {
            return Ok(current);
        }

        let metrics = self.metrics_cache.read();
        let mut adjusted_entropy = current;

        for condition in &self.config.entropy_conditions {
            let metric_value = metrics.get(&condition.metric).copied().unwrap_or(0.0);
            // EDGE CASE: Validate metric value
            let metric_value = if metric_value.is_finite() {
                metric_value
            } else {
                0.0
            };
            // EDGE CASE: Validate threshold
            let threshold = if condition.threshold.is_finite() {
                condition.threshold
            } else {
                warn!("Invalid threshold in condition, skipping");
                continue;
            };
            
            let should_apply = match condition.operator.as_str() {
                "gt" => metric_value > threshold,
                "lt" => metric_value < threshold,
                "eq" => (metric_value - threshold).abs() < 1e-6,
                "gte" => metric_value >= threshold,
                "lte" => metric_value <= threshold,
                _ => {
                    warn!("Unknown operator: {}", condition.operator);
                    false
                }
            };

            if should_apply {
                // EDGE CASE: Validate adjustment value
                let adjustment = if condition.entropy_adjustment.is_finite() {
                    condition.entropy_adjustment.clamp(-1.0, 1.0)
                } else {
                    warn!("Invalid entropy_adjustment in condition, skipping");
                    continue;
                };
                
                adjusted_entropy += adjustment;
                // EDGE CASE: Clamp result after each adjustment
                adjusted_entropy = adjusted_entropy.clamp(0.0, 1.0);
                
                debug!("Conditional entropy adjustment: {} {} {} -> adjustment: {:.3}, new entropy: {:.3}", 
                       condition.metric, condition.operator, condition.threshold, 
                       adjustment, adjusted_entropy);
            }
        }

        // EDGE CASE: Final validation
        if adjusted_entropy.is_finite() {
            Ok(adjusted_entropy.clamp(0.0, 1.0))
        } else {
            Ok(current) // Return current if calculation fails
        }
    }

    /// Apply entropy stabilization (EntroPIC-inspired)
    fn apply_entropy_stabilization(&self, entropy: f64) -> Result<f64> {
        // EDGE CASE: Validate input entropy
        let entropy = if entropy.is_finite() {
            entropy.clamp(0.0, 1.0)
        } else {
            return Err(Error::Storage("Invalid entropy value (NaN or Infinity)".to_string()));
        };
        
        let target = self.config.target_entropy.unwrap_or(0.5);
        // EDGE CASE: Validate target
        let target = if target.is_finite() {
            target.clamp(0.0, 1.0)
        } else {
            0.5 // Default if invalid
        };
        
        let error = target - entropy;
        // EDGE CASE: Validate error
        let error = if error.is_finite() {
            error
        } else {
            0.0
        };

        let mut pi = self.pi_controller.write();
        let adjustment = pi.update(error);
        // EDGE CASE: Validate adjustment
        let adjustment = if adjustment.is_finite() {
            adjustment.clamp(-1.0, 1.0)
        } else {
            0.0
        };

        let stabilized = (entropy + adjustment).clamp(0.0, 1.0);
        // EDGE CASE: Final validation
        if stabilized.is_finite() {
            debug!("Entropy stabilization: target={:.3}, current={:.3}, adjustment={:.3}, stabilized={:.3}",
                   target, entropy, adjustment, stabilized);
            Ok(stabilized)
        } else {
            Ok(entropy) // Return original if stabilization fails
        }
    }

    /// Update performance metrics (for adaptive/conditional policies)
    pub fn update_metrics(&self, metrics: HashMap<String, f64>) {
        *self.metrics_cache.write() = metrics;
    }

    /// Record entropy in history
    fn record_entropy(&self, entropy: f64, source: String) {
        // EDGE CASE: Validate entropy before recording
        let entropy = if entropy.is_finite() {
            entropy.clamp(0.0, 1.0)
        } else {
            warn!("Attempted to record invalid entropy: {}, skipping", entropy);
            return;
        };
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut history = self.history.write();
        history.push_back(EntropyHistoryEntry {
            timestamp,
            entropy,
            source,
        });
        
        // EDGE CASE: Limit history size to prevent memory growth
        const MAX_HISTORY_SIZE: usize = 10000;
        while history.len() > MAX_HISTORY_SIZE {
            history.pop_front();
        }

        // Keep history bounded
        const MAX_HISTORY: usize = 1000;
        while history.len() > MAX_HISTORY {
            history.pop_front();
        }
    }

    /// Get entropy history
    pub fn get_history(&self) -> Vec<EntropyHistoryEntry> {
        self.history.read().iter().cloned().collect()
    }

    /// Reset entropy controller
    pub fn reset(&self) {
        let initial = self.config.initial_entropy.unwrap_or(0.5);
        *self.current_entropy.write() = initial;
        *self.iteration.write() = 0;
        self.pi_controller.write().reset();
        self.metrics_cache.write().clear();
        self.history.write().clear();
        self.record_entropy(initial, "reset".to_string());
    }

    /// Get configuration
    pub fn config(&self) -> &EntropyConfig {
        &self.config
    }

    /// Get current iteration count
    pub fn iteration(&self) -> u64 {
        *self.iteration.read()
    }

    /// Get entropy statistics
    pub fn get_statistics(&self) -> EntropyStatistics {
        let history = self.history.read();
        let current = *self.current_entropy.read();
        
        let min_entropy = history.iter().map(|e| e.entropy).fold(f64::INFINITY, f64::min);
        let max_entropy = history.iter().map(|e| e.entropy).fold(f64::NEG_INFINITY, f64::max);
        let avg_entropy = if history.is_empty() {
            current
        } else {
            history.iter().map(|e| e.entropy).sum::<f64>() / history.len() as f64
        };

        EntropyStatistics {
            current,
            min: min_entropy,
            max: max_entropy,
            average: avg_entropy,
            history_length: history.len(),
        }
    }
}

/// Entropy statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntropyStatistics {
    pub current: f64,
    pub min: f64,
    pub max: f64,
    pub average: f64,
    pub history_length: usize,
}

/// Entropy controller state for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntropyControllerState {
    pub current_entropy: f64,
    pub iteration: u64,
    pub history: Vec<EntropyHistoryEntry>,
}

impl EntropyController {
    /// Export state for persistence
    pub fn export_state(&self) -> EntropyControllerState {
        EntropyControllerState {
            current_entropy: *self.current_entropy.read(),
            iteration: *self.iteration.read(),
            history: self.get_history(),
        }
    }

    /// Import state from persistence
    pub fn import_state(&self, state: EntropyControllerState) -> Result<()> {
        *self.current_entropy.write() = state.current_entropy.clamp(0.0, 1.0);
        *self.iteration.write() = state.iteration;
        
        let mut history = self.history.write();
        history.clear();
        for entry in state.history {
            history.push_back(entry);
        }
        
        Ok(())
    }
}

