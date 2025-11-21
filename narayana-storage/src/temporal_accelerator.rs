// Temporal Accelerator - Accelerate training through complexity ranges
// Compresses experience sequences while maintaining immersion and causality
// References:
// - Curriculum Learning: Bengio et al. (2009) ICML
// - Experience Replay: Mnih et al. (2015) Nature

use crate::cognitive::Experience;
use crate::entropy_controller::EntropyController;
use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{debug, info, warn};
use std::collections::HashMap;

/// Temporal acceleration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccelerationConfig {
    /// Acceleration ratio (e.g., 4.0 = 4x acceleration)
    pub acceleration_ratio: f64,
    /// Minimum experiences to keep (even if redundant)
    pub min_experiences: usize,
    /// Entropy threshold for compression (experiences below this can be compressed)
    pub compression_entropy_threshold: f64,
    /// Preserve causality relationships
    pub preserve_causality: bool,
    /// Maintain experience quality/immersion
    pub maintain_immersion: bool,
}

impl Default for AccelerationConfig {
    fn default() -> Self {
        Self {
            acceleration_ratio: 4.0,
            min_experiences: 10,
            compression_entropy_threshold: 0.1,
            preserve_causality: true,
            maintain_immersion: true,
        }
    }
}

impl AccelerationConfig {
    /// Validate acceleration configuration
    pub fn validate(&self) -> Result<()> {
        // SECURITY: Validate acceleration ratio
        if self.acceleration_ratio <= 0.0 || self.acceleration_ratio.is_nan() || self.acceleration_ratio.is_infinite() {
            return Err(Error::Storage("acceleration_ratio must be positive and finite".to_string()));
        }
        if self.acceleration_ratio > 1000.0 {
            return Err(Error::Storage("acceleration_ratio too large (max 1000.0)".to_string()));
        }
        
        // SECURITY: Validate compression threshold
        if self.compression_entropy_threshold < 0.0 || self.compression_entropy_threshold > 1.0 {
            return Err(Error::Storage("compression_entropy_threshold must be in [0.0, 1.0]".to_string()));
        }
        if self.compression_entropy_threshold.is_nan() || self.compression_entropy_threshold.is_infinite() {
            return Err(Error::Storage("compression_entropy_threshold must be finite".to_string()));
        }
        
        // SECURITY: Validate min_experiences
        if self.min_experiences == 0 {
            return Err(Error::Storage("min_experiences must be > 0".to_string()));
        }
        if self.min_experiences > 100000 {
            return Err(Error::Storage("min_experiences too large (max 100000)".to_string()));
        }
        
        Ok(())
    }
}

/// Temporal Accelerator
pub struct TemporalAccelerator {
    config: AccelerationConfig,
    entropy_controller: Arc<EntropyController>,
}

impl TemporalAccelerator {
    /// Create new temporal accelerator
    pub fn new(
        config: AccelerationConfig,
        entropy_controller: Arc<EntropyController>,
    ) -> Result<Self> {
        // SECURITY: Validate config
        config.validate()?;
        
        info!("TemporalAccelerator initialized with ratio: {:.1}x", config.acceleration_ratio);
        Ok(Self {
            config,
            entropy_controller,
        })
    }

    /// Accelerate experience sequence (compress while maintaining quality)
    pub fn accelerate(&self, experiences: Vec<Experience>) -> Result<Vec<Experience>> {
        let exp_len = experiences.len();
        if exp_len <= self.config.min_experiences {
            // Too few experiences to compress
            return Ok(experiences);
        }

        // EDGE CASE: Prevent overflow in calculation
        let exp_len_f64 = exp_len as f64;
        if exp_len_f64.is_infinite() || exp_len_f64.is_nan() {
            return Err(Error::Storage("Invalid experience count (NaN or Infinity)".to_string()));
        }
        
        // EDGE CASE: Prevent division by zero
        if self.config.acceleration_ratio <= 0.0 || self.config.acceleration_ratio.is_nan() || self.config.acceleration_ratio.is_infinite() {
            return Err(Error::Storage("Invalid acceleration_ratio".to_string()));
        }
        
        let target_count_f64 = (exp_len_f64 / self.config.acceleration_ratio).ceil();
        // EDGE CASE: Check for overflow before casting
        if target_count_f64 > usize::MAX as f64 {
            return Err(Error::Storage("Target count exceeds maximum usize".to_string()));
        }
        let target_count = target_count_f64 as usize;
        let target_count = target_count.max(self.config.min_experiences);

        if target_count >= exp_len {
            // No compression needed
            return Ok(experiences);
        }

        debug!("Accelerating {} experiences to {} (ratio: {:.1}x)",
               exp_len, target_count, self.config.acceleration_ratio);

        if self.config.preserve_causality {
            self.accelerate_with_causality(experiences, target_count)
        } else {
            self.accelerate_simple(experiences, target_count)
        }
    }

