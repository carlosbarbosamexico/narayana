// Traits Equations System
// Calculates trait values from genetic and environmental factors
// Supports trait interactions and dynamic equation evaluation

use crate::genetics::{GeneticSystem, Genome};
use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info};

/// Cognitive trait types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TraitType {
    AttentionSpan,
    MemoryCapacity,
    Curiosity,
    Creativity,
    SocialAffinity,
    RiskTaking,
    Patience,
    LearningRate,
    MoralReceptivity,
    Conscientiousness,
}

impl TraitType {
    /// Get all trait types
    pub fn all() -> Vec<Self> {
        vec![
            TraitType::AttentionSpan,
            TraitType::MemoryCapacity,
            TraitType::Curiosity,
            TraitType::Creativity,
            TraitType::SocialAffinity,
            TraitType::RiskTaking,
            TraitType::Patience,
            TraitType::LearningRate,
            TraitType::MoralReceptivity,
            TraitType::Conscientiousness,
        ]
    }
    
    /// Get trait name as string
    pub fn as_str(&self) -> &str {
        match self {
            TraitType::AttentionSpan => "attention_span",
            TraitType::MemoryCapacity => "memory_capacity",
            TraitType::Curiosity => "curiosity",
            TraitType::Creativity => "creativity",
            TraitType::SocialAffinity => "social_affinity",
            TraitType::RiskTaking => "risk_taking",
            TraitType::Patience => "patience",
            TraitType::LearningRate => "learning_rate",
            TraitType::MoralReceptivity => "moral_receptivity",
            TraitType::Conscientiousness => "conscientiousness",
        }
    }
}

/// Trait value - computed from genes + environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trait {
    pub trait_type: TraitType,
    pub value: f64, // 0.0-1.0
    pub genetic_component: f64,
    pub environmental_component: f64,
    pub last_updated: u64,
}

/// Environmental factor - influences traits through experience
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentalFactor {
    pub factor_type: String,
    pub value: f64, // 0.0-1.0
    pub timestamp: u64,
    pub decay_rate: f64, // How quickly this factor decays
}

/// Trait interaction matrix - how traits influence each other
type TraitInteractionMatrix = HashMap<(TraitType, TraitType), f64>;

/// Trait Calculator - computes traits from genes + environment
pub struct TraitCalculator {
    genetic_system: Arc<GeneticSystem>,
    environmental_factors: Arc<RwLock<HashMap<String, EnvironmentalFactor>>>,
    trait_interactions: Arc<RwLock<TraitInteractionMatrix>>,
    genetic_weight: f64, // Weight for genetic component (0.0-1.0)
    environmental_weight: f64, // Weight for environmental component
    cached_traits: Arc<RwLock<HashMap<TraitType, Trait>>>,
}

