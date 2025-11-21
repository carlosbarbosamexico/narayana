// Complexity Range Simulator - Generate immersive experiences across complexity ranges
// Supports multi-modal experience generation (visual, audio, voice, sound)
// References:
// - Curriculum Learning: Bengio et al. (2009) ICML
// - Multimodal Learning: Baltrusaitis et al. (2018) IEEE TPAMI

use crate::cognitive::{Experience, Pattern, PatternType};
use crate::entropy_controller::EntropyController;
use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info};
use uuid::Uuid;
use rand::Rng;

/// Experience modality type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExperienceModality {
    Visual,
    Audio,
    Voice,
    Sound,
    MultiModal, // Combined visual+audio
}

/// Complexity range configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityRange {
    /// Start complexity (0.0 to 1.0)
    pub start_complexity: f64,
    /// End complexity (0.0 to 1.0)
    pub end_complexity: f64,
    /// Enable audio experiences
    pub enable_audio: bool,
    /// Ratio of audio experiences (0.0-1.0)
    pub audio_experience_ratio: f64,
    /// Enable multi-modal experiences
    pub enable_multi_modal: bool,
}

impl Default for ComplexityRange {
    fn default() -> Self {
        Self {
            start_complexity: 0.0,
            end_complexity: 1.0,
            enable_audio: true,
            audio_experience_ratio: 0.3,
            enable_multi_modal: true,
        }
    }
}

/// Complexity Range Simulator
pub struct ComplexityRangeSimulator {
    range: ComplexityRange,
    entropy_controller: Arc<EntropyController>,
    current_complexity: Arc<RwLock<f64>>,
    direction: Arc<RwLock<ComplexityDirection>>,
}

/// Direction of complexity progression
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ComplexityDirection {
    Increasing, // Low to high
    Decreasing, // High to low
}

impl ComplexityRangeSimulator {
    /// Create new complexity range simulator
    pub fn new(
        range: ComplexityRange,
        entropy_controller: Arc<EntropyController>,
    ) -> Result<Self> {
        // Validate range
        if range.start_complexity < 0.0 || range.start_complexity > 1.0 {
            return Err(Error::Storage("start_complexity must be in [0.0, 1.0]".to_string()));
        }
        if range.end_complexity < 0.0 || range.end_complexity > 1.0 {
            return Err(Error::Storage("end_complexity must be in [0.0, 1.0]".to_string()));
        }

        let direction = if range.end_complexity > range.start_complexity {
            ComplexityDirection::Increasing
        } else {
            ComplexityDirection::Decreasing
        };

        let current = range.start_complexity;

        info!("ComplexityRangeSimulator initialized: {:.2} -> {:.2} ({:?})",
              range.start_complexity, range.end_complexity, direction);

        Ok(Self {
            range,
            entropy_controller,
            current_complexity: Arc::new(RwLock::new(current)),
            direction: Arc::new(RwLock::new(direction)),
        })
    }

    /// Generate an experience at current complexity level
    pub fn generate_experience(&self, modality: Option<ExperienceModality>) -> Result<Experience> {
        let complexity = *self.current_complexity.read();
        let modality = modality.unwrap_or_else(|| self.select_modality());

        let experience = match modality {
            ExperienceModality::Visual => self.generate_visual_experience(complexity)?,
            ExperienceModality::Audio => self.generate_audio_experience(complexity)?,
            ExperienceModality::Voice => self.generate_voice_experience(complexity)?,
            ExperienceModality::Sound => self.generate_sound_experience(complexity)?,
            ExperienceModality::MultiModal => self.generate_multi_modal_experience(complexity)?,
        };

        debug!("Generated {:?} experience at complexity {:.3}", modality, complexity);
        Ok(experience)
    }

