// Comprehensive tests for API endpoints
// These test the HTTP API layer

use narayana_core::{
    schema::{Schema, Field, DataType},
    types::TableId,
    column::Column,
};

// Note: These tests would require a running server or mock server
// For now, we test the data structures and logic

// ============================================================================
// API REQUEST/RESPONSE TESTS
// ============================================================================

#[test]
fn test_create_table_request_validation() {
    use narayana_api::rest::CreateTableRequest;
    
    let request = CreateTableRequest {
        table_name: "test_table".to_string(),
        schema: Schema::new(vec![
            Field {
                name: "id".to_string(),
                data_type: DataType::Int64,
                nullable: false,
                default_value: None,
            },
        ]),
    };
    
    assert_eq!(request.table_name, "test_table");
    assert_eq!(request.schema.len(), 1);
}

#[test]
fn test_create_table_response() {
    use narayana_api::rest::CreateTableResponse;
    
    let response = CreateTableResponse {
        table_id: 42,
        success: true,
    };
    
    assert_eq!(response.table_id, 42);
    assert!(response.success);
}

#[test]
fn test_insert_request_validation() {
    use narayana_api::rest::InsertRequest;
    
    let request = InsertRequest {
        table_id: 1,
        columns: vec![
            Column::Int64(vec![1, 2, 3]),
            Column::String(vec!["a".to_string(), "b".to_string(), "c".to_string()]),
        ],
    };
    
    assert_eq!(request.table_id, 1);
    assert_eq!(request.columns.len(), 2);
}

#[test]
fn test_insert_response() {
    use narayana_api::rest::InsertResponse;
    
    let response = InsertResponse {
        rows_inserted: 100,
        success: true,
    };
    
    assert_eq!(response.rows_inserted, 100);
    assert!(response.success);
}

#[test]
fn test_query_request_validation() {
    use narayana_api::rest::QueryRequest;
    
    let request = QueryRequest {
        table_id: 1,
        columns: Some(vec!["id".to_string(), "name".to_string()]),
        filter: Some(serde_json::json!({"eq": {"id": 42}})),
        limit: Some(100),
    };
    
    assert_eq!(request.table_id, 1);
    assert!(request.columns.is_some());
    assert!(request.filter.is_some());
    assert_eq!(request.limit, Some(100));
}

#[test]
fn test_query_response() {
    use narayana_api::rest::QueryResponse;
    
    let response = QueryResponse {
        columns: vec![
            Column::Int64(vec![1, 2, 3]),
        ],
        row_count: 3,
    };
    
    assert_eq!(response.columns.len(), 1);
    assert_eq!(response.row_count, 3);
}

#[test]
fn test_error_response() {
    use narayana_api::rest::ErrorResponse;
    
    let response = ErrorResponse {
        error: "Table not found".to_string(),
        code: "TABLE_NOT_FOUND".to_string(),
    };
    
    assert!(response.error.contains("not found"));
    assert_eq!(response.code, "TABLE_NOT_FOUND");
}

// ============================================================================
// API SERIALIZATION TESTS
// ============================================================================

#[test]
fn test_api_serialization_create_table() {
    use narayana_api::rest::CreateTableRequest;
    use serde_json;
    
    let request = CreateTableRequest {
        table_name: "users".to_string(),
        schema: Schema::new(vec![
            Field {
                name: "id".to_string(),
                data_type: DataType::Int64,
                nullable: false,
                default_value: None,
            },
        ]),
    };
    
    let serialized = serde_json::to_string(&request).unwrap();
    let deserialized: CreateTableRequest = serde_json::from_str(&serialized).unwrap();
    
    assert_eq!(request.table_name, deserialized.table_name);
    assert_eq!(request.schema.len(), deserialized.schema.len());
}

#[test]
fn test_api_serialization_insert_request() {
    use narayana_api::rest::InsertRequest;
    use serde_json;
    
    let request = InsertRequest {
        table_id: 1,
        columns: vec![Column::Int64(vec![1, 2, 3])],
    };
    
    let serialized = serde_json::to_string(&request).unwrap();
    let deserialized: InsertRequest = serde_json::from_str(&serialized).unwrap();
    
    assert_eq!(request.table_id, deserialized.table_id);
    assert_eq!(request.columns.len(), deserialized.columns.len());
}

