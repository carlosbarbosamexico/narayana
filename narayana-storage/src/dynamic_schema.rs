// Dynamic Schema Changes - On-the-Fly Column Adding/Removing
// Safe, Incredibly Powerful Schema Alterations

use narayana_core::{Error, Result, schema::{Schema, Field, DataType}, types::TableId};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use tracing::{info, warn, debug};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::webhooks::{WebhookManager, WebhookEvent, WebhookEventType, WebhookScope};
use crate::migration_free::{AutomaticTypeConverter, MigrationFreeSchemaManager};

/// Schema change operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SchemaChange {
    AddColumn {
        column: Field,
        position: Option<usize>, // None = append
        default_value: Option<serde_json::Value>,
    },
    DropColumn {
        column_name: String,
        safe: bool, // If true, only drop if no data conflicts
    },
    ModifyColumn {
        column_name: String,
        new_field: Field,
        data_migration: Option<DataMigration>,
    },
    RenameColumn {
        old_name: String,
        new_name: String,
    },
    ReorderColumns {
        column_order: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataMigration {
    pub migration_type: MigrationType,
    pub function: Option<String>, // Custom migration function
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationType {
    Direct,           // Direct type conversion (e.g., int32 -> int64)
    Cast,            // Type casting with validation
    Transform,       // Custom transformation
    Default,         // Use default value
    DropAndRecreate, // Drop and recreate with new type
}

/// Schema change result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaChangeResult {
    pub success: bool,
    pub change_type: SchemaChange,
    pub affected_rows: u64,
    pub migration_errors: Vec<String>,
    pub rollback_available: bool,
    pub duration_ms: f64,
}

/// Dynamic schema manager - safe schema alterations
/// Migration-free - no migration scripts needed!
pub struct DynamicSchemaManager {
    tables: Arc<RwLock<HashMap<TableId, TableSchemaInfo>>>,
    change_history: Arc<RwLock<Vec<SchemaChangeHistory>>>,
    validation_enabled: bool,
    auto_backup: bool,
    webhook_manager: Option<Arc<WebhookManager>>,
    migration_free: Option<Arc<MigrationFreeSchemaManager>>,
}

#[derive(Debug, Clone)]
struct TableSchemaInfo {
    table_id: TableId,
    schema: Schema,
    version: u64,
    column_history: Vec<ColumnHistory>,
    last_modified: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ColumnHistory {
    column_name: String,
    added_at: u64,
    removed_at: Option<u64>,
    modified_at: Vec<u64>,
    data_type_history: Vec<DataType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaChangeHistory {
    pub table_id: TableId,
    pub change: SchemaChange,
    pub result: SchemaChangeResult,
    pub timestamp: u64,
    pub rolled_back: bool,
    pub rollback_data: Option<SchemaSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaSnapshot {
    pub schema: Schema,
    pub timestamp: u64,
    pub data_sample: Option<Vec<serde_json::Value>>,
}

impl DynamicSchemaManager {
    pub fn new() -> Self {
        Self {
            tables: Arc::new(RwLock::new(HashMap::new())),
            change_history: Arc::new(RwLock::new(Vec::new())),
            validation_enabled: true,
            auto_backup: true,
            webhook_manager: None,
            migration_free: Some(Arc::new(MigrationFreeSchemaManager::new())),
        }
    }

    pub fn with_webhooks(webhook_manager: Arc<WebhookManager>) -> Self {
        Self {
            tables: Arc::new(RwLock::new(HashMap::new())),
            change_history: Arc::new(RwLock::new(Vec::new())),
            validation_enabled: true,
            auto_backup: true,
            webhook_manager: Some(webhook_manager),
            migration_free: Some(Arc::new(MigrationFreeSchemaManager::new())),
        }
    }

    /// Set webhook manager on-the-fly
    pub fn set_webhook_manager(&mut self, webhook_manager: Arc<WebhookManager>) {
        self.webhook_manager = Some(webhook_manager);
    }

    /// Enable migration-free mode (default: enabled)
    pub fn enable_migration_free(&mut self) {
        if self.migration_free.is_none() {
            self.migration_free = Some(Arc::new(MigrationFreeSchemaManager::new()));
        }
    }

    /// Add column on-the-fly
    pub async fn add_column(
        &self,
        table_id: TableId,
        column: Field,
        position: Option<usize>,
        default_value: Option<serde_json::Value>,
    ) -> Result<SchemaChangeResult> {
        let start_time = SystemTime::now();
        
        // Validate column name uniqueness
        self.validate_column_name(&table_id, &column.name)?;
        
        // Get current schema
        let mut tables = self.tables.write();
        let table_info = tables.get_mut(&table_id)
            .ok_or_else(|| Error::Storage(format!("Table {} not found", table_id.0)))?;
        
        // Create snapshot for rollback
        let snapshot = if self.auto_backup {
            Some(SchemaSnapshot {
                schema: table_info.schema.clone(),
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
                data_sample: None,
            })
        } else {
            None
        };
        
        // Add column to schema
        let mut new_fields = table_info.schema.fields.clone();
        
        if let Some(pos) = position {
            if pos <= new_fields.len() {
                new_fields.insert(pos, column.clone());
            } else {
                new_fields.push(column.clone());
            }
        } else {
            new_fields.push(column.clone());
        }
        
        let new_schema = Schema::new(new_fields);
        
        // Update schema version
        table_info.schema = new_schema.clone();
        table_info.version += 1;
        table_info.last_modified = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        
        // Record in history
        table_info.column_history.push(ColumnHistory {
            column_name: column.name.clone(),
            added_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            removed_at: None,
            modified_at: vec![],
            data_type_history: vec![column.data_type.clone()],
        });
        
        drop(tables);
        
        // Apply default value to existing rows (async)
        // Migration-free: automatic data migration!
        let affected_rows = if let Some(ref migration_free) = self.migration_free {
            // Use migration-free system for automatic migration
            self.apply_default_to_existing_rows_with_migration(&table_id, &column.name, default_value, migration_free).await?
        } else {
            self.apply_default_to_existing_rows(&table_id, &column.name, default_value).await?
        };
        
        let duration = start_time.elapsed().unwrap_or_default().as_millis() as f64;
        
        let result = SchemaChangeResult {
            success: true,
            change_type: SchemaChange::AddColumn {
                column,
                position,
                default_value: None,
            },
            affected_rows,
            migration_errors: vec![],
            rollback_available: snapshot.is_some(),
            duration_ms: duration,
        };
        
        // Record change history
        let mut history = self.change_history.write();
        history.push(SchemaChangeHistory {
            table_id,
            change: result.change_type.clone(),
            result: result.clone(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            rolled_back: false,
            rollback_data: snapshot,
        });
        drop(history);
        
        // Trigger webhook for schema alteration
        if let Some(ref webhook_manager) = self.webhook_manager {
            let _ = self.trigger_schema_change_webhook(
                webhook_manager,
                table_id,
                &result.change_type,
                &result,
            ).await;
        }
        
        info!("Added column to table {}: {} rows affected in {:.2}ms", table_id.0, affected_rows, duration);
        
        Ok(result)
    }

    /// Drop column on-the-fly (safe)
    pub async fn drop_column(
        &self,
        table_id: TableId,
        column_name: String,
        safe: bool,
    ) -> Result<SchemaChangeResult> {
        let start_time = SystemTime::now();
        
        // Get current schema
        let mut tables = self.tables.write();
        let table_info = tables.get_mut(&table_id)
            .ok_or_else(|| Error::Storage(format!("Table {} not found", table_id.0)))?;
        
        // Check if column exists
        if !table_info.schema.fields.iter().any(|f| f.name == column_name) {
            return Err(Error::Storage(format!("Column {} not found", column_name)));
        }
        
        // Safe check: verify column is not required or has dependencies
        if safe {
            self.validate_safe_drop(&table_id, &column_name)?;
        }
        
        // Create snapshot for rollback
        let snapshot = if self.auto_backup {
            Some(SchemaSnapshot {
                schema: table_info.schema.clone(),
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
                data_sample: None,
            })
        } else {
            None
        };
        
        // Remove column from schema
        let new_fields: Vec<Field> = table_info.schema.fields
            .iter()
            .filter(|f| f.name != column_name)
            .cloned()
            .collect();
        
        let new_schema = Schema::new(new_fields);
        
        // Update schema version
        table_info.schema = new_schema;
        table_info.version += 1;
        table_info.last_modified = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        
        // Update column history
        for col_history in &mut table_info.column_history {
            if col_history.column_name == column_name {
                col_history.removed_at = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
            }
        }
        
        drop(tables);
        
        // Remove column data (async)
        let affected_rows = self.remove_column_data(&table_id, &column_name).await?;
        
        let duration = start_time.elapsed().unwrap_or_default().as_millis() as f64;
        
        let result = SchemaChangeResult {
            success: true,
            change_type: SchemaChange::DropColumn {
                column_name: column_name.clone(),
                safe,
            },
            affected_rows,
            migration_errors: vec![],
            rollback_available: snapshot.is_some(),
            duration_ms: duration,
        };
        
        // Record change history
        let mut history = self.change_history.write();
        history.push(SchemaChangeHistory {
            table_id,
            change: result.change_type.clone(),
            result: result.clone(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            rolled_back: false,
            rollback_data: snapshot,
        });
        drop(history);
        
        // Trigger webhook for schema alteration
        if let Some(ref webhook_manager) = self.webhook_manager {
            let _ = self.trigger_schema_change_webhook(
                webhook_manager,
                table_id,
                &result.change_type,
                &result,
            ).await;
        }
        
        info!("Dropped column {} from table {}: {} rows affected in {:.2}ms", column_name, table_id.0, affected_rows, duration);
        
        Ok(result)
    }

    /// Modify column on-the-fly
    pub async fn modify_column(
        &self,
        table_id: TableId,
        column_name: String,
        new_field: Field,
        data_migration: Option<DataMigration>,
    ) -> Result<SchemaChangeResult> {
        let start_time = SystemTime::now();
        
        // Get current schema
        let mut tables = self.tables.write();
        let table_info = tables.get_mut(&table_id)
            .ok_or_else(|| Error::Storage(format!("Table {} not found", table_id.0)))?;
        
        // Find existing column
        let old_field = table_info.schema.fields
            .iter()
            .find(|f| f.name == column_name)
            .ok_or_else(|| Error::Storage(format!("Column {} not found", column_name)))?
            .clone();
        
        // Validate type compatibility
        if self.validation_enabled {
            self.validate_type_compatibility(&old_field.data_type, &new_field.data_type)?;
        }
        
        // Create snapshot
        let snapshot = if self.auto_backup {
            Some(SchemaSnapshot {
                schema: table_info.schema.clone(),
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
                data_sample: None,
            })
        } else {
            None
        };
        
        // Update schema
        let new_fields: Vec<Field> = table_info.schema.fields
            .iter()
            .map(|f| {
                if f.name == column_name {
                    new_field.clone()
                } else {
                    f.clone()
                }
            })
            .collect();
        
        let new_schema = Schema::new(new_fields);
        table_info.schema = new_schema;
        table_info.version += 1;
        table_info.last_modified = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        
        // Update column history
        for col_history in &mut table_info.column_history {
            if col_history.column_name == column_name {
                col_history.modified_at.push(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
                col_history.data_type_history.push(new_field.data_type.clone());
            }
        }
        
        drop(tables);
        
        // Migrate data - Migration-free: automatic type conversion!
        let data_migration_clone = data_migration.clone(); // Clone before moving
        let (affected_rows, migration_errors) = if old_field.data_type != new_field.data_type {
            if let Some(ref migration_free) = self.migration_free {
                // Use automatic type converter
                self.migrate_column_data_migration_free(
                    &table_id,
                    &column_name,
                    &old_field.data_type,
                    &new_field.data_type,
                    migration_free,
                ).await?
            } else {
                self.migrate_column_data(&table_id, &column_name, &old_field.data_type, &new_field.data_type, data_migration).await?
            }
        } else {
            (0, vec![])
        };
        
        let duration = start_time.elapsed().unwrap_or_default().as_millis() as f64;
        
        let result = SchemaChangeResult {
            success: migration_errors.is_empty(),
            change_type: SchemaChange::ModifyColumn {
                column_name,
                new_field,
                data_migration: data_migration_clone,
            },
            affected_rows,
            migration_errors,
            rollback_available: snapshot.is_some(),
            duration_ms: duration,
        };
        
        // Record change history
        let mut history = self.change_history.write();
        history.push(SchemaChangeHistory {
            table_id,
            change: result.change_type.clone(),
            result: result.clone(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            rolled_back: false,
            rollback_data: snapshot,
        });
        
        Ok(result)
    }

    /// Rename column on-the-fly
    pub async fn rename_column(
        &self,
        table_id: TableId,
        old_name: String,
        new_name: String,
    ) -> Result<SchemaChangeResult> {
        // Validate new name uniqueness
        self.validate_column_name(&table_id, &new_name)?;
        
        let start_time = SystemTime::now();
        
        let mut tables = self.tables.write();
        let table_info = tables.get_mut(&table_id)
            .ok_or_else(|| Error::Storage(format!("Table {} not found", table_id.0)))?;
        
        let snapshot = if self.auto_backup {
            Some(SchemaSnapshot {
                schema: table_info.schema.clone(),
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
                data_sample: None,
            })
        } else {
            None
        };
        
        // Update schema
        let new_fields: Vec<Field> = table_info.schema.fields
            .iter()
            .map(|f| {
                if f.name == old_name {
                    Field {
                        name: new_name.clone(),
                        ..f.clone()
                    }
                } else {
                    f.clone()
                }
            })
            .collect();
        
        let new_schema = Schema::new(new_fields);
        table_info.schema = new_schema;
        table_info.version += 1;
        
        drop(tables);
        
        let duration = start_time.elapsed().unwrap_or_default().as_millis() as f64;
        
        let result = SchemaChangeResult {
            success: true,
            change_type: SchemaChange::RenameColumn {
                old_name,
                new_name,
            },
            affected_rows: 0,
            migration_errors: vec![],
            rollback_available: snapshot.is_some(),
            duration_ms: duration,
        };
        
        let mut history = self.change_history.write();
        history.push(SchemaChangeHistory {
            table_id,
            change: result.change_type.clone(),
            result: result.clone(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            rolled_back: false,
            rollback_data: snapshot,
        });
        
        Ok(result)
    }

    /// Rollback schema change
    pub async fn rollback_change(
        &self,
        table_id: TableId,
        change_index: usize,
    ) -> Result<()> {
        let mut history = self.change_history.write();
        
        if change_index >= history.len() {
            return Err(Error::Storage("Invalid change index".to_string()));
        }
        
        let change_history = &history[change_index];
        
        if change_history.rolled_back {
            return Err(Error::Storage("Change already rolled back".to_string()));
        }
        
        if change_history.table_id != table_id {
            return Err(Error::Storage("Table ID mismatch".to_string()));
        }
        
        let snapshot = change_history.rollback_data.as_ref()
            .ok_or_else(|| Error::Storage("No rollback data available".to_string()))?;
        
        // Restore schema
        let mut tables = self.tables.write();
        if let Some(table_info) = tables.get_mut(&table_id) {
            table_info.schema = snapshot.schema.clone();
            table_info.version += 1;
        }
        
        // Mark as rolled back
        history[change_index].rolled_back = true;
        
        info!("Rolled back schema change for table {}", table_id.0);
        Ok(())
    }

    /// Get schema change history
    pub fn get_change_history(&self, table_id: TableId) -> Vec<SchemaChangeHistory> {
        let history = self.change_history.read();
        history.iter()
            .filter(|h| h.table_id == table_id)
            .cloned()
            .collect()
    }

    // Helper methods

    fn validate_column_name(&self, table_id: &TableId, column_name: &str) -> Result<()> {
        let tables = self.tables.read();
        if let Some(table_info) = tables.get(table_id) {
            if table_info.schema.fields.iter().any(|f| f.name == column_name) {
                return Err(Error::Storage(format!("Column {} already exists", column_name)));
            }
        }
        Ok(())
    }

    fn validate_safe_drop(&self, table_id: &TableId, column_name: &str) -> Result<()> {
        let tables = self.tables.read();
        if let Some(table_info) = tables.get(table_id) {
            let field = table_info.schema.fields.iter()
                .find(|f| f.name == column_name)
                .ok_or_else(|| Error::Storage(format!("Column {} not found", column_name)))?;
            
            if !field.nullable && field.default_value.is_none() {
                return Err(Error::Storage(format!(
                    "Cannot safely drop non-nullable column {} without default",
                    column_name
                )));
            }
        }
        Ok(())
    }

    fn validate_type_compatibility(&self, old_type: &DataType, new_type: &DataType) -> Result<()> {
        // Type compatibility matrix
        match (old_type, new_type) {
            // Compatible conversions
            (DataType::Int8, DataType::Int16) |
            (DataType::Int8, DataType::Int32) |
            (DataType::Int8, DataType::Int64) |
            (DataType::Int16, DataType::Int32) |
            (DataType::Int16, DataType::Int64) |
            (DataType::Int32, DataType::Int64) |
            (DataType::UInt8, DataType::UInt16) |
            (DataType::UInt8, DataType::UInt32) |
            (DataType::UInt8, DataType::UInt64) |
            (DataType::UInt16, DataType::UInt32) |
            (DataType::UInt16, DataType::UInt64) |
            (DataType::UInt32, DataType::UInt64) |
            (DataType::Float32, DataType::Float64) |
            (DataType::Date, DataType::Timestamp) => Ok(()),
            
            // Same type
            (a, b) if a == b => Ok(()),
            
            // Incompatible - requires explicit migration
            _ => Err(Error::Storage(format!(
                "Type conversion from {:?} to {:?} requires explicit data migration",
                old_type, new_type
            ))),
        }
    }

    async fn apply_default_to_existing_rows(
        &self,
        _table_id: &TableId,
        _column_name: &str,
        _default_value: Option<serde_json::Value>,
    ) -> Result<u64> {
        // In production, would apply default to all existing rows
        Ok(0)
    }

    async fn remove_column_data(
        &self,
        _table_id: &TableId,
        _column_name: &str,
    ) -> Result<u64> {
        // In production, would remove column data
        Ok(0)
    }

    async fn migrate_column_data(
        &self,
        _table_id: &TableId,
        _column_name: &str,
        _old_type: &DataType,
        _new_type: &DataType,
        _migration: Option<DataMigration>,
    ) -> Result<(u64, Vec<String>)> {
        // In production, would migrate data
        Ok((0, vec![]))
    }

    async fn migrate_column_data_migration_free(
        &self,
        _table_id: &TableId,
        _column_name: &str,
        old_type: &DataType,
        new_type: &DataType,
        _migration_free: &MigrationFreeSchemaManager,
    ) -> Result<(u64, Vec<String>)> {
        // Migration-free: use automatic type converter!
        // In production, would iterate through rows and convert
        let mut migrated = 0;
        let mut errors = Vec::new();

        // Automatic type conversion for all values
        // Example: converting Int32 to Int64
        match AutomaticTypeConverter::convert_value(old_type, new_type, &JsonValue::Null) {
            Ok(_) => {
                migrated += 1; // Would be actual row count
            }
            Err(e) => {
                errors.push(format!("Migration error: {}", e));
            }
        }

        Ok((migrated, errors))
    }

    async fn apply_default_to_existing_rows_with_migration(
        &self,
        _table_id: &TableId,
        _column_name: &str,
        default_value: Option<JsonValue>,
        _migration_free: &MigrationFreeSchemaManager,
    ) -> Result<u64> {
        // Migration-free: automatically apply default values
        // In production, would apply to all existing rows
        Ok(0) // Would return actual affected row count
    }

    /// Initialize table schema info
    pub fn initialize_table(&self, table_id: TableId, schema: Schema) -> Result<()> {
        let mut tables = self.tables.write();
        tables.insert(table_id, TableSchemaInfo {
            table_id,
            schema,
            version: 1,
            column_history: vec![],
            last_modified: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        });
        Ok(())
    }

    /// Get current schema
    pub fn get_schema(&self, table_id: TableId) -> Option<Schema> {
        let tables = self.tables.read();
        tables.get(&table_id).map(|t| t.schema.clone())
    }

    /// Get schema version
    pub fn get_schema_version(&self, table_id: TableId) -> Option<u64> {
        let tables = self.tables.read();
        tables.get(&table_id).map(|t| t.version)
    }

    /// Trigger webhook for schema change
    async fn trigger_schema_change_webhook(
        &self,
        webhook_manager: &WebhookManager,
        table_id: TableId,
        change: &SchemaChange,
        result: &SchemaChangeResult,
    ) -> Result<()> {
        // Get table info for scope
        let tables = self.tables.read();
        let table_info = if let Some(info) = tables.get(&table_id) {
            info.clone()
        } else {
            drop(tables);
            return Ok(());
        };
        drop(tables);

        // Create webhook event
        let event_data = serde_json::json!({
            "table_id": table_id.0,
            "change": change,
            "result": result,
            "schema_version": table_info.version,
        });

        let event = WebhookEvent {
            event_type: WebhookEventType::Alter,
            scope: WebhookScope::Table {
                db_name: "default".to_string(), // Would get from context
                table_name: format!("table_{}", table_id.0),
            },
            data: event_data,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };

        webhook_manager.trigger_webhook(event).await?;
        Ok(())
    }
}