    /// Simple acceleration (no causality preservation)
    fn accelerate_simple(&self, experiences: Vec<Experience>, target_count: usize) -> Result<Vec<Experience>> {
        // SECURITY: Limit experiences to prevent DoS
        const MAX_EXPERIENCES: usize = 1_000_000;
        let experiences: Vec<Experience> = if experiences.len() > MAX_EXPERIENCES {
            experiences.into_iter().take(MAX_EXPERIENCES).collect()
        } else {
            experiences
        };
        
        // Calculate entropy for each experience
        let mut experiences_with_entropy: Vec<(Experience, f64)> = experiences
            .into_iter()
            .map(|exp| {
                let entropy = self.entropy_controller
                    .calculate_experience_entropy(&exp)
                    .unwrap_or(0.0);
                // EDGE CASE: Ensure entropy is finite
                let entropy = if entropy.is_finite() {
                    entropy.clamp(0.0, 1.0)
                } else {
                    0.0
                };
                (exp, entropy)
            })
            .collect();

        // Sort by entropy (descending - keep high entropy experiences)
        experiences_with_entropy.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Take top N experiences
        let selected: Vec<Experience> = experiences_with_entropy
            .into_iter()
            .take(target_count)
            .map(|(exp, _)| exp)
            .collect();

        Ok(selected)
    }

    /// Accelerate with causality preservation
    fn accelerate_with_causality(&self, experiences: Vec<Experience>, target_count: usize) -> Result<Vec<Experience>> {
        // SECURITY: Limit experiences to prevent DoS
        const MAX_EXPERIENCES: usize = 1_000_000;
        let experiences: Vec<Experience> = if experiences.len() > MAX_EXPERIENCES {
            experiences.into_iter().take(MAX_EXPERIENCES).collect()
        } else {
            experiences
        };
        
        // Group experiences by temporal proximity and calculate entropy
        let mut experiences_with_entropy: Vec<(Experience, f64, usize)> = Vec::new();
        for (idx, exp) in experiences.into_iter().enumerate() {
            let entropy = self.entropy_controller
                .calculate_experience_entropy(&exp)
                .unwrap_or(0.0);
            // EDGE CASE: Ensure entropy is finite
            let entropy = if entropy.is_finite() {
                entropy.clamp(0.0, 1.0)
            } else {
                0.0
            };
            experiences_with_entropy.push((exp, entropy, idx));
        }

        // Sort by timestamp to maintain temporal order
        experiences_with_entropy.sort_by(|a, b| {
            a.0.timestamp.cmp(&b.0.timestamp)
        });

        // Identify critical transition points (high entropy changes)
        let critical_indices = self.identify_critical_transitions(&experiences_with_entropy);

        // Select experiences:
        // 1. Always include critical transitions
        // 2. Fill remaining slots with high-entropy experiences
        let mut selected = Vec::new();
        let mut selected_indices = std::collections::HashSet::new();

                // Add critical transitions
                // EDGE CASE: Validate indices before access
                for &idx in &critical_indices {
                    // EDGE CASE: Bounds check
                    if idx < experiences_with_entropy.len() {
                        selected_indices.insert(idx);
                        // EDGE CASE: Clone to avoid borrow issues
                        selected.push(experiences_with_entropy[idx].0.clone());
                    } else {
                        warn!("Critical index {} out of bounds (len: {})", idx, experiences_with_entropy.len());
                    }
                }

        // Fill remaining slots with high-entropy experiences
        let mut remaining_slots = target_count.saturating_sub(selected.len());
        if remaining_slots > 0 {
            // Sort by entropy (descending)
            let mut sorted_by_entropy: Vec<(usize, f64)> = experiences_with_entropy
                .iter()
                .enumerate()
                .map(|(idx, (_, entropy, _))| (idx, *entropy))
                .collect();
            sorted_by_entropy.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            for (idx, _) in sorted_by_entropy {
                // EDGE CASE: Bounds check before access
                if idx < experiences_with_entropy.len() && !selected_indices.contains(&idx) && remaining_slots > 0 {
                    selected_indices.insert(idx);
                    selected.push(experiences_with_entropy[idx].0.clone());
                    remaining_slots -= 1;
                }
            }
        }

        // Sort selected by timestamp to maintain temporal order
        selected.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        debug!("Accelerated to {} experiences ({} critical transitions)",
               selected.len(), critical_indices.len());

        Ok(selected)
    }