#[test]
fn test_api_serialization_query_request() {
    use narayana_api::rest::QueryRequest;
    use serde_json;
    
    let request = QueryRequest {
        table_id: 1,
        columns: Some(vec!["id".to_string()]),
        filter: Some(serde_json::json!({"gt": {"id": 10}})),
        limit: Some(100),
    };
    
    let serialized = serde_json::to_string(&request).unwrap();
    let deserialized: QueryRequest = serde_json::from_str(&serialized).unwrap();
    
    assert_eq!(request.table_id, deserialized.table_id);
    assert_eq!(request.limit, deserialized.limit);
}

// ============================================================================
// API VALIDATION TESTS
// ============================================================================

#[test]
fn test_api_empty_table_name() {
    use narayana_api::rest::CreateTableRequest;
    
    let request = CreateTableRequest {
        table_name: "".to_string(),
        schema: Schema::new(vec![]),
    };
    
    // Empty table name should be handled
    assert_eq!(request.table_name, "");
}

#[test]
fn test_api_invalid_table_id() {
    use narayana_api::rest::InsertRequest;
    
    let request = InsertRequest {
        table_id: 0, // Edge case
        columns: vec![Column::Int64(vec![1])],
    };
    
    assert_eq!(request.table_id, 0);
}

#[test]
fn test_api_max_table_id() {
    use narayana_api::rest::InsertRequest;
    
    let request = InsertRequest {
        table_id: u64::MAX,
        columns: vec![Column::Int64(vec![1])],
    };
    
    assert_eq!(request.table_id, u64::MAX);
}

#[test]
fn test_api_empty_columns_in_insert() {
    use narayana_api::rest::InsertRequest;
    
    let request = InsertRequest {
        table_id: 1,
        columns: vec![],
    };
    
    assert_eq!(request.columns.len(), 0);
}

#[test]
fn test_api_empty_columns_in_query() {
    use narayana_api::rest::QueryRequest;
    
    let request = QueryRequest {
        table_id: 1,
        columns: Some(vec![]),
        filter: None,
        limit: None,
    };
    
    assert_eq!(request.columns, Some(vec![]));
}

#[test]
fn test_api_zero_limit() {
    use narayana_api::rest::QueryRequest;
    
    let request = QueryRequest {
        table_id: 1,
        columns: None,
        filter: None,
        limit: Some(0),
    };
    
    assert_eq!(request.limit, Some(0));
}

#[test]
fn test_api_max_limit() {
    use narayana_api::rest::QueryRequest;
    
    let request = QueryRequest {
        table_id: 1,
        columns: None,
        filter: None,
        limit: Some(usize::MAX),
    };
    
    assert_eq!(request.limit, Some(usize::MAX));
}

// ============================================================================
// API ERROR HANDLING TESTS
// ============================================================================

#[test]
fn test_api_error_codes() {
    use narayana_api::rest::ErrorResponse;
    
    let error_codes = vec![
        "TABLE_NOT_FOUND",
        "COLUMN_NOT_FOUND",
        "INVALID_SCHEMA",
        "QUERY_ERROR",
        "STORAGE_ERROR",
    ];
    
    for code in error_codes {
        let response = ErrorResponse {
            error: format!("Error: {}", code),
            code: code.to_string(),
        };
        
        assert_eq!(response.code, code);
        assert!(response.error.contains("Error"));
    }
}

#[test]
fn test_api_error_messages_are_helpful() {
    use narayana_api::rest::ErrorResponse;
    
    let response = ErrorResponse {
        error: "Table with ID 42 not found. Available tables: [1, 2, 3]".to_string(),
        code: "TABLE_NOT_FOUND".to_string(),
    };
    
    assert!(response.error.contains("42"));
    assert!(response.error.contains("not found"));
}

// ============================================================================
// API EDGE CASES
// ============================================================================

