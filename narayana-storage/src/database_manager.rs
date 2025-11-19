// Database management system - create databases and tables at runtime
// With Transform & Filter System - Dynamic Output Configuration!

use narayana_core::{
    Error, Result, schema::Schema, types::TableId,
    transforms::OutputConfig,
};
use crate::dynamic_output::DynamicOutputManager;
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Database ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DatabaseId(pub u64);

/// Database metadata
#[derive(Debug, Clone)]
pub struct Database {
    pub id: DatabaseId,
    pub name: String,
    pub created_at: u64,
    pub tables: HashMap<TableId, String>, // table_id -> table_name
}

/// Database manager - true DBMS functionality
/// With Transform & Filter System!
pub struct DatabaseManager {
    databases: Arc<RwLock<HashMap<DatabaseId, Database>>>,
    tables: Arc<RwLock<HashMap<TableId, TableInfo>>>,
    name_to_db: Arc<RwLock<HashMap<String, DatabaseId>>>,
    name_to_table: Arc<RwLock<HashMap<String, TableId>>>,
    next_db_id: Arc<std::sync::atomic::AtomicU64>,
    next_table_id: Arc<std::sync::atomic::AtomicU64>,
    // NEW: Transform & Filter System
    output_manager: Arc<DynamicOutputManager>,
}

#[derive(Debug, Clone)]
pub struct TableInfo {
    pub table_id: TableId,
    pub name: String,
    pub database_id: DatabaseId,
    pub schema: Schema,
    pub created_at: u64,
    // NEW: Output configuration for transforms/filters
    pub output_config: Option<OutputConfig>,
}

impl DatabaseManager {
    pub fn new() -> Self {
        Self {
            databases: Arc::new(RwLock::new(HashMap::new())),
            tables: Arc::new(RwLock::new(HashMap::new())),
            name_to_db: Arc::new(RwLock::new(HashMap::new())),
            name_to_table: Arc::new(RwLock::new(HashMap::new())),
            next_db_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            next_table_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            output_manager: Arc::new(DynamicOutputManager::new()),
        }
    }
    
    /// Get output manager for dynamic transforms/filters
    pub fn output_manager(&self) -> &DynamicOutputManager {
        &self.output_manager
    }

