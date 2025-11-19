// Advanced consensus algorithms for quantum-like synchronization

use narayana_core::{Error, Result};
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Raft consensus for strong consistency
pub struct RaftConsensus {
    node_id: String,
    state: Arc<RwLock<RaftState>>,
    peers: Arc<RwLock<Vec<String>>>,
}

#[derive(Debug, Clone)]
enum RaftState {
    Follower { leader: Option<String>, term: u64 },
    Candidate { term: u64, votes: usize },
    Leader { term: u64 },
}

impl RaftConsensus {
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            state: Arc::new(RwLock::new(RaftState::Follower {
                leader: None,
                term: 0,
            })),
            peers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Request vote (election)
    pub async fn request_vote(&self, term: u64) -> bool {
        let mut state = self.state.write();
        match *state {
            RaftState::Follower { term: t, .. } | RaftState::Candidate { term: t, .. } => {
                if term > t {
                    *state = RaftState::Follower {
                        leader: None,
                        term,
                    };
                    return true;
                }
            }
            _ => {}
        }
        false
    }

    /// Append entries (replication)
    pub async fn append_entries(&self, term: u64, entries: Vec<ConsensusEntry>) -> bool {
        let mut state = self.state.write();
        match *state {
            RaftState::Follower { term: t, .. } => {
                if term >= t {
                    *state = RaftState::Follower {
                        leader: None,
                        term,
                    };
                    return true;
                }
            }
            _ => {}
        }
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusEntry {
    pub term: u64,
    pub index: u64,
    pub data: Vec<u8>,
}

/// PBFT (Practical Byzantine Fault Tolerance) for Byzantine fault tolerance
pub struct PBFTConsensus {
    node_id: String,
    pub f: usize, // Maximum faulty nodes
    state: Arc<RwLock<PBFTState>>,
}

#[derive(Debug, Clone)]
enum PBFTState {
    PrePrepare { view: u64, sequence: u64 },
    Prepare { view: u64, sequence: u64, prepares: usize },
    Commit { view: u64, sequence: u64, commits: usize },
}

impl PBFTConsensus {
    pub fn new(node_id: String, total_nodes: usize) -> Self {
        let f = (total_nodes - 1) / 3; // PBFT requires 3f+1 nodes
        Self {
            node_id,
            f,
            state: Arc::new(RwLock::new(PBFTState::PrePrepare {
                view: 0,
                sequence: 0,
            })),
        }
    }

    /// Pre-prepare phase
    pub async fn pre_prepare(&self, view: u64, sequence: u64, data: Vec<u8>) -> Result<()> {
        let mut state = self.state.write();
        *state = PBFTState::PrePrepare { view, sequence };
        Ok(())
    }

    /// Prepare phase (need 2f prepares)
    pub async fn prepare(&self, view: u64, sequence: u64) -> bool {
        let mut state = self.state.write();
        match *state {
            PBFTState::PrePrepare { view: v, sequence: s } if v == view && s == sequence => {
                *state = PBFTState::Prepare {
                    view,
                    sequence,
                    prepares: 1,
                };
                true
            }
            PBFTState::Prepare { view: v, sequence: s, ref mut prepares } if v == view && s == sequence => {
                *prepares += 1;
                *prepares >= 2 * self.f
            }
            _ => false,
        }
    }

    /// Commit phase (need 2f+1 commits)
    pub async fn commit(&self, view: u64, sequence: u64) -> bool {
        let mut state = self.state.write();
        match *state {
            PBFTState::Prepare { view: v, sequence: s, prepares } 
                if v == view && s == sequence && prepares >= 2 * self.f => {
                *state = PBFTState::Commit {
                    view,
                    sequence,
                    commits: 1,
                };
                true
            }
            PBFTState::Commit { view: v, sequence: s, ref mut commits } 
                if v == view && s == sequence => {
                *commits += 1;
                *commits >= 2 * self.f + 1
            }
            _ => false,
        }
    }
}

/// CRDT-based consensus (no leader, automatic merging)
pub struct CRDTConsensus {
    crdt_states: Arc<RwLock<HashMap<String, CRDTValue>>>,
}

impl CRDTConsensus {
    pub fn new() -> Self {
        Self {
            crdt_states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Update CRDT state (automatically merges)
    pub fn update(&self, key: String, value: CRDTValue) {
        let mut states = self.crdt_states.write();
        if let Some(existing) = states.get(&key).cloned() {
            states.insert(key, existing.merge(&value));
        } else {
            states.insert(key, value);
        }
    }

    /// Get merged state
    pub fn get(&self, key: &str) -> Option<CRDTValue> {
        let states = self.crdt_states.read();
        states.get(key).cloned()
    }
}

use crate::quantum_sync::CRDTValue;

/// Vector clock-based consensus (causality-based)
pub struct VectorClockConsensus {
    vector_clocks: Arc<RwLock<HashMap<String, VectorClock>>>,
}

use crate::quantum_sync::VectorClock;

impl VectorClockConsensus {
    pub fn new() -> Self {
        Self {
            vector_clocks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if operation should be applied (causality check)
    pub fn should_apply(&self, key: &str, new_clock: &VectorClock) -> bool {
        let clocks = self.vector_clocks.read();
        if let Some(existing) = clocks.get(key) {
            // Apply if new clock happened after existing
            existing.happened_before(new_clock)
        } else {
            true // First time seeing this key
        }
    }

    /// Update vector clock
    pub fn update_clock(&self, key: String, clock: VectorClock) {
        let mut clocks = self.vector_clocks.write();
        if let Some(existing) = clocks.get_mut(&key) {
            existing.merge(&clock);
        } else {
            clocks.insert(key, clock);
        }
    }
}

