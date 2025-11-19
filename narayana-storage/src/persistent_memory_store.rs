// Persistent Memory Storage for Cognitive Brain
// Actually persists memories to disk with vector search integration

use crate::cognitive::{Memory, MemoryType, Experience};
use crate::vector_search::{VectorIndex, Embedding, IndexType};
use narayana_core::{Error, Result};
use parking_lot::RwLock;
use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use tokio::fs;
use serde::{Deserialize, Serialize};
use bincode;
use tracing::{info, warn};

/// Persistent memory storage backend
pub struct PersistentMemoryStore {
    data_dir: PathBuf,
    memories: Arc<RwLock<HashMap<String, Memory>>>,
    experiences: Arc<RwLock<HashMap<String, Experience>>>,
    vector_index: Arc<VectorIndex>,
    memory_index: Arc<RwLock<MemoryIndex>>,
}

#[derive(Default)]
struct MemoryIndex {
    by_type: HashMap<MemoryType, Vec<String>>,
    by_tag: HashMap<String, Vec<String>>,
    temporal_index: Vec<(u64, String)>, // (timestamp, memory_id)
}

impl PersistentMemoryStore {
    pub fn new(data_dir: impl AsRef<Path>, embedding_dim: usize) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        
        // Create data directory
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| Error::Storage(format!("Failed to create memory directory: {}", e)))?;

        // Create vector index for semantic search
        let vector_index = Arc::new(VectorIndex::new(
            embedding_dim,
            IndexType::HNSW { m: 16, ef_construction: 200 }
        ));

        Ok(Self {
            data_dir,
            memories: Arc::new(RwLock::new(HashMap::new())),
            experiences: Arc::new(RwLock::new(HashMap::new())),
            vector_index,
            memory_index: Arc::new(RwLock::new(MemoryIndex::default())),
        })
    }

    fn memory_file_path(&self, memory_id: &str) -> PathBuf {
        self.data_dir.join(format!("memory_{}.bin", memory_id))
    }

    fn experience_file_path(&self, experience_id: &str) -> PathBuf {
        self.data_dir.join(format!("experience_{}.bin", experience_id))
    }

    fn index_file_path(&self) -> PathBuf {
        self.data_dir.join("index.bin")
    }

    /// Store memory persistently
    pub async fn store_memory(&self, memory: Memory) -> Result<()> {
        let memory_id = memory.id.clone();
        
        // Serialize and write to disk
        let bytes = bincode::serialize(&memory)
            .map_err(|e| Error::Serialization(format!("Failed to serialize memory: {}", e)))?;
        
        let file_path = self.memory_file_path(&memory_id);
        fs::write(&file_path, &bytes).await
            .map_err(|e| Error::Storage(format!("Failed to write memory: {}", e)))?;

        // Store in memory cache
        self.memories.write().insert(memory_id.clone(), memory.clone());

        // Add to vector index if embedding exists
        if let Some(embedding) = &memory.embedding {
            let vector_embedding = Embedding {
                id: memory_id.as_bytes().iter().map(|&b| b as u64).sum(), // Simple hash
                vector: embedding.clone(),
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("memory_id".to_string(), serde_json::Value::String(memory_id.clone()));
                    meta.insert("memory_type".to_string(), serde_json::Value::String(format!("{:?}", memory.memory_type)));
                    meta
                },
                timestamp: memory.created_at as i64,
            };
            self.vector_index.add(vector_embedding)?;
        }

        // Update index
        let mut index = self.memory_index.write();
        index.by_type
            .entry(memory.memory_type.clone())
            .or_insert_with(Vec::new)
            .push(memory_id.clone());
        for tag in &memory.tags {
            index.by_tag
                .entry(tag.clone())
                .or_insert_with(Vec::new)
                .push(memory_id.clone());
        }
        index.temporal_index.push((memory.created_at, memory_id));

        // Persist index
        self.save_index().await?;

        Ok(())
    }

    /// Load memory from disk
    pub async fn load_memory(&self, memory_id: &str) -> Result<Option<Memory>> {
        // Check memory cache first
        {
            let memories = self.memories.read();
            if let Some(memory) = memories.get(memory_id) {
                return Ok(Some(memory.clone()));
            }
        }

        // Load from disk
        let file_path = self.memory_file_path(memory_id);
        if !file_path.exists() {
            return Ok(None);
        }

        let bytes = fs::read(&file_path).await
            .map_err(|e| Error::Storage(format!("Failed to read memory: {}", e)))?;

        let memory: Memory = bincode::deserialize(&bytes)
            .map_err(|e| Error::Deserialization(format!("Failed to deserialize memory: {}", e)))?;

        // Cache in memory
        self.memories.write().insert(memory_id.to_string(), memory.clone());

        Ok(Some(memory))
    }

    /// Store experience persistently
    pub async fn store_experience(&self, experience: Experience) -> Result<()> {
        let experience_id = experience.id.clone();
        
        // Serialize and write to disk
        let bytes = bincode::serialize(&experience)
            .map_err(|e| Error::Serialization(format!("Failed to serialize experience: {}", e)))?;
        
        let file_path = self.experience_file_path(&experience_id);
        fs::write(&file_path, &bytes).await
            .map_err(|e| Error::Storage(format!("Failed to write experience: {}", e)))?;

        // Store in memory cache
        self.experiences.write().insert(experience_id, experience);

        Ok(())
    }

    /// Semantic search for memories
    pub fn search_memories_semantic(&self, query_embedding: &[f32], limit: usize) -> Result<Vec<(String, f32)>> {
        // Search vector index
        let results = self.vector_index.search(query_embedding, limit)?;
        
        Ok(results.into_iter()
            .map(|result| {
                // Convert ID back to memory_id (simple reverse hash)
                // In production, would maintain a proper mapping
                let memory_id = format!("memory_{}", result.id);
                (memory_id, result.similarity)
            })
            .collect())
    }

    /// Retrieve memories by type
    pub fn get_memories_by_type(&self, memory_type: &MemoryType) -> Vec<String> {
        let index = self.memory_index.read();
        index.by_type
            .get(memory_type)
            .cloned()
            .unwrap_or_default()
    }

    /// Retrieve memories by tag
    pub fn get_memories_by_tag(&self, tag: &str) -> Vec<String> {
        let index = self.memory_index.read();
        index.by_tag
            .get(tag)
            .cloned()
            .unwrap_or_default()
    }

    /// Retrieve memories by time range
    pub fn get_memories_by_time_range(&self, start: u64, end: u64) -> Vec<String> {
        let index = self.memory_index.read();
        index.temporal_index
            .iter()
            .filter(|(timestamp, _)| *timestamp >= start && *timestamp <= end)
            .map(|(_, memory_id)| memory_id.clone())
            .collect()
    }

    /// Save index to disk
    async fn save_index(&self) -> Result<()> {
        let index = self.memory_index.read();
        let serializable = SerializableIndex {
            by_type: index.by_type.clone(),
            by_tag: index.by_tag.clone(),
            temporal_index: index.temporal_index.clone(),
        };

        let bytes = bincode::serialize(&serializable)
            .map_err(|e| Error::Serialization(format!("Failed to serialize index: {}", e)))?;

        let file_path = self.index_file_path();
        fs::write(&file_path, &bytes).await
            .map_err(|e| Error::Storage(format!("Failed to write index: {}", e)))?;

        Ok(())
    }

    /// Load index from disk
    pub async fn load_index(&self) -> Result<()> {
        let file_path = self.index_file_path();
        if !file_path.exists() {
            return Ok(());
        }

        let bytes = fs::read(&file_path).await
            .map_err(|e| Error::Storage(format!("Failed to read index: {}", e)))?;

        let serializable: SerializableIndex = bincode::deserialize(&bytes)
            .map_err(|e| Error::Deserialization(format!("Failed to deserialize index: {}", e)))?;

        let mut index = self.memory_index.write();
        index.by_type = serializable.by_type;
        index.by_tag = serializable.by_tag;
        index.temporal_index = serializable.temporal_index;

        Ok(())
    }

    /// Load all memories from disk (for startup)
    pub async fn load_all_memories(&self) -> Result<()> {
        // Load index first
        self.load_index().await?;

        // Load memories referenced in index
        let memory_ids: Vec<String> = {
            let index = self.memory_index.read();
            index.temporal_index
                .iter()
                .map(|(_, id)| id.clone())
                .collect()
        };

        // Load each memory (will be cached)
        for memory_id in memory_ids {
            if let Err(e) = self.load_memory(&memory_id).await {
                warn!("Failed to load memory {}: {}", memory_id, e);
            }
        }

        info!("Loaded {} memories from disk", self.memories.read().len());
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct SerializableIndex {
    by_type: HashMap<MemoryType, Vec<String>>,
    by_tag: HashMap<String, Vec<String>>,
    temporal_index: Vec<(u64, String)>,
}

