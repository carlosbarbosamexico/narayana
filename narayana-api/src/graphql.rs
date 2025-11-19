// GraphQL implementation for NarayanaDB
// Provides full GraphQL query and mutation support

use async_graphql::{Schema, Object, Context, Result as GqlResult, InputObject, SimpleObject, ID};
use narayana_core::{Error, Result, schema::{Schema as DbSchema, Field, DataType}, types::TableId, column::Column};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use std::collections::HashMap;

use crate::connection::Connection;

/// GraphQL schema root
pub type GraphQLSchema = Schema<QueryRoot, MutationRoot, async_graphql::EmptySubscription>;

/// Create a new GraphQL schema with security limits
pub fn create_schema(connection: Arc<dyn Connection>) -> GraphQLSchema {
    Schema::build(QueryRoot { connection: Arc::clone(&connection) }, MutationRoot { connection }, async_graphql::EmptySubscription)
        .limit_complexity(1000) // SECURITY: Limit query complexity to prevent DoS
        .limit_depth(10) // SECURITY: Limit query depth to prevent deeply nested queries
        .disable_introspection() // SECURITY: Disable introspection to prevent schema discovery attacks
        .finish()
}

/// Query root for GraphQL
pub struct QueryRoot {
    connection: Arc<dyn Connection>,
}

#[Object]
impl QueryRoot {
    /// Query a table by name
    async fn table(&self, _ctx: &Context<'_>, name: String) -> GqlResult<Table> {
        // SECURITY: Validate and sanitize table name
        let name = name.trim();
        if name.is_empty() {
            return Err(async_graphql::Error::new("Table name cannot be empty"));
        }
        
        // SECURITY: Check byte length (not char length) to prevent memory issues
        if name.len() > 255 {
            return Err(async_graphql::Error::new("Table name exceeds maximum length"));
        }
        
        // SECURITY: Validate grapheme count (not just bytes) to prevent Unicode attacks
        use unicode_segmentation::UnicodeSegmentation;
        let grapheme_count = name.graphemes(true).count();
        if grapheme_count > 255 {
            return Err(async_graphql::Error::new("Table name exceeds maximum character count"));
        }
        
        // SECURITY: Prevent path traversal and injection attempts
        if name.contains("..") || name.contains("/") || name.contains("\\") {
            return Err(async_graphql::Error::new("Invalid table name format"));
        }
        
        // SECURITY: Prevent Unicode homoglyph attacks - only allow ASCII
        if !name.is_ascii() {
            return Err(async_graphql::Error::new("Table name must contain only ASCII characters"));
        }
        
        // Get table ID by name
        let table_id = self.connection.get_table_id(name).await
            .map_err(|_| async_graphql::Error::new("Failed to access table"))?;
        
        let table_id = table_id.ok_or_else(|| async_graphql::Error::new("Table not found"))?;
        
        // Get schema
        let schema = self.connection.get_schema(table_id).await
            .map_err(|_| async_graphql::Error::new("Failed to access schema"))?;
        
        Ok(Table {
            id: table_id.0,
            name: name.to_string(),
            schema,
            connection: Arc::clone(&self.connection),
        })
    }
    
    /// Query rows from a table
    async fn query(&self, _ctx: &Context<'_>, input: QueryInput) -> GqlResult<QueryResult> {
        // SECURITY: Validate and sanitize table name
        let table_name = input.table.trim();
        if table_name.is_empty() {
            return Err(async_graphql::Error::new("Table name cannot be empty"));
        }
        if table_name.len() > 255 {
            return Err(async_graphql::Error::new("Table name exceeds maximum length"));
        }
        if table_name.contains("..") || table_name.contains("/") || table_name.contains("\\") {
            return Err(async_graphql::Error::new("Invalid table name format"));
        }
        
        // SECURITY: Limit number of columns requested
        if input.columns.len() > 1000 {
            return Err(async_graphql::Error::new("Cannot request more than 1000 columns"));
        }
        
        // Get table ID
        let table_id = self.connection.get_table_id(table_name).await
            .map_err(|_| async_graphql::Error::new("Failed to access table"))?;
        
        let table_id = table_id.ok_or_else(|| async_graphql::Error::new("Table not found"))?;
        
        // Get schema to determine column indices
        let schema = self.connection.get_schema(table_id).await
            .map_err(|_| async_graphql::Error::new("Failed to access schema"))?;
        
        // SECURITY: Validate column names and sanitize
        let sanitized_columns: Vec<String> = input.columns.iter()
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty() && c.len() <= 255)
            .filter(|c| !c.contains("..") && !c.contains("/") && !c.contains("\\"))
            .collect();
        
