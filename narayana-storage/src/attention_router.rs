// Attention Router
// Priority-based attention allocation, salience computation, focus management

use crate::cognitive::{CognitiveBrain, Thought, Memory, ThoughtState};
use crate::conscience_persistent_loop::CPLEvent;
use crate::traits_equations::TraitType;
use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use parking_lot::RwLock;
use tokio::sync::broadcast;
use std::collections::HashMap;
use tracing::{debug, info};

/// Attention Router - Allocates cognitive resources
pub struct AttentionRouter {
    brain: Arc<CognitiveBrain>,
    event_sender: broadcast::Sender<CPLEvent>,
    
    // Attention allocation
    attention_weights: Arc<RwLock<HashMap<String, f64>>>, // ID -> attention weight
    current_focus: Arc<RwLock<Option<String>>>, // Currently focused item
    
    // Salience computation
    salience_cache: Arc<RwLock<HashMap<String, f64>>>, // Cached salience scores
    
    // Attention history
    attention_history: Arc<RwLock<Vec<AttentionShift>>>,
}

/// Attention shift record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionShift {
    pub from: Option<String>,
    pub to: String,
    pub timestamp: u64,
    pub salience: f64,
}

impl AttentionRouter {
    /// Create new Attention Router
    pub fn new(
        brain: Arc<CognitiveBrain>,
        event_sender: broadcast::Sender<CPLEvent>,
    ) -> Self {
        Self {
            brain,
            event_sender,
            attention_weights: Arc::new(RwLock::new(HashMap::new())),
            current_focus: Arc::new(RwLock::new(None)),
            salience_cache: Arc::new(RwLock::new(HashMap::new())),
            attention_history: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Route attention (main cycle)
    pub async fn route_attention(&self) -> Result<()> {
        // 1. Compute salience for all candidates
        self.compute_salience().await?;
        
        // 2. Allocate attention weights
        self.allocate_attention().await?;
        
        // 3. Update focus (shift if needed)
        self.update_focus().await?;
        
        Ok(())
    }
    
    /// Compute salience for thoughts/memories
    async fn compute_salience(&self) -> Result<()> {
        let mut salience = self.salience_cache.write();
        salience.clear();
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Edge case: Handle clock going backwards
        if now == 0 {
            return Ok(());
        }
        
        // Compute salience for thoughts
        let thoughts = self.brain.thoughts.read();
        for thought in thoughts.values() {
            if thought.state == ThoughtState::Active {
                let score = self.compute_thought_salience(thought, now);
                salience.insert(thought.id.clone(), score);
            }
        }
        drop(thoughts);
        
        // Compute salience for memories
        let memories = self.brain.memories.read();
        for memory in memories.values() {
            let score = self.compute_memory_salience(memory, now);
            if score > 0.1 {
                salience.insert(memory.id.clone(), score);
            }
        }
        drop(memories);
        
        Ok(())
    }
    
    /// Compute salience for a thought
    fn compute_thought_salience(&self, thought: &Thought, now: u64) -> f64 {
        // Salience factors:
        // 1. Priority (explicit importance)
        let priority_score = thought.priority;
        
        // 2. Recency (recently updated)
        let recency = 1.0 / (1.0 + (now.saturating_sub(thought.updated_at)) as f64 / 60.0);
        
        // 3. Association count (connectedness)
        let association_score = (thought.associations.len() as f64 + 1.0).ln() / 5.0;
        
        // 4. Memory access count (active processing)
        let access_score = (thought.memory_accesses.len() as f64 + 1.0).ln() / 10.0;
        
        // Combined salience
        // SECURITY: Validate all inputs and clamp result
        let priority_safe = if priority_score.is_nan() || priority_score.is_infinite() {
            0.0
        } else {
            priority_score.max(0.0).min(1.0)
        };
        let recency_safe = if recency.is_nan() || recency.is_infinite() {
            0.0
        } else {
            recency.max(0.0).min(1.0)
        };
        let mut result = (priority_safe * 0.4 + recency_safe * 0.3 + association_score * 0.2 + access_score * 0.1);
        
        // Apply trait modifiers: attention_span and curiosity affect thought salience
        if let Ok(attention_trait) = self.brain.get_trait(&TraitType::AttentionSpan) {
            // Higher attention span = better focus = higher salience for important thoughts
            result *= (0.7 + attention_trait * 0.3);
        }
        if let Ok(curiosity_trait) = self.brain.get_trait(&TraitType::Curiosity) {
            // Higher curiosity = notices more novel thoughts = slight boost
            result *= (1.0 + curiosity_trait * 0.1);
        }
        
        if result.is_nan() || result.is_infinite() {
            0.0
        } else {
            result.max(0.0).min(1.0)
        }
    }
    
    /// Compute salience for a memory
    fn compute_memory_salience(&self, memory: &Memory, now: u64) -> f64 {
        // Salience factors:
        // 1. Strength (memory strength)
        let strength_score = memory.strength;
        
        // 2. Recency (recently accessed)
        let recency = 1.0 / (1.0 + (now.saturating_sub(memory.last_accessed)) as f64 / 3600.0);
        
        // 3. Access frequency
        let access_score = (memory.access_count as f64 + 1.0).ln() / 10.0;
        
        // 4. Association count
        let association_score = (memory.associations.len() as f64 + 1.0).ln() / 5.0;
        
        // Combined salience
        // SECURITY: Validate all inputs and clamp result
        let strength_safe = if strength_score.is_nan() || strength_score.is_infinite() {
            0.0
        } else {
            strength_score.max(0.0).min(1.0)
        };
        let recency_safe = if recency.is_nan() || recency.is_infinite() {
            0.0
        } else {
            recency.max(0.0).min(1.0)
        };
        let mut result = (strength_safe * 0.4 + recency_safe * 0.3 + access_score * 0.2 + association_score * 0.1);
        
        // Apply trait modifiers: memory_capacity affects memory salience
        if let Ok(memory_trait) = self.brain.get_trait(&TraitType::MemoryCapacity) {
            // Higher memory capacity = better memory encoding = higher salience
            result *= (0.7 + memory_trait * 0.3);
        }
        
        if result.is_nan() || result.is_infinite() {
            0.0
        } else {
            result.max(0.0).min(1.0)
        }
    }
    
    /// Allocate attention weights based on salience
    async fn allocate_attention(&self) -> Result<()> {
        let salience = self.salience_cache.read();
        let mut weights = self.attention_weights.write();
        weights.clear();
        
        // Normalize salience scores to attention weights (softmax-like)
        // SECURITY: Filter NaN/Inf values and prevent division by zero
        let total_salience: f64 = salience.values()
            .map(|&s| {
                if s.is_nan() || s.is_infinite() || s < 0.0 {
                    0.0
                } else {
                    s
                }
            })
            .sum();
        
        // SECURITY: Prevent division by zero
        if total_salience > 0.0 && !total_salience.is_nan() && !total_salience.is_infinite() {
            for (id, salience_score) in salience.iter() {
                // SECURITY: Validate salience_score before division
                let safe_score = if salience_score.is_nan() || salience_score.is_infinite() || *salience_score < 0.0 {
                    0.0
                } else {
                    *salience_score
                };
                
                // Softmax normalization
                let weight = safe_score / total_salience;
                // Clamp to valid range
                let weight = weight.max(0.0).min(1.0);
                weights.insert(id.clone(), weight);
            }
        }
        
        Ok(())
    }
    
    /// Update focus (shift attention to highest salience item)
    async fn update_focus(&self) -> Result<()> {
        let salience = self.salience_cache.read();
        let mut current_focus = self.current_focus.write();
        
        // Find item with highest salience
        let new_focus = salience
            .iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(id, _)| id.clone());
        
        // Check if focus shifted
        let old_focus_clone = current_focus.clone();
        
        if new_focus != *current_focus {
            *current_focus = new_focus.clone();
            
            // Record attention shift
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            if let Some(ref new_id) = new_focus {
                let salience_score = salience.get(new_id).copied().unwrap_or(0.0);
                
                let from_id = old_focus_clone.clone().unwrap_or_default();
                
                let mut history = self.attention_history.write();
                history.push(AttentionShift {
                    from: old_focus_clone.clone(),
                    to: new_id.clone(),
                    timestamp: now,
                    salience: salience_score,
                });
                
                // Keep history bounded
                // SECURITY: Prevent unbounded growth
                const MAX_HISTORY: usize = 1000;
                while history.len() >= MAX_HISTORY {
                    history.remove(0);
                }
                
                drop(history);
                
                // Emit event
                let _ = self.event_sender.send(CPLEvent::AttentionShifted {
                    from: from_id,
                    to: new_id.clone(),
                });
                
                debug!("Attention shifted to {}", new_id);
            }
        }
        
        Ok(())
    }
    
    /// Get current focus
    pub fn get_current_focus(&self) -> Option<String> {
        self.current_focus.read().clone()
    }
    
    /// Get attention weights
    pub fn get_attention_weights(&self) -> HashMap<String, f64> {
        self.attention_weights.read().clone()
    }
    
    /// Get salience scores
    pub fn get_salience_scores(&self) -> HashMap<String, f64> {
        self.salience_cache.read().clone()
    }
    
    /// Get attention history
    pub fn get_attention_history(&self) -> Vec<AttentionShift> {
        self.attention_history.read().clone()
    }
}

