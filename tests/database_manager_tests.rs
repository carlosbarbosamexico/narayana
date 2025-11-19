// Tests for database manager

use narayana_storage::database_manager::*;
use narayana_core::schema::{Schema, Field, DataType};

#[test]
fn test_database_manager_creation() {
    let manager = DatabaseManager::new();
    // Should create successfully
}

#[test]
fn test_create_database() {
    let manager = DatabaseManager::new();
    let db_id = manager.create_database("test_db".to_string()).unwrap();
    assert!(db_id.0 > 0);
}

#[test]
fn test_create_duplicate_database() {
    let manager = DatabaseManager::new();
    manager.create_database("test_db".to_string()).unwrap();
    let result = manager.create_database("test_db".to_string());
    assert!(result.is_err());
}

#[test]
fn test_get_database_by_name() {
    let manager = DatabaseManager::new();
    let db_id = manager.create_database("test_db".to_string()).unwrap();
    let retrieved = manager.get_database_by_name("test_db");
    assert_eq!(retrieved, Some(db_id));
}

#[test]
fn test_create_table() {
    let manager = DatabaseManager::new();
    let db_id = manager.create_database("test_db".to_string()).unwrap();
    
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);
    
    let table_id = manager.create_table(db_id, "test_table".to_string(), schema).unwrap();
    assert!(table_id.0 > 0);
}

#[test]
fn test_list_databases() {
    let manager = DatabaseManager::new();
    manager.create_database("db1".to_string()).unwrap();
    manager.create_database("db2".to_string()).unwrap();
    
    let databases = manager.list_databases();
    assert_eq!(databases.len(), 2);
}

#[test]
fn test_list_tables() {
    let manager = DatabaseManager::new();
    let db_id = manager.create_database("test_db".to_string()).unwrap();
    
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);
    
    manager.create_table(db_id, "table1".to_string(), schema.clone()).unwrap();
    manager.create_table(db_id, "table2".to_string(), schema).unwrap();
    
    let tables = manager.list_tables(db_id).unwrap();
    assert_eq!(tables.len(), 2);
}

#[test]
fn test_drop_database() {
    let manager = DatabaseManager::new();
    let db_id = manager.create_database("test_db".to_string()).unwrap();
    
    manager.drop_database(db_id).unwrap();
    let databases = manager.list_databases();
    assert_eq!(databases.len(), 0);
}

#[test]
fn test_drop_table() {
    let manager = DatabaseManager::new();
    let db_id = manager.create_database("test_db".to_string()).unwrap();
    
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);
    
    let table_id = manager.create_table(db_id, "test_table".to_string(), schema).unwrap();
    manager.drop_table(table_id).unwrap();
    
    let tables = manager.list_tables(db_id).unwrap();
    assert_eq!(tables.len(), 0);
}

#[test]
fn test_alter_table() {
    let manager = DatabaseManager::new();
    let db_id = manager.create_database("test_db".to_string()).unwrap();
    
    let schema1 = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);
    
    let table_id = manager.create_table(db_id, "test_table".to_string(), schema1).unwrap();
    
    let schema2 = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
        Field {
            name: "name".to_string(),
            data_type: DataType::String,
            nullable: false,
            default_value: None,
        },
    ]);
    
    manager.alter_table(table_id, schema2).unwrap();
    let table_info = manager.get_table_info(table_id).unwrap();
    assert_eq!(table_info.schema.fields.len(), 2);
}

