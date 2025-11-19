// Vector search for AI embeddings - semantic search for conversations and events
// GPU-accelerated with automatic backend detection

use narayana_core::{Error, Result};
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use super::gpu_execution::{GpuEngine, GpuTensor, Backend};
use super::hnsw::HNSWIndex;

/// Vector embedding for semantic search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub id: u64,
    pub vector: Vec<f32>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub timestamp: i64,
}

/// Vector index for fast similarity search (GPU-accelerated)
pub struct VectorIndex {
    embeddings: Arc<RwLock<HashMap<u64, Embedding>>>,
    pub dimension: usize,
    index_type: IndexType,
    gpu_engine: Option<Arc<GpuEngine>>,
    use_gpu: bool,
    hnsw_index: Option<Arc<HNSWIndex>>,
}

#[derive(Debug, Clone)]
pub enum IndexType {
    Flat, // Linear search (exact)
    HNSW { m: usize, ef_construction: usize }, // Hierarchical Navigable Small World (approximate)
    IVF { nlist: usize }, // Inverted File Index (approximate)
}

impl Default for IndexType {
    fn default() -> Self {
        IndexType::Flat
    }
}

impl VectorIndex {
    pub fn new(dimension: usize, index_type: IndexType) -> Self {
        let hnsw_index = match &index_type {
            IndexType::HNSW { m, ef_construction } => {
                Some(Arc::new(HNSWIndex::new(*m, *ef_construction, dimension)))
            }
            _ => None,
        };

        Self {
            embeddings: Arc::new(RwLock::new(HashMap::new())),
            dimension,
            index_type,
            gpu_engine: None,
            use_gpu: false,
            hnsw_index,
        }
    }

    /// Create with GPU acceleration
    pub fn with_gpu(dimension: usize, index_type: IndexType, backend: Option<Backend>) -> Result<Self> {
        let gpu_engine = GpuEngine::with_backend(backend.unwrap_or(Backend::CPU))?;
        let hnsw_index = match &index_type {
            IndexType::HNSW { m, ef_construction } => {
                Some(Arc::new(HNSWIndex::new(*m, *ef_construction, dimension)))
            }
            _ => None,
        };
        Ok(Self {
            embeddings: Arc::new(RwLock::new(HashMap::new())),
            dimension,
            index_type,
            gpu_engine: Some(Arc::new(gpu_engine)),
            use_gpu: true,
            hnsw_index,
        })
    }

    /// Enable GPU acceleration
    pub fn enable_gpu(&mut self, backend: Option<Backend>) -> Result<()> {
        let gpu_engine = GpuEngine::with_backend(backend.unwrap_or(Backend::CPU))?;
        self.gpu_engine = Some(Arc::new(gpu_engine));
        self.use_gpu = true;
        Ok(())
    }

    /// Add embedding to index
    pub fn add(&self, embedding: Embedding) -> Result<()> {
        if embedding.vector.len() != self.dimension {
            return Err(Error::Storage(format!(
                "Embedding dimension {} doesn't match index dimension {}",
                embedding.vector.len(),
                self.dimension
            )));
        }

        let mut embeddings = self.embeddings.write();
        embeddings.insert(embedding.id, embedding.clone());

        // Add to HNSW index if available
        if let Some(ref hnsw) = self.hnsw_index {
            hnsw.insert(embedding.id, embedding.vector)?;
        }

        Ok(())
    }

    /// Search for similar embeddings (GPU-accelerated if enabled)
    pub fn search(&self, query_vector: &[f32], k: usize) -> Result<Vec<SearchResult>> {
        if query_vector.len() != self.dimension {
            return Err(Error::Storage(format!(
                "Query vector dimension {} doesn't match index dimension {}",
                query_vector.len(),
                self.dimension
            )));
        }

        let embeddings = self.embeddings.read();
        
        // Use HNSW if available (fastest for large datasets)
        if let Some(ref hnsw) = self.hnsw_index {
            let hnsw_results = hnsw.search(query_vector, k)?;
            let mut results = Vec::new();
            for (id, similarity) in hnsw_results {
                if let Some(embedding) = embeddings.get(&id) {
                    results.push(SearchResult {
                        id,
                        similarity: similarity as f32,
                        embedding: embedding.clone(),
                    });
                }
            }
            return Ok(results);
        }
        
        // Use GPU if available
        if self.use_gpu {
            if let Some(ref engine) = self.gpu_engine {
                let query_tensor = GpuTensor::from_vec(query_vector.to_vec());
                let mut results: Vec<SearchResult> = embeddings
                    .iter()
                    .filter_map(|(id, embedding)| {
                        let embedding_tensor = GpuTensor::from_vec(embedding.vector.clone());
                        engine.cosine_similarity(&query_tensor, &embedding_tensor)
                            .ok()
                            .map(|similarity| SearchResult {
                                id: *id,
                                similarity,
                                embedding: embedding.clone(),
                            })
                    })
                    .collect();

                results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
                results.truncate(k);
                return Ok(results);
            }
        }

        // CPU fallback (linear search)
        let mut results: Vec<SearchResult> = embeddings.iter()
            .map(|(id, embedding)| {
                let similarity = cosine_similarity(query_vector, &embedding.vector);
                SearchResult {
                    id: *id,
                    similarity,
                    embedding: embedding.clone(),
                }
            })
            .collect();

        // Sort by similarity (descending)
        results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
        results.truncate(k);

        Ok(results)
    }

