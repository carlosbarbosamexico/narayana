// Tests for JSON support

use narayana_core::json_support::*;
use serde_json::json;

#[test]
fn test_json_column_creation() {
    let column = JsonColumn::new();
    assert_eq!(column.len(), 0);
}

#[test]
fn test_json_column_extract() {
    let mut column = JsonColumn::new();
    column.values.push(json!({
        "user": {
            "name": "John",
            "age": 30
        }
    }));
    
    let extracted = column.extract("user.name");
    assert_eq!(extracted[0], Some(json!("John")));
}

#[test]
fn test_json_column_filter() {
    let mut column = JsonColumn::new();
    column.values.push(json!({"status": "active"}));
    column.values.push(json!({"status": "inactive"}));
    
    let filtered = column.filter("status", JsonCondition::Eq(json!("active")));
    assert_eq!(filtered, vec![true, false]);
}

#[test]
fn test_flexible_schema_creation() {
    let schema = FlexibleSchema::new();
    assert_eq!(schema.fields.len(), 0);
    assert!(schema.allow_extra);
}

#[test]
fn test_flexible_schema_required() {
    let schema = FlexibleSchema::new()
        .required("id", narayana_core::schema::DataType::Int64);
    
    assert_eq!(schema.fields.len(), 1);
}

#[test]
fn test_flexible_schema_optional() {
    let schema = FlexibleSchema::new()
        .optional("name", narayana_core::schema::DataType::String);
    
    assert_eq!(schema.fields.len(), 1);
}

#[test]
fn test_flexible_schema_json() {
    let schema = FlexibleSchema::new()
        .json("metadata");
    
    assert_eq!(schema.fields.len(), 1);
}

#[test]
fn test_flexible_schema_validate() {
    let schema = FlexibleSchema::new()
        .required("id", narayana_core::schema::DataType::Int64)
        .optional("name", narayana_core::schema::DataType::String);
    
    let mut data = HashMap::new();
    data.insert("id".to_string(), json!(1));
    data.insert("name".to_string(), json!("test"));
    
    let result = schema.validate(&data);
    assert!(result.is_ok());
}

#[test]
fn test_flexible_schema_validate_missing_required() {
    let schema = FlexibleSchema::new()
        .required("id", narayana_core::schema::DataType::Int64);
    
    let data = HashMap::new();
    let result = schema.validate(&data);
    assert!(result.is_err());
}

use std::collections::HashMap;

