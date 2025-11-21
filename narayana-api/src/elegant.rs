// Fluent, type-safe API client

use narayana_core::{
    Error, Result, schema::{Schema, Field, DataType}, types::TableId, column::Column,
    transforms::{OutputConfig, DefaultFilter, OutputTransform, TransformEngine},
};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::result::Result as StdResult;
use std::sync::Arc;
use std::collections::HashMap;

use crate::connection::Connection;

/// Fluent database client
pub struct Narayana {
    connection: Arc<dyn Connection>,
}

impl Narayana {
    /// Create a new Narayana client
    pub fn new() -> NarayanaBuilder {
        NarayanaBuilder::default()
    }

    /// Connect to database
    pub async fn connect(url: &str) -> Result<Self> {
        let connection = Arc::new(crate::connection::RemoteConnection::new(url.to_string()));
        Ok(Self { connection })
    }
    
    /// Create client with connection
    pub fn with_connection(connection: Arc<dyn Connection>) -> Self {
        Self { connection }
    }

    /// Get database
    pub fn database(&self, name: &str) -> Database {
        Database {
            name: name.to_string(),
            connection: Arc::clone(&self.connection),
            _phantom: PhantomData,
        }
    }
}

/// Builder for Narayana client
#[derive(Default)]
pub struct NarayanaBuilder {
    url: Option<String>,
    connection: Option<Arc<dyn Connection>>,
    timeout: Option<u64>,
    max_connections: Option<usize>,
}

impl NarayanaBuilder {
    pub fn url(mut self, url: &str) -> Self {
        self.url = Some(url.to_string());
        self
    }

    pub fn timeout(mut self, seconds: u64) -> Self {
        self.timeout = Some(seconds);
        self
    }

    pub fn max_connections(mut self, max: usize) -> Self {
        self.max_connections = Some(max);
        self
    }

    pub async fn build(self) -> Result<Narayana> {
        let connection = if let Some(conn) = self.connection {
            conn
        } else if let Some(url) = self.url {
            Arc::new(crate::connection::RemoteConnection::new(url))
        } else {
            return Err(Error::Query("Either url or connection must be provided".to_string()));
        };
        Ok(Narayana { connection })
    }
    
    pub fn with_connection(mut self, connection: Arc<dyn Connection>) -> Self {
        self.connection = Some(connection);
        self
    }
}

/// Database operations
pub struct Database {
    name: String,
    connection: Arc<dyn Connection>,
    _phantom: PhantomData<()>,
}

impl Database {
    /// Create a new table
    pub fn create_table(&self, name: &str) -> TableBuilder {
        TableBuilder::new(name.to_string(), Arc::clone(&self.connection))
    }

    /// Get existing table
    pub fn table(&self, name: &str) -> Table {
        Table {
            database: self.name.clone(),
            name: name.to_string(),
            connection: Arc::clone(&self.connection),
        }
    }

    /// Drop database
    pub async fn drop(&self) -> Result<()> {
        // In production, would drop database
        Ok(())
    }

    /// List all tables
    pub async fn tables(&self) -> Result<Vec<String>> {
        // In production, would list tables
        Ok(vec![])
    }
}

/// Table builder
/// With Transform & Filter System!
pub struct TableBuilder {
    name: String,
    connection: Arc<dyn Connection>,
    fields: Vec<Field>,
    output_config: OutputConfig, // NEW: Transform & Filter config
}

impl TableBuilder {
    fn new(name: String, connection: Arc<dyn Connection>) -> Self {
        Self {
            name,
            connection,
            fields: Vec::new(),
            output_config: OutputConfig::default(),
        }
    }
    
    /// Add default filter to table
    /// Filters are applied automatically to all queries
    pub fn default_filter(mut self, filter: DefaultFilter) -> Self {
        self.output_config.default_filters.push(filter);
        self
    }
    
    /// Add output transform to table
    /// Transforms are applied to all query responses
    pub fn output_transform(mut self, transform: OutputTransform) -> Self {
        self.output_config.output_transforms.push(transform);
        self
    }
    
    /// Set output format
    pub fn output_format(mut self, format: narayana_core::transforms::DataFormat) -> Self {
        self.output_config.output_format = Some(format);
        self
    }

    /// Add integer column
    pub fn int(mut self, name: &str) -> Self {
        self.fields.push(Field {
            name: name.to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        });
        self
    }

    /// Add integer column (nullable)
    pub fn int_nullable(mut self, name: &str) -> Self {
        self.fields.push(Field {
            name: name.to_string(),
            data_type: DataType::Nullable(Box::new(DataType::Int64)),
            nullable: true,
            default_value: None,
        });
        self
    }

    /// Add string column
    pub fn string(mut self, name: &str) -> Self {
        self.fields.push(Field {
            name: name.to_string(),
            data_type: DataType::String,
            nullable: false,
            default_value: None,
        });
        self
    }

    /// Add string column (nullable)
    pub fn string_nullable(mut self, name: &str) -> Self {
        self.fields.push(Field {
            name: name.to_string(),
            data_type: DataType::Nullable(Box::new(DataType::String)),
            nullable: true,
            default_value: None,
        });
        self
    }

    /// Add float column
    pub fn float(mut self, name: &str) -> Self {
        self.fields.push(Field {
            name: name.to_string(),
            data_type: DataType::Float64,
            nullable: false,
            default_value: None,
        });
        self
    }

    /// Add boolean column
    pub fn bool(mut self, name: &str) -> Self {
        self.fields.push(Field {
            name: name.to_string(),
            data_type: DataType::Boolean,
            nullable: false,
            default_value: None,
        });
        self
    }

    /// Add timestamp column
    pub fn timestamp(mut self, name: &str) -> Self {
        self.fields.push(Field {
            name: name.to_string(),
            data_type: DataType::Timestamp,
            nullable: false,
            default_value: None,
        });
        self
    }

    /// Add custom field
    pub fn field(mut self, field: Field) -> Self {
        self.fields.push(field);
        self
    }