    /// Generate a batch of experiences
    pub fn generate_batch(&self, count: usize) -> Result<Vec<Experience>> {
        // SECURITY: Limit batch size to prevent DoS
        const MAX_BATCH_SIZE: usize = 10000;
        let count = count.min(MAX_BATCH_SIZE);
        
        if count == 0 {
            return Ok(Vec::new());
        }

        let mut experiences = Vec::new();
        // EDGE CASE: Calculate step (count is guaranteed > 0 here)
        let complexity_range = self.range.end_complexity - self.range.start_complexity;
        // EDGE CASE: Check for NaN/Infinity
        if complexity_range.is_nan() || complexity_range.is_infinite() {
            return Err(Error::Storage("Invalid complexity range (NaN or Infinity)".to_string()));
        }
        let step = complexity_range / count as f64;
        // EDGE CASE: Validate step
        if step.is_nan() || step.is_infinite() {
            return Err(Error::Storage("Invalid step calculation (NaN or Infinity)".to_string()));
        }

        for i in 0..count {
            // EDGE CASE: Prevent overflow in i conversion
            let i_f64 = i as f64;
            if i_f64.is_infinite() || i_f64.is_nan() {
                continue; // Skip invalid iteration
            }
            
            let complexity = if *self.direction.read() == ComplexityDirection::Increasing {
                self.range.start_complexity + step * i_f64
            } else {
                self.range.end_complexity - step * i_f64
            };
            
            // EDGE CASE: Validate complexity before setting
            let complexity = if complexity.is_finite() {
                complexity.clamp(0.0, 1.0)
            } else {
                // Fallback to start complexity if calculation fails
                self.range.start_complexity
            };

            *self.current_complexity.write() = complexity;
            let modality = self.select_modality();
            let experience = self.generate_experience(Some(modality))?;
            experiences.push(experience);
        }

        Ok(experiences)
    }

    /// Select modality based on configuration
    fn select_modality(&self) -> ExperienceModality {
        let mut rng = rand::thread_rng();
        let roll = rng.gen::<f64>();

        if self.range.enable_multi_modal && roll < 0.2 {
            return ExperienceModality::MultiModal;
        }

        if self.range.enable_audio {
            if roll < self.range.audio_experience_ratio {
                // Choose between audio, voice, or sound
                let sub_roll = rng.gen::<f64>();
                if sub_roll < 0.33 {
                    return ExperienceModality::Audio;
                } else if sub_roll < 0.66 {
                    return ExperienceModality::Voice;
                } else {
                    return ExperienceModality::Sound;
                }
            }
        }

        ExperienceModality::Visual
    }

    /// Generate visual experience at given complexity
    fn generate_visual_experience(&self, complexity: f64) -> Result<Experience> {
        let observation = self.create_visual_observation(complexity)?;
        let patterns = self.create_patterns(complexity, PatternType::Spatial);
        
        Ok(Experience {
            id: Uuid::new_v4().to_string(),
            event_type: "visual_experience".to_string(),
            observation,
            action: None,
            outcome: None,
            reward: Some(self.complexity_to_reward(complexity)),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            context: self.create_context(complexity, ExperienceModality::Visual),
            patterns,
            embedding: self.create_embedding(complexity),
            complexity: Some(complexity),
            entropy: None, // Will be calculated later
            modality: Some("Visual".to_string()),
        })
    }

    /// Generate audio experience at given complexity
    fn generate_audio_experience(&self, complexity: f64) -> Result<Experience> {
        let observation = self.create_audio_observation(complexity)?;
        let patterns = self.create_patterns(complexity, PatternType::Temporal);
        
        Ok(Experience {
            id: Uuid::new_v4().to_string(),
            event_type: "audio_experience".to_string(),
            observation,
            action: None,
            outcome: None,
            reward: Some(self.complexity_to_reward(complexity)),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            context: self.create_context(complexity, ExperienceModality::Audio),
            patterns,
            embedding: self.create_embedding(complexity),
            complexity: Some(complexity),
            entropy: None,
            modality: Some("Audio".to_string()),
        })
    }

