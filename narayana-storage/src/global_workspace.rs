// Global Workspace Model (GWM)
// Implements Baars' Global Workspace Theory (1988)
// Broadcast workspace for conscious content, competition for access

use crate::cognitive::{CognitiveBrain, Thought, Memory, MemoryType};
use crate::conscience_persistent_loop::CPLEvent;
use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use parking_lot::RwLock;
use tokio::sync::broadcast;
use std::collections::{HashMap, VecDeque};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Global Workspace - Consciousness layer
/// Implements broadcast workspace where thoughts/memories compete for conscious access
pub struct GlobalWorkspace {
    brain: Arc<CognitiveBrain>,
    event_sender: broadcast::Sender<CPLEvent>,
    
    // Broadcast workspace - currently conscious content
    workspace: Arc<RwLock<VecDeque<ConsciousContent>>>,
    
    // Competition scores for access to workspace
    competition_scores: Arc<RwLock<HashMap<String, f64>>>,
    
    // Workspace capacity (limited conscious capacity)
    capacity: usize,
    
    // Integration history (what was broadcast together)
    integration_history: Arc<RwLock<VecDeque<IntegrationEvent>>>,
}

/// Content in the global workspace (conscious)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousContent {
    pub id: String,
    pub content_type: ContentType,
    pub content_id: String, // ID of thought/memory/experience
    pub priority: f64,
    pub salience: f64,
    pub timestamp: u64,
    pub associations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentType {
    Thought,
    Memory,
    Experience,
    Pattern,
    Goal,
}

/// Integration event - records what was broadcast together
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IntegrationEvent {
    content_ids: Vec<String>,
    timestamp: u64,
    integration_strength: f64,
}

impl GlobalWorkspace {
    /// Create new Global Workspace
    pub fn new(
        brain: Arc<CognitiveBrain>,
        event_sender: broadcast::Sender<CPLEvent>,
    ) -> Self {
        Self {
            brain,
            event_sender,
            workspace: Arc::new(RwLock::new(VecDeque::with_capacity(10))),
            competition_scores: Arc::new(RwLock::new(HashMap::new())),
            capacity: 7, // Limited conscious capacity (Miller's Law)
            integration_history: Arc::new(RwLock::new(VecDeque::with_capacity(100))),
        }
    }
    
    /// Process broadcast - main GWM cycle
    pub async fn process_broadcast(&self) -> Result<()> {
        // 1. Compute competition scores for all candidates
        self.compute_competition_scores().await?;
        
        // 2. Select winners (highest scores)
        let winners = self.select_conscious_content().await?;
        
        // 3. Update workspace with new conscious content
        self.update_workspace(winners).await?;
        
        // 4. Broadcast to all systems (integration)
        self.broadcast_to_systems().await?;
        
        // 5. Record integration events
        self.record_integration().await?;
        
        Ok(())
    }
    
    /// Compute competition scores for thoughts/memories
    async fn compute_competition_scores(&self) -> Result<()> {
        let mut scores = self.competition_scores.write();
        scores.clear();
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Edge case: Handle clock going backwards
        if now == 0 {
            warn!("System time is 0, using fallback timestamp");
            return Ok(());
        }
        
        // Score thoughts
        let thoughts = self.brain.thoughts.read();
        for thought in thoughts.values() {
            if thought.state == crate::cognitive::ThoughtState::Active {
                let score = self.compute_thought_score(thought, now);
                scores.insert(thought.id.clone(), score);
            }
        }
        drop(thoughts);
        
        // Score memories (recent, strong, relevant)
        let memories = self.brain.memories.read();
        for memory in memories.values() {
            // Only consider recent or strong memories
            let recency = 1.0 / (1.0 + (now.saturating_sub(memory.last_accessed)) as f64 / 3600.0);
            let strength_score = memory.strength;
            
            if recency > 0.1 || strength_score > 0.5 {
                let score = recency * strength_score * (memory.access_count as f64 + 1.0).ln();
                scores.insert(memory.id.clone(), score);
            }
        }
        drop(memories);
        
        // Score experiences (recent, high reward)
        let experiences = self.brain.experiences.read();
        for experience in experiences.values() {
            let recency = 1.0 / (1.0 + (now.saturating_sub(experience.timestamp)) as f64 / 3600.0);
            let reward_score = experience.reward.unwrap_or(0.0).abs();
            
            if recency > 0.1 || reward_score > 0.5 {
                let score = recency * (reward_score + 1.0);
                scores.insert(experience.id.clone(), score);
            }
        }
        drop(experiences);
        
        Ok(())
    }
    
    /// Compute score for a thought
    fn compute_thought_score(&self, thought: &Thought, now: u64) -> f64 {
        let recency = 1.0 / (1.0 + (now.saturating_sub(thought.updated_at)) as f64 / 60.0);
        let priority = thought.priority;
        let association_bonus = (thought.associations.len() as f64 + 1.0).ln();
        
        // SECURITY: Validate all inputs and clamp result
        let recency_safe = if recency.is_nan() || recency.is_infinite() {
            0.0
        } else {
            recency.max(0.0).min(1.0)
        };
        let priority_safe = if priority.is_nan() || priority.is_infinite() {
            0.0
        } else {
            priority.max(0.0).min(1.0)
        };
        let result = recency_safe * priority_safe * association_bonus;
        if result.is_nan() || result.is_infinite() {
            0.0
        } else {
            result.max(0.0)
        }
    }
    
