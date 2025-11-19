// Sharding support for infinite horizontal scalability

use narayana_core::types::TableId;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

/// Shard key for distributing data across nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShardKey(pub u64);

/// Shard configuration for distributed storage
#[derive(Debug, Clone)]
pub struct ShardConfig {
    pub shard_count: usize,
    pub replication_factor: usize,
}

impl ShardConfig {
    pub fn new(shard_count: usize, replication_factor: usize) -> Self {
        Self {
            shard_count,
            replication_factor,
        }
    }

    /// Default configuration for single-node deployment
    pub fn single_node() -> Self {
        Self {
            shard_count: 1,
            replication_factor: 1,
        }
    }
}

/// Shard router for determining which shard a table belongs to
pub struct ShardRouter {
    config: ShardConfig,
}

impl ShardRouter {
    pub fn new(config: ShardConfig) -> Self {
        Self { config }
    }

    /// Determine shard ID for a table
    pub fn get_shard(&self, table_id: TableId) -> usize {
        // EDGE CASE: Prevent division by zero if shard_count is 0
        if self.config.shard_count == 0 {
            return 0; // Default to shard 0 if no shards configured
        }
        let mut hasher = DefaultHasher::new();
        table_id.hash(&mut hasher);
        (hasher.finish() as usize) % self.config.shard_count
    }

    /// Get all replica shards for a table
    pub fn get_replicas(&self, table_id: TableId) -> Vec<usize> {
        // EDGE CASE: Prevent division by zero if shard_count is 0
        if self.config.shard_count == 0 {
            return vec![0]; // Default to shard 0 if no shards configured
        }
        let primary_shard = self.get_shard(table_id);
        (0..self.config.replication_factor)
            .map(|i| (primary_shard + i) % self.config.shard_count)
            .collect()
    }

    /// Determine if this node should handle a table
    pub fn should_handle(&self, table_id: TableId, node_id: usize) -> bool {
        self.get_replicas(table_id).contains(&node_id)
    }
}

/// Consistent hashing for shard distribution
pub struct ConsistentHasher {
    ring: Vec<(u64, usize)>, // (hash, shard_id)
}

impl ConsistentHasher {
    pub fn new(shard_count: usize) -> Self {
        let mut ring = Vec::new();
        for shard_id in 0..shard_count {
            // Create multiple virtual nodes per shard for better distribution
            for vnode in 0..100 {
                let mut hasher = DefaultHasher::new();
                shard_id.hash(&mut hasher);
                vnode.hash(&mut hasher);
                ring.push((hasher.finish(), shard_id));
            }
        }
        ring.sort_by_key(|(hash, _)| *hash);
        Self { ring }
    }

    /// Get shard for a key using consistent hashing
    pub fn get_shard(&self, key: &[u8]) -> usize {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hash = hasher.finish();

        // Find first node with hash >= key hash
        match self.ring.binary_search_by_key(&hash, |(h, _)| *h) {
            Ok(idx) => self.ring[idx].1,
            Err(idx) => {
                if idx < self.ring.len() {
                    self.ring[idx].1
                } else {
                    // Wrap around
                    self.ring[0].1
                }
            }
        }
    }
}

/// Shard metadata for tracking data distribution
#[derive(Debug, Clone)]
pub struct ShardMetadata {
    pub shard_id: usize,
    pub node_id: usize,
    pub table_count: usize,
    pub data_size: u64,
}

impl ShardMetadata {
    pub fn new(shard_id: usize, node_id: usize) -> Self {
        Self {
            shard_id,
            node_id,
            table_count: 0,
            data_size: 0,
        }
    }
}

/// Distributed storage coordinator
pub struct DistributedCoordinator {
    router: ShardRouter,
    local_node_id: usize,
}

impl DistributedCoordinator {
    pub fn new(config: ShardConfig, local_node_id: usize) -> Self {
        Self {
            router: ShardRouter::new(config),
            local_node_id,
        }
    }

    /// Check if this node should handle the request
    pub fn is_local(&self, table_id: TableId) -> bool {
        self.router.should_handle(table_id, self.local_node_id)
    }

    /// Get all nodes that should handle this table
    pub fn get_nodes(&self, table_id: TableId) -> Vec<usize> {
        self.router.get_replicas(table_id)
    }
}

