// Cognitive Graph Layer - Concept Graph with Relationships
// Weighted associations, decay, reinforcement for knowledge graph fusion
// Production-ready implementation

use crate::cognitive::*;
use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use parking_lot::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use tracing::{info, debug, warn};
use uuid::Uuid;

/// Cognitive graph - knowledge graph + episodic graph fusion
pub struct CognitiveGraph {
    concepts: Arc<RwLock<HashMap<String, Concept>>>,
    relationships: Arc<RwLock<HashMap<String, Relationship>>>,
    concept_index: Arc<RwLock<HashMap<String, HashSet<String>>>>, // concept -> relationship IDs
    decay_scheduler: Arc<RwLock<DecayScheduler>>,
    reinforcement_engine: Arc<RwLock<ReinforcementEngine>>,
}

impl CognitiveGraph {
    pub fn new() -> Self {
        Self {
            concepts: Arc::new(RwLock::new(HashMap::new())),
            relationships: Arc::new(RwLock::new(HashMap::new())),
            concept_index: Arc::new(RwLock::new(HashMap::new())),
            decay_scheduler: Arc::new(RwLock::new(DecayScheduler::new())),
            reinforcement_engine: Arc::new(RwLock::new(ReinforcementEngine::new())),
        }
    }

    /// Add concept to graph
    /// SECURITY: Prevent unbounded HashMap growth
    pub fn add_concept(&self, concept: Concept) -> Result<String> {
        // SECURITY: Limit number of concepts to prevent memory exhaustion
        const MAX_CONCEPTS: usize = 100_000_000; // Maximum concepts in graph
        
        let concept_id = concept.id.clone();
        let mut concepts = self.concepts.write();
        
        // SECURITY: If graph is too large, reject new concepts
        if concepts.len() >= MAX_CONCEPTS && !concepts.contains_key(&concept_id) {
            return Err(Error::Storage(format!(
                "Concept limit reached: maximum {} concepts allowed",
                MAX_CONCEPTS
            )));
        }
        
        concepts.insert(concept_id.clone(), concept);
        info!("Added concept: {}", concept_id);
        Ok(concept_id)
    }

    /// Create relationship between concepts
    pub fn create_relationship(
        &self,
        from_concept: &str,
        to_concept: &str,
        relationship_type: RelationshipType,
        weight: f64,
    ) -> Result<String> {
        // Verify concepts exist
        {
            let concepts = self.concepts.read();
            if !concepts.contains_key(from_concept) {
                return Err(Error::Storage(format!("Concept {} not found", from_concept)));
            }
            if !concepts.contains_key(to_concept) {
                return Err(Error::Storage(format!("Concept {} not found", to_concept)));
            }
        }

        // Clone relationship_type before moving it into Relationship
        let rel_type_for_log = relationship_type.clone();
        
        // SECURITY: Limit number of relationships to prevent memory exhaustion
        const MAX_RELATIONSHIPS: usize = 1_000_000_000; // Maximum relationships in graph
        
        let relationship_id = Uuid::new_v4().to_string();
        let relationship = Relationship {
            id: relationship_id.clone(),
            from_concept: from_concept.to_string(),
            to_concept: to_concept.to_string(),
            relationship_type,
            weight,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            last_accessed: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            access_count: 0,
        };

        let mut relationships = self.relationships.write();
        
        // SECURITY: If graph is too large, reject new relationships
        if relationships.len() >= MAX_RELATIONSHIPS && !relationships.contains_key(&relationship_id) {
            return Err(Error::Storage(format!(
                "Relationship limit reached: maximum {} relationships allowed",
                MAX_RELATIONSHIPS
            )));
        }
        
        relationships.insert(relationship_id.clone(), relationship.clone());

        // Update index
        {
            let mut index = self.concept_index.write();
            index.entry(from_concept.to_string())
                .or_insert_with(HashSet::new)
                .insert(relationship_id.clone());
            index.entry(to_concept.to_string())
                .or_insert_with(HashSet::new)
                .insert(relationship_id.clone());
        }

        info!("Created relationship: {} -> {} ({:?}, weight: {})", 
              from_concept, to_concept, rel_type_for_log, weight);
        Ok(relationship_id)
    }

