// Infinite Context System - Instant Context for Next-Generation Agent Interactions
// So fast it can provide infinite context instantly

use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use parking_lot::RwLock;
use dashmap::DashMap;
use tokio::sync::broadcast;
use bytes::{Bytes, BytesMut};
use std::hash::Hash;

/// Context entry - represents a piece of context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    pub id: String,
    #[serde(skip_serializing, skip_deserializing)] // Bytes doesn't implement Serialize/Deserialize
    pub content: Bytes,
    pub metadata: ContextMetadata,
    pub embedding: Option<Vec<f32>>,
    pub tokens: Option<usize>,
    pub created_at: u64,
    pub accessed_at: u64,
    pub access_count: u64,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMetadata {
    pub agent_id: Option<String>,
    pub conversation_id: Option<String>,
    pub message_id: Option<String>,
    pub context_type: ContextType,
    pub tags: Vec<String>,
    pub priority: f64,
    pub importance: f64,
    pub related_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContextType {
    Message,
    Memory,
    Knowledge,
    Observation,
    Action,
    Result,
    Reflection,
    Plan,
    Goal,
    State,
}

/// Infinite context manager - instant context retrieval
pub struct InfiniteContextManager {
    // Primary storage - optimized for instant access
    contexts: Arc<DashMap<String, ContextEntry>>,
    
    // Indexes for fast lookup
    by_agent: Arc<DashMap<String, Vec<String>>>, // agent_id -> context_ids
    by_conversation: Arc<DashMap<String, Vec<String>>>, // conversation_id -> context_ids
    by_type: Arc<DashMap<ContextType, Vec<String>>>, // type -> context_ids
    by_tag: Arc<DashMap<String, Vec<String>>>, // tag -> context_ids
    temporal_index: Arc<RwLock<Vec<(u64, String)>>>, // (timestamp, context_id)
    
    // Embedding index for semantic search
    embedding_index: Arc<RwLock<HashMap<String, Vec<f32>>>>,
    
    // Hot cache for instant access
    hot_cache: Arc<DashMap<String, Bytes>>, // context_id -> content
    
    // Compression
    compressed_contexts: Arc<DashMap<String, Bytes>>, // context_id -> compressed
    
    // Statistics
    stats: Arc<RwLock<ContextStats>>,
    
    // Configuration
    config: InfiniteContextConfig,
    
    // Event channel
    event_sender: broadcast::Sender<ContextEvent>,
}

/// Context statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextStats {
    pub total_contexts: usize,
    pub total_size_bytes: u64,
    pub total_tokens: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub retrievals: u64,
    pub retrievals_per_second: f64,
    pub average_retrieval_time_ns: u64,
}

/// Infinite context configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfiniteContextConfig {
    // Performance settings
    pub enable_hot_cache: bool,
    pub hot_cache_size: usize,
    pub enable_compression: bool,
    pub compression_threshold_bytes: usize,
    pub enable_embedding_index: bool,
    
    // Retrieval settings
    pub enable_parallel_retrieval: bool,
    pub max_parallel_retrievals: usize,
    pub enable_prefetch: bool,
    pub prefetch_threshold: usize,
    
    // Storage settings
    pub enable_persistence: bool,
    pub persistence_path: Option<String>,
    pub enable_memory_mapping: bool,
    
    // Indexing settings
    pub enable_temporal_index: bool,
    pub enable_semantic_index: bool,
    pub enable_full_text_index: bool,
    
    // Streaming settings
    pub enable_streaming: bool,
    pub stream_chunk_size: usize,
    
    // Caching settings
    pub enable_context_cache: bool,
    pub cache_ttl: Option<u64>,
    
    // Advanced settings
    pub enable_incremental_updates: bool,
    pub enable_context_versioning: bool,
    pub enable_context_deduplication: bool,
}

impl Default for InfiniteContextConfig {
    fn default() -> Self {
        Self {
            enable_hot_cache: true,
            hot_cache_size: 100_000,
            enable_compression: true,
            compression_threshold_bytes: 1024,
            enable_embedding_index: true,
            enable_parallel_retrieval: true,
            max_parallel_retrievals: 1000,
            enable_prefetch: true,
            prefetch_threshold: 3,
            enable_persistence: false,
            persistence_path: None,
            enable_memory_mapping: true,
            enable_temporal_index: true,
            enable_semantic_index: true,
            enable_full_text_index: false,
            enable_streaming: true,
            stream_chunk_size: 8192,
            enable_context_cache: true,
            cache_ttl: Some(3600),
            enable_incremental_updates: true,
            enable_context_versioning: true,
            enable_context_deduplication: true,
        }
    }
}