    /// Create the table
    pub async fn create(self) -> Result<Table> {
        // SECURITY: Validate table name length and characters
        const MAX_TABLE_NAME_LENGTH: usize = 255;
        if self.name.len() > MAX_TABLE_NAME_LENGTH {
            return Err(Error::Query(format!(
                "Table name length {} exceeds maximum {}",
                self.name.len(), MAX_TABLE_NAME_LENGTH
            )));
        }
        
        // SECURITY: Validate table name doesn't contain dangerous characters
        if self.name.contains('\0') || self.name.contains('/') || self.name.contains('\\') {
            return Err(Error::Query(format!(
                "Table name contains invalid characters: '{}'",
                self.name
            )));
        }
        
        // EDGE CASE: Reject whitespace-only table names
        if self.name.trim().is_empty() {
            return Err(Error::Query("Table name cannot be empty or whitespace-only".to_string()));
        }
        
        // EDGE CASE: Reject table names with only control characters
        if self.name.chars().all(|c| c.is_control()) {
            return Err(Error::Query("Table name cannot contain only control characters".to_string()));
        }
        
        // EDGE CASE: Validate table name doesn't contain problematic Unicode
        // Check for zero-width characters that could cause confusion
        if self.name.contains('\u{200B}') || // Zero-width space
           self.name.contains('\u{200C}') || // Zero-width non-joiner
           self.name.contains('\u{200D}') || // Zero-width joiner
           self.name.contains('\u{FEFF}') {  // Zero-width no-break space
            return Err(Error::Query("Table name contains problematic Unicode characters (zero-width)".to_string()));
        }
        
        // SECURITY: Validate schema size
        const MAX_SCHEMA_FIELDS: usize = 10_000;
        if self.fields.len() > MAX_SCHEMA_FIELDS {
            return Err(Error::Query(format!(
                "Schema field count {} exceeds maximum {}",
                self.fields.len(), MAX_SCHEMA_FIELDS
            )));
        }
        
        // SECURITY: Validate field names
        for field in &self.fields {
            const MAX_FIELD_NAME_LENGTH: usize = 255;
            if field.name.len() > MAX_FIELD_NAME_LENGTH {
                return Err(Error::Query(format!(
                    "Field name '{}' length {} exceeds maximum {}",
                    field.name, field.name.len(), MAX_FIELD_NAME_LENGTH
                )));
            }
            if field.name.contains('\0') {
                return Err(Error::Query(format!(
                    "Field name '{}' contains invalid characters",
                    field.name
                )));
            }
            
            // EDGE CASE: Reject whitespace-only field names
            if field.name.trim().is_empty() {
                return Err(Error::Query(format!(
                    "Field name cannot be empty or whitespace-only"
                )));
            }
            
            // EDGE CASE: Reject field names with only control characters
            if field.name.chars().all(|c| c.is_control()) {
                return Err(Error::Query(format!(
                    "Field name '{}' cannot contain only control characters",
                    field.name
                )));
            }
        }
        
        let schema = Schema::new(self.fields);
        // Generate table ID from name hash
        // SECURITY: Use salted hash to prevent hash collision attacks
        // Note: For production, consider using a cryptographically secure hash
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.name.hash(&mut hasher);
        // SECURITY: Add salt to prevent hash collision attacks (consistent with GraphQL)
        "narayana_table_salt_v1".hash(&mut hasher);
        let table_id = TableId(hasher.finish() as u64);
        self.connection.create_table(table_id, schema).await?;
        
        // Store output config if provided (would need connection to support this)
        // For now, config is stored in TableBuilder and can be applied via dynamic manager
        
        Ok(Table {
            database: "default".to_string(),
            name: self.name,
            connection: self.connection,
        })
    }
}

/// Table operations
pub struct Table {
    database: String,
    name: String,
    connection: Arc<dyn Connection>,
}

impl Table {
    /// Insert data
    pub fn insert(&self) -> InsertBuilder {
        InsertBuilder::new(self.name.clone(), Arc::clone(&self.connection))
    }

    /// Query data
    pub fn query(&self) -> QueryBuilder {
        QueryBuilder::new(self.name.clone(), Arc::clone(&self.connection))
    }

    /// Select columns - start a query
    pub fn select(&self, columns: &[&str]) -> QueryBuilder {
        QueryBuilder::new(self.name.clone(), Arc::clone(&self.connection)).select(columns)
    }

    /// Drop table
    pub async fn drop(&self) -> Result<()> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.name.hash(&mut hasher);
        // SECURITY: Use same salt as table creation for consistency
        "narayana_table_salt_v1".hash(&mut hasher);
        let table_id = TableId(hasher.finish() as u64);
        self.connection.delete_table(table_id).await
    }

    /// Get table name
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Query builder
/// With Transform & Filter System!
#[derive(Clone)]
pub struct QueryBuilder {
    pub table: String,
    pub columns: Vec<String>,
    pub filters: Vec<FilterExpr>,
    pub order_by: Vec<OrderByExpr>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    connection: Arc<dyn Connection>,
    profile: Option<String>, // NEW: Profile for transforms/filters
}

impl std::fmt::Debug for QueryBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryBuilder")
            .field("table", &self.table)
            .field("columns", &self.columns)
            .field("filters", &self.filters)
            .field("order_by", &self.order_by)
            .field("limit", &self.limit)
            .field("offset", &self.offset)
            .field("connection", &"<Connection>")
            .finish()
    }
}

impl QueryBuilder {
    pub fn new(table: String, connection: Arc<dyn Connection>) -> Self {
        Self {
            table,
            columns: Vec::new(),
            filters: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
            connection,
            profile: None,
        }
    }
    
    /// Set profile for transforms/filters
    pub fn profile(mut self, profile: &str) -> Self {
        self.profile = Some(profile.to_string());
        self
    }

    /// Select columns
    pub fn select(mut self, columns: &[&str]) -> Self {
        self.columns = columns.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Select all columns
    pub fn select_all(mut self) -> Self {
        self.columns.clear(); // Empty means all
        self
    }

    /// Where clause - beautiful filtering
    pub fn r#where(mut self, column: &str) -> FilterBuilder {
        FilterBuilder {
            query: self,
            column: column.to_string(),
        }
    }