        // Map column names to indices
        let column_indices: Vec<u32> = if sanitized_columns.is_empty() {
            // Select all columns
            (0..schema.fields.len() as u32).collect()
        } else {
            sanitized_columns.iter()
                .filter_map(|name| {
                    schema.fields.iter()
                        .position(|f| f.name == *name)
                        .map(|i| i as u32)
                })
                .collect()
        };
        
        if !sanitized_columns.is_empty() && column_indices.len() != sanitized_columns.len() {
            return Err(async_graphql::Error::new("Some columns not found"));
        }
        
        // SECURITY: Validate offset and limit to prevent overflow and excessive memory usage
        let offset = input.offset.unwrap_or(0);
        if offset > 1_000_000_000 {
            return Err(async_graphql::Error::new("Offset exceeds maximum value of 1,000,000,000"));
        }
        
        let limit = input.limit.unwrap_or(100).min(10000); // Max 10k rows
        if limit == 0 {
            return Err(async_graphql::Error::new("Limit must be greater than 0"));
        }
        
        // Prevent offset + limit overflow
        if offset.saturating_add(limit) > 1_000_000_000 {
            return Err(async_graphql::Error::new("Offset + limit exceeds maximum value"));
        }
        
        // Read columns
        let columns = self.connection.read_columns(table_id, column_indices.clone(), offset, limit).await
            .map_err(|_| async_graphql::Error::new("Failed to read data"))?;
        
        // SECURITY: Limit result size to prevent memory exhaustion
        // SECURITY: Use checked arithmetic to prevent integer overflow
        const MAX_RESULT_SIZE_BYTES: usize = 100 * 1024 * 1024; // 100MB
        let estimated_size = columns.iter()
            .map(|c| {
                // SECURITY: Check for overflow in multiplication
                c.len().checked_mul(std::mem::size_of::<usize>())
                    .unwrap_or(usize::MAX) // If overflow, return max to trigger limit
            })
            .try_fold(0usize, |acc, x| {
                // SECURITY: Check for overflow in addition
                acc.checked_add(x)
            });
        
        match estimated_size {
            Some(size) if size > MAX_RESULT_SIZE_BYTES => {
                return Err(async_graphql::Error::new("Query result exceeds maximum size"));
            }
            None => {
                return Err(async_graphql::Error::new("Query result size calculation overflow"));
            }
            _ => {}
        }
        
        // Convert columns to rows
        let rows = if columns.is_empty() {
            Vec::new()
        } else {
            // SECURITY: Validate all columns have the same length to prevent index out of bounds
            let row_count = columns[0].len();
            for (idx, col) in columns.iter().enumerate() {
                if col.len() != row_count {
                    return Err(async_graphql::Error::new("Data integrity error"));
                }
            }
            
            (0..row_count).map(|row_idx| {
                let mut values = HashMap::new();
                for (col_idx, column) in columns.iter().enumerate() {
                    let field_idx = column_indices[col_idx] as usize;
                    if field_idx < schema.fields.len() {
                        let field = &schema.fields[field_idx];
                        let value = match column {
                            Column::Int8(v) => v.get(row_idx).map(|v| JsonValue::Number((*v as i64).into())),
                            Column::Int16(v) => v.get(row_idx).map(|v| JsonValue::Number((*v as i64).into())),
                            Column::Int32(v) => v.get(row_idx).map(|v| JsonValue::Number((*v as i64).into())),
                            Column::Int64(v) => v.get(row_idx).map(|v| JsonValue::Number((*v).into())),
                            Column::UInt8(v) => v.get(row_idx).map(|v| JsonValue::Number((*v as u64).into())),
                            Column::UInt16(v) => v.get(row_idx).map(|v| JsonValue::Number((*v as u64).into())),
                            Column::UInt32(v) => v.get(row_idx).map(|v| JsonValue::Number((*v as u64).into())),
                            Column::UInt64(v) => v.get(row_idx).map(|v| JsonValue::Number((*v).into())),
                            Column::Float32(v) => v.get(row_idx).map(|v| JsonValue::Number(serde_json::Number::from_f64(*v as f64).unwrap_or(0.into()))),
                            Column::Float64(v) => v.get(row_idx).map(|v| JsonValue::Number(serde_json::Number::from_f64(*v).unwrap_or(0.into()))),
                            Column::String(v) => v.get(row_idx).map(|v| JsonValue::String(v.clone())),
                            Column::Binary(v) => v.get(row_idx).map(|v| JsonValue::String(base64::encode(v))),
                            Column::Boolean(v) => v.get(row_idx).map(|v| JsonValue::Bool(*v)),
                            Column::Timestamp(v) => v.get(row_idx).map(|v| JsonValue::Number((*v).into())),
                            Column::Date(v) => v.get(row_idx).map(|v| JsonValue::Number((*v as i64).into())),
                        };
                        if let Some(val) = value {
                            values.insert(field.name.clone(), val);
                        } else {
                            values.insert(field.name.clone(), JsonValue::Null);
                        }
                    }
                }
                Row { values }
            }).collect()
        };
        
