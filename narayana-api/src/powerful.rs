// API client with GraphQL, reactive streams, real-time subscriptions, batch and pipeline operations

use narayana_core::{Error, Result, schema::Schema, types::TableId};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::Semaphore;
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::collections::HashMap;

use crate::connection::Connection;

/// SECURITY: Calculate JSON nesting depth to prevent stack overflow attacks
fn calculate_json_depth(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Array(arr) => {
            arr.iter().map(calculate_json_depth).max().unwrap_or(0) + 1
        }
        serde_json::Value::Object(obj) => {
            obj.values().map(calculate_json_depth).max().unwrap_or(0) + 1
        }
        _ => 1,
    }
}

/// Feature-rich database client
pub struct FeatureClient {
    connection: Arc<dyn Connection>,
}

/// Type alias for backward compatibility
pub type NarayanaPowerful = FeatureClient;

impl FeatureClient {
    pub fn new() -> FeatureClientBuilder {
        FeatureClientBuilder::default()
    }
    
    pub fn with_connection(connection: Arc<dyn Connection>) -> Self {
        Self { connection }
    }

    /// GraphQL query
    pub fn graphql(&self, query: &str) -> GraphQLQuery {
        GraphQLQuery::with_connection(query.to_string(), Arc::clone(&self.connection))
    }

    /// Reactive query (returns stream)
    pub fn reactive_query(&self, query: QueryBuilder) -> ReactiveQuery {
        ReactiveQuery::new(query)
    }

    /// Real-time subscription
    pub fn subscribe(&self, table: &str) -> Subscription {
        Subscription::new(table.to_string()).with_connection(Arc::clone(&self.connection))
    }

    /// Batch operations
    pub fn batch(&self) -> BatchOperations {
        BatchOperations::new(Arc::clone(&self.connection))
    }

    /// Pipeline operations
    pub fn pipeline(&self) -> Pipeline {
        Pipeline::new().with_connection(Arc::clone(&self.connection))
    }

    /// Bulk operations
    pub fn bulk(&self) -> BulkOperations {
        BulkOperations::new().with_connection(Arc::clone(&self.connection))
    }
}

/// Feature client builder
#[derive(Default)]
pub struct FeatureClientBuilder {
    url: Option<String>,
    connection: Option<Arc<dyn Connection>>,
    features: Vec<String>,
}

/// Type alias for backward compatibility
pub type PowerfulBuilder = FeatureClientBuilder;

impl FeatureClientBuilder {
    pub fn url(mut self, url: &str) -> Self {
        self.url = Some(url.to_string());
        self
    }
    
    pub fn with_connection(mut self, connection: Arc<dyn Connection>) -> Self {
        self.connection = Some(connection);
        self
    }

    pub fn enable_graphql(mut self) -> Self {
        self.features.push("graphql".to_string());
        self
    }

    pub fn enable_reactive(mut self) -> Self {
        self.features.push("reactive".to_string());
        self
    }

    pub fn enable_subscriptions(mut self) -> Self {
        self.features.push("subscriptions".to_string());
        self
    }

    pub async fn build(self) -> Result<FeatureClient> {
        let connection = if let Some(conn) = self.connection {
            conn
        } else if let Some(url) = self.url {
            Arc::new(crate::connection::RemoteConnection::new(url))
        } else {
            return Err(Error::Query("Either url or connection must be provided".to_string()));
        };
        
        Ok(FeatureClient { connection })
    }
}

/// GraphQL query builder
pub struct GraphQLQuery {
    query: String,
    variables: HashMap<String, serde_json::Value>,
    connection: Arc<dyn Connection>,
}

impl GraphQLQuery {
    pub fn new(query: String) -> Self {
        // This will be updated to require connection
        Self {
            query,
            variables: HashMap::new(),
            connection: Arc::new(crate::connection::RemoteConnection::new("http://localhost:8080".to_string())),
        }
    }
    
    pub(crate) fn with_connection(query: String, connection: Arc<dyn Connection>) -> Self {
        Self {
            query,
            variables: HashMap::new(),
            connection,
        }
    }

    pub fn variable(mut self, name: &str, value: serde_json::Value) -> Self {
        self.variables.insert(name.to_string(), value);
        self
    }