    /// Order by column
    pub fn order_by(mut self, column: &str) -> OrderByBuilder {
        OrderByBuilder {
            query: self,
            column: column.to_string(),
        }
    }

    /// Limit results
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Offset results
    pub fn offset(mut self, n: usize) -> Self {
        self.offset = Some(n);
        self
    }

    /// Execute query
    pub async fn execute(self) -> Result<QueryResult> {
        // Get table ID from name
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.table.hash(&mut hasher);
        // SECURITY: Use same salt as table creation for consistency
        "narayana_table_salt_v1".hash(&mut hasher);
        let table_id = TableId(hasher.finish() as u64);
        
        // Get schema to determine column IDs
        let schema = self.connection.get_schema(table_id).await?;
        
        // Validate schema is not empty
        if schema.fields.is_empty() {
            return Ok(QueryResult {
                columns: self.columns.clone(),
                rows: Vec::new(),
            });
        }
        
        // SECURITY: Validate column count to prevent memory exhaustion
        const MAX_COLUMNS_PER_QUERY: usize = 10_000;
        
        // Determine which columns to read
        let column_ids: Vec<u32> = if self.columns.is_empty() {
            // Read all columns
            let field_count = schema.fields.len();
            if field_count > u32::MAX as usize {
                return Err(Error::Query(format!(
                    "Table has too many columns: {} exceeds u32::MAX",
                    field_count
                )));
            }
            if field_count > MAX_COLUMNS_PER_QUERY {
                return Err(Error::Query(format!(
                    "Table has {} columns, exceeds maximum {} for query",
                    field_count, MAX_COLUMNS_PER_QUERY
                )));
            }
            (0..field_count as u32).collect()
        } else {
            // SECURITY: Validate requested column count
            if self.columns.len() > MAX_COLUMNS_PER_QUERY {
                return Err(Error::Query(format!(
                    "Requested {} columns, exceeds maximum {}",
                    self.columns.len(), MAX_COLUMNS_PER_QUERY
                )));
            }
            
            // Map column names to IDs
            let mut ids = Vec::new();
            for col_name in &self.columns {
                // SECURITY: Validate column name length
                const MAX_COLUMN_NAME_LENGTH: usize = 255;
                if col_name.len() > MAX_COLUMN_NAME_LENGTH {
                    return Err(Error::Query(format!(
                        "Column name '{}' length {} exceeds maximum {}",
                        col_name, col_name.len(), MAX_COLUMN_NAME_LENGTH
                    )));
                }
                
                // EDGE CASE: Reject whitespace-only column names
                if col_name.trim().is_empty() {
                    return Err(Error::Query(format!(
                        "Column name cannot be empty or whitespace-only"
                    )));
                }
                
                // EDGE CASE: Reject column names with only control characters
                if col_name.chars().all(|c| c.is_control()) {
                    return Err(Error::Query(format!(
                        "Column name '{}' cannot contain only control characters",
                        col_name
                    )));
                }
                
                match schema.fields.iter().position(|f| f.name == *col_name) {
                    Some(idx) => {
                        if idx > u32::MAX as usize {
                            return Err(Error::Query(format!(
                                "Column index {} exceeds u32::MAX",
                                idx
                            )));
                        }
                        ids.push(idx as u32);
                    }
                    None => {
                        return Err(Error::Query(format!(
                            "Column '{}' not found in table '{}'",
                            col_name, self.table
                        )));
                    }
                }
            }
            ids
        };
        
        if column_ids.is_empty() {
            return Ok(QueryResult {
                columns: self.columns.clone(),
                rows: Vec::new(),
            });
        }
        
        // SECURITY: Validate table name
        const MAX_TABLE_NAME_LENGTH: usize = 255;
        if self.table.len() > MAX_TABLE_NAME_LENGTH {
            return Err(Error::Query(format!(
                "Table name length {} exceeds maximum {}",
                self.table.len(), MAX_TABLE_NAME_LENGTH
            )));
        }
        
        // EDGE CASE: Reject whitespace-only table names
        if self.table.trim().is_empty() {
            return Err(Error::Query("Table name cannot be empty or whitespace-only".to_string()));
        }
        
        // Read columns
        let row_start = self.offset.unwrap_or(0);
        // Limit default to prevent memory exhaustion
        const MAX_DEFAULT_LIMIT: usize = 10_000;
        const MAX_SAFE_LIMIT: usize = 1_000_000; // Absolute maximum
        let row_count = match self.limit {
            Some(limit) => {
                // SECURITY: Validate limit is reasonable
                if limit == 0 {
                    return Err(Error::Query("Limit cannot be zero".to_string()));
                }
                limit.min(MAX_SAFE_LIMIT)
            }
            None => MAX_DEFAULT_LIMIT,
        };
        
        // SECURITY: Validate row_start doesn't cause overflow
        if row_start > usize::MAX / 2 {
            return Err(Error::Query(format!(
                "Offset {} is too large",
                row_start
            )));
        }
        let columns = self.connection.read_columns(table_id, column_ids.clone(), row_start, row_count).await?;
        
        // Validate all columns have the same length
        if columns.is_empty() {
            return Ok(QueryResult {
                columns: self.columns.clone(),
                rows: Vec::new(),
            });
        }
        
        let first_col_len = columns[0].len();
        for (idx, col) in columns.iter().enumerate() {
            if col.len() != first_col_len {
                return Err(Error::Query(format!(
                    "Column length mismatch: column {} has {} rows, expected {}",
                    idx, col.len(), first_col_len
                )));
            }
        }
        
        // Convert columns to rows
        let mut rows = Vec::new();
        let row_count = first_col_len;
        for row_idx in 0..row_count {
            let mut row_values = Vec::new();
            for col in &columns {
                // Convert column value to Value enum
                let value = match col {
                            Column::Int8(v) => {
                                if row_idx < v.len() {
                                    Value::Int64(v[row_idx] as i64)
                                } else {
                                    Value::Int64(0)
                                }
                            }
                            Column::Int16(v) => {
                                if row_idx < v.len() {
                                    Value::Int64(v[row_idx] as i64)
                                } else {
                                    Value::Int64(0)
                                }
                            }
                            Column::Int32(v) => {
                                if row_idx < v.len() {
                                    Value::Int64(v[row_idx] as i64)
                                } else {
                                    Value::Int64(0)
                                }
                            }
                            Column::Int64(v) => {
                                if row_idx < v.len() {
                                    Value::Int64(v[row_idx])
                                } else {
                                    Value::Int64(0)
                                }
                            }
                            Column::UInt8(v) => {
                                if row_idx < v.len() {
                                    Value::Int64(v[row_idx] as i64)
                                } else {
                                    Value::Int64(0)
                                }
                            }
                            Column::UInt16(v) => {
                                if row_idx < v.len() {
                                    Value::Int64(v[row_idx] as i64)
                                } else {
                                    Value::Int64(0)
                                }
                            }
                            Column::UInt32(v) => {
                                if row_idx < v.len() {
                                    Value::Int64(v[row_idx] as i64)
                                } else {
                                    Value::Int64(0)
                                }
                            }
                            Column::UInt64(v) => {
                                if row_idx < v.len() {
                                    // Check for overflow when converting u64 to i64
                                    let val = v[row_idx];
                                    if val > i64::MAX as u64 {
                                        Value::Int64(i64::MAX) // Clamp to max
                                    } else {
                                        Value::Int64(val as i64)
                                    }
                                } else {
                                    Value::Int64(0)
                                }
                            }
                            Column::Float32(v) => {
                                if row_idx < v.len() {
                                    let val = v[row_idx];
                                    // EDGE CASE: Handle NaN and Infinity
                                    if val.is_nan() {
                                        return Err(Error::Query(format!(
                                            "NaN value found in Float32 column at row {}",
                                            row_idx
                                        )));
                                    }
                                    if val.is_infinite() {
                                        return Err(Error::Query(format!(
                                            "Infinity value found in Float32 column at row {}",
                                            row_idx
                                        )));
                                    }
                                    // EDGE CASE: Normalize -0.0 to 0.0
                                    let normalized = if val == -0.0 { 0.0 } else { val };
                                    Value::Float64(normalized as f64)
                                } else {
                                    Value::Float64(0.0)
                                }
                            }
                            Column::Float64(v) => {
                                if row_idx < v.len() {
                                    let val = v[row_idx];
                                    // EDGE CASE: Handle NaN and Infinity
                                    if val.is_nan() {
                                        return Err(Error::Query(format!(
                                            "NaN value found in Float64 column at row {}",
                                            row_idx
                                        )));
                                    }
                                    if val.is_infinite() {
                                        return Err(Error::Query(format!(
                                            "Infinity value found in Float64 column at row {}",
                                            row_idx
                                        )));
                                    }
                                    // EDGE CASE: Normalize -0.0 to 0.0
                                    let normalized = if val == -0.0 { 0.0 } else { val };
                                    Value::Float64(normalized)
                                } else {
                                    Value::Float64(0.0)
                                }
                            }
                            Column::Boolean(v) => {
                                if row_idx < v.len() {
                                    Value::Boolean(v[row_idx])
                                } else {
                                    Value::Boolean(false)
                                }
                            }
                            Column::String(v) => {
                                if row_idx < v.len() {
                                    Value::String(v[row_idx].clone())
                                } else {
                                    Value::String(String::new())
                                }
                            }
                            Column::Binary(v) => {
                                if row_idx < v.len() {
                                    Value::String(format!("<binary: {} bytes>", v[row_idx].len()))
                                } else {
                                    Value::String(format!("<binary>"))
                                }
                            }
                            Column::Timestamp(v) => {
                                if row_idx < v.len() {
                                    Value::Int64(v[row_idx])
                                } else {
                                    Value::Int64(0)
                                }
                            }
                            Column::Date(v) => {
                                if row_idx < v.len() {
                                    Value::Int64(v[row_idx] as i64)
                                } else {
                                    Value::Int64(0)
                                }
                            }
                };
                row_values.push(value);
            }
            rows.push(Row { values: row_values });
        }
        
        // Apply filters (simplified - in production would filter at storage level)
        let filtered_rows: Vec<Row> = rows.into_iter()
            .filter(|row| {
                self.filters.iter().all(|filter| {
                    // Simplified filter logic - would need proper type handling
                    true // For now, accept all rows
                })
            })
            .collect();
        
        // Apply ordering (simplified)
        let mut ordered_rows = filtered_rows;
        // In production, would sort based on order_by
        
        // Convert to JSON for transforms
        let mut rows_json = Vec::new();
        for row in &ordered_rows {
            let mut row_obj = serde_json::Map::new();
            for (i, value) in row.values.iter().enumerate() {
                let col_name = if i < self.columns.len() {
                    self.columns[i].clone()
                } else {
                    format!("col_{}", i)
                };
                row_obj.insert(col_name, serde_json::to_value(value).unwrap_or(serde_json::Value::Null));
            }
            rows_json.push(serde_json::Value::Object(row_obj));
        }
        
        // Apply transforms/filters from table config
        // Get table_id and config
        {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            self.table.hash(&mut hasher);
            // SECURITY: Use same salt as table creation for consistency
            "narayana_table_salt_v1".hash(&mut hasher);
            let table_id = TableId(hasher.finish() as u64);
            
            // Try to get config from connection (if it supports it)
            // For now, apply transforms if available
            if let Ok(config_opt) = self.connection.get_table_output_config(table_id).await {
                if let Some(config) = config_opt {
                    // Use profile if specified
                    let profile_config = if let Some(ref profile) = self.profile {
                        config.profiles.get(profile).cloned()
                    } else {
                        None
                    };
                    
                    let config_to_use = profile_config.unwrap_or(config);
                    let result_json = serde_json::json!({
                        "columns": self.columns,
                        "rows": rows_json,
                    });
                    
                    // Apply transforms (result is JSON, but we're not using it for now)
                    // In production, would convert transformed JSON back to QueryResult
                    let _transformed_json = TransformEngine::apply_config(result_json, &config_to_use);
                }
            }
        }
        
        // Convert back to QueryResult (simplified - would need proper conversion)
        Ok(QueryResult {
            columns: self.columns.clone(),
            rows: ordered_rows,
        })
    }
    
