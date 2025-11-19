use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Schema mismatch: {0}")]
    SchemaMismatch(String),

    #[error("Column not found: {0}")]
    ColumnNotFound(String),

    #[error("Invalid data type: expected {expected}, got {actual}")]
    InvalidDataType { expected: String, actual: String },

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Transaction error: {0}")]
    Transaction(String),

    #[error("Index error: {0}")]
    Index(String),

    #[error("Concurrency error: {0}")]
    Concurrency(String),

    #[error("Configuration error: {0}")]
    Configuration(String),
}

pub type Result<T> = std::result::Result<T, Error>;

