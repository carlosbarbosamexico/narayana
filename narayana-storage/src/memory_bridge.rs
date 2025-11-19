// Episodic ↔ Semantic Memory Bridge
// Implements McClelland et al. (1995) complementary learning systems
// Converts episodic memories to semantic knowledge through consolidation

use crate::cognitive::{CognitiveBrain, Memory, MemoryType, Pattern, PatternType};
use crate::working_memory::WorkingMemoryScratchpad;
use crate::conscience_persistent_loop::CPLEvent;
use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use parking_lot::RwLock;
use tokio::sync::broadcast;
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Memory Bridge - Episodic to Semantic conversion
/// Implements hippocampal replay and consolidation
pub struct MemoryBridge {
    brain: Arc<CognitiveBrain>,
    working_memory: Arc<WorkingMemoryScratchpad>,
    event_sender: broadcast::Sender<CPLEvent>,
    
    // Consolidation state
    consolidation_queue: Arc<RwLock<Vec<String>>>, // Episodic memory IDs to consolidate
    consolidation_history: Arc<RwLock<HashMap<String, ConsolidationRecord>>>,
    
    // Pattern extraction
    extracted_patterns: Arc<RwLock<Vec<ExtractedPattern>>>,
    
    // Configuration
    consolidation_threshold: f64, // Activation threshold for consolidation
    replay_frequency: u64, // Replay every N iterations
}

/// Consolidation record
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConsolidationRecord {
    episodic_id: String,
    semantic_id: String,
    timestamp: u64,
    strength: f64,
}

/// Extracted pattern from episodic memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedPattern {
    pub id: String,
    pub pattern_type: PatternType,
    pub conditions: serde_json::Value,
    pub outcome: serde_json::Value,
    pub confidence: f64,
    pub source_memories: Vec<String>,
}

impl MemoryBridge {
    /// Create new Memory Bridge
    pub fn new(
        brain: Arc<CognitiveBrain>,
        working_memory: Arc<WorkingMemoryScratchpad>,
        event_sender: broadcast::Sender<CPLEvent>,
    ) -> Self {
        Self {
            brain,
            working_memory,
            event_sender,
            consolidation_queue: Arc::new(RwLock::new(Vec::new())),
            consolidation_history: Arc::new(RwLock::new(HashMap::new())),
            extracted_patterns: Arc::new(RwLock::new(Vec::new())),
            consolidation_threshold: 0.7, // 70% activation threshold
            replay_frequency: 5, // Replay every 5 iterations
        }
    }
    
    /// Process bridge (main consolidation cycle)
    pub async fn process_bridge(&self) -> Result<()> {
        // 1. Identify episodic memories ready for consolidation
        self.identify_consolidation_candidates().await?;
        
        // 2. Replay episodic memories (hippocampal replay)
        self.replay_episodic_memories().await?;
        
        // 3. Extract patterns from episodic memories
        self.extract_patterns().await?;
        
        // 4. Consolidate episodic to semantic
        self.consolidate_memories().await?;
        
        Ok(())
    }
    
    /// Identify episodic memories ready for consolidation
    async fn identify_consolidation_candidates(&self) -> Result<()> {
        let memories = self.brain.memories.read();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Edge case: Handle clock going backwards
        if now == 0 {
            drop(memories);
            return Ok(());
        }
        
        let mut candidates = Vec::new();
        
        // Find episodic memories that meet consolidation criteria
        for memory in memories.values() {
            if memory.memory_type == MemoryType::Episodic {
                // Check if memory is strong enough and old enough
                let age_hours = (now.saturating_sub(memory.created_at)) as f64 / 3600.0;
                let strength_score = memory.strength;
                let access_score = (memory.access_count as f64 + 1.0).ln() / 10.0;
                
                // Consolidation score
                let consolidation_score = strength_score * (1.0 + access_score) * (1.0 + age_hours / 24.0).min(2.0);
                
                if consolidation_score >= self.consolidation_threshold {
                    // Check if not already consolidated
                    let history = self.consolidation_history.read();
                    if !history.contains_key(&memory.id) {
                        drop(history);
                        candidates.push(memory.id.clone());
                    }
                }
            }
        }
        
        drop(memories);
        
        // Add to consolidation queue
        // SECURITY: Prevent unbounded queue growth
        const MAX_QUEUE_SIZE: usize = 1000;
        if !candidates.is_empty() {
            let mut queue = self.consolidation_queue.write();
            for candidate in candidates {
                if !queue.contains(&candidate) {
                    // SECURITY: Enforce queue size limit
                    while queue.len() >= MAX_QUEUE_SIZE {
                        queue.remove(0);
                    }
                    queue.push(candidate);
                }
            }
            debug!("Identified {} consolidation candidates", queue.len());
        }
        
        Ok(())
    }
    
