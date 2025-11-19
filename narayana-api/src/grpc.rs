// gRPC API definitions will be generated from .proto files
// For now, we'll define the service traits

use narayana_core::{schema::Schema, types::TableId, column::Column};

pub trait NarayanaService {
    async fn create_table(&self, request: CreateTableRequest) -> Result<CreateTableResponse, String>;
    async fn insert(&self, request: InsertRequest) -> Result<InsertResponse, String>;
    async fn query(&self, request: QueryRequest) -> Result<QueryResponse, String>;
}

pub struct CreateTableRequest {
    pub table_name: String,
    pub schema: Schema,
}

pub struct CreateTableResponse {
    pub table_id: u64,
}

pub struct InsertRequest {
    pub table_id: u64,
    pub columns: Vec<Column>,
}

pub struct InsertResponse {
    pub rows_inserted: usize,
}

pub struct QueryRequest {
    pub table_id: u64,
    pub columns: Option<Vec<String>>,
    pub filter: Option<serde_json::Value>,
    pub limit: Option<usize>,
}

pub struct QueryResponse {
    pub columns: Vec<Column>,
    pub row_count: usize,
}

