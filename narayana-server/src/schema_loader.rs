// Schema and Seed Data Loader
// Loads schema.nyn and seeds.nyn files to create tables and seed data

use anyhow::{Context, Result};
use narayana_core::{
    schema::{Schema, Field, DataType},
    types::TableId,
    column::Column,
};
use narayana_storage::database_manager::DatabaseManager;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};
use serde::Deserialize;
use std::collections::HashMap;

/// Schema definition from TOML
#[derive(Debug, Deserialize)]
struct SchemaFile {
    #[serde(default)]
    database: HashMap<String, DatabaseSection>,
    #[serde(default)]
    table: HashMap<String, TableDefinition>,
}

#[derive(Debug, Deserialize)]
struct DatabaseSection {
    // Database configuration can go here
}

#[derive(Debug, Deserialize)]
struct TableDefinition {
    fields: Vec<FieldDefinition>,
}

#[derive(Debug, Deserialize)]
struct FieldDefinition {
    name: String,
    #[serde(rename = "data_type")]
    data_type_str: String,
    nullable: bool,
    #[serde(default)]
    default_value: Option<toml::Value>,
}

/// Seed data from TOML
#[derive(Debug, Deserialize)]
struct SeedsFile {
    #[serde(default)]
    seeds: HashMap<String, Vec<HashMap<String, toml::Value>>>,
}

/// Parse data type string to DataType enum
fn parse_data_type(s: &str) -> Result<DataType> {
    match s {
        "Int8" => Ok(DataType::Int8),
        "Int16" => Ok(DataType::Int16),
        "Int32" => Ok(DataType::Int32),
        "Int64" => Ok(DataType::Int64),
        "UInt8" => Ok(DataType::UInt8),
        "UInt16" => Ok(DataType::UInt16),
        "UInt32" => Ok(DataType::UInt32),
        "UInt64" => Ok(DataType::UInt64),
        "Float32" => Ok(DataType::Float32),
        "Float64" => Ok(DataType::Float64),
        "Boolean" => Ok(DataType::Boolean),
        "String" => Ok(DataType::String),
        "Binary" => Ok(DataType::Binary),
        "Timestamp" => Ok(DataType::Timestamp),
        "Date" => Ok(DataType::Date),
        "Json" => Ok(DataType::Json),
        _ => {
            // Handle Nullable(Type), Array(Type), Map(Key, Value)
            if s.starts_with("Nullable(") && s.ends_with(")") {
                let inner = &s[9..s.len()-1];
                Ok(DataType::Nullable(Box::new(parse_data_type(inner)?)))
            } else if s.starts_with("Array(") && s.ends_with(")") {
                let inner = &s[6..s.len()-1];
                Ok(DataType::Array(Box::new(parse_data_type(inner)?)))
            } else if s.starts_with("Map(") && s.ends_with(")") {
                let inner = &s[4..s.len()-1];
                let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
                if parts.len() == 2 {
                    Ok(DataType::Map(
                        Box::new(parse_data_type(parts[0])?),
                        Box::new(parse_data_type(parts[1])?),
                    ))
                } else {
                    anyhow::bail!("Invalid Map type format: {}", s)
                }
            } else {
                anyhow::bail!("Unknown data type: {}", s)
            }
        }
    }
}

/// Convert TOML value to serde_json::Value
fn toml_to_json(value: toml::Value) -> serde_json::Value {
    match value {
        toml::Value::String(s) => serde_json::Value::String(s),
        toml::Value::Integer(i) => serde_json::Value::Number(i.into()),
        toml::Value::Float(f) => {
            serde_json::Value::Number(
                serde_json::Number::from_f64(f).unwrap_or_else(|| serde_json::Number::from(0))
            )
        }
        toml::Value::Boolean(b) => serde_json::Value::Bool(b),
        toml::Value::Datetime(dt) => serde_json::Value::String(dt.to_string()),
        toml::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(toml_to_json).collect())
        }
        toml::Value::Table(map) => {
            let mut json_map = serde_json::Map::new();
            for (k, v) in map {
                json_map.insert(k, toml_to_json(v));
            }
            serde_json::Value::Object(json_map)
        }
    }
}

