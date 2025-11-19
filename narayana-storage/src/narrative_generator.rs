// Narrative Generator - Sense of Self
// Continuous narrative construction from experiences
// Identity formation through memory integration

use crate::cognitive::{CognitiveBrain, Memory, Experience, MemoryType};
use crate::conscience_persistent_loop::CPLEvent;
use crate::traits_equations::TraitType;
use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use parking_lot::RwLock;
use tokio::sync::broadcast;
use std::collections::VecDeque;
use tracing::{debug, info};

/// Narrative Generator - Constructs sense of self
pub struct NarrativeGenerator {
    brain: Arc<CognitiveBrain>,
    event_sender: broadcast::Sender<CPLEvent>,
    
    // Current narrative
    narrative: Arc<RwLock<Narrative>>,
    
    // Narrative history
    narrative_history: Arc<RwLock<VecDeque<NarrativeSnapshot>>>,
    
    // Identity markers
    identity_markers: Arc<RwLock<Vec<IdentityMarker>>>,
}

/// Narrative - Continuous story of self
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Narrative {
    pub id: String,
    pub narrative_text: String,
    pub coherence_score: f64,
    pub last_updated: u64,
    pub key_events: Vec<String>, // Memory/experience IDs
    pub themes: Vec<String>,
    pub temporal_span: (u64, u64), // (start, end) timestamps
}

/// Narrative snapshot at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NarrativeSnapshot {
    narrative: Narrative,
    timestamp: u64,
}

/// Identity marker - Persistent aspect of identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityMarker {
    pub id: String,
    pub marker_type: IdentityMarkerType,
    pub content: serde_json::Value,
    pub strength: f64,
    pub first_observed: u64,
    pub last_observed: u64,
    pub frequency: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdentityMarkerType {
    Trait,
    Preference,
    Skill,
    Goal,
    Value,
    Relationship,
}