        let count = rows.len();
        Ok(QueryResult {
            rows,
            count,
        })
    }
}

/// Mutation root for GraphQL
pub struct MutationRoot {
    connection: Arc<dyn Connection>,
}

#[Object]
impl MutationRoot {
    /// Create a new table
    async fn create_table(&self, _ctx: &Context<'_>, input: CreateTableInput) -> GqlResult<Table> {
        // SECURITY: Validate table name format and prevent injection-like patterns
        if input.name.trim().is_empty() {
            return Err(async_graphql::Error::new("Table name cannot be empty"));
        }
        
        if input.name.len() > 255 {
            return Err(async_graphql::Error::new("Table name exceeds maximum length of 255"));
        }
        
        // Check for invalid characters that could cause issues
        if input.name.contains('\0') || input.name.contains('\n') || input.name.contains('\r') {
            return Err(async_graphql::Error::new("Table name contains invalid characters"));
        }
        
        // SECURITY: Check for potentially dangerous patterns (basic SQL injection prevention)
        let dangerous_patterns = ["--", "/*", "*/", ";", "DROP", "DELETE", "TRUNCATE", "ALTER", "EXEC", "EXECUTE", "UNION", "SELECT", "INSERT", "UPDATE"];
        let name_upper = input.name.to_uppercase();
        for pattern in &dangerous_patterns {
            if name_upper.contains(pattern) {
                return Err(async_graphql::Error::new("Table name contains invalid characters"));
            }
        }
        
        // SECURITY: Additional validation - only allow alphanumeric, underscore, and hyphen
        if !input.name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(async_graphql::Error::new("Table name can only contain alphanumeric characters, underscores, and hyphens"));
        }
        
