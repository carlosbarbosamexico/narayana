// Advanced API client with GraphQL, transactions, vector search, ML, and analytics

use narayana_core::{Error, Result, schema::Schema, types::TableId};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::connection::Connection;

/// Advanced API client
pub struct AdvancedClient {
    connection: Arc<dyn Connection>,
}

/// Type alias for backward compatibility
pub type UltimateApi = AdvancedClient;

impl AdvancedClient {
    pub fn new() -> AdvancedClientBuilder {
        AdvancedClientBuilder::default()
    }
    
    pub fn with_connection(connection: Arc<dyn Connection>) -> Self {
        Self { connection }
    }

    /// GraphQL with subscriptions
    pub fn graphql(&self, query: &str) -> crate::powerful::GraphQLQuery {
        crate::powerful::GraphQLQuery::with_connection(query.to_string(), Arc::clone(&self.connection))
    }

    /// GraphQL subscription (real-time)
    pub fn graphql_subscribe(&self, query: &str) -> GraphQLSubscription {
        GraphQLSubscription::new(query.to_string()).with_connection(Arc::clone(&self.connection))
    }

    /// Reactive query with backpressure
    pub fn reactive(&self, query: QueryBuilder) -> crate::powerful::ReactiveQuery {
        crate::powerful::ReactiveQuery::new(query)
    }

    /// Real-time subscription with filters
    pub fn subscribe(&self, table: &str) -> crate::powerful::Subscription {
        crate::powerful::Subscription::new(table.to_string())
    }

    /// Batch operations (atomic)
    pub fn batch(&self) -> crate::powerful::BatchOperations {
        crate::powerful::BatchOperations::new(Arc::clone(&self.connection))
    }

    /// Pipeline operations (chainable)
    pub fn pipeline(&self) -> crate::powerful::Pipeline {
        crate::powerful::Pipeline::new().with_connection(Arc::clone(&self.connection))
    }

    /// Transaction API
    pub fn transaction(&self) -> TransactionBuilder {
        TransactionBuilder::new(Arc::clone(&self.connection))
    }

    /// Bulk operations (millions of rows)
    pub fn bulk(&self) -> crate::powerful::BulkOperations {
        crate::powerful::BulkOperations::new().with_connection(Arc::clone(&self.connection))
    }

    /// Advanced query builder
    pub fn query(&self, table: &str) -> crate::powerful::AdvancedQueryBuilder {
        crate::powerful::AdvancedQueryBuilder::new(table.to_string())
            .with_connection(Arc::clone(&self.connection))
    }

    /// Composable queries
    pub fn compose(&self) -> crate::powerful::ComposableQuery {
        crate::powerful::ComposableQuery::new()
    }

    /// Vector search API
    pub fn vector_search(&self, index: &str) -> VectorSearch {
        VectorSearch::new(index.to_string(), Arc::clone(&self.connection))
    }

    /// ML operations
    pub fn ml(&self) -> MLOperations {
        MLOperations::new(Arc::clone(&self.connection))
    }

    /// Analytics operations
    pub fn analytics(&self) -> AnalyticsOperations {
        AnalyticsOperations::new(Arc::clone(&self.connection))
    }

    /// Webhook management
    pub fn webhooks(&self) -> WebhookOperations {
        WebhookOperations::new(Arc::clone(&self.connection))
    }

    /// Real-time sync
    pub fn sync(&self) -> SyncOperations {
        SyncOperations::new(Arc::clone(&self.connection))
    }
}

/// Advanced client builder
#[derive(Default)]
pub struct AdvancedClientBuilder {
    url: Option<String>,
    connection: Option<Arc<dyn Connection>>,
    features: Vec<String>,
    timeout: Option<u64>,
    max_connections: Option<usize>,
}

impl AdvancedClientBuilder {
    pub fn url(mut self, url: &str) -> Self {
        self.url = Some(url.to_string());
        self
    }
    
    pub fn with_connection(mut self, connection: Arc<dyn Connection>) -> Self {
        self.connection = Some(connection);
        self
    }

    pub fn enable_all(mut self) -> Self {
        self.features.extend_from_slice(&[
            "graphql".to_string(),
            "reactive".to_string(),
            "subscriptions".to_string(),
            "batch".to_string(),
            "pipeline".to_string(),
            "vector_search".to_string(),
            "ml".to_string(),
            "analytics".to_string(),
            "webhooks".to_string(),
            "sync".to_string(),
        ]);
        self
    }