/// Context event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextEvent {
    ContextAdded { context_id: String },
    ContextUpdated { context_id: String, version: u64 },
    ContextRetrieved { context_id: String },
    ContextDeleted { context_id: String },
    ContextStreamed { context_id: String, chunk: usize },
}

impl InfiniteContextManager {
    pub fn new(config: InfiniteContextConfig) -> Self {
        let (sender, _) = broadcast::channel(10000);
        
        Self {
            contexts: Arc::new(DashMap::new()),
            by_agent: Arc::new(DashMap::new()),
            by_conversation: Arc::new(DashMap::new()),
            by_type: Arc::new(DashMap::new()),
            by_tag: Arc::new(DashMap::new()),
            temporal_index: Arc::new(RwLock::new(Vec::new())),
            embedding_index: Arc::new(RwLock::new(HashMap::new())),
            hot_cache: Arc::new(DashMap::new()),
            compressed_contexts: Arc::new(DashMap::new()),
            stats: Arc::new(RwLock::new(ContextStats {
                total_contexts: 0,
                total_size_bytes: 0,
                total_tokens: 0,
                cache_hits: 0,
                cache_misses: 0,
                retrievals: 0,
                retrievals_per_second: 0.0,
                average_retrieval_time_ns: 0,
            })),
            config: config.clone(),
            event_sender: sender,
        }
    }

    /// Add context - instant insertion
    pub fn add_context(
        &self,
        content: Bytes,
        metadata: ContextMetadata,
        embedding: Option<Vec<f32>>,
        tokens: Option<usize>,
    ) -> Result<String> {
        let start = std::time::Instant::now();
        let context_id = uuid::Uuid::new_v4().to_string();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // SECURITY: Prevent memory exhaustion with extremely large content
        const MAX_CONTENT_SIZE: usize = 1_000_000_000; // 1GB max per context
        if content.len() > MAX_CONTENT_SIZE {
            return Err(Error::Storage(format!("Content size {} exceeds maximum {}", content.len(), MAX_CONTENT_SIZE)));
        }
        
        // Compress if needed
        let stored_content = if self.config.enable_compression 
            && content.len() >= self.config.compression_threshold_bytes {
            self.compress(&content)?
        } else {
            content.clone()
        };

        let entry = ContextEntry {
            id: context_id.clone(),
            content: stored_content,
            metadata: metadata.clone(),
            embedding: embedding.clone(),
            tokens,
            created_at: now,
            accessed_at: now,
            access_count: 0,
            version: 1,
        };

        // SECURITY: Prevent unbounded growth of primary storage
        const MAX_CONTEXTS: usize = 100_000_000; // Maximum contexts
        if self.contexts.len() >= MAX_CONTEXTS {
            // Evict oldest contexts (FIFO) - in production, would use LRU or similar
            // For now, remove random entries to prevent unbounded growth
            let keys_to_remove: Vec<String> = self.contexts.iter().take(MAX_CONTEXTS / 10).map(|e| e.key().clone()).collect();
            for key in keys_to_remove {
                self.contexts.remove(&key);
            }
        }
        
        // Store in primary storage
        self.contexts.insert(context_id.clone(), entry.clone());

        // Update indexes
        self.update_indexes(&context_id, &metadata, embedding.as_deref())?;

        // Store content length before moving
        let content_len = content.len();

        // Add to hot cache if enabled
        if self.config.enable_hot_cache {
            self.hot_cache.insert(context_id.clone(), content);
            // Limit hot cache size
            if self.hot_cache.len() > self.config.hot_cache_size {
                self.evict_from_hot_cache();
            }
        }

        // Update statistics
        let mut stats = self.stats.write();
        stats.total_contexts += 1;
        stats.total_size_bytes += content_len as u64;
        if let Some(tokens) = tokens {
            stats.total_tokens += tokens as u64;
        }

        let _ = self.event_sender.send(ContextEvent::ContextAdded {
            context_id: context_id.clone(),
        });

        let duration = start.elapsed();
        debug!("Added context {} in {:?}", context_id, duration);

        Ok(context_id)
    }