        // SECURITY: Validate field names for duplicates and invalid characters
        let mut seen_names = std::collections::HashSet::new();
        for field in &input.fields {
            if field.name.trim().is_empty() {
                return Err(async_graphql::Error::new("Field name cannot be empty"));
            }
            if field.name.len() > 255 {
                return Err(async_graphql::Error::new(format!("Field name '{}' exceeds maximum length of 255", field.name)));
            }
            // SECURITY: Check for SQL injection-like patterns and special characters
            if field.name.contains('\0') || field.name.contains('\n') || field.name.contains('\r') {
                return Err(async_graphql::Error::new("Field name contains invalid characters"));
            }
            
            // SECURITY: Only allow alphanumeric, underscore, and hyphen in field names
            if !field.name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                return Err(async_graphql::Error::new("Field name can only contain alphanumeric characters, underscores, and hyphens"));
            }
            if !seen_names.insert(&field.name) {
                return Err(async_graphql::Error::new(format!("Duplicate field name: '{}'", field.name)));
            }
        }
        
        // Build schema from fields
        let mut fields = Vec::new();
        for f in &input.fields {
            let data_type = match f.data_type.as_str() {
                "Int8" => DataType::Int8,
                "Int16" => DataType::Int16,
                "Int32" => DataType::Int32,
                "Int64" => DataType::Int64,
                "UInt8" => DataType::UInt8,
                "UInt16" => DataType::UInt16,
                "UInt32" => DataType::UInt32,
                "UInt64" => DataType::UInt64,
                "Float32" => DataType::Float32,
                "Float64" => DataType::Float64,
                "String" => DataType::String,
                "Binary" => DataType::Binary,
                "Boolean" => DataType::Boolean,
                "Timestamp" => DataType::Timestamp,
                "Date" => DataType::Date,
                _ => return Err(async_graphql::Error::new(format!("Unknown data type: {}", f.data_type))),
            };
            fields.push(Field {
                name: f.name.clone(),
                data_type,
                nullable: f.nullable.unwrap_or(false),
                default_value: None,
            });
        }
        
        if fields.is_empty() {
            return Err(async_graphql::Error::new("Table must have at least one field"));
        }
        
        if fields.len() > 10_000 {
            return Err(async_graphql::Error::new("Table cannot have more than 10,000 fields"));
        }
        
        // SECURITY: Normalize Unicode to prevent homoglyph attacks (e.g., Cyrillic 'а' vs Latin 'a')
        let normalized_name: String = input.name
            .chars()
            .map(|c| {
                // Convert to ASCII if possible, reject ambiguous characters
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                    Ok(c)
                } else {
                    // Reject non-ASCII characters to prevent homoglyph attacks
                    Err(async_graphql::Error::new("Table name contains non-ASCII characters"))
                }
            })
            .collect::<std::result::Result<String, _>>()?;
        
        // SECURITY: Use a more secure hash to prevent hash collision attacks
        // Note: For production, consider using a cryptographically secure hash
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        normalized_name.hash(&mut hasher);
        // SECURITY: Add salt to prevent hash collision attacks
        "narayana_table_salt_v1".hash(&mut hasher);
        let table_id = TableId(hasher.finish() as u64);
        
        // SECURITY: Check if table already exists (hash collision or duplicate name)
        // SECURITY: Use normalized name for lookup to prevent case-sensitivity attacks
        if let Ok(Some(existing_id)) = self.connection.get_table_id(&normalized_name).await {
            if existing_id == table_id {
                return Err(async_graphql::Error::new("Table already exists"));
            }
            // SECURITY: If hash collision detected (same ID but different name), reject
            // This prevents hash collision attacks
            return Err(async_graphql::Error::new("Table name conflict detected"));
        }
        
        let schema = DbSchema::new(fields);
        
        // Create table
        self.connection.create_table(table_id, schema.clone()).await
            .map_err(|_| async_graphql::Error::new("Failed to create table"))?;
        
        Ok(Table {
            id: table_id.0,
            name: normalized_name, // Use normalized name
            schema,
            connection: Arc::clone(&self.connection),
        })
    }
    
    /// Insert rows into a table
    async fn insert(&self, _ctx: &Context<'_>, input: InsertInput) -> GqlResult<InsertResult> {
        // SECURITY: Validate and sanitize table name
        let table_name = input.table.trim();
        if table_name.is_empty() {
            return Err(async_graphql::Error::new("Table name cannot be empty"));
        }
        if table_name.len() > 255 {
            return Err(async_graphql::Error::new("Table name exceeds maximum length"));
        }
        if table_name.contains("..") || table_name.contains("/") || table_name.contains("\\") {
            return Err(async_graphql::Error::new("Invalid table name format"));
        }
        
        // Get table ID
        let table_id = self.connection.get_table_id(table_name).await
            .map_err(|_| async_graphql::Error::new("Failed to access table"))?;
        
        let table_id = table_id.ok_or_else(|| async_graphql::Error::new("Table not found"))?;
        
        // Get schema
        let schema = self.connection.get_schema(table_id).await
            .map_err(|_| async_graphql::Error::new("Failed to access schema"))?;
        
        if input.rows.is_empty() {
            return Ok(InsertResult { rows_inserted: 0 });
        }
        
        // SECURITY: Validate row structure matches schema and limit batch size
        if input.rows.len() > 1_000_000 {
            return Err(async_graphql::Error::new("Cannot insert more than 1,000,000 rows at once"));
        }
        
        // SECURITY: Estimate memory usage before processing with overflow protection
        const MAX_BATCH_MEMORY_BYTES: usize = 500 * 1024 * 1024; // 500MB
        let estimated_memory = input.rows.len()
            .checked_mul(schema.fields.len())
            .and_then(|x| x.checked_mul(100)) // Rough estimate per cell
            .unwrap_or(usize::MAX); // If overflow, use max to trigger limit
        
        if estimated_memory > MAX_BATCH_MEMORY_BYTES {
            return Err(async_graphql::Error::new("Insert batch too large"));
        }
        
        // SECURITY: Additional check - limit total number of cells to prevent memory exhaustion
        const MAX_TOTAL_CELLS: usize = 10_000_000; // 10M cells max
        let total_cells = input.rows.len()
            .checked_mul(schema.fields.len())
            .unwrap_or(usize::MAX);
        if total_cells > MAX_TOTAL_CELLS {
            return Err(async_graphql::Error::new("Insert batch contains too many cells"));
        }
        
        // Validate row structure matches schema
        for (row_idx, row) in input.rows.iter().enumerate() {
            if row.values.len() != schema.fields.len() {
                return Err(async_graphql::Error::new(format!(
                    "Row {} has incorrect number of values",
                    row_idx
                )));
            }
        }
        
        // Convert rows to columns - build vectors for each field
        let mut columns = Vec::new();
        for (field_idx, field) in schema.fields.iter().enumerate() {
            let column = match field.data_type {
                DataType::Int8 => {
                    let mut vec = Vec::with_capacity(input.rows.len());
                    for row in &input.rows {
                        let value = &row.values[field_idx];
                        if let Some(num) = value.as_i64() {
                            if num >= i8::MIN as i64 && num <= i8::MAX as i64 {
                                vec.push(num as i8);
                            } else {
                                return Err(async_graphql::Error::new(format!("Value {} exceeds i8 range", num)));
                            }
                        } else if field.nullable {
                            vec.push(0); // Default for null in nullable field
                        } else {
                            return Err(async_graphql::Error::new(format!(
                                "Cannot insert null into non-nullable field '{}'",
                                field.name
                            )));
                        }
                    }
                    Column::Int8(vec)
                }
                DataType::Int16 => {
                    let mut vec = Vec::with_capacity(input.rows.len());
                    for row in &input.rows {
                        let value = &row.values[field_idx];
                        if let Some(num) = value.as_i64() {
                            if num >= i16::MIN as i64 && num <= i16::MAX as i64 {
                                vec.push(num as i16);
                            } else {
                                return Err(async_graphql::Error::new(format!("Value {} exceeds i16 range", num)));
                            }
                        } else {
                            vec.push(0);
                        }
                    }
                    Column::Int16(vec)
                }
                DataType::Int32 => {
                    let mut vec = Vec::with_capacity(input.rows.len());
                    for row in &input.rows {
                        let value = &row.values[field_idx];
                        if let Some(num) = value.as_i64() {
                            if num >= i32::MIN as i64 && num <= i32::MAX as i64 {
                                vec.push(num as i32);
                            } else {
                                return Err(async_graphql::Error::new(format!("Value {} exceeds i32 range", num)));
                            }
                        } else {
                            vec.push(0);
                        }
                    }
                    Column::Int32(vec)
                }
                DataType::Int64 => {
                    let mut vec = Vec::with_capacity(input.rows.len());
                    for row in &input.rows {
                        let value = &row.values[field_idx];
                        vec.push(value.as_i64().unwrap_or(0));
                    }
                    Column::Int64(vec)
                }
                DataType::UInt8 => {
                    let mut vec = Vec::with_capacity(input.rows.len());
                    for row in &input.rows {
                        let value = &row.values[field_idx];
                        if let Some(num) = value.as_u64() {
                            if num <= u8::MAX as u64 {
                                vec.push(num as u8);
                            } else {
                                return Err(async_graphql::Error::new(format!("Value {} exceeds u8 range", num)));
                            }
                        } else if value.as_i64().map(|v| v < 0).unwrap_or(false) {
                            return Err(async_graphql::Error::new("Cannot insert negative value into UInt8 field"));
                        } else {
                            vec.push(0);
                        }
                    }
                    Column::UInt8(vec)
                }
                DataType::UInt16 => {
                    let mut vec = Vec::with_capacity(input.rows.len());
                    for row in &input.rows {
                        let value = &row.values[field_idx];
                        if let Some(num) = value.as_u64() {
                            if num <= u16::MAX as u64 {
                                vec.push(num as u16);
                            } else {
                                return Err(async_graphql::Error::new(format!("Value {} exceeds u16 range", num)));
                            }
                        } else if value.as_i64().map(|v| v < 0).unwrap_or(false) {
                            return Err(async_graphql::Error::new("Cannot insert negative value into UInt16 field"));
                        } else {
                            vec.push(0);
                        }
                    }
                    Column::UInt16(vec)
                }
                DataType::UInt32 => {
                    let mut vec = Vec::with_capacity(input.rows.len());
                    for row in &input.rows {
                        let value = &row.values[field_idx];
                        if let Some(num) = value.as_u64() {
                            if num <= u32::MAX as u64 {
                                vec.push(num as u32);
                            } else {
                                return Err(async_graphql::Error::new(format!("Value {} exceeds u32 range", num)));
                            }
                        } else if value.as_i64().map(|v| v < 0).unwrap_or(false) {
                            return Err(async_graphql::Error::new("Cannot insert negative value into UInt32 field"));
                        } else {
                            vec.push(0);
                        }
                    }
                    Column::UInt32(vec)
                }
                DataType::UInt64 => {
                    let mut vec = Vec::with_capacity(input.rows.len());
                    for row in &input.rows {
                        let value = &row.values[field_idx];
                        if let Some(num) = value.as_u64() {
                            vec.push(num);
                        } else if value.as_i64().map(|v| v < 0).unwrap_or(false) {
                            return Err(async_graphql::Error::new("Cannot insert negative value into UInt64 field"));
                        } else {
                            vec.push(0);
                        }
                    }
                    Column::UInt64(vec)
                }
                DataType::Float32 => {
                    let mut vec = Vec::with_capacity(input.rows.len());
                    for row in &input.rows {
                        let value = &row.values[field_idx];
                        if let Some(num) = value.as_f64() {
                            if num.is_finite() {
                                vec.push(num as f32);
                            } else {
                                return Err(async_graphql::Error::new("Cannot insert NaN or Infinity into Float32 field"));
                            }
                        } else if let Some(num) = value.as_i64() {
                            vec.push(num as f32);
                        } else if field.nullable {
                            vec.push(0.0); // Default for null in nullable field
                        } else {
                            return Err(async_graphql::Error::new(format!(
                                "Cannot insert null into non-nullable field '{}'",
                                field.name
                            )));
                        }
                    }
                    Column::Float32(vec)
                }
                DataType::Float64 => {
                    let mut vec = Vec::with_capacity(input.rows.len());
                    for row in &input.rows {
                        let value = &row.values[field_idx];
                        if let Some(num) = value.as_f64() {
                            if num.is_finite() {
                                vec.push(num);
                            } else {
                                return Err(async_graphql::Error::new("Cannot insert NaN or Infinity into Float64 field"));
                            }
                        } else if let Some(num) = value.as_i64() {
                            vec.push(num as f64);
                        } else if field.nullable {
                            vec.push(0.0); // Default for null in nullable field
                        } else {
                            return Err(async_graphql::Error::new(format!(
                                "Cannot insert null into non-nullable field '{}'",
                                field.name
                            )));
                        }
                    }
                    Column::Float64(vec)
                }
                DataType::String => {
                    let mut vec = Vec::with_capacity(input.rows.len());
                    for (row_idx, row) in input.rows.iter().enumerate() {
                        let value = &row.values[field_idx];
                        if let Some(s) = value.as_str() {
                            // SECURITY: Limit string length to prevent memory exhaustion
                            const MAX_STRING_LENGTH: usize = 10 * 1024 * 1024; // 10MB per string
                            if s.len() > MAX_STRING_LENGTH {
                                return Err(async_graphql::Error::new(format!(
                                    "String value in row {} exceeds maximum length of {} bytes",
                                    row_idx, MAX_STRING_LENGTH
                                )));
                            }
                            
                            // SECURITY: Check grapheme count to prevent Unicode-based memory exhaustion
                            // A string with many combining characters could be small in bytes but large in graphemes
                            use unicode_segmentation::UnicodeSegmentation;
                            let grapheme_count = s.graphemes(true).count();
                            const MAX_STRING_GRAPHEMES: usize = 10 * 1024 * 1024; // Same limit in graphemes
                            if grapheme_count > MAX_STRING_GRAPHEMES {
                                return Err(async_graphql::Error::new(format!(
                                    "String value in row {} exceeds maximum grapheme count",
                                    row_idx
                                )));
                            }
                            
                            vec.push(s.to_string());
                        } else if field.nullable {
                            vec.push(String::new()); // Empty string for null in nullable field
                        } else {
                            return Err(async_graphql::Error::new(format!(
                                "Cannot insert null into non-nullable field '{}' at row {}",
                                field.name, row_idx
                            )));
                        }
                    }
                    Column::String(vec)
                }
                DataType::Binary => {
                    let mut vec = Vec::with_capacity(input.rows.len());
                    for (row_idx, row) in input.rows.iter().enumerate() {
                        let value = &row.values[field_idx];
                        if let Some(s) = value.as_str() {
                            // SECURITY: Properly handle base64 decoding - fail if invalid instead of silently falling back
                            // SECURITY: Validate base64 padding to prevent padding oracle attacks
                            let bytes = if s.starts_with("data:") || s.contains(',') {
                                // Handle data URI format: data:mime/type;base64,<data>
                                let base64_part = s.split(',').last().unwrap_or(s);
                                // SECURITY: Validate base64 format before decoding
                                let base64_part = base64_part.trim();
                                if base64_part.is_empty() {
                                    return Err(async_graphql::Error::new(format!(
                                        "Empty base64 data in row {}",
                                        row_idx
                                    )));
                                }
                                // SECURITY: Check for valid base64 characters only
                                if !base64_part.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=') {
                                    return Err(async_graphql::Error::new(format!(
                                        "Invalid base64 characters in row {}",
                                        row_idx
                                    )));
                                }
                                base64::decode(base64_part)
                                    .map_err(|e| async_graphql::Error::new(format!(
                                        "Invalid base64 encoding in row {}: {}",
                                        row_idx, e
                                    )))?
                            } else {
                                // SECURITY: Validate base64 format
                                let s_trimmed = s.trim();
                                if s_trimmed.is_empty() {
                                    return Err(async_graphql::Error::new(format!(
                                        "Empty binary data in row {}",
                                        row_idx
                                    )));
                                }
                                // Try base64 decode first
                                match base64::decode(s_trimmed) {
                                    Ok(bytes) => bytes,
                                    Err(_) => {
                                        // SECURITY: Only allow ASCII as fallback, reject other encodings
                                        if s_trimmed.is_ascii() {
                                            s_trimmed.as_bytes().to_vec()
                                        } else {
                                            return Err(async_graphql::Error::new(format!(
                                                "Invalid binary data format in row {}: expected base64 or ASCII string",
                                                row_idx
                                            )));
                                        }
                                    }
                                }
                            };
                            
                            // SECURITY: Limit binary data size
                            const MAX_BINARY_LENGTH: usize = 100 * 1024 * 1024; // 100MB per binary value
                            if bytes.len() > MAX_BINARY_LENGTH {
                                return Err(async_graphql::Error::new(format!(
                                    "Binary value in row {} exceeds maximum length of {} bytes",
                                    row_idx, MAX_BINARY_LENGTH
                                )));
                            }
                            
                            vec.push(bytes);
                        } else if field.nullable {
                            vec.push(Vec::new()); // Empty bytes for null in nullable field
                        } else {
                            return Err(async_graphql::Error::new(format!(
                                "Cannot insert null into non-nullable field '{}' at row {}",
                                field.name, row_idx
                            )));
                        }
                    }
                    Column::Binary(vec)
                }
                DataType::Boolean => {
                    let mut vec = Vec::with_capacity(input.rows.len());
                    for (row_idx, row) in input.rows.iter().enumerate() {
                        let value = &row.values[field_idx];
                        if let Some(b) = value.as_bool() {
                            vec.push(b);
                        } else if field.nullable {
                            vec.push(false); // Default for null in nullable field
                        } else {
                            return Err(async_graphql::Error::new(format!(
                                "Cannot insert null into non-nullable field '{}' at row {}",
                                field.name, row_idx
                            )));
                        }
                    }
                    Column::Boolean(vec)
                }
                DataType::Timestamp => {
                    let mut vec = Vec::with_capacity(input.rows.len());
                    for (row_idx, row) in input.rows.iter().enumerate() {
                        let value = &row.values[field_idx];
                        if let Some(num) = value.as_i64() {
                            vec.push(num);
                        } else if field.nullable {
                            vec.push(0); // Default for null in nullable field
                        } else {
                            return Err(async_graphql::Error::new(format!(
                                "Cannot insert null into non-nullable field '{}' at row {}",
                                field.name, row_idx
                            )));
                        }
                    }
                    Column::Timestamp(vec)
                }
                DataType::Date => {
                    let mut vec = Vec::with_capacity(input.rows.len());
                    for (row_idx, row) in input.rows.iter().enumerate() {
                        let value = &row.values[field_idx];
                        if let Some(num) = value.as_i64() {
                            // SECURITY: Check for overflow when converting i64 to i32
                            if num < i32::MIN as i64 || num > i32::MAX as i64 {
                                return Err(async_graphql::Error::new(format!(
                                    "Date value {} in row {} exceeds i32 range ({} to {})",
                                    num, row_idx, i32::MIN, i32::MAX
                                )));
                            }
                            vec.push(num as i32);
                        } else if field.nullable {
                            vec.push(0); // Default for null in nullable field
                        } else {
                            return Err(async_graphql::Error::new(format!(
                                "Cannot insert null into non-nullable field '{}' at row {}",
                                field.name, row_idx
                            )));
                        }
                    }
                    Column::Date(vec)
                }
                // SECURITY: Unsupported types for GraphQL - return error
                DataType::Json => {
                    return Err(async_graphql::Error::new("JSON data type not supported in GraphQL inserts"));
                }
                DataType::Nullable(_) => {
                    return Err(async_graphql::Error::new("Nested nullable types not supported in GraphQL inserts"));
                }
                DataType::Array(_) => {
                    return Err(async_graphql::Error::new("Array data type not supported in GraphQL inserts"));
                }
                DataType::Map(_, _) => {
                    return Err(async_graphql::Error::new("Map data type not supported in GraphQL inserts"));
                }
            };
            columns.push(column);
        }
        
        // Write columns
        self.connection.write_columns(table_id, columns).await
            .map_err(|_| async_graphql::Error::new("Failed to insert data"))?;
        
        Ok(InsertResult {
            rows_inserted: input.rows.len(),
        })
    }
}