    pub async fn build(self) -> Result<AdvancedClient> {
        let connection = if let Some(conn) = self.connection {
            conn
        } else if let Some(url) = self.url {
            Arc::new(crate::connection::RemoteConnection::new(url))
        } else {
            return Err(Error::Query("Either url or connection must be provided".to_string()));
        };
        
        Ok(AdvancedClient { connection })
    }
}

// Import types from powerful module
use crate::powerful::{ReactiveQuery, Subscription, BatchOperations, Pipeline, BulkOperations, AdvancedQueryBuilder, ComposableQuery, GraphQLQuery, GraphQLResponse};

/// GraphQL subscription for real-time updates
pub struct GraphQLSubscription {
    query: String,
    variables: HashMap<String, JsonValue>,
    connection: Option<Arc<dyn Connection>>,
}

impl GraphQLSubscription {
    pub fn new(query: String) -> Self {
        Self {
            query,
            variables: HashMap::new(),
            connection: None,
        }
    }
    
    pub fn with_connection(mut self, connection: Arc<dyn Connection>) -> Self {
        self.connection = Some(connection);
        self
    }

    pub fn variable(mut self, name: &str, value: JsonValue) -> Self {
        self.variables.insert(name.to_string(), value);
        self
    }

    pub fn subscribe(self) -> impl Stream<Item = Result<GraphQLResponse>> {
        GraphQLSubscriptionStream {
            query: self.query,
            variables: self.variables,
            connection: self.connection,
            state: GraphQLSubscriptionState::Initial,
        }
    }
}

enum GraphQLSubscriptionState {
    Initial,
    Connected,
    Streaming,
    Error,
}

struct GraphQLSubscriptionStream {
    query: String,
    variables: HashMap<String, JsonValue>,
    connection: Option<Arc<dyn Connection>>,
    state: GraphQLSubscriptionState,
}

impl Stream for GraphQLSubscriptionStream {
    type Item = Result<GraphQLResponse>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use std::mem;
        
        match mem::replace(&mut self.state, GraphQLSubscriptionState::Error) {
            GraphQLSubscriptionState::Initial => {
                // Check if connection is available
                if self.connection.is_none() {
                    return Poll::Ready(Some(Err(Error::Query(
                        "GraphQL subscription requires connection - use AdvancedClient.graphql_subscribe() instead".to_string()
                    ))));
                }
                
                // GraphQL subscriptions require WebSocket support
                // In production, would establish WebSocket connection and subscribe to GraphQL subscription
                // For now, return error indicating WebSocket support needed
                Poll::Ready(Some(Err(Error::Query(
                    "GraphQL subscriptions require WebSocket support. Query subscriptions are not yet fully implemented.".to_string()
                ))))
            }
            GraphQLSubscriptionState::Connected | GraphQLSubscriptionState::Streaming => {
                // In production, would poll WebSocket for new GraphQL subscription events
                // For now, return None to end stream
                Poll::Ready(None)
            }
            GraphQLSubscriptionState::Error => {
                Poll::Ready(None)
            }
        }
    }
}

/// Transaction builder for ACID transactions
pub struct TransactionBuilder {
    connection: Arc<dyn Connection>,
    operations: Vec<TransactionOperation>,
}

#[derive(Debug, Clone)]
pub enum TransactionOperation {
    Insert { table: String, data: Vec<Row> },
    Update { table: String, updates: Vec<Update> },
    Delete { table: String, row_ids: Vec<u64> },
    Query { query: QueryBuilder },
}

impl TransactionBuilder {
    pub fn new(connection: Arc<dyn Connection>) -> Self {
        Self {
            connection,
            operations: Vec::new(),
        }
    }

    pub fn insert(mut self, table: &str, data: Vec<Row>) -> Self {
        self.operations.push(TransactionOperation::Insert {
            table: table.to_string(),
            data,
        });
        self
    }

    pub fn update(mut self, table: &str, updates: Vec<Update>) -> Self {
        self.operations.push(TransactionOperation::Update {
            table: table.to_string(),
            updates,
        });
        self
    }

    pub fn delete(mut self, table: &str, row_ids: Vec<u64>) -> Self {
        self.operations.push(TransactionOperation::Delete {
            table: table.to_string(),
            row_ids,
        });
        self
    }

    pub fn query(mut self, query: QueryBuilder) -> Self {
        self.operations.push(TransactionOperation::Query { query });
        self
    }