impl NarrativeGenerator {
    /// Create new Narrative Generator
    pub fn new(
        brain: Arc<CognitiveBrain>,
        event_sender: broadcast::Sender<CPLEvent>,
    ) -> Self {
        let narrative = Narrative {
            id: uuid::Uuid::new_v4().to_string(),
            narrative_text: String::new(),
            coherence_score: 0.0,
            last_updated: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            key_events: Vec::new(),
            themes: Vec::new(),
            temporal_span: (0, 0),
        };
        
        Self {
            brain,
            event_sender,
            narrative: Arc::new(RwLock::new(narrative)),
            narrative_history: Arc::new(RwLock::new(VecDeque::with_capacity(100))),
            identity_markers: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Update narrative (main cycle)
    pub async fn update_narrative(&self) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // 1. Extract key events from recent memories/experiences
        let key_events = self.extract_key_events().await?;
        
        // 2. Update identity markers
        self.update_identity_markers(&key_events).await?;
        
        // 3. Construct narrative from events and markers
        let narrative = self.construct_narrative(&key_events).await?;
        
        // 4. Update narrative
        {
            let mut current = self.narrative.write();
            *current = narrative.clone();
        }
        
        // 5. Save snapshot
        {
            let mut history = self.narrative_history.write();
            history.push_back(NarrativeSnapshot {
                narrative: narrative.clone(),
                timestamp: now,
            });
            
            // Keep history bounded
            // SECURITY: Prevent unbounded growth
            const MAX_HISTORY: usize = 100;
            while history.len() >= MAX_HISTORY {
                history.pop_front();
            }
        }
        
        // 6. Emit event
        let _ = self.event_sender.send(CPLEvent::NarrativeUpdated {
            narrative_id: narrative.id.clone(),
        });
        
        debug!("Narrative updated: {} events, coherence: {:.2}", 
               key_events.len(), narrative.coherence_score);
        
        Ok(())
    }
    
    /// Extract key events from recent memories/experiences
    async fn extract_key_events(&self) -> Result<Vec<String>> {
        let memories = self.brain.memories.read();
        let experiences = self.brain.experiences.read();
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Edge case: Handle clock going backwards
        if now == 0 {
            drop(memories);
            drop(experiences);
            return Ok(Vec::new());
        }
        
        let mut key_events = Vec::new();
        
        // Get recent episodic memories (last 24 hours)
        let recent_threshold = now.saturating_sub(86400);
        
        for memory in memories.values() {
            if memory.memory_type == MemoryType::Episodic {
                if memory.created_at >= recent_threshold {
                    // Score by strength and recency
                    let recency = 1.0 / (1.0 + (now.saturating_sub(memory.created_at)) as f64 / 3600.0);
                    let mut score = memory.strength * recency;
                    
                    // Adjust score based on genetic traits (e.g., curiosity affects what events are noticed)
                    if let Ok(curiosity_trait) = self.brain.get_trait(&TraitType::Curiosity) {
                        score *= (0.5 + curiosity_trait * 0.5);
                    }
                    
                    if score > 0.5 {
                        key_events.push(memory.id.clone());
                    }
                }
            }
        }
        
        // Get recent experiences with high reward
        // Trait-based filtering: curiosity and risk_taking affect what experiences are noticed
        let curiosity_modifier = self.brain.get_trait(&TraitType::Curiosity).unwrap_or(0.5);
        let risk_modifier = self.brain.get_trait(&TraitType::RiskTaking).unwrap_or(0.5);
        
        for experience in experiences.values() {
            if experience.timestamp >= recent_threshold {
                if let Some(reward) = experience.reward {
                    let mut threshold = 0.5;
                    // Higher curiosity = lower threshold (notices more events)
                    threshold *= (1.0 - curiosity_modifier * 0.3);
                    // Higher risk_taking = notices more extreme rewards
                    if reward.abs() > 0.3 {
                        threshold *= (1.0 - risk_modifier * 0.2);
                    }
                    
                    if reward.abs() > threshold {
                        key_events.push(experience.id.clone());
                    }
                }
            }
        }
        
        drop(memories);
        drop(experiences);
        
        // Sort by importance and take top N
        key_events.truncate(20);
        
        Ok(key_events)
    }
    
    /// Update identity markers from events
    async fn update_identity_markers(&self, event_ids: &[String]) -> Result<()> {
        let memories = self.brain.memories.read();
        let experiences = self.brain.experiences.read();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let mut markers = self.identity_markers.write();
        
        // Collect content first
        let mut content_to_process = Vec::new();
        
        for event_id in event_ids {
            // Get event content
            let content = if memories.contains_key(event_id) {
                memories.get(event_id).map(|m| m.content.clone())
            } else if experiences.contains_key(event_id) {
                experiences.get(event_id).map(|e| e.observation.clone())
            } else {
                None
            };
            
            if let Some(content) = content {
                content_to_process.push(content);
            }
        }
        
        drop(memories);
        drop(experiences);
        
        // Process content and update markers
        for content in content_to_process {
            // Extract potential identity markers
            if let Some(marker) = self.extract_identity_marker(&content, now) {
                // Update or create marker
                if let Some(existing) = markers.iter_mut().find(|m| m.marker_type == marker.marker_type) {
                    // Strengthen existing marker
                    let base_strength_increase = 0.1;
                    // Genetic predisposition affects how strongly traits are reinforced
                    let genetic_modifier = if marker.marker_type == IdentityMarkerType::Trait {
                        self.brain.get_trait(&TraitType::LearningRate).unwrap_or(0.5)
                    } else {
                        0.5
                    };
                    let strength_increase = base_strength_increase * (0.5 + genetic_modifier * 0.5);
                    existing.strength = (existing.strength + strength_increase).min(1.0);
                    existing.last_observed = now;
                    existing.frequency += 1;
                } else {
                    // Add new marker with genetic predisposition affecting initial strength
                    let mut new_marker = marker;
                    if new_marker.marker_type == IdentityMarkerType::Trait {
                        if let Ok(genetic_trait) = self.brain.get_trait(&TraitType::SocialAffinity) {
                            new_marker.strength *= (0.5 + genetic_trait * 0.5);
                        }
                    }
                    markers.push(new_marker);
                }
            }
        }
        
        // Decay markers that haven't been observed recently
        for marker in markers.iter_mut() {
            let age = now.saturating_sub(marker.last_observed);
            if age > 86400 * 7 { // 7 days
                marker.strength *= 0.9; // Decay
            }
        }
        
        // Remove weak markers
        markers.retain(|marker| marker.strength > 0.1);
        
        Ok(())
    }
    
    /// Extract identity marker from content
    fn extract_identity_marker(&self, content: &serde_json::Value, now: u64) -> Option<IdentityMarker> {
        // Simple extraction - in production would use NLP/ML
        // For now, create markers based on content structure
        
        if let Some(obj) = content.as_object() {
            // Look for common identity indicators
            for (key, value) in obj {
                let marker_type = match key.as_str() {
                    "trait" | "personality" => Some(IdentityMarkerType::Trait),
                    "preference" | "like" | "dislike" => Some(IdentityMarkerType::Preference),
                    "skill" | "ability" => Some(IdentityMarkerType::Skill),
                    "goal" | "objective" => Some(IdentityMarkerType::Goal),
                    "value" | "principle" => Some(IdentityMarkerType::Value),
                    "relationship" | "connection" => Some(IdentityMarkerType::Relationship),
                    _ => None,
                };
                
                if let Some(mt) = marker_type {
                    return Some(IdentityMarker {
                        id: uuid::Uuid::new_v4().to_string(),
                        marker_type: mt,
                        content: value.clone(),
                        strength: 0.5,
                        first_observed: now,
                        last_observed: now,
                        frequency: 1,
                    });
                }
            }
        }
        
        None
    }
    
    /// Construct narrative from events and identity markers
    async fn construct_narrative(&self, event_ids: &[String]) -> Result<Narrative> {
        let memories = self.brain.memories.read();
        let experiences = self.brain.experiences.read();
        let markers = self.identity_markers.read();
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Build narrative text from events
        let mut narrative_parts = Vec::new();
        let mut timestamps = Vec::new();
        
        for event_id in event_ids {
            let (content, timestamp) = if memories.contains_key(event_id) {
                let mem = &memories[event_id];
                (mem.content.clone(), mem.created_at)
            } else if experiences.contains_key(event_id) {
                let exp = &experiences[event_id];
                (exp.observation.clone(), exp.timestamp)
            } else {
                continue;
            };
            
            timestamps.push(timestamp);
            
            // Convert content to narrative fragment
            let fragment = self.content_to_narrative_fragment(&content);
            narrative_parts.push(fragment);
        }
        
        // Combine narrative parts
        let narrative_text = if narrative_parts.is_empty() {
            "No recent events to narrate.".to_string()
        } else {
            narrative_parts.join(" ")
        };
        
        // Compute coherence score (how well events fit together)
        let coherence_score = self.compute_coherence(event_ids, &markers);
        
        // Determine temporal span
        // SECURITY: Safe unwrap - we check is_empty() first, but use unwrap_or for extra safety
        let temporal_span = if timestamps.is_empty() {
            (now, now)
        } else {
            let min_ts = timestamps.iter().min().copied().unwrap_or(now);
            let max_ts = timestamps.iter().max().copied().unwrap_or(now);
            (min_ts, max_ts)
        };
        
        // Extract themes from markers
        let themes: Vec<String> = markers
            .iter()
            .filter(|m| m.strength > 0.7)
            .map(|m| format!("{:?}", m.marker_type))
            .collect();
        
        drop(memories);
        drop(experiences);
        drop(markers);
        
        Ok(Narrative {
            id: uuid::Uuid::new_v4().to_string(),
            narrative_text,
            coherence_score,
            last_updated: now,
            key_events: event_ids.to_vec(),
            themes,
            temporal_span,
        })
    }
    
    /// Convert content to narrative fragment
    fn content_to_narrative_fragment(&self, content: &serde_json::Value) -> String {
        // Simple conversion - in production would use sophisticated NLP
        if let Some(obj) = content.as_object() {
            if let Some(desc) = obj.get("description") {
                if let Some(desc_str) = desc.as_str() {
                    return desc_str.to_string();
                }
            }
            if let Some(event) = obj.get("event") {
                if let Some(event_str) = event.as_str() {
                    return format!("Experienced: {}", event_str);
                }
            }
        }
        
        format!("Event: {}", content)
    }
    
    /// Compute narrative coherence
    fn compute_coherence(&self, event_ids: &[String], markers: &[IdentityMarker]) -> f64 {
        if event_ids.is_empty() {
            return 0.0;
        }
        
        // Coherence based on:
        // 1. Identity marker consistency
        // SECURITY: Prevent division by zero and filter NaN/Inf
        let marker_coherence = if markers.is_empty() {
            0.5
        } else {
            let sum: f64 = markers.iter().map(|m| {
                let strength = m.strength;
                // SECURITY: Filter NaN/Inf values
                if strength.is_nan() || strength.is_infinite() {
                    0.0
                } else {
                    strength.max(0.0).min(1.0)
                }
            }).sum();
            let count = markers.len() as f64;
            if count > 0.0 {
                (sum / count).max(0.0).min(1.0)
            } else {
                0.5
            }
        };
        
        // 2. Event count (more events = more coherent story)
        let event_coherence = (event_ids.len() as f64 / 10.0).min(1.0);
        
        // Combined coherence
        (marker_coherence * 0.6 + event_coherence * 0.4).min(1.0)
    }
    
    /// Get current narrative
    pub fn get_narrative(&self) -> Narrative {
        self.narrative.read().clone()
    }
    
    /// Get identity markers
    pub fn get_identity_markers(&self) -> Vec<IdentityMarker> {
        self.identity_markers.read().clone()
    }
    
    /// Get narrative history
    pub fn get_narrative_history(&self) -> Vec<NarrativeSnapshot> {
        self.narrative_history.read().iter().cloned().collect()
    }
}