    /// Get concepts related to a concept
    pub fn get_related_concepts(&self, concept_id: &str, max_depth: usize) -> Result<Vec<RelatedConcept>> {
        // EDGE CASE: Prevent stack overflow with extremely large max_depth
        // Limit max_depth to reasonable value (1000 should be more than enough for any real graph)
        const MAX_SAFE_DEPTH: usize = 1000;
        let safe_max_depth = max_depth.min(MAX_SAFE_DEPTH);
        
        let mut visited = HashSet::new();
        let mut results = Vec::new();
        self.get_related_recursive(concept_id, 0, safe_max_depth, &mut visited, &mut results)?;
        Ok(results)
    }

    fn get_related_recursive(
        &self,
        concept_id: &str,
        depth: usize,
        max_depth: usize,
        visited: &mut HashSet<String>,
        results: &mut Vec<RelatedConcept>,
    ) -> Result<()> {
        if depth >= max_depth || visited.contains(concept_id) {
            return Ok(());
        }
        visited.insert(concept_id.to_string());

        let relationship_ids = {
            let index = self.concept_index.read();
            index.get(concept_id).cloned().unwrap_or_default()
        };

        let relationships = {
            let rels = self.relationships.read();
            relationship_ids.iter()
                .filter_map(|id| rels.get(id).cloned())
                .collect::<Vec<_>>()
        };

        for relationship in relationships {
            let related_id = if relationship.from_concept == concept_id {
                &relationship.to_concept
            } else {
                &relationship.from_concept
            };

            // Apply decay
            let decayed_weight = self.decay_scheduler.read().apply_decay(&relationship)?;

            results.push(RelatedConcept {
                concept_id: related_id.clone(),
                relationship_type: relationship.relationship_type.clone(),
                weight: decayed_weight,
                depth,
            });

            // Recursive
            self.get_related_recursive(related_id, depth + 1, max_depth, visited, results)?;
        }

        Ok(())
    }

    /// Reinforce relationship (increase weight)
    pub fn reinforce_relationship(&self, relationship_id: &str, reinforcement: f64) -> Result<()> {
        // Validate reinforcement value
        if !reinforcement.is_finite() {
            return Err(Error::Storage("Reinforcement value must be finite".to_string()));
        }
        
        let mut relationships = self.relationships.write();
        if let Some(relationship) = relationships.get_mut(relationship_id) {
            // Clamp weight to valid range [0.0, 1.0]
            relationship.weight = (relationship.weight + reinforcement).clamp(0.0, 1.0);
            relationship.last_accessed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            relationship.access_count += 1;

            // Update reinforcement engine
            self.reinforcement_engine.write().record_reinforcement(relationship_id, reinforcement)?;
        }
        Ok(())
    }

    /// Apply decay to all relationships
    pub fn apply_decay(&self) -> Result<()> {
        // EDGE CASE: Acquire locks in consistent order to prevent deadlock
        // Get decay_scheduler first, then relationships
        let decay_scheduler = self.decay_scheduler.read();
        let mut relationships = self.relationships.write();
        
        // EDGE CASE: Handle potential errors during decay computation
        // Collect errors instead of failing immediately
        let mut errors = Vec::new();
        for relationship in relationships.values_mut() {
            match decay_scheduler.compute_decay(relationship) {
                Ok(decayed) => {
                    relationship.weight = decayed;
                }
                Err(e) => {
                    errors.push(e);
                }
            }
        }
        
        // Return first error if any occurred, otherwise Ok
        if let Some(first_error) = errors.into_iter().next() {
            Err(first_error)
        } else {
            Ok(())
        }
    }

    /// Get concept by ID
    pub fn get_concept(&self, concept_id: &str) -> Option<Concept> {
        self.concepts.read().get(concept_id).cloned()
    }

    /// Search concepts by pattern
    pub fn search_concepts(&self, pattern: &str) -> Vec<Concept> {
        let concepts = self.concepts.read();
        concepts.values()
            .filter(|c| c.name.contains(pattern) || c.description.contains(pattern))
            .cloned()
            .collect()
    }