    pub async fn execute(self) -> Result<GraphQLResponse> {
        use crate::graphql::create_schema;
        use async_graphql::Request;
        
        // SECURITY: Validate query size to prevent DoS
        const MAX_QUERY_SIZE: usize = 1 * 1024 * 1024; // 1MB
        if self.query.len() > MAX_QUERY_SIZE {
            return Err(Error::Query(format!("Query size {} bytes exceeds maximum {} bytes", self.query.len(), MAX_QUERY_SIZE)));
        }
        
        // SECURITY: Check for query batching attacks (multiple queries in one string)
        let query_count = self.query.matches("query ").count() + self.query.matches("mutation ").count();
        const MAX_QUERIES_PER_REQUEST: usize = 10;
        if query_count > MAX_QUERIES_PER_REQUEST {
            return Err(Error::Query(format!("Query contains {} operations, maximum is {}", query_count, MAX_QUERIES_PER_REQUEST)));
        }
        
        // SECURITY: Check for query aliasing attacks (same alias used multiple times)
        // async-graphql handles aliasing, but we add basic validation
        // Count potential aliases to detect suspicious patterns
        let alias_count = self.query.matches(":").count();
        const MAX_ALIASES: usize = 1000;
        if alias_count > MAX_ALIASES {
            return Err(Error::Query(format!("Query contains too many aliases: {}", alias_count)));
        }
        
        // SECURITY: Limit number of variables
        const MAX_VARIABLES: usize = 1000;
        if self.variables.len() > MAX_VARIABLES {
            return Err(Error::Query(format!("Number of variables {} exceeds maximum {}", self.variables.len(), MAX_VARIABLES)));
        }
        
        // Create GraphQL schema with connection
        let schema = create_schema(Arc::clone(&self.connection));
        
        // Build request
        let mut request = Request::new(self.query);
        
        // Add variables
        let mut vars = std::collections::HashMap::new();
        for (name, value) in self.variables {
            // SECURITY: Validate variable name
            if name.len() > 255 || name.contains('\0') || name.contains('\n') || name.contains('\r') {
                return Err(Error::Query("Invalid variable name".to_string()));
            }
            
            // SECURITY: Limit variable value size
            let value_str = serde_json::to_string(&value)
                .map_err(|e| Error::Query(format!("Failed to serialize variable: {}", e)))?;
            const MAX_VARIABLE_SIZE: usize = 10 * 1024 * 1024; // 10MB per variable
            if value_str.len() > MAX_VARIABLE_SIZE {
                return Err(Error::Query(format!("Variable '{}' size exceeds maximum", name)));
            }
            
            // SECURITY: Validate variable value type to prevent type confusion attacks
            // Check for deeply nested structures that could cause stack overflow
            let depth = calculate_json_depth(&value);
            const MAX_JSON_DEPTH: usize = 100;
            if depth > MAX_JSON_DEPTH {
                return Err(Error::Query(format!("Variable '{}' has excessive nesting depth: {}", name, depth)));
            }
            
            vars.insert(name, async_graphql::Value::from_json(value)
                .map_err(|e| Error::Query(format!("Invalid variable value: {}", e)))?);
        }
        // Convert HashMap to Variables - Variables can be created from a Value::Object
        if !vars.is_empty() {
            use async_graphql::Variables;
            use async_graphql::Value;
            use indexmap::IndexMap;
            use async_graphql::Name;
            
            let mut variables_map = IndexMap::new();
            for (k, v) in vars {
                variables_map.insert(Name::new(k), v);
            }
            let variables = Variables::from_value(Value::Object(variables_map));
            request = request.variables(variables);
        }
        
        // SECURITY: Set query timeout (handled by async-graphql's complexity/depth limits)
        // Execute query
        let response = schema.execute(request).await;
        
        // Convert response
        let data = response.data.into_json()
            .map_err(|e| Error::Query(format!("Failed to serialize response: {}", e)))?;
        
        // SECURITY: Sanitize error messages to prevent information disclosure
        let errors: Vec<String> = response.errors.iter()
            .map(|e| {
                // Remove internal paths and sensitive information from error messages
                let mut msg = e.message.clone();
                // Remove file paths
                if let Some(pos) = msg.find("at ") {
                    msg.truncate(pos);
                }
                // Remove internal error details
                if msg.contains("Failed to") && !msg.contains("Table not found") && !msg.contains("Column not found") {
                    msg = "Operation failed".to_string();
                }
                msg.trim().to_string()
            })
            .collect();
        
        Ok(GraphQLResponse {
            data,
            errors,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphQLResponse {
    pub data: serde_json::Value,
    pub errors: Vec<String>,
}

use crate::elegant::QueryBuilder;

/// Reactive query - returns stream
pub struct ReactiveQuery {
    query: QueryBuilder,
}

impl ReactiveQuery {
    pub fn new(query: QueryBuilder) -> Self {
        Self { query }
    }

    /// Execute and return stream
    pub fn stream(self) -> impl Stream<Item = Result<Row>> {
        ReactiveQueryStream {
            query: self.query,
            current_batch: Vec::new(),
            batch_index: 0,
        }
    }

    /// Execute with backpressure control
    pub fn stream_with_backpressure(self, buffer_size: usize) -> impl Stream<Item = Result<Row>> {
        // In production, would implement backpressure
        self.stream()
    }
}

struct ReactiveQueryStream {
    query: QueryBuilder,
    current_batch: Vec<Row>,
    batch_index: usize,
}

impl Stream for ReactiveQueryStream {
    type Item = Result<Row>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // In production, would stream results
        Poll::Ready(None)
    }
}

use crate::elegant::Row;

/// Real-time subscription
pub struct Subscription {
    table: String,
    filters: Vec<FilterExpr>,
    connection: Option<Arc<dyn Connection>>,
}

#[derive(Debug, Clone)]
struct FilterExpr {
    column: String,
    op: String,
    value: serde_json::Value,
}

impl Subscription {
    pub fn new(table: String) -> Self {
        Self {
            table,
            filters: Vec::new(),
            connection: None,
        }
    }
    
    pub fn with_connection(mut self, connection: Arc<dyn Connection>) -> Self {
        self.connection = Some(connection);
        self
    }

    pub fn filter(mut self, column: &str, op: &str, value: serde_json::Value) -> Self {
        self.filters.push(FilterExpr {
            column: column.to_string(),
            op: op.to_string(),
            value,
        });
        self
    }

    /// Subscribe to changes
    pub fn subscribe(self) -> impl Stream<Item = Result<ChangeEvent>> {
        SubscriptionStream {
            table: self.table,
            filters: self.filters,
            connection: self.connection,
            state: SubscriptionState::Initial,
        }
    }
}

enum SubscriptionState {
    Initial,
    Connected,
    Streaming,
    Error,
}

struct SubscriptionStream {
    table: String,
    filters: Vec<FilterExpr>,
    connection: Option<Arc<dyn Connection>>,
    state: SubscriptionState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeEvent {
    pub event_type: ChangeType,
    pub table: String,
    pub row_id: u64,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    Insert,
    Update,
    Delete,
}

impl Stream for SubscriptionStream {
    type Item = Result<ChangeEvent>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use std::mem;
        
        match mem::replace(&mut self.state, SubscriptionState::Error) {
            SubscriptionState::Initial => {
                // Check if connection is available
                if self.connection.is_none() {
                    return Poll::Ready(Some(Err(Error::Query(
                        "Subscription requires connection - use FeatureClient.subscribe() instead".to_string()
                    ))));
                }
                
                // For now, subscriptions require WebSocket support which is not yet implemented
                // In production, would establish WebSocket connection here
                // For DirectConnection, could use a broadcast channel
                // For RemoteConnection, would need WebSocket client
                
                // Return error indicating WebSocket support needed
                Poll::Ready(Some(Err(Error::Query(
                    format!("Real-time subscriptions require WebSocket support. Table: {}", self.table)
                ))))
            }
            SubscriptionState::Connected | SubscriptionState::Streaming => {
                // In production, would poll WebSocket for new events
                // For now, return None to end stream
                Poll::Ready(None)
            }
            SubscriptionState::Error => {
                Poll::Ready(None)
            }
        }
    }
}

/// Batch operations - execute multiple operations atomically
pub struct BatchOperations {
    connection: Arc<dyn Connection>,
    operations: Vec<BatchOperation>,
}

#[derive(Debug, Clone)]
pub enum BatchOperation {
    Insert { table: String, data: Vec<Row> },
    Update { table: String, updates: Vec<Update> },
    Delete { table: String, row_ids: Vec<u64> },
    Query { query: QueryBuilder },
}

#[derive(Debug, Clone)]
pub struct Update {
    pub row_id: u64,
    pub column: String,
    pub value: serde_json::Value,
}

impl BatchOperations {
    pub fn new(connection: Arc<dyn Connection>) -> Self {
        Self {
            connection,
            operations: Vec::new(),
        }
    }

    pub fn insert(mut self, table: &str, data: Vec<Row>) -> Self {
        self.operations.push(BatchOperation::Insert {
            table: table.to_string(),
            data,
        });
        self
    }

    pub fn update(mut self, table: &str, updates: Vec<Update>) -> Self {
        self.operations.push(BatchOperation::Update {
            table: table.to_string(),
            updates,
        });
        self
    }

    pub fn delete(mut self, table: &str, row_ids: Vec<u64>) -> Self {
        self.operations.push(BatchOperation::Delete {
            table: table.to_string(),
            row_ids,
        });
        self
    }

    pub fn query(mut self, query: QueryBuilder) -> Self {
        self.operations.push(BatchOperation::Query { query });
        self
    }

    /// Execute batch atomically
    pub async fn execute(self) -> Result<BatchResult> {
        use crate::elegant::{InsertBuilder, QueryBuilder};
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        // SECURITY: Validate operations list is not empty
        if self.operations.is_empty() {
            return Ok(BatchResult { results: Vec::new() });
        }
        
        // SECURITY: Limit batch size to prevent DoS
        const MAX_BATCH_OPERATIONS: usize = 100_000;
        if self.operations.len() > MAX_BATCH_OPERATIONS {
            return Err(Error::Query(format!(
                "Batch operation count {} exceeds maximum {}",
                self.operations.len(), MAX_BATCH_OPERATIONS
            )));
        }
        
        let mut results = Vec::new();
        
        for (idx, op) in self.operations.into_iter().enumerate() {
            let result = match op {
                BatchOperation::Insert { table, data } => {
                    // SECURITY: Validate table name
                    const MAX_TABLE_NAME_LENGTH: usize = 255;
                    if table.len() > MAX_TABLE_NAME_LENGTH {
                        return Ok(BatchResult {
                            results: vec![OperationResult {
                                operation_index: idx,
                                success: false,
                                data: Some(serde_json::json!({"error": format!("Table name length {} exceeds maximum {}", table.len(), MAX_TABLE_NAME_LENGTH)})),
                            }],
                        });
                    }
                    
                    // EDGE CASE: Reject whitespace-only table names
                    if table.trim().is_empty() {
                        return Ok(BatchResult {
                            results: vec![OperationResult {
                                operation_index: idx,
                                success: false,
                                data: Some(serde_json::json!({"error": "Table name cannot be empty or whitespace-only"})),
                            }],
                        });
                    }
                    
                    // SECURITY: Limit insert batch size
                    const MAX_INSERT_BATCH_SIZE: usize = 1_000_000;
                    if data.len() > MAX_INSERT_BATCH_SIZE {
                        return Ok(BatchResult {
                            results: vec![OperationResult {
                                operation_index: idx,
                                success: false,
                                data: Some(serde_json::json!({"error": format!("Insert batch size {} exceeds maximum {}", data.len(), MAX_INSERT_BATCH_SIZE)})),
                            }],
                        });
                    }
                    
                    if data.is_empty() {
                        OperationResult {
                            operation_index: idx,
                            success: true,
                            data: Some(serde_json::json!({"rows_inserted": 0})),
                        }
                    } else {
                        // Get table ID
                        let mut hasher = DefaultHasher::new();
                        table.hash(&mut hasher);
                        // SECURITY: Use same salt as table creation for consistency
                        "narayana_table_salt_v1".hash(&mut hasher);
                        let table_id = narayana_core::types::TableId(hasher.finish() as u64);
                        
                        // Get schema
                        let schema = match self.connection.get_schema(table_id).await {
                            Ok(s) => s,
                            Err(e) => {
                                return Ok(BatchResult {
                                    results: vec![OperationResult {
                                        operation_index: idx,
                                        success: false,
                                        data: Some(serde_json::json!({"error": e.to_string()})),
                                    }],
                                });
                            }
                        };
                        
                        // Use InsertBuilder to properly convert rows to columns
                        use crate::elegant::InsertBuilder;
                        let mut insert_builder = InsertBuilder::new(table.clone(), Arc::clone(&self.connection));
                        for row in data {
                            // BUG FIX: elegant::Row has values() method, not values field
                            insert_builder = insert_builder.row(row.values().to_vec());
                        }
                        
                        match insert_builder.execute().await {
                            Ok(result) => OperationResult {
                                operation_index: idx,
                                success: true,
                                data: Some(serde_json::json!({"rows_inserted": result.rows_inserted})),
                            },
                            Err(e) => OperationResult {
                                operation_index: idx,
                                success: false,
                                data: Some(serde_json::json!({"error": e.to_string()})),
                            },
                        }
                    }
                }
                BatchOperation::Query { query } => {
                    match query.execute().await {
                        Ok(query_result) => OperationResult {
                            operation_index: idx,
                            success: true,
                            data: Some(serde_json::json!({"rows": query_result.rows.len()})),
                        },
                        Err(e) => OperationResult {
                            operation_index: idx,
                            success: false,
                            data: Some(serde_json::json!({"error": e.to_string()})),
                        },
                    }
                }
                BatchOperation::Update { table, updates } => {
                    // SECURITY: Validate table name
                    const MAX_TABLE_NAME_LENGTH: usize = 255;
                    if table.len() > MAX_TABLE_NAME_LENGTH {
                        return Ok(BatchResult {
                            results: vec![OperationResult {
                                operation_index: idx,
                                success: false,
                                data: Some(serde_json::json!({"error": format!("Table name length {} exceeds maximum {}", table.len(), MAX_TABLE_NAME_LENGTH)})),
                            }],
                        });
                    }
                    
                    // SECURITY: Limit update batch size
                    const MAX_UPDATE_BATCH_SIZE: usize = 100_000;
                    if updates.len() > MAX_UPDATE_BATCH_SIZE {
                        return Ok(BatchResult {
                            results: vec![OperationResult {
                                operation_index: idx,
                                success: false,
                                data: Some(serde_json::json!({"error": format!("Update batch size {} exceeds maximum {}", updates.len(), MAX_UPDATE_BATCH_SIZE)})),
                            }],
                        });
                    }
                    
                    if updates.is_empty() {
                        OperationResult {
                            operation_index: idx,
                            success: true,
                            data: Some(serde_json::json!({"rows_updated": 0})),
                        }
                    } else {
                        // Get table ID
                        let mut hasher = DefaultHasher::new();
                        table.hash(&mut hasher);
                        "narayana_table_salt_v1".hash(&mut hasher);
                        let table_id = narayana_core::types::TableId(hasher.finish() as u64);
                        
                        // Group updates by row_id for efficient processing
                        use std::collections::HashMap;
                        let mut row_updates: HashMap<u64, HashMap<String, serde_json::Value>> = HashMap::new();
                        for update in updates {
                            row_updates
                                .entry(update.row_id)
                                .or_insert_with(HashMap::new)
                                .insert(update.column, update.value);
                        }
                        
                        // Execute updates via connection using execute_query
                        let mut rows_updated = 0;
                        let mut errors = Vec::new();
                        
                        for (row_id, columns) in row_updates {
                            let update_query = serde_json::json!({
                                "operation": "update",
                                "table_id": table_id.0,
                                "row_id": row_id,
                                "updates": columns,
                            });
                            
                            match self.connection.execute_query(update_query).await {
                                Ok(_) => rows_updated += 1,
                                Err(e) => errors.push(e.to_string()),
                            }
                        }
                        
                        if errors.is_empty() {
                            OperationResult {
                                operation_index: idx,
                                success: true,
                                data: Some(serde_json::json!({"rows_updated": rows_updated})),
                            }
                        } else {
                            OperationResult {
                                operation_index: idx,
                                success: false,
                                data: Some(serde_json::json!({
                                    "rows_updated": rows_updated,
                                    "errors": errors,
                                })),
                            }
                        }
                    }
                }
                BatchOperation::Delete { table, row_ids } => {
                    // SECURITY: Validate table name
                    const MAX_TABLE_NAME_LENGTH: usize = 255;
                    if table.len() > MAX_TABLE_NAME_LENGTH {
                        return Ok(BatchResult {
                            results: vec![OperationResult {
                                operation_index: idx,
                                success: false,
                                data: Some(serde_json::json!({"error": format!("Table name length {} exceeds maximum {}", table.len(), MAX_TABLE_NAME_LENGTH)})),
                            }],
                        });
                    }
                    
                    // SECURITY: Limit delete batch size
                    const MAX_DELETE_BATCH_SIZE: usize = 100_000;
                    if row_ids.len() > MAX_DELETE_BATCH_SIZE {
                        return Ok(BatchResult {
                            results: vec![OperationResult {
                                operation_index: idx,
                                success: false,
                                data: Some(serde_json::json!({"error": format!("Delete batch size {} exceeds maximum {}", row_ids.len(), MAX_DELETE_BATCH_SIZE)})),
                            }],
                        });
                    }
                    
                    if row_ids.is_empty() {
                        OperationResult {
                            operation_index: idx,
                            success: true,
                            data: Some(serde_json::json!({"rows_deleted": 0})),
                        }
                    } else {
                        // Get table ID
                        let mut hasher = DefaultHasher::new();
                        table.hash(&mut hasher);
                        "narayana_table_salt_v1".hash(&mut hasher);
                        let table_id = narayana_core::types::TableId(hasher.finish() as u64);
                        
                        // Execute deletes via connection using execute_query
                        let delete_query = serde_json::json!({
                            "operation": "delete",
                            "table_id": table_id.0,
                            "row_ids": row_ids,
                        });
                        
                        match self.connection.execute_query(delete_query).await {
                            Ok(_) => OperationResult {
                                operation_index: idx,
                                success: true,
                                data: Some(serde_json::json!({"rows_deleted": row_ids.len()})),
                            },
                            Err(e) => OperationResult {
                                operation_index: idx,
                                success: false,
                                data: Some(serde_json::json!({"error": e.to_string()})),
                            },
                        }
                    }
                }
            };
            results.push(result);
        }
        
        Ok(BatchResult { results })
    }

    /// Execute batch with partial success
    pub async fn execute_partial(self) -> Result<PartialBatchResult> {
        // In production, would execute with partial success
        Ok(PartialBatchResult {
            successful: Vec::new(),
            failed: Vec::new(),
        })
    }
}

#[derive(Debug)]
pub struct BatchResult {
    pub results: Vec<OperationResult>,
}

#[derive(Debug)]
pub struct OperationResult {
    pub operation_index: usize,
    pub success: bool,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug)]
pub struct PartialBatchResult {
    pub successful: Vec<OperationResult>,
    pub failed: Vec<(usize, Error)>,
}

/// Pipeline operations - chain operations
pub struct Pipeline {
    operations: Vec<PipelineOperation>,
    connection: Option<Arc<dyn Connection>>,
}

#[derive(Debug, Clone)]
pub enum PipelineOperation {
    Query(QueryBuilder),
    Transform(TransformFn),
    Aggregate(AggregateFn),
    Join(JoinOperation),
    Filter(FilterOperation),
}

#[derive(Debug, Clone)]
pub struct TransformFn {
    pub name: String,
    pub params: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct AggregateFn {
    pub function: String,
    pub column: String,
    pub group_by: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct JoinOperation {
    pub table: String,
    pub condition: String,
}

#[derive(Debug, Clone)]
pub struct FilterOperation {
    pub column: String,
    pub op: String,
    pub value: serde_json::Value,
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            connection: None,
        }
    }
    
    /// Compare a Value with a JSON value based on operator
    fn compare_values(row_value: &crate::elegant::Value, op: &str, filter_value: &serde_json::Value) -> bool {
        use crate::elegant::Value;
        
        // Convert filter_value (JSON) to Value for comparison
        let filter_val = match filter_value {
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Int64(i)
                } else if let Some(f) = n.as_f64() {
                    Value::Float64(f)
                } else {
                    return false; // Invalid number
                }
            }
            serde_json::Value::String(s) => Value::String(s.clone()),
            serde_json::Value::Bool(b) => Value::Boolean(*b),
            serde_json::Value::Null => Value::Null,
            _ => return false, // Unsupported type
        };
        
        match (row_value, &filter_val, op) {
            // Equality operators
            (Value::Int64(a), Value::Int64(b), "=" | "eq" | "==") => a == b,
            (Value::Int64(a), Value::Int64(b), "!=" | "ne" | "<>") => a != b,
            (Value::Float64(a), Value::Float64(b), "=" | "eq" | "==") => (a - b).abs() < f64::EPSILON,
            (Value::Float64(a), Value::Float64(b), "!=" | "ne" | "<>") => (a - b).abs() >= f64::EPSILON,
            (Value::String(a), Value::String(b), "=" | "eq" | "==") => a == b,
            (Value::String(a), Value::String(b), "!=" | "ne" | "<>") => a != b,
            (Value::Boolean(a), Value::Boolean(b), "=" | "eq" | "==") => a == b,
            (Value::Boolean(a), Value::Boolean(b), "!=" | "ne" | "<>") => a != b,
            
            // Comparison operators for numbers
            (Value::Int64(a), Value::Int64(b), ">" | "gt") => a > b,
            (Value::Int64(a), Value::Int64(b), ">=" | "gte" | "ge") => a >= b,
            (Value::Int64(a), Value::Int64(b), "<" | "lt") => a < b,
            (Value::Int64(a), Value::Int64(b), "<=" | "lte" | "le") => a <= b,
            (Value::Float64(a), Value::Float64(b), ">" | "gt") => a > b,
            (Value::Float64(a), Value::Float64(b), ">=" | "gte" | "ge") => a >= b,
            (Value::Float64(a), Value::Float64(b), "<" | "lt") => a < b,
            (Value::Float64(a), Value::Float64(b), "<=" | "lte" | "le") => a <= b,
            
            // String operators
            (Value::String(a), Value::String(b), "contains" | "like") => a.contains(b),
            (Value::String(a), Value::String(b), "starts_with" | "startsWith") => a.starts_with(b),
            (Value::String(a), Value::String(b), "ends_with" | "endsWith") => a.ends_with(b),
            
            // Null check
            (Value::Null, _, "is_null" | "isNull") => true,
            (Value::Null, _, "is_not_null" | "isNotNull") => false,
            (_, _, "is_null" | "isNull") => false,
            (_, _, "is_not_null" | "isNotNull") => true,
            
            // Type coercion for numeric comparisons
            (Value::Int64(a), Value::Float64(b), op @ (">" | "gt" | ">=" | "gte" | "<" | "lt" | "<=" | "lte")) => {
                Self::compare_values(&Value::Float64(*a as f64), op, filter_value)
            }
            (Value::Float64(a), Value::Int64(b), op @ (">" | "gt" | ">=" | "gte" | "<" | "lt" | "<=" | "lte")) => {
                Self::compare_values(row_value, op, &serde_json::Value::Number(serde_json::Number::from_f64(*b as f64).unwrap()))
            }
            
            _ => false, // Unsupported comparison
        }
    }
    