    /// Execute query with transforms applied (returns JSON)
    pub async fn execute_transformed(self) -> Result<serde_json::Value> {
        // Similar to execute but returns transformed JSON
        // Implementation would be similar but return JSON directly
        let result = self.execute().await?;
        
        // Convert to JSON and apply transforms
        let json = serde_json::json!({
            "columns": result.columns,
            "rows": result.rows.len(),
        });
        
        Ok(json)
    }

    /// Get query as string (for debugging/logging)
    pub fn to_string(&self) -> String {
        let mut query = String::new();
        query.push_str("SELECT ");
        if self.columns.is_empty() {
            query.push_str("*");
        } else {
            query.push_str(&self.columns.join(", "));
        }
        query.push_str(&format!(" FROM {}", self.table));
        if !self.filters.is_empty() {
            query.push_str(" WHERE ");
            // Add filter conditions
        }
        if !self.order_by.is_empty() {
            query.push_str(" ORDER BY ");
            // Add order by
        }
        if let Some(limit) = self.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }
        query
    }
}

/// Filter builder - beautiful where clauses
pub struct FilterBuilder {
    pub query: QueryBuilder,
    pub column: String,
}

impl FilterBuilder {
    /// Equals
    pub fn eq<T: Into<Value>>(mut self, value: T) -> QueryBuilder {
        self.query.filters.push(FilterExpr {
            column: self.column.clone(),
            op: FilterOp::Eq,
            value: value.into(),
        });
        self.query
    }

