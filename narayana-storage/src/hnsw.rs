// HNSW (Hierarchical Navigable Small World) - Fast approximate nearest neighbor search
// Implementation based on the HNSW paper by Malkov and Yashunin

use narayana_core::{Error, Result};
use std::collections::{HashMap, HashSet, BinaryHeap};
use std::cmp::Ordering;
use rand::Rng;
use parking_lot::RwLock;

/// HNSW index for approximate nearest neighbor search
pub struct HNSWIndex {
    /// Maximum number of connections per element at each layer
    m: usize,
    /// Size of dynamic candidate list during search
    ef_construction: usize,
    /// Maximum layer for the index
    max_layer: RwLock<usize>,
    /// Entry point (top layer)
    entry_point: RwLock<Option<u64>>,
    /// Layers: layer -> node_id -> neighbors
    layers: RwLock<Vec<RwLock<HashMap<u64, Vec<u64>>>>>,
    /// All vectors: node_id -> vector
    vectors: RwLock<HashMap<u64, Vec<f32>>>,
    /// Dimension of vectors
    dimension: usize,
    /// Layer assignment for each node
    node_layers: RwLock<HashMap<u64, usize>>,
}

#[derive(Debug, Clone)]
struct Candidate {
    id: u64,
    distance: f32,
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance
    }
}

impl Eq for Candidate {}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Reverse order for max-heap (we want closest first)
        other.distance.partial_cmp(&self.distance)
    }
}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

impl HNSWIndex {
    /// Create new HNSW index
    pub fn new(m: usize, ef_construction: usize, dimension: usize) -> Self {
        Self {
            m,
            ef_construction,
            max_layer: RwLock::new(0),
            entry_point: RwLock::new(None),
            layers: RwLock::new(vec![RwLock::new(HashMap::new())]), // Layer 0 always exists
            vectors: RwLock::new(HashMap::new()),
            dimension,
            node_layers: RwLock::new(HashMap::new()),
        }
    }

    /// Generate random layer level (exponential distribution)
    fn random_layer(&self) -> usize {
        let mut rng = rand::thread_rng();
        let mut layer = 0;
        while rng.gen::<f64>() < 0.5 && layer < 16 {
            layer += 1;
        }
        layer
    }