    /// Generate voice experience at given complexity
    fn generate_voice_experience(&self, complexity: f64) -> Result<Experience> {
        let observation = self.create_voice_observation(complexity)?;
        let patterns = self.create_patterns(complexity, PatternType::Sequential);
        
        Ok(Experience {
            id: Uuid::new_v4().to_string(),
            event_type: "voice_experience".to_string(),
            observation,
            action: None,
            outcome: None,
            reward: Some(self.complexity_to_reward(complexity)),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            context: self.create_context(complexity, ExperienceModality::Voice),
            patterns,
            embedding: self.create_embedding(complexity),
            complexity: Some(complexity),
            entropy: None,
            modality: Some("Voice".to_string()),
        })
    }

    /// Generate sound experience at given complexity
    fn generate_sound_experience(&self, complexity: f64) -> Result<Experience> {
        let observation = self.create_sound_observation(complexity)?;
        let patterns = self.create_patterns(complexity, PatternType::Temporal);
        
        Ok(Experience {
            id: Uuid::new_v4().to_string(),
            event_type: "sound_experience".to_string(),
            observation,
            action: None,
            outcome: None,
            reward: Some(self.complexity_to_reward(complexity)),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            context: self.create_context(complexity, ExperienceModality::Sound),
            patterns,
            embedding: self.create_embedding(complexity),
            complexity: Some(complexity),
            entropy: None,
            modality: Some("Sound".to_string()),
        })
    }

    /// Generate multi-modal experience (visual + audio)
    fn generate_multi_modal_experience(&self, complexity: f64) -> Result<Experience> {
        let visual_obs = self.create_visual_observation(complexity)?;
        let audio_obs = self.create_audio_observation(complexity)?;
        
        let observation = serde_json::json!({
            "modality": "multi_modal",
            "visual": visual_obs,
            "audio": audio_obs,
            "complexity": complexity,
        });

        let mut patterns = self.create_patterns(complexity, PatternType::Spatial);
        patterns.extend(self.create_patterns(complexity, PatternType::Temporal));
        
        Ok(Experience {
            id: Uuid::new_v4().to_string(),
            event_type: "multi_modal_experience".to_string(),
            observation,
            action: None,
            outcome: None,
            reward: Some(self.complexity_to_reward(complexity)),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            context: self.create_context(complexity, ExperienceModality::MultiModal),
            patterns,
            embedding: self.create_embedding(complexity),
            complexity: Some(complexity),
            entropy: None,
            modality: Some("MultiModal".to_string()),
        })
    }

    /// Create visual observation based on complexity
    fn create_visual_observation(&self, complexity: f64) -> Result<serde_json::Value> {
        let observation = if complexity < 0.2 {
            // Low complexity: simple shapes
            serde_json::json!({
                "type": "visual",
                "content": "simple_shape",
                "elements": ["circle", "square"],
                "colors": 2,
            })
        } else if complexity < 0.4 {
            // Medium-low: basic scenes
            serde_json::json!({
                "type": "visual",
                "content": "basic_scene",
                "elements": ["object1", "object2", "object3"],
                "colors": 5,
                "spatial_relations": true,
            })
        } else if complexity < 0.6 {
            // Medium: complex scenes
            serde_json::json!({
                "type": "visual",
                "content": "complex_scene",
                "elements": (5.0 + complexity * 10.0) as usize,
                "colors": 10,
                "spatial_relations": true,
                "depth": true,
                "lighting": true,
            })
        } else if complexity < 0.8 {
            // Medium-high: abstract scenes
            serde_json::json!({
                "type": "visual",
                "content": "abstract_scene",
                "elements": (10.0 + complexity * 15.0) as usize,
                "colors": 20,
                "spatial_relations": true,
                "depth": true,
                "lighting": true,
                "textures": true,
                "abstract_patterns": true,
            })
        } else {
            // High: meta-visual
            serde_json::json!({
                "type": "visual",
                "content": "meta_visual",
                "elements": (20.0 + complexity * 20.0) as usize,
                "colors": 30,
                "spatial_relations": true,
                "depth": true,
                "lighting": true,
                "textures": true,
                "abstract_patterns": true,
                "meta_patterns": true,
                "self_referential": true,
            })
        };

        Ok(observation)
    }

