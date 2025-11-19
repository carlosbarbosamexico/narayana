// Advanced REST API client

use narayana_core::{Error, Result, schema::Schema, types::TableId};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Advanced REST API client
pub struct AdvancedRestApi {
    base_url: String,
    client: reqwest::Client,
}

impl AdvancedRestApi {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }

    /// Create table with advanced options
    pub async fn create_table_advanced(&self, request: CreateTableAdvancedRequest) -> Result<CreateTableResponse> {
        let url = format!("{}/api/v2/tables", self.base_url);
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::Query(format!("Request failed: {}", e)))?;
        
        response.json().await
            .map_err(|e| Error::Query(format!("Failed to parse response: {}", e)))
    }

    /// Insert with options
    pub async fn insert_advanced(&self, table_id: u64, request: InsertAdvancedRequest) -> Result<InsertResponse> {
        let url = format!("{}/api/v2/tables/{}/insert", self.base_url, table_id);
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::Query(format!("Request failed: {}", e)))?;
        
        response.json().await
            .map_err(|e| Error::Query(format!("Failed to parse response: {}", e)))
    }

    /// Query with advanced options
    pub async fn query_advanced(&self, table_id: u64, request: QueryAdvancedRequest) -> Result<QueryResponse> {
        let url = format!("{}/api/v2/tables/{}/query", self.base_url, table_id);
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::Query(format!("Request failed: {}", e)))?;
        
        response.json().await
            .map_err(|e| Error::Query(format!("Failed to parse response: {}", e)))
    }

    /// Bulk operations
    pub async fn bulk_operation(&self, request: BulkOperationRequest) -> Result<BulkOperationResponse> {
        let url = format!("{}/api/v2/bulk", self.base_url);
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::Query(format!("Request failed: {}", e)))?;
        
        response.json().await
            .map_err(|e| Error::Query(format!("Failed to parse response: {}", e)))
    }

    /// Stream query results
    pub fn stream_query(&self, table_id: u64, request: QueryAdvancedRequest) -> impl Stream<Item = Result<JsonValue>> {
        StreamQuery {
            client: self.client.clone(),
            base_url: self.base_url.clone(),
            table_id,
            request,
            done: false,
        }
    }
}

use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

struct StreamQuery {
    client: reqwest::Client,
    base_url: String,
    table_id: u64,
    request: QueryAdvancedRequest,
    done: bool,
}

impl Stream for StreamQuery {
    type Item = Result<JsonValue>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.done {
            return Poll::Ready(None);
        }
        self.done = true;
        // In production, would stream results
        Poll::Ready(Some(Ok(JsonValue::Null)))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTableAdvancedRequest {
    pub name: String,
    pub schema: Schema,
    pub options: TableOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableOptions {
    pub compression: Option<String>,
    pub encryption: Option<EncryptionOptions>,
    pub indexes: Vec<IndexOptions>,
    pub partitioning: Option<PartitioningOptions>,
    pub replication: Option<ReplicationOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionOptions {
    pub algorithm: String,
    pub key_id: String,
    pub scope: String, // database, table, column, record
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexOptions {
    pub name: String,
    pub columns: Vec<String>,
    pub index_type: String, // btree, bloom, skip, minmax
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitioningOptions {
    pub strategy: String, // hash, range, list
    pub columns: Vec<String>,
    pub partitions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationOptions {
    pub factor: usize,
    pub strategy: String, // sync, async, quorum
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertAdvancedRequest {
    pub columns: Vec<narayana_core::column::Column>,
    pub options: InsertOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertOptions {
    pub batch_size: Option<usize>,
    pub compression: Option<String>,
    pub encryption: Option<EncryptionOptions>,
    pub async_insert: Option<bool>,
    pub deduplicate: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryAdvancedRequest {
    pub select: Vec<String>,
    pub filter: Option<FilterExpression>,
    pub order_by: Vec<OrderByExpression>,
    pub group_by: Vec<String>,
    pub having: Option<FilterExpression>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub join: Vec<JoinExpression>,
    pub options: QueryOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterExpression {
    pub operator: String, // eq, ne, gt, lt, gte, lte, in, like, between
    pub column: String,
    pub value: JsonValue,
    pub and: Option<Box<FilterExpression>>,
    pub or: Option<Box<FilterExpression>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderByExpression {
    pub column: String,
    pub direction: String, // asc, desc
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinExpression {
    pub table: String,
    pub condition: String,
    pub join_type: String, // inner, left, right, full
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryOptions {
    pub use_cache: Option<bool>,
    pub cache_ttl: Option<u64>,
    pub parallel: Option<bool>,
    pub max_threads: Option<usize>,
    pub timeout: Option<u64>,
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkOperationRequest {
    pub operations: Vec<BulkOperation>,
    pub transaction: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkOperation {
    pub operation_type: String, // insert, update, delete, upsert
    pub table: String,
    pub data: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkOperationResponse {
    pub operations_executed: usize,
    pub rows_affected: usize,
    pub errors: Vec<String>,
}

use crate::rest::*;