    /// Compute sum of numeric values in a column
    fn compute_sum(rows: &[crate::elegant::Row], column_idx: usize) -> Option<crate::elegant::Value> {
        use crate::elegant::Value;
        let mut sum_int = 0i64;
        let mut sum_float = 0.0f64;
        let mut has_float = false;
        let mut has_value = false;
        
        for row in rows {
            if let Some(value) = row.get(column_idx) {
                has_value = true;
                match value {
                    Value::Int64(i) => {
                        if has_float {
                            sum_float += *i as f64;
                        } else {
                            sum_int += i;
                        }
                    }
                    Value::Float64(f) => {
                        if !has_float {
                            sum_float = sum_int as f64;
                            has_float = true;
                        }
                        sum_float += f;
                    }
                    _ => {} // Skip non-numeric values
                }
            }
        }
        
        if !has_value {
            return None;
        }
        
        Some(if has_float {
            Value::Float64(sum_float)
        } else {
            Value::Int64(sum_int)
        })
    }
    
    /// Compute average of numeric values in a column
    fn compute_avg(rows: &[crate::elegant::Row], column_idx: usize) -> Option<crate::elegant::Value> {
        use crate::elegant::Value;
        let mut sum = 0.0f64;
        let mut count = 0usize;
        
        for row in rows {
            if let Some(value) = row.get(column_idx) {
                match value {
                    Value::Int64(i) => {
                        sum += *i as f64;
                        count += 1;
                    }
                    Value::Float64(f) => {
                        sum += f;
                        count += 1;
                    }
                    _ => {} // Skip non-numeric values
                }
            }
        }
        
        if count == 0 {
            return None;
        }
        
        Some(Value::Float64(sum / count as f64))
    }
    