/// Load schema from schema.nyn file and create tables
pub async fn load_schema(
    schema_path: &Path,
    db_manager: Arc<DatabaseManager>,
    storage: Arc<dyn narayana_storage::ColumnStore>,
) -> Result<HashMap<String, TableId>> {
    info!("📋 Loading schema from: {}", schema_path.display());
    
    if !schema_path.exists() {
        warn!("⚠️  Schema file not found: {}. Skipping schema loading.", schema_path.display());
        return Ok(HashMap::new());
    }
    
    let content = std::fs::read_to_string(schema_path)
        .with_context(|| format!("Failed to read schema file: {}", schema_path.display()))?;
    
    let schema_file: SchemaFile = toml::from_str(&content)
        .with_context(|| format!("Failed to parse schema file: {}", schema_path.display()))?;
    
    // Get or create default database
    let db_id = match db_manager.get_database_by_name("default") {
        Some(id) => {
            info!("✅ Using existing 'default' database");
            id
        }
        None => {
            info!("📦 Creating 'default' database");
            db_manager.create_database("default".to_string())
                .context("Failed to create default database")?
        }
    };
    
    let mut table_ids = HashMap::new();
    
    // Create tables
    for (table_name, table_def) in schema_file.table {
        info!("📊 Creating table: {}", table_name);
        
        // Convert field definitions to Fields
        let mut fields = Vec::new();
        for field_def in table_def.fields {
            let data_type = parse_data_type(&field_def.data_type_str)
                .with_context(|| format!("Invalid data type '{}' for field '{}'", field_def.data_type_str, field_def.name))?;
            
            let default_value = field_def.default_value.map(toml_to_json);
            
            fields.push(Field {
                name: field_def.name,
                data_type,
                nullable: field_def.nullable,
                default_value,
            });
        }
        
        let schema = Schema::new(fields);
        
        // Create table in database manager
        let table_id = db_manager.create_table(db_id, table_name.clone(), schema.clone())
            .with_context(|| format!("Failed to create table '{}' in database manager", table_name))?;
        
        // Create table in storage
        storage.create_table(table_id, schema.clone()).await
            .with_context(|| format!("Failed to create table '{}' in storage", table_name))?;
        
        table_ids.insert(table_name.clone(), table_id);
        info!("✅ Created table '{}' with ID {}", table_name, table_id.0);
    }
    
    info!("✅ Schema loaded: {} tables created", table_ids.len());
    Ok(table_ids)
}

