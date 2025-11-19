// Tests for consensus algorithms

use narayana_storage::consensus::*;

#[test]
fn test_raft_consensus_creation() {
    let raft = RaftConsensus::new("node-1".to_string());
    // Raft should start as follower
}

#[tokio::test]
async fn test_raft_request_vote() {
    let raft = RaftConsensus::new("node-1".to_string());
    let voted = raft.request_vote(1).await;
    assert!(voted);
}

#[tokio::test]
async fn test_raft_append_entries() {
    let raft = RaftConsensus::new("node-1".to_string());
    let entries = vec![ConsensusEntry {
        term: 1,
        index: 1,
        data: b"data".to_vec(),
    }];
    
    let success = raft.append_entries(1, entries).await;
    assert!(success);
}

#[test]
fn test_pbft_consensus_creation() {
    let pbft = PBFTConsensus::new("node-1".to_string(), 4);
    assert_eq!(pbft.f, 1); // (4-1)/3 = 1
}

#[tokio::test]
async fn test_pbft_pre_prepare() {
    let pbft = PBFTConsensus::new("node-1".to_string(), 4);
    let result = pbft.pre_prepare(0, 1, b"data".to_vec()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_pbft_prepare() {
    let pbft = PBFTConsensus::new("node-1".to_string(), 4);
    pbft.pre_prepare(0, 1, b"data".to_vec()).await.unwrap();
    let prepared = pbft.prepare(0, 1).await;
    assert!(prepared);
}

#[tokio::test]
async fn test_pbft_commit() {
    let pbft = PBFTConsensus::new("node-1".to_string(), 4);
    pbft.pre_prepare(0, 1, b"data".to_vec()).await.unwrap();
    pbft.prepare(0, 1).await;
    let committed = pbft.commit(0, 1).await;
    assert!(committed);
}

#[test]
fn test_crdt_consensus_creation() {
    let crdt = CRDTConsensus::new();
    // Should create successfully
}

#[test]
fn test_crdt_consensus_update() {
    let crdt = CRDTConsensus::new();
    let value = CRDTValue::LWWRegister {
        value: b"data".to_vec(),
        timestamp: 100,
    };
    
    crdt.update("key".to_string(), value);
    let retrieved = crdt.get("key");
    assert!(retrieved.is_some());
}

#[test]
fn test_vector_clock_consensus_creation() {
    let vc = VectorClockConsensus::new();
    // Should create successfully
}

#[test]
fn test_vector_clock_consensus_should_apply() {
    let vc = VectorClockConsensus::new();
    let new_clock = VectorClock::new("node-1".to_string());
    
    let should_apply = vc.should_apply("key", &new_clock);
    assert!(should_apply);
}

#[test]
fn test_vector_clock_consensus_update() {
    let vc = VectorClockConsensus::new();
    let clock = VectorClock::new("node-1".to_string());
    
    vc.update_clock("key".to_string(), clock);
    // Should update successfully
}

use narayana_storage::quantum_sync::CRDTValue;
use narayana_storage::quantum_sync::VectorClock;

