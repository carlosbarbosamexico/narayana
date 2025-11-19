use crate::types::{TransactionId, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    Active,
    Committed,
    Aborted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: TransactionId,
    pub start_time: Timestamp,
    pub status: TransactionStatus,
    pub read_set: Vec<u64>, // Table IDs read
    pub write_set: Vec<u64>, // Table IDs written
}

impl Transaction {
    pub fn new(id: TransactionId) -> Self {
        Self {
            id,
            start_time: Timestamp::now(),
            status: TransactionStatus::Active,
            read_set: Vec::new(),
            write_set: Vec::new(),
        }
    }

    pub fn commit(&mut self) {
        self.status = TransactionStatus::Committed;
    }

    pub fn abort(&mut self) {
        self.status = TransactionStatus::Aborted;
    }
}

/// MVCC (Multi-Version Concurrency Control) version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    pub transaction_id: TransactionId,
    pub timestamp: Timestamp,
    pub data: Vec<u8>, // Serialized column data
}

pub struct TransactionManager {
    active_transactions: HashMap<TransactionId, Transaction>,
    next_transaction_id: u64,
}

impl TransactionManager {
    pub fn new() -> Self {
        Self {
            active_transactions: HashMap::new(),
            next_transaction_id: 1,
        }
    }

    pub fn begin_transaction(&mut self) -> TransactionId {
        let id = TransactionId(self.next_transaction_id);
        self.next_transaction_id += 1;
        let transaction = Transaction::new(id);
        self.active_transactions.insert(id, transaction);
        id
    }

    pub fn commit_transaction(&mut self, id: TransactionId) -> crate::Result<()> {
        if let Some(txn) = self.active_transactions.get_mut(&id) {
            txn.commit();
            self.active_transactions.remove(&id);
            Ok(())
        } else {
            Err(crate::Error::Transaction(format!("Transaction {} not found", id.0)))
        }
    }

    pub fn abort_transaction(&mut self, id: TransactionId) -> crate::Result<()> {
        if let Some(txn) = self.active_transactions.get_mut(&id) {
            txn.abort();
            self.active_transactions.remove(&id);
            Ok(())
        } else {
            Err(crate::Error::Transaction(format!("Transaction {} not found", id.0)))
        }
    }

    pub fn get_transaction(&self, id: TransactionId) -> Option<&Transaction> {
        self.active_transactions.get(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_creation() {
        let id = TransactionId(1);
        let txn = Transaction::new(id);
        assert_eq!(txn.id, id);
        assert_eq!(txn.status, TransactionStatus::Active);
        assert!(txn.read_set.is_empty());
        assert!(txn.write_set.is_empty());
    }

    #[test]
    fn test_transaction_commit() {
        let id = TransactionId(1);
        let mut txn = Transaction::new(id);
        txn.commit();
        assert_eq!(txn.status, TransactionStatus::Committed);
    }

    #[test]
    fn test_transaction_abort() {
        let id = TransactionId(1);
        let mut txn = Transaction::new(id);
        txn.abort();
        assert_eq!(txn.status, TransactionStatus::Aborted);
    }

    #[test]
    fn test_transaction_manager() {
        let mut manager = TransactionManager::new();
        
        let id1 = manager.begin_transaction();
        let id2 = manager.begin_transaction();
        
        assert_ne!(id1, id2);
        assert!(manager.get_transaction(id1).is_some());
        assert!(manager.get_transaction(id2).is_some());
        
        manager.commit_transaction(id1).unwrap();
        assert!(manager.get_transaction(id1).is_none());
        assert!(manager.get_transaction(id2).is_some());
        
        manager.abort_transaction(id2).unwrap();
        assert!(manager.get_transaction(id2).is_none());
    }

    #[test]
    fn test_transaction_manager_commit_nonexistent() {
        let mut manager = TransactionManager::new();
        let result = manager.commit_transaction(TransactionId(999));
        assert!(result.is_err());
    }

    #[test]
    fn test_transaction_manager_abort_nonexistent() {
        let mut manager = TransactionManager::new();
        let result = manager.abort_transaction(TransactionId(999));
        assert!(result.is_err());
    }
}