    /// Identify critical transition points (where entropy changes significantly)
    fn identify_critical_transitions(&self, experiences: &[(Experience, f64, usize)]) -> Vec<usize> {
        if experiences.len() < 2 {
            return Vec::new();
        }

        let mut critical = Vec::new();
        let entropy_threshold = 0.2; // Significant entropy change threshold

        // Always include first
        critical.push(0);
        
        // Find transitions where entropy changes significantly
        // EDGE CASE: Prevent underflow (i starts at 1, so i-1 is safe)
        for i in 1..experiences.len() {
            // EDGE CASE: Bounds check (should be safe, but be defensive)
            if i >= experiences.len() || (i - 1) >= experiences.len() {
                break;
            }
            
            let prev_entropy = experiences[i - 1].1;
            let curr_entropy = experiences[i].1;
            
            // EDGE CASE: Ensure entropies are finite
            if !prev_entropy.is_finite() || !curr_entropy.is_finite() {
                continue; // Skip invalid entropies
            }
            
            let entropy_delta = (curr_entropy - prev_entropy).abs();
            
            // EDGE CASE: Ensure delta is finite
            if entropy_delta.is_finite() && entropy_delta > entropy_threshold {
                critical.push(i);
            }
        }

        // Always include last (if experiences is not empty)
        if !experiences.is_empty() {
            let last_idx = experiences.len() - 1;
            // EDGE CASE: Prevent duplicate
            if !critical.contains(&last_idx) {
                critical.push(last_idx);
            }
        }

        critical.sort();
        critical
    }

    /// Merge similar experiences while preserving key information
    pub fn merge_similar_experiences(&self, experiences: Vec<Experience>) -> Result<Vec<Experience>> {
        let exp_len = experiences.len();
        if exp_len <= 1 {
            return Ok(experiences);
        }

        let mut merged = Vec::new();
        let similarity_threshold = 0.1; // Entropy difference threshold for similarity

        for exp in experiences {
            if merged.is_empty() {
                merged.push(exp);
                continue;
            }

            let exp_entropy = self.entropy_controller
                .calculate_experience_entropy(&exp)
                .unwrap_or(0.0);

            // Check if similar to last merged experience
            // SECURITY: Prevent panic if merged is empty
            let last_merged = match merged.last() {
                Some(exp) => exp,
                None => {
                    // First experience, just add it
                    merged.push(exp);
                    continue;
                }
            };
            let last_entropy = self.entropy_controller
                .calculate_experience_entropy(last_merged)
                .unwrap_or(0.0);

            let entropy_diff = (exp_entropy - last_entropy).abs();

            if entropy_diff < similarity_threshold && self.config.maintain_immersion {
                // Merge: combine patterns and context, keep most recent timestamp
                let merged_exp = self.merge_two_experiences(last_merged, &exp)?;
                merged.pop();
                merged.push(merged_exp);
            } else {
                // Keep separate
                merged.push(exp);
            }
        }

        debug!("Merged {} experiences to {}", exp_len, merged.len());
        Ok(merged)
    }