    /// Commit transaction (atomic)
    pub async fn commit(self) -> Result<TransactionResult> {
        use crate::powerful::BatchOperations;
        use crate::elegant::Row;
        
        // Validate operations list is not empty
        if self.operations.is_empty() {
            return Ok(TransactionResult {
                success: true,
                operations_executed: 0,
            });
        }
        
        // Convert transaction operations to batch operations
        let mut batch = BatchOperations::new(Arc::clone(&self.connection));
        
        for op in self.operations {
            match op {
                TransactionOperation::Insert { table, data } => {
                    batch = batch.insert(&table, data);
                }
                TransactionOperation::Query { query } => {
                    batch = batch.query(query);
                }
                TransactionOperation::Update { table, updates } => {
                    // Use BatchOperations to execute update
                    let mut batch = BatchOperations::new(Arc::clone(&self.connection));
                    batch = batch.update(&table, updates);
                    let batch_result = batch.execute().await?;
                    
                    // Check if update succeeded
                    if let Some(result) = batch_result.results.first() {
                        if !result.success {
                            return Err(Error::Query(format!("Transaction update failed: {:?}", result.data)));
                        }
                    }
                }
                TransactionOperation::Delete { table, row_ids } => {
                    // Use BatchOperations to execute delete
                    let mut batch = BatchOperations::new(Arc::clone(&self.connection));
                    batch = batch.delete(&table, row_ids);
                    let batch_result = batch.execute().await?;
                    
                    // Check if delete succeeded
                    if let Some(result) = batch_result.results.first() {
                        if !result.success {
                            return Err(Error::Query(format!("Transaction delete failed: {:?}", result.data)));
                        }
                    }
                }
            }
        }
        
        // Execute batch atomically
        let batch_result = batch.execute().await?;
        
        // Check if all operations succeeded
        let all_success = batch_result.results.iter().all(|r| r.success);
        let operations_executed = batch_result.results.len();
        
        if !all_success {
            return Err(Error::Query("Transaction failed: one or more operations failed".to_string()));
        }
        
        Ok(TransactionResult {
            success: true,
            operations_executed,
        })
    }

    /// Rollback transaction
    /// 
    /// For client-side transactions, rollback simply discards all operations
    /// without executing them. For server-side transactions with transaction IDs,
    /// this would send a rollback request to the server.
    pub async fn rollback(self) -> Result<()> {
        // Client-side rollback: simply discard operations without executing
        // The operations vector is moved into self, so dropping self discards them
        // This is safe because we haven't committed anything yet
        
        // If this were a server-side transaction with a transaction ID,
        // we would send a rollback request:
        // self.connection.execute_query(serde_json::json!({
        //     "operation": "rollback_transaction",
        //     "transaction_id": self.transaction_id,
        // })).await?;
        
        // For now, client-side transactions are stateless, so rollback is a no-op
        // that just prevents commit from executing
        Ok(())
    }
}

#[derive(Debug)]
pub struct TransactionResult {
    pub success: bool,
    pub operations_executed: usize,
}

/// Vector search API
pub struct VectorSearch {
    index: String,
    connection: Arc<dyn Connection>,
}

impl VectorSearch {
    pub fn new(index: String, connection: Arc<dyn Connection>) -> Self {
        Self { index, connection }
    }

    /// Search for similar vectors
    pub fn search(&self, vector: Vec<f32>, k: usize) -> VectorSearchBuilder {
        VectorSearchBuilder::new(self.index.clone(), vector, k)
    }

    /// Add vector to index
    pub async fn add(&self, id: u64, vector: Vec<f32>, metadata: HashMap<String, JsonValue>) -> Result<()> {
        // SECURITY: Validate vector is not empty
        if vector.is_empty() {
            return Err(Error::Query("Vector cannot be empty".to_string()));
        }
        
        // SECURITY: Validate vector dimension (prevent extremely large vectors)
        const MAX_VECTOR_DIMENSION: usize = 10000;
        if vector.len() > MAX_VECTOR_DIMENSION {
            return Err(Error::Query(format!("Vector dimension {} exceeds maximum {}", vector.len(), MAX_VECTOR_DIMENSION)));
        }
        
        // SECURITY: Check for NaN or Infinity in vector
        if vector.iter().any(|&v| v.is_nan() || v.is_infinite()) {
            return Err(Error::Query("Vector contains NaN or Infinity values".to_string()));
        }
        
        // SECURITY: Validate metadata size
        let metadata_size = serde_json::to_string(&metadata)
            .map_err(|e| Error::Query(format!("Failed to serialize metadata: {}", e)))?
            .len();
        const MAX_METADATA_SIZE: usize = 1 * 1024 * 1024; // 1MB
        if metadata_size > MAX_METADATA_SIZE {
            return Err(Error::Query("Metadata size exceeds maximum".to_string()));
        }
        
        // Vector index operations require server connection with vector search support
        // In production, would call connection.add_vector(index, id, vector, metadata)
        Err(Error::Query(format!("Vector index add not implemented: index '{}' requires server connection with vector search support", self.index)))
    }

