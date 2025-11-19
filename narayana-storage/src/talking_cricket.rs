// Talking Cricket - Moral Guide System
// In honor of MRM::Carlo Colhodi
// Optional, pluggable moral guide for CPL that provides moral assessments,
// filters actions, influences decisions, and evolves principles over time

use crate::cognitive::{CognitiveBrain, Memory, Experience, Thought, MemoryType};
use crate::genetics::GeneticSystem;
use crate::traits_equations::{TraitCalculator, TraitType};
use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn, error};
use uuid::Uuid;
use serde_json::Value as JsonValue;

/// Trait for LLM manager (to allow optional LLM dependency)
#[async_trait::async_trait]
pub trait LLMManagerTrait: Send + Sync {
    async fn chat(&self, messages: Vec<LLMMessage>, provider: Option<()>) -> Result<String>;
}

/// LLM Message type
#[derive(Debug, Clone)]
pub struct LLMMessage {
    pub role: String,
    pub content: String,
}

// WorldAction is defined in narayana-wld, but we need a trait or type alias
// For now, we'll use a generic approach - the actual WorldAction will be passed from narayana-wld
// This is a placeholder - in practice, Talking Cricket will work with narayana-wld::WorldAction

/// Configuration for Talking Cricket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TalkingCricketConfig {
    /// Enable LLM for principle evolution
    pub llm_enabled: bool,
    /// Threshold below which actions are vetoed (0.0-1.0)
    pub veto_threshold: f64,
    /// Iterations between principle evolution cycles
    pub evolution_frequency: u64,
    /// Database table name for principles
    pub principles_table: String,
}

impl Default for TalkingCricketConfig {
    fn default() -> Self {
        Self {
            llm_enabled: false,
            veto_threshold: 0.3,
            evolution_frequency: 1000,
            principles_table: "talking_cricket_principles".to_string(),
        }
    }
}

/// Rule type for moral principles
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleType {
    Scoring,    // Provides a score (0.0-1.0)
    Threshold,  // Sets a threshold for action acceptance
    Veto,       // Can veto actions entirely
}

/// Moral principle - dynamic rule stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoralPrinciple {
    pub id: String,
    pub name: String,
    pub rule_type: RuleType,
    /// JSON-serialized scoring function or rule description
    pub scoring_function: String,
    /// Optional threshold value
    pub threshold: Option<f64>,
    /// Context for when this principle applies
    pub context: HashMap<String, JsonValue>,
    pub created_at: u64,
    pub usage_count: u64,
    /// Effectiveness score (0.0-1.0) - how well this principle works
    pub effectiveness_score: f64,
}

/// Moral assessment result for an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoralAssessment {
    pub action_id: String,
    /// Moral score: 0.0 (immoral) to 1.0 (highly moral)
    pub moral_score: f64,
    /// Confidence in assessment (0.0-1.0)
    pub confidence: f64,
    /// Human-readable reasoning
    pub reasoning: String,
    /// IDs of principles used in assessment
    pub principle_ids: Vec<String>,
    /// Whether this action should be vetoed
    pub should_veto: bool,
    /// Influence weight for adjusting action probability (0.0-1.0)
    pub influence_weight: f64,
}

/// Context for moral assessment - includes CPL cognitive state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessmentContext {
    pub cpl_id: Option<String>,
    pub current_traits: HashMap<String, f64>,
    pub recent_actions: Vec<String>,
    pub environmental_factors: HashMap<String, JsonValue>,
    /// Relevant memories for moral assessment
    pub relevant_memories: Vec<MemorySummary>,
    /// Recent experiences that inform moral judgment
    pub recent_experiences: Vec<ExperienceSummary>,
    /// Current thoughts that may influence moral assessment
    pub active_thoughts: Vec<ThoughtSummary>,
    /// Working memory state
    pub working_memory_state: Vec<JsonValue>,
}

/// Summary of a memory for context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySummary {
    pub id: String,
    pub memory_type: String,
    pub content_summary: String,
    pub strength: f64,
    pub tags: Vec<String>,
    pub created_at: u64,
}

/// Summary of an experience for context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceSummary {
    pub id: String,
    pub event_type: String,
    pub observation_summary: String,
    pub action_summary: Option<String>,
    pub outcome_summary: Option<String>,
    pub reward: Option<f64>,
    pub timestamp: u64,
}