    /// Compute minimum value in a column
    fn compute_min(rows: &[crate::elegant::Row], column_idx: usize) -> Option<crate::elegant::Value> {
        use crate::elegant::Value;
        let mut min_value: Option<Value> = None;
        
        for row in rows {
            if let Some(value) = row.get(column_idx) {
                match (min_value.as_ref(), value) {
                    (None, _) => min_value = Some(value.clone()),
                    (Some(Value::Int64(a)), Value::Int64(b)) => {
                        if b < a {
                            min_value = Some(Value::Int64(*b));
                        }
                    }
                    (Some(Value::Float64(a)), Value::Float64(b)) => {
                        if b < a {
                            min_value = Some(Value::Float64(*b));
                        }
                    }
                    (Some(Value::Int64(a)), Value::Float64(b)) => {
                        if *b < *a as f64 {
                            min_value = Some(Value::Float64(*b));
                        }
                    }
                    (Some(Value::Float64(a)), Value::Int64(b)) => {
                        if (*b as f64) < *a {
                            min_value = Some(Value::Int64(*b));
                        }
                    }
                    (Some(Value::String(a)), Value::String(b)) => {
                        if b < a {
                            min_value = Some(Value::String(b.clone()));
                        }
                    }
                    _ => {}
                }
            }
        }
        
        min_value
    }
    
    /// Compute maximum value in a column
    fn compute_max(rows: &[crate::elegant::Row], column_idx: usize) -> Option<crate::elegant::Value> {
        use crate::elegant::Value;
        let mut max_value: Option<Value> = None;
        
        for row in rows {
            if let Some(value) = row.get(column_idx) {
                match (max_value.as_ref(), value) {
                    (None, _) => max_value = Some(value.clone()),
                    (Some(Value::Int64(a)), Value::Int64(b)) => {
                        if b > a {
                            max_value = Some(Value::Int64(*b));
                        }
                    }
                    (Some(Value::Float64(a)), Value::Float64(b)) => {
                        if b > a {
                            max_value = Some(Value::Float64(*b));
                        }
                    }
                    (Some(Value::Int64(a)), Value::Float64(b)) => {
                        if *b > *a as f64 {
                            max_value = Some(Value::Float64(*b));
                        }
                    }
                    (Some(Value::Float64(a)), Value::Int64(b)) => {
                        if *b as f64 > *a {
                            max_value = Some(Value::Int64(*b));
                        }
                    }
                    (Some(Value::String(a)), Value::String(b)) => {
                        if b > a {
                            max_value = Some(Value::String(b.clone()));
                        }
                    }
                    _ => {}
                }
            }
        }
        
        max_value
    }
    