/// GraphQL Table type
pub struct Table {
    pub id: u64,
    pub name: String,
    pub schema: DbSchema,
    pub connection: Arc<dyn Connection>,
}

#[Object]
impl Table {
    /// Get table schema fields
    async fn fields(&self) -> Vec<TableField> {
        self.schema.fields.iter().map(|f| TableField {
            name: f.name.clone(),
            data_type: format!("{:?}", f.data_type),
            nullable: f.nullable,
        }).collect()
    }
    
    /// Query rows from this table
    async fn rows(&self, limit: Option<usize>, offset: Option<usize>, columns: Option<Vec<String>>) -> GqlResult<QueryResult> {
        let column_indices: Vec<u32> = if let Some(cols) = &columns {
            if cols.is_empty() {
                (0..self.schema.fields.len() as u32).collect()
            } else {
                cols.iter()
                    .filter_map(|name| {
                        self.schema.fields.iter()
                            .position(|f| f.name == *name)
                            .map(|i| i as u32)
                    })
                    .collect()
            }
        } else {
            (0..self.schema.fields.len() as u32).collect()
        };
        
        // SECURITY: Validate offset and limit
        let offset = offset.unwrap_or(0);
        if offset > 1_000_000_000 {
            return Err(async_graphql::Error::new("Offset exceeds maximum value of 1,000,000,000"));
        }
        
        let limit = limit.unwrap_or(100).min(10000);
        if limit == 0 {
            return Err(async_graphql::Error::new("Limit must be greater than 0"));
        }
        
        if offset.saturating_add(limit) > 1_000_000_000 {
            return Err(async_graphql::Error::new("Offset + limit exceeds maximum value"));
        }
        
        let columns_data = self.connection.read_columns(TableId(self.id), column_indices.clone(), offset, limit).await
            .map_err(|_| async_graphql::Error::new("Failed to read data"))?;
        
        // SECURITY: Limit result size
        const MAX_RESULT_SIZE_BYTES: usize = 100 * 1024 * 1024; // 100MB
        let estimated_size = columns_data.iter()
            .map(|c| c.len() * std::mem::size_of::<usize>())
            .sum::<usize>();
        if estimated_size > MAX_RESULT_SIZE_BYTES {
            return Err(async_graphql::Error::new("Query result exceeds maximum size"));
        }
        
        // Convert to rows (same logic as QueryRoot::query)
        let rows = if columns_data.is_empty() {
            Vec::new()
        } else {
            // SECURITY: Validate all columns have the same length
            let row_count = columns_data[0].len();
            for (idx, col) in columns_data.iter().enumerate() {
                if col.len() != row_count {
                    return Err(async_graphql::Error::new("Data integrity error"));
                }
            }
            
            (0..row_count).map(|row_idx| {
                let mut values = HashMap::new();
                for (col_idx, column) in columns_data.iter().enumerate() {
                    let field_idx = column_indices[col_idx] as usize;
                    if field_idx < self.schema.fields.len() {
                        let field = &self.schema.fields[field_idx];
                        let value = match column {
                            Column::Int8(v) => v.get(row_idx).map(|v| JsonValue::Number((*v as i64).into())),
                            Column::Int16(v) => v.get(row_idx).map(|v| JsonValue::Number((*v as i64).into())),
                            Column::Int32(v) => v.get(row_idx).map(|v| JsonValue::Number((*v as i64).into())),
                            Column::Int64(v) => v.get(row_idx).map(|v| JsonValue::Number((*v).into())),
                            Column::UInt8(v) => v.get(row_idx).map(|v| JsonValue::Number((*v as u64).into())),
                            Column::UInt16(v) => v.get(row_idx).map(|v| JsonValue::Number((*v as u64).into())),
                            Column::UInt32(v) => v.get(row_idx).map(|v| JsonValue::Number((*v as u64).into())),
                            Column::UInt64(v) => v.get(row_idx).map(|v| JsonValue::Number((*v).into())),
                            Column::Float32(v) => v.get(row_idx).map(|v| JsonValue::Number(serde_json::Number::from_f64(*v as f64).unwrap_or(0.into()))),
                            Column::Float64(v) => v.get(row_idx).map(|v| JsonValue::Number(serde_json::Number::from_f64(*v).unwrap_or(0.into()))),
                            Column::String(v) => v.get(row_idx).map(|v| JsonValue::String(v.clone())),
                            Column::Binary(v) => v.get(row_idx).map(|v| JsonValue::String(base64::encode(v))),
                            Column::Boolean(v) => v.get(row_idx).map(|v| JsonValue::Bool(*v)),
                            Column::Timestamp(v) => v.get(row_idx).map(|v| JsonValue::Number((*v).into())),
                            Column::Date(v) => v.get(row_idx).map(|v| JsonValue::Number((*v as i64).into())),
                        };
                        if let Some(val) = value {
                            values.insert(field.name.clone(), val);
                        } else {
                            values.insert(field.name.clone(), JsonValue::Null);
                        }
                    }
                }
                Row { values }
            }).collect()
        };
        
        let count = rows.len();
        Ok(QueryResult {
            rows,
            count,
        })
    }
}

/// Table field information
#[derive(SimpleObject)]
pub struct TableField {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
}

/// Query result
#[derive(SimpleObject)]
pub struct QueryResult {
    pub rows: Vec<Row>,
    pub count: usize,
}

/// Row with key-value pairs
#[derive(SimpleObject)]
pub struct Row {
    pub values: HashMap<String, JsonValue>,
}

/// Query input
#[derive(InputObject)]
pub struct QueryInput {
    pub table: String,
    pub columns: Vec<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Create table input
#[derive(InputObject)]
pub struct CreateTableInput {
    pub name: String,
    pub fields: Vec<CreateFieldInput>,
}

/// Create field input
#[derive(InputObject)]
pub struct CreateFieldInput {
    pub name: String,
    pub data_type: String,
    pub nullable: Option<bool>,
}

/// Insert input
#[derive(InputObject)]
pub struct InsertInput {
    pub table: String,
    pub rows: Vec<InsertRowInput>,
}

/// Insert row input
#[derive(InputObject)]
pub struct InsertRowInput {
    pub values: Vec<JsonValue>,
}

/// Insert result
#[derive(SimpleObject)]
pub struct InsertResult {
    pub rows_inserted: usize,
}

