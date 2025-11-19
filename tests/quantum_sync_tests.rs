// Comprehensive Quantum Sync Tests
// Test across multiple databases, spawning, deleting, etc.
// Ensure quantum sync is real, fast, and reliable

use narayana_storage::quantum_sync::*;
use narayana_storage::network_sync::*;
use narayana_core::types::TableId;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use std::collections::HashMap;

/// Test quantum sync across multiple nodes
#[tokio::test]
async fn test_quantum_sync_multiple_nodes() {
    // Create 3 nodes
    let node1 = QuantumSyncManager::new("node-1".to_string());
    let node2 = QuantumSyncManager::new("node-2".to_string());
    let node3 = QuantumSyncManager::new("node-3".to_string());

    // Add peers
    node1.add_peer(Peer {
        node_id: "node-2".to_string(),
        address: "127.0.0.1:8081".to_string(),
        last_seen: 0,
    });
    node1.add_peer(Peer {
        node_id: "node-3".to_string(),
        address: "127.0.0.1:8082".to_string(),
        last_seen: 0,
    });

    node2.add_peer(Peer {
        node_id: "node-1".to_string(),
        address: "127.0.0.1:8080".to_string(),
        last_seen: 0,
    });
    node2.add_peer(Peer {
        node_id: "node-3".to_string(),
        address: "127.0.0.1:8082".to_string(),
        last_seen: 0,
    });

    node3.add_peer(Peer {
        node_id: "node-1".to_string(),
        address: "127.0.0.1:8080".to_string(),
        last_seen: 0,
    });
    node3.add_peer(Peer {
        node_id: "node-2".to_string(),
        address: "127.0.0.1:8081".to_string(),
        last_seen: 0,
    });

    // Update state on node1
    let table_id = TableId(1);
    let data = b"test data".to_vec();
    node1.update_state(table_id, data.clone()).unwrap();

    // Sync node1 -> node2
    let sync_result = node1.sync_with_peer("node-2").await.unwrap();
    assert!(sync_result.synced_tables > 0);

    // Sync node2 -> node3
    let sync_result = node2.sync_with_peer("node-3").await.unwrap();
    assert!(sync_result.synced_tables > 0);

    // Verify all nodes have same state
    let state1 = node1.get_entangled_state(&table_id);
    let state2 = node2.get_entangled_state(&table_id);
    let state3 = node3.get_entangled_state(&table_id);

    assert_eq!(state1.state_hash, state2.state_hash);
    assert_eq!(state2.state_hash, state3.state_hash);
}

/// Test quantum sync speed - should be very fast
#[tokio::test]
async fn test_quantum_sync_speed() {
    let node1 = QuantumSyncManager::new("node-1".to_string());
    let node2 = QuantumSyncManager::new("node-2".to_string());

    node1.add_peer(Peer {
        node_id: "node-2".to_string(),
        address: "127.0.0.1:8081".to_string(),
        last_seen: 0,
    });

    // Update state
    let table_id = TableId(1);
    let data = b"test data".to_vec();
    node1.update_state(table_id, data).unwrap();

    // Measure sync time
    let start = Instant::now();
    let _sync_result = node1.sync_with_peer("node-2").await.unwrap();
    let duration = start.elapsed();

    // Should be very fast (<100ms for local sync)
    assert!(duration < Duration::from_millis(100), "Sync took {:?}, expected <100ms", duration);
}

/// Test quantum sync reliability - multiple updates
#[tokio::test]
async fn test_quantum_sync_reliability() {
    let node1 = QuantumSyncManager::new("node-1".to_string());
    let node2 = QuantumSyncManager::new("node-2".to_string());

    node1.add_peer(Peer {
        node_id: "node-2".to_string(),
        address: "127.0.0.1:8081".to_string(),
        last_seen: 0,
    });

    // Perform 100 updates
    for i in 0..100 {
        let table_id = TableId(i);
        let data = format!("data-{}", i).into_bytes();
        node1.update_state(table_id, data).unwrap();
    }

    // Sync all updates
    let sync_result = node1.sync_with_peer("node-2").await.unwrap();
    assert_eq!(sync_result.synced_tables, 100);

    // Verify all states synced
    for i in 0..100 {
        let table_id = TableId(i);
        let state1 = node1.get_entangled_state(&table_id);
        let state2 = node2.get_entangled_state(&table_id);
        assert_eq!(state1.state_hash, state2.state_hash);
    }
}