    /// Calculate cosine distance (1 - cosine similarity)
    fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            1.0
        } else {
            1.0 - (dot_product / (norm_a * norm_b))
        }
    }

    /// Search for nearest neighbors in a specific layer
    fn search_layer(
        &self,
        query: &[f32],
        entry_points: &[u64],
        ef: usize,
        layer: usize,
    ) -> Vec<Candidate> {
        {
            let layers = self.layers.read();
            if layer >= layers.len() {
                return Vec::new();
            }
        }

        let mut visited = HashSet::new();
        let mut candidates = BinaryHeap::new();
        let mut dynamic_candidates = BinaryHeap::new();

        // Initialize with entry points
        for &ep in entry_points {
            if !visited.contains(&ep) {
                visited.insert(ep);
                if let Some(vector) = self.vectors.read().get(&ep) {
                    let dist = Self::cosine_distance(query, vector);
                    let candidate = Candidate { id: ep, distance: dist };
                    candidates.push(candidate.clone());
                    dynamic_candidates.push(candidate);
                }
            }
        }

        // Expand search
        while let Some(current) = dynamic_candidates.pop() {
            // Check if we should stop - only if we have candidates and distance is worse
            if candidates.len() >= ef {
                if let Some(best_candidate) = candidates.peek() {
                    if current.distance > best_candidate.distance {
                        break;
                    }
                } else {
                    // No candidates yet, continue
                }
            }

            // Get layer connections - drop lock quickly to avoid holding multiple locks
            let neighbors_opt = {
                let layers = self.layers.read();
                if layer < layers.len() {
                    let layer_connections = layers[layer].read();
                    layer_connections.get(&current.id).cloned()
                } else {
                    None
                }
            };
            
            if let Some(neighbors) = neighbors_opt {
                for &neighbor_id in &neighbors {
                    if !visited.contains(&neighbor_id) {
                        visited.insert(neighbor_id);
                        if let Some(vector) = self.vectors.read().get(&neighbor_id) {
                            let dist = Self::cosine_distance(query, vector);
                            let candidate = Candidate {
                                id: neighbor_id,
                                distance: dist,
                            };

                            if candidates.len() < ef {
                                candidates.push(candidate.clone());
                                dynamic_candidates.push(candidate);
                            } else if let Some(best_candidate) = candidates.peek() {
                                if dist < best_candidate.distance {
                                    candidates.pop();
                                    candidates.push(candidate.clone());
                                    dynamic_candidates.push(candidate);
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut sorted: Vec<Candidate> = candidates.into_vec();
        sorted.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(Ordering::Equal));
        sorted
    }

    /// Select neighbors using heuristic (keep closest M neighbors)
    fn select_neighbors_heuristic(
        &self,
        candidates: &[Candidate],
        m: usize,
    ) -> Vec<u64> {
        // Simple: just take the closest M
        candidates
            .iter()
            .take(m.min(candidates.len()))
            .map(|c| c.id)
            .collect()
    }

    /// Insert a vector into the index
    pub fn insert(&self, id: u64, vector: Vec<f32>) -> Result<()> {
        if vector.len() != self.dimension {
            return Err(Error::Storage(format!(
                "Vector dimension {} doesn't match index dimension {}",
                vector.len(),
                self.dimension
            )));
        }

        // Store vector
        self.vectors.write().insert(id, vector.clone());

        // Determine layer for this node
        let node_layer = self.random_layer();
        self.node_layers.write().insert(id, node_layer);

        // Ensure we have enough layers
        {
            let mut layers = self.layers.write();
            while layers.len() <= node_layer {
                layers.push(RwLock::new(HashMap::new()));
            }
        }

        // If this is the first node or it's at a higher layer, update entry point
        let mut max_layer = *self.max_layer.read();
        let mut entry_point = self.entry_point.read().clone();
        if entry_point.is_none() || node_layer > max_layer {
            max_layer = node_layer;
            entry_point = Some(id);
            *self.max_layer.write() = max_layer;
            *self.entry_point.write() = entry_point;
        }

        // Search for entry point at top layer
        let mut current_closest = if let Some(ep) = *self.entry_point.read() {
            if ep == id {
                return Ok(()); // Already inserted
            }
            vec![ep]
        } else {
            return Ok(());
        };

        // Search from top layer down to node_layer + 1
        let max_layer = *self.max_layer.read();
        for layer in (node_layer + 1..=max_layer).rev() {
            let candidates = self.search_layer(&vector, &current_closest, 1, layer);
            if let Some(closest) = candidates.first() {
                current_closest = vec![closest.id];
            }
        }

        // Insert at each layer from node_layer down to 0
        let max_layer = *self.max_layer.read();
        for layer in (0..=node_layer).rev() {
            // Search for candidates
            let candidates = self.search_layer(&vector, &current_closest, self.ef_construction, layer);

            // Select neighbors
            let neighbors = self.select_neighbors_heuristic(&candidates, self.m);

            // Add connections
            {
                let layers = self.layers.read();
                let mut layer_map = layers[layer].write();
                layer_map.insert(id, neighbors.clone());
            }

            // Add reverse connections and prune
            for &neighbor_id in &neighbors {
                if neighbor_id == id {
                    continue;
                }
                let layers = self.layers.read();
                let mut layer_map = layers[layer].write();
                let neighbor_neighbors = layer_map.entry(neighbor_id).or_insert_with(Vec::new);
                
                // Add connection if not already present
                if !neighbor_neighbors.contains(&id) {
                    neighbor_neighbors.push(id);
                    
                    // Prune if too many connections
                    if neighbor_neighbors.len() > self.m {
                        // Get distances to all neighbors
                        let mut neighbor_candidates: Vec<Candidate> = neighbor_neighbors
                            .iter()
                            .filter_map(|&nid| {
                                self.vectors.read().get(&nid).map(|v| {
                                    let dist = Self::cosine_distance(v, &vector);
                                    Candidate { id: nid, distance: dist }
                                })
                            })
                            .collect();
                        neighbor_candidates.sort_by(|a, b| {
                            a.distance.partial_cmp(&b.distance)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                        *neighbor_neighbors = neighbor_candidates
                            .iter()
                            .take(self.m)
                            .map(|c| c.id)
                            .collect();
                    }
                }
            }

            // Update current_closest for next layer
            if let Some(closest) = candidates.first() {
                current_closest = vec![closest.id];
            }
        }

        Ok(())
    }

    /// Search for k nearest neighbors
    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<(u64, f32)>> {
        if query.len() != self.dimension {
            return Err(Error::Storage(format!(
                "Query dimension {} doesn't match index dimension {}",
                query.len(),
                self.dimension
            )));
        }

        let entry_point = if let Some(ep) = *self.entry_point.read() {
            ep
        } else {
            return Ok(Vec::new());
        };
        let mut current_closest = vec![entry_point];

        // Search from top layer down
        let max_layer = *self.max_layer.read();
        for layer in (1..=max_layer).rev() {
            let candidates = self.search_layer(query, &current_closest, 1, layer);
            if let Some(closest) = candidates.first() {
                current_closest = vec![closest.id];
            }
        }

        // Search at layer 0 with ef = k
        let candidates = self.search_layer(query, &current_closest, k.max(1), 0);

        Ok(candidates
            .iter()
            .take(k)
            .map(|c| (c.id, 1.0 - c.distance)) // Convert distance back to similarity
            .collect())
    }

    /// Get number of vectors in index
    pub fn len(&self) -> usize {
        self.vectors.read().len()
    }

    /// Check if index is empty
    pub fn is_empty(&self) -> bool {
        self.vectors.read().is_empty()
    }
}

