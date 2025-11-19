use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for a table
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TableId(pub u64);

/// Unique identifier for a column
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ColumnId(pub u32);

/// Unique identifier for a transaction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransactionId(pub u64);

/// Timestamp for MVCC
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp(pub u64);

impl Timestamp {
    pub fn now() -> Self {
        Self(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        )
    }
}

/// Compression algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionType {
    None,
    LZ4,
    Zstd,
    Snappy,
}

/// Storage format for columns
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageFormat {
    Dense,      // Dense array storage
    Sparse,     // Sparse storage with indices
    Dictionary, // Dictionary-encoded
    Delta,      // Delta-encoded
}

impl fmt::Display for CompressionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompressionType::None => write!(f, "none"),
            CompressionType::LZ4 => write!(f, "lz4"),
            CompressionType::Zstd => write!(f, "zstd"),
            CompressionType::Snappy => write!(f, "snappy"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_id() {
        let id1 = TableId(1);
        let id2 = TableId(2);
        assert_ne!(id1, id2);
        assert_eq!(id1, TableId(1));
    }

    #[test]
    fn test_timestamp() {
        let ts1 = Timestamp::now();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let ts2 = Timestamp::now();
        assert!(ts2 > ts1);
    }

    #[test]
    fn test_compression_type_display() {
        assert_eq!(CompressionType::LZ4.to_string(), "lz4");
        assert_eq!(CompressionType::Zstd.to_string(), "zstd");
        assert_eq!(CompressionType::Snappy.to_string(), "snappy");
        assert_eq!(CompressionType::None.to_string(), "none");
    }

    #[test]
    fn test_transaction_id() {
        let id1 = TransactionId(100);
        let id2 = TransactionId(100);
        assert_eq!(id1, id2);
    }
}