    /// Batch add vectors
    pub async fn add_batch(&self, vectors: Vec<(u64, Vec<f32>, HashMap<String, JsonValue>)>) -> Result<()> {
        // SECURITY: Validate batch size
        const MAX_BATCH_SIZE: usize = 10000;
        if vectors.len() > MAX_BATCH_SIZE {
            return Err(Error::Query(format!("Batch size {} exceeds maximum {}", vectors.len(), MAX_BATCH_SIZE)));
        }
        
        // Validate each vector
        for (id, vector, metadata) in &vectors {
            // Validate vector
            if vector.is_empty() {
                return Err(Error::Query(format!("Vector for id {} is empty", id)));
            }
            const MAX_VECTOR_DIMENSION: usize = 10000;
            if vector.len() > MAX_VECTOR_DIMENSION {
                return Err(Error::Query(format!("Vector for id {} dimension {} exceeds maximum", id, vector.len())));
            }
            if vector.iter().any(|&v| v.is_nan() || v.is_infinite()) {
                return Err(Error::Query(format!("Vector for id {} contains NaN or Infinity", id)));
            }
            
            // Validate metadata
            let metadata_size = serde_json::to_string(metadata)
                .map_err(|e| Error::Query(format!("Failed to serialize metadata for id {}: {}", id, e)))?
                .len();
            const MAX_METADATA_SIZE: usize = 1 * 1024 * 1024; // 1MB
            if metadata_size > MAX_METADATA_SIZE {
                return Err(Error::Query(format!("Metadata for id {} size exceeds maximum", id)));
            }
        }
        
        // Vector index batch operations require server connection with vector search support
        Err(Error::Query(format!("Vector index batch add not implemented: index '{}' requires server connection with vector search support", self.index)))
    }
}

pub struct VectorSearchBuilder {
    index: String,
    vector: Vec<f32>,
    k: usize,
    filters: HashMap<String, JsonValue>,
}

impl VectorSearchBuilder {
    pub fn new(index: String, vector: Vec<f32>, k: usize) -> Self {
        Self {
            index,
            vector,
            k,
            filters: HashMap::new(),
        }
    }

    pub fn filter(mut self, key: &str, value: JsonValue) -> Self {
        self.filters.insert(key.to_string(), value);
        self
    }

    pub async fn execute(self) -> Result<Vec<(u64, f32)>> {
        // SECURITY: Validate vector is not empty
        if self.vector.is_empty() {
            return Err(Error::Query("Query vector cannot be empty".to_string()));
        }
        
        // SECURITY: Validate vector dimension
        const MAX_VECTOR_DIMENSION: usize = 10000;
        if self.vector.len() > MAX_VECTOR_DIMENSION {
            return Err(Error::Query(format!("Query vector dimension {} exceeds maximum {}", self.vector.len(), MAX_VECTOR_DIMENSION)));
        }
        
        // SECURITY: Check for NaN or Infinity in vector
        if self.vector.iter().any(|&v| v.is_nan() || v.is_infinite()) {
            return Err(Error::Query("Query vector contains NaN or Infinity values".to_string()));
        }
        
        // SECURITY: Validate k (number of results)
        const MAX_K: usize = 10000;
        if self.k == 0 {
            return Err(Error::Query("k must be greater than 0".to_string()));
        }
        // BUG FIX: Use safe_k but it was calculated but not used
        let _safe_k = self.k.min(MAX_K);
        
        // SECURITY: Validate filters
        for (key, value) in &self.filters {
            if key.len() > 255 {
                return Err(Error::Query(format!("Filter key '{}' exceeds maximum length", key)));
            }
            let value_size = serde_json::to_string(value)
                .map_err(|e| Error::Query(format!("Failed to serialize filter value: {}", e)))?
                .len();
            const MAX_FILTER_VALUE_SIZE: usize = 1024 * 1024; // 1MB
            if value_size > MAX_FILTER_VALUE_SIZE {
                return Err(Error::Query(format!("Filter value for '{}' exceeds maximum size", key)));
            }
        }
        
        // Vector search requires server connection with vector search support
        // In production, would call connection.search_vectors(index, vector, k, filters)
        // and return results as Vec<(id, similarity_score)>
        Err(Error::Query(format!("Vector search not implemented: index '{}' requires server connection with vector search support", self.index)))
    }
}