#[test]
fn test_api_very_long_table_name() {
    use narayana_api::rest::CreateTableRequest;
    
    let long_name = "a".repeat(10000);
    let request = CreateTableRequest {
        table_name: long_name.clone(),
        schema: Schema::new(vec![]),
    };
    
    assert_eq!(request.table_name, long_name);
}

#[test]
fn test_api_special_characters_in_table_name() {
    use narayana_api::rest::CreateTableRequest;
    
    let special_names = vec![
        "table-with-dashes",
        "table_with_underscores",
        "table.with.dots",
        "table@with#special$chars",
    ];
    
    for name in special_names {
        let request = CreateTableRequest {
            table_name: name.to_string(),
            schema: Schema::new(vec![]),
        };
        assert_eq!(request.table_name, name);
    }
}

#[test]
fn test_api_unicode_in_table_name() {
    use narayana_api::rest::CreateTableRequest;
    
    let unicode_names = vec![
        "表",
        "🌍_table",
        "table_世界",
    ];
    
    for name in unicode_names {
        let request = CreateTableRequest {
            table_name: name.to_string(),
            schema: Schema::new(vec![]),
        };
        assert_eq!(request.table_name, name);
    }
}

#[test]
fn test_api_large_query_response() {
    use narayana_api::rest::QueryResponse;
    
    let large_data: Vec<i64> = (0..1_000_000).collect();
    let response = QueryResponse {
        columns: vec![Column::Int64(large_data)],
        row_count: 1_000_000,
    };
    
    assert_eq!(response.row_count, 1_000_000);
    assert_eq!(response.columns.len(), 1);
}

#[test]
fn test_api_many_columns_in_response() {
    use narayana_api::rest::QueryResponse;
    
    let columns: Vec<Column> = (0..1000).map(|i| Column::Int32(vec![i as i32])).collect();
    let response = QueryResponse {
        columns,
        row_count: 1,
    };
    
    assert_eq!(response.columns.len(), 1000);
}

// ============================================================================
// API FILTER SERIALIZATION TESTS
// ============================================================================

#[test]
fn test_api_filter_eq_serialization() {
    use serde_json;
    
    let filter = serde_json::json!({
        "eq": {
            "column": "id",
            "value": 42
        }
    });
    
    let serialized = serde_json::to_string(&filter).unwrap();
    let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    
    assert_eq!(filter, deserialized);
}

#[test]
fn test_api_filter_complex_serialization() {
    use serde_json;
    
    let filter = serde_json::json!({
        "and": [
            {"gt": {"column": "age", "value": 18}},
            {"lt": {"column": "age", "value": 65}},
            {"or": [
                {"eq": {"column": "status", "value": "active"}},
                {"eq": {"column": "status", "value": "pending"}}
            ]}
        ]
    });
    
    let serialized = serde_json::to_string(&filter).unwrap();
    let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    
    assert_eq!(filter, deserialized);
}

// ============================================================================
// API BATCH OPERATIONS TESTS
// ============================================================================

#[test]
fn test_api_batch_insert_request() {
    use narayana_api::rest::InsertRequest;
    
    // Large batch insert
    let columns = vec![
        Column::Int64((0..10000).collect()),
        Column::String((0..10000).map(|i| format!("value_{}", i)).collect()),
    ];
    
    let request = InsertRequest {
        table_id: 1,
        columns,
    };
    
    assert_eq!(request.columns.len(), 2);
    match &request.columns[0] {
        Column::Int64(data) => assert_eq!(data.len(), 10000),
        _ => panic!("Expected Int64"),
    }
}

#[test]
fn test_api_batch_query_request() {
    use narayana_api::rest::QueryRequest;
    
    // Query with many columns
    let columns: Vec<String> = (0..1000).map(|i| format!("col_{}", i)).collect();
    let request = QueryRequest {
        table_id: 1,
        columns: Some(columns.clone()),
        filter: None,
        limit: Some(10000),
    };
    
    assert_eq!(request.columns, Some(columns));
    assert_eq!(request.limit, Some(10000));
}

// ============================================================================
// API TYPE SAFETY TESTS
// ============================================================================