    /// Batch search
    pub fn batch_search(&self, query_vectors: &[Vec<f32>], k: usize) -> Result<Vec<Vec<SearchResult>>> {
        query_vectors.iter()
            .map(|qv| self.search(qv, k))
            .collect()
    }
}

/// Search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: u64,
    pub similarity: f32,
    pub embedding: Embedding,
}

/// Cosine similarity (for semantic search)
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}

/// Vector store for AI agents (GPU-accelerated)
pub struct VectorStore {
    indexes: Arc<RwLock<HashMap<String, VectorIndex>>>,
    default_gpu_backend: Option<Backend>,
}

impl VectorStore {
    pub fn new() -> Self {
        Self {
            indexes: Arc::new(RwLock::new(HashMap::new())),
            default_gpu_backend: None,
        }
    }

    /// Create with GPU backend
    pub fn with_gpu(backend: Option<Backend>) -> Self {
        Self {
            indexes: Arc::new(RwLock::new(HashMap::new())),
            default_gpu_backend: backend,
        }
    }

    /// Create vector index
    pub fn create_index(&self, name: String, dimension: usize, index_type: IndexType) {
        let mut indexes = self.indexes.write();
        if let Some(backend) = self.default_gpu_backend {
            if let Ok(index) = VectorIndex::with_gpu(dimension, index_type.clone(), Some(backend)) {
                indexes.insert(name, index);
                return;
            }
        }
        indexes.insert(name, VectorIndex::new(dimension, index_type));
    }

    /// Create vector index with GPU acceleration
    pub fn create_index_with_gpu(&self, name: String, dimension: usize, index_type: IndexType, backend: Option<Backend>) -> Result<()> {
        let mut indexes = self.indexes.write();
        let index = VectorIndex::with_gpu(dimension, index_type, backend)?;
        indexes.insert(name, index);
        Ok(())
    }

    /// Add embedding to index
    pub fn add_embedding(&self, index_name: &str, embedding: Embedding) -> Result<()> {
        let indexes = self.indexes.read();
        if let Some(index) = indexes.get(index_name) {
            index.add(embedding)
        } else {
            Err(Error::Storage(format!("Index '{}' not found", index_name)))
        }
    }

    /// Search in index
    pub fn search(&self, index_name: &str, query_vector: &[f32], k: usize) -> Result<Vec<SearchResult>> {
        let indexes = self.indexes.read();
        if let Some(index) = indexes.get(index_name) {
            index.search(query_vector, k)
        } else {
            Err(Error::Storage(format!("Index '{}' not found", index_name)))
        }
    }

    /// Semantic search for conversations
    pub fn search_conversations(
        &self,
        query_embedding: &[f32],
        k: usize,
    ) -> Result<Vec<SearchResult>> {
        self.search("conversations", query_embedding, k)
    }

    /// Semantic search for events
    pub fn search_events(
        &self,
        query_embedding: &[f32],
        k: usize,
    ) -> Result<Vec<SearchResult>> {
        self.search("events", query_embedding, k)
    }
}

/// Hybrid search (vector + metadata filtering)
pub struct HybridSearch {
    vector_store: VectorStore,
}

impl HybridSearch {
    pub fn new(vector_store: VectorStore) -> Self {
        Self { vector_store }
    }

    /// Search with metadata filters
    pub fn search_with_filters(
        &self,
        index_name: &str,
        query_vector: &[f32],
        k: usize,
        metadata_filters: HashMap<String, serde_json::Value>,
    ) -> Result<Vec<SearchResult>> {
        // First, do vector search
        let mut results = self.vector_store.search(index_name, query_vector, k * 2)?;

        // Then, filter by metadata
        results.retain(|result| {
            metadata_filters.iter().all(|(key, value)| {
                result.embedding.metadata.get(key) == Some(value)
            })
        });

        results.truncate(k);
        Ok(results)
    }
}

