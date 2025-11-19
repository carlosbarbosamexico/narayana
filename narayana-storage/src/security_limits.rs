// Security limits to prevent resource exhaustion attacks

/// Maximum number of webhooks per database/table/scope
pub const MAX_WEBHOOKS_PER_SCOPE: usize = 1000;

/// Maximum number of webhooks globally
pub const MAX_WEBHOOKS_GLOBAL: usize = 10_000;

/// Maximum webhook URL length
pub const MAX_WEBHOOK_URL_LENGTH: usize = 2048;

/// Maximum webhook name length
pub const MAX_WEBHOOK_NAME_LENGTH: usize = 255;

/// Maximum webhook payload size
pub const MAX_WEBHOOK_PAYLOAD_SIZE: usize = 10 * 1024 * 1024; // 10MB

/// Maximum number of webhook headers
pub const MAX_WEBHOOK_HEADERS: usize = 50;

/// Maximum webhook header key length
pub const MAX_WEBHOOK_HEADER_KEY_LENGTH: usize = 256;

/// Maximum webhook header value length
pub const MAX_WEBHOOK_HEADER_VALUE_LENGTH: usize = 8192;

/// Maximum number of tables per database
pub const MAX_TABLES_PER_DATABASE: usize = 100_000;

/// Maximum number of databases
pub const MAX_DATABASES: usize = 10_000;

/// Maximum schema fields per table
pub const MAX_SCHEMA_FIELDS: usize = 10_000;

/// Maximum field name length
pub const MAX_FIELD_NAME_LENGTH: usize = 255;

/// Maximum table name length
pub const MAX_TABLE_NAME_LENGTH: usize = 255;

/// Maximum database name length
pub const MAX_DATABASE_NAME_LENGTH: usize = 255;

/// Maximum query result size (in rows)
pub const MAX_QUERY_RESULT_ROWS: usize = 1_000_000;

/// Maximum query result size (in bytes)
pub const MAX_QUERY_RESULT_SIZE: usize = 100 * 1024 * 1024; // 100MB

/// Maximum batch operation size
pub const MAX_BATCH_SIZE: usize = 100_000;

/// Maximum string length in column
pub const MAX_STRING_LENGTH: usize = 10 * 1024 * 1024; // 10MB

/// Maximum binary length in column
pub const MAX_BINARY_LENGTH: usize = 100 * 1024 * 1024; // 100MB

/// Maximum column count per table
pub const MAX_COLUMNS_PER_TABLE: usize = 10_000;

/// Maximum number of concurrent connections
pub const MAX_CONCURRENT_CONNECTIONS: usize = 100_000;

/// Maximum request body size
pub const MAX_REQUEST_BODY_SIZE: usize = 100 * 1024 * 1024; // 100MB

/// Maximum query length
pub const MAX_QUERY_LENGTH: usize = 1_000_000; // 1MB

/// Maximum number of query parameters
pub const MAX_QUERY_PARAMETERS: usize = 1000;

/// Validate size against limit
pub fn validate_size(value: usize, limit: usize, resource: &str) -> Result<(), narayana_core::Error> {
    if value > limit {
        return Err(narayana_core::Error::Storage(format!(
            "{} size {} exceeds maximum {}",
            resource, value, limit
        )));
    }
    Ok(())
}

/// Validate string length
pub fn validate_string_length(s: &str, limit: usize, resource: &str) -> Result<(), narayana_core::Error> {
    if s.len() > limit {
        return Err(narayana_core::Error::Storage(format!(
            "{} length {} exceeds maximum {}",
            resource, s.len(), limit
        )));
    }
    Ok(())
}

/// Validate collection size
pub fn validate_collection_size<T>(collection: &[T], limit: usize, resource: &str) -> Result<(), narayana_core::Error> {
    if collection.len() > limit {
        return Err(narayana_core::Error::Storage(format!(
            "{} count {} exceeds maximum {}",
            resource, collection.len(), limit
        )));
    }
    Ok(())
}