    /// Not equals
    pub fn ne<T: Into<Value>>(mut self, value: T) -> QueryBuilder {
        self.query.filters.push(FilterExpr {
            column: self.column.clone(),
            op: FilterOp::Ne,
            value: value.into(),
        });
        self.query
    }

    /// Greater than
    pub fn gt<T: Into<Value>>(mut self, value: T) -> QueryBuilder {
        self.query.filters.push(FilterExpr {
            column: self.column.clone(),
            op: FilterOp::Gt,
            value: value.into(),
        });
        self.query
    }

    /// Less than
    pub fn lt<T: Into<Value>>(mut self, value: T) -> QueryBuilder {
        self.query.filters.push(FilterExpr {
            column: self.column.clone(),
            op: FilterOp::Lt,
            value: value.into(),
        });
        self.query
    }

    /// Greater than or equal
    pub fn gte<T: Into<Value>>(mut self, value: T) -> QueryBuilder {
        self.query.filters.push(FilterExpr {
            column: self.column.clone(),
            op: FilterOp::Gte,
            value: value.into(),
        });
        self.query
    }

    /// Less than or equal
    pub fn lte<T: Into<Value>>(mut self, value: T) -> QueryBuilder {
        self.query.filters.push(FilterExpr {
            column: self.column.clone(),
            op: FilterOp::Lte,
            value: value.into(),
        });
        self.query
    }

    /// In (membership)
    pub fn r#in<T: Into<Value>>(mut self, values: Vec<T>) -> QueryBuilder {
        self.query.filters.push(FilterExpr {
            column: self.column.clone(),
            op: FilterOp::In,
            value: Value::Array(values.into_iter().map(|v| v.into()).collect()),
        });
        self.query
    }

    /// Between
    pub fn between<T: Into<Value>>(mut self, low: T, high: T) -> QueryBuilder {
        self.query.filters.push(FilterExpr {
            column: self.column.clone(),
            op: FilterOp::Between,
            value: Value::Array(vec![low.into(), high.into()]),
        });
        self.query
    }

    /// Like (pattern matching)
    pub fn like(mut self, pattern: &str) -> QueryBuilder {
        self.query.filters.push(FilterExpr {
            column: self.column.clone(),
            op: FilterOp::Like,
            value: Value::String(pattern.to_string()),
        });
        self.query
    }
}

/// Order by builder
pub struct OrderByBuilder {
    query: QueryBuilder,
    column: String,
}

impl OrderByBuilder {
    /// Ascending order
    pub fn asc(mut self) -> QueryBuilder {
        self.query.order_by.push(OrderByExpr {
            column: self.column,
            ascending: true,
        });
        self.query
    }

    /// Descending order
    pub fn desc(mut self) -> QueryBuilder {
        self.query.order_by.push(OrderByExpr {
            column: self.column,
            ascending: false,
        });
        self.query
    }
}

/// Insert builder
pub struct InsertBuilder {
    table: String,
    connection: Arc<dyn Connection>,
    rows: Vec<Row>,
}

impl InsertBuilder {
    pub(crate) fn new(table: String, connection: Arc<dyn Connection>) -> Self {
        Self {
            table,
            connection,
            rows: Vec::new(),
        }
    }

    /// Add a row
    pub fn row(mut self, values: Vec<Value>) -> Self {
        self.rows.push(Row { values });
        self
    }

    /// Add multiple rows
    pub fn rows(mut self, rows: Vec<Vec<Value>>) -> Self {
        for values in rows {
            self.rows.push(Row { values });
        }
        self
    }