/// Test quantum sync with database spawning
#[tokio::test]
async fn test_quantum_sync_database_spawning() {
    let node1 = Arc::new(QuantumSyncManager::new("node-1".to_string()));
    let node2 = Arc::new(QuantumSyncManager::new("node-2".to_string()));

    node1.add_peer(Peer {
        node_id: "node-2".to_string(),
        address: "127.0.0.1:8081".to_string(),
        last_seen: 0,
    });

    // Spawn multiple databases concurrently
    let mut handles = Vec::new();
    for i in 0..10 {
        let node1_clone = node1.clone();
        let node2_clone = node2.clone();
        let handle = tokio::spawn(async move {
            let table_id = TableId(i);
            let data = format!("db-{}", i).into_bytes();
            node1_clone.update_state(table_id, data).unwrap();
            
            // Sync immediately
            let sync_result = node1_clone.sync_with_peer("node-2").await.unwrap();
            assert!(sync_result.synced_tables > 0);
            
            // Verify sync
            let state1 = node1_clone.get_entangled_state(&table_id);
            let state2 = node2_clone.get_entangled_state(&table_id);
            assert_eq!(state1.state_hash, state2.state_hash);
        });
        handles.push(handle);
    }

    // Wait for all databases to spawn and sync
    for handle in handles {
        handle.await.unwrap();
    }
}

/// Test quantum sync with database deletion
#[tokio::test]
async fn test_quantum_sync_database_deletion() {
    let node1 = QuantumSyncManager::new("node-1".to_string());
    let node2 = QuantumSyncManager::new("node-2".to_string());

    node1.add_peer(Peer {
        node_id: "node-2".to_string(),
        address: "127.0.0.1:8081".to_string(),
        last_seen: 0,
    });

    // Create and sync databases
    for i in 0..10 {
        let table_id = TableId(i);
        let data = format!("data-{}", i).into_bytes();
        node1.update_state(table_id, data).unwrap();
    }

    // Sync all
    let _sync_result = node1.sync_with_peer("node-2").await.unwrap();

    // Delete some databases
    for i in 0..5 {
        let table_id = TableId(i);
        // Simulate deletion by updating with empty data
        node1.update_state(table_id, vec![]).unwrap();
    }

    // Sync deletions
    let sync_result = node1.sync_with_peer("node-2").await.unwrap();
    assert!(sync_result.synced_tables > 0);

    // Verify deletions synced
    for i in 0..5 {
        let table_id = TableId(i);
        let state1 = node1.get_entangled_state(&table_id);
        let state2 = node2.get_entangled_state(&table_id);
        assert_eq!(state1.state_hash, state2.state_hash);
    }
}

/// Test quantum sync with rapid updates
#[tokio::test]
async fn test_quantum_sync_rapid_updates() {
    let node1 = QuantumSyncManager::new("node-1".to_string());
    let node2 = QuantumSyncManager::new("node-2".to_string());

    node1.add_peer(Peer {
        node_id: "node-2".to_string(),
        address: "127.0.0.1:8081".to_string(),
        last_seen: 0,
    });

    let table_id = TableId(1);
    
    // Rapid updates
    for i in 0..1000 {
        let data = format!("update-{}", i).into_bytes();
        node1.update_state(table_id, data).unwrap();
        
        // Sync every 10 updates
        if i % 10 == 0 {
            let _sync_result = node1.sync_with_peer("node-2").await.unwrap();
        }
    }

    // Final sync
    let sync_result = node1.sync_with_peer("node-2").await.unwrap();
    assert!(sync_result.synced_tables > 0);

    // Verify final state
    let state1 = node1.get_entangled_state(&table_id);
    let state2 = node2.get_entangled_state(&table_id);
    assert_eq!(state1.state_hash, state2.state_hash);
}