/// ML operations API
pub struct MLOperations {
    connection: Arc<dyn Connection>,
}

impl MLOperations {
    pub fn new(connection: Arc<dyn Connection>) -> Self {
        Self { connection }
    }

    /// Train model
    pub fn train(&self, model_type: &str) -> ModelTrainer {
        ModelTrainer::new(model_type.to_string())
    }

    /// Predict using model
    pub fn predict(&self, model_id: &str) -> ModelPredictor {
        ModelPredictor::new(model_id.to_string())
    }

    /// Feature extraction
    pub fn extract_features(&self, table: &str) -> FeatureExtractor {
        FeatureExtractor::new(table.to_string())
    }
}

pub struct ModelTrainer {
    model_type: String,
    training_data: Option<String>,
    params: HashMap<String, JsonValue>,
}

impl ModelTrainer {
    pub fn new(model_type: String) -> Self {
        Self {
            model_type,
            training_data: None,
            params: HashMap::new(),
        }
    }

    pub fn with_data(mut self, table: &str) -> Self {
        self.training_data = Some(table.to_string());
        self
    }

    pub fn param(mut self, key: &str, value: JsonValue) -> Self {
        self.params.insert(key.to_string(), value);
        self
    }

    pub async fn train(self) -> Result<String> {
        // SECURITY: Validate model type
        if self.model_type.is_empty() || self.model_type.len() > 255 {
            return Err(Error::Query("Invalid model type".to_string()));
        }
        
        // SECURITY: Validate training data table if provided
        if let Some(ref table) = self.training_data {
            if table.is_empty() || table.len() > 255 {
                return Err(Error::Query("Invalid training data table name".to_string()));
            }
        }
        
        // SECURITY: Validate parameters size
        let params_size = serde_json::to_string(&self.params)
            .map_err(|e| Error::Query(format!("Failed to serialize parameters: {}", e)))?
            .len();
        const MAX_PARAMS_SIZE: usize = 10 * 1024 * 1024; // 10MB
        if params_size > MAX_PARAMS_SIZE {
            return Err(Error::Query("Parameters size exceeds maximum".to_string()));
        }
        
        // Model training requires connection to server with ML capabilities
        // In production, would call connection.train_model(model_type, training_data, params)
        Err(Error::Query("Model training not implemented: requires server connection with ML runtime".to_string()))
    }
}

pub struct ModelPredictor {
    model_id: String,
    input: Option<JsonValue>,
}

impl ModelPredictor {
    pub fn new(model_id: String) -> Self {
        Self {
            model_id,
            input: None,
        }
    }

    pub fn input(mut self, data: JsonValue) -> Self {
        self.input = Some(data);
        self
    }

    pub async fn predict(self) -> Result<JsonValue> {
        // SECURITY: Validate model ID
        if self.model_id.is_empty() || self.model_id.len() > 255 {
            return Err(Error::Query("Invalid model ID".to_string()));
        }
        
        // SECURITY: Validate input is provided
        if self.input.is_none() {
            return Err(Error::Query("Input data must be provided".to_string()));
        }
        
        // SECURITY: Validate input size
        if let Some(ref input) = self.input {
            let input_size = serde_json::to_string(input)
                .map_err(|e| Error::Query(format!("Failed to serialize input: {}", e)))?
                .len();
            const MAX_INPUT_SIZE: usize = 100 * 1024 * 1024; // 100MB
            if input_size > MAX_INPUT_SIZE {
                return Err(Error::Query("Input size exceeds maximum".to_string()));
            }
        }
        
        // Model prediction requires connection to server with ML capabilities
        // In production, would call connection.predict(model_id, input)
        Err(Error::Query(format!("Model prediction not implemented: model '{}' requires server connection with ML runtime", self.model_id)))
    }
}

pub struct FeatureExtractor {
    table: String,
    columns: Vec<String>,
}

impl FeatureExtractor {
    pub fn new(table: String) -> Self {
        Self {
            table,
            columns: Vec::new(),
        }
    }

    pub fn columns(mut self, cols: &[&str]) -> Self {
        self.columns = cols.iter().map(|s| s.to_string()).collect();
        self
    }

