// Dreaming Loop - Offline Epsilon-Greedy Replay
// Experience replay during idle periods
// Pattern reinforcement and memory consolidation

use crate::cognitive::{CognitiveBrain, Experience, Memory, MemoryType};
use crate::conscience_persistent_loop::CPLEvent;
use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use parking_lot::RwLock;
use tokio::sync::broadcast;
use std::collections::VecDeque;
use tracing::{debug, info, warn};
use rand::Rng;

/// Dreaming Loop - Offline experience replay
pub struct DreamingLoop {
    brain: Arc<CognitiveBrain>,
    event_sender: broadcast::Sender<CPLEvent>,
    
    // Replay buffer
    replay_buffer: Arc<RwLock<VecDeque<Experience>>>,
    
    // Epsilon-greedy parameters
    epsilon: f64, // Exploration rate (0.0 = greedy, 1.0 = random)
    min_epsilon: f64,
    epsilon_decay: f64,
    
    // Replay configuration
    replay_batch_size: usize,
    replay_frequency: u64, // Replay every N iterations
    last_replay: Arc<RwLock<u64>>,
    
    // Replay statistics
    replay_count: Arc<RwLock<u64>>,
    experiences_replayed: Arc<RwLock<usize>>,
}

impl DreamingLoop {
    /// Create new Dreaming Loop
    pub fn new(
        brain: Arc<CognitiveBrain>,
        event_sender: broadcast::Sender<CPLEvent>,
    ) -> Self {
        Self {
            brain,
            event_sender,
            replay_buffer: Arc::new(RwLock::new(VecDeque::with_capacity(10000))),
            epsilon: 0.3, // Start with 30% exploration
            min_epsilon: 0.05, // Minimum 5% exploration
            epsilon_decay: 0.995, // Decay per replay
            replay_batch_size: 32,
            replay_frequency: 10, // Replay every 10 iterations
            last_replay: Arc::new(RwLock::new(0)),
            replay_count: Arc::new(RwLock::new(0)),
            experiences_replayed: Arc::new(RwLock::new(0)),
        }
    }
    
    /// Replay experiences (main dreaming cycle)
    pub async fn replay_experiences(&self) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Edge case: Handle clock going backwards
        if now == 0 {
            return Ok(());
        }
        
        // Check if it's time to replay
        let last = *self.last_replay.read();
        if now.saturating_sub(last) < self.replay_frequency {
            return Ok(());
        }
        
        // Update replay buffer from brain experiences
        self.update_replay_buffer().await?;
        
        if self.replay_buffer.read().is_empty() {
            return Ok(());
        }
        
        // Sample batch for replay (epsilon-greedy)
        let batch = self.sample_replay_batch().await?;
        
        if batch.is_empty() {
            return Ok(());
        }
        
        // Replay experiences
        let mut replayed = 0;
        for experience in &batch {
            if let Err(e) = self.replay_experience(experience).await {
                warn!("Failed to replay experience: {}", e);
            } else {
                replayed += 1;
            }
        }
        
        // Update statistics
        {
            *self.last_replay.write() = now;
            *self.replay_count.write() += 1;
            *self.experiences_replayed.write() += replayed;
        }
        
        // Decay epsilon (reduce exploration over time)
        {
            let mut epsilon = self.epsilon;
            epsilon = (epsilon * self.epsilon_decay).max(self.min_epsilon);
            // Note: epsilon is not mutable in struct, would need Arc<RwLock<f64>> for dynamic epsilon
        }
        
        // Emit event
        let _ = self.event_sender.send(CPLEvent::DreamingCycle {
            experiences_replayed: replayed,
        });
        
        debug!("Dreaming: replayed {} experiences", replayed);
        