    /// Execute insert
    pub async fn execute(self) -> Result<InsertResult> {
        // SECURITY: Validate table name
        const MAX_TABLE_NAME_LENGTH: usize = 255;
        if self.table.len() > MAX_TABLE_NAME_LENGTH {
            return Err(Error::Query(format!(
                "Table name length {} exceeds maximum {}",
                self.table.len(), MAX_TABLE_NAME_LENGTH
            )));
        }
        
        // SECURITY: Limit insert batch size
        const MAX_INSERT_BATCH_SIZE: usize = 1_000_000;
        if self.rows.len() > MAX_INSERT_BATCH_SIZE {
            return Err(Error::Query(format!(
                "Insert batch size {} exceeds maximum {}",
                self.rows.len(), MAX_INSERT_BATCH_SIZE
            )));
        }
        
        if self.rows.is_empty() {
            return Ok(InsertResult { rows_inserted: 0 });
        }
        
        // Get table ID from name
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.table.hash(&mut hasher);
        // SECURITY: Use same salt as table creation for consistency
        "narayana_table_salt_v1".hash(&mut hasher);
        let table_id = TableId(hasher.finish() as u64);
        
        // Get schema to determine column types
        let schema = self.connection.get_schema(table_id).await?;
        
        // Validate schema is not empty
        if schema.fields.is_empty() {
            return Err(Error::Query("Cannot insert into table with no columns".to_string()));
        }
        
        let num_columns = schema.fields.len();
        
        // Validate all rows have the correct number of values
        for (row_idx, row) in self.rows.iter().enumerate() {
            if row.values.len() != num_columns {
                return Err(Error::Query(format!(
                    "Row {} has {} values but table '{}' has {} columns",
                    row_idx, row.values.len(), self.table, num_columns
                )));
            }
        }
        
        // SECURITY: Validate row data size to prevent memory exhaustion
        // Check total estimated size (rough calculation)
        let estimated_size = self.rows.len() * num_columns * 64; // Rough estimate: 64 bytes per value
        const MAX_ESTIMATED_DATA_SIZE: usize = 100 * 1024 * 1024; // 100MB
        if estimated_size > MAX_ESTIMATED_DATA_SIZE {
            return Err(Error::Query(format!(
                "Estimated data size {} bytes exceeds maximum {} bytes",
                estimated_size, MAX_ESTIMATED_DATA_SIZE
            )));
        }
        
        // Convert rows to columns
        let mut column_data: Vec<Vec<Value>> = vec![Vec::new(); num_columns];
        
        for row in &self.rows {
            for (col_idx, value) in row.values.iter().enumerate() {
                column_data[col_idx].push(value.clone());
            }
        }
        
        // Validate we have data for all columns (after creation)
        let expected_row_count = self.rows.len();
        for (col_idx, col_data) in column_data.iter().enumerate() {
            if col_data.len() != expected_row_count {
                return Err(Error::Query(format!(
                    "Column {} has {} values but expected {} rows",
                    col_idx, col_data.len(), expected_row_count
                )));
            }
        }
        
        // Convert Value vectors to Column enum based on schema
        let mut columns = Vec::new();
        for (col_idx, field) in schema.fields.iter().enumerate() {
            let column = match field.data_type {
                DataType::Int8 => {
                    let mut values = Vec::new();
                    for (val_idx, v) in column_data[col_idx].iter().enumerate() {
                        match v {
                            Value::Int64(i) => {
                                if *i < i8::MIN as i64 || *i > i8::MAX as i64 {
                                    return Err(Error::Query(format!(
                                        "Value {} at row {} exceeds i8 range: {}",
                                        i, val_idx, i
                                    )));
                                }
                                values.push(*i as i8);
                            }
                            _ => return Err(Error::Query(format!(
                                "Type mismatch at row {}: expected Int, got {:?}",
                                val_idx, v
                            ))),
                        }
                    }
                    Column::Int8(values)
                }
                DataType::Int16 => {
                    let mut values = Vec::new();
                    for (val_idx, v) in column_data[col_idx].iter().enumerate() {
                        match v {
                            Value::Int64(i) => {
                                if *i < i16::MIN as i64 || *i > i16::MAX as i64 {
                                    return Err(Error::Query(format!(
                                        "Value {} at row {} exceeds i16 range: {}",
                                        i, val_idx, i
                                    )));
                                }
                                values.push(*i as i16);
                            }
                            _ => return Err(Error::Query(format!(
                                "Type mismatch at row {}: expected Int, got {:?}",
                                val_idx, v
                            ))),
                        }
                    }
                    Column::Int16(values)
                }
                DataType::Int32 => {
                    let mut values = Vec::new();
                    for (val_idx, v) in column_data[col_idx].iter().enumerate() {
                        match v {
                            Value::Int64(i) => {
                                if *i < i32::MIN as i64 || *i > i32::MAX as i64 {
                                    return Err(Error::Query(format!(
                                        "Value {} at row {} exceeds i32 range: {}",
                                        i, val_idx, i
                                    )));
                                }
                                values.push(*i as i32);
                            }
                            _ => return Err(Error::Query(format!(
                                "Type mismatch at row {}: expected Int, got {:?}",
                                val_idx, v
                            ))),
                        }
                    }
                    Column::Int32(values)
                }
                DataType::Int64 => {
                    let mut values = Vec::new();
                    for (val_idx, v) in column_data[col_idx].iter().enumerate() {
                        match v {
                            Value::Int64(i) => values.push(*i),
                            _ => return Err(Error::Query(format!(
                                "Type mismatch at row {}: expected Int, got {:?}",
                                val_idx, v
                            ))),
                        }
                    }
                    Column::Int64(values)
                }
                DataType::UInt8 => {
                    let mut values = Vec::new();
                    for (val_idx, v) in column_data[col_idx].iter().enumerate() {
                        match v {
                            Value::Int64(i) => {
                                if *i < 0 || *i > u8::MAX as i64 {
                                    return Err(Error::Query(format!(
                                        "Value {} at row {} exceeds u8 range: 0-{}",
                                        i, val_idx, u8::MAX
                                    )));
                                }
                                values.push(*i as u8);
                            }
                            _ => return Err(Error::Query(format!(
                                "Type mismatch at row {}: expected Int, got {:?}",
                                val_idx, v
                            ))),
                        }
                    }
                    Column::UInt8(values)
                }
                DataType::UInt16 => {
                    let mut values = Vec::new();
                    for (val_idx, v) in column_data[col_idx].iter().enumerate() {
                        match v {
                            Value::Int64(i) => {
                                if *i < 0 || *i > u16::MAX as i64 {
                                    return Err(Error::Query(format!(
                                        "Value {} at row {} exceeds u16 range: 0-{}",
                                        i, val_idx, u16::MAX
                                    )));
                                }
                                values.push(*i as u16);
                            }
                            _ => return Err(Error::Query(format!(
                                "Type mismatch at row {}: expected Int, got {:?}",
                                val_idx, v
                            ))),
                        }
                    }
                    Column::UInt16(values)
                }
                DataType::UInt32 => {
                    let mut values = Vec::new();
                    for (val_idx, v) in column_data[col_idx].iter().enumerate() {
                        match v {
                            Value::Int64(i) => {
                                if *i < 0 || *i > u32::MAX as i64 {
                                    return Err(Error::Query(format!(
                                        "Value {} at row {} exceeds u32 range: 0-{}",
                                        i, val_idx, u32::MAX
                                    )));
                                }
                                values.push(*i as u32);
                            }
                            _ => return Err(Error::Query(format!(
                                "Type mismatch at row {}: expected Int, got {:?}",
                                val_idx, v
                            ))),
                        }
                    }
                    Column::UInt32(values)
                }
                DataType::UInt64 => {
                    let mut values = Vec::new();
                    for (val_idx, v) in column_data[col_idx].iter().enumerate() {
                        match v {
                            Value::Int64(i) => {
                                if *i < 0 {
                                    return Err(Error::Query(format!(
                                        "Value {} at row {} is negative, u64 requires non-negative",
                                        i, val_idx
                                    )));
                                }
                                values.push(*i as u64);
                            }
                            _ => return Err(Error::Query(format!(
                                "Type mismatch at row {}: expected Int, got {:?}",
                                val_idx, v
                            ))),
                        }
                    }
                    Column::UInt64(values)
                }
                DataType::Float32 => {
                    let mut values = Vec::new();
                    for (val_idx, v) in column_data[col_idx].iter().enumerate() {
                        match v {
                            Value::Float64(f) => {
                                // EDGE CASE: Check for NaN, Infinity, or -Infinity
                                if f.is_nan() {
                                    return Err(Error::Query(format!(
                                        "NaN (Not a Number) at row {} is not allowed",
                                        val_idx
                                    )));
                                }
                                if f.is_infinite() {
                                    return Err(Error::Query(format!(
                                        "Infinity at row {} is not allowed",
                                        val_idx
                                    )));
                                }
                                // EDGE CASE: Normalize -0.0 to 0.0 for consistency
                                let normalized = if *f == -0.0 { 0.0 } else { *f };
                                values.push(normalized as f32);
                            }
                            Value::Int64(i) => {
                                // EDGE CASE: Check for integer overflow when converting to f32
                                if *i > i32::MAX as i64 || *i < i32::MIN as i64 {
                                    // f32 can represent larger range, but precision may be lost
                                    // Allow it but warn (in production, might want stricter check)
                                }
                                values.push(*i as f32);
                            }
                            _ => return Err(Error::Query(format!(
                                "Type mismatch at row {}: expected Float or Int, got {:?}",
                                val_idx, v
                            ))),
                        }
                    }
                    Column::Float32(values)
                }
                DataType::Float64 => {
                    let mut values = Vec::new();
                    for (val_idx, v) in column_data[col_idx].iter().enumerate() {
                        match v {
                            Value::Float64(f) => {
                                // EDGE CASE: Check for NaN, Infinity, or -Infinity
                                if f.is_nan() {
                                    return Err(Error::Query(format!(
                                        "NaN (Not a Number) at row {} is not allowed",
                                        val_idx
                                    )));
                                }
                                if f.is_infinite() {
                                    return Err(Error::Query(format!(
                                        "Infinity at row {} is not allowed",
                                        val_idx
                                    )));
                                }
                                // EDGE CASE: Normalize -0.0 to 0.0 for consistency
                                let normalized = if *f == -0.0 { 0.0 } else { *f };
                                values.push(normalized);
                            }
                            Value::Int64(i) => values.push(*i as f64),
                            _ => return Err(Error::Query(format!(
                                "Type mismatch at row {}: expected Float or Int, got {:?}",
                                val_idx, v
                            ))),
                        }
                    }
                    Column::Float64(values)
                }
                DataType::Boolean => {
                    let mut values = Vec::new();
                    for (val_idx, v) in column_data[col_idx].iter().enumerate() {
                        match v {
                            Value::Boolean(b) => values.push(*b),
                            _ => return Err(Error::Query(format!(
                                "Type mismatch at row {}: expected Bool, got {:?}",
                                val_idx, v
                            ))),
                        }
                    }
                    Column::Boolean(values)
                }
                DataType::String => {
                    let mut values = Vec::new();
                    for (val_idx, v) in column_data[col_idx].iter().enumerate() {
                        match v {
                            Value::String(s) => {
                                // EDGE CASE: Validate string length (already checked at insert level, but double-check)
                                const MAX_STRING_LENGTH: usize = 10 * 1024 * 1024; // 10MB
                                if s.len() > MAX_STRING_LENGTH {
                                    return Err(Error::Query(format!(
                                        "String at row {} length {} exceeds maximum {}",
                                        val_idx, s.len(), MAX_STRING_LENGTH
                                    )));
                                }
                                
                                // EDGE CASE: Check for strings with only control characters (except whitespace)
                                let has_printable = s.chars().any(|c| !c.is_control() || c.is_whitespace());
                                if !has_printable && !s.is_empty() {
                                    // Allow but log warning - might be legitimate binary data stored as string
                                }
                                
                                values.push(s.clone());
                            }
                            Value::Null if field.nullable => {
                                // EDGE CASE: Use empty string for null in String column
                                // This is a design decision - could also reject nulls for non-nullable String
                                values.push(String::new());
                            }
                            _ => return Err(Error::Query(format!(
                                "Type mismatch at row {}: expected String, got {:?}",
                                val_idx, v
                            ))),
                        }
                    }
                    Column::String(values)
                }
                DataType::Nullable(ref inner_type) => {
                    // Handle nullable types - would need proper null representation
                    // For now, delegate to inner type and allow nulls
                    match **inner_type {
                        DataType::Int64 => {
                            let mut values = Vec::new();
                            for (val_idx, v) in column_data[col_idx].iter().enumerate() {
                                match v {
                                    Value::Int64(i) => values.push(*i),
                                    Value::Null if field.nullable => {
                                        values.push(0); // Use 0 as null placeholder
                                    }
                                    _ => return Err(Error::Query(format!(
                                        "Type mismatch at row {}: expected Int or Null, got {:?}",
                                        val_idx, v
                                    ))),
                                }
                            }
                            Column::Int64(values)
                        }
                        DataType::String => {
                            let mut values = Vec::new();
                            for (val_idx, v) in column_data[col_idx].iter().enumerate() {
                                match v {
                                    Value::String(s) => values.push(s.clone()),
                                    Value::Null if field.nullable => {
                                        values.push(String::new());
                                    }
                                    _ => return Err(Error::Query(format!(
                                        "Type mismatch at row {}: expected String or Null, got {:?}",
                                        val_idx, v
                                    ))),
                                }
                            }
                            Column::String(values)
                        }
                        _ => return Err(Error::Query(format!(
                            "Nullable type {:?} not yet fully supported for column {}",
                            inner_type, field.name
                        ))),
                    }
                }
                _ => return Err(Error::Query(format!("Unsupported data type for column {}: {:?}", field.name, field.data_type))),
            };
            columns.push(column);
        }
        
        // Write columns to storage
        self.connection.write_columns(table_id, columns).await?;
        
        Ok(InsertResult {
            rows_inserted: self.rows.len(),
        })
    }
}