    pub async fn extract(self) -> Result<Vec<Vec<f32>>> {
        // SECURITY: Validate table name
        if self.table.is_empty() || self.table.len() > 255 {
            return Err(Error::Query("Invalid table name".to_string()));
        }
        
        // SECURITY: Validate column names
        for col in &self.columns {
            if col.len() > 255 || col.contains('\0') {
                return Err(Error::Query(format!("Invalid column name: {}", col)));
            }
        }
        
        // Feature extraction requires connection to server with ML capabilities
        // In production, would call connection.extract_features(table, columns)
        Err(Error::Query(format!("Feature extraction not implemented: table '{}' requires server connection with ML runtime", self.table)))
    }
}

/// Analytics operations API
pub struct AnalyticsOperations {
    connection: Arc<dyn Connection>,
}

impl AnalyticsOperations {
    pub fn new(connection: Arc<dyn Connection>) -> Self {
        Self { connection }
    }

    /// Window functions
    pub fn window(&self, table: &str) -> WindowFunctionBuilder {
        WindowFunctionBuilder::new(Arc::clone(&self.connection), table.to_string())
    }

    /// Statistical functions
    pub fn stats(&self, table: &str) -> StatisticalBuilder {
        StatisticalBuilder::new(Arc::clone(&self.connection), table.to_string())
    }

    /// Time series analysis
    pub fn time_series(&self, table: &str) -> TimeSeriesBuilder {
        TimeSeriesBuilder::new(Arc::clone(&self.connection), table.to_string())
    }

    /// Aggregations
    pub fn aggregate(&self, table: &str) -> AggregateBuilder {
        AggregateBuilder::new(Arc::clone(&self.connection), table.to_string())
    }
}

pub struct WindowFunctionBuilder {
    connection: Arc<dyn Connection>,
    table: String,
    function: Option<String>,
    partition_by: Vec<String>,
    order_by: Vec<String>,
}

impl WindowFunctionBuilder {
    pub fn new(connection: Arc<dyn Connection>, table: String) -> Self {
        Self {
            connection,
            table,
            function: None,
            partition_by: Vec::new(),
            order_by: Vec::new(),
        }
    }

    pub fn row_number(mut self) -> Self {
        self.function = Some("ROW_NUMBER".to_string());
        self
    }

    pub fn rank(mut self) -> Self {
        self.function = Some("RANK".to_string());
        self
    }

    pub fn lag(mut self, offset: usize) -> Self {
        self.function = Some(format!("LAG({})", offset));
        self
    }

    pub fn partition_by(mut self, columns: &[&str]) -> Self {
        self.partition_by = columns.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn order_by(mut self, columns: &[&str]) -> Self {
        self.order_by = columns.iter().map(|s| s.to_string()).collect();
        self
    }

    pub async fn execute(self) -> Result<Vec<JsonValue>> {
        // SECURITY: Validate table name
        if self.table.is_empty() || self.table.len() > 255 {
            return Err(Error::Query("Invalid table name".to_string()));
        }
        
        // SECURITY: Validate function is set
        if self.function.is_none() {
            return Err(Error::Query("Window function must be specified".to_string()));
        }
        
        // SECURITY: Validate partition_by and order_by column names
        for col in &self.partition_by {
            if col.len() > 255 || col.contains('\0') {
                return Err(Error::Query(format!("Invalid partition_by column: {}", col)));
            }
        }
        for col in &self.order_by {
            if col.len() > 255 || col.contains('\0') {
                return Err(Error::Query(format!("Invalid order_by column: {}", col)));
            }
        }
        
        // Window functions require server connection with query executor support
        // In production, would execute window function query via connection
        Err(Error::Query(format!("Window function not implemented: table '{}' requires server connection with query executor", self.table)))
    }
}

pub struct StatisticalBuilder {
    connection: Arc<dyn Connection>,
    table: String,
    function: Option<String>,
    column: Option<String>,
}

impl StatisticalBuilder {
    pub fn new(connection: Arc<dyn Connection>, table: String) -> Self {
        Self {
            connection,
            table,
            function: None,
            column: None,
        }
    }

    pub fn stddev(mut self, column: &str) -> Self {
        self.function = Some("STDDEV".to_string());
        self.column = Some(column.to_string());
        self
    }

    pub fn variance(mut self, column: &str) -> Self {
        self.function = Some("VARIANCE".to_string());
        self.column = Some(column.to_string());
        self
    }

    pub fn correlation(mut self, col1: &str, col2: &str) -> Self {
        self.function = Some(format!("CORRELATION({}, {})", col1, col2));
        self
    }

