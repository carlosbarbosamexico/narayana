// Arrow of Time Controller - Bidirectional Temporal Training
// Manages temporal direction and experience ordering for CPL training
// References:
// - Bidirectional Training: https://arxiv.org/abs/2109.07780
// - Arrow of Time in Physics: https://en.wikipedia.org/wiki/Arrow_of_time

use crate::cognitive::Experience;
use crate::entropy_controller::EntropyController;
use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use std::cmp::Ordering;
use tracing::{debug, info};

/// Temporal direction for training
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeDirection {
    /// Forward: chronological order (oldest → newest)
    Forward,
    /// Backward: reverse chronological order (newest → oldest)
    Backward,
    /// Bidirectional: mix forward and backward based on entropy
    Bidirectional,
}

/// Experience ordering strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderingStrategy {
    /// Order by timestamp (chronological or reverse)
    Temporal,
    /// Order by entropy (highest first)
    Entropy,
    /// Order by complexity (increasing or decreasing)
    Complexity,
    /// Mixed: combine temporal and entropy
    Mixed,
}

/// Arrow of Time Controller configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AOTConfig {
    /// Enable arrow of time system
    pub enable_arrow_of_time: bool,
    /// Temporal direction
    pub time_direction: TimeDirection,
    /// Ordering strategy
    pub ordering_strategy: OrderingStrategy,
    /// Enable entropy-based sampling
    pub entropy_based_sampling: bool,
    /// Entropy threshold for bidirectional mode (0.0-1.0)
    pub bidirectional_entropy_threshold: f64,
}

impl Default for AOTConfig {
    fn default() -> Self {
        Self {
            enable_arrow_of_time: true,
            time_direction: TimeDirection::Forward,
            ordering_strategy: OrderingStrategy::Temporal,
            entropy_based_sampling: true,
            bidirectional_entropy_threshold: 0.5,
        }
    }
}

/// Arrow of Time Controller
pub struct ArrowOfTimeController {
    config: AOTConfig,
    entropy_controller: Arc<EntropyController>,
}

impl ArrowOfTimeController {
    /// Create new Arrow of Time Controller
    pub fn new(
        config: AOTConfig,
        entropy_controller: Arc<EntropyController>,
    ) -> Self {
        info!("ArrowOfTimeController initialized with direction: {:?}", config.time_direction);
        Self {
            config,
            entropy_controller,
        }
    }

    /// Order experiences based on temporal direction and strategy
    pub fn order_experiences(&self, experiences: &mut Vec<Experience>) -> Result<()> {
        if !self.config.enable_arrow_of_time {
            return Ok(()); // No ordering if disabled
        }

        // SECURITY: Limit experience count to prevent DoS
        const MAX_EXPERIENCES: usize = 1_000_000;
        if experiences.len() > MAX_EXPERIENCES {
            // Truncate to prevent DoS
            experiences.truncate(MAX_EXPERIENCES);
            tracing::warn!("Experience list truncated to {} to prevent DoS", MAX_EXPERIENCES);
        }

        match self.config.ordering_strategy {
            OrderingStrategy::Temporal => {
                self.order_by_temporal(experiences)?;
            }
            OrderingStrategy::Entropy => {
                self.order_by_entropy(experiences)?;
            }
            OrderingStrategy::Complexity => {
                self.order_by_complexity(experiences)?;
            }
            OrderingStrategy::Mixed => {
                self.order_by_mixed(experiences)?;
            }
        }

        Ok(())
    }

    /// Order experiences by temporal direction
    fn order_by_temporal(&self, experiences: &mut Vec<Experience>) -> Result<()> {
        match self.config.time_direction {
            TimeDirection::Forward => {
                // Chronological: oldest first
                experiences.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
            }
            TimeDirection::Backward => {
                // Reverse chronological: newest first
                experiences.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            }
            TimeDirection::Bidirectional => {
                // Mix based on entropy
                self.order_bidirectional(experiences)?;
            }
        }
        Ok(())
    }

