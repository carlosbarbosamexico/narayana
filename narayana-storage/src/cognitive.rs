// Cognitive Architecture for Next-Generation Robots
// NarayanaDB as a Brain: Multiple Thoughts, Memories, Experiences
// With Transform & Filter System - Filter Thoughts, Transform Memories into Actions!

use narayana_core::{
    Error, Result, types::TableId,
    transforms::{
        OutputConfig, DefaultFilter, OutputTransform, ConfigContext, TransformEngine,
    },
};
use crate::dynamic_output::DynamicOutputManager;
use crate::genetics::{GeneticSystem, Genome};
use crate::traits_equations::{TraitCalculator, TraitType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use parking_lot::RwLock;
use tokio::sync::broadcast;
use std::collections::{HashMap, VecDeque};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use uuid::Uuid;

/// Cognitive state - represents a thought or cognitive process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveState {
    pub id: String,
    pub thought_id: String,
    pub state_type: CognitiveStateType,
    pub content: serde_json::Value,
    pub context: HashMap<String, serde_json::Value>,
    pub timestamp: u64,
    pub priority: f64,
    pub associations: Vec<String>, // Links to other thoughts/memories
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CognitiveStateType {
    Thought,
    Memory,
    Experience,
    Decision,
    Plan,
    Observation,
    Reflection,
    Prediction,
    Hypothesis,
    Goal,
}

/// Memory access record for thought tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAccessRecord {
    pub memory_id: String,
    pub access_type: MemoryAccessType,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryAccessType {
    Read,
    Write,
    Delete,
}

/// Thought - parallel cognitive process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thought {
    pub id: String,
    pub thread_id: String,
    pub content: serde_json::Value,
    pub state: ThoughtState,
    pub created_at: u64,
    pub updated_at: u64,
    pub context: HashMap<String, serde_json::Value>,
    pub associations: Vec<String>,
    pub priority: f64,
    pub memory_accesses: Vec<MemoryAccessRecord>, // Track memory accesses
    pub spawned_thoughts: Vec<String>, // Track spawned thoughts
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThoughtState {
    Active,
    Paused,
    Completed,
    Merged,
    Discarded,
}

/// Memory types for robot brain
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryType {
    Episodic,      // Specific events and experiences
    Semantic,      // Facts and knowledge
    Procedural,    // Skills and how-to knowledge
    Working,      // Short-term active memory
    LongTerm,      // Permanent storage
    Associative,   // Linked memories
    Emotional,     // Emotional associations
    Spatial,       // Spatial/topological memory
    Temporal,      // Time-based memory
}

/// Memory - stored experience or knowledge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub memory_type: MemoryType,
    pub content: serde_json::Value,
    pub embedding: Option<Vec<f32>>, // Vector embedding for semantic search
    pub strength: f64,               // Memory strength (for forgetting curves)
    pub access_count: u64,
    pub last_accessed: u64,
    pub created_at: u64,
    pub associations: Vec<String>,   // Links to other memories
    pub context: HashMap<String, serde_json::Value>,
    pub tags: Vec<String>,
}

/// Experience - learned pattern or event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub id: String,
    pub event_type: String,
    pub observation: serde_json::Value,
    pub action: Option<serde_json::Value>,
    pub outcome: Option<serde_json::Value>,
    pub reward: Option<f64>,
    pub timestamp: u64,
    pub context: HashMap<String, serde_json::Value>,
    pub patterns: Vec<Pattern>, // Learned patterns from this experience
    pub embedding: Option<Vec<f32>>,
}

/// Pattern - learned pattern from experiences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: String,
    pub pattern_type: PatternType,
    pub conditions: serde_json::Value,
    pub action: serde_json::Value,
    pub outcome: serde_json::Value,
    pub confidence: f64,
    pub frequency: u64,
    pub last_seen: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternType {
    Causal,        // Cause-effect relationships
    Temporal,      // Time-based patterns
    Spatial,       // Spatial patterns
    Behavioral,    // Behavioral patterns
    Sequential,    // Sequential patterns
    Associative,   // Associative patterns
}

/// Cognitive Brain - main cognitive architecture
/// With Transform & Filter System - The Real Brain!
pub struct CognitiveBrain {
    pub(crate) thoughts: Arc<RwLock<HashMap<String, Thought>>>,
    pub(crate) memories: Arc<RwLock<HashMap<String, Memory>>>,
    pub(crate) experiences: Arc<RwLock<HashMap<String, Experience>>>,
    pub(crate) patterns: Arc<RwLock<HashMap<String, Pattern>>>,
    working_memory: Arc<RwLock<Vec<CognitiveState>>>,
    thought_threads: Arc<RwLock<HashMap<String, ThoughtThread>>>,
    event_sender: broadcast::Sender<CognitiveEvent>,
    memory_index: Arc<RwLock<MemoryIndex>>,
    // NEW: Transform & Filter System
    output_manager: Arc<DynamicOutputManager>,
    // RL Engine integration (optional, can be set after creation)
    rl_engine: Arc<RwLock<Option<Arc<crate::reinforcement_learning::RLEngine>>>>,
    // NEW: Event history for timeline
    event_history: Arc<RwLock<VecDeque<CognitiveEventWithTimestamp>>>,
    // Genetics and traits
    genetic_system: Arc<RwLock<Option<Arc<GeneticSystem>>>>,
    trait_calculator: Arc<RwLock<Option<Arc<TraitCalculator>>>>,
    // LLM Manager integration (optional, can be set after creation)
    #[cfg(feature = "llm")]
    llm_manager: Arc<RwLock<Option<Arc<narayana_llm::LLMManager>>>>,
}

/// Thought thread - parallel thought process
#[derive(Debug, Clone)]
struct ThoughtThread {
    id: String,
    thoughts: Vec<String>,
    state: ThoughtState,
    priority: f64,
}

/// Memory index for fast retrieval
struct MemoryIndex {
    by_type: HashMap<MemoryType, Vec<String>>,
    by_tag: HashMap<String, Vec<String>>,
    by_association: HashMap<String, Vec<String>>,
    temporal_index: Vec<(u64, String)>, // (timestamp, memory_id)
}