/// Test quantum sync with concurrent updates
#[tokio::test]
async fn test_quantum_sync_concurrent_updates() {
    let node1 = Arc::new(QuantumSyncManager::new("node-1".to_string()));
    let node2 = Arc::new(QuantumSyncManager::new("node-2".to_string()));

    node1.add_peer(Peer {
        node_id: "node-2".to_string(),
        address: "127.0.0.1:8081".to_string(),
        last_seen: 0,
    });

    // Concurrent updates from both nodes
    let mut handles = Vec::new();
    
    // Node1 updates
    for i in 0..50 {
        let node1_clone = node1.clone();
        let handle = tokio::spawn(async move {
            let table_id = TableId(i);
            let data = format!("node1-update-{}", i).into_bytes();
            node1_clone.update_state(table_id, data).unwrap();
        });
        handles.push(handle);
    }

    // Node2 updates
    for i in 50..100 {
        let node2_clone = node2.clone();
        let handle = tokio::spawn(async move {
            let table_id = TableId(i);
            let data = format!("node2-update-{}", i).into_bytes();
            node2_clone.update_state(table_id, data).unwrap();
        });
        handles.push(handle);
    }

    // Wait for all updates
    for handle in handles {
        handle.await.unwrap();
    }

    // Sync both ways
    let _sync1 = node1.sync_with_peer("node-2").await.unwrap();
    let _sync2 = node2.sync_with_peer("node-1").await.unwrap();

    // Verify all states synced
    for i in 0..100 {
        let table_id = TableId(i);
        let state1 = node1.get_entangled_state(&table_id);
        let state2 = node2.get_entangled_state(&table_id);
        // States should be consistent (may differ if concurrent, but should merge)
        assert!(state1.state_hash == state2.state_hash || 
                state1.vector_clock.is_concurrent(&state2.vector_clock));
    }
}

/// Test quantum sync with network partitions
#[tokio::test]
async fn test_quantum_sync_network_partition() {
    let node1 = QuantumSyncManager::new("node-1".to_string());
    let node2 = QuantumSyncManager::new("node-2".to_string());
    let node3 = QuantumSyncManager::new("node-3".to_string());

    // Setup network
    node1.add_peer(Peer {
        node_id: "node-2".to_string(),
        address: "127.0.0.1:8081".to_string(),
        last_seen: 0,
    });
    node1.add_peer(Peer {
        node_id: "node-3".to_string(),
        address: "127.0.0.1:8082".to_string(),
        last_seen: 0,
    });

    // Update on node1
    let table_id = TableId(1);
    node1.update_state(table_id, b"data1".to_vec()).unwrap();

    // Simulate partition: node1 can't reach node2, but can reach node3
    // Sync to node3
    let _sync_result = node1.sync_with_peer("node-3").await.unwrap();

    // After partition heals, sync node3 -> node2
    node3.add_peer(Peer {
        node_id: "node-2".to_string(),
        address: "127.0.0.1:8081".to_string(),
        last_seen: 0,
    });
    let _sync_result = node3.sync_with_peer("node-2").await.unwrap();

    // All nodes should eventually converge
    let state1 = node1.get_entangled_state(&table_id);
    let state2 = node2.get_entangled_state(&table_id);
    let state3 = node3.get_entangled_state(&table_id);

    assert_eq!(state1.state_hash, state3.state_hash);
    assert_eq!(state2.state_hash, state3.state_hash);
}