    /// Retrieve context - instant retrieval
    pub fn retrieve_context(&self, context_id: &str) -> Result<Option<Bytes>> {
        let start = std::time::Instant::now();

        // Try hot cache first
        if self.config.enable_hot_cache {
            if let Some(content) = self.hot_cache.get(context_id) {
                let mut stats = self.stats.write();
                stats.cache_hits += 1;
                stats.retrievals += 1;
                self.update_retrieval_stats(&mut stats, start.elapsed());
                return Ok(Some(content.clone()));
            }
        }

        // Retrieve from primary storage
        if let Some(mut entry) = self.contexts.get_mut(context_id) {
            // Decompress if needed
            let content = if self.compressed_contexts.contains_key(context_id) {
                self.decompress(&entry.content)?
            } else {
                entry.content.clone()
            };

            // Update access metadata
            entry.accessed_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            entry.access_count += 1;

            // Add to hot cache
            if self.config.enable_hot_cache {
                self.hot_cache.insert(context_id.to_string(), content.clone());
            }

            // Update statistics
            let mut stats = self.stats.write();
            stats.cache_misses += 1;
            stats.retrievals += 1;
            self.update_retrieval_stats(&mut stats, start.elapsed());

            let _ = self.event_sender.send(ContextEvent::ContextRetrieved {
                context_id: context_id.to_string(),
            });

            Ok(Some(content))
        } else {
            let mut stats = self.stats.write();
            stats.cache_misses += 1;
            stats.retrievals += 1;
            Ok(None)
        }
    }

    /// Retrieve contexts by agent - instant batch retrieval
    pub fn retrieve_by_agent(&self, agent_id: &str) -> Result<Vec<Bytes>> {
        if let Some(context_ids) = self.by_agent.get(agent_id) {
            self.retrieve_batch(context_ids.value())
        } else {
            Ok(Vec::new())
        }
    }

    /// Retrieve contexts by conversation - instant batch retrieval
    pub fn retrieve_by_conversation(&self, conversation_id: &str) -> Result<Vec<Bytes>> {
        if let Some(context_ids) = self.by_conversation.get(conversation_id) {
            self.retrieve_batch(context_ids.value())
        } else {
            Ok(Vec::new())
        }
    }

    /// Retrieve contexts by type - instant batch retrieval
    pub fn retrieve_by_type(&self, context_type: ContextType) -> Result<Vec<Bytes>> {
        if let Some(context_ids) = self.by_type.get(&context_type) {
            self.retrieve_batch(context_ids.value())
        } else {
            Ok(Vec::new())
        }
    }

    /// Retrieve contexts by tag - instant batch retrieval
    pub fn retrieve_by_tag(&self, tag: &str) -> Result<Vec<Bytes>> {
        if let Some(context_ids) = self.by_tag.get(tag) {
            self.retrieve_batch(context_ids.value())
        } else {
            Ok(Vec::new())
        }
    }

    /// Retrieve contexts semantically - instant semantic search
    pub fn retrieve_semantic(&self, query_embedding: &[f32], k: usize) -> Result<Vec<Bytes>> {
        // SECURITY: Prevent DoS with excessive k values
        const MAX_K: usize = 100_000; // Maximum k for semantic search
        let k = k.min(MAX_K);
        
        let index = self.embedding_index.read();
        let mut similarities: Vec<(String, f64)> = Vec::new();

        for (context_id, embedding) in index.iter() {
            let similarity = Self::cosine_similarity(query_embedding, embedding)?;
            similarities.push((context_id.clone(), similarity));
        }

        // Sort by similarity and take top k
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        similarities.truncate(k);

        // Retrieve contexts
        let mut results = Vec::new();
        for (context_id, _) in similarities {
            if let Some(content) = self.retrieve_context(&context_id)? {
                results.push(content);
            }
        }

        Ok(results)
    }

    /// Retrieve contexts temporally - instant temporal retrieval
    pub fn retrieve_temporal(&self, start_time: u64, end_time: u64) -> Result<Vec<Bytes>> {
        let index = self.temporal_index.read();
        let context_ids: Vec<String> = index
            .iter()
            .filter(|(ts, _)| *ts >= start_time && *ts <= end_time)
            .map(|(_, id)| id.clone())
            .collect();
        drop(index);

        self.retrieve_batch(&context_ids)
    }