    /// Create database at runtime (no restart needed)
    pub fn create_database(&self, name: String) -> Result<DatabaseId> {
        let mut name_to_db = self.name_to_db.write();
        if name_to_db.contains_key(&name) {
            return Err(Error::Storage(format!("Database '{}' already exists", name)));
        }

        let db_id = DatabaseId(self.next_db_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
        
        let database = Database {
            id: db_id,
            name: name.clone(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            tables: HashMap::new(),
        };

        let mut databases = self.databases.write();
        databases.insert(db_id, database);
        name_to_db.insert(name, db_id);

        Ok(db_id)
    }

    /// Create table at runtime (no restart needed)
    pub fn create_table(&self, database_id: DatabaseId, name: String, schema: Schema) -> Result<TableId> {
        self.create_table_with_config(database_id, name, schema, None)
    }
    
    /// Create table with output configuration
    pub fn create_table_with_config(
        &self,
        database_id: DatabaseId,
        name: String,
        schema: Schema,
        output_config: Option<OutputConfig>,
    ) -> Result<TableId> {
        // Verify database exists
        let databases = self.databases.read();
        let database = databases.get(&database_id)
            .ok_or_else(|| Error::Storage(format!("Database {} not found", database_id.0)))?;

        // Check if table name already exists in database
        let name_to_table = self.name_to_table.read();
        let full_name = format!("{}.{}", database.name, name);
        if name_to_table.contains_key(&full_name) {
            return Err(Error::Storage(format!("Table '{}' already exists", full_name)));
        }

        let table_id = TableId(self.next_table_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed));

        let table_info = TableInfo {
            table_id,
            name: name.clone(),
            database_id,
            schema: schema.clone(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            output_config: output_config.clone(),
        };
        
        // Initialize output config in dynamic manager
        if let Some(config) = output_config {
            use narayana_core::transforms::ConfigContext;
            let context = ConfigContext::Database { table_id: table_id.0 };
            self.output_manager.initialize_config(
                context,
                table_id.0.to_string(),
                config,
            )?;
        }

        // Update databases
        drop(databases);
        let mut databases = self.databases.write();
        if let Some(db) = databases.get_mut(&database_id) {
            db.tables.insert(table_id, name.clone());
        }

        // Update tables
        let mut tables = self.tables.write();
        tables.insert(table_id, table_info);

        // Update name mapping
        drop(name_to_table);
        let mut name_to_table = self.name_to_table.write();
        name_to_table.insert(full_name, table_id);

        Ok(table_id)
    }

    /// Get database by name
    pub fn get_database_by_name(&self, name: &str) -> Option<DatabaseId> {
        let name_to_db = self.name_to_db.read();
        name_to_db.get(name).copied()
    }

    /// Get table by name
    pub fn get_table_by_name(&self, database_name: &str, table_name: &str) -> Option<TableId> {
        let name_to_table = self.name_to_table.read();
        let full_name = format!("{}.{}", database_name, table_name);
        name_to_table.get(&full_name).copied()
    }

    /// List all databases
    pub fn list_databases(&self) -> Vec<Database> {
        let databases = self.databases.read();
        databases.values().cloned().collect()
    }

    /// List tables in database
    pub fn list_tables(&self, database_id: DatabaseId) -> Result<Vec<TableInfo>> {
        let databases = self.databases.read();
        let database = databases.get(&database_id)
            .ok_or_else(|| Error::Storage(format!("Database {} not found", database_id.0)))?;

        let tables = self.tables.read();
        let mut result = Vec::new();
        for table_id in database.tables.keys() {
            if let Some(table_info) = tables.get(table_id) {
                result.push(table_info.clone());
            }
        }

        Ok(result)
    }

    /// Drop database (cascades to tables)
    pub fn drop_database(&self, database_id: DatabaseId) -> Result<()> {
        let mut databases = self.databases.write();
        let database = databases.remove(&database_id)
            .ok_or_else(|| Error::Storage(format!("Database {} not found", database_id.0)))?;

        // Drop all tables
        let mut tables = self.tables.write();
        let mut name_to_table = self.name_to_table.write();
        
        for table_id in database.tables.keys() {
            tables.remove(table_id);
            // Remove from name mapping
            name_to_table.retain(|_, &mut tid| tid != *table_id);
        }

        // Remove from name mapping
        let mut name_to_db = self.name_to_db.write();
        name_to_db.remove(&database.name);

        Ok(())
    }

    /// Drop table
    pub fn drop_table(&self, table_id: TableId) -> Result<()> {
        let mut tables = self.tables.write();
        let table_info = tables.remove(&table_id)
            .ok_or_else(|| Error::Storage(format!("Table {} not found", table_id.0)))?;

        // Remove from database
        let mut databases = self.databases.write();
        if let Some(db) = databases.get_mut(&table_info.database_id) {
            db.tables.remove(&table_id);
        }

        // Remove from name mapping
        let mut name_to_table = self.name_to_table.write();
        let full_name = format!("{}.{}", 
            databases.get(&table_info.database_id)
                .map(|d| d.name.clone())
                .unwrap_or_default(),
            table_info.name
        );
        name_to_table.remove(&full_name);

        Ok(())
    }

    /// Get table info
    pub fn get_table_info(&self, table_id: TableId) -> Option<TableInfo> {
        let tables = self.tables.read();
        tables.get(&table_id).cloned()
    }

    /// Alter table schema at runtime (no restart needed)
    pub fn alter_table(&self, table_id: TableId, new_schema: Schema) -> Result<()> {
        let mut tables = self.tables.write();
        let table_info = tables.get_mut(&table_id)
            .ok_or_else(|| Error::Storage(format!("Table {} not found", table_id.0)))?;

        // Update schema (in production, would validate compatibility)
        table_info.schema = new_schema;

        Ok(())
    }
    
    // ============================================
    // TRANSFORM & FILTER SYSTEM FOR DATABASE
    // ============================================
    
    /// Get table output config
    pub fn get_table_output_config(&self, table_id: TableId) -> Option<OutputConfig> {
        use narayana_core::transforms::ConfigContext;
        let context = ConfigContext::Database { table_id: table_id.0 };
        self.output_manager.get_config(&context, &table_id.0.to_string())
    }
    
    /// Get table output config with profile
    pub fn get_table_output_config_with_profile(
        &self,
        table_id: TableId,
        profile: Option<&str>,
    ) -> Option<OutputConfig> {
        use narayana_core::transforms::ConfigContext;
        let context = ConfigContext::Database { table_id: table_id.0 };
        self.output_manager.get_config_with_profile(&context, &table_id.0.to_string(), profile)
    }
}