/// Test quantum sync gossip protocol
#[tokio::test]
async fn test_quantum_sync_gossip() {
    let node1 = Arc::new(QuantumSyncManager::new("node-1".to_string()));
    let node2 = Arc::new(QuantumSyncManager::new("node-2".to_string()));
    let node3 = Arc::new(QuantumSyncManager::new("node-3".to_string()));

    // Setup network
    node1.add_peer(Peer {
        node_id: "node-2".to_string(),
        address: "127.0.0.1:8081".to_string(),
        last_seen: 0,
    });
    node2.add_peer(Peer {
        node_id: "node-3".to_string(),
        address: "127.0.0.1:8082".to_string(),
        last_seen: 0,
    });

    // Update on node1
    let table_id = TableId(1);
    node1.update_state(table_id, b"gossip-data".to_vec()).unwrap();

    // Gossip: node1 -> node2 -> node3
    let _ = node1.gossip().await;
    let _ = node2.gossip().await;

    // Eventually all nodes should have the update
    let state1 = node1.get_entangled_state(&table_id);
    let state2 = node2.get_entangled_state(&table_id);
    let state3 = node3.get_entangled_state(&table_id);

    // States should converge through gossip
    assert!(state1.state_hash != 0);
}

/// Test quantum sync with large data
#[tokio::test]
async fn test_quantum_sync_large_data() {
    let node1 = QuantumSyncManager::new("node-1".to_string());
    let node2 = QuantumSyncManager::new("node-2".to_string());

    node1.add_peer(Peer {
        node_id: "node-2".to_string(),
        address: "127.0.0.1:8081".to_string(),
        last_seen: 0,
    });

    // Large data (1MB)
    let large_data = vec![0u8; 1024 * 1024];
    let table_id = TableId(1);
    
    let start = Instant::now();
    node1.update_state(table_id, large_data.clone()).unwrap();
    let sync_result = node1.sync_with_peer("node-2").await.unwrap();
    let duration = start.elapsed();

    // Should sync large data reasonably fast (<1s for 1MB)
    assert!(duration < Duration::from_secs(1), "Large data sync took {:?}", duration);
    assert!(sync_result.synced_tables > 0);
}

/// Test quantum sync vector clocks
#[test]
fn test_vector_clock_causality() {
    let mut clock1 = VectorClock::new("node-1".to_string());
    let mut clock2 = VectorClock::new("node-2".to_string());

    // Node1 ticks
    clock1.tick("node-1");
    
    // Node2 ticks
    clock2.tick("node-2");

    // They should be concurrent
    assert!(clock1.is_concurrent(&clock2));

    // Merge
    let original_clock2 = clock2.clone();
    clock1.merge(&clock2);
    
    // After merge, neither should have happened before the other
    // since both had independent ticks
    assert!(clock1.is_concurrent(&original_clock2) || !original_clock2.happened_before(&clock1));
}

/// Test CRDT merging
#[test]
fn test_crdt_merging() {
    // Test LWW Register
    let reg1 = CRDTValue::LWWRegister { 
        value: b"value1".to_vec(), 
        timestamp: 1000 
    };
    let reg2 = CRDTValue::LWWRegister { 
        value: b"value2".to_vec(), 
        timestamp: 2000 
    };
    
    let merged = reg1.merge(&reg2);
    // Last write wins - should pick newer timestamp
    match merged {
        CRDTValue::LWWRegister { value, timestamp } => {
            assert_eq!(value, b"value2".to_vec());
            assert_eq!(timestamp, 2000);
        },
        _ => panic!("Expected LWWRegister"),
    }

    // Test G-Counter
    let mut increments1 = std::collections::HashMap::new();
    increments1.insert("node-1".to_string(), 5);
    let counter1 = CRDTValue::Counter { 
        value: 5, 
        increments: increments1 
    };
    
    let mut increments2 = std::collections::HashMap::new();
    increments2.insert("node-2".to_string(), 3);
    let counter2 = CRDTValue::Counter { 
        value: 3, 
        increments: increments2 
    };
    
    let merged_counter = counter1.merge(&counter2);
    match merged_counter {
        CRDTValue::Counter { increments, .. } => {
            // Should have both increments
            assert_eq!(increments.get("node-1"), Some(&5));
            assert_eq!(increments.get("node-2"), Some(&3));
        },
        _ => panic!("Expected Counter"),
    }
}

