// Dynamic Thoughts - On-the-Fly Additional Thoughts During Processing
// Cognitive Brain can spawn new thoughts dynamically during processing

use crate::cognitive::*;
use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use tokio::sync::broadcast;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, debug};
use uuid::Uuid;

/// Dynamic thought spawner - creates thoughts on-the-fly during processing
pub struct DynamicThoughtSpawner {
    brain: Arc<CognitiveBrain>,
    active_processors: Arc<RwLock<HashMap<String, ActiveProcessor>>>,
    thought_triggers: Arc<RwLock<HashMap<String, ThoughtTrigger>>>,
    spawn_callbacks: Arc<RwLock<Vec<Box<dyn Fn(&Thought, &serde_json::Value) -> Option<ThoughtSpawnRequest> + Send + Sync>>>>,
}

#[derive(Debug, Clone)]
struct ActiveProcessor {
    processor_id: String,
    parent_thought_id: Option<String>,
    spawned_thoughts: Vec<String>,
    state: ProcessorState,
    context: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProcessorState {
    Processing,
    Paused,
    Completed,
    Spawning, // Actively spawning new thoughts
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtSpawnRequest {
    pub content: serde_json::Value,
    pub priority: f64,
    pub trigger_reason: String,
    pub parent_thought_id: Option<String>,
    pub context: HashMap<String, serde_json::Value>,
    pub associations: Vec<String>,
    pub spawn_type: ThoughtSpawnType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThoughtSpawnType {
    Parallel,      // Process in parallel with parent
    Sequential,    // Process after parent completes
    Branch,        // Branch from parent thought
    Reflection,    // Reflect on parent thought
    Prediction,    // Predict based on parent thought
    Hypothesis,    // Form hypothesis from parent thought
    Observation,   // Observe something from parent thought
    Decision,      // Make decision based on parent thought
    Plan,          // Create plan from parent thought
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtTrigger {
    pub trigger_id: String,
    pub trigger_type: TriggerType,
    pub condition: TriggerCondition,
    pub spawn_request: ThoughtSpawnRequest,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerType {
    OnContentMatch,      // Trigger when content matches pattern
    OnStateChange,       // Trigger on thought state change
    OnMemoryAccess,      // Trigger when memory is accessed
    OnAssociation,       // Trigger when association is created
    OnPriority,          // Trigger when priority threshold reached
    OnDuration,          // Trigger after processing duration
    OnCompletion,        // Trigger when thought completes
    OnError,             // Trigger on error
    Always,              // Always trigger (for continuous processing)
    Custom(String),      // Custom trigger condition
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerCondition {
    ContentContains(String),
    ContentMatches(serde_json::Value),
    StateIs(ThoughtState),
    PriorityGreaterThan(f64),
    DurationGreaterThan(u64),
    Always,
    Custom(serde_json::Value),
}

/// Thought processing context - tracks processing state
#[derive(Debug, Clone)]
pub struct ThoughtProcessingContext {
    pub processor_id: String,
    pub parent_thought_id: Option<String>,
    pub current_thought_id: String,
    pub spawned_thoughts: Vec<String>,
    pub processing_start: u64,
    pub context: HashMap<String, serde_json::Value>,
    pub can_spawn: bool,
}

impl DynamicThoughtSpawner {
    pub fn new(brain: Arc<CognitiveBrain>) -> Self {
        Self {
            brain,
            active_processors: Arc::new(RwLock::new(HashMap::new())),
            thought_triggers: Arc::new(RwLock::new(HashMap::new())),
            spawn_callbacks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Process thought with dynamic spawning capability
    pub async fn process_with_spawning<F>(
        &self,
        thought_id: String,
        processor: F,
    ) -> Result<Vec<String>>
    where
        F: Fn(ThoughtProcessingContext, serde_json::Value) -> Result<serde_json::Value> + Send + Sync,
    {
        let processor_id = Uuid::new_v4().to_string();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        // Get thought
        let thought = {
            let thoughts = self.brain.thoughts.read();
            thoughts.get(&thought_id).cloned()
        }.ok_or_else(|| Error::Storage(format!("Thought {} not found", thought_id)))?;

        // Create processing context
        let mut context = ThoughtProcessingContext {
            processor_id: processor_id.clone(),
            parent_thought_id: None,
            current_thought_id: thought_id.clone(),
            spawned_thoughts: Vec::new(),
            processing_start: now,
            context: HashMap::new(),
            can_spawn: true,
        };

        // Register active processor
        self.active_processors.write().insert(processor_id.clone(), ActiveProcessor {
            processor_id: processor_id.clone(),
            parent_thought_id: None,
            spawned_thoughts: Vec::new(),
            state: ProcessorState::Processing,
            context: HashMap::new(),
        });

        // Process thought (can spawn new thoughts during processing)
        let result = processor(context.clone(), thought.content.clone())?;

        // Update thought with result
        {
            let mut thoughts = self.brain.thoughts.write();
            if let Some(t) = thoughts.get_mut(&thought_id) {
                t.content = result;
                t.state = ThoughtState::Completed;
                t.updated_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            }
        }

        // Get spawned thoughts
        let spawned = {
            let processors = self.active_processors.read();
            processors.get(&processor_id)
                .map(|p| p.spawned_thoughts.clone())
                .unwrap_or_default()
        };

        // Cleanup
        self.active_processors.write().remove(&processor_id);

        Ok(spawned)
    }

    /// Spawn new thought on-the-fly during processing
    pub fn spawn_thought_during_processing(
        &self,
        processor_id: &str,
        spawn_request: ThoughtSpawnRequest,
    ) -> Result<String> {
        let processor = {
            let processors = self.active_processors.read();
            processors.get(processor_id).cloned()
        };

        if processor.is_none() {
            return Err(Error::Storage("Processor not found".to_string()));
        }

        // Create new thought
        let new_thought_id = self.brain.create_thought(
            spawn_request.content.clone(),
            spawn_request.priority,
        )?;

        // Update context
        let mut processors = self.active_processors.write();
        if let Some(proc) = processors.get_mut(processor_id) {
            proc.spawned_thoughts.push(new_thought_id.clone());
            proc.state = ProcessorState::Spawning;

            // Update parent thought if specified
            if let Some(ref parent_id) = spawn_request.parent_thought_id {
                proc.parent_thought_id = Some(parent_id.clone());
                self.brain.create_association(parent_id, &new_thought_id)?;
            }
        }

        // Set thought context
        {
            let mut thoughts = self.brain.thoughts.write();
            if let Some(thought) = thoughts.get_mut(&new_thought_id) {
                thought.context.extend(spawn_request.context.clone());
                thought.associations.extend(spawn_request.associations.clone());
            }
        }

        // Handle spawn type
        match spawn_request.spawn_type {
            ThoughtSpawnType::Parallel => {
                // Process in parallel - already spawned
                debug!("Spawned parallel thought: {} (reason: {})", new_thought_id, spawn_request.trigger_reason);
            }
            ThoughtSpawnType::Sequential => {
                // Queue for sequential processing
                // In production, would add to sequential queue
                debug!("Spawned sequential thought: {} (reason: {})", new_thought_id, spawn_request.trigger_reason);
            }
            ThoughtSpawnType::Branch => {
                // Branch from parent
                debug!("Spawned branch thought: {} (reason: {})", new_thought_id, spawn_request.trigger_reason);
            }
            _ => {
                debug!("Spawned {:?} thought: {} (reason: {})", spawn_request.spawn_type, new_thought_id, spawn_request.trigger_reason);
            }
        }

        info!("Spawned thought {} during processing: {}", new_thought_id, spawn_request.trigger_reason);

        Ok(new_thought_id)
    }

    /// Register thought trigger (automatic spawning)
    pub fn register_trigger(&self, trigger: ThoughtTrigger) -> Result<String> {
        let trigger_id = trigger.trigger_id.clone();
        let mut triggers = self.thought_triggers.write();
        triggers.insert(trigger_id.clone(), trigger);
        Ok(trigger_id)
    }

    /// Enable automatic thought spawning based on triggers
    pub async fn enable_automatic_spawning(&self) {
        let brain = self.brain.clone();
        let triggers = self.thought_triggers.clone();
        let active_processors = self.active_processors.clone();
        let spawner = Arc::new(self.clone());

        tokio::spawn(async move {
            let mut receiver = brain.subscribe();
            loop {
                match receiver.recv().await {
                    Ok(event) => {
                        // Check triggers
                        let matching_triggers: Vec<_> = {
                            let triggers_guard = triggers.read();
                            triggers_guard.values()
                                .filter(|trigger| trigger.enabled && Self::check_trigger(&event, trigger))
                                .cloned()
                                .collect()
                        };

                        // Spawn thoughts for matching triggers
                        for trigger in matching_triggers {
                            // Find active processor or create context
                            let processor_id = {
                                let processors = active_processors.read();
                                processors.values()
                                    .find(|p| p.state == ProcessorState::Processing)
                                    .map(|p| p.processor_id.clone())
                            };

                            if let Some(proc_id) = processor_id {
                                if let Err(e) = spawner.spawn_thought_during_processing(&proc_id, trigger.spawn_request.clone()) {
                                    tracing::warn!("Failed to spawn thought: {}", e);
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    }

    fn check_trigger(event: &CognitiveEvent, trigger: &ThoughtTrigger) -> bool {
        match (&trigger.trigger_type, &trigger.condition) {
            (TriggerType::OnStateChange, TriggerCondition::StateIs(state)) => {
                matches!(event, CognitiveEvent::ThoughtCompleted { .. })
            }
            (TriggerType::OnMemoryAccess, _) => {
                matches!(event, CognitiveEvent::MemoryRetrieved { .. })
            }
            (TriggerType::OnAssociation, _) => {
                matches!(event, CognitiveEvent::AssociationCreated { .. })
            }
            (TriggerType::Always, _) => true,
            _ => false,
        }
    }

    /// Register spawn callback - called when thought is processed
    pub fn register_spawn_callback<F>(&self, callback: F)
    where
        F: Fn(&Thought, &serde_json::Value) -> Option<ThoughtSpawnRequest> + Send + Sync + 'static,
    {
        self.spawn_callbacks.write().push(Box::new(callback));
    }

    /// Check if thought should spawn additional thoughts
    pub fn check_spawn_triggers(
        &self,
        thought: &Thought,
        content: &serde_json::Value,
    ) -> Vec<ThoughtSpawnRequest> {
        let callbacks = self.spawn_callbacks.read();
        let mut spawns = Vec::new();

        for callback in callbacks.iter() {
            if let Some(spawn_request) = callback(thought, content) {
                spawns.push(spawn_request);
            }
        }

        spawns
    }

    /// Spawn thoughts from context analysis
    pub fn spawn_from_context(
        &self,
        processor_id: &str,
        thought: &Thought,
        content: &serde_json::Value,
    ) -> Result<Vec<String>> {
        let spawns = self.check_spawn_triggers(thought, content);
        let mut spawned_ids = Vec::new();

        for spawn_request in spawns {
            match self.spawn_thought_during_processing(processor_id, spawn_request) {
                Ok(id) => spawned_ids.push(id),
                Err(e) => {
                    debug!("Failed to spawn thought: {}", e);
                }
            }
        }

        Ok(spawned_ids)
    }

    /// Branch thought - create branch thoughts from current thought
    pub fn branch_thought(
        &self,
        processor_id: &str,
        parent_thought_id: &str,
        branches: Vec<ThoughtSpawnRequest>,
    ) -> Result<Vec<String>> {
        let mut branch_ids = Vec::new();

        for branch in branches {
            let mut branch_request = branch;
            branch_request.parent_thought_id = Some(parent_thought_id.to_string());
            branch_request.spawn_type = ThoughtSpawnType::Branch;

            match self.spawn_thought_during_processing(processor_id, branch_request) {
                Ok(id) => branch_ids.push(id),
                Err(e) => {
                    debug!("Failed to create branch: {}", e);
                }
            }
        }

        Ok(branch_ids)
    }

    /// Reflect on thought - spawn reflection thought
    pub fn reflect_on_thought(
        &self,
        processor_id: &str,
        thought_id: &str,
        reflection_content: serde_json::Value,
    ) -> Result<String> {
        let reflection_request = ThoughtSpawnRequest {
            content: reflection_content,
            priority: 0.8,
            trigger_reason: format!("Reflection on thought {}", thought_id),
            parent_thought_id: Some(thought_id.to_string()),
            context: HashMap::new(),
            associations: vec![thought_id.to_string()],
            spawn_type: ThoughtSpawnType::Reflection,
        };

        self.spawn_thought_during_processing(processor_id, reflection_request)
    }

    /// Predict from thought - spawn prediction thought
    pub fn predict_from_thought(
        &self,
        processor_id: &str,
        thought_id: &str,
        prediction_content: serde_json::Value,
    ) -> Result<String> {
        let prediction_request = ThoughtSpawnRequest {
            content: prediction_content,
            priority: 0.7,
            trigger_reason: format!("Prediction from thought {}", thought_id),
            parent_thought_id: Some(thought_id.to_string()),
            context: HashMap::new(),
            associations: vec![thought_id.to_string()],
            spawn_type: ThoughtSpawnType::Prediction,
        };

        self.spawn_thought_during_processing(processor_id, prediction_request)
    }

    /// Form hypothesis from thought
    pub fn form_hypothesis(
        &self,
        processor_id: &str,
        thought_id: &str,
        hypothesis_content: serde_json::Value,
    ) -> Result<String> {
        let hypothesis_request = ThoughtSpawnRequest {
            content: hypothesis_content,
            priority: 0.75,
            trigger_reason: format!("Hypothesis from thought {}", thought_id),
            parent_thought_id: Some(thought_id.to_string()),
            context: HashMap::new(),
            associations: vec![thought_id.to_string()],
            spawn_type: ThoughtSpawnType::Hypothesis,
        };

        self.spawn_thought_during_processing(processor_id, hypothesis_request)
    }

    /// Get spawned thoughts for a processor
    pub fn get_spawned_thoughts(&self, processor_id: &str) -> Vec<String> {
        let processors = self.active_processors.read();
        processors.get(processor_id)
            .map(|p| p.spawned_thoughts.clone())
            .unwrap_or_default()
    }

    /// Get all active processors
    pub fn get_active_processors(&self) -> Vec<String> {
        let processors = self.active_processors.read();
        processors.keys().cloned().collect()
    }
}

impl Clone for DynamicThoughtSpawner {
    fn clone(&self) -> Self {
        Self {
            brain: self.brain.clone(),
            active_processors: self.active_processors.clone(),
            thought_triggers: self.thought_triggers.clone(),
            spawn_callbacks: Arc::new(RwLock::new(Vec::new())), // Don't clone callbacks
        }
    }
}

/// Enhanced thought processor with dynamic spawning
pub struct EnhancedThoughtProcessor {
    brain: Arc<CognitiveBrain>,
    spawner: Arc<DynamicThoughtSpawner>,
}

impl EnhancedThoughtProcessor {
    pub fn new(brain: Arc<CognitiveBrain>) -> Self {
        let spawner = Arc::new(DynamicThoughtSpawner::new(brain.clone()));
        Self { brain, spawner }
    }

    /// Process thought with automatic spawning
    pub async fn process_with_auto_spawning<F>(
        &self,
        thought_id: String,
        processor: F,
    ) -> Result<Vec<String>>
    where
        F: Fn(ThoughtProcessingContext, serde_json::Value) -> Result<serde_json::Value> + Send + Sync + 'static,
    {
        self.spawner.process_with_spawning(thought_id, Box::new(processor)).await
    }

    /// Get spawner
    pub fn spawner(&self) -> Arc<DynamicThoughtSpawner> {
        self.spawner.clone()
    }
}