/// Load seed data from seeds.nyn file and insert into tables
pub async fn load_seeds(
    seeds_path: &Path,
    table_ids: &HashMap<String, TableId>,
    db_manager: Arc<DatabaseManager>,
    storage: Arc<dyn narayana_storage::ColumnStore>,
) -> Result<()> {
    info!("🌱 Loading seeds from: {}", seeds_path.display());
    
    if !seeds_path.exists() {
        warn!("⚠️  Seeds file not found: {}. Skipping seed loading.", seeds_path.display());
        return Ok(());
    }
    
    let content = std::fs::read_to_string(seeds_path)
        .with_context(|| format!("Failed to read seeds file: {}", seeds_path.display()))?;
    
    let seeds_file: SeedsFile = toml::from_str(&content)
        .with_context(|| format!("Failed to parse seeds file: {}", seeds_path.display()))?;
    
    // Get database to access table schemas
    let db_id = match db_manager.get_database_by_name("default") {
        Some(id) => id,
        None => {
            warn!("⚠️  Default database not found. Skipping seed loading.");
            return Ok(());
        }
    };
    
    let mut total_rows = 0;
    
    // Insert seed data for each table
    for (table_name, rows) in seeds_file.seeds {
        let table_id = match table_ids.get(&table_name) {
            Some(id) => *id,
            None => {
                warn!("⚠️  Table '{}' not found in schema. Skipping seeds.", table_name);
                continue;
            }
        };
        
        // Get table schema
        let tables = db_manager.list_tables(db_id)
            .context("Failed to list tables")?;
        
        let table_info = tables.iter()
            .find(|t| t.table_id == table_id)
            .with_context(|| format!("Table '{}' not found in database", table_name))?;
        
        let schema = &table_info.schema;
        
        let row_count = rows.len();
        info!("🌱 Seeding table '{}' with {} rows", table_name, row_count);
        
        if rows.is_empty() {
            continue;
        }
        
        // Collect all values for each field across all rows
        let mut column_data: Vec<Vec<toml::Value>> = vec![Vec::new(); schema.fields.len()];
        
        for row in rows {
            for (field_idx, field) in schema.fields.iter().enumerate() {
                let value = row.get(&field.name)
                    .cloned()
                    .or_else(|| {
                        // Use default value if available
                        field.default_value.as_ref().map(|_| {
                            // We'll handle defaults when creating columns
                            toml::Value::String("__DEFAULT__".to_string())
                        })
                    })
                    .unwrap_or_else(|| {
                        if field.nullable {
                            toml::Value::String("__NULL__".to_string())
                        } else {
                            toml::Value::String("".to_string())
                        }
                    });
                column_data[field_idx].push(value);
            }
        }
        
        // Convert collected values to columns
        let mut columns = Vec::new();
        for (field_idx, field) in schema.fields.iter().enumerate() {
            let values = &column_data[field_idx];
            
            // Convert TOML values to Column based on field type
            let column = match &field.data_type {
                    DataType::String | DataType::Json => {
                        let string_values: Vec<String> = values.iter().map(|v| {
                            match v {
                                toml::Value::String(s) => s.clone(),
                                _ => v.to_string(),
                            }
                        }).collect();
                        Column::String(string_values)
                    }
                    DataType::Int64 | DataType::Timestamp => {
                        let int_values: Vec<i64> = values.iter().map(|v| {
                            match v {
                                toml::Value::Integer(i) => *i,
                                _ => 0, // Default to 0 for invalid values
                            }
                        }).collect();
                        Column::Int64(int_values)
                    }
                    DataType::Int32 | DataType::Date => {
                        let int_values: Vec<i32> = values.iter().map(|v| {
                            match v {
                                toml::Value::Integer(i) => (*i).try_into().unwrap_or(0),
                                _ => 0,
                            }
                        }).collect();
                        Column::Int32(int_values)
                    }
                    DataType::Int16 => {
                        let int_values: Vec<i16> = values.iter().map(|v| {
                            match v {
                                toml::Value::Integer(i) => (*i).try_into().unwrap_or(0),
                                _ => 0,
                            }
                        }).collect();
                        Column::Int16(int_values)
                    }
                    DataType::Int8 => {
                        let int_values: Vec<i8> = values.iter().map(|v| {
                            match v {
                                toml::Value::Integer(i) => (*i).try_into().unwrap_or(0),
                                _ => 0,
                            }
                        }).collect();
                        Column::Int8(int_values)
                    }
                    DataType::UInt64 => {
                        let uint_values: Vec<u64> = values.iter().map(|v| {
                            match v {
                                toml::Value::Integer(i) => (*i).try_into().unwrap_or(0),
                                _ => 0,
                            }
                        }).collect();
                        Column::UInt64(uint_values)
                    }
                    DataType::UInt32 => {
                        let uint_values: Vec<u32> = values.iter().map(|v| {
                            match v {
                                toml::Value::Integer(i) => (*i).try_into().unwrap_or(0),
                                _ => 0,
                            }
                        }).collect();
                        Column::UInt32(uint_values)
                    }
                    DataType::UInt16 => {
                        let uint_values: Vec<u16> = values.iter().map(|v| {
                            match v {
                                toml::Value::Integer(i) => (*i).try_into().unwrap_or(0),
                                _ => 0,
                            }
                        }).collect();
                        Column::UInt16(uint_values)
                    }
                    DataType::UInt8 => {
                        let uint_values: Vec<u8> = values.iter().map(|v| {
                            match v {
                                toml::Value::Integer(i) => (*i).try_into().unwrap_or(0),
                                _ => 0,
                            }
                        }).collect();
                        Column::UInt8(uint_values)
                    }
                    DataType::Float64 => {
                        let float_values: Vec<f64> = values.iter().map(|v| {
                            match v {
                                toml::Value::Float(f) => *f,
                                toml::Value::Integer(i) => *i as f64,
                                _ => 0.0,
                            }
                        }).collect();
                        Column::Float64(float_values)
                    }
                    DataType::Float32 => {
                        let float_values: Vec<f32> = values.iter().map(|v| {
                            match v {
                                toml::Value::Float(f) => *f as f32,
                                toml::Value::Integer(i) => *i as f32,
                                _ => 0.0,
                            }
                        }).collect();
                        Column::Float32(float_values)
                    }
                    DataType::Boolean => {
                        let bool_values: Vec<bool> = values.iter().map(|v| {
                            match v {
                                toml::Value::Boolean(b) => *b,
                                _ => false,
                            }
                        }).collect();
                        Column::Boolean(bool_values)
                    }
                    DataType::Binary => {
                        // Convert strings to binary
                        let binary_values: Vec<Vec<u8>> = values.iter().map(|v| {
                            match v {
                                toml::Value::String(s) => s.as_bytes().to_vec(),
                                _ => v.to_string().as_bytes().to_vec(),
                            }
                        }).collect();
                        Column::Binary(binary_values)
                    }
                    DataType::Nullable(_inner) => {
                        // For nullable types, extract the inner type and handle nulls
                        // For simplicity, convert to string representation
                        let string_values: Vec<String> = values.iter().map(|v| {
                            if matches!(v, toml::Value::String(s) if s == "__NULL__") {
                                String::new()
                            } else {
                                v.to_string()
                            }
                        }).collect();
                        Column::String(string_values)
                    }
                    DataType::Array(_) | DataType::Map(_, _) => {
                        // For complex types, serialize to JSON string
                        let string_values: Vec<String> = values.iter().map(|v| {
                            serde_json::to_string(&toml_to_json(v.clone())).unwrap_or_else(|_| v.to_string())
                        }).collect();
                        Column::String(string_values)
                    }
                };
            
            columns.push(column);
        }
        
        // Ensure all columns have the same length
        let expected_len = columns.first().map(|c| c.len()).unwrap_or(0);
        if expected_len == 0 {
            warn!("⚠️  No valid data to insert for table '{}'", table_name);
            continue;
        }
        
        // Insert all rows at once
        storage.write_columns(table_id, columns).await
            .with_context(|| format!("Failed to insert seed data into table '{}'", table_name))?;
        
        total_rows += expected_len;
        
        info!("✅ Seeded table '{}' with {} rows", table_name, row_count);
    }
    
    info!("✅ Seeds loaded: {} total rows inserted", total_rows);
    Ok(())
}

/// Load both schema and seeds
pub async fn load_schema_and_seeds(
    schema_dir: &Path,
    db_manager: Arc<DatabaseManager>,
    storage: Arc<dyn narayana_storage::ColumnStore>,
) -> Result<()> {
    let schema_path = schema_dir.join("schema.nyn");
    let seeds_path = schema_dir.join("seeds.nyn");
    
    // Load schema first
    let table_ids = load_schema(&schema_path, db_manager.clone(), storage.clone()).await?;
    
    // Then load seeds
    load_seeds(&seeds_path, &table_ids, db_manager, storage).await?;
    
    Ok(())
}