    /// Select content that wins competition (enters consciousness)
    async fn select_conscious_content(&self) -> Result<Vec<ConsciousContent>> {
        let scores = self.competition_scores.read();
        let mut candidates: Vec<(String, f64, ContentType)> = Vec::new();
        
        // Collect top candidates
        for (id, score) in scores.iter() {
            // Determine content type
            let content_type = if self.brain.thoughts.read().contains_key(id) {
                ContentType::Thought
            } else if self.brain.memories.read().contains_key(id) {
                ContentType::Memory
            } else if self.brain.experiences.read().contains_key(id) {
                ContentType::Experience
            } else {
                continue;
            };
            
            candidates.push((id.clone(), *score, content_type));
        }
        drop(scores);
        
        // Sort by score (highest first)
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Select top N (workspace capacity)
        let winners: Vec<ConsciousContent> = candidates
            .into_iter()
            .take(self.capacity)
            .filter_map(|(id, score, content_type)| {
                self.create_conscious_content(&id, content_type, score).ok()
            })
            .collect();
        
        Ok(winners)
    }
    
    /// Create conscious content from ID
    fn create_conscious_content(
        &self,
        id: &str,
        content_type: ContentType,
        salience: f64,
    ) -> Result<ConsciousContent> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let (priority, associations) = match content_type {
            ContentType::Thought => {
                let thoughts = self.brain.thoughts.read();
                if let Some(thought) = thoughts.get(id) {
                    (thought.priority, thought.associations.clone())
                } else {
                    return Err(Error::Storage("Thought not found".to_string()));
                }
            }
            ContentType::Memory => {
                let memories = self.brain.memories.read();
                if let Some(memory) = memories.get(id) {
                    (memory.strength, memory.associations.clone())
                } else {
                    return Err(Error::Storage("Memory not found".to_string()));
                }
            }
            ContentType::Experience => {
                let experiences = self.brain.experiences.read();
                if let Some(exp) = experiences.get(id) {
                    (exp.reward.unwrap_or(0.0).abs(), vec![])
                } else {
                    return Err(Error::Storage("Experience not found".to_string()));
                }
            }
            _ => (0.0, vec![]),
        };
        
        Ok(ConsciousContent {
            id: Uuid::new_v4().to_string(),
            content_type,
            content_id: id.to_string(),
            priority,
            salience,
            timestamp: now,
            associations,
        })
    }
    
    /// Update workspace with new conscious content
    async fn update_workspace(&self, new_content: Vec<ConsciousContent>) -> Result<()> {
        let mut workspace = self.workspace.write();
        
        // Clear old content (consciousness is transient)
        workspace.clear();
        
        // Add new content
        for content in new_content {
            // Emit broadcast event
            let _ = self.event_sender.send(CPLEvent::GlobalWorkspaceBroadcast {
                content_id: content.content_id.clone(),
                priority: content.priority,
            });
            
            workspace.push_back(content);
        }
        
        debug!("Global workspace updated with {} items", workspace.len());
        Ok(())
    }
    
    /// Broadcast to all systems (integration)
    async fn broadcast_to_systems(&self) -> Result<()> {
        let workspace = self.workspace.read();
        
        // The broadcast happens implicitly through the workspace
        // Other systems can read from workspace to get conscious content
        // This enables integration across cognitive systems
        
        debug!("Broadcasting {} items to systems", workspace.len());
        Ok(())
    }
    
    /// Record integration events (what was conscious together)
    async fn record_integration(&self) -> Result<()> {
        let workspace = self.workspace.read();
        
        if workspace.len() > 1 {
            let content_ids: Vec<String> = workspace
                .iter()
                .map(|c| c.content_id.clone())
                .collect();
            
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            // Compute integration strength (how related are the items)
            let integration_strength = self.compute_integration_strength(&content_ids);
            
            let mut history = self.integration_history.write();
            history.push_back(IntegrationEvent {
                content_ids,
                timestamp: now,
                integration_strength,
            });
            
            // Keep history bounded
            // SECURITY: Prevent unbounded growth
            const MAX_HISTORY: usize = 100;
            while history.len() >= MAX_HISTORY {
                history.pop_front();
            }
        }
        
        Ok(())
    }
    
    /// Compute integration strength (how related are items)
    fn compute_integration_strength(&self, content_ids: &[String]) -> f64 {
        // Count shared associations
        let mut shared_associations = 0;
        let mut total_associations = 0;
        
        for id in content_ids {
            let associations = if self.brain.thoughts.read().contains_key(id) {
                self.brain.thoughts.read().get(id)
                    .map(|t| t.associations.clone())
                    .unwrap_or_default()
            } else if self.brain.memories.read().contains_key(id) {
                self.brain.memories.read().get(id)
                    .map(|m| m.associations.clone())
                    .unwrap_or_default()
            } else {
                vec![]
            };
            
            total_associations += associations.len();
            
            // Check for shared associations with other items
            for other_id in content_ids {
                if other_id != id {
                    let other_associations = if self.brain.thoughts.read().contains_key(other_id) {
                        self.brain.thoughts.read().get(other_id)
                            .map(|t| t.associations.clone())
                            .unwrap_or_default()
                    } else if self.brain.memories.read().contains_key(other_id) {
                        self.brain.memories.read().get(other_id)
                            .map(|m| m.associations.clone())
                            .unwrap_or_default()
                    } else {
                        vec![]
                    };
                    
                    // Count shared associations
                    for assoc in &associations {
                        if other_associations.contains(assoc) {
                            shared_associations += 1;
                        }
                    }
                }
            }
        }
        
        if total_associations > 0 {
            (shared_associations as f64) / (total_associations as f64)
        } else {
            0.0
        }
    }
    
    /// Get current conscious content
    pub fn get_conscious_content(&self) -> Vec<ConsciousContent> {
        self.workspace.read().iter().cloned().collect()
    }
    
    /// Get competition scores
    pub fn get_competition_scores(&self) -> HashMap<String, f64> {
        self.competition_scores.read().clone()
    }
}