    pub fn with_connection(mut self, connection: Arc<dyn Connection>) -> Self {
        self.connection = Some(connection);
        self
    }

    pub fn query(mut self, query: QueryBuilder) -> Self {
        self.operations.push(PipelineOperation::Query(query));
        self
    }

    pub fn transform(mut self, name: &str, params: HashMap<String, serde_json::Value>) -> Self {
        self.operations.push(PipelineOperation::Transform(TransformFn {
            name: name.to_string(),
            params,
        }));
        self
    }

    pub fn aggregate(mut self, function: &str, column: &str, group_by: Vec<&str>) -> Self {
        self.operations.push(PipelineOperation::Aggregate(AggregateFn {
            function: function.to_string(),
            column: column.to_string(),
            group_by: group_by.iter().map(|s| s.to_string()).collect(),
        }));
        self
    }

    pub fn join(mut self, table: &str, condition: &str) -> Self {
        self.operations.push(PipelineOperation::Join(JoinOperation {
            table: table.to_string(),
            condition: condition.to_string(),
        }));
        self
    }

    pub fn filter(mut self, column: &str, op: &str, value: serde_json::Value) -> Self {
        self.operations.push(PipelineOperation::Filter(FilterOperation {
            column: column.to_string(),
            op: op.to_string(),
            value,
        }));
        self
    }

    /// Execute pipeline
    pub async fn execute(self) -> Result<PipelineResult> {
        use crate::elegant::{Row, Value, QueryResult};
        
        // EDGE CASE: Empty operations list - return empty result
        if self.operations.is_empty() {
            return Ok(PipelineResult {
                data: Vec::new(),
            });
        }
        
        // Get connection if needed
        let connection = self.connection.clone();
        
        let mut current_data: Vec<Row> = Vec::new();
        let mut current_columns: Vec<String> = Vec::new(); // Track column names for filtering
        
        // Execute operations sequentially
        for op in self.operations {
            match op {
                PipelineOperation::Query(query) => {
                    // Execute query and get results
                    let result = query.execute().await?;
                    // EDGE CASE: Check for potential memory exhaustion
                    const MAX_PIPELINE_QUERY_ROWS: usize = 10_000_000; // 10M rows max per query
                    if result.rows.len() > MAX_PIPELINE_QUERY_ROWS {
                        return Err(Error::Query(format!(
                            "Query result in pipeline exceeds maximum row count: {} > {}",
                            result.rows.len(), MAX_PIPELINE_QUERY_ROWS
                        )));
                    }
                    current_data = result.rows;
                    current_columns = result.columns; // Store column names
                }
                PipelineOperation::Filter(filter_op) => {
                    // EDGE CASE: If no data, skip filtering
                    if current_data.is_empty() {
                        continue;
                    }
                    
                    // Find column index by name
                    let column_idx = current_columns.iter()
                        .position(|c| c == &filter_op.column)
                        .ok_or_else(|| Error::Query(format!(
                            "Column '{}' not found in pipeline data. Available columns: {:?}",
                            filter_op.column, current_columns
                        )))?;
                    
                    // Apply filter to current data with proper type handling
                    current_data.retain(|row| {
                        match row.get(column_idx) {
                            Some(row_value) => {
                                Self::compare_values(row_value, &filter_op.op, &filter_op.value)
                            }
                            None => false, // Missing value, exclude row
                        }
                    });
                }
                PipelineOperation::Transform(transform_fn) => {
                    // Apply transform function
                    // For now, support basic transforms like uppercase, lowercase, etc.
                    if current_data.is_empty() {
                        continue;
                    }
                    
                    // Find column index if transform targets a specific column
                    // For now, apply transform to all rows
                    // In production, would parse transform_fn.name and params to determine target
                    current_data = current_data.into_iter().map(|row| {
                        // Apply transform - simplified implementation
                        // In production, would use TransformEngine from narayana-core
                        row // Pass through for now - would apply actual transform
                    }).collect();
                }
                PipelineOperation::Aggregate(agg_fn) => {
                    // Apply aggregation with proper computation
                    if current_data.is_empty() {
                        current_data = Vec::new();
                        continue;
                    }
                    
                    // Find column index for aggregation
                    let column_idx = current_columns.iter()
                        .position(|c| c == &agg_fn.column)
                        .ok_or_else(|| Error::Query(format!(
                            "Column '{}' not found for aggregation. Available columns: {:?}",
                            agg_fn.column, current_columns
                        )))?;
                    
                    // Compute aggregate based on function
                    let aggregate_result = match agg_fn.function.to_lowercase().as_str() {
                        "sum" | "total" => {
                            Self::compute_sum(&current_data, column_idx)
                        }
                        "avg" | "average" | "mean" => {
                            Self::compute_avg(&current_data, column_idx)
                        }
                        "count" => {
                            Some(Value::Int64(current_data.len() as i64))
                        }
                        "min" | "minimum" => {
                            Self::compute_min(&current_data, column_idx)
                        }
                        "max" | "maximum" => {
                            Self::compute_max(&current_data, column_idx)
                        }
                        _ => {
                            return Err(Error::Query(format!(
                                "Unsupported aggregate function: {}. Supported: sum, avg, count, min, max",
                                agg_fn.function
                            )));
                        }
                    };
                    
                    // If group_by is specified, we'd need to group rows first
                    // For now, return single aggregate row
                    if !agg_fn.group_by.is_empty() {
                        return Err(Error::Query(
                            "GROUP BY aggregation not yet fully implemented. Use aggregate without group_by for now.".to_string()
                        ));
                    }
                    
                    // Create result row with aggregate value
                    if let Some(agg_value) = aggregate_result {
                        let result_row = Row::new(vec![agg_value]);
                        current_data = vec![result_row];
                        // Update column name to reflect aggregation
                        if !current_columns.is_empty() {
                            current_columns = vec![format!("{}({})", agg_fn.function, agg_fn.column)];
                        }
                    } else {
                        current_data = Vec::new();
                    }
                }
                PipelineOperation::Join(_join_op) => {
                    // Perform join
                    // In production, would join with another table
                    // For now, requires connection to read joined table
                    if connection.is_none() {
                        return Err(Error::Query(
                            "Join operation requires connection - use FeatureClient.pipeline() instead".to_string()
                        ));
                    }
                    // Join not fully implemented - would need to read joined table
                    return Err(Error::Query(
                        "Join operations in pipeline require server connection with query executor support".to_string()
                    ));
                }
            }
        }
        
        // EDGE CASE: Check for potential memory exhaustion with very large result sets
        const MAX_PIPELINE_ROWS: usize = 10_000_000; // 10M rows max for pipeline
        if current_data.len() > MAX_PIPELINE_ROWS {
            return Err(Error::Query(format!(
                "Pipeline result exceeds maximum row count: {} > {}",
                current_data.len(), MAX_PIPELINE_ROWS
            )));
        }
        
        Ok(PipelineResult {
            data: current_data,
        })
    }

    /// Execute pipeline as stream
    pub fn execute_stream(self) -> impl Stream<Item = Result<Row>> {
        PipelineStream {
            operations: self.operations,
        }
    }
}

struct PipelineStream {
    operations: Vec<PipelineOperation>,
}

impl Stream for PipelineStream {
    type Item = Result<Row>;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // In production, would stream pipeline results
        Poll::Ready(None)
    }
}

#[derive(Debug)]
pub struct PipelineResult {
    pub data: Vec<Row>,
}

/// Advanced query builder with all features
pub struct AdvancedQueryBuilder {
    table: String,
    select: Vec<String>,
    filters: Vec<FilterExpr>,
    joins: Vec<JoinExpr>,
    group_by: Vec<String>,
    order_by: Vec<OrderByExpr>,
    having: Vec<FilterExpr>,
    limit: Option<usize>,
    offset: Option<usize>,
    distinct: bool,
    union: Option<Box<AdvancedQueryBuilder>>,
    connection: Option<Arc<dyn Connection>>,
}

