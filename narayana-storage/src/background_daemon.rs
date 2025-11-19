// Background Daemon - Unconscious Processes
// Continuous background processing of memories/experiences
// Pattern detection, memory consolidation, association formation

use crate::cognitive::{CognitiveBrain, Memory, Experience, MemoryType, Pattern, PatternType};
use crate::conscience_persistent_loop::CPLEvent;
use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use parking_lot::RwLock;
use tokio::sync::broadcast;
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Background Daemon - Unconscious cognitive processes
pub struct BackgroundDaemon {
    brain: Arc<CognitiveBrain>,
    event_sender: broadcast::Sender<CPLEvent>,
    
    // Processing queues
    memory_queue: Arc<RwLock<Vec<String>>>, // Memory IDs to process
    experience_queue: Arc<RwLock<Vec<String>>>, // Experience IDs to process
    
    // Processing state
    last_memory_consolidation: Arc<RwLock<u64>>,
    last_pattern_detection: Arc<RwLock<u64>>,
    last_association_formation: Arc<RwLock<u64>>,
    
    // Configuration
    consolidation_interval: u64, // Seconds between consolidation cycles
    pattern_detection_interval: u64,
    association_interval: u64,
}

impl BackgroundDaemon {
    /// Create new Background Daemon
    pub fn new(
        brain: Arc<CognitiveBrain>,
        event_sender: broadcast::Sender<CPLEvent>,
    ) -> Self {
        Self {
            brain,
            event_sender,
            memory_queue: Arc::new(RwLock::new(Vec::new())),
            experience_queue: Arc::new(RwLock::new(Vec::new())),
            last_memory_consolidation: Arc::new(RwLock::new(0)),
            last_pattern_detection: Arc::new(RwLock::new(0)),
            last_association_formation: Arc::new(RwLock::new(0)),
            consolidation_interval: 60, // Every minute
            pattern_detection_interval: 30, // Every 30 seconds
            association_interval: 20, // Every 20 seconds
        }
    }
    
    /// Main processing cycle
    pub async fn process(&self) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // 1. Memory consolidation (forgetting curves, strength updates)
        {
            let last = *self.last_memory_consolidation.read();
            if now.saturating_sub(last) >= self.consolidation_interval {
                if let Err(e) = self.consolidate_memories().await {
                    warn!("Memory consolidation error: {}", e);
                } else {
                    *self.last_memory_consolidation.write() = now;
                }
            }
        }
        
        // 2. Pattern detection from experiences
        {
            let last = *self.last_pattern_detection.read();
            if now.saturating_sub(last) >= self.pattern_detection_interval {
                if let Err(e) = self.detect_patterns().await {
                    warn!("Pattern detection error: {}", e);
                } else {
                    *self.last_pattern_detection.write() = now;
                }
            }
        }
        
        // 3. Association formation
        {
            let last = *self.last_association_formation.read();
            if now.saturating_sub(last) >= self.association_interval {
                if let Err(e) = self.form_associations().await {
                    warn!("Association formation error: {}", e);
                } else {
                    *self.last_association_formation.write() = now;
                }
            }
        }
        
        // 4. Process queued items
        self.process_queues().await?;
        
