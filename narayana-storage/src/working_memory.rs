// Working Memory Scratchpad
// Enhanced working memory with capacity limits, fast access, temporal decay
// Implements Baddeley's Working Memory Model (2000) with Miller's 7±2 capacity

use crate::cognitive::{CognitiveBrain, CognitiveState, Memory, MemoryType};
use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use std::collections::VecDeque;
use tracing::{debug, warn};

/// Working Memory Scratchpad - Active cognitive states
/// Limited capacity (7±2 items), fast access, temporary storage
pub struct WorkingMemoryScratchpad {
    brain: Arc<CognitiveBrain>,
    
    // Active cognitive states (scratchpad)
    scratchpad: Arc<RwLock<VecDeque<ScratchpadEntry>>>,
    
    // Capacity limit (Miller's Law: 7±2)
    capacity: usize,
    
    // Temporal decay parameters
    decay_rate: f64, // Decay per second
    access_boost: f64, // Boost from access
}

/// Entry in working memory scratchpad
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScratchpadEntry {
    pub id: String,
    pub content_id: String, // ID of thought/memory/experience
    pub content_type: ScratchpadContentType,
    pub activation: f64, // Current activation level (0.0-1.0)
    pub created_at: u64,
    pub last_accessed: u64,
    pub access_count: u64,
    pub context: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScratchpadContentType {
    Thought,
    Memory,
    Experience,
    Goal,
    Plan,
}

impl WorkingMemoryScratchpad {
    /// Create new Working Memory Scratchpad
    pub fn new(capacity: usize, brain: Arc<CognitiveBrain>) -> Self {
        Self {
            brain,
            scratchpad: Arc::new(RwLock::new(VecDeque::with_capacity(capacity * 2))),
            capacity,
            decay_rate: 0.01, // 1% decay per second
            access_boost: 0.1, // 10% boost per access
        }
    }
    
    /// Update working memory (maintain activation, decay, capacity)
    pub async fn update(&self) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Edge case: Handle clock going backwards
        if now == 0 {
            warn!("System time is 0, skipping update");
            return Ok(());
        }
        
        // Phase 1: Apply decay and collect entries to promote (with lock)
        let entries_to_promote = {
            let mut scratchpad = self.scratchpad.write().await;
            
            // 1. Apply temporal decay to all entries
            for entry in scratchpad.iter_mut() {
                let time_since_access = now.saturating_sub(entry.last_accessed);
                // Edge case: Prevent overflow in time calculation
                let time_seconds = (time_since_access as f64).min(1e6);
                let decay = (self.decay_rate * time_seconds).min(1.0);
                entry.activation = (entry.activation * (1.0 - decay)).max(0.0).min(1.0);
            }
            
            // 2. Remove entries with low activation
            scratchpad.retain(|entry| entry.activation > 0.1);
            
            // 3. Enforce capacity limit (remove lowest activation if over capacity)
            let mut to_promote = Vec::new();
            while scratchpad.len() > self.capacity {
                // Find entry with lowest activation
                let min_idx = scratchpad
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        a.activation.partial_cmp(&b.activation).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(idx, _)| idx);
                
                if let Some(idx) = min_idx {
                    // Clone entry for promotion
                    if let Some(entry) = scratchpad.get(idx).cloned() {
                        to_promote.push(entry);
                        scratchpad.remove(idx);
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            to_promote
        }; // Lock dropped here
        
        // Phase 2: Promote entries to episodic memory (no lock held)
        for entry in &entries_to_promote {
            if let Err(e) = self.promote_to_episodic(entry).await {
                warn!("Failed to promote to episodic: {}", e);
            }
        }
        
        // Phase 3: Sort by activation (re-acquire lock)
        {
            let mut scratchpad = self.scratchpad.write().await;
            let mut entries: Vec<ScratchpadEntry> = scratchpad.drain(..).collect();
            entries.sort_by(|a, b| b.activation.partial_cmp(&a.activation).unwrap_or(std::cmp::Ordering::Equal));
            
            for entry in entries {
                scratchpad.push_back(entry);
            }
        }
        
        Ok(())
    }
    
    /// Add content to working memory
    pub async fn add(&self, content_id: String, content_type: ScratchpadContentType, context: serde_json::Value) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let mut scratchpad = self.scratchpad.write().await;
        
        // Check if already in scratchpad
        if let Some(entry) = scratchpad.iter_mut().find(|e| e.content_id == content_id) {
            // Boost activation
            entry.activation = (entry.activation + self.access_boost).min(1.0);
            entry.last_accessed = now;
            entry.access_count += 1;
            return Ok(());
        }
        
        // Create new entry
        let entry = ScratchpadEntry {
            id: uuid::Uuid::new_v4().to_string(),
            content_id,
            content_type,
            activation: 0.8, // Start with high activation
            created_at: now,
            last_accessed: now,
            access_count: 1,
            context,
        };
        
        // If at capacity, remove lowest activation entry
        if scratchpad.len() >= self.capacity {
            let min_idx = scratchpad
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    a.activation.partial_cmp(&b.activation).unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(idx, _)| idx);
            
            if let Some(idx) = min_idx {
                // Clone the entry before dropping the lock
                let old_entry = scratchpad.get(idx).cloned();
                drop(scratchpad);
                
                // Promote to episodic memory
                if let Some(ref entry_to_promote) = old_entry {
                    if let Err(e) = self.promote_to_episodic(entry_to_promote).await {
                        warn!("Failed to promote to episodic: {}", e);
                    }
                }
                
                // Remove the entry
                let mut scratchpad = self.scratchpad.write().await;
                scratchpad.remove(idx);
                scratchpad.push_back(entry);
            } else {
                scratchpad.push_back(entry);
            }
        } else {
            scratchpad.push_back(entry);
        }
        Ok(())
    }
    
    /// Access content in working memory (boosts activation)
    pub async fn access(&self, content_id: &str) -> Result<Option<ScratchpadEntry>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let mut scratchpad = self.scratchpad.write().await;
        
        // Find entry and clone it first
        let entry_opt = scratchpad.iter().find(|e| e.content_id == content_id).cloned();
        
        if let Some(mut entry) = entry_opt {
            // Boost activation
            entry.activation = (entry.activation + self.access_boost).min(1.0);
            entry.last_accessed = now;
            entry.access_count += 1;
            
            // Remove old and add to front
            scratchpad.retain(|e| e.id != entry.id);
            scratchpad.push_front(entry.clone());
            
            return Ok(Some(entry));
        }
        
        Ok(None)
    }
    
    /// Get all active entries
    pub async fn get_active(&self) -> Vec<ScratchpadEntry> {
        self.scratchpad.read().await.iter().cloned().collect()
    }
    
    /// Get entries sorted by activation
    pub async fn get_by_activation(&self, limit: usize) -> Vec<ScratchpadEntry> {
        let mut entries: Vec<ScratchpadEntry> = self.scratchpad.read().await.iter().cloned().collect();
        entries.sort_by(|a, b| b.activation.partial_cmp(&a.activation).unwrap_or(std::cmp::Ordering::Equal));
        entries.truncate(limit);
        entries
    }
    
    /// Promote entry to episodic memory before removal
    async fn promote_to_episodic(&self, entry: &ScratchpadEntry) -> Result<()> {
        // Convert scratchpad entry to episodic memory
        let content = serde_json::json!({
            "scratchpad_entry": entry.id,
            "content_id": entry.content_id,
            "content_type": format!("{:?}", entry.content_type),
            "context": entry.context,
            "access_count": entry.access_count,
            "promoted_at": SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        });
        
        // Store as episodic memory
        self.brain.store_memory(
            MemoryType::Episodic,
            content,
            None,
            vec!["working_memory".to_string(), "scratchpad".to_string()],
            None,
        )?;
        
        debug!("Promoted scratchpad entry {} to episodic memory", entry.id);
        Ok(())
    }
    
    /// Clear working memory
    pub async fn clear(&self) {
        self.scratchpad.write().await.clear();
    }
    
    /// Get capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }
    
    /// Get current size
    pub async fn size(&self) -> usize {
        self.scratchpad.read().await.len()
    }
}

