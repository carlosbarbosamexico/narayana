// Experience Seeder - Seed initial experiences for CPL training
// Generates foundational experiences at specified complexity levels
// Supports multi-modal experience seeding (visual, audio, voice, sound)

use crate::cognitive::{CognitiveBrain, Experience};
use crate::complexity_range_simulator::{ComplexityRangeSimulator, ComplexityRange, ExperienceModality};
use crate::entropy_controller::EntropyController;
use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, debug};

/// Experience seeding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedingConfig {
    /// Number of experiences to seed
    pub seed_count: usize,
    /// Initial complexity level (0.0-1.0)
    pub initial_complexity: f64,
    /// Enable audio experiences in seeding
    pub enable_audio: bool,
    /// Ratio of audio experiences (0.0-1.0)
    pub audio_ratio: f64,
    /// Enable multi-modal experiences
    pub enable_multi_modal: bool,
    /// Distribution of modalities
    pub modality_distribution: ModalityDistribution,
}

/// Modality distribution for seeding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalityDistribution {
    /// Ratio of visual experiences (0.0-1.0)
    pub visual: f64,
    /// Ratio of audio experiences (0.0-1.0)
    pub audio: f64,
    /// Ratio of voice experiences (0.0-1.0)
    pub voice: f64,
    /// Ratio of sound experiences (0.0-1.0)
    pub sound: f64,
    /// Ratio of multi-modal experiences (0.0-1.0)
    pub multi_modal: f64,
}

impl Default for ModalityDistribution {
    fn default() -> Self {
        Self {
            visual: 0.4,
            audio: 0.2,
            voice: 0.2,
            sound: 0.1,
            multi_modal: 0.1,
        }
    }
}

impl Default for SeedingConfig {
    fn default() -> Self {
        Self {
            seed_count: 100,
            initial_complexity: 0.2, // Start with low complexity
            enable_audio: true,
            audio_ratio: 0.3,
            enable_multi_modal: true,
            modality_distribution: ModalityDistribution::default(),
        }
    }
}

/// Experience Seeder
pub struct ExperienceSeeder {
    brain: Arc<CognitiveBrain>,
    entropy_controller: Arc<EntropyController>,
}

impl ExperienceSeeder {
    /// Create new experience seeder
    pub fn new(
        brain: Arc<CognitiveBrain>,
        entropy_controller: Arc<EntropyController>,
    ) -> Self {
        Self {
            brain,
            entropy_controller,
        }
    }

    /// Seed experiences into the brain
    pub async fn seed_experiences(&self, config: SeedingConfig) -> Result<Vec<String>> {
        // SECURITY: Limit seed count to prevent DoS
        const MAX_SEED_COUNT: usize = 100000;
        let seed_count = config.seed_count.min(MAX_SEED_COUNT);
        
        // SECURITY: Validate complexity
        if config.initial_complexity.is_nan() || config.initial_complexity.is_infinite() {
            return Err(Error::Storage("Invalid initial_complexity (NaN or Infinity)".to_string()));
        }
        let initial_complexity = config.initial_complexity.clamp(0.0, 1.0);
        
        info!("Seeding {} experiences at complexity {:.2}", seed_count, initial_complexity);

        // Create complexity range simulator for generating experiences
        let complexity_range = ComplexityRange {
            start_complexity: initial_complexity,
            end_complexity: initial_complexity, // Same start/end for seeding
            enable_audio: config.enable_audio,
            audio_experience_ratio: config.audio_ratio.clamp(0.0, 1.0),
            enable_multi_modal: config.enable_multi_modal,
        };

        let simulator = ComplexityRangeSimulator::new(complexity_range, self.entropy_controller.clone())?;

        let mut experience_ids = Vec::new();
        let mut modality_counts = std::collections::HashMap::new();

        for i in 0..seed_count {
            // Select modality based on distribution
            let modality = self.select_modality(&config.modality_distribution, i);
            *modality_counts.entry(format!("{:?}", modality)).or_insert(0) += 1;

            // Generate experience
            match simulator.generate_experience(Some(modality)) {
                Ok(experience) => {
                    // Calculate and set entropy
                    let entropy = self.entropy_controller
                        .calculate_experience_entropy(&experience)
                        .ok();

                    // Store experience in brain
                    let experience_id = self.brain.store_experience(
                        experience.event_type.clone(),
                        experience.observation.clone(),
                        experience.action.clone(),
                        experience.outcome.clone(),
                        experience.reward,
                        experience.embedding.clone(),
                    )?;

                    // Update experience with entropy if available
                    if let Some(entropy_val) = entropy {
                        // Update experience metadata with calculated entropy
                        if let Err(e) = self.brain.update_experience_metadata(
                            &experience_id,
                            experience.complexity,
                            Some(entropy_val),
                            experience.modality.clone(),
                        ) {
                            tracing::warn!("Failed to update experience metadata: {}", e);
                        } else {
                            debug!("Seeded experience {} with entropy {:.3}", experience_id, entropy_val);
                        }
                    }

                    experience_ids.push(experience_id);
                }
                Err(e) => {
                    tracing::warn!("Failed to generate experience {}: {}", i, e);
                }
            }
        }

        info!("Seeded {} experiences: {:?}", experience_ids.len(), modality_counts);
        Ok(experience_ids)
    }

    /// Select modality based on distribution
    fn select_modality(&self, distribution: &ModalityDistribution, index: usize) -> ExperienceModality {
        // Use index to deterministically select modality
        let total = distribution.visual + distribution.audio + distribution.voice + 
                    distribution.sound + distribution.multi_modal;
        
        if total == 0.0 {
            return ExperienceModality::Visual; // Default
        }

        let normalized_index = (index as f64 % total) / total;
        let mut cumulative = 0.0;

        cumulative += distribution.visual / total;
        if normalized_index < cumulative {
            return ExperienceModality::Visual;
        }

        cumulative += distribution.audio / total;
        if normalized_index < cumulative {
            return ExperienceModality::Audio;
        }

        cumulative += distribution.voice / total;
        if normalized_index < cumulative {
            return ExperienceModality::Voice;
        }

        cumulative += distribution.sound / total;
        if normalized_index < cumulative {
            return ExperienceModality::Sound;
        }

        ExperienceModality::MultiModal
    }

    /// Seed experiences in batches (for progressive seeding)
    pub async fn seed_batch(&self, batch_size: usize, complexity: f64, modality: Option<ExperienceModality>) -> Result<Vec<String>> {
        let complexity_range = ComplexityRange {
            start_complexity: complexity,
            end_complexity: complexity,
            enable_audio: true,
            audio_experience_ratio: 0.3,
            enable_multi_modal: true,
        };

        let simulator = ComplexityRangeSimulator::new(complexity_range, self.entropy_controller.clone())?;
        let mut experience_ids = Vec::new();

        for _ in 0..batch_size {
            match simulator.generate_experience(modality) {
                Ok(experience) => {
                    let experience_id = self.brain.store_experience(
                        experience.event_type.clone(),
                        experience.observation.clone(),
                        experience.action.clone(),
                        experience.outcome.clone(),
                        experience.reward,
                        experience.embedding.clone(),
                    )?;
                    experience_ids.push(experience_id);
                }
                Err(e) => {
                    tracing::warn!("Failed to generate experience in batch: {}", e);
                }
            }
        }

        Ok(experience_ids)
    }
}