    /// Replay episodic memories (hippocampal replay simulation)
    async fn replay_episodic_memories(&self) -> Result<()> {
        // Collect all data while holding lock, then drop it
        let (updates, working_mem_adds, replay_count) = {
            let memories = self.brain.memories.read();
            
            // Select episodic memories for replay (clone to avoid lifetime issues)
            let mut replay_candidates: Vec<Memory> = memories
                .values()
                .filter(|m| m.memory_type == MemoryType::Episodic)
                .cloned()
                .collect();
            
            // Sort by replay priority (strength * recency * access_count)
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            replay_candidates.sort_by(|a, b| {
                let score_a = a.strength * (1.0 / (1.0 + (now.saturating_sub(a.last_accessed)) as f64 / 3600.0)) * (a.access_count as f64 + 1.0).ln();
                let score_b = b.strength * (1.0 / (1.0 + (now.saturating_sub(b.last_accessed)) as f64 / 3600.0)) * (b.access_count as f64 + 1.0).ln();
                score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
            });
            
            // Collect memory updates and working memory additions
            let replay_count = replay_candidates.len().min(10);
            let mut updates = Vec::new();
            let mut working_mem_adds = Vec::new();
            
            for memory in replay_candidates.iter().take(replay_count) {
                // Strengthen memory through replay
                let new_strength = (memory.strength + 0.05).min(1.0);
                updates.push((memory.id.clone(), new_strength));
                
                // Add to working memory temporarily
                let context = serde_json::json!({
                    "replay": true,
                    "original_strength": memory.strength,
                    "new_strength": new_strength,
                });
                
                working_mem_adds.push((
                    memory.id.clone(),
                    crate::working_memory::ScratchpadContentType::Memory,
                    context,
                ));
            }
            (updates, working_mem_adds, replay_count)
        }; // Lock dropped here
        
        // Apply updates
        for (memory_id, strength) in updates {
            if let Err(e) = self.brain.update_memory_strength(&memory_id, strength) {
                warn!("Failed to strengthen memory during replay: {}", e);
            }
        }
        
        // Add to working memory (no lock held)
        for (id, content_type, context) in working_mem_adds {
            if let Err(e) = self.working_memory.add(id, content_type, context).await {
                warn!("Failed to add memory to working memory during replay: {}", e);
            }
        }
        debug!("Replayed {} episodic memories", replay_count);
        
        Ok(())
    }
    
    /// Extract patterns from episodic memories
    async fn extract_patterns(&self) -> Result<()> {
        // Collect all data while holding lock
        let (clusters, memory_data) = {
            let memories = self.brain.memories.read();
            
            // Group episodic memories by similarity (clone to avoid lifetime issues)
            let episodic_memories: Vec<Memory> = memories
                .values()
                .filter(|m| m.memory_type == MemoryType::Episodic)
                .cloned()
                .collect();
            
            if episodic_memories.len() < 3 {
                // Need at least 3 memories to extract patterns
                return Ok(());
            }
            
            // Find clusters of similar memories
            let mut clusters: Vec<Vec<String>> = Vec::new();
            let mut processed = std::collections::HashSet::new();
            
            for i in 0..episodic_memories.len().min(50) { // Limit to prevent O(n²)
                if processed.contains(&episodic_memories[i].id) {
                    continue;
                }
                
                let mut cluster = vec![episodic_memories[i].id.clone()];
                processed.insert(episodic_memories[i].id.clone());
                
                // Find similar memories
                for j in (i + 1)..episodic_memories.len().min(50) {
                    if processed.contains(&episodic_memories[j].id) {
                        continue;
                    }
                    
                    let similarity = self.compute_memory_similarity(&episodic_memories[i], &episodic_memories[j]);
                    if similarity > 0.6 {
                        cluster.push(episodic_memories[j].id.clone());
                        processed.insert(episodic_memories[j].id.clone());
                    }
                }
                
                if cluster.len() >= 3 {
                    clusters.push(cluster);
                }
            }
            
            // Clone memory data
            let memory_data: HashMap<String, Memory> = memories
                .iter()
                .map(|(id, mem)| (id.clone(), mem.clone()))
                .collect();
            
            (clusters, memory_data)
        }; // Lock dropped here
        
        // Extract patterns from clusters (no lock held)
        for cluster in clusters {
            if let Ok(pattern) = self.extract_pattern_from_cluster_with_data(&cluster, &memory_data).await {
                let mut patterns = self.extracted_patterns.write();
                patterns.push(pattern);
                debug!("Extracted pattern from {} memories", cluster.len());
            }
        }
        
        Ok(())
    }
    