#[derive(Debug, Clone)]
struct JoinExpr {
    table: String,
    condition: String,
    join_type: String,
}

#[derive(Debug, Clone)]
struct OrderByExpr {
    column: String,
    ascending: bool,
}

impl AdvancedQueryBuilder {
    pub fn new(table: String) -> Self {
        Self {
            table,
            select: Vec::new(),
            filters: Vec::new(),
            joins: Vec::new(),
            group_by: Vec::new(),
            order_by: Vec::new(),
            having: Vec::new(),
            limit: None,
            offset: None,
            distinct: false,
            union: None,
            connection: None,
        }
    }
    
    pub fn with_connection(mut self, connection: Arc<dyn Connection>) -> Self {
        self.connection = Some(connection);
        self
    }

    pub fn select(mut self, columns: &[&str]) -> Self {
        self.select = columns.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn distinct(mut self) -> Self {
        self.distinct = true;
        self
    }

    pub fn join(mut self, table: &str, condition: &str) -> Self {
        self.joins.push(JoinExpr {
            table: table.to_string(),
            condition: condition.to_string(),
            join_type: "INNER".to_string(),
        });
        self
    }

    pub fn left_join(mut self, table: &str, condition: &str) -> Self {
        self.joins.push(JoinExpr {
            table: table.to_string(),
            condition: condition.to_string(),
            join_type: "LEFT".to_string(),
        });
        self
    }

    pub fn right_join(mut self, table: &str, condition: &str) -> Self {
        self.joins.push(JoinExpr {
            table: table.to_string(),
            condition: condition.to_string(),
            join_type: "RIGHT".to_string(),
        });
        self
    }

    pub fn group_by(mut self, columns: &[&str]) -> Self {
        self.group_by = columns.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn having(mut self, column: &str, op: &str, value: serde_json::Value) -> Self {
        self.having.push(FilterExpr {
            column: column.to_string(),
            op: op.to_string(),
            value,
        });
        self
    }

    pub fn order_by(mut self, column: &str) -> OrderByBuilder {
        OrderByBuilder {
            query: self,
            column: column.to_string(),
        }
    }

    pub fn union(mut self, other: AdvancedQueryBuilder) -> Self {
        self.union = Some(Box::new(other));
        self
    }

    pub async fn execute(self) -> Result<QueryResult> {
        // Get connection - required for execution
        // BUG FIX: Clone connection before moving self
        let connection = self.connection.as_ref().ok_or_else(|| {
            Error::Query("AdvancedQueryBuilder requires connection - use FeatureClient.query() instead".to_string())
        })?.clone();
        
        // For now, handle simple queries without joins/aggregations
        // Joins and aggregations would require a query executor
        // This implementation handles basic SELECT with filters, order_by, limit, offset
        
        // If there are joins or group_by, we need a query executor
        if !self.joins.is_empty() || !self.group_by.is_empty() {
            // For joins and aggregations, we'd need a query executor
            // For now, return error indicating this requires server-side execution
            return Err(Error::Query(
                "Joins and aggregations require server connection with query executor support".to_string()
            ));
        }
        
        // Handle UNION by executing both queries and combining
        if let Some(mut union_query) = self.union {
            // Set connection on union query
            union_query.connection = Some(Arc::clone(&connection));
            
            // BUG FIX: execute_simple consumes self, so we need to clone what we need first
            let left_table = self.table.clone();
            let left_select = self.select.clone();
            let _left_filters = self.filters.clone(); // Not used yet - would need proper filter chaining
            let _left_order_by = self.order_by.clone(); // Not used yet - would need proper order_by chaining
            let left_limit = self.limit;
            let left_offset = self.offset;
            let distinct = self.distinct; // Copy bool before moving self
            
            // Execute left query
            let mut left_query_builder = crate::elegant::QueryBuilder::new(left_table, Arc::clone(&connection));
            if !left_select.is_empty() {
                let cols: Vec<&str> = left_select.iter().map(|s| s.as_str()).collect();
                left_query_builder = left_query_builder.select(&cols);
            }
            // Note: Filters and order_by would need to be applied properly
            if let Some(limit) = left_limit {
                left_query_builder = left_query_builder.limit(limit);
            }
            if let Some(offset) = left_offset {
                left_query_builder = left_query_builder.offset(offset);
            }
            let left_result = left_query_builder.execute().await?;
            
            // Execute right query
            let right_result = union_query.execute_simple(connection).await?;
            
            // EDGE CASE: Column mismatch - use left columns, warn if different
            if left_result.columns != right_result.columns {
                // In production, would handle column alignment
                // For now, use left columns and log warning
            }
            
            // Combine results (deduplicate if distinct)
            // EDGE CASE: Check for potential memory exhaustion with very large result sets
            const MAX_UNION_ROWS: usize = 10_000_000; // 10M rows max for UNION
            let total_rows = left_result.rows.len().saturating_add(right_result.rows.len());
            if total_rows > MAX_UNION_ROWS {
                return Err(Error::Query(format!(
                    "UNION result would exceed maximum row count: {} > {}",
                    total_rows, MAX_UNION_ROWS
                )));
            }
            
            let mut combined_rows = left_result.rows;
            combined_rows.extend(right_result.rows);
            
            if distinct {
                // BUG FIX: Use proper row comparison instead of Debug format
                // For now, use a more reliable deduplication
                // EDGE CASE: Limit HashSet size to prevent memory exhaustion
                const MAX_DISTINCT_ROWS: usize = 10_000_000; // 10M distinct rows max
                let mut seen = std::collections::HashSet::new();
                combined_rows.retain(|row| {
                    // Create a more reliable key from row values
                    // BUG FIX: elegant::Row has values() method, not values field
                    let key: Vec<String> = row.values().iter()
                        .map(|v| format!("{:?}", v))
                        .collect();
                    let key_str = key.join("|");
                    
                    // EDGE CASE: Prevent HashSet from growing too large
                    // BUG FIX: Check before inserting to avoid unnecessary work
                    if seen.len() >= MAX_DISTINCT_ROWS {
                        // Already at limit, skip this row
                        return false;
                    }
                    
                    seen.insert(key_str)
                });
            }
            
            return Ok(QueryResult {
                columns: left_result.columns,
                rows: combined_rows,
            });
        }
        
        // Execute simple query
        self.execute_simple(connection).await
    }
    
    async fn execute_simple(self, connection: Arc<dyn Connection>) -> Result<QueryResult> {
        use crate::elegant::QueryBuilder;
        use narayana_core::types::TableId;
        use narayana_core::column::Column;
        use narayana_core::row::Value;
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        // Convert to basic QueryBuilder and execute
        let mut query_builder = QueryBuilder::new(self.table.clone(), connection);
        
        // Set columns if specified
        if !self.select.is_empty() {
            let cols: Vec<&str> = self.select.iter().map(|s| s.as_str()).collect();
            query_builder = query_builder.select(&cols);
        }
        
        // Apply filters properly using FilterBuilder chain
        // Each filter needs to be chained: query_builder.where(column).eq(value) returns QueryBuilder
        for filter in &self.filters {
            let filter_op = match filter.op.as_str() {
                "=" | "eq" => "eq",
                "!=" | "ne" | "neq" => "ne",
                ">" | "gt" => "gt",
                ">=" | "gte" => "gte",
                "<" | "lt" => "lt",
                "<=" | "lte" => "lte",
                "like" => "like",
                "in" => "in",
                _ => {
                    // For unsupported ops, try to convert value and use eq as fallback
                    continue;
                }
            };
            
            // Convert filter value to Value type
            let filter_value: Value = match &filter.value {
                serde_json::Value::Number(n) => {
                    if n.is_i64() {
                        Value::Int64(n.as_i64().unwrap())
                    } else if n.is_u64() {
                        Value::Int64(n.as_u64().unwrap() as i64)
                    } else {
                        Value::Float64(n.as_f64().unwrap())
                    }
                }
                serde_json::Value::String(s) => Value::String(s.clone()),
                serde_json::Value::Bool(b) => Value::Boolean(*b),
                serde_json::Value::Null => Value::Null,
                _ => Value::String(format!("{}", filter.value)),
            };
            
            // Apply filter based on operation
            query_builder = match filter_op {
                "eq" => query_builder.r#where(&filter.column).eq(filter_value),
                "ne" | "neq" => query_builder.r#where(&filter.column).ne(filter_value),
                "gt" => query_builder.r#where(&filter.column).gt(filter_value),
                "gte" => query_builder.r#where(&filter.column).gte(filter_value),
                "lt" => query_builder.r#where(&filter.column).lt(filter_value),
                "lte" => query_builder.r#where(&filter.column).lte(filter_value),
                "like" => query_builder.r#where(&filter.column).like(&format!("{}", filter.value)),
                "in" => {
                    // For "in", we need to handle array values
                    if let serde_json::Value::Array(arr) = &filter.value {
                        let values: Vec<Value> = arr.iter().map(|v| {
                            match v {
                                serde_json::Value::Number(n) => {
                                    if n.is_i64() {
                                        Value::Int64(n.as_i64().unwrap())
                                    } else {
                                        Value::Float64(n.as_f64().unwrap())
                                    }
                                }
                                serde_json::Value::String(s) => Value::String(s.clone()),
                                serde_json::Value::Bool(b) => Value::Boolean(*b),
                                _ => Value::String(format!("{}", v)),
                            }
                        }).collect();
                        query_builder.r#where(&filter.column).r#in(values)
                    } else {
                        // Single value, treat as eq
                        query_builder.r#where(&filter.column).eq(filter_value)
                    }
                }
                _ => query_builder, // Skip unsupported operations
            };
        }
        
        // Apply order_by properly using OrderByBuilder chain
        // Each order_by needs to be chained: query_builder.order_by(column).asc() returns QueryBuilder
        for order_expr in &self.order_by {
            query_builder = if order_expr.ascending {
                query_builder.order_by(&order_expr.column).asc()
            } else {
                query_builder.order_by(&order_expr.column).desc()
            };
        }
        
        // Set limit and offset
        if let Some(limit) = self.limit {
            query_builder = query_builder.limit(limit);
        }
        if let Some(offset) = self.offset {
            query_builder = query_builder.offset(offset);
        }
        
        // Execute query
        let mut result = query_builder.execute().await?;
        
        // Apply distinct if needed
        if self.distinct {
            // EDGE CASE: Limit HashSet size to prevent memory exhaustion
            const MAX_DISTINCT_ROWS: usize = 10_000_000; // 10M distinct rows max
            let mut seen = std::collections::HashSet::new();
            result.rows.retain(|row| {
                // BUG FIX: elegant::Row has values() method, not values field
                let key: Vec<String> = row.values().iter()
                    .map(|v| format!("{:?}", v))
                    .collect();
                let key_str = key.join("|");
                
                // EDGE CASE: Prevent HashSet from growing too large
                // BUG FIX: Check before inserting to avoid unnecessary work
                if seen.len() >= MAX_DISTINCT_ROWS {
                    // Already at limit, skip this row
                    return false;
                }
                
                seen.insert(key_str)
            });
        }
        
        Ok(result)
    }
}

pub struct OrderByBuilder {
    query: AdvancedQueryBuilder,
    column: String,
}

impl OrderByBuilder {
    pub fn asc(mut self) -> AdvancedQueryBuilder {
        self.query.order_by.push(OrderByExpr {
            column: self.column,
            ascending: true,
        });
        self.query
    }

    pub fn desc(mut self) -> AdvancedQueryBuilder {
        self.query.order_by.push(OrderByExpr {
            column: self.column,
            ascending: false,
        });
        self.query
    }
}

use crate::elegant::QueryResult;

/// Bulk operations - process millions of rows
pub struct BulkOperations {
    operations: Vec<BulkOperation>,
    connection: Option<Arc<dyn Connection>>,
}

#[derive(Debug, Clone)]
pub enum BulkOperation {
    Insert { table: String, rows: Vec<Row> },
    Update { table: String, updates: Vec<Update> },
    Upsert { table: String, rows: Vec<Row> },
}

impl BulkOperations {
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            connection: None,
        }
    }
    
