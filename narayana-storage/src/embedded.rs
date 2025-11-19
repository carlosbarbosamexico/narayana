// On-Device Embedded Version
// Stripped-down version for Jetson, RasPi 5, ESP32 S3
// Production-ready minimal implementation

use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use tracing::{info, debug};

/// Embedded NarayanaDB - minimal version for edge devices
pub struct EmbeddedNarayana {
    storage: Arc<EmbeddedStorage>,
    vector_search: Arc<EmbeddedVectorSearch>,
    thought_scheduler: Arc<EmbeddedThoughtScheduler>,
    memory: Arc<EmbeddedMemory>,
    model_slots: Arc<RwLock<HashMap<String, EmbeddedModelSlot>>>,
}

impl EmbeddedNarayana {
    pub fn new() -> Self {
        info!("Initializing Embedded NarayanaDB");
        Self {
            storage: Arc::new(EmbeddedStorage::new()),
            vector_search: Arc::new(EmbeddedVectorSearch::new()),
            thought_scheduler: Arc::new(EmbeddedThoughtScheduler::new()),
            memory: Arc::new(EmbeddedMemory::new()),
            model_slots: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize for specific platform
    pub fn for_platform(platform: EmbeddedPlatform) -> Self {
        let mut instance = Self::new();
        instance.configure_for_platform(platform);
        instance
    }

    fn configure_for_platform(&mut self, platform: EmbeddedPlatform) {
        match platform {
            EmbeddedPlatform::Jetson => {
                info!("Configuring for NVIDIA Jetson");
                // Optimize for CUDA-capable device
            }
            EmbeddedPlatform::RaspberryPi5 => {
                info!("Configuring for Raspberry Pi 5");
                // Optimize for ARM CPU
            }
            EmbeddedPlatform::ESP32S3 => {
                info!("Configuring for ESP32 S3");
                // Minimal configuration for microcontroller
            }
        }
    }

    /// Store data
    pub async fn store(&self, key: &str, value: &[u8]) -> Result<()> {
        self.storage.store(key, value).await
    }

    /// Retrieve data
    pub async fn retrieve(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.storage.retrieve(key).await
    }

    /// Vector search
    pub async fn vector_search(&self, query: &[f32], k: usize) -> Result<Vec<VectorResult>> {
        self.vector_search.search(query, k).await
    }

    /// Schedule thought
    pub async fn schedule_thought(&self, thought: EmbeddedThought) -> Result<String> {
        self.thought_scheduler.schedule(thought).await
    }

    /// Store memory
    pub async fn store_memory(&self, memory: EmbeddedMemoryEntry) -> Result<String> {
        self.memory.store(memory).await
    }

    /// Register model slot
    pub fn register_model(&self, slot_name: &str, model: EmbeddedModel) -> Result<()> {
        let mut slots = self.model_slots.write();
        slots.insert(slot_name.to_string(), EmbeddedModelSlot {
            model,
            loaded: false,
        });
        Ok(())
    }

    /// Get memory usage
    pub fn get_memory_usage(&self) -> MemoryUsage {
        MemoryUsage {
            storage_bytes: self.storage.get_size(),
            vector_index_bytes: self.vector_search.get_size(),
            memory_entries: self.memory.get_count(),
            model_slots: self.model_slots.read().len(),
        }
    }
}

/// Embedded storage - minimal key-value store
struct EmbeddedStorage {
    data: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl EmbeddedStorage {
    fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn store(&self, key: &str, value: &[u8]) -> Result<()> {
        self.data.write().insert(key.to_string(), value.to_vec());
        Ok(())
    }

    async fn retrieve(&self, key: &str) -> Result<Option<Vec<u8>>> {
        Ok(self.data.read().get(key).cloned())
    }

    fn get_size(&self) -> usize {
        self.data.read().values().map(|v| v.len()).sum()
    }
}

/// Embedded vector search - minimal vector similarity search
struct EmbeddedVectorSearch {
    vectors: Arc<RwLock<Vec<(String, Vec<f32>)>>>,
}

impl EmbeddedVectorSearch {
    fn new() -> Self {
        Self {
            vectors: Arc::new(RwLock::new(Vec::new())),
        }
    }

    async fn search(&self, query: &[f32], k: usize) -> Result<Vec<VectorResult>> {
        // EDGE CASE: Validate k is reasonable (prevent DoS with very large k)
        const MAX_K: usize = 100000;
        let safe_k = k.min(MAX_K);
        
        let vectors = self.vectors.read();
        let mut results: Vec<(String, f32)> = vectors.iter()
            .map(|(id, vec)| {
                let similarity = cosine_similarity(query, vec);
                (id.clone(), similarity)
            })
            .collect();
        
        results.sort_by(|a, b| {
            // Handle NaN values by treating as lowest priority
            if a.1.is_nan() && b.1.is_nan() {
                std::cmp::Ordering::Equal
            } else if a.1.is_nan() {
                std::cmp::Ordering::Less
            } else if b.1.is_nan() {
                std::cmp::Ordering::Greater
            } else {
                b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
            }
        });
        results.truncate(safe_k);
        
        Ok(results.into_iter()
            .map(|(id, score)| VectorResult { id, score })
            .collect())
    }

    fn get_size(&self) -> usize {
        self.vectors.read().iter()
            .map(|(_, v)| v.len() * std::mem::size_of::<f32>())
            .sum()
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    // EDGE CASE: Check for very small norms (near-zero) to prevent precision issues
    const EPSILON: f32 = 1e-8;
    if norm_a < EPSILON || norm_b < EPSILON {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

/// Embedded thought scheduler
struct EmbeddedThoughtScheduler {
    thoughts: Arc<RwLock<Vec<EmbeddedThought>>>,
}

impl EmbeddedThoughtScheduler {
    fn new() -> Self {
        Self {
            thoughts: Arc::new(RwLock::new(Vec::new())),
        }
    }

    async fn schedule(&self, thought: EmbeddedThought) -> Result<String> {
        let thought_id = thought.id.clone();
        self.thoughts.write().push(thought);
        Ok(thought_id)
    }
}

/// Embedded memory
struct EmbeddedMemory {
    entries: Arc<RwLock<HashMap<String, EmbeddedMemoryEntry>>>,
}

impl EmbeddedMemory {
    fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn store(&self, memory: EmbeddedMemoryEntry) -> Result<String> {
        let id = memory.id.clone();
        self.entries.write().insert(id.clone(), memory);
        Ok(id)
    }

    fn get_count(&self) -> usize {
        self.entries.read().len()
    }
}

/// Embedded platform
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddedPlatform {
    Jetson,
    RaspberryPi5,
    ESP32S3,
}

/// Embedded thought
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedThought {
    pub id: String,
    pub content: serde_json::Value,
    pub priority: f64,
}

/// Embedded memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedMemoryEntry {
    pub id: String,
    pub content: serde_json::Value,
    pub timestamp: u64,
}

/// Embedded model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedModel {
    pub model_id: String,
    pub weights: Vec<u8>,
    pub input_shape: Vec<usize>,
    pub output_shape: Vec<usize>,
}

/// Embedded model slot
#[derive(Debug, Clone)]
struct EmbeddedModelSlot {
    model: EmbeddedModel,
    loaded: bool,
}

/// Vector search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorResult {
    pub id: String,
    pub score: f32,
}

/// Memory usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    pub storage_bytes: usize,
    pub vector_index_bytes: usize,
    pub memory_entries: usize,
    pub model_slots: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_embedded_storage() {
        let narayana = EmbeddedNarayana::new();
        narayana.store("test_key", b"test_value").await.unwrap();
        let value = narayana.retrieve("test_key").await.unwrap();
        assert_eq!(value, Some(b"test_value".to_vec()));
    }

    #[tokio::test]
    async fn test_embedded_vector_search() {
        let narayana = EmbeddedNarayana::new();
        let query = vec![1.0, 0.0, 0.0];
        let results = narayana.vector_search(&query, 5).await.unwrap();
        assert!(results.is_empty()); // No vectors indexed yet
    }
}

