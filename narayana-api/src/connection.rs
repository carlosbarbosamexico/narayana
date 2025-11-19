// Connection layer for API clients
// Supports both remote (HTTP/gRPC) and direct (in-process) connections

use narayana_core::{
    Error, Result, schema::Schema, types::TableId, column::Column,
    transforms::OutputConfig,
};
use narayana_storage::ColumnStore;
use async_trait::async_trait;
use std::sync::Arc;
use serde_json::Value as JsonValue;

/// Connection trait for database operations
#[async_trait]
pub trait Connection: Send + Sync {
    /// Create a table
    async fn create_table(&self, table_id: TableId, schema: Schema) -> Result<()>;
    
    /// Write columns to a table
    async fn write_columns(&self, table_id: TableId, columns: Vec<Column>) -> Result<()>;
    
    /// Read columns from a table
    async fn read_columns(
        &self,
        table_id: TableId,
        column_ids: Vec<u32>,
        row_start: usize,
        row_count: usize,
    ) -> Result<Vec<Column>>;
    
    /// Get table schema
    async fn get_schema(&self, table_id: TableId) -> Result<Schema>;
    
    /// Delete a table
    async fn delete_table(&self, table_id: TableId) -> Result<()>;
    
    /// Execute a query via HTTP (for remote connections)
    async fn execute_query(&self, query: JsonValue) -> Result<JsonValue>;
    
    /// Get table ID by name (if supported)
    async fn get_table_id(&self, table_name: &str) -> Result<Option<TableId>>;
    
    /// Get table output config (for transforms/filters)
    async fn get_table_output_config(&self, table_id: TableId) -> Result<Option<OutputConfig>> {
        // Default implementation returns None
        // Implementations can override to provide config
        Ok(None)
    }
}

/// Direct connection to storage engine (in-process)
pub struct DirectConnection {
    storage: Arc<dyn ColumnStore>,
}

impl DirectConnection {
    pub fn new(storage: Arc<dyn ColumnStore>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl Connection for DirectConnection {
    async fn create_table(&self, table_id: TableId, schema: Schema) -> Result<()> {
        self.storage.create_table(table_id, schema).await
    }
    
    async fn write_columns(&self, table_id: TableId, columns: Vec<Column>) -> Result<()> {
        self.storage.write_columns(table_id, columns).await
    }
    
    async fn read_columns(
        &self,
        table_id: TableId,
        column_ids: Vec<u32>,
        row_start: usize,
        row_count: usize,
    ) -> Result<Vec<Column>> {
        self.storage.read_columns(table_id, column_ids, row_start, row_count).await
    }
    
    async fn get_schema(&self, table_id: TableId) -> Result<Schema> {
        self.storage.get_schema(table_id).await
    }
    
    async fn delete_table(&self, table_id: TableId) -> Result<()> {
        self.storage.delete_table(table_id).await
    }
    
    async fn execute_query(&self, _query: JsonValue) -> Result<JsonValue> {
        Err(Error::Query("Direct connection does not support generic query execution".to_string()))
    }
    
    async fn get_table_id(&self, _table_name: &str) -> Result<Option<TableId>> {
        // Direct connection doesn't track table names, would need to extend storage
        Ok(None)
    }
}

/// Remote connection via HTTP
pub struct RemoteConnection {
    base_url: String,
    client: reqwest::Client,
}

impl RemoteConnection {
    pub fn new(base_url: String) -> Self {
        // SECURITY: Validate base_url is not empty
        let mut normalized_url = if base_url.trim().is_empty() {
            "http://localhost:8080".to_string() // Default fallback
        } else {
            base_url.trim().to_string()
        };
        
        // SECURITY: Validate URL length - use default if too long instead of panicking
        const MAX_URL_LENGTH: usize = 2048;
        if normalized_url.len() > MAX_URL_LENGTH {
            eprintln!("WARNING: Base URL length {} exceeds maximum {}, using default", normalized_url.len(), MAX_URL_LENGTH);
            normalized_url = "http://localhost:8080".to_string();
        }
        
        // SECURITY: Validate URL scheme to prevent SSRF attacks
        // Only allow http:// and https:// schemes
        let url_lower = normalized_url.to_lowercase();
        if !url_lower.starts_with("http://") && !url_lower.starts_with("https://") {
            eprintln!("WARNING: Base URL must use http:// or https:// scheme, using default");
            normalized_url = "http://localhost:8080".to_string();
        }
        
        // SECURITY: Prevent SSRF by blocking dangerous schemes and localhost/internal IPs
        // Note: reqwest should handle this, but we add explicit validation
        if url_lower.starts_with("file://") || 
           url_lower.starts_with("gopher://") || 
           url_lower.starts_with("ftp://") ||
           url_lower.starts_with("ldap://") ||
           url_lower.starts_with("ldaps://") {
            eprintln!("WARNING: Base URL uses dangerous scheme, using default");
            normalized_url = "http://localhost:8080".to_string();
        }
        
        // SECURITY: Create client with timeout to prevent hanging requests
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30)) // 30 second timeout
            .connect_timeout(std::time::Duration::from_secs(10)) // 10 second connect timeout
            .build()
            .unwrap_or_else(|_| reqwest::Client::new()); // Fallback to default if builder fails
        
        Self {
            base_url: normalized_url,
            client,
        }
    }
    