    /// Create audio observation based on complexity
    fn create_audio_observation(&self, complexity: f64) -> Result<serde_json::Value> {
        let observation = if complexity < 0.2 {
            // Low: single tones
            serde_json::json!({
                "type": "audio",
                "content": "single_tone",
                "frequency": 440.0,
                "duration": 1.0,
            })
        } else if complexity < 0.4 {
            // Medium-low: simple melodies
            serde_json::json!({
                "type": "audio",
                "content": "simple_melody",
                "frequencies": [440.0, 523.25, 659.25],
                "duration": 3.0,
                "harmony": false,
            })
        } else if complexity < 0.6 {
            // Medium: complex audio patterns
            serde_json::json!({
                "type": "audio",
                "content": "complex_pattern",
                "frequencies": (3.0 + complexity * 10.0) as usize,
                "duration": 5.0,
                "harmony": true,
                "layers": 2,
            })
        } else if complexity < 0.8 {
            // Medium-high: abstract audio
            serde_json::json!({
                "type": "audio",
                "content": "abstract_composition",
                "frequencies": (10.0 + complexity * 15.0) as usize,
                "duration": 10.0,
                "harmony": true,
                "layers": 3,
                "effects": true,
                "modulation": true,
            })
        } else {
            // High: meta-audio
            serde_json::json!({
                "type": "audio",
                "content": "meta_audio",
                "frequencies": (20.0 + complexity * 20.0) as usize,
                "duration": 15.0,
                "harmony": true,
                "layers": 5,
                "effects": true,
                "modulation": true,
                "self_referential": true,
                "meta_patterns": true,
            })
        };

        Ok(observation)
    }

    /// Create voice observation based on complexity
    fn create_voice_observation(&self, complexity: f64) -> Result<serde_json::Value> {
        let observation = if complexity < 0.2 {
            // Low: single words
            serde_json::json!({
                "type": "voice",
                "content": "single_word",
                "text": "hello",
                "phonemes": 2,
            })
        } else if complexity < 0.4 {
            // Medium-low: short sentences
            serde_json::json!({
                "type": "voice",
                "content": "short_sentence",
                "text": "Hello world",
                "words": 2,
                "phonemes": 5,
            })
        } else if complexity < 0.6 {
            // Medium: conversations
            serde_json::json!({
                "type": "voice",
                "content": "conversation",
                "text": "Hello, how are you today?",
                "words": 5,
                "phonemes": 15,
                "prosody": true,
            })
        } else if complexity < 0.8 {
            // Medium-high: abstract concepts
            serde_json::json!({
                "type": "voice",
                "content": "abstract_discourse",
                "text": "The nature of consciousness involves complex interactions",
                "words": 8,
                "phonemes": 25,
                "prosody": true,
                "metaphors": true,
            })
        } else {
            // High: meta-linguistic
            serde_json::json!({
                "type": "voice",
                "content": "meta_linguistic",
                "text": "Language about language creates recursive meaning structures",
                "words": 10,
                "phonemes": 35,
                "prosody": true,
                "metaphors": true,
                "self_referential": true,
            })
        };

        Ok(observation)
    }