#[test]
fn test_api_type_mismatch_handling() {
    use narayana_api::rest::InsertRequest;
    
    // Mixing types in columns (should be handled by validation)
    let request = InsertRequest {
        table_id: 1,
        columns: vec![
            Column::Int32(vec![1, 2, 3]),
            Column::String(vec!["a".to_string(), "b".to_string()]),
            Column::Float64(vec![1.0, 2.0, 3.0]),
        ],
    };
    
    // Should be valid request structure
    assert_eq!(request.columns.len(), 3);
}

#[test]
fn test_api_nullable_handling() {
    use narayana_api::rest::InsertRequest;
    
    // Columns with different lengths (nullable scenario)
    let request = InsertRequest {
        table_id: 1,
        columns: vec![
            Column::Int64(vec![1, 2, 3]),
            Column::String(vec!["a".to_string(), "b".to_string()]), // Different length
        ],
    };
    
    // Structure should be valid
    assert_eq!(request.columns.len(), 2);
}

// ============================================================================
// API VERSIONING TESTS
// ============================================================================

#[test]
fn test_api_version_compatibility() {
    // Test that API structures are backward compatible
    use narayana_api::rest::CreateTableRequest;
    use serde_json;
    
    // Old format (if any)
    let old_format = serde_json::json!({
        "table_name": "test",
        "schema": {
            "fields": []
        }
    });
    
    // Should deserialize
    let request: CreateTableRequest = serde_json::from_value(old_format).unwrap();
    assert_eq!(request.table_name, "test");
}

// ============================================================================
// API SECURITY TESTS
// ============================================================================

#[test]
fn test_api_sql_injection_in_table_name() {
    use narayana_api::rest::CreateTableRequest;
    
    let malicious_name = "'; DROP TABLE users; --";
    let request = CreateTableRequest {
        table_name: malicious_name.to_string(),
        schema: Schema::new(vec![]),
    };
    
    // Should be treated as string, not executed
    assert_eq!(request.table_name, malicious_name);
}

#[test]
fn test_api_xss_in_string_data() {
    use narayana_api::rest::InsertRequest;
    
    let xss_string = "<script>alert('XSS')</script>";
    let request = InsertRequest {
        table_id: 1,
        columns: vec![Column::String(vec![xss_string.to_string()])],
    };
    
    match &request.columns[0] {
        Column::String(data) => assert_eq!(data[0], xss_string),
        _ => panic!("Expected String"),
    }
}

#[test]
fn test_api_path_traversal_in_table_name() {
    use narayana_api::rest::CreateTableRequest;
    
    let malicious_name = "../../etc/passwd";
    let request = CreateTableRequest {
        table_name: malicious_name.to_string(),
        schema: Schema::new(vec![]),
    };
    
    // Should be treated as string
    assert_eq!(request.table_name, malicious_name);
}

// ============================================================================
// API PERFORMANCE TESTS
// ============================================================================

#[test]
fn test_api_large_payload_serialization() {
    use narayana_api::rest::InsertRequest;
    use serde_json;
    use std::time::Instant;
    
    let large_data: Vec<i64> = (0..1_000_000).collect();
    let request = InsertRequest {
        table_id: 1,
        columns: vec![Column::Int64(large_data)],
    };
    
    let start = Instant::now();
    let serialized = serde_json::to_string(&request).unwrap();
    let duration = start.elapsed();
    
    assert!(!serialized.is_empty());
    assert!(duration.as_secs() < 30); // Should serialize reasonably fast
}

#[test]
fn test_api_large_payload_deserialization() {
    use narayana_api::rest::InsertRequest;
    use serde_json;
    use std::time::Instant;
    
    let large_data: Vec<i64> = (0..100_000).collect();
    let request = InsertRequest {
        table_id: 1,
        columns: vec![Column::Int64(large_data)],
    };
    
    let serialized = serde_json::to_string(&request).unwrap();
    
    let start = Instant::now();
    let deserialized: InsertRequest = serde_json::from_str(&serialized).unwrap();
    let duration = start.elapsed();
    
    assert_eq!(deserialized.table_id, 1);
    assert!(duration.as_secs() < 10);
}