    /// Extract pattern from a cluster of similar memories
    async fn extract_pattern_from_cluster_with_data(
        &self,
        memory_ids: &[String],
        memory_data: &HashMap<String, Memory>,
    ) -> Result<ExtractedPattern> {
        
        // Analyze commonalities in memory cluster
        let mut common_tags = std::collections::HashSet::new();
        let mut all_tags = std::collections::HashSet::new();
        let mut content_samples = Vec::new();
        
        for id in memory_ids {
            if let Some(memory) = memory_data.get(id) {
                for tag in &memory.tags {
                    all_tags.insert(tag.clone());
                    if memory_ids.iter().all(|mid| {
                        memory_data.get(mid).map(|m| m.tags.contains(tag)).unwrap_or(false)
                    }) {
                        common_tags.insert(tag.clone());
                    }
                }
                content_samples.push(memory.content.clone());
            }
        }
        
        // Create pattern from commonalities
        let conditions = serde_json::json!({
            "tags": common_tags.iter().collect::<Vec<_>>(),
            "memory_count": memory_ids.len(),
        });
        
        let outcome = serde_json::json!({
            "pattern_type": "semantic_generalization",
            "source_memories": memory_ids,
        });
        
        let confidence = (memory_ids.len() as f64 / 10.0).min(1.0);
        
        Ok(ExtractedPattern {
            id: uuid::Uuid::new_v4().to_string(),
            pattern_type: PatternType::Associative,
            conditions,
            outcome,
            confidence,
            source_memories: memory_ids.to_vec(),
        })
    }
    
    /// Consolidate episodic memories to semantic
    async fn consolidate_memories(&self) -> Result<()> {
        let mut queue = self.consolidation_queue.write();
        let mut consolidated = 0;
        
        while let Some(episodic_id) = queue.pop() {
            let memories = self.brain.memories.read();
            
            if let Some(episodic_memory) = memories.get(&episodic_id) {
                // Clone memory data before dropping the lock
                let memory_clone = episodic_memory.clone();
                drop(memories);
                
                // Create semantic memory from episodic
                let semantic_content = serde_json::json!({
                    "consolidated_from": episodic_id,
                    "original_content": memory_clone.content,
                    "consolidation_timestamp": SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    "generalization": self.generalize_content(&memory_clone.content),
                });
                
                // Store as semantic memory
                match self.brain.store_memory(
                    MemoryType::Semantic,
                    semantic_content,
                    memory_clone.embedding.clone(),
                    memory_clone.tags.clone(),
                    None,
                ) {
                    Ok(semantic_id) => {
                        // Record consolidation
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        
                        let mut history = self.consolidation_history.write();
                        history.insert(episodic_id.clone(), ConsolidationRecord {
                            episodic_id: episodic_id.clone(),
                            semantic_id,
                            timestamp: now,
                            strength: memory_clone.strength,
                        });
                        
                        consolidated += 1;
                        
                        // Emit event
                        let _ = self.event_sender.send(CPLEvent::MemoryConsolidated {
                            memory_id: episodic_id,
                        });
                    }
                    Err(e) => {
                        warn!("Failed to consolidate memory: {}", e);
                        // Put back in queue
                        queue.push(episodic_id);
                    }
                }
            }
        }
        
        if consolidated > 0 {
            info!("Consolidated {} episodic memories to semantic", consolidated);
        }
        
        Ok(())
    }
    
    /// Generalize content (remove specific details, keep general patterns)
    fn generalize_content(&self, content: &serde_json::Value) -> serde_json::Value {
        // Simple generalization: extract key concepts
        // In production, would use more sophisticated NLP/ML
        serde_json::json!({
            "generalized": true,
            "original": content,
        })
    }
    
    /// Compute similarity between two memories
    fn compute_memory_similarity(&self, mem1: &Memory, mem2: &Memory) -> f64 {
        // Tag overlap
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
        
        // Embedding similarity
        let embedding_similarity = if let (Some(emb1), Some(emb2)) = (&mem1.embedding, &mem2.embedding) {
            if emb1.len() == emb2.len() {
                self.cosine_similarity(emb1, emb2)
            } else {
                0.0
            }
        } else {
            0.0
        };
        
        // Combined similarity
        (tag_similarity * 0.5 + embedding_similarity * 0.5).min(1.0)
    }
    
    /// Cosine similarity
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
    
    /// Get consolidation history
    pub fn get_consolidation_history(&self) -> HashMap<String, ConsolidationRecord> {
        self.consolidation_history.read().clone()
    }
    
    /// Get extracted patterns
    pub fn get_extracted_patterns(&self) -> Vec<ExtractedPattern> {
        self.extracted_patterns.read().clone()
    }
}