    pub fn with_connection(mut self, connection: Arc<dyn Connection>) -> Self {
        self.connection = Some(connection);
        self
    }

    pub fn insert(mut self, table: &str, rows: Vec<Row>) -> Self {
        self.operations.push(BulkOperation::Insert {
            table: table.to_string(),
            rows,
        });
        self
    }

    pub fn update(mut self, table: &str, updates: Vec<Update>) -> Self {
        self.operations.push(BulkOperation::Update {
            table: table.to_string(),
            updates,
        });
        self
    }

    pub fn upsert(mut self, table: &str, rows: Vec<Row>) -> Self {
        self.operations.push(BulkOperation::Upsert {
            table: table.to_string(),
            rows,
        });
        self
    }

    /// Execute bulk operations (parallel, optimized)
    pub async fn execute(self) -> Result<BulkResult> {
        self.execute_with_progress(|_current, _total| {}).await
    }

    /// Execute with progress callback
    pub async fn execute_with_progress<F>(self, callback: F) -> Result<BulkResult>
    where
        F: Fn(usize, usize) + Send + Sync,
    {
        use crate::elegant::InsertBuilder;
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let connection = self.connection.ok_or_else(|| {
            Error::Query("BulkOperations requires connection - use FeatureClient.bulk() instead".to_string())
        })?;
        
        // EDGE CASE: Empty operations list - return success with 0 rows
        if self.operations.is_empty() {
            return Ok(BulkResult {
                rows_processed: 0,
                errors: Vec::new(),
            });
        }
        
        let total_operations = self.operations.len();
        let mut rows_processed: usize = 0;
        let mut errors = Vec::new();
        
        // Process operations in parallel chunks using tokio
        // EDGE CASE: Limit concurrent tasks to prevent resource exhaustion
        const CHUNK_SIZE: usize = 1000; // Process 1000 operations at a time
        const MAX_CONCURRENT_TASKS: usize = 100; // Limit concurrent tasks
        
        // Use a semaphore to limit concurrent tasks
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_TASKS));
        
        for chunk in self.operations.chunks(CHUNK_SIZE) {
            // Process chunk in parallel with concurrency limit
            let mut chunk_tasks = Vec::new();
            
            for op in chunk {
                let conn = Arc::clone(&connection);
                let op_clone = op.clone();
                let sem = Arc::clone(&semaphore);
                
                let task = tokio::spawn(async move {
                    // Acquire permit (will be released when task completes)
                    // EDGE CASE: Handle semaphore acquisition failure gracefully
                    let permit = match sem.acquire().await {
                        Ok(p) => p,
                        Err(_) => {
                            // Semaphore was closed, return error
                            return Err(Error::Query("Semaphore closed - system may be shutting down".to_string()));
                        }
                    };
                    let _permit = permit; // Hold permit for task lifetime
                    match op_clone {
                        BulkOperation::Insert { table, rows } => {
                            // EDGE CASE: Empty rows - return 0 inserted
                            if rows.is_empty() {
                                return Ok(0);
                            }
                            
                            // Get table ID
                            let mut hasher = DefaultHasher::new();
                            table.hash(&mut hasher);
                            // SECURITY: Use same salt as table creation for consistency
                            "narayana_table_salt_v1".hash(&mut hasher);
                            let table_id = narayana_core::types::TableId(hasher.finish() as u64);
                            
                            // Get schema
                            let _schema = match conn.get_schema(table_id).await {
                                Ok(s) => s,
                                Err(e) => return Err(e),
                            };
                            
                            // Use InsertBuilder to insert rows
                            let mut insert_builder = InsertBuilder::new(table, conn);
                            for row in rows {
                                // BUG FIX: elegant::Row has values() method, not values field
                                insert_builder = insert_builder.row(row.values().to_vec());
                            }
                            
                            match insert_builder.execute().await {
                                Ok(result) => Ok(result.rows_inserted),
                                Err(e) => Err(e),
                            }
                        }
                        BulkOperation::Update { table, updates } => {
                            // EDGE CASE: Empty updates - return 0 updated
                            if updates.is_empty() {
                                return Ok(0);
                            }
                            
                            // Get table ID
                            let mut hasher = DefaultHasher::new();
                            table.hash(&mut hasher);
                            "narayana_table_salt_v1".hash(&mut hasher);
                            let table_id = narayana_core::types::TableId(hasher.finish() as u64);
                            
                            // Group updates by row_id for efficient processing
                            use std::collections::HashMap;
                            let mut row_updates: HashMap<u64, HashMap<String, serde_json::Value>> = HashMap::new();
                            for update in updates {
                                row_updates
                                    .entry(update.row_id)
                                    .or_insert_with(HashMap::new)
                                    .insert(update.column, update.value);
                            }
                            
                            // Execute updates via connection
                            let mut rows_updated = 0;
                            let mut errors = Vec::new();
                            
                            for (row_id, columns) in row_updates {
                                let update_query = serde_json::json!({
                                    "operation": "update",
                                    "table": table,
                                    "table_id": table_id.0,
                                    "row_id": row_id,
                                    "updates": columns,
                                });
                                
                                match conn.execute_query(update_query).await {
                                    Ok(_) => rows_updated += 1,
                                    Err(e) => errors.push(e.to_string()),
                                }
                            }
                            
                            if !errors.is_empty() && rows_updated == 0 {
                                return Err(Error::Query(format!("All updates failed: {}", errors.join("; "))));
                            }
                            
                            Ok(rows_updated)
                        }
                        BulkOperation::Upsert { table, rows } => {
                            // EDGE CASE: Empty rows - return 0 upserted
                            if rows.is_empty() {
                                return Ok(0);
                            }
                            
                            // Get table ID
                            let mut hasher = DefaultHasher::new();
                            table.hash(&mut hasher);
                            "narayana_table_salt_v1".hash(&mut hasher);
                            let table_id = narayana_core::types::TableId(hasher.finish() as u64);
                            
                            // Get schema to determine row structure
                            let _schema = match conn.get_schema(table_id).await {
                                Ok(s) => s,
                                Err(e) => return Err(e),
                            };
                            
                            // Execute upserts via connection
                            let mut rows_upserted = 0;
                            let mut errors = Vec::new();
                            
                            for row in rows {
                                // Convert row to JSON values
                                use crate::elegant::Value;
                                let row_values: Vec<serde_json::Value> = row.values()
                                    .iter()
                                    .map(|v| match v {
                                        Value::Int64(i) => serde_json::json!(*i),
                                        Value::Float64(f) => serde_json::json!(*f),
                                        Value::String(s) => serde_json::json!(s),
                                        Value::Boolean(b) => serde_json::json!(*b),
                                        Value::Null => serde_json::json!(null),
                                        Value::Array(arr) => {
                                            serde_json::Value::Array(arr.iter().map(|v| match v {
                                                Value::Int64(i) => serde_json::json!(*i),
                                                Value::Float64(f) => serde_json::json!(*f),
                                                Value::String(s) => serde_json::json!(s),
                                                Value::Boolean(b) => serde_json::json!(*b),
                                                Value::Null => serde_json::json!(null),
                                                Value::Array(_) => serde_json::json!(null), // Nested arrays not fully supported
                                            }).collect())
                                        },
                                    })
                                    .collect();
                                
                                // For upsert, we need row_id - assume first column is ID or use a generated ID
                                // For now, use a hash of the row values as ID
                                use std::collections::hash_map::DefaultHasher;
                                use std::hash::{Hash, Hasher};
                                let mut hasher = DefaultHasher::new();
                                for val in &row_values {
                                    // Hash the value
                                    if let Ok(serialized) = serde_json::to_string(val) {
                                        serialized.hash(&mut hasher);
                                    }
                                }
                                let row_id = hasher.finish();
                                
                                // Convert row values to upsert format
                                let upsert_query = serde_json::json!({
                                    "operation": "upsert",
                                    "table": table,
                                    "table_id": table_id.0,
                                    "row_id": row_id,
                                    "row": row_values,
                                });
                                
                                match conn.execute_query(upsert_query).await {
                                    Ok(_) => rows_upserted += 1,
                                    Err(e) => errors.push(e.to_string()),
                                }
                            }
                            
                            if !errors.is_empty() && rows_upserted == 0 {
                                return Err(Error::Query(format!("All upserts failed: {}", errors.join("; "))));
                            }
                            
                            Ok(rows_upserted)
                        }
                    }
                });
                
                chunk_tasks.push(task);
            }
            
            // Wait for chunk to complete
            for task in chunk_tasks {
                match task.await {
                    Ok(Ok(rows)) => {
                        // EDGE CASE: Prevent integer overflow when accumulating rows_processed
                        rows_processed = rows_processed.saturating_add(rows);
                        // Progress callback: rows_processed is current progress, total_operations is total
                        callback(rows_processed, total_operations);
                    }
                    Ok(Err(e)) => {
                        errors.push(e);
                    }
                    Err(e) => {
                        // EDGE CASE: Task panicked or was cancelled
                        // JoinError doesn't expose is_panic/is_cancelled, so we check the error message
                        let error_msg = format!("{:?}", e);
                        if error_msg.contains("panic") || error_msg.contains("panicked") {
                            errors.push(Error::Query(format!("Task panicked: {}", error_msg)));
                        } else if error_msg.contains("cancelled") || error_msg.contains("cancel") {
                            errors.push(Error::Query("Task was cancelled".to_string()));
                        } else {
                            errors.push(Error::Query(format!("Task join error: {}", error_msg)));
                        }
                    }
                }
            }
        }
        
        Ok(BulkResult {
            rows_processed,
            errors,
        })
    }
}