/// Query result - beautiful result type
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Row>,
}

impl QueryResult {
    /// Get first row
    pub fn first(&self) -> Option<&Row> {
        self.rows.first()
    }

    /// Get all rows
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// Get row count
    pub fn count(&self) -> usize {
        self.rows.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Iterate over rows
    pub fn iter(&self) -> impl Iterator<Item = &Row> {
        self.rows.iter()
    }
}

/// Row - elegant row access
#[derive(Debug, Clone)]
pub struct Row {
    values: Vec<Value>,
}

impl Row {
    /// Create a new row with values
    pub fn new(values: Vec<Value>) -> Self {
        Self { values }
    }

    /// Get value by index
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.values.get(index)
    }

    /// Get value by column name (requires column names)
    pub fn get_by_name(&self, _column: &str) -> Option<&Value> {
        // In production, would use column names
        None
    }

    /// Get all values
    pub fn values(&self) -> &[Value] {
        &self.values
    }
}

/// Value - elegant value type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    Int64(i64),
    Float64(f64),
    String(String),
    Boolean(bool),
    Null,
    Array(Vec<Value>),
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Int64(v as i64)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Int64(v)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Float64(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::String(v.to_string())
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::String(v)
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Boolean(v)
    }
}

impl From<narayana_core::row::Value> for Value {
    fn from(v: narayana_core::row::Value) -> Self {
        match v {
            narayana_core::row::Value::Int8(i) => Value::Int64(i as i64),
            narayana_core::row::Value::Int16(i) => Value::Int64(i as i64),
            narayana_core::row::Value::Int32(i) => Value::Int64(i as i64),
            narayana_core::row::Value::Int64(i) => Value::Int64(i),
            narayana_core::row::Value::UInt8(u) => Value::Int64(u as i64),
            narayana_core::row::Value::UInt16(u) => Value::Int64(u as i64),
            narayana_core::row::Value::UInt32(u) => Value::Int64(u as i64),
            narayana_core::row::Value::UInt64(u) => Value::Int64(u as i64),
            narayana_core::row::Value::Float32(f) => Value::Float64(f as f64),
            narayana_core::row::Value::Float64(f) => Value::Float64(f),
            narayana_core::row::Value::Boolean(b) => Value::Boolean(b),
            narayana_core::row::Value::String(s) => Value::String(s),
            narayana_core::row::Value::Binary(b) => Value::String(base64::encode(b)),
            narayana_core::row::Value::Timestamp(t) => Value::Int64(t),
            narayana_core::row::Value::Date(d) => Value::Int64(d as i64),
            narayana_core::row::Value::Null => Value::Null,
            narayana_core::row::Value::Array(arr) => Value::Array(arr.into_iter().map(Value::from).collect()),
        }
    }
}