        Ok(())
    }
    
    /// Consolidate memories (update strength, apply forgetting curves)
    async fn consolidate_memories(&self) -> Result<()> {
        let memories = self.brain.memories.read();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Edge case: Handle clock going backwards
        if now == 0 {
            warn!("System time is 0, skipping consolidation");
            return Ok(());
        }
        
        let mut consolidated_count = 0;
        
        // Collect memory updates first
        let mut updates: Vec<(String, f64)> = Vec::new();
        
        for memory in memories.values() {
            // Apply forgetting curve: strength(t) = initial * e^(-decay_rate * t)
            let time_since_access = now.saturating_sub(memory.last_accessed);
            
            // Decay rate based on memory type
            let decay_rate = match memory.memory_type {
                MemoryType::Working => 0.1, // Fast decay
                MemoryType::Episodic => 0.01, // Slow decay
                MemoryType::Semantic => 0.001, // Very slow decay
                MemoryType::LongTerm => 0.0001, // Minimal decay
                _ => 0.01,
            };
            
            // Compute new strength
            // Edge case: Prevent overflow in time calculation
            let hours_since_access = (time_since_access as f64 / 3600.0).min(1e6);
            let decay_factor = (-decay_rate * hours_since_access).exp();
            let new_strength = (memory.strength * decay_factor).max(0.0).min(1.0);
            
            // Memory consolidation: repeated access strengthens
            let consolidation_bonus = if memory.access_count > 0 {
                1.0 + (memory.access_count as f64).ln() * 0.1
            } else {
                1.0
            };
            
            let final_strength = (new_strength * consolidation_bonus).min(1.0).max(0.0);
            updates.push((memory.id.clone(), final_strength));
        }
        
        drop(memories);
        
        // Apply updates
        for (memory_id, strength) in updates {
            if let Err(e) = self.brain.update_memory_strength(&memory_id, strength) {
                warn!("Failed to update memory strength: {}", e);
            } else {
                consolidated_count += 1;
            }
        }
        
        if consolidated_count > 0 {
            debug!("Consolidated {} memories", consolidated_count);
            let _ = self.event_sender.send(CPLEvent::BackgroundProcessCompleted {
                process_type: "memory_consolidation".to_string(),
            });
        }
        
        Ok(())
    }
    
    /// Detect patterns from experiences
    async fn detect_patterns(&self) -> Result<()> {
        // Use existing pattern detection from brain
        match self.brain.detect_patterns_from_experiences() {
            Ok(pattern_ids) => {
                if !pattern_ids.is_empty() {
                    debug!("Detected {} patterns", pattern_ids.len());
                    let _ = self.event_sender.send(CPLEvent::BackgroundProcessCompleted {
                        process_type: "pattern_detection".to_string(),
                    });
                }
                Ok(())
            }
            Err(e) => {
                warn!("Pattern detection failed: {}", e);
                Err(e)
            }
        }
    }
    
    /// Form associations between related memories/thoughts
    async fn form_associations(&self) -> Result<()> {
        let memories = self.brain.memories.read();
        let experiences = self.brain.experiences.read();
        
        let mut associations_formed = 0;
        
        // Find memories with similar content or temporal proximity
        let memory_vec: Vec<(String, Memory)> = memories.values().map(|m| (m.id.clone(), m.clone())).collect();
        
        drop(memories);
        drop(experiences);
        
        // Collect associations to form
        let mut associations_to_form = Vec::new();
        
        for i in 0..memory_vec.len().min(100) { // Limit to prevent O(n²) explosion
            for j in (i + 1)..memory_vec.len().min(100) {
                let (_, mem1) = &memory_vec[i];
                let (_, mem2) = &memory_vec[j];
                
                // Check for similarity
                let similarity = self.compute_memory_similarity(mem1, mem2);
                
                // Check temporal proximity (within 1 hour)
                let time_diff = if mem1.created_at > mem2.created_at {
                    mem1.created_at - mem2.created_at
                } else {
                    mem2.created_at - mem1.created_at
                };
                let temporal_proximity = if time_diff < 3600 {
                    1.0 / (1.0 + time_diff as f64 / 3600.0)
                } else {
                    0.0
                };
                
                // Form association if similarity or temporal proximity is high
                if similarity > 0.7 || temporal_proximity > 0.5 {
                    associations_to_form.push((mem1.id.clone(), mem2.id.clone()));
                }
            }
        }
        
        // Form associations
        for (id1, id2) in associations_to_form {
            if self.brain.create_association(&id1, &id2).is_ok() {
                associations_formed += 1;
            }
        }
        
        if associations_formed > 0 {
            debug!("Formed {} associations", associations_formed);
            let _ = self.event_sender.send(CPLEvent::BackgroundProcessCompleted {
                process_type: "association_formation".to_string(),
            });
        }
        
        Ok(())
    }
    
    /// Compute similarity between two memories
    fn compute_memory_similarity(&self, mem1: &Memory, mem2: &Memory) -> f64 {
        // Check tag overlap
        let mut tag_overlap = 0;
        for tag in &mem1.tags {
            if mem2.tags.contains(tag) {
                tag_overlap += 1;
            }
        }
        let tag_similarity = if !mem1.tags.is_empty() || !mem2.tags.is_empty() {
            // SECURITY: Prevent division by zero
            let denominator = ((mem1.tags.len() + mem2.tags.len()) as f64).max(1.0);
            if denominator > 0.0 {
                (tag_overlap as f64 / denominator).max(0.0).min(1.0)
            } else {
                0.0
            }
        } else {
            0.0
        };
        
        // Check embedding similarity if available
        let embedding_similarity = if let (Some(emb1), Some(emb2)) = (&mem1.embedding, &mem2.embedding) {
            if emb1.len() == emb2.len() {
                self.cosine_similarity(emb1, emb2)
            } else {
                0.0
            }
        } else {
            0.0
        };
        
        // Check memory type match
        let type_match = if mem1.memory_type == mem2.memory_type {
            0.3
        } else {
            0.0
        };
        
        // Combined similarity
        (tag_similarity * 0.4 + embedding_similarity * 0.3 + type_match).min(1.0)
    }
    
    /// Cosine similarity between vectors
    fn cosine_similarity(&self, vec1: &[f32], vec2: &[f32]) -> f64 {
        if vec1.len() != vec2.len() {
            return 0.0;
        }
        
        let dot_product: f32 = vec1.iter().zip(vec2.iter()).map(|(a, b)| a * b).sum();
        let norm1: f32 = vec1.iter().map(|a| a * a).sum::<f32>().sqrt();
        let norm2: f32 = vec2.iter().map(|a| a * a).sum::<f32>().sqrt();
        
        if norm1 > 0.0 && norm2 > 0.0 {
            // SECURITY: Prevent division by zero
            let denominator = norm1 * norm2;
            if denominator > 0.0 && !denominator.is_nan() && !denominator.is_infinite() {
                let similarity = (dot_product / denominator) as f64;
                // SECURITY: Clamp and validate result
                if similarity.is_nan() || similarity.is_infinite() {
                    0.0
                } else {
                    similarity.max(-1.0).min(1.0)
                }
            } else {
                0.0
            }
        } else {
            0.0
        }
    }
    
    /// Process queued items
    async fn process_queues(&self) -> Result<()> {
        // Process memory queue
        {
            let mut queue = self.memory_queue.write();
            while let Some(memory_id) = queue.pop() {
                // Process memory (e.g., update embeddings, strengthen)
                debug!("Processing queued memory: {}", memory_id);
            }
        }
        
        // Process experience queue
        {
            let mut queue = self.experience_queue.write();
            while let Some(exp_id) = queue.pop() {
                // Process experience (e.g., extract patterns)
                debug!("Processing queued experience: {}", exp_id);
            }
        }
        
        Ok(())
    }
    
    /// Queue memory for processing
    /// SECURITY: Prevent unbounded queue growth
    pub fn queue_memory(&self, memory_id: String) {
        const MAX_QUEUE_SIZE: usize = 10000;
        let mut queue = self.memory_queue.write();
        while queue.len() >= MAX_QUEUE_SIZE {
            queue.remove(0);
        }
        queue.push(memory_id);
    }
    
    /// Queue experience for processing
    /// SECURITY: Prevent unbounded queue growth
    pub fn queue_experience(&self, experience_id: String) {
        const MAX_QUEUE_SIZE: usize = 10000;
        let mut queue = self.experience_queue.write();
        while queue.len() >= MAX_QUEUE_SIZE {
            queue.remove(0);
        }
        queue.push(experience_id);
    }
}