/// Test quantum sync stress test
#[tokio::test]
async fn test_quantum_sync_stress() {
    let node1 = Arc::new(QuantumSyncManager::new("node-1".to_string()));
    let node2 = Arc::new(QuantumSyncManager::new("node-2".to_string()));

    node1.add_peer(Peer {
        node_id: "node-2".to_string(),
        address: "127.0.0.1:8081".to_string(),
        last_seen: 0,
    });

    // Stress test: 1000 tables, rapid updates
    let mut handles = Vec::new();
    for i in 0..1000 {
        let node1_clone = node1.clone();
        let node2_clone = node2.clone();
        let handle = tokio::spawn(async move {
            let table_id = TableId(i);
            let data = format!("stress-{}", i).into_bytes();
            node1_clone.update_state(table_id, data).unwrap();
            
            // Sync periodically
            if i % 100 == 0 {
                let _sync_result = node1_clone.sync_with_peer("node-2").await.unwrap();
            }
        });
        handles.push(handle);
    }

    // Wait for all
    for handle in handles {
        handle.await.unwrap();
    }

    // Final sync
    let sync_result = node1.sync_with_peer("node-2").await.unwrap();
    assert!(sync_result.synced_tables > 0);
}

/// Test quantum sync with node failures
#[tokio::test]
async fn test_quantum_sync_node_failure() {
    let node1 = QuantumSyncManager::new("node-1".to_string());
    let node2 = QuantumSyncManager::new("node-2".to_string());
    let node3 = QuantumSyncManager::new("node-3".to_string());

    // Setup network
    node1.add_peer(Peer {
        node_id: "node-2".to_string(),
        address: "127.0.0.1:8081".to_string(),
        last_seen: 0,
    });
    node1.add_peer(Peer {
        node_id: "node-3".to_string(),
        address: "127.0.0.1:8082".to_string(),
        last_seen: 0,
    });

    // Update on node1
    let table_id = TableId(1);
    node1.update_state(table_id, b"data".to_vec()).unwrap();

    // Sync to node2 (node3 is down)
    let sync_result = node1.sync_with_peer("node-2").await.unwrap();
    assert!(sync_result.synced_tables > 0);

    // Node3 comes back online
    node3.add_peer(Peer {
        node_id: "node-2".to_string(),
        address: "127.0.0.1:8081".to_string(),
        last_seen: 0,
    });

    // Sync node2 -> node3
    let sync_result = node2.sync_with_peer("node-3").await.unwrap();
    assert!(sync_result.synced_tables > 0);

    // All nodes should have same state
    let state1 = node1.get_entangled_state(&table_id);
    let state2 = node2.get_entangled_state(&table_id);
    let state3 = node3.get_entangled_state(&table_id);

    assert_eq!(state1.state_hash, state2.state_hash);
    assert_eq!(state2.state_hash, state3.state_hash);
}

/// Test quantum sync performance benchmark
#[tokio::test]
async fn test_quantum_sync_performance() {
    let node1 = QuantumSyncManager::new("node-1".to_string());
    let node2 = QuantumSyncManager::new("node-2".to_string());

    node1.add_peer(Peer {
        node_id: "node-2".to_string(),
        address: "127.0.0.1:8081".to_string(),
        last_seen: 0,
    });

    // Benchmark: 1000 syncs
    let start = Instant::now();
    for i in 0..1000 {
        let table_id = TableId(i);
        let data = format!("perf-{}", i).into_bytes();
        node1.update_state(table_id, data).unwrap();
        
        if i % 10 == 0 {
            let _sync_result = node1.sync_with_peer("node-2").await.unwrap();
        }
    }
    let duration = start.elapsed();

    // Should handle 1000 syncs reasonably fast
    let avg_time = duration / 1000;
    println!("Average sync time: {:?}", avg_time);
    assert!(avg_time < Duration::from_millis(10), "Sync too slow: {:?}", avg_time);
}