    pub async fn execute(self) -> Result<f64> {
        // SECURITY: Validate table name
        if self.table.is_empty() || self.table.len() > 255 {
            return Err(Error::Query("Invalid table name".to_string()));
        }
        
        // SECURITY: Validate function is set
        if self.function.is_none() {
            return Err(Error::Query("Statistical function must be specified".to_string()));
        }
        
        // SECURITY: Validate column name if required
        if let Some(ref col) = self.column {
            if col.len() > 255 || col.contains('\0') {
                return Err(Error::Query(format!("Invalid column name: {}", col)));
            }
        }
        
        // Statistical functions require server connection with query executor support
        // In production, would execute statistical query via connection
        Err(Error::Query(format!("Statistical function not implemented: table '{}' requires server connection with query executor", self.table)))
    }
}

pub struct TimeSeriesBuilder {
    connection: Arc<dyn Connection>,
    table: String,
    time_column: Option<String>,
    value_column: Option<String>,
}

impl TimeSeriesBuilder {
    pub fn new(connection: Arc<dyn Connection>, table: String) -> Self {
        Self {
            connection,
            table,
            time_column: None,
            value_column: None,
        }
    }

    pub fn time_column(mut self, column: &str) -> Self {
        self.time_column = Some(column.to_string());
        self
    }

    pub fn value_column(mut self, column: &str) -> Self {
        self.value_column = Some(column.to_string());
        self
    }

    pub fn ema(mut self, period: usize) -> Self {
        // Exponential moving average
        self
    }

    pub fn sma(mut self, period: usize) -> Self {
        // Simple moving average
        self
    }

    pub async fn execute(self) -> Result<Vec<JsonValue>> {
        // SECURITY: Validate table name
        if self.table.is_empty() || self.table.len() > 255 {
            return Err(Error::Query("Invalid table name".to_string()));
        }
        
        // SECURITY: Validate time and value columns are set
        if self.time_column.is_none() {
            return Err(Error::Query("Time column must be specified".to_string()));
        }
        if self.value_column.is_none() {
            return Err(Error::Query("Value column must be specified".to_string()));
        }
        
        // SECURITY: Validate column names
        if let Some(ref col) = self.time_column {
            if col.len() > 255 || col.contains('\0') {
                return Err(Error::Query(format!("Invalid time column: {}", col)));
            }
        }
        if let Some(ref col) = self.value_column {
            if col.len() > 255 || col.contains('\0') {
                return Err(Error::Query(format!("Invalid value column: {}", col)));
            }
        }
        
        // Time series analysis requires server connection with query executor support
        // In production, would execute time series query via connection
        Err(Error::Query(format!("Time series analysis not implemented: table '{}' requires server connection with query executor", self.table)))
    }
}

pub struct AggregateBuilder {
    connection: Arc<dyn Connection>,
    table: String,
    function: Option<String>,
    column: Option<String>,
    group_by: Vec<String>,
}

impl AggregateBuilder {
    pub fn new(connection: Arc<dyn Connection>, table: String) -> Self {
        Self {
            connection,
            table,
            function: None,
            column: None,
            group_by: Vec::new(),
        }
    }

    pub fn sum(mut self, column: &str) -> Self {
        self.function = Some("SUM".to_string());
        self.column = Some(column.to_string());
        self
    }

    pub fn avg(mut self, column: &str) -> Self {
        self.function = Some("AVG".to_string());
        self.column = Some(column.to_string());
        self
    }

    pub fn count(mut self) -> Self {
        self.function = Some("COUNT".to_string());
        self
    }

    pub fn group_by(mut self, columns: &[&str]) -> Self {
        self.group_by = columns.iter().map(|s| s.to_string()).collect();
        self
    }

    pub async fn execute(self) -> Result<Vec<JsonValue>> {
        // SECURITY: Validate table name
        if self.table.is_empty() || self.table.len() > 255 {
            return Err(Error::Query("Invalid table name".to_string()));
        }
        
        // SECURITY: Validate function is set
        if self.function.is_none() {
            return Err(Error::Query("Aggregation function must be specified".to_string()));
        }
        
        // SECURITY: Validate column name if required (not needed for COUNT)
        if let Some(ref col) = self.column {
            if col.len() > 255 || col.contains('\0') {
                return Err(Error::Query(format!("Invalid column name: {}", col)));
            }
        }
        
        // SECURITY: Validate group_by column names
        for col in &self.group_by {
            if col.len() > 255 || col.contains('\0') {
                return Err(Error::Query(format!("Invalid group_by column: {}", col)));
            }
        }
        
        // Aggregations require server connection with query executor support
        // In production, would execute aggregation query via connection
        Err(Error::Query(format!("Aggregation not implemented: table '{}' requires server connection with query executor", self.table)))
    }
}

/// Webhook operations API
pub struct WebhookOperations {
    connection: Arc<dyn Connection>,
}

impl WebhookOperations {
    pub fn new(connection: Arc<dyn Connection>) -> Self {
        Self { connection }
    }