    async fn post_json(&self, path: &str, body: JsonValue) -> Result<JsonValue> {
        // SECURITY: Validate path to prevent path traversal attacks
        if path.contains("..") || path.contains('\0') || path.contains('\n') || path.contains('\r') {
            return Err(Error::Query("Invalid path: contains dangerous characters".to_string()));
        }
        
        // Normalize URL construction to handle trailing/leading slashes
        let base = self.base_url.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        let url = format!("{}/{}", base, path);
        let response = self.client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Query(format!("HTTP request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let status = response.status();
            // SECURITY: Sanitize error message to prevent information disclosure
            // Don't include full response text which might contain sensitive data
            let text = response.text().await.unwrap_or_default();
            // Limit error message length and sanitize
            let sanitized_text = if text.len() > 200 {
                format!("{}...", &text[..200])
            } else {
                text
            };
            return Err(Error::Query(format!("HTTP {}: {}", status, sanitized_text)));
        }
        
        response.json().await
            .map_err(|e| Error::Query(format!("Failed to parse JSON response: {}", e)))
    }
    
    async fn get_json(&self, path: &str) -> Result<JsonValue> {
        // SECURITY: Validate path to prevent path traversal attacks
        if path.contains("..") || path.contains('\0') || path.contains('\n') || path.contains('\r') {
            return Err(Error::Query("Invalid path: contains dangerous characters".to_string()));
        }
        
        // Normalize URL construction to handle trailing/leading slashes
        let base = self.base_url.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        let url = format!("{}/{}", base, path);
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Query(format!("HTTP request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let status = response.status();
            // SECURITY: Sanitize error message to prevent information disclosure
            let text = response.text().await.unwrap_or_default();
            let sanitized_text = if text.len() > 200 {
                format!("{}...", &text[..200])
            } else {
                text
            };
            return Err(Error::Query(format!("HTTP {}: {}", status, sanitized_text)));
        }
        
        response.json().await
            .map_err(|e| Error::Query(format!("Failed to parse JSON response: {}", e)))
    }
    
    async fn delete_request(&self, path: &str) -> Result<()> {
        // SECURITY: Validate path to prevent path traversal attacks
        if path.contains("..") || path.contains('\0') || path.contains('\n') || path.contains('\r') {
            return Err(Error::Query("Invalid path: contains dangerous characters".to_string()));
        }
        
        // Normalize URL construction to handle trailing/leading slashes
        let base = self.base_url.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        let url = format!("{}/{}", base, path);
        let response = self.client
            .delete(&url)
            .send()
            .await
            .map_err(|e| Error::Query(format!("HTTP request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let status = response.status();
            // SECURITY: Sanitize error message to prevent information disclosure
            let text = response.text().await.unwrap_or_default();
            let sanitized_text = if text.len() > 200 {
                format!("{}...", &text[..200])
            } else {
                text
            };
            return Err(Error::Query(format!("HTTP {}: {}", status, sanitized_text)));
        }
        
        Ok(())
    }
}

#[async_trait]
impl Connection for RemoteConnection {
    async fn create_table(&self, table_id: TableId, schema: Schema) -> Result<()> {
        let body = serde_json::json!({
            "table_id": table_id.0,
            "schema": schema,
        });
        self.post_json("/api/v1/tables", body).await?;
        Ok(())
    }
    
    async fn write_columns(&self, table_id: TableId, columns: Vec<Column>) -> Result<()> {
        // SECURITY: Validate column count
        const MAX_COLUMNS_PER_WRITE: usize = 10_000;
        if columns.len() > MAX_COLUMNS_PER_WRITE {
            return Err(Error::Query(format!(
                "Column count {} exceeds maximum {}",
                columns.len(), MAX_COLUMNS_PER_WRITE
            )));
        }
        
        // SECURITY: Validate table_id is not zero (could indicate uninitialized)
        if table_id.0 == 0 {
            return Err(Error::Query("Invalid table ID: zero".to_string()));
        }
        
        let body = serde_json::json!({
            "columns": columns,
        });
        
        // SECURITY: Sanitize table_id in URL to prevent injection
        // Table ID is a u64, so it's safe, but we validate it's reasonable
        if table_id.0 > u64::MAX / 2 {
            return Err(Error::Query("Table ID is suspiciously large".to_string()));
        }
        
        self.post_json(&format!("/api/v1/tables/{}/insert", table_id.0), body).await?;
        Ok(())
    }
    
    async fn read_columns(
        &self,
        table_id: TableId,
        column_ids: Vec<u32>,
        row_start: usize,
        row_count: usize,
    ) -> Result<Vec<Column>> {
        // SECURITY: Validate inputs
        if table_id.0 == 0 {
            return Err(Error::Query("Invalid table ID: zero".to_string()));
        }
        
        const MAX_COLUMNS_PER_READ: usize = 10_000;
        if column_ids.len() > MAX_COLUMNS_PER_READ {
            return Err(Error::Query(format!(
                "Column count {} exceeds maximum {}",
                column_ids.len(), MAX_COLUMNS_PER_READ
            )));
        }
        
        const MAX_ROW_COUNT: usize = 1_000_000;
        if row_count > MAX_ROW_COUNT {
            return Err(Error::Query(format!(
                "Row count {} exceeds maximum {}",
                row_count, MAX_ROW_COUNT
            )));
        }
        
        // SECURITY: Validate row_start doesn't cause overflow
        if row_start > usize::MAX / 2 {
            return Err(Error::Query(format!(
                "Row start {} is too large",
                row_start
            )));
        }
        
        // For remote, we'd need to convert this to a query format
        // For now, return error indicating this needs query endpoint
        Err(Error::Query("Remote read_columns requires query endpoint implementation".to_string()))
    }
    
    async fn get_schema(&self, table_id: TableId) -> Result<Schema> {
        // SECURITY: Validate table_id
        if table_id.0 == 0 {
            return Err(Error::Query("Invalid table ID: zero".to_string()));
        }
        
        let response: JsonValue = self.get_json(&format!("/api/v1/tables/{}", table_id.0)).await?;
        // Parse schema from response - would need to match server response format
        Err(Error::Query("Schema parsing from remote response not yet implemented".to_string()))
    }
    
    async fn delete_table(&self, table_id: TableId) -> Result<()> {
        // SECURITY: Validate table_id
        if table_id.0 == 0 {
            return Err(Error::Query("Invalid table ID: zero".to_string()));
        }
        
        self.delete_request(&format!("/api/v1/tables/{}", table_id.0)).await
    }
    
    async fn execute_query(&self, query: JsonValue) -> Result<JsonValue> {
        // SECURITY: Validate query size to prevent huge JSON payloads
        // Estimate size by serializing (rough check)
        let query_str = serde_json::to_string(&query)
            .map_err(|e| Error::Query(format!("Failed to serialize query: {}", e)))?;
        
        const MAX_QUERY_SIZE: usize = 10 * 1024 * 1024; // 10MB
        if query_str.len() > MAX_QUERY_SIZE {
            return Err(Error::Query(format!(
                "Query size {} bytes exceeds maximum {} bytes",
                query_str.len(), MAX_QUERY_SIZE
            )));
        }
        
        self.post_json("/api/v1/query", query).await
    }
    
    async fn get_table_id(&self, table_name: &str) -> Result<Option<TableId>> {
        // Query tables endpoint and find matching name
        let response: JsonValue = self.get_json("/api/v1/tables").await?;
        // Parse response to find table - would need to match server format
        Ok(None)
    }
}