    /// Merge two experiences into one
    fn merge_two_experiences(&self, exp1: &Experience, exp2: &Experience) -> Result<Experience> {
        // Combine patterns (deduplicate by pattern type)
        let mut merged_patterns = exp1.patterns.clone();
        let mut pattern_types: std::collections::HashSet<&crate::cognitive::PatternType> = std::collections::HashSet::new();
        for pattern in &exp1.patterns {
            pattern_types.insert(&pattern.pattern_type);
        }

        for pattern in &exp2.patterns {
            if !pattern_types.contains(&pattern.pattern_type) {
                merged_patterns.push(pattern.clone());
                pattern_types.insert(&pattern.pattern_type);
            }
        }

        // Combine context
        let mut merged_context = exp1.context.clone();
        for (key, value) in &exp2.context {
            merged_context.insert(key.clone(), value.clone());
        }

        // Use most recent timestamp
        let merged_timestamp = exp1.timestamp.max(exp2.timestamp);

        // Combine observations (merge JSON objects)
        // SECURITY: Safe unwrap with validation
        let merged_observation = if exp1.observation.is_object() && exp2.observation.is_object() {
            if let (Some(obj1_ref), Some(obj2_ref)) = (exp1.observation.as_object(), exp2.observation.as_object()) {
                let mut obj1 = obj1_ref.clone();
                // SECURITY: Limit object size to prevent DoS
                const MAX_OBJECT_KEYS: usize = 1000;
                if obj1.len() + obj2_ref.len() > MAX_OBJECT_KEYS {
                    // Truncate to prevent unbounded growth
                    let mut count = 0;
                    for (key, value) in obj2_ref {
                        if count >= MAX_OBJECT_KEYS - obj1.len() {
                            break;
                        }
                        obj1.insert(key.clone(), value.clone());
                        count += 1;
                    }
                } else {
                    for (key, value) in obj2_ref {
                        obj1.insert(key.clone(), value.clone());
                    }
                }
                serde_json::Value::Object(obj1)
            } else {
                // Fallback if unwrap fails
                exp1.observation.clone()
            }
        } else {
            // If not objects, prefer the more complex one
            let entropy1 = self.entropy_controller
                .calculate_experience_entropy(exp1)
                .unwrap_or(0.0);
            let entropy2 = self.entropy_controller
                .calculate_experience_entropy(exp2)
                .unwrap_or(0.0);
            
            if entropy2 > entropy1 {
                exp2.observation.clone()
            } else {
                exp1.observation.clone()
            }
        };

        // Average reward
        let merged_reward = match (exp1.reward, exp2.reward) {
            (Some(r1), Some(r2)) => Some((r1 + r2) / 2.0),
            (Some(r), None) | (None, Some(r)) => Some(r),
            (None, None) => None,
        };

        Ok(Experience {
            id: exp1.id.clone(), // Keep first ID
            event_type: format!("merged_{}", exp1.event_type),
            observation: merged_observation,
            action: exp1.action.clone().or_else(|| exp2.action.clone()),
            outcome: exp1.outcome.clone().or_else(|| exp2.outcome.clone()),
            reward: merged_reward,
            timestamp: merged_timestamp,
            context: merged_context,
            patterns: merged_patterns,
            embedding: exp1.embedding.clone().or_else(|| exp2.embedding.clone()),
            complexity: exp1.complexity.or(exp2.complexity),
            entropy: exp1.entropy.or(exp2.entropy),
            modality: exp1.modality.clone().or_else(|| exp2.modality.clone()),
        })
    }

    /// Get acceleration ratio
    pub fn acceleration_ratio(&self) -> f64 {
        self.config.acceleration_ratio
    }

    /// Set acceleration ratio
    pub fn set_acceleration_ratio(&mut self, ratio: f64) {
        if ratio > 0.0 {
            self.config.acceleration_ratio = ratio;
            info!("Acceleration ratio set to {:.1}x", ratio);
        }
    }
}