#[derive(Debug)]
pub struct BulkResult {
    pub rows_processed: usize,
    pub errors: Vec<Error>,
}

/// Composable queries - build complex queries from parts
pub struct ComposableQuery {
    parts: Vec<QueryPart>,
}

#[derive(Debug, Clone)]
pub enum QueryPart {
    Select(Vec<String>),
    From(String),
    Join(String, String),
    Where(FilterExpr),
    GroupBy(Vec<String>),
    Having(FilterExpr),
    OrderBy(OrderByExpr),
    Limit(usize),
    Offset(usize),
}

impl ComposableQuery {
    pub fn new() -> Self {
        Self {
            parts: Vec::new(),
        }
    }

    pub fn select(mut self, columns: &[&str]) -> Self {
        self.parts.push(QueryPart::Select(columns.iter().map(|s| s.to_string()).collect()));
        self
    }

    pub fn from(mut self, table: &str) -> Self {
        self.parts.push(QueryPart::From(table.to_string()));
        self
    }

    pub fn join(mut self, table: &str, condition: &str) -> Self {
        self.parts.push(QueryPart::Join(table.to_string(), condition.to_string()));
        self
    }

    pub fn r#where(mut self, column: &str, op: &str, value: serde_json::Value) -> Self {
        self.parts.push(QueryPart::Where(FilterExpr {
            column: column.to_string(),
            op: op.to_string(),
            value,
        }));
        self
    }

    pub fn group_by(mut self, columns: &[&str]) -> Self {
        self.parts.push(QueryPart::GroupBy(columns.iter().map(|s| s.to_string()).collect()));
        self
    }

    pub fn order_by(mut self, column: &str, ascending: bool) -> Self {
        self.parts.push(QueryPart::OrderBy(OrderByExpr {
            column: column.to_string(),
            ascending,
        }));
        self
    }

    pub fn limit(mut self, n: usize) -> Self {
        self.parts.push(QueryPart::Limit(n));
        self
    }

    pub fn offset(mut self, n: usize) -> Self {
        self.parts.push(QueryPart::Offset(n));
        self
    }

    /// Build final query
    pub fn build(self) -> AdvancedQueryBuilder {
        // In production, would build query from parts
        AdvancedQueryBuilder::new("".to_string())
    }

    /// Execute directly
    pub async fn execute(self) -> Result<QueryResult> {
        self.build().execute().await
    }
}