    /// Get graph statistics
    pub fn get_statistics(&self) -> GraphStatistics {
        let concepts = self.concepts.read();
        let relationships = self.relationships.read();
        
        GraphStatistics {
            total_concepts: concepts.len(),
            total_relationships: relationships.len(),
            average_weight: relationships.values()
                .map(|r| r.weight)
                .sum::<f64>() / relationships.len().max(1) as f64,
        }
    }
}

/// Concept in cognitive graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub id: String,
    pub name: String,
    pub description: String,
    pub concept_type: ConceptType,
    pub properties: HashMap<String, serde_json::Value>,
    pub created_at: u64,
    pub last_accessed: u64,
    pub access_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConceptType {
    Entity,
    Event,
    Property,
    Relation,
    Abstract,
}

/// Relationship between concepts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub id: String,
    pub from_concept: String,
    pub to_concept: String,
    pub relationship_type: RelationshipType,
    pub weight: f64, // 0.0 to 1.0
    pub created_at: u64,
    pub last_accessed: u64,
    pub access_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipType {
    IsA,           // Taxonomic
    PartOf,        // Meronymic
    Causes,        // Causal
    RelatedTo,     // Associative
    SimilarTo,     // Similarity
    OppositeOf,    // Antonymy
    LocatedAt,     // Spatial
    OccursAt,      // Temporal
    Custom(String), // Custom relationship
}

/// Related concept
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedConcept {
    pub concept_id: String,
    pub relationship_type: RelationshipType,
    pub weight: f64,
    pub depth: usize,
}

/// Decay scheduler - implements forgetting curves
struct DecayScheduler {
    decay_rate: f64,
    half_life: Duration,
}

impl DecayScheduler {
    fn new() -> Self {
        Self {
            decay_rate: 0.1, // 10% decay per half-life
            half_life: Duration::from_secs(86400), // 1 day
        }
    }

    fn apply_decay(&self, relationship: &Relationship) -> Result<f64> {
        self.compute_decay(relationship)
    }

    fn compute_decay(&self, relationship: &Relationship) -> Result<f64> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let age = now.saturating_sub(relationship.last_accessed);
        let half_life_secs = self.half_life.as_secs();
        if half_life_secs == 0 {
            return Err(Error::Storage("Half-life cannot be zero".to_string()));
        }
        let half_lives = age as f64 / half_life_secs as f64;
        let decayed = relationship.weight * (1.0 - self.decay_rate).powf(half_lives);
        Ok(decayed.max(0.0).min(1.0)) // Clamp to [0.0, 1.0]
    }
}

/// Reinforcement engine
struct ReinforcementEngine {
    reinforcement_history: HashMap<String, Vec<f64>>,
}

impl ReinforcementEngine {
    fn new() -> Self {
        Self {
            reinforcement_history: HashMap::new(),
        }
    }

    fn record_reinforcement(&mut self, relationship_id: &str, reinforcement: f64) -> Result<()> {
        self.reinforcement_history
            .entry(relationship_id.to_string())
            .or_insert_with(Vec::new)
            .push(reinforcement);
        Ok(())
    }
}

/// Graph statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStatistics {
    pub total_concepts: usize,
    pub total_relationships: usize,
    pub average_weight: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cognitive_graph() {
        let graph = CognitiveGraph::new();
        
        let concept1 = Concept {
            id: "concept1".to_string(),
            name: "Robot".to_string(),
            description: "A robot".to_string(),
            concept_type: ConceptType::Entity,
            properties: HashMap::new(),
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            last_accessed: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            access_count: 0,
        };

        let concept2 = Concept {
            id: "concept2".to_string(),
            name: "Arm".to_string(),
            description: "Robot arm".to_string(),
            concept_type: ConceptType::Entity,
            properties: HashMap::new(),
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            last_accessed: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            access_count: 0,
        };

        graph.add_concept(concept1).unwrap();
        graph.add_concept(concept2).unwrap();

        let rel_id = graph.create_relationship(
            "concept1",
            "concept2",
            RelationshipType::PartOf,
            0.9,
        ).unwrap();

        let related = graph.get_related_concepts("concept1", 1).unwrap();
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].concept_id, "concept2");

        graph.reinforce_relationship(&rel_id, 0.1).unwrap();
        
        let stats = graph.get_statistics();
        assert_eq!(stats.total_concepts, 2);
        assert_eq!(stats.total_relationships, 1);
    }
}