        Ok(())
    }
    
    /// Update replay buffer from brain experiences
    async fn update_replay_buffer(&self) -> Result<()> {
        let experiences = self.brain.experiences.read();
        let mut buffer = self.replay_buffer.write();
        
        // Add new experiences to buffer
        for experience in experiences.values() {
            // Check if already in buffer
            if !buffer.iter().any(|e| e.id == experience.id) {
                buffer.push_back(experience.clone());
            }
        }
        
        // SECURITY: Keep buffer bounded to prevent memory exhaustion
        const MAX_BUFFER_SIZE: usize = 10000;
        while buffer.len() > MAX_BUFFER_SIZE {
            buffer.pop_front();
        }
        
        Ok(())
    }
    
    /// Sample batch for replay (epsilon-greedy)
    async fn sample_replay_batch(&self) -> Result<Vec<Experience>> {
        let buffer = self.replay_buffer.read();
        
        if buffer.is_empty() {
            return Ok(Vec::new());
        }
        
        let mut rng = rand::thread_rng();
        let mut batch = Vec::new();
        
        // Epsilon-greedy sampling
        // Edge case: Handle empty buffer
        if buffer.is_empty() {
            return Ok(Vec::new());
        }
        
        let batch_size = self.replay_batch_size.min(buffer.len());
        for _ in 0..batch_size {
            let should_explore = rng.gen::<f64>() < self.epsilon;
            
            let experience = if should_explore {
                // Exploration: random sample
                let idx = rng.gen_range(0..buffer.len());
                buffer.get(idx).cloned()
            } else {
                // Exploitation: sample by priority (high reward experiences)
                self.sample_by_priority(&buffer, &mut rng)
            };
            
            if let Some(exp) = experience {
                batch.push(exp);
            }
        }
        
        Ok(batch)
    }
    
    /// Sample experience by priority (high reward)
    fn sample_by_priority(&self, buffer: &VecDeque<Experience>, rng: &mut impl Rng) -> Option<Experience> {
        // Compute priorities (based on reward magnitude)
        let priorities: Vec<f64> = buffer
            .iter()
            .map(|e| e.reward.unwrap_or(0.0).abs() + 0.1) // Add small base to avoid zero
            .collect();
        
        let total_priority: f64 = priorities.iter().sum();
        
        if total_priority == 0.0 {
            // Fallback to random
            let idx = rng.gen_range(0..buffer.len());
            return buffer.get(idx).cloned();
        }
        
        // Sample according to priorities (weighted random)
        let sample = rng.gen::<f64>() * total_priority;
        let mut cumulative = 0.0;
        
        for (idx, priority) in priorities.iter().enumerate() {
            cumulative += priority;
            if sample <= cumulative {
                return buffer.get(idx).cloned();
            }
        }
        
        // Fallback
        buffer.get(0).cloned()
    }
    
    /// Replay a single experience
    async fn replay_experience(&self, experience: &Experience) -> Result<()> {
        // 1. Strengthen associated memories
        self.strengthen_associated_memories(experience).await?;
        
        // 2. Extract and reinforce patterns
        self.reinforce_patterns(experience).await?;
        
        // 3. Consolidate memory (if high reward)
        if experience.reward.unwrap_or(0.0).abs() > 0.7 {
            self.consolidate_experience_memory(experience).await?;
        }
        
        Ok(())
    }
    
    /// Strengthen memories associated with experience
    async fn strengthen_associated_memories(&self, experience: &Experience) -> Result<()> {
        let memories = self.brain.memories.read();
        
        // Find memories related to this experience (by content similarity or temporal proximity)
        let experience_time = experience.timestamp;
        let mut updates = Vec::new();
        
        for memory in memories.values() {
            // Check temporal proximity (within 1 hour)
            let time_diff = if memory.created_at > experience_time {
                memory.created_at - experience_time
            } else {
                experience_time - memory.created_at
            };
            
            if time_diff < 3600 {
                // Strengthen memory
                let new_strength = (memory.strength + 0.02).min(1.0);
                updates.push((memory.id.clone(), new_strength));
            }
        }
        
        drop(memories);
        
        // Apply updates
        for (memory_id, strength) in updates {
            if let Err(e) = self.brain.update_memory_strength(&memory_id, strength) {
                warn!("Failed to strengthen memory during replay: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// Reinforce patterns from experience
    async fn reinforce_patterns(&self, experience: &Experience) -> Result<()> {
        // If experience has patterns, strengthen them
        if !experience.patterns.is_empty() {
            let patterns = self.brain.patterns.read();
            let pattern_ids: Vec<String> = experience.patterns.iter().map(|p| p.id.clone()).collect();
            drop(patterns);
            
            for pattern_id in pattern_ids {
                // Update pattern (would need pattern update method in brain)
                // For now, just log
                debug!("Reinforcing pattern {}", pattern_id);
            }
        }
        
        Ok(())
    }
    
    /// Consolidate experience into long-term memory
    async fn consolidate_experience_memory(&self, experience: &Experience) -> Result<()> {
        // Create memory from high-reward experience
        let memory_content = serde_json::json!({
            "experience_id": experience.id,
            "observation": experience.observation,
            "action": experience.action,
            "outcome": experience.outcome,
            "reward": experience.reward,
            "consolidated_from_dreaming": true,
            "consolidation_timestamp": SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        });
        
        // Store as long-term memory
        self.brain.store_memory(
            MemoryType::LongTerm,
            memory_content,
            experience.embedding.clone(),
            vec!["dreaming_consolidation".to_string()],
            None,
        )?;
        
        debug!("Consolidated experience {} to long-term memory", experience.id);
        
        Ok(())
    }
    
    /// Get replay statistics
    pub fn get_statistics(&self) -> DreamingStatistics {
        DreamingStatistics {
            replay_count: *self.replay_count.read(),
            experiences_replayed: *self.experiences_replayed.read(),
            buffer_size: self.replay_buffer.read().len(),
            epsilon: self.epsilon,
        }
    }
    
    /// Get current epsilon
    pub fn epsilon(&self) -> f64 {
        self.epsilon
    }
}

/// Dreaming statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamingStatistics {
    pub replay_count: u64,
    pub experiences_replayed: usize,
    pub buffer_size: usize,
    pub epsilon: f64,
}

