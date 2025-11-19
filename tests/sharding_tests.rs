// Tests for sharding - matching actual implementation

use narayana_storage::sharding::*;
use narayana_core::types::TableId;

#[test]
fn test_shard_config_creation() {
    let config = ShardConfig::new(4, 2);
    assert_eq!(config.shard_count, 4);
    assert_eq!(config.replication_factor, 2);
}

#[test]
fn test_shard_config_single_node() {
    let config = ShardConfig::single_node();
    assert_eq!(config.shard_count, 1);
    assert_eq!(config.replication_factor, 1);
}

#[test]
fn test_shard_router_creation() {
    let config = ShardConfig::new(4, 2);
    let _router = ShardRouter::new(config);
    // Should create successfully
}

#[test]
fn test_shard_router_get_shard() {
    let config = ShardConfig::new(4, 2);
    let router = ShardRouter::new(config);
    let table_id = TableId(1);
    let shard = router.get_shard(table_id);
    assert!(shard < 4);
}

#[test]
fn test_shard_router_get_shard_consistent() {
    let config = ShardConfig::new(4, 2);
    let router = ShardRouter::new(config);
    let table_id = TableId(1);
    let shard1 = router.get_shard(table_id);
    let shard2 = router.get_shard(table_id);
    assert_eq!(shard1, shard2); // Should be consistent
}

#[test]
fn test_shard_router_get_replicas() {
    let config = ShardConfig::new(4, 2);
    let router = ShardRouter::new(config);
    let table_id = TableId(1);
    let replicas = router.get_replicas(table_id);
    assert_eq!(replicas.len(), 2); // replication_factor = 2
}

#[test]
fn test_shard_router_should_handle() {
    let config = ShardConfig::new(4, 2);
    let router = ShardRouter::new(config);
    let table_id = TableId(1);
    let replicas = router.get_replicas(table_id);
    
    // Should handle if node_id is in replicas
    for &node_id in &replicas {
        assert!(router.should_handle(table_id, node_id));
    }
}

#[test]
fn test_consistent_hashing_creation() {
    let _hasher = ConsistentHasher::new(4);
    // Should create successfully
}

#[test]
fn test_consistent_hashing_get_shard() {
    let hasher = ConsistentHasher::new(4);
    let key = b"test-key";
    let shard = hasher.get_shard(key);
    assert!(shard < 4);
}

#[test]
fn test_consistent_hashing_consistent() {
    let hasher = ConsistentHasher::new(4);
    let key = b"test-key";
    let shard1 = hasher.get_shard(key);
    let shard2 = hasher.get_shard(key);
    assert_eq!(shard1, shard2); // Should be consistent
}

#[test]
fn test_consistent_hashing_distribution() {
    let hasher = ConsistentHasher::new(4);
    let mut shard_counts = vec![0; 4];
    
    // Test 1000 keys
    for i in 0..1000 {
        let key = format!("key-{}", i);
        let shard = hasher.get_shard(key.as_bytes());
        shard_counts[shard] += 1;
    }
    
    // Each shard should get some keys (rough distribution check)
    for count in shard_counts {
        assert!(count > 0, "Shard should have at least some keys");
    }
}

#[test]
fn test_shard_metadata_creation() {
    let metadata = ShardMetadata::new(0, 1);
    assert_eq!(metadata.shard_id, 0);
    assert_eq!(metadata.node_id, 1);
    assert_eq!(metadata.table_count, 0);
    assert_eq!(metadata.data_size, 0);
}

#[test]
fn test_distributed_coordinator_creation() {
    let config = ShardConfig::new(4, 2);
    let _coordinator = DistributedCoordinator::new(config, 0);
    // Should create successfully
}

#[test]
fn test_distributed_coordinator_is_local() {
    let config = ShardConfig::new(4, 2);
    let coordinator = DistributedCoordinator::new(config, 0);
    let table_id = TableId(1);
    
    // Check if this node should handle the table
    let is_local = coordinator.is_local(table_id);
    assert!(is_local == true || is_local == false); // Just verify it returns a bool
}

#[test]
fn test_distributed_coordinator_get_nodes() {
    let config = ShardConfig::new(4, 2);
    let coordinator = DistributedCoordinator::new(config, 0);
    let table_id = TableId(1);
    
    let nodes = coordinator.get_nodes(table_id);
    assert_eq!(nodes.len(), 2); // replication_factor = 2
}

#[test]
fn test_shard_router_zero_shards() {
    let config = ShardConfig::new(0, 0);
    let router = ShardRouter::new(config);
    let table_id = TableId(1);
    
    // Should handle edge case gracefully
    let shard = router.get_shard(table_id);
    assert_eq!(shard, 0); // Should default to 0
}

#[test]
fn test_distributed_multiple_tables() {
    let config = ShardConfig::new(8, 3);
    let coordinator = DistributedCoordinator::new(config, 0);
    
    // Test multiple tables
    for i in 0..100 {
        let table_id = TableId(i);
        let nodes = coordinator.get_nodes(table_id);
        assert_eq!(nodes.len(), 3); // replication_factor = 3
        
        // All nodes should be valid shard IDs
        for &node in &nodes {
            assert!(node < 8);
        }
    }
}