    /// Order experiences by entropy (highest first)
    fn order_by_entropy(&self, experiences: &mut Vec<Experience>) -> Result<()> {
        // Calculate entropy for each experience
        let mut experiences_with_entropy: Vec<(Experience, f64)> = experiences
            .drain(..)
            .map(|exp| {
                let entropy = self.entropy_controller
                    .calculate_experience_entropy(&exp)
                    .unwrap_or(0.0);
                (exp, entropy)
            })
            .collect();

        // Sort by entropy (descending - highest first)
        experiences_with_entropy.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal)
        });

        // Extract experiences
        *experiences = experiences_with_entropy
            .into_iter()
            .map(|(exp, _)| exp)
            .collect();

        debug!("Ordered {} experiences by entropy", experiences.len());
        Ok(())
    }

    /// Order experiences by complexity (inferred from entropy)
    fn order_by_complexity(&self, experiences: &mut Vec<Experience>) -> Result<()> {
        // Calculate complexity (similar to entropy)
        let mut experiences_with_complexity: Vec<(Experience, f64)> = experiences
            .drain(..)
            .map(|exp| {
                let complexity = self.calculate_complexity(&exp);
                // EDGE CASE: Ensure complexity is finite
                let complexity = if complexity.is_finite() {
                    complexity.clamp(0.0, 1.0)
                } else {
                    0.0
                };
                (exp, complexity)
            })
            .collect();

        // Sort by complexity based on direction
        match self.config.time_direction {
            TimeDirection::Forward => {
                // Increasing complexity
                experiences_with_complexity.sort_by(|a, b| {
                    a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal)
                });
            }
            TimeDirection::Backward => {
                // Decreasing complexity
                experiences_with_complexity.sort_by(|a, b| {
                    b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal)
                });
            }
            TimeDirection::Bidirectional => {
                // Mix based on entropy
                experiences_with_complexity.sort_by(|a, b| {
                    b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal)
                });
            }
        }

        *experiences = experiences_with_complexity
            .into_iter()
            .map(|(exp, _)| exp)
            .collect();

        Ok(())
    }

    /// Order experiences by mixed strategy (temporal + entropy)
    fn order_by_mixed(&self, experiences: &mut Vec<Experience>) -> Result<()> {
        // First order by temporal
        self.order_by_temporal(experiences)?;

        // Then apply entropy-based sampling if enabled
        if self.config.entropy_based_sampling {
            // Weight experiences by entropy for sampling
            // Higher entropy experiences get higher priority
            let current_entropy = self.entropy_controller.get_entropy();
            
            // Reorder to prioritize experiences with entropy close to current
            experiences.sort_by(|a, b| {
                let entropy_a = self.entropy_controller
                    .calculate_experience_entropy(a)
                    .unwrap_or(0.0);
                let entropy_b = self.entropy_controller
                    .calculate_experience_entropy(b)
                    .unwrap_or(0.0);
                
                // EDGE CASE: Ensure entropies are finite
                let entropy_a = if entropy_a.is_finite() {
                    entropy_a.clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let entropy_b = if entropy_b.is_finite() {
                    entropy_b.clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let current_entropy_safe = if current_entropy.is_finite() {
                    current_entropy.clamp(0.0, 1.0)
                } else {
                    0.5
                };
                
                // Prefer experiences with entropy close to current
                let diff_a = (entropy_a - current_entropy_safe).abs();
                let diff_b = (entropy_b - current_entropy_safe).abs();
                
                // EDGE CASE: Ensure differences are finite before comparison
                if diff_a.is_finite() && diff_b.is_finite() {
                    diff_a.partial_cmp(&diff_b).unwrap_or(Ordering::Equal)
                } else {
                    Ordering::Equal
                }
            });
        }

        Ok(())
    }

    /// Order experiences in bidirectional mode
    fn order_bidirectional(&self, experiences: &mut Vec<Experience>) -> Result<()> {
        // SECURITY: Validate and clamp threshold
        let threshold = if self.config.bidirectional_entropy_threshold.is_finite() {
            self.config.bidirectional_entropy_threshold.clamp(0.0, 1.0)
        } else {
            0.5 // Default if invalid
        };
        let _current_entropy = self.entropy_controller.get_entropy();

        // Split experiences into forward and backward based on entropy
        let mut forward_experiences = Vec::new();
        let mut backward_experiences = Vec::new();

        for exp in experiences.drain(..) {
            let entropy = self.entropy_controller
                .calculate_experience_entropy(&exp)
                .unwrap_or(0.0);

            if entropy >= threshold {
                // High entropy -> forward (increasing complexity)
                forward_experiences.push(exp);
            } else {
                // Low entropy -> backward (decreasing complexity)
                backward_experiences.push(exp);
            }
        }

        // Sort forward experiences chronologically
        forward_experiences.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        
        // Sort backward experiences reverse chronologically
        backward_experiences.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Interleave: alternate between forward and backward
        let mut result = Vec::new();
        let mut forward_iter = forward_experiences.into_iter();
        let mut backward_iter = backward_experiences.into_iter();

        loop {
            let mut added = false;
            if let Some(exp) = forward_iter.next() {
                result.push(exp);
                added = true;
            }
            if let Some(exp) = backward_iter.next() {
                result.push(exp);
                added = true;
            }
            if !added {
                break;
            }
        }

        *experiences = result;
        debug!("Bidirectional ordering: {} experiences", experiences.len());
        Ok(())
    }

    /// Calculate complexity score for experience
    fn calculate_complexity(&self, experience: &Experience) -> f64 {
        // Complexity is similar to entropy but can include additional factors
        let entropy = self.entropy_controller
            .calculate_experience_entropy(experience)
            .unwrap_or(0.0);

        // Add complexity factors:
        // - Number of patterns
        let pattern_complexity = (experience.patterns.len() as f64 / 10.0).min(1.0) * 0.2;
        
        // - Context size
        let context_complexity = (experience.context.len() as f64 / 20.0).min(1.0) * 0.1;

        // - Observation complexity (if JSON object/array)
        let obs_complexity = if experience.observation.is_object() || experience.observation.is_array() {
            0.1
        } else {
            0.0
        };

        (entropy * 0.6 + pattern_complexity + context_complexity + obs_complexity).clamp(0.0, 1.0)
    }

    /// Select experiences based on entropy sampling
    pub fn sample_by_entropy(&self, experiences: &[Experience], count: usize) -> Result<Vec<Experience>> {
        // SECURITY: Limit sample count to prevent DoS
        const MAX_SAMPLE_COUNT: usize = 10000;
        let count = count.min(MAX_SAMPLE_COUNT).min(experiences.len());
        
        if !self.config.entropy_based_sampling {
            // Return first N experiences
            return Ok(experiences.iter().take(count).cloned().collect());
        }

        let current_entropy = self.entropy_controller.get_entropy();

        // Calculate entropy for each experience
        let mut experiences_with_entropy: Vec<(Experience, f64)> = experiences
            .iter()
            .map(|exp| {
                let entropy = self.entropy_controller
                    .calculate_experience_entropy(exp)
                    .unwrap_or(0.0);
                // EDGE CASE: Ensure entropy is finite
                let entropy = if entropy.is_finite() {
                    entropy.clamp(0.0, 1.0)
                } else {
                    0.0
                };
                // Weight by distance from current entropy (prefer similar entropy)
                let weight = 1.0 - (entropy - current_entropy).abs();
                // EDGE CASE: Ensure weight is finite
                let weight = if weight.is_finite() {
                    weight.clamp(0.0, 1.0)
                } else {
                    0.0
                };
                (exp.clone(), weight)
            })
            .collect();

        // Sort by weight (descending)
        experiences_with_entropy.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal)
        });

        // Take top N
        let selected: Vec<Experience> = experiences_with_entropy
            .into_iter()
            .take(count)
            .map(|(exp, _)| exp)
            .collect();

        Ok(selected)
    }

    /// Get current time direction
    pub fn time_direction(&self) -> TimeDirection {
        self.config.time_direction
    }

    /// Set time direction
    pub fn set_time_direction(&mut self, direction: TimeDirection) {
        self.config.time_direction = direction;
        info!("Time direction changed to: {:?}", direction);
    }

    /// Get entropy controller reference
    pub fn entropy_controller(&self) -> &Arc<EntropyController> {
        &self.entropy_controller
    }

    /// Get ordering strategy
    pub fn ordering_strategy(&self) -> OrderingStrategy {
        self.config.ordering_strategy
    }

    /// Set ordering strategy
    pub fn set_ordering_strategy(&mut self, strategy: OrderingStrategy) {
        self.config.ordering_strategy = strategy;
        info!("Ordering strategy changed to: {:?}", strategy);
    }

    /// Get configuration
    pub fn config(&self) -> &AOTConfig {
        &self.config
    }
}

