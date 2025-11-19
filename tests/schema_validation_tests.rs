// Tests for schema validation and constraints

use narayana_core::{
    schema::{Schema, Field, DataType},
    Error,
};

#[test]
fn test_schema_validation_field_names() {
    // Test various field name validations
    let valid_names = vec![
        "id",
        "user_id",
        "user_name",
        "field123",
        "_private",
    ];
    
    for name in valid_names {
        let schema = Schema::new(vec![
            Field {
                name: name.to_string(),
                data_type: DataType::Int32,
                nullable: false,
                default_value: None,
            },
        ]);
        assert_eq!(schema.field_index(name), Some(0));
    }
}

#[test]
fn test_schema_validation_required_fields() {
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
        Field {
            name: "optional".to_string(),
            data_type: DataType::String,
            nullable: true,
            default_value: None,
        },
    ]);
    
    assert_eq!(schema.fields[0].nullable, false);
    assert_eq!(schema.fields[1].nullable, true);
}

#[test]
fn test_schema_validation_default_values() {
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: Some(serde_json::Value::Number(0.into())),
        },
        Field {
            name: "name".to_string(),
            data_type: DataType::String,
            nullable: true,
            default_value: Some(serde_json::Value::String("unknown".to_string())),
        },
    ]);
    
    assert!(schema.fields[0].default_value.is_some());
    assert!(schema.fields[1].default_value.is_some());
}

#[test]
fn test_schema_validation_type_consistency() {
    // Test that schema fields have consistent types
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
        Field {
            name: "value".to_string(),
            data_type: DataType::Float64,
            nullable: false,
            default_value: None,
        },
    ]);
    
    assert_eq!(schema.fields[0].data_type, DataType::Int64);
    assert_eq!(schema.fields[1].data_type, DataType::Float64);
}