    /// Create sound observation based on complexity
    fn create_sound_observation(&self, complexity: f64) -> Result<serde_json::Value> {
        let observation = if complexity < 0.2 {
            // Low: basic sounds
            serde_json::json!({
                "type": "sound",
                "content": "basic_sound",
                "source": "environment",
                "sources": 1,
            })
        } else if complexity < 0.4 {
            // Medium-low: multiple sources
            serde_json::json!({
                "type": "sound",
                "content": "soundscape",
                "source": "environment",
                "sources": 3,
                "spatial": true,
            })
        } else if complexity < 0.6 {
            // Medium: rich soundscapes
            serde_json::json!({
                "type": "sound",
                "content": "rich_soundscape",
                "source": "environment",
                "sources": (5.0 + complexity * 5.0) as usize,
                "spatial": true,
                "temporal": true,
            })
        } else if complexity < 0.8 {
            // Medium-high: complex interactions
            serde_json::json!({
                "type": "sound",
                "content": "complex_interaction",
                "source": "environment",
                "sources": (10.0 + complexity * 10.0) as usize,
                "spatial": true,
                "temporal": true,
                "acoustic_ecology": true,
            })
        } else {
            // High: meta-acoustic
            serde_json::json!({
                "type": "sound",
                "content": "meta_acoustic",
                "source": "environment",
                "sources": (20.0 + complexity * 15.0) as usize,
                "spatial": true,
                "temporal": true,
                "acoustic_ecology": true,
                "self_referential": true,
            })
        };

        Ok(observation)
    }

    /// Create patterns based on complexity
    fn create_patterns(&self, complexity: f64, pattern_type: PatternType) -> Vec<Pattern> {
        let pattern_count = (complexity * 5.0) as usize;
        let mut patterns = Vec::new();

        for i in 0..pattern_count {
            patterns.push(Pattern {
                id: Uuid::new_v4().to_string(),
                pattern_type: pattern_type.clone(),
                conditions: serde_json::json!({"complexity": complexity}),
                action: serde_json::json!({"type": "pattern_action", "index": i}),
                outcome: serde_json::json!({"success": true, "complexity": complexity}),
                confidence: 0.5 + complexity * 0.5,
                frequency: 1,
                last_seen: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            });
        }

        patterns
    }

    /// Create context based on complexity and modality
    fn create_context(&self, complexity: f64, modality: ExperienceModality) -> HashMap<String, serde_json::Value> {
        let mut context = HashMap::new();
        context.insert("complexity".to_string(), serde_json::json!(complexity));
        context.insert("modality".to_string(), serde_json::json!(format!("{:?}", modality)));
        context.insert("timestamp".to_string(), serde_json::json!(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        ));
        context
    }

    /// Create embedding based on complexity
    fn create_embedding(&self, complexity: f64) -> Option<Vec<f32>> {
        // EDGE CASE: Validate complexity
        if !complexity.is_finite() {
            return None;
        }
        let complexity = complexity.clamp(0.0, 1.0);
        
        // Create a simple embedding vector based on complexity
        const DIM: usize = 128;
        let mut embedding = Vec::with_capacity(DIM);
        let mut rng = rand::thread_rng();

        for _ in 0..DIM {
            // Embedding values correlate with complexity
            let base = complexity as f32;
            let noise = rng.gen::<f32>() * 0.1;
            let value = base + noise;
            // EDGE CASE: Ensure value is finite
            if value.is_finite() {
                embedding.push(value.clamp(0.0, 1.0));
            } else {
                embedding.push(0.0);
            }
        }

        Some(embedding)
    }

    /// Convert complexity to reward
    fn complexity_to_reward(&self, complexity: f64) -> f64 {
        // Reward increases with complexity, but with diminishing returns
        complexity * 0.8 + 0.2
    }

    /// Get current complexity
    pub fn current_complexity(&self) -> f64 {
        *self.current_complexity.read()
    }

    /// Set current complexity
    pub fn set_complexity(&self, complexity: f64) -> Result<()> {
        // SECURITY: Validate complexity
        if complexity.is_nan() || complexity.is_infinite() {
            return Err(Error::Storage("Invalid complexity (NaN or Infinity)".to_string()));
        }
        let complexity = complexity.clamp(0.0, 1.0);
        *self.current_complexity.write() = complexity;
        Ok(())
    }

    /// Advance complexity (for forward progression)
    pub fn advance_complexity(&self, step: f64) -> Result<()> {
        let current = *self.current_complexity.read();
        let new = if *self.direction.read() == ComplexityDirection::Increasing {
            (current + step).min(self.range.end_complexity)
        } else {
            (current - step).max(self.range.end_complexity)
        };
        self.set_complexity(new)
    }
}