    /// Retrieve batch - parallel batch retrieval
    fn retrieve_batch(&self, context_ids: &[String]) -> Result<Vec<Bytes>> {
        if self.config.enable_parallel_retrieval {
            // Parallel retrieval
            use rayon::prelude::*;
            let results: Vec<Option<Bytes>> = context_ids
                .par_iter()
                .map(|id| self.retrieve_context(id).unwrap_or(None))
                .collect();
            Ok(results.into_iter().flatten().collect())
        } else {
            // Sequential retrieval
            let mut results = Vec::new();
            for id in context_ids {
                if let Some(content) = self.retrieve_context(id)? {
                    results.push(content);
                }
            }
            Ok(results)
        }
    }

    /// Stream context - stream large contexts
    pub fn stream_context(&self, context_id: &str) -> impl Iterator<Item = Result<Bytes>> {
        ContextStreamer::new(self.contexts.clone(), context_id.to_string())
    }

    /// Update indexes
    fn update_indexes(
        &self,
        context_id: &str,
        metadata: &ContextMetadata,
        embedding: Option<&[f32]>,
    ) -> Result<()> {
        // SECURITY: Prevent unbounded growth of index vectors
        const MAX_INDEX_VECTOR_SIZE: usize = 1_000_000; // Maximum contexts per index entry
        
        // Index by agent
        if let Some(ref agent_id) = metadata.agent_id {
            let mut entry = self.by_agent
                .entry(agent_id.clone())
                .or_insert_with(Vec::new);
            // SECURITY: If vector is too large, remove oldest entries (FIFO)
            if entry.len() >= MAX_INDEX_VECTOR_SIZE {
                entry.drain(0..(MAX_INDEX_VECTOR_SIZE / 10)); // Remove 10% of oldest
            }
            entry.push(context_id.to_string());
        }

        // Index by conversation
        if let Some(ref conversation_id) = metadata.conversation_id {
            let mut entry = self.by_conversation
                .entry(conversation_id.clone())
                .or_insert_with(Vec::new);
            if entry.len() < MAX_INDEX_VECTOR_SIZE {
                entry.push(context_id.to_string());
            } else {
                entry.drain(0..(MAX_INDEX_VECTOR_SIZE / 10));
                entry.push(context_id.to_string());
            }
        }

        // Index by type
        {
            let mut entry = self.by_type
                .entry(metadata.context_type.clone())
                .or_insert_with(Vec::new);
            if entry.len() < MAX_INDEX_VECTOR_SIZE {
                entry.push(context_id.to_string());
            } else {
                entry.drain(0..(MAX_INDEX_VECTOR_SIZE / 10));
                entry.push(context_id.to_string());
            }
        }

        // Index by tags
        // SECURITY: Limit number of tags to prevent DoS
        const MAX_TAGS: usize = 100;
        for tag in metadata.tags.iter().take(MAX_TAGS) {
            let mut entry = self.by_tag
                .entry(tag.clone())
                .or_insert_with(Vec::new);
            if entry.len() < MAX_INDEX_VECTOR_SIZE {
                entry.push(context_id.to_string());
            } else {
                entry.drain(0..(MAX_INDEX_VECTOR_SIZE / 10));
                entry.push(context_id.to_string());
            }
        }

        // Index temporally
        if self.config.enable_temporal_index {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let mut temporal = self.temporal_index.write();
            // SECURITY: Prevent unbounded growth of temporal index
            const MAX_TEMPORAL_ENTRIES: usize = 10_000_000; // Maximum temporal entries
            if temporal.len() >= MAX_TEMPORAL_ENTRIES {
                // Remove oldest entries (FIFO eviction)
                temporal.drain(0..(MAX_TEMPORAL_ENTRIES / 10)); // Remove 10% of oldest
            }
            temporal.push((now, context_id.to_string()));
        }

        // Index embedding
        if self.config.enable_embedding_index {
            if let Some(embedding) = embedding {
                // SECURITY: Prevent unbounded growth of embedding index
                const MAX_EMBEDDING_ENTRIES: usize = 10_000_000; // Maximum embedding entries
                let mut index = self.embedding_index.write();
                if index.len() >= MAX_EMBEDDING_ENTRIES {
                    // Remove oldest entries (FIFO eviction)
                    // Since HashMap doesn't preserve order, remove random entries
                    let keys_to_remove: Vec<String> = index.keys().take(MAX_EMBEDDING_ENTRIES / 10).cloned().collect();
                    for key in keys_to_remove {
                        index.remove(&key);
                    }
                }
                index.insert(context_id.to_string(), embedding.to_vec());
            }
        }

        Ok(())
    }