/// Summary of a thought for context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtSummary {
    pub id: String,
    pub content_summary: String,
    pub priority: f64,
    pub state: String,
}

/// Talking Cricket - Main moral guide orchestrator
pub struct TalkingCricket {
    brain: Arc<CognitiveBrain>,
    llm_manager: Option<Arc<dyn LLMManagerTrait + Send + Sync>>,
    principles: Arc<RwLock<HashMap<String, MoralPrinciple>>>,
    trait_calculator: Arc<RwLock<Option<Arc<TraitCalculator>>>>,
    genetic_system: Arc<RwLock<Option<Arc<GeneticSystem>>>>,
    config: TalkingCricketConfig,
    is_attached: Arc<RwLock<bool>>,
    assessment_cache: Arc<RwLock<HashMap<String, (MoralAssessment, u64)>>>, // action_hash -> (assessment, timestamp)
    evolution_count: Arc<RwLock<u64>>,
}

impl TalkingCricket {
    /// Create new Talking Cricket instance
    pub fn new(
        brain: Arc<CognitiveBrain>,
        config: TalkingCricketConfig,
    ) -> Self {
        Self {
            brain,
            llm_manager: None,
            principles: Arc::new(RwLock::new(HashMap::new())),
            trait_calculator: Arc::new(RwLock::new(None)),
            genetic_system: Arc::new(RwLock::new(None)),
            config,
            is_attached: Arc::new(RwLock::new(false)),
            assessment_cache: Arc::new(RwLock::new(HashMap::new())),
            evolution_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Create with LLM manager
    pub fn with_llm(
        brain: Arc<CognitiveBrain>,
        llm_manager: Arc<dyn LLMManagerTrait + Send + Sync>,
        config: TalkingCricketConfig,
    ) -> Self {
        let mut tc = Self::new(brain, config);
        tc.llm_manager = Some(llm_manager);
        tc
    }

    /// Set trait calculator (for calculating moral influence)
    pub fn set_trait_calculator(&self, calculator: Arc<TraitCalculator>) {
        *self.trait_calculator.write() = Some(calculator);
    }

    /// Set genetic system (for calculating moral influence)
    pub fn set_genetic_system(&self, genetic_system: Arc<GeneticSystem>) {
        *self.genetic_system.write() = Some(genetic_system);
    }

    /// Attach to CPL
    pub fn attach_to_cpl(&self) -> Result<()> {
        *self.is_attached.write() = true;
        info!("Talking Cricket attached to CPL");
        Ok(())
    }

    /// Detach from CPL
    pub fn detach_from_cpl(&self) -> Result<()> {
        *self.is_attached.write() = false;
        info!("Talking Cricket detached from CPL");
        Ok(())
    }

    /// Check if attached
    pub fn is_attached(&self) -> bool {
        *self.is_attached.read()
    }

    /// Build assessment context from CPL state (memories, experiences, thoughts)
    pub async fn build_cpl_context(&self, cpl_id: Option<&str>) -> Result<AssessmentContext> {
        // Get current traits
        let mut current_traits = HashMap::new();
        if let Some(calc) = self.trait_calculator.read().as_ref() {
            for trait_type in crate::traits_equations::TraitType::all() {
                if let Ok(value) = calc.get_trait(&trait_type) {
                    current_traits.insert(trait_type.as_str().to_string(), value);
                }
            }
        }

        // Get relevant memories (recent and high-strength)
        let relevant_memories = self.get_relevant_memories(10).await?;

        // Get recent experiences
        let recent_experiences = self.get_recent_experiences_for_context(10).await?;

        // Get active thoughts
        let active_thoughts = self.get_active_thoughts(5).await?;

        // Get working memory
        let working_memory_state = self.get_working_memory_state().await?;

        Ok(AssessmentContext {
            cpl_id: cpl_id.map(|s| s.to_string()),
            current_traits,
            recent_actions: Vec::new(), // Could be populated from action history
            environmental_factors: HashMap::new(), // Could be populated from brain
            relevant_memories,
            recent_experiences,
            active_thoughts,
            working_memory_state,
        })
    }

    /// Assess an action morally with full CPL context
    /// Note: action should be serializable to JSON for hashing
    pub async fn assess_action<T: serde::Serialize>(
        &self,
        action: &T,
        context: Option<&AssessmentContext>,
    ) -> Result<MoralAssessment> {
        // Build context if not provided
        let context = if let Some(ctx) = context {
            ctx.clone()
        } else {
            self.build_cpl_context(None).await?
        };

        // Generate action hash for caching (include context hash for cache key)
        let context_hash = self.hash_context(&context);
        let action_hash = format!("{}_{}", self.hash_action_serializable(action), context_hash);
        
        // Check cache (valid for 60 seconds)
        {
            let cache = self.assessment_cache.read();
            if let Some((assessment, timestamp)) = cache.get(&action_hash) {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                if now.saturating_sub(*timestamp) < 60 {
                    return Ok(assessment.clone());
                }
            }
        }

        // Get applicable principles (now with full CPL context)
        let principles = self.get_applicable_principles(action, &context)?;
        
        // Calculate moral score from principles (using CPL context)
        let mut total_score = 0.0;
        let mut total_weight = 0.0;
        let mut principle_ids = Vec::new();
        let mut reasoning_parts = Vec::new();

        for principle in &principles {
            let score = self.evaluate_principle(principle, action, &context)?;
            let weight = principle.effectiveness_score.max(0.1); // Minimum weight
            
            total_score += score * weight;
            total_weight += weight;
            principle_ids.push(principle.id.clone());
            reasoning_parts.push(format!("{}: {:.2}", principle.name, score));
        }

        // Normalize score
        let moral_score = if total_weight > 0.0 {
            (total_score / total_weight).max(0.0).min(1.0)
        } else {
            0.5 // Default neutral if no principles
        };

        // Calculate confidence based on number of applicable principles
        let confidence = (principles.len() as f64 / 10.0).min(1.0).max(0.1);

        // Determine if should veto
        let should_veto = moral_score < self.config.veto_threshold;

        // Calculate influence weight based on moral influence
        let moral_influence = self.calculate_moral_influence()?;
        let influence_weight = moral_influence * (1.0 - (moral_score - 0.5).abs() * 2.0); // Higher influence for neutral scores

        // Include CPL context in reasoning
        let context_info = if !context.relevant_memories.is_empty() || !context.recent_experiences.is_empty() {
            format!(" (considering {} memories, {} experiences)", 
                context.relevant_memories.len(), 
                context.recent_experiences.len())
        } else {
            String::new()
        };
        
        let reasoning = if reasoning_parts.is_empty() {
            format!("No applicable principles found{}", context_info)
        } else {
            format!("Assessed using {} principles: {}{}", 
                principles.len(), 
                reasoning_parts.join(", "),
                context_info)
        };

        let action_id = Uuid::new_v4().to_string();
        let assessment = MoralAssessment {
            action_id,
            moral_score,
            confidence,
            reasoning,
            principle_ids,
            should_veto,
            influence_weight: influence_weight.max(0.0).min(1.0),
        };

        // Cache result
        {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            self.assessment_cache.write().insert(action_hash, (assessment.clone(), now));
        }

        Ok(assessment)
    }

    /// Calculate moral influence based on traits and genetics
    /// Equation: MoralInfluence = f(traits) × g(gene)
    pub fn calculate_moral_influence(&self) -> Result<f64> {
        let trait_calc = self.trait_calculator.read()
            .as_ref()
            .cloned()
            .ok_or_else(|| Error::Storage("Trait calculator not set".to_string()))?;
        
        let genetic_sys = self.genetic_system.read()
            .as_ref()
            .cloned()
            .ok_or_else(|| Error::Storage("Genetic system not set".to_string()))?;

        // f(traits): Combine multiple traits
        let moral_receptivity = trait_calc.get_trait(&TraitType::MoralReceptivity)
            .unwrap_or(0.5);
        let social_affinity = trait_calc.get_trait(&TraitType::SocialAffinity)
            .unwrap_or(0.5);
        let conscientiousness = trait_calc.get_trait(&TraitType::Conscientiousness)
            .unwrap_or(0.5);
        let risk_taking = trait_calc.get_trait(&TraitType::RiskTaking)
            .unwrap_or(0.5);

        // Combine traits: MoralReceptivity (primary) + SocialAffinity + Conscientiousness - RiskTaking (inverse)
        let trait_component = (moral_receptivity * 0.5) 
            + (social_affinity * 0.2)
            + (conscientiousness * 0.2)
            - (risk_taking * 0.1); // Inverse relationship

        // g(gene): Get moral_sensitivity gene value
        let gene_value = genetic_sys.get_trait_genetic_value("moral_receptivity");

        // Final influence: f(traits) × g(gene)
        let influence = trait_component.max(0.0).min(1.0) * gene_value.max(0.0).min(1.0);
        
        Ok(influence.max(0.0).min(1.0))
    }

    /// Evolve principles using LLM and CPL experiences
    pub async fn evolve_principles(&self) -> Result<()> {
        if !self.config.llm_enabled {
            return Ok(()); // Skip if LLM not enabled
        }

        let llm = self.llm_manager.as_ref()
            .ok_or_else(|| Error::Storage("LLM manager not available".to_string()))?;

        // Get recent experiences from brain
        let recent_experiences = self.get_recent_experiences().await?;

        if recent_experiences.is_empty() {
            return Ok(()); // No experiences to learn from
        }

        // Generate prompt for LLM
        let prompt = format!(
            "Based on these experiences, suggest moral principles or refine existing ones:\n\n{}\n\n\
            Provide principles in JSON format with: name, rule_type (Scoring/Threshold/Veto), \
            scoring_function description, threshold (if applicable), and context.",
            recent_experiences.iter()
                .take(10)
                .map(|e| format!("- {}", e))
                .collect::<Vec<_>>()
                .join("\n")
        );

        // Call LLM
        let response = llm.chat(
            vec![LLMMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            None,
        ).await.map_err(|e| Error::Storage(format!("LLM error: {}", e)))?;

        // Parse and create new principles (simplified - would need proper JSON parsing)
        // For now, just log the response
        info!("LLM generated principle evolution: {}", response);

        // Update evolution count
        *self.evolution_count.write() += 1;

        Ok(())
    }

    /// Load principles from database
    pub async fn load_principles_from_db(&self) -> Result<()> {
        // TODO: Implement database loading
        // For now, create some default principles
        let default_principles = vec![
            MoralPrinciple {
                id: Uuid::new_v4().to_string(),
                name: "Harm Prevention".to_string(),
                rule_type: RuleType::Veto,
                scoring_function: "Veto actions that cause harm to others".to_string(),
                threshold: Some(0.3),
                context: HashMap::new(),
                created_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                usage_count: 0,
                effectiveness_score: 0.8,
            },
            MoralPrinciple {
                id: Uuid::new_v4().to_string(),
                name: "Fairness".to_string(),
                rule_type: RuleType::Scoring,
                scoring_function: "Score based on fairness to all parties".to_string(),
                threshold: None,
                context: HashMap::new(),
                created_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                usage_count: 0,
                effectiveness_score: 0.7,
            },
        ];

        let mut principles = self.principles.write();
        for principle in default_principles {
            principles.insert(principle.id.clone(), principle);
        }

        info!("Loaded {} principles", principles.len());
        Ok(())
    }

    /// Save principles to database
    pub async fn save_principles_to_db(&self) -> Result<()> {
        // TODO: Implement database saving
        let principles = self.principles.read();
        info!("Would save {} principles to database", principles.len());
        Ok(())
    }

    // Private helper methods

    fn hash_action_serializable<T: serde::Serialize>(&self, action: &T) -> String {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        
        // Serialize to JSON string for hashing
        let json_str = serde_json::to_string(action).unwrap_or_else(|_| "{}".to_string());
        let mut hasher = DefaultHasher::new();
        json_str.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    fn get_applicable_principles<T: serde::Serialize>(
        &self,
        _action: &T,
        context: &AssessmentContext,
    ) -> Result<Vec<MoralPrinciple>> {
        // Filter principles based on CPL context
        // Use memories, experiences, and thoughts to determine which principles apply
        let principles = self.principles.read();
        let mut applicable = Vec::new();
        
        for principle in principles.values() {
            // Check if principle context matches current CPL state
            let mut applies = true;
            
            // Filter by principle context if specified
            if !principle.context.is_empty() {
                // Check if principle context matches current traits, memories, etc.
                // For now, apply all principles but could filter based on context
            }
            
            // Consider memories - if principle is about harm and we have harm-related memories
            if principle.name.to_lowercase().contains("harm") {
                let has_harm_memories = context.relevant_memories
                    .iter()
                    .any(|m| m.tags.iter().any(|t| t.contains("harm") || t.contains("violence")));
                // Harm principles are always applicable, but more relevant if harm memories exist
            }
            
            // Consider experiences - if principle is about fairness and we have unfair experiences
            if principle.name.to_lowercase().contains("fair") {
                let has_unfair_experiences = context.recent_experiences
                    .iter()
                    .any(|e| e.reward.map(|r| r < 0.0).unwrap_or(false));
                // Fairness principles are always applicable
            }
            
            if applies {
                applicable.push(principle.clone());
            }
        }
        
        // If no context-based filtering, return all principles
        if applicable.is_empty() {
            Ok(principles.values().cloned().collect())
        } else {
            Ok(applicable)
        }
    }

    fn evaluate_principle<T: serde::Serialize>(
        &self,
        principle: &MoralPrinciple,
        _action: &T,
        context: &AssessmentContext,
    ) -> Result<f64> {
        // Use CPL context to inform principle evaluation
        // Consider relevant memories, experiences, and thoughts
        
        let base_score = match principle.rule_type {
            RuleType::Scoring => {
                // Return effectiveness score as base score
                principle.effectiveness_score
            }
            RuleType::Threshold => {
                // Return threshold if set, otherwise effectiveness
                principle.threshold.unwrap_or(principle.effectiveness_score)
            }
            RuleType::Veto => {
                // Veto principles return low score if threshold not met
                if let Some(threshold) = principle.threshold {
                    threshold
                } else {
                    0.3 // Default low score for veto
                }
            }
        };

        // Adjust score based on CPL context
        // Memories and experiences can inform moral judgment
        let mut adjusted_score = base_score;
        
        // Consider relevant memories (memories with moral tags or high strength)
        let memory_influence: f64 = context.relevant_memories
            .iter()
            .filter(|m| m.strength > 0.7 || m.tags.iter().any(|t| t.contains("moral") || t.contains("ethics")))
            .map(|m| m.strength * 0.1)
            .sum();
        adjusted_score = (adjusted_score + memory_influence.min(0.2)).min(1.0);
        
        // Consider recent experiences (negative outcomes reduce score, positive increase)
        let experience_influence: f64 = context.recent_experiences
            .iter()
            .filter_map(|e| e.reward)
            .map(|r| r * 0.05) // Small influence from experiences
            .sum();
        adjusted_score = (adjusted_score + experience_influence).max(0.0).min(1.0);
        
        Ok(adjusted_score)
    }

    async fn get_recent_experiences(&self) -> Result<Vec<String>> {
        // Get recent experiences for principle evolution
        let experiences = self.brain.experiences.read();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let mut recent: Vec<(u64, String)> = experiences
            .values()
            .filter(|exp| now.saturating_sub(exp.timestamp) < 86400) // Last 24 hours
            .map(|exp| (exp.timestamp, format!("{}: {:?}", exp.event_type, exp.observation)))
            .collect();
        
        recent.sort_by(|a, b| b.0.cmp(&a.0)); // Most recent first
        Ok(recent.into_iter().map(|(_, s)| s).take(20).collect())
    }

    /// Get relevant memories for moral assessment
    async fn get_relevant_memories(&self, limit: usize) -> Result<Vec<MemorySummary>> {
        // Get recent, high-strength memories that might inform moral judgment
        let memories = self.brain.memories.read();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let mut candidates: Vec<(Memory, f64)> = memories
            .values()
            .map(|m| {
                // Score by strength, recency, and access count
                let recency = 1.0 / (1.0 + (now.saturating_sub(m.last_accessed)) as f64 / 86400.0);
                let score = m.strength * (m.access_count as f64 + 1.0) * recency;
                (m.clone(), score)
            })
            .collect();
        
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(limit);
        
        Ok(candidates
            .into_iter()
            .map(|(m, _)| {
                let content_str = serde_json::to_string(&m.content)
                    .unwrap_or_else(|_| format!("{:?}", m.content));
                let content_summary = if content_str.len() > 200 {
                    format!("{}...", &content_str[..200])
                } else {
                    content_str
                };
                
                MemorySummary {
                    id: m.id,
                    memory_type: format!("{:?}", m.memory_type),
                    content_summary,
                    strength: m.strength,
                    tags: m.tags,
                    created_at: m.created_at,
                }
            })
            .collect())
    }

    /// Get recent experiences for context
    async fn get_recent_experiences_for_context(&self, limit: usize) -> Result<Vec<ExperienceSummary>> {
        let experiences = self.brain.experiences.read();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let mut recent: Vec<(u64, Experience)> = experiences
            .values()
            .filter(|exp| now.saturating_sub(exp.timestamp) < 86400) // Last 24 hours
            .map(|exp| (exp.timestamp, exp.clone()))
            .collect();
        
        recent.sort_by(|a, b| b.0.cmp(&a.0)); // Most recent first
        recent.truncate(limit);
        
        Ok(recent
            .into_iter()
            .map(|(_, exp)| {
                let obs_str = serde_json::to_string(&exp.observation)
                    .unwrap_or_else(|_| format!("{:?}", exp.observation));
                let obs_summary = if obs_str.len() > 150 {
                    format!("{}...", &obs_str[..150])
                } else {
                    obs_str
                };
                
                let action_summary = exp.action.as_ref().map(|a| {
                    let s = serde_json::to_string(a).unwrap_or_else(|_| format!("{:?}", a));
                    if s.len() > 100 {
                        format!("{}...", &s[..100])
                    } else {
                        s
                    }
                });
                
                let outcome_summary = exp.outcome.as_ref().map(|o| {
                    let s = serde_json::to_string(o).unwrap_or_else(|_| format!("{:?}", o));
                    if s.len() > 100 {
                        format!("{}...", &s[..100])
                    } else {
                        s
                    }
                });
                
                ExperienceSummary {
                    id: exp.id,
                    event_type: exp.event_type,
                    observation_summary: obs_summary,
                    action_summary,
                    outcome_summary,
                    reward: exp.reward,
                    timestamp: exp.timestamp,
                }
            })
            .collect())
    }

    /// Get active thoughts for context
    async fn get_active_thoughts(&self, limit: usize) -> Result<Vec<ThoughtSummary>> {
        let thoughts = self.brain.thoughts.read();
        
        let mut active: Vec<Thought> = thoughts
            .values()
            .filter(|t| matches!(t.state, crate::cognitive::ThoughtState::Active))
            .cloned()
            .collect();
        
        // Sort by priority
        active.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap_or(std::cmp::Ordering::Equal));
        active.truncate(limit);
        
        Ok(active
            .into_iter()
            .map(|t| {
                let content_str = serde_json::to_string(&t.content)
                    .unwrap_or_else(|_| format!("{:?}", t.content));
                let content_summary = if content_str.len() > 200 {
                    format!("{}...", &content_str[..200])
                } else {
                    content_str
                };
                
                ThoughtSummary {
                    id: t.id,
                    content_summary,
                    priority: t.priority,
                    state: format!("{:?}", t.state),
                }
            })
            .collect())
    }

    /// Get working memory state
    async fn get_working_memory_state(&self) -> Result<Vec<JsonValue>> {
        let working_memory = self.brain.get_working_memory();
        Ok(working_memory
            .into_iter()
            .map(|state| {
                serde_json::json!({
                    "id": state.id,
                    "thought_id": state.thought_id,
                    "state_type": format!("{:?}", state.state_type),
                    "content": state.content,
                    "priority": state.priority,
                })
            })
            .collect())
    }

    /// Hash context for cache key
    fn hash_context(&self, context: &AssessmentContext) -> String {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        
        let mut hasher = DefaultHasher::new();
        // Hash key context elements
        if let Some(cpl_id) = &context.cpl_id {
            cpl_id.hash(&mut hasher);
        }
        context.relevant_memories.len().hash(&mut hasher);
        context.recent_experiences.len().hash(&mut hasher);
        context.active_thoughts.len().hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