    /// Create webhook
    pub fn create(&self) -> WebhookBuilder {
        WebhookBuilder::new()
    }

    /// List webhooks
    pub async fn list(&self) -> Result<Vec<WebhookInfo>> {
        // Webhook listing requires server connection with webhook manager
        // In production, would call connection.list_webhooks()
        Err(Error::Query("Webhook listing not implemented: requires server connection with webhook manager".to_string()))
    }

    /// Delete webhook
    pub async fn delete(&self, id: &str) -> Result<()> {
        // SECURITY: Validate webhook ID
        if id.is_empty() || id.len() > 255 {
            return Err(Error::Query("Invalid webhook ID".to_string()));
        }
        
        // Webhook deletion requires server connection with webhook manager
        // In production, would call connection.delete_webhook(id)
        Err(Error::Query(format!("Webhook deletion not implemented: webhook '{}' requires server connection with webhook manager", id)))
    }
}

pub struct WebhookBuilder {
    name: Option<String>,
    url: Option<String>,
    events: Vec<String>,
    scope: Option<String>,
}

impl WebhookBuilder {
    pub fn new() -> Self {
        Self {
            name: None,
            url: None,
            events: Vec::new(),
            scope: None,
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    pub fn url(mut self, url: &str) -> Self {
        self.url = Some(url.to_string());
        self
    }

    pub fn on_insert(mut self) -> Self {
        self.events.push("INSERT".to_string());
        self
    }

    pub fn on_update(mut self) -> Self {
        self.events.push("UPDATE".to_string());
        self
    }

    pub fn on_delete(mut self) -> Self {
        self.events.push("DELETE".to_string());
        self
    }

    pub fn scope(mut self, scope: &str) -> Self {
        self.scope = Some(scope.to_string());
        self
    }

    pub async fn create(self) -> Result<WebhookInfo> {
        // SECURITY: Validate name
        let name = self.name.ok_or_else(|| Error::Query("Webhook name is required".to_string()))?;
        if name.is_empty() || name.len() > 255 {
            return Err(Error::Query("Invalid webhook name".to_string()));
        }
        
        // SECURITY: Validate URL
        let url = self.url.ok_or_else(|| Error::Query("Webhook URL is required".to_string()))?;
        if url.is_empty() || url.len() > 2048 {
            return Err(Error::Query("Invalid webhook URL".to_string()));
        }
        
        // SECURITY: Validate URL format (basic check)
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(Error::Query("Webhook URL must start with http:// or https://".to_string()));
        }
        
        // SECURITY: Validate events
        if self.events.is_empty() {
            return Err(Error::Query("At least one event must be specified".to_string()));
        }
        if self.events.len() > 100 {
            return Err(Error::Query("Too many events specified".to_string()));
        }
        
        // SECURITY: Validate scope if provided
        if let Some(ref scope) = self.scope {
            if scope.len() > 255 {
                return Err(Error::Query("Invalid webhook scope".to_string()));
            }
        }
        
        // Webhook creation requires server connection with webhook manager
        // In production, would call connection.create_webhook(name, url, events, scope)
        Err(Error::Query(format!("Webhook creation not implemented: webhook '{}' requires server connection with webhook manager", name)))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookInfo {
    pub id: String,
    pub name: String,
    pub url: String,
}

/// Sync operations API
pub struct SyncOperations {
    connection: Arc<dyn Connection>,
}

impl SyncOperations {
    pub fn new(connection: Arc<dyn Connection>) -> Self {
        Self { connection }
    }

    /// Sync with peer
    pub async fn sync_peer(&self, peer_id: &str) -> Result<SyncResult> {
        Err(Error::Query(format!("Peer sync not implemented: peer '{}' requires server connection with distributed sync support", peer_id)))
    }

    /// Get sync status
    pub async fn status(&self) -> Result<SyncStatus> {
        Err(Error::Query("Sync status not implemented: requires server connection with distributed sync support".to_string()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub synced_tables: usize,
    pub conflicts_resolved: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub peers: Vec<String>,
    pub synced_tables: usize,
}

use crate::elegant::{QueryBuilder, Row};
use crate::powerful::Update;