    /// Compress content
    fn compress(&self, content: &Bytes) -> Result<Bytes> {
        use lz4::EncoderBuilder;
        let mut encoder = EncoderBuilder::new()
            .level(4)
            .build(Vec::new())
            .map_err(|e| Error::Storage(format!("Compression error: {}", e)))?;
        encoder.write_all(content.as_ref())
            .map_err(|e| Error::Storage(format!("Compression write error: {}", e)))?;
        let (compressed, result) = encoder.finish();
        result.map_err(|e| Error::Storage(format!("Compression finish error: {}", e)))?;
        Ok(Bytes::from(compressed))
    }

    /// Decompress content
    fn decompress(&self, compressed: &Bytes) -> Result<Bytes> {
        use lz4::Decoder;
        let mut decoder = Decoder::new(compressed.as_ref())
            .map_err(|e| Error::Storage(format!("Decompression error: {}", e)))?;
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| Error::Storage(format!("Decompression read error: {}", e)))?;
        Ok(Bytes::from(decompressed))
    }

    /// Evict from hot cache
    fn evict_from_hot_cache(&self) {
        // Simple LRU eviction
        if let Some(oldest_key) = self.hot_cache.iter()
            .min_by_key(|entry| {
                self.contexts.get(entry.key())
                    .map(|e| e.accessed_at)
                    .unwrap_or(0)
            })
            .map(|entry| entry.key().clone()) {
            self.hot_cache.remove(&oldest_key);
        }
    }

    /// Update retrieval statistics
    fn update_retrieval_stats(&self, stats: &mut ContextStats, duration: std::time::Duration) {
        let duration_ns = duration.as_nanos() as u64;
        if stats.retrievals > 0 {
            stats.average_retrieval_time_ns = 
                (stats.average_retrieval_time_ns * (stats.retrievals - 1) as u64 + duration_ns) / stats.retrievals as u64;
        }
    }

    /// Get statistics
    pub fn stats(&self) -> ContextStats {
        self.stats.read().clone()
    }

    /// Subscribe to context events
    pub fn subscribe(&self) -> broadcast::Receiver<ContextEvent> {
        self.event_sender.subscribe()
    }
}

/// Context streamer - streams large contexts
struct ContextStreamer {
    contexts: Arc<DashMap<String, ContextEntry>>,
    context_id: String,
    position: usize,
}

impl ContextStreamer {
    fn new(contexts: Arc<DashMap<String, ContextEntry>>, context_id: String) -> Self {
        Self {
            contexts,
            context_id,
            position: 0,
        }
    }
}

impl Iterator for ContextStreamer {
    type Item = Result<Bytes>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(entry) = self.contexts.get(&self.context_id) {
            let chunk_size = 8192; // Default chunk size
            if self.position >= entry.content.len() {
                return None;
            }
            let end = (self.position + chunk_size).min(entry.content.len());
            let chunk = entry.content.slice(self.position..end);
            self.position = end;
            Some(Ok(chunk))
        } else {
            None
        }
    }
}

/// Cosine similarity for embeddings
impl InfiniteContextManager {
    fn cosine_similarity(v1: &[f32], v2: &[f32]) -> Result<f64> {
        if v1.len() != v2.len() {
            return Err(Error::Query("Vector dimensions mismatch".to_string()));
        }
        let dot_product: f32 = v1.iter().zip(v2.iter()).map(|(&a, &b)| a * b).sum();
        let magnitude1: f32 = v1.iter().map(|&a| a * a).sum::<f32>().sqrt();
        let magnitude2: f32 = v2.iter().map(|&b| b * b).sum::<f32>().sqrt();

        if magnitude1 == 0.0 || magnitude2 == 0.0 {
            Ok(0.0)
        } else {
            Ok((dot_product / (magnitude1 * magnitude2)) as f64)
        }
    }
}

use std::io::{Read, Write};
use tracing::debug;
use uuid;