impl TraitCalculator {
    /// Create new trait calculator
    pub fn new(genetic_system: Arc<GeneticSystem>, environmental_weight: f64) -> Self {
        // SECURITY: Validate and clamp environmental weight
        let environmental_weight = environmental_weight.max(0.0).min(1.0);
        let genetic_weight = 1.0 - environmental_weight;
        
        // Initialize trait interactions (default: no interactions)
        let mut interactions = HashMap::new();
        let trait_types = TraitType::all();
        for trait1 in trait_types.iter() {
            for trait2 in trait_types.iter() {
                if trait1 != trait2 {
                    // Default: small positive interaction
                    interactions.insert((trait1.clone(), trait2.clone()), 0.05);
                }
            }
        }
        
        // Some specific interactions
        interactions.insert((TraitType::Curiosity, TraitType::LearningRate), 0.2);
        interactions.insert((TraitType::LearningRate, TraitType::Curiosity), 0.15);
        interactions.insert((TraitType::AttentionSpan, TraitType::MemoryCapacity), 0.1);
        interactions.insert((TraitType::MemoryCapacity, TraitType::AttentionSpan), 0.1);
        interactions.insert((TraitType::Creativity, TraitType::RiskTaking), 0.15);
        interactions.insert((TraitType::RiskTaking, TraitType::Creativity), 0.1);
        // Moral trait interactions
        interactions.insert((TraitType::SocialAffinity, TraitType::MoralReceptivity), 0.2);
        interactions.insert((TraitType::MoralReceptivity, TraitType::SocialAffinity), 0.15);
        interactions.insert((TraitType::Conscientiousness, TraitType::MoralReceptivity), 0.25);
        interactions.insert((TraitType::MoralReceptivity, TraitType::Conscientiousness), 0.2);
        interactions.insert((TraitType::RiskTaking, TraitType::MoralReceptivity), -0.1); // Inverse relationship
        
        Self {
            genetic_system,
            environmental_factors: Arc::new(RwLock::new(HashMap::new())),
            trait_interactions: Arc::new(RwLock::new(interactions)),
            genetic_weight: genetic_weight.max(0.0).min(1.0),
            environmental_weight: environmental_weight.max(0.0).min(1.0),
            cached_traits: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Calculate trait value from genes + environment
    pub fn calculate_trait(&self, trait_type: &TraitType) -> Result<Trait> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Get genetic component
        let genetic_value = self.genetic_system.get_trait_genetic_value(trait_type.as_str());
        
        // SECURITY: Validate genetic value
        let genetic_value = if genetic_value.is_finite() && genetic_value >= 0.0 && genetic_value <= 1.0 {
            genetic_value
        } else {
            0.5 // Default neutral value
        };
        
        // Get environmental component
        let environmental_value = self.get_environmental_factor(trait_type, now);
        
        // SECURITY: Validate environmental value
        let environmental_value = if environmental_value.is_finite() && environmental_value >= 0.0 && environmental_value <= 1.0 {
            environmental_value
        } else {
            0.5 // Default neutral value
        };
        
        // Base trait calculation: weighted sum
        let base_value = (genetic_value * self.genetic_weight) + (environmental_value * self.environmental_weight);
        
        // SECURITY: Validate base value
        let base_value = if base_value.is_finite() {
            base_value
        } else {
            0.5 // Default if invalid
        };
        
        // Apply trait interactions
        let final_value = self.apply_trait_interactions(trait_type, base_value)?;
        
        // Clamp to [0.0, 1.0]
        let final_value = final_value.max(0.0).min(1.0);
        
        let trait_val = Trait {
            trait_type: trait_type.clone(),
            value: final_value,
            genetic_component: genetic_value,
            environmental_component: environmental_value,
            last_updated: now,
        };
        
        // Cache result
        self.cached_traits.write().insert(trait_type.clone(), trait_val.clone());
        
        Ok(trait_val)
    }
    
    /// Get environmental factor for a trait
    fn get_environmental_factor(&self, trait_type: &TraitType, now: u64) -> f64 {
        let factors = self.environmental_factors.read();
        
        // SECURITY: Limit processing to prevent DoS
        const MAX_FACTORS: usize = 10000;
        let factor_count = factors.len().min(MAX_FACTORS);
        
        // Look for matching factors
        let mut total = 0.0;
        let mut count = 0;
        let mut processed = 0;
        
        for factor in factors.values() {
            if processed >= factor_count {
                break;
            }
            processed += 1;
            
            // Check if factor is relevant to this trait
            if factor.factor_type == trait_type.as_str() {
                // SECURITY: Validate factor values
                if !factor.value.is_finite() || factor.value < 0.0 || factor.value > 1.0 {
                    continue;
                }
                if !factor.decay_rate.is_finite() || factor.decay_rate < 0.0 || factor.decay_rate > 1.0 {
                    continue;
                }
                
                // Apply decay
                let age = now.saturating_sub(factor.timestamp);
                // SECURITY: Prevent overflow in age calculation
                let age_hours = if age > u64::MAX / 3600 {
                    u64::MAX as f64 / 3600.0
                } else {
                    age as f64 / 3600.0
                };
                
                // SECURITY: Validate decay calculation
                let decay_base = (1.0 - factor.decay_rate).max(0.0).min(1.0);
                let decayed_value = factor.value * decay_base.powf(age_hours.min(1000.0)); // Cap age_hours
                
                // SECURITY: Validate decayed value
                if decayed_value.is_finite() && decayed_value >= 0.0 {
                    total += decayed_value.min(1.0);
                    count += 1;
                }
            }
        }
        
        // SECURITY: Prevent division by zero
        if count > 0 {
            let result = total / count as f64;
            if result.is_finite() {
                result.max(0.0).min(1.0)
            } else {
                0.5 // Default if invalid
            }
        } else {
            0.5 // Default neutral value
        }
    }
    
    /// Apply trait interactions
    fn apply_trait_interactions(&self, trait_type: &TraitType, base_value: f64) -> Result<f64> {
        // SECURITY: Validate base value
        if !base_value.is_finite() {
            return Ok(0.5); // Default if invalid
        }
        
        let interactions = self.trait_interactions.read();
        let cached = self.cached_traits.read();
        
        let mut adjusted_value = base_value;
        let mut interaction_count = 0;
        const MAX_INTERACTIONS: usize = 100; // Prevent excessive calculations
        
        // Sum interactions from other traits
        for (other_trait_type, trait_val) in cached.iter() {
            if interaction_count >= MAX_INTERACTIONS {
                break;
            }
            
            if other_trait_type != trait_type {
                // SECURITY: Validate trait value
                if !trait_val.value.is_finite() || trait_val.value < 0.0 || trait_val.value > 1.0 {
                    continue;
                }
                
                if let Some(interaction_strength) = interactions.get(&(other_trait_type.clone(), trait_type.clone())) {
                    // SECURITY: Validate interaction strength
                    let valid_strength = if interaction_strength.is_finite() {
                        interaction_strength.max(-1.0).min(1.0)
                    } else {
                        continue;
                    };
                    
                    // Interaction: other trait influences this trait
                    let interaction_contribution = trait_val.value * valid_strength;
                    if interaction_contribution.is_finite() {
                        adjusted_value += interaction_contribution;
                        interaction_count += 1;
                    }
                }
            }
        }
        
        // SECURITY: Validate final value
        if adjusted_value.is_finite() {
            Ok(adjusted_value)
        } else {
            Ok(base_value) // Return base if interactions caused invalid value
        }
    }
    
    /// Update environmental factor from experience
    pub fn update_environmental_factor(
        &self,
        factor_type: &str,
        value: f64,
        decay_rate: f64,
    ) -> Result<()> {
        // SECURITY: Validate inputs
        if !value.is_finite() || !decay_rate.is_finite() {
            return Err(Error::Storage("Invalid environmental factor values (NaN/Inf)".to_string()));
        }
        
        // SECURITY: Limit factor_type length to prevent DoS
        const MAX_FACTOR_TYPE_LEN: usize = 256;
        if factor_type.len() > MAX_FACTOR_TYPE_LEN {
            return Err(Error::Storage("Factor type name too long".to_string()));
        }
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let factor = EnvironmentalFactor {
            factor_type: factor_type.to_string(),
            value: value.max(0.0).min(1.0),
            timestamp: now,
            decay_rate: decay_rate.max(0.0).min(1.0),
        };
        
        let mut factors = self.environmental_factors.write();
        
        // SECURITY: Limit number of environmental factors to prevent DoS
        const MAX_FACTORS: usize = 100000;
        if factors.len() >= MAX_FACTORS {
            // Remove oldest factors (simple FIFO)
            let keys: Vec<String> = factors.keys().cloned().take(1000).collect();
            for key in keys {
                factors.remove(&key);
            }
        }
        
        factors.insert(factor_type.to_string(), factor);
        drop(factors);
        
        // Invalidate cache for this trait
        if let Ok(trait_type) = self.trait_type_from_string(factor_type) {
            self.cached_traits.write().remove(&trait_type);
        }
        
        Ok(())
    }
    
    /// Get all calculated traits
    pub fn get_all_traits(&self) -> Result<HashMap<TraitType, Trait>> {
        let mut traits = HashMap::new();
        
        for trait_type in TraitType::all() {
            let trait_val = self.calculate_trait(&trait_type)?;
            traits.insert(trait_type, trait_val);
        }
        
        Ok(traits)
    }
    
    /// Get trait value (cached if available)
    pub fn get_trait(&self, trait_type: &TraitType) -> Result<f64> {
        // Check cache first
        if let Some(cached) = self.cached_traits.read().get(trait_type) {
            // Check if cache is still valid (within last minute)
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            if now.saturating_sub(cached.last_updated) < 60 {
                return Ok(cached.value);
            }
        }
        
        // Recalculate
        let trait_val = self.calculate_trait(trait_type)?;
        Ok(trait_val.value)
    }
    
    /// Set trait interaction strength
    pub fn set_trait_interaction(
        &self,
        trait1: TraitType,
        trait2: TraitType,
        strength: f64,
    ) {
        self.trait_interactions.write().insert(
            (trait1, trait2),
            strength.max(-1.0).min(1.0),
        );
    }
    
    /// Helper: convert string to trait type
    fn trait_type_from_string(&self, s: &str) -> Result<TraitType> {
        match s {
            "attention_span" => Ok(TraitType::AttentionSpan),
            "memory_capacity" => Ok(TraitType::MemoryCapacity),
            "curiosity" => Ok(TraitType::Curiosity),
            "creativity" => Ok(TraitType::Creativity),
            "social_affinity" => Ok(TraitType::SocialAffinity),
            "risk_taking" => Ok(TraitType::RiskTaking),
            "patience" => Ok(TraitType::Patience),
            "learning_rate" => Ok(TraitType::LearningRate),
            "moral_receptivity" => Ok(TraitType::MoralReceptivity),
            "conscientiousness" => Ok(TraitType::Conscientiousness),
            _ => Err(Error::Storage(format!("Unknown trait type: {}", s))),
        }
    }
    
    /// Recalculate all traits (force refresh)
    pub fn recalculate_all(&self) -> Result<()> {
        self.cached_traits.write().clear();
        self.get_all_traits()?;
        Ok(())
    }
}