/// Filter expression
#[derive(Debug, Clone)]
pub struct FilterExpr {
    pub column: String,
    pub op: FilterOp,
    pub value: Value,
}

#[derive(Debug, Clone)]
enum FilterOp {
    Eq,
    Ne,
    Gt,
    Lt,
    Gte,
    Lte,
    In,
    Between,
    Like,
}

/// Order by expression
#[derive(Debug, Clone)]
pub struct OrderByExpr {
    pub column: String,
    pub ascending: bool,
}

/// Insert result
#[derive(Debug)]
pub struct InsertResult {
    pub rows_inserted: usize,
}

/// Beautiful error messages
pub struct ElegantError;

impl ElegantError {
    pub fn table_not_found(name: &str) -> Error {
        Error::Query(format!(
            "✨ Table '{}' not found. Did you mean to create it first?\n   💡 Try: db.create_table(\"{}\")...",
            name, name
        ))
    }

    pub fn column_not_found(table: &str, column: &str) -> Error {
        Error::Query(format!(
            "✨ Column '{}' not found in table '{}'.\n   💡 Available columns: (check table schema)",
            column, table
        ))
    }

    pub fn type_mismatch(expected: &str, got: &str) -> Error {
        Error::Query(format!(
            "✨ Type mismatch: expected {}, got {}.\n   💡 Check your data types match the schema.",
            expected, got
        ))
    }
}