/// Cognitive event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CognitiveEvent {
    ThoughtCreated { thought_id: String },
    ThoughtCompleted { thought_id: String },
    MemoryFormed { memory_id: String, memory_type: MemoryType },
    ExperienceStored { experience_id: String },
    PatternLearned { pattern_id: String },
    AssociationCreated { from: String, to: String },
    MemoryRetrieved { memory_id: String },
    ThoughtMerged { from: Vec<String>, to: String },
    ThoughtDiscarded { thought_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveEventWithTimestamp {
    pub event: CognitiveEvent,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub conflict_type: String,
    pub description: String,
    pub thought_ids: Vec<String>,
    pub severity: f64,
}

impl CognitiveBrain {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self {
            thoughts: Arc::new(RwLock::new(HashMap::new())),
            memories: Arc::new(RwLock::new(HashMap::new())),
            experiences: Arc::new(RwLock::new(HashMap::new())),
            patterns: Arc::new(RwLock::new(HashMap::new())),
            working_memory: Arc::new(RwLock::new(Vec::new())),
            thought_threads: Arc::new(RwLock::new(HashMap::new())),
            event_sender: sender,
            memory_index: Arc::new(RwLock::new(MemoryIndex {
                by_type: HashMap::new(),
                by_tag: HashMap::new(),
                by_association: HashMap::new(),
                temporal_index: Vec::new(),
            })),
            output_manager: Arc::new(DynamicOutputManager::new()),
            rl_engine: Arc::new(RwLock::new(None)),
            event_history: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            genetic_system: Arc::new(RwLock::new(None)),
            trait_calculator: Arc::new(RwLock::new(None)),
            #[cfg(feature = "llm")]
            llm_manager: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Cleanup old/completed thoughts to prevent memory leaks
    pub fn cleanup_thoughts(&self) {
        let mut thoughts = self.thoughts.write();
        let mut to_remove = Vec::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        // Remove thoughts that are completed/discarded and older than 1 hour
        for (id, thought) in thoughts.iter() {
            if (thought.state == ThoughtState::Completed || thought.state == ThoughtState::Discarded)
                && (now.saturating_sub(thought.updated_at) > 3600) {
                to_remove.push(id.clone());
            }
        }
        
        for id in to_remove {
            thoughts.remove(&id);
        }
    }
    
    /// Set RL engine for learning from experiences
    pub fn set_rl_engine(&self, rl_engine: Arc<crate::reinforcement_learning::RLEngine>) {
        *self.rl_engine.write() = Some(rl_engine);
    }
    
    /// Get RL engine if available
    pub fn get_rl_engine(&self) -> Option<Arc<crate::reinforcement_learning::RLEngine>> {
        self.rl_engine.read().clone()
    }
    
    /// Set genetic system and trait calculator
    pub fn set_genetics(
        &self,
        genetic_system: Arc<GeneticSystem>,
        trait_calculator: Arc<TraitCalculator>,
    ) {
        *self.genetic_system.write() = Some(genetic_system);
        *self.trait_calculator.write() = Some(trait_calculator);
    }
    
    /// Get genetic system if available
    pub fn get_genetic_system(&self) -> Option<Arc<GeneticSystem>> {
        self.genetic_system.read().clone()
    }
    
    /// Get trait calculator if available
    pub fn get_trait_calculator(&self) -> Option<Arc<TraitCalculator>> {
        self.trait_calculator.read().clone()
    }
    
    /// Set LLM manager for LLM integration
    #[cfg(feature = "llm")]
    pub fn set_llm_manager(&self, llm_manager: Arc<narayana_llm::LLMManager>) {
        *self.llm_manager.write() = Some(llm_manager);
    }
    
    /// Get LLM manager if available
    #[cfg(feature = "llm")]
    pub fn get_llm_manager(&self) -> Option<Arc<narayana_llm::LLMManager>> {
        self.llm_manager.read().clone()
    }
    
    /// Get trait value
    pub fn get_trait(&self, trait_type: &TraitType) -> Result<f64> {
        if let Some(calc) = self.trait_calculator.read().as_ref() {
            calc.get_trait(trait_type)
        } else {
            Ok(0.5) // Default neutral value
        }
    }
    
    /// Update environmental factor from experience
    pub fn update_environmental_factor(
        &self,
        factor_type: &str,
        value: f64,
        decay_rate: f64,
    ) -> Result<()> {
        if let Some(calc) = self.trait_calculator.read().as_ref() {
            calc.update_environmental_factor(factor_type, value, decay_rate)
        } else {
            Ok(())
        }
    }
    
    /// Get output manager for dynamic transforms/filters
    pub fn output_manager(&self) -> &DynamicOutputManager {
        &self.output_manager
    }

    /// Create a new thought (parallel cognitive process)
    /// Supports on-the-fly thought creation during processing
    pub fn create_thought(&self, content: serde_json::Value, priority: f64) -> Result<String> {
        // Cleanup old thoughts occasionally (1 in 10 chance)
        if rand::random::<f64>() < 0.1 {
            self.cleanup_thoughts();
        }

        let thought_id = Uuid::new_v4().to_string();
        let thread_id = Uuid::new_v4().to_string();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Adjust priority based on traits (e.g., attention span affects thought priority)
        let adjusted_priority = if let Some(calc) = self.trait_calculator.read().as_ref() {
            if let Ok(attention_trait) = calc.get_trait(&TraitType::AttentionSpan) {
                // Higher attention span = more focused thoughts = higher priority
                priority * (0.5 + attention_trait * 0.5)
            } else {
                priority
            }
        } else {
            priority
        };

        let thought = Thought {
            id: thought_id.clone(),
            thread_id: thread_id.clone(),
            content,
            state: ThoughtState::Active,
            created_at: now,
            updated_at: now,
            context: HashMap::new(),
            associations: Vec::new(),
            priority: adjusted_priority,
            memory_accesses: Vec::new(),
            spawned_thoughts: Vec::new(),
        };

        self.thoughts.write().insert(thought_id.clone(), thought);

        // Create thought thread
        let thread = ThoughtThread {
            id: thread_id.clone(),
            thoughts: vec![thought_id.clone()],
            state: ThoughtState::Active,
            priority: adjusted_priority,
        };
        self.thought_threads.write().insert(thread_id, thread);

        self.track_event(CognitiveEvent::ThoughtCreated {
            thought_id: thought_id.clone(),
        });

        Ok(thought_id)
    }

    /// Store a memory
    pub fn store_memory(
        &self,
        memory_type: MemoryType,
        content: serde_json::Value,
        embedding: Option<Vec<f32>>,
        tags: Vec<String>,
        thought_id: Option<&str>,
    ) -> Result<String> {
        let memory_id = Uuid::new_v4().to_string();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Adjust memory strength based on traits (e.g., memory capacity affects initial strength)
        let base_strength = 1.0;
        let adjusted_strength = if let Some(calc) = self.trait_calculator.read().as_ref() {
            if let Ok(memory_trait) = calc.get_trait(&TraitType::MemoryCapacity) {
                // Higher memory capacity = stronger initial memory encoding
                base_strength * (0.5 + memory_trait * 0.5)
            } else {
                base_strength
            }
        } else {
            base_strength
        };

        let memory = Memory {
            id: memory_id.clone(),
            memory_type: memory_type.clone(),
            content,
            embedding: embedding.clone(),
            strength: adjusted_strength,
            access_count: 0,
            last_accessed: now,
            created_at: now,
            associations: Vec::new(),
            context: HashMap::new(),
            tags: tags.clone(),
        };

        self.memories.write().insert(memory_id.clone(), memory);

        // Update index
        let mut index = self.memory_index.write();
        index.by_type
            .entry(memory_type.clone())
            .or_insert_with(Vec::new)
            .push(memory_id.clone());
        for tag in tags {
            index.by_tag
                .entry(tag)
                .or_insert_with(Vec::new)
                .push(memory_id.clone());
        }
        index.temporal_index.push((now, memory_id.clone()));

        // Track memory write if thought_id is provided
        if let Some(tid) = thought_id {
            self.track_memory_access(tid, &memory_id, MemoryAccessType::Write, now)?;
        }

        self.track_event(CognitiveEvent::MemoryFormed {
            memory_id: memory_id.clone(),
            memory_type,
        });

        Ok(memory_id)
    }

    /// Store an experience
    pub fn store_experience(
        &self,
        event_type: String,
        observation: serde_json::Value,
        action: Option<serde_json::Value>,
        outcome: Option<serde_json::Value>,
        reward: Option<f64>,
        embedding: Option<Vec<f32>>,
    ) -> Result<String> {
        let experience_id = Uuid::new_v4().to_string();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Update environmental factors from experience (reward influences traits)
        if let Some(reward_val) = reward {
            // Reward affects learning_rate and curiosity traits
            let _ = self.update_environmental_factor("learning_rate", reward_val.max(0.0).min(1.0), 0.1);
            let _ = self.update_environmental_factor("curiosity", (reward_val.abs() * 0.5).max(0.0).min(1.0), 0.15);
        }

        let experience = Experience {
            id: experience_id.clone(),
            event_type,
            observation,
            action,
            outcome,
            reward,
            timestamp: now,
            context: HashMap::new(),
            patterns: Vec::new(),
            embedding,
        };

        self.experiences.write().insert(experience_id.clone(), experience.clone());

        // If RL engine is available, learn from this experience
        if let Some(rl_engine) = self.get_rl_engine() {
            // Store experience in RL replay buffer
            if let Err(e) = rl_engine.store_experience(experience) {
                // Log error but don't fail - RL is optional
                tracing::warn!("Failed to store experience in RL engine: {}", e);
            }
        }

        let _ = self.track_event(CognitiveEvent::ExperienceStored {
            experience_id: experience_id.clone(),
        });

        Ok(experience_id)
    }

    /// Retrieve memories by semantic similarity
    pub fn retrieve_memories_semantic(
        &self,
        query_embedding: &[f32],
        k: usize,
        memory_type: Option<MemoryType>,
        thought_id: Option<&str>,
    ) -> Result<Vec<Memory>> {
        // Limit k to prevent DoS
        let k = k.min(100);
        
        let memories = self.memories.read();
        let mut candidates: Vec<(&Memory, f64)> = Vec::new();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        for memory in memories.values() {
            // Filter by type if specified
            if let Some(ref mt) = memory_type {
                if memory.memory_type != *mt {
                    continue;
                }
            }

            if let Some(ref embedding) = memory.embedding {
                let similarity = Self::cosine_similarity(query_embedding, embedding)?;
                candidates.push((memory, similarity));
                
                // Track memory access if thought_id is provided
                if let Some(tid) = thought_id {
                    self.track_memory_access(tid, &memory.id, MemoryAccessType::Read, now)?;
                }
            }
        }

        // Sort by similarity and take top k
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(k);

        Ok(candidates.into_iter().map(|(m, _)| m.clone()).collect())
    }

    /// Retrieve memories using type-specific strategies
    pub fn retrieve_memories_by_type(
        &self,
        memory_type: MemoryType,
        query: Option<&str>,
        query_embedding: Option<&[f32]>,
        k: usize,
    ) -> Result<Vec<Memory>> {
        match memory_type {
            MemoryType::Episodic => {
                // Episodic: retrieve by time recency and event context
                self.retrieve_episodic_memories(query, k)
            }
            MemoryType::Semantic => {
                // Semantic: retrieve by semantic similarity
                if let Some(embedding) = query_embedding {
                    self.retrieve_memories_semantic(embedding, k, Some(MemoryType::Semantic), None)
                } else {
                    // Fallback to tag-based if no embedding
                    if let Some(q) = query {
                        self.retrieve_memories_by_tag(q)
                    } else {
                        Ok(Vec::new())
                    }
                }
            }
            MemoryType::Procedural => {
                // Procedural: retrieve by skill/action matching
                self.retrieve_procedural_memories(query, k)
            }
            MemoryType::Working => {
                // Working: retrieve most recent active memories
                self.retrieve_working_memories(k)
            }
            MemoryType::Associative => {
                // Associative: retrieve by association links
                if let Some(q) = query {
                    // Try to find memory by ID or tag, then get associations
                    let memories = self.memories.read();
                    if let Some(memory) = memories.values().find(|m| m.id == q || m.tags.contains(&q.to_string())) {
                        self.retrieve_memories_by_association(&memory.id)
                    } else {
                        Ok(Vec::new())
                    }
                } else {
                    Ok(Vec::new())
                }
            }
            MemoryType::Emotional => {
                // Emotional: retrieve by emotional valence
                self.retrieve_emotional_memories(query, k)
            }
            MemoryType::Spatial => {
                // Spatial: retrieve by location/spatial context
                self.retrieve_spatial_memories(query, k)
            }
            MemoryType::Temporal => {
                // Temporal: retrieve by time range
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let start = now.saturating_sub(86400); // Last 24 hours
                self.retrieve_memories_temporal(start, now)
            }
            MemoryType::LongTerm => {
                // LongTerm: retrieve by strength and recency
                self.retrieve_longterm_memories(k)
            }
        }
    }

    /// Retrieve episodic memories (time-based, event-focused)
    fn retrieve_episodic_memories(&self, event_query: Option<&str>, k: usize) -> Result<Vec<Memory>> {
        let index = self.memory_index.read();
        let memories = self.memories.read();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut candidates: Vec<(&Memory, f64)> = Vec::new();

        if let Some(episodic_ids) = index.by_type.get(&MemoryType::Episodic) {
            for id in episodic_ids {
                if let Some(memory) = memories.get(id) {
                    // Score by recency and event match
                    let time_score = 1.0 / (1.0 + (now.saturating_sub(memory.created_at)) as f64 / 3600.0); // Decay per hour
                    let mut score = time_score * memory.strength;

                    // Boost if event query matches
                    if let Some(query) = event_query {
                        if memory.content.to_string().to_lowercase().contains(&query.to_lowercase()) {
                            score *= 1.5;
                        }
                    }

                    candidates.push((memory, score));
                }
            }
        }

        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(k);

        Ok(candidates.into_iter().map(|(m, _)| m.clone()).collect())
    }

    /// Retrieve procedural memories (skill/action-based)
    fn retrieve_procedural_memories(&self, skill_query: Option<&str>, k: usize) -> Result<Vec<Memory>> {
        let index = self.memory_index.read();
        let memories = self.memories.read();

        let mut candidates: Vec<(&Memory, f64)> = Vec::new();

        if let Some(procedural_ids) = index.by_type.get(&MemoryType::Procedural) {
            for id in procedural_ids {
                if let Some(memory) = memories.get(id) {
                    let mut score = memory.strength * (memory.access_count as f64 + 1.0);

                    // Boost if skill query matches
                    if let Some(query) = skill_query {
                        let content_str = memory.content.to_string().to_lowercase();
                        if content_str.contains(&query.to_lowercase()) {
                            score *= 2.0;
                        }
                        // Also check tags
                        if memory.tags.iter().any(|t| t.to_lowercase().contains(&query.to_lowercase())) {
                            score *= 1.5;
                        }
                    }

                    candidates.push((memory, score));
                }
            }
        }

        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(k);

        Ok(candidates.into_iter().map(|(m, _)| m.clone()).collect())
    }

    /// Retrieve working memories (most recent, active)
    fn retrieve_working_memories(&self, k: usize) -> Result<Vec<Memory>> {
        let working_states = self.working_memory.read();
        let memories = self.memories.read();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut candidates: Vec<(&Memory, f64)> = Vec::new();

        // Get memories referenced in working memory
        for state in working_states.iter() {
            if let Some(memory) = memories.get(&state.id) {
                // Score by recency and priority
                let recency = 1.0 / (1.0 + (now.saturating_sub(state.timestamp)) as f64 / 60.0); // Decay per minute
                let score = recency * state.priority as f64;
                candidates.push((memory, score));
            }
        }

        // Also get recently accessed memories
        let index = self.memory_index.read();
        if let Some(all_ids) = index.by_type.get(&MemoryType::Working) {
            for id in all_ids {
                if let Some(memory) = memories.get(id) {
                    let recency = 1.0 / (1.0 + (now.saturating_sub(memory.last_accessed)) as f64 / 60.0);
                    let score = recency * memory.strength;
                    if !candidates.iter().any(|(m, _)| m.id == memory.id) {
                        candidates.push((memory, score));
                    }
                }
            }
        }

        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(k);

        Ok(candidates.into_iter().map(|(m, _)| m.clone()).collect())
    }

    /// Retrieve emotional memories (valence-based)
    fn retrieve_emotional_memories(&self, valence_query: Option<&str>, k: usize) -> Result<Vec<Memory>> {
        let index = self.memory_index.read();
        let memories = self.memories.read();

        let mut candidates: Vec<(&Memory, f64)> = Vec::new();

        if let Some(emotional_ids) = index.by_type.get(&MemoryType::Emotional) {
            for id in emotional_ids {
                if let Some(memory) = memories.get(id) {
                    let mut score = memory.strength;

                    // Filter by valence if specified
                    if let Some(query) = valence_query {
                        let query_lower = query.to_lowercase();
                        let content_str = memory.content.to_string().to_lowercase();
                        if query_lower == "positive" || query_lower == "happy" || query_lower == "good" {
                            if content_str.contains("positive") || content_str.contains("happy") || content_str.contains("good") {
                                score *= 2.0;
                            } else {
                                score *= 0.1; // Filter out negative
                            }
                        } else if query_lower == "negative" || query_lower == "sad" || query_lower == "bad" {
                            if content_str.contains("negative") || content_str.contains("sad") || content_str.contains("bad") {
                                score *= 2.0;
                            } else {
                                score *= 0.1; // Filter out positive
                            }
                        }
                    }

                    candidates.push((memory, score));
                }
            }
        }

        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(k);

        Ok(candidates.into_iter().map(|(m, _)| m.clone()).collect())
    }

    /// Retrieve spatial memories (location-based)
    fn retrieve_spatial_memories(&self, location_query: Option<&str>, k: usize) -> Result<Vec<Memory>> {
        let index = self.memory_index.read();
        let memories = self.memories.read();

        let mut candidates: Vec<(&Memory, f64)> = Vec::new();

        if let Some(spatial_ids) = index.by_type.get(&MemoryType::Spatial) {
            for id in spatial_ids {
                if let Some(memory) = memories.get(id) {
                    let mut score = memory.strength;

                    // Boost if location query matches
                    if let Some(query) = location_query {
                        let content_str = memory.content.to_string().to_lowercase();
                        if content_str.contains(&query.to_lowercase()) {
                            score *= 2.0;
                        }
                        // Check context for location
                        if let Some(location) = memory.context.get("location") {
                            if location.to_string().to_lowercase().contains(&query.to_lowercase()) {
                                score *= 2.5;
                            }
                        }
                    }

                    candidates.push((memory, score));
                }
            }
        }

        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(k);

        Ok(candidates.into_iter().map(|(m, _)| m.clone()).collect())
    }

    /// Retrieve long-term memories (strength and recency)
    fn retrieve_longterm_memories(&self, k: usize) -> Result<Vec<Memory>> {
        let index = self.memory_index.read();
        let memories = self.memories.read();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut candidates: Vec<(&Memory, f64)> = Vec::new();

        if let Some(lt_ids) = index.by_type.get(&MemoryType::LongTerm) {
            for id in lt_ids {
                if let Some(memory) = memories.get(id) {
                    // Score by strength and access frequency
                    let recency = 1.0 / (1.0 + (now.saturating_sub(memory.last_accessed)) as f64 / 86400.0); // Decay per day
                    let score = memory.strength * (memory.access_count as f64 + 1.0) * recency;
                    candidates.push((memory, score));
                }
            }
        }

        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(k);

        Ok(candidates.into_iter().map(|(m, _)| m.clone()).collect())
    }

    /// Track memory access for a thought
    fn track_memory_access(
        &self,
        thought_id: &str,
        memory_id: &str,
        access_type: MemoryAccessType,
        timestamp: u64,
    ) -> Result<()> {
        let mut thoughts = self.thoughts.write();
        if let Some(thought) = thoughts.get_mut(thought_id) {
            thought.memory_accesses.push(MemoryAccessRecord {
                memory_id: memory_id.to_string(),
                access_type,
                timestamp,
            });
        }
        Ok(())
    }

    /// Retrieve memories by association
    pub fn retrieve_memories_by_association(&self, memory_id: &str) -> Result<Vec<Memory>> {
        let memories = self.memories.read();
        let memory = memories
            .get(memory_id)
            .ok_or_else(|| Error::Storage(format!("Memory not found: {}", memory_id)))?;

        let mut associated = Vec::new();
        for assoc_id in &memory.associations {
            if let Some(assoc_memory) = memories.get(assoc_id) {
                associated.push(assoc_memory.clone());
            }
        }

        Ok(associated)
    }

    /// Retrieve memories by tag
    pub fn retrieve_memories_by_tag(&self, tag: &str) -> Result<Vec<Memory>> {
        let index = self.memory_index.read();
        let memory_ids = index.by_tag.get(tag).cloned().unwrap_or_default();
        let memories = self.memories.read();

        Ok(memory_ids
            .iter()
            .filter_map(|id| memories.get(id).cloned())
            .collect())
    }

    /// Retrieve memories temporally
    pub fn retrieve_memories_temporal(
        &self,
        start_time: u64,
        end_time: u64,
    ) -> Result<Vec<Memory>> {
        let index = self.memory_index.read();
        let memories = self.memories.read();

        let memory_ids: Vec<String> = index
            .temporal_index
            .iter()
            .filter(|(ts, _)| *ts >= start_time && *ts <= end_time)
            .map(|(_, id)| id.clone())
            .collect();

        Ok(memory_ids
            .iter()
            .filter_map(|id| memories.get(id).cloned())
            .collect())
    }

    /// Learn pattern from experience
    pub fn learn_pattern(
        &self,
        experience_id: &str,
        pattern_type: PatternType,
        conditions: serde_json::Value,
        action: serde_json::Value,
        outcome: serde_json::Value,
    ) -> Result<String> {
        let pattern_id = Uuid::new_v4().to_string();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let pattern = Pattern {
            id: pattern_id.clone(),
            pattern_type,
            conditions,
            action,
            outcome,
            confidence: 1.0,
            frequency: 1,
            last_seen: now,
        };

        // Clone pattern before moving it
        let pattern_for_experience = Pattern {
            id: pattern_id.clone(),
            pattern_type: pattern.pattern_type.clone(),
            conditions: pattern.conditions.clone(),
            action: pattern.action.clone(),
            outcome: pattern.outcome.clone(),
            confidence: pattern.confidence,
            frequency: pattern.frequency,
            last_seen: pattern.last_seen,
        };

        self.patterns.write().insert(pattern_id.clone(), pattern);

        // Associate pattern with experience
        if let Some(experience) = self.experiences.write().get_mut(experience_id) {
            experience.patterns.push(pattern_for_experience);
        }

        self.track_event(CognitiveEvent::PatternLearned {
            pattern_id: pattern_id.clone(),
        });

        Ok(pattern_id)
    }

    /// Automatically detect patterns from experiences
    pub fn detect_patterns_from_experiences(&self) -> Result<Vec<String>> {
        // Collect data while holding read lock, then release it before calling learn_pattern
        let (total_experiences, pattern_groups) = {
            let experiences = self.experiences.read();
            let total = experiences.len();
            
            // Limit processing to avoid performance issues with large datasets
            let max_experiences_to_process = 10_000.min(total);

            // Group experiences by similar conditions and outcomes
            // Use a more efficient key: hash of event_type + simplified observation/outcome
            let mut groups: HashMap<u64, Vec<(String, serde_json::Value, serde_json::Value, serde_json::Value)>> = HashMap::new();

            let mut processed = 0;
            for experience in experiences.values() {
                if processed >= max_experiences_to_process {
                    break;
                }
                
                // Create pattern key using hash instead of full JSON serialization
                let mut hasher = DefaultHasher::new();
                experience.event_type.hash(&mut hasher);
                
                // Use a simplified key from observation/outcome instead of full JSON
                // Extract key fields if they exist, otherwise use a hash of the JSON
                let obs_key = if experience.observation.is_object() {
                    // Use pattern_id or sequence if available (common in benchmarks)
                    experience.observation.get("pattern_id")
                        .or_else(|| experience.observation.get("sequence"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                } else {
                    0
                };
                obs_key.hash(&mut hasher);
                
                // Hash outcome if present
                if let Some(outcome) = &experience.outcome {
                    if outcome.is_object() {
                        outcome.get("result")
                            .or_else(|| outcome.get("success"))
                            .and_then(|v| v.as_u64().or_else(|| v.as_bool().map(|b| if b { 1 } else { 0 })))
                            .unwrap_or(0)
                            .hash(&mut hasher);
                    }
                }
                
                let pattern_key = hasher.finish();
                groups.entry(pattern_key)
                    .or_insert_with(Vec::new)
                    .push((
                        experience.id.clone(),
                        experience.observation.clone(),
                        experience.action.clone().unwrap_or_default(),
                        experience.outcome.clone().unwrap_or_default(),
                    ));
                
                processed += 1;
            }
            
            (total, groups)
        }; // Read lock is dropped here

        // Now detect patterns that occur frequently (no lock held)
        let mut detected_patterns = Vec::new();
        for (_pattern_key, group) in pattern_groups {
            if group.len() >= 3 { // Pattern must occur at least 3 times
                // Extract common pattern from first experience in group
                let (experience_id, observation, action, outcome) = &group[0];
                let pattern_id = self.learn_pattern(
                    experience_id,
                    PatternType::Causal,
                    observation.clone(),
                    action.clone(),
                    outcome.clone(),
                )?;

                // Update pattern frequency and confidence
                if let Some(pattern) = self.patterns.write().get_mut(&pattern_id) {
                    pattern.frequency = group.len() as u64;
                    pattern.confidence = (group.len() as f64 / total_experiences as f64).min(1.0);
                }

                detected_patterns.push(pattern_id);
            }
        }

        Ok(detected_patterns)
    }

    /// Find patterns matching conditions
    pub fn find_matching_patterns(&self, conditions: &serde_json::Value) -> Result<Vec<Pattern>> {
        let patterns = self.patterns.read();
        let mut matches = Vec::new();

        for pattern in patterns.values() {
            // Simple matching: check if conditions overlap
            let conditions_str = serde_json::to_string(&pattern.conditions).unwrap_or_default();
            let query_str = serde_json::to_string(conditions).unwrap_or_default();

            // Check for similarity (in production, would use proper JSON comparison)
            if conditions_str.contains(&query_str) || query_str.contains(&conditions_str) {
                matches.push(pattern.clone());
            }
        }

        // Sort by confidence and frequency
        matches.sort_by(|a, b| {
            b.confidence.partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.frequency.cmp(&a.frequency))
        });

        Ok(matches)
    }

    /// Apply learned pattern (predict action from conditions)
    pub fn apply_pattern(&self, pattern_id: &str, conditions: &serde_json::Value) -> Result<Option<serde_json::Value>> {
        let patterns = self.patterns.read();
        
        if let Some(pattern) = patterns.get(pattern_id) {
            // Check if conditions match
            let pattern_conditions_str = serde_json::to_string(&pattern.conditions).unwrap_or_default();
            let query_conditions_str = serde_json::to_string(conditions).unwrap_or_default();

            if pattern_conditions_str.contains(&query_conditions_str) || query_conditions_str.contains(&pattern_conditions_str) {
                // Return predicted action
                return Ok(Some(pattern.action.clone()));
            }
        }

        Ok(None)
    }

    /// Create association between memories/thoughts
    pub fn create_association(&self, from_id: &str, to_id: &str) -> Result<()> {
        // Add association to memory if it exists
        if let Some(memory) = self.memories.write().get_mut(from_id) {
            if !memory.associations.contains(&to_id.to_string()) {
                memory.associations.push(to_id.to_string());
            }
        }

        // Add association to thought if it exists
        if let Some(thought) = self.thoughts.write().get_mut(from_id) {
            if !thought.associations.contains(&to_id.to_string()) {
                thought.associations.push(to_id.to_string());
            }
        }

        // Update index
        let mut index = self.memory_index.write();
        index
            .by_association
            .entry(from_id.to_string())
            .or_insert_with(Vec::new)
            .push(to_id.to_string());

        self.track_event(CognitiveEvent::AssociationCreated {
            from: from_id.to_string(),
            to: to_id.to_string(),
        });

        Ok(())
    }

    /// Merge multiple thoughts into one
    pub fn merge_thoughts(&self, thought_ids: Vec<String>) -> Result<String> {
        let thoughts = self.thoughts.read();
        let mut merged_content = serde_json::json!({});
        let mut merged_context = HashMap::new();
        let mut merged_associations = Vec::new();
        let mut max_priority: f64 = 0.0;

        for thought_id in &thought_ids {
            if let Some(thought) = thoughts.get(thought_id) {
                // Merge content
                if let Some(obj) = merged_content.as_object_mut() {
                    obj.insert(thought_id.clone(), thought.content.clone());
                }
                // Merge context
                merged_context.extend(thought.context.clone());
                // Merge associations
                merged_associations.extend(thought.associations.clone());
                // Track max priority
                max_priority = max_priority.max(thought.priority);
            }
        }

        drop(thoughts);

        // Create merged thought
        let merged_id = self.create_thought(merged_content, max_priority)?;

        // Update associations
        for thought_id in &thought_ids {
            self.create_association(thought_id, &merged_id)?;
        }

        // Mark original thoughts as merged
        let mut thoughts = self.thoughts.write();
        for thought_id in &thought_ids {
            if let Some(thought) = thoughts.get_mut(thought_id) {
                thought.state = ThoughtState::Merged;
            }
        }

        self.track_event(CognitiveEvent::ThoughtMerged {
            from: thought_ids,
            to: merged_id.clone(),
        });

        Ok(merged_id)
    }

    /// Add to working memory (short-term active memory)
    pub fn add_to_working_memory(&self, state: CognitiveState) {
        let mut working = self.working_memory.write();
        working.push(state);
        // Limit working memory size (e.g., last 100 items)
        if working.len() > 100 {
            working.remove(0);
        }
    }

    /// Retrieve from working memory
    pub fn get_working_memory(&self) -> Vec<CognitiveState> {
        self.working_memory.read().clone()
    }
    
    /// Track event for timeline
    fn track_event(&self, event: CognitiveEvent) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        let event_with_timestamp = CognitiveEventWithTimestamp {
            event: event.clone(),
            timestamp: now,
        };
        
        // Store in history
        let mut history = self.event_history.write();
        history.push_back(event_with_timestamp);
        
        // Limit history size
        if history.len() > 1000 {
            history.pop_front();
        }
        
        // Broadcast event
        let _ = self.event_sender.send(event);
    }

    /// Get thought timeline
    pub fn get_thought_timeline(&self) -> Vec<CognitiveEventWithTimestamp> {
        self.event_history.read().iter().cloned().collect()
    }

    /// Get thoughts by state
    pub fn get_thoughts_by_state(&self, state: Option<ThoughtState>) -> Vec<Thought> {
        let thoughts = self.thoughts.read();
        if let Some(target_state) = state {
            thoughts.values()
                .filter(|t| t.state == target_state)
                .cloned()
                .collect()
        } else {
            thoughts.values().cloned().collect()
        }
    }
    
    /// Get all memory accesses
    pub fn get_all_memory_accesses(&self) -> Vec<MemoryAccessRecord> {
        let thoughts = self.thoughts.read();
        let mut all_accesses = Vec::new();
        
        for thought in thoughts.values() {
            all_accesses.extend(thought.memory_accesses.clone());
        }
        
        // Sort by timestamp (descending)
        all_accesses.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        all_accesses
    }
    
    /// Detect conflicts between thoughts
    pub fn detect_conflicts(&self) -> Vec<Conflict> {
        let thoughts = self.thoughts.read();
        let active_thoughts: Vec<&Thought> = thoughts.values()
            .filter(|t| t.state == ThoughtState::Active)
            .collect();
            
        let mut conflicts = Vec::new();
        
        // Check 1: Priority conflicts (multiple very high priority thoughts)
        let high_priority_thoughts: Vec<&Thought> = active_thoughts.iter()
            .filter(|t| t.priority > 0.8)
            .cloned()
            .collect();
            
        if high_priority_thoughts.len() > 3 {
            conflicts.push(Conflict {
                conflict_type: "Priority Overload".to_string(),
                description: format!("{} high priority thoughts competing for resources", high_priority_thoughts.len()),
                thought_ids: high_priority_thoughts.iter().map(|t| t.id.clone()).collect(),
                severity: 0.7,
            });
        }
        
        // Check 2: Resource conflicts (accessing same memories)
        let mut memory_access_counts: HashMap<String, Vec<String>> = HashMap::new();
        
        for thought in &active_thoughts {
            // Look at recent accesses (last 5)
            for access in thought.memory_accesses.iter().rev().take(5) {
                if access.access_type == MemoryAccessType::Write {
                    memory_access_counts.entry(access.memory_id.clone())
                        .or_insert_with(Vec::new)
                        .push(thought.id.clone());
                }
            }
        }
        
        for (memory_id, thought_ids) in memory_access_counts {
            if thought_ids.len() > 1 {
                // Remove duplicates
                let mut unique_ids = thought_ids.clone();
                unique_ids.sort();
                unique_ids.dedup();
                
                if unique_ids.len() > 1 {
                    conflicts.push(Conflict {
                        conflict_type: "Memory Contention".to_string(),
                        description: format!("Multiple thoughts writing to memory {}", memory_id),
                        thought_ids: unique_ids,
                        severity: 0.8,
                    });
                }
            }
        }
        
        conflicts
    }
    
    /// Cancel a thought
    pub fn cancel_thought(&self, thought_id: &str) -> Result<()> {
        let mut thoughts = self.thoughts.write();
        
        if let Some(thought) = thoughts.get_mut(thought_id) {
            if thought.state != ThoughtState::Completed && thought.state != ThoughtState::Discarded {
                thought.state = ThoughtState::Discarded;
                thought.updated_at = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                    
                // Emit event manually since we're holding the lock
                self.track_event(CognitiveEvent::ThoughtDiscarded {
                    thought_id: thought_id.to_string(),
                });
                
                return Ok(());
            }
        }
        
        Err(Error::Storage(format!("Thought {} not found or cannot be cancelled", thought_id)))
    }

    /// Update memory strength (for forgetting curves)
    pub fn update_memory_strength(&self, memory_id: &str, new_strength: f64) -> Result<()> {
        let mut memories = self.memories.write();
        if let Some(memory) = memories.get_mut(memory_id) {
            memory.strength = new_strength.max(0.0).min(1.0);
        }
        Ok(())
    }

    /// Access memory (updates access count and timestamp)
    pub fn access_memory(&self, memory_id: &str) -> Result<Memory> {
        let mut memories = self.memories.write();
        if let Some(memory) = memories.get_mut(memory_id) {
            memory.access_count += 1;
            memory.last_accessed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            self.track_event(CognitiveEvent::MemoryRetrieved {
                memory_id: memory_id.to_string(),
            });
            Ok(memory.clone())
        } else {
            Err(Error::Storage(format!("Memory not found: {}", memory_id)))
        }
    }

    /// Subscribe to cognitive events
    pub fn subscribe(&self) -> broadcast::Receiver<CognitiveEvent> {
        self.event_sender.subscribe()
    }

    /// Cosine similarity for vector embeddings
    fn cosine_similarity(v1: &[f32], v2: &[f32]) -> Result<f64> {
        if v1.len() != v2.len() {
            return Err(Error::Query("Vector dimensions mismatch".to_string()));
        }
        let dot_product: f32 = v1.iter().zip(v2.iter()).map(|(&a, &b)| a * b).sum();
        let magnitude1: f32 = v1.iter().map(|&a| a * a).sum::<f32>().sqrt();
        let magnitude2: f32 = v2.iter().map(|&b| b * b).sum::<f32>().sqrt();

        if magnitude1 == 0.0 || magnitude2 == 0.0 {
            Ok(0.0)
        } else {
            Ok((dot_product / (magnitude1 * magnitude2)) as f64)
        }
    }
}

/// AI Model State Manager - manages AI model states and weights
pub struct AIModelStateManager {
    models: Arc<RwLock<HashMap<String, ModelState>>>,
    checkpoints: Arc<RwLock<HashMap<String, ModelCheckpoint>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelState {
    pub model_id: String,
    pub model_type: String,
    pub weights: Vec<u8>, // Serialized weights
    pub architecture: serde_json::Value,
    pub hyperparameters: HashMap<String, serde_json::Value>,
    pub training_state: TrainingState,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCheckpoint {
    pub checkpoint_id: String,
    pub model_id: String,
    pub weights: Vec<u8>,
    pub metrics: HashMap<String, f64>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainingState {
    NotStarted,
    Training,
    Paused,
    Completed,
    Failed,
}

impl AIModelStateManager {
    pub fn new() -> Self {
        Self {
            models: Arc::new(RwLock::new(HashMap::new())),
            checkpoints: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store model state
    pub fn store_model(&self, model: ModelState) -> Result<()> {
        self.models.write().insert(model.model_id.clone(), model);
        Ok(())
    }

    /// Get model state
    pub fn get_model(&self, model_id: &str) -> Option<ModelState> {
        self.models.read().get(model_id).cloned()
    }

    /// Save checkpoint
    pub fn save_checkpoint(&self, checkpoint: ModelCheckpoint) -> Result<()> {
        self.checkpoints.write().insert(checkpoint.checkpoint_id.clone(), checkpoint);
        Ok(())
    }

    /// Get checkpoint
    pub fn get_checkpoint(&self, checkpoint_id: &str) -> Option<ModelCheckpoint> {
        self.checkpoints.read().get(checkpoint_id).cloned()
    }
}

impl CognitiveBrain {
    // ============================================
    // TRANSFORM & FILTER SYSTEM - THE REAL BRAIN!
    // Filter Thoughts, Transform Memories into Actions
    // ============================================
    
    /// Retrieve memory with transforms and filters applied
    /// This is how the brain filters thoughts and transforms information!
    pub fn retrieve_memory_transformed(
        &self,
        memory_id: &str,
        profile: Option<&str>,
    ) -> Result<serde_json::Value> {
        let memory = self.access_memory(memory_id)?;
        
        // Save memory_type before moving memory
        let memory_type_str = format!("{:?}", memory.memory_type);
        
        // Convert memory to JSON
        let memory_json = serde_json::json!({
            "id": memory.id,
            "memory_type": memory_type_str.clone(),
            "content": memory.content,
            "strength": memory.strength,
            "access_count": memory.access_count,
            "created_at": memory.created_at,
            "tags": memory.tags,
            "associations": memory.associations,
        });
        
        // Get output config for this memory type
        let context = ConfigContext::Brain {
            memory_type: Some(memory_type_str),
        };
        
        if let Some(config) = self.output_manager.get_config_with_profile(&context, "all", profile) {
            // Apply transforms and filters
            TransformEngine::apply_config(memory_json, &config)
        } else {
            Ok(memory_json)
        }
    }
    
    /// Retrieve memories with transforms applied
    pub fn retrieve_memories_transformed(
        &self,
        memory_type: MemoryType,
        k: usize,
        profile: Option<&str>,
    ) -> Result<serde_json::Value> {
        // Save memory_type string before moving
        let memory_type_str = format!("{:?}", memory_type);
        
        let memories = self.retrieve_memories_by_type(memory_type, None, None, k)?;
        
        // Convert to JSON array
        let memories_json: Vec<serde_json::Value> = memories.iter()
            .map(|m| serde_json::json!({
                "id": m.id,
                "memory_type": format!("{:?}", m.memory_type),
                "content": m.content,
                "strength": m.strength,
                "tags": m.tags,
            }))
            .collect();
        
        let data = serde_json::json!({ "memories": memories_json });
        
        // Get config and apply transforms
        let context = ConfigContext::Brain {
            memory_type: Some(memory_type_str),
        };
        
        if let Some(config) = self.output_manager.get_config_with_profile(&context, "all", profile) {
            TransformEngine::apply_config(data, &config)
        } else {
            Ok(data)
        }
    }
    
    /// Get thought with transforms applied
    /// Filter thoughts based on context, transform into actions!
    pub fn get_thought_transformed(
        &self,
        thought_id: &str,
        profile: Option<&str>,
    ) -> Result<serde_json::Value> {
        let thoughts = self.thoughts.read();
        let thought = thoughts.get(thought_id)
            .ok_or_else(|| Error::Storage(format!("Thought {} not found", thought_id)))?;
        
        let thought_json = serde_json::json!({
            "id": thought.id,
            "content": thought.content,
            "state": format!("{:?}", thought.state),
            "priority": thought.priority,
            "created_at": thought.created_at,
            "context": thought.context,
        });
        
        // Get config for thoughts
        let context = ConfigContext::Brain {
            memory_type: None, // Thoughts don't have memory type
        };
        
        if let Some(config) = self.output_manager.get_config_with_profile(&context, "thoughts", profile) {
            TransformEngine::apply_config(thought_json, &config)
        } else {
            Ok(thought_json)
        }
    }
    
    /// Filter thoughts based on filters
    /// This is how the brain filters thoughts - like human cognition!
    pub fn filter_thoughts(
        &self,
        filters: Vec<DefaultFilter>,
    ) -> Result<Vec<Thought>> {
        let thoughts = self.thoughts.read();
        let mut filtered = Vec::new();
        
        for thought in thoughts.values() {
            let thought_json = serde_json::json!({
                "id": thought.id,
                "content": thought.content,
                "priority": thought.priority,
                "state": format!("{:?}", thought.state),
            });
            
            // Apply filters
            let filtered_json = TransformEngine::apply_filters(thought_json, &filters)?;
            
            // If thought passes filters, include it
            if !filtered_json.is_null() {
                filtered.push(thought.clone());
            }
        }
        
        Ok(filtered)
    }
    
    /// Store experience with transforms
    pub fn store_experience_transformed(
        &self,
        event_type: String,
        observation: serde_json::Value,
        action: Option<serde_json::Value>,
        outcome: Option<serde_json::Value>,
        reward: Option<f64>,
        embedding: Option<Vec<f32>>,
    ) -> Result<String> {
        // Store experience
        let experience_id = self.store_experience(
            event_type,
            observation,
            action,
            outcome,
            reward,
            embedding,
        )?;
        
        // Initialize output config for experiences if not exists
        let context = ConfigContext::Brain {
            memory_type: Some("Experience".to_string()),
        };
        
        if self.output_manager.get_config(&context, &experience_id).is_none() {
            self.output_manager.initialize_config(
                context,
                experience_id.clone(),
                OutputConfig::default(),
            )?;
        }
        
        Ok(experience_id)
    }
    
    /// Get experience with transforms
    pub fn get_experience_transformed(
        &self,
        experience_id: &str,
        profile: Option<&str>,
    ) -> Result<serde_json::Value> {
        let experiences = self.experiences.read();
        let experience = experiences.get(experience_id)
            .ok_or_else(|| Error::Storage(format!("Experience {} not found", experience_id)))?;
        
        let exp_json = serde_json::json!({
            "id": experience.id,
            "event_type": experience.event_type,
            "observation": experience.observation,
            "action": experience.action,
            "outcome": experience.outcome,
            "reward": experience.reward,
            "timestamp": experience.timestamp,
        });
        
        let context = ConfigContext::Brain {
            memory_type: Some("Experience".to_string()),
        };
        
        if let Some(config) = self.output_manager.get_config_with_profile(&context, experience_id, profile) {
            TransformEngine::apply_config(exp_json, &config)
        } else {
            Ok(exp_json)
        }
    }
    
    /// Add filter to memory type on-the-fly
    pub async fn add_memory_type_filter(
        &self,
        memory_type: MemoryType,
        filter: DefaultFilter,
    ) -> Result<()> {
        let context = ConfigContext::Brain {
            memory_type: Some(format!("{:?}", memory_type)),
        };
        
        // Initialize config if needed
        if self.output_manager.get_config(&context, "all").is_none() {
            self.output_manager.initialize_config(
                context.clone(),
                "all".to_string(),
                OutputConfig::default(),
            )?;
        }
        
        self.output_manager.add_filter(context, "all".to_string(), filter).await?;
        Ok(())
    }
    
    /// Add transform to memory type on-the-fly
    pub async fn add_memory_type_transform(
        &self,
        memory_type: MemoryType,
        transform: OutputTransform,
    ) -> Result<()> {
        let context = ConfigContext::Brain {
            memory_type: Some(format!("{:?}", memory_type)),
        };
        
        if self.output_manager.get_config(&context, "all").is_none() {
            self.output_manager.initialize_config(
                context.clone(),
                "all".to_string(),
                OutputConfig::default(),
            )?;
        }
        
        self.output_manager.add_transform(context, "all".to_string(), transform).await?;
        Ok(())
    }
}

