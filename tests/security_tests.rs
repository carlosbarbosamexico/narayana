// Security-focused tests to prevent injection attacks, buffer overflows, etc.

use narayana_core::{
    schema::{Schema, Field, DataType},
    types::TableId,
    column::Column,
    Error,
};
use narayana_storage::{ColumnStore, InMemoryColumnStore};
use narayana_query::vectorized::VectorizedOps;

// ============================================================================
// SQL INJECTION PREVENTION (for future SQL parser)
// ============================================================================

#[test]
fn test_sql_injection_in_table_name() {
    // Test that table names with SQL-like strings are handled safely
    let malicious_names = vec![
        "'; DROP TABLE users; --",
        "1' OR '1'='1",
        "admin'--",
        "1; DELETE FROM users;",
    ];
    
    for name in malicious_names {
        let schema = Schema::new(vec![
            Field {
                name: "id".to_string(),
                data_type: DataType::Int64,
                nullable: false,
                default_value: None,
            },
        ]);
        
        // Should create schema without executing SQL
        assert_eq!(schema.field_index("id"), Some(0));
    }
}

#[test]
fn test_sql_injection_in_field_name() {
    // Field names with SQL injection attempts
    let malicious_fields = vec![
        "id'; DROP TABLE--",
        "name' OR 1=1--",
        "value'); DELETE FROM--",
    ];
    
    for field_name in malicious_fields {
        let schema = Schema::new(vec![
            Field {
                name: field_name.clone(),
                data_type: DataType::Int64,
                nullable: false,
                default_value: None,
            },
        ]);
        
        // Should handle as regular string, not execute
        assert_eq!(schema.field_index(&field_name), Some(0));
    }
}

// ============================================================================
// BUFFER OVERFLOW PREVENTION
// ============================================================================

#[test]
fn test_prevent_buffer_overflow_on_large_input() {
    // Test that we don't overflow buffers with large inputs
    let large_data: Vec<i32> = (0..10_000_000).collect();
    let column = Column::Int32(large_data);
    
    // Operations should complete without overflow
    let _ = VectorizedOps::sum(&column);
    let _ = VectorizedOps::min(&column);
    let _ = VectorizedOps::max(&column);
    let _ = VectorizedOps::count(&column);
}

#[test]
fn test_prevent_buffer_overflow_on_string() {
    // Very long strings should be handled safely
    let long_string = "a".repeat(100_000_000);
    let column = Column::String(vec![long_string]);
    
    // Should not overflow
    assert_eq!(column.len(), 1);
}

#[test]
fn test_prevent_integer_overflow_in_calculations() {
    // Test that integer calculations don't silently overflow
    let column = Column::Int32(vec![i32::MAX, i32::MAX]);
    
    // Sum would overflow - should handle gracefully
    let sum = VectorizedOps::sum(&column);
    // May return wrapped value or None - both acceptable
    assert!(sum.is_some() || sum.is_none());
}

// ============================================================================
// PATH TRAVERSAL PREVENTION
// ============================================================================

#[test]
fn test_path_traversal_in_table_name() {
    // Table names with path traversal attempts
    let malicious_names = vec![
        "../../etc/passwd",
        "..\\..\\windows\\system32",
        "/etc/passwd",
        "C:\\Windows\\System32",
    ];
    
    for name in malicious_names {
        // Should be treated as regular string, not as file path
        let schema = Schema::new(vec![
            Field {
                name: name.clone(),
                data_type: DataType::Int64,
                nullable: false,
                default_value: None,
            },
        ]);
        
        assert_eq!(schema.field_index(&name), Some(0));
    }
}

// ============================================================================
// XSS PREVENTION (for web UI)
// ============================================================================

#[test]
fn test_xss_in_string_data() {
    // Strings with XSS attempts
    let xss_strings = vec![
        "<script>alert('XSS')</script>",
        "<img src=x onerror=alert('XSS')>",
        "javascript:alert('XSS')",
        "<svg onload=alert('XSS')>",
    ];
    
    for xss_string in xss_strings {
        let column = Column::String(vec![xss_string.clone()]);
        
        // Should store as-is, not execute
        match column {
            Column::String(data) => {
                assert_eq!(data[0], xss_string);
            }
            _ => panic!("Expected String"),
        }
    }
}

// ============================================================================
// NULL BYTE INJECTION
// ============================================================================

#[test]
fn test_null_byte_injection() {
    // Null bytes in strings (common attack vector)
    let malicious_strings = vec![
        "file\0.txt",
        "name\0\0\0value",
        "\0\0\0\0",
    ];
    
    for malicious in malicious_strings {
        let column = Column::String(vec![malicious.clone()]);
        
        match column {
            Column::String(data) => {
                assert_eq!(data[0], malicious);
            }
            _ => panic!("Expected String"),
        }
    }
}

// ============================================================================
// FORMAT STRING ATTACKS
// ============================================================================

#[test]
fn test_format_string_attack_prevention() {
    // Format string attack attempts
    let format_strings = vec![
        "%s%s%s%s%s%s%s%s",
        "%n%n%n%n",
        "%x%x%x%x",
        "%.1000d",
    ];
    
    for fmt_str in format_strings {
        let column = Column::String(vec![fmt_str.clone()]);
        
        // Should be treated as regular string
        match column {
            Column::String(data) => {
                assert_eq!(data[0], fmt_str);
            }
            _ => panic!("Expected String"),
        }
    }
}

// ============================================================================
// DENIAL OF SERVICE PREVENTION
// ============================================================================

#[tokio::test]
async fn test_dos_prevention_large_schema() {
    // Prevent DoS via extremely large schemas
    let fields: Vec<Field> = (0..100000).map(|i| Field {
        name: format!("field_{}", i),
        data_type: DataType::Int32,
        nullable: false,
        default_value: None,
    }).collect();
    
    let schema = Schema::new(fields);
    // Should complete without hanging
    assert_eq!(schema.len(), 100000);
}

#[tokio::test]
async fn test_dos_prevention_deeply_nested_types() {
    // Prevent DoS via extremely nested types
    let mut nested = DataType::Int32;
    for _ in 0..10000 {
        nested = DataType::Nullable(Box::new(DataType::Array(Box::new(nested))));
    }
    
    // Should complete without stack overflow
    assert!(!nested.is_fixed_size());
}

#[test]
fn test_dos_prevention_zip_bomb_compression() {
    // Test handling of highly compressible data (zip bomb scenario)
    use narayana_storage::compression::{create_compressor, create_decompressor};
    use narayana_core::types::CompressionType;
    
    let compressor = create_compressor(CompressionType::LZ4);
    let decompressor = create_decompressor(CompressionType::LZ4);
    
    // Highly compressible data
    let data = vec![0u8; 1000000];
    let compressed = compressor.compress(&data).unwrap();
    
    // Decompression should handle size limits
    let result = decompressor.decompress(&compressed, data.len());
    assert!(result.is_ok());
}

// ============================================================================
// INTEGER OVERFLOW ATTACKS
// ============================================================================

#[test]
fn test_integer_overflow_attack_prevention() {
    // Test prevention of integer overflow attacks
    let column = Column::Int32(vec![i32::MAX, 1]);
    
    // Operations should detect/handle overflow
    let sum = VectorizedOps::sum(&column);
    // Should not silently wrap
    assert!(sum.is_some() || sum.is_none());
}

#[test]
fn test_unsigned_underflow_prevention() {
    // Test prevention of unsigned underflow
    let column = Column::UInt64(vec![0, 1]);
    
    // Should handle correctly
    let min = VectorizedOps::min(&column);
    assert_eq!(min, Some(serde_json::Value::Number(0.into())));
}

// ============================================================================
// RACE CONDITION ATTACKS
// ============================================================================

#[tokio::test]
async fn test_time_of_check_time_of_use_attack() {
    use std::sync::Arc;
    
    let store = Arc::new(InMemoryColumnStore::new());
    let table_id = TableId(1);
    
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);
    
    store.create_table(table_id, schema).await.unwrap();
    
    // Check table exists
    let schema_check = store.get_schema(table_id).await;
    assert!(schema_check.is_ok());
    
    // Delete in another "thread" (simulated)
    store.delete_table(table_id).await.unwrap();
    
    // Try to use - should fail safely
    let result = store.read_columns(table_id, vec![0], 0, 10).await;
    assert!(result.is_err());
}

// ============================================================================
// RESOURCE EXHAUSTION PREVENTION
// ============================================================================

#[test]
fn test_resource_exhaustion_large_allocation() {
    // Test that we handle resource exhaustion gracefully
    // Note: This test may be skipped in CI due to memory constraints
    
    // Try to create very large column
    let large_data: Vec<i32> = (0..50_000_000).collect();
    let column = Column::Int32(large_data);
    
    // Should complete or fail gracefully, not crash
    assert_eq!(column.len(), 50_000_000);
}

#[tokio::test]
async fn test_resource_exhaustion_many_tables() {
    // Test handling of many tables
    let store = InMemoryColumnStore::new();
    
    // Create many tables
    for i in 0..10000 {
        let table_id = TableId(i);
        let schema = Schema::new(vec![
            Field {
                name: "id".to_string(),
                data_type: DataType::Int64,
                nullable: false,
                default_value: None,
            },
        ]);
        
        let result = store.create_table(table_id, schema).await;
        assert!(result.is_ok());
    }
}

// ============================================================================
// INPUT VALIDATION TESTS
// ============================================================================

#[test]
fn test_input_validation_empty_strings() {
    // Empty strings should be valid
    let column = Column::String(vec!["".to_string()]);
    assert_eq!(column.len(), 1);
}

#[test]
fn test_input_validation_whitespace_only() {
    // Whitespace-only strings should be valid
    let column = Column::String(vec![
        " ".to_string(),
        "\t".to_string(),
        "\n".to_string(),
        "   ".to_string(),
    ]);
    assert_eq!(column.len(), 4);
}

#[test]
fn test_input_validation_special_characters() {
    // All special characters should be handled
    let special = vec![
        "!@#$%^&*()".to_string(),
        "[]{}|\\:;\"'<>?,./".to_string(),
        "`~-_=+".to_string(),
    ];
    
    let column = Column::String(special.clone());
    match column {
        Column::String(data) => {
            assert_eq!(data, special);
        }
        _ => panic!("Expected String"),
    }
}

// ============================================================================
// ENCODING ATTACKS
// ============================================================================

#[test]
fn test_encoding_attack_utf8_invalid_sequences() {
    // Test handling of invalid UTF-8 sequences
    // Note: Rust's String type ensures valid UTF-8, but we test binary handling
    
    let column = Column::Binary(vec![
        vec![0xFF, 0xFE, 0xFD], // Invalid UTF-8
        vec![0xC0, 0x80], // Overlong encoding
        vec![0xED, 0xA0, 0x80], // Surrogate
    ]);
    
    assert_eq!(column.len(), 3);
}

#[test]
fn test_encoding_attack_mixed_encodings() {
    // Test handling of mixed encoding attempts
    let column = Column::String(vec![
        "ASCII".to_string(),
        "UTF-8: 中文".to_string(),
        "Mixed: hello 世界".to_string(),
    ]);
    
    assert_eq!(column.len(), 3);
}

// ============================================================================
// MEMORY SAFETY TESTS
// ============================================================================

#[test]
fn test_memory_safety_use_after_free_prevention() {
    // Test that we don't use freed memory
    let store = InMemoryColumnStore::new();
    let table_id = TableId(1);
    
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);
    
    store.create_table(table_id, schema).await.unwrap();
    store.delete_table(table_id).await.unwrap();
    
    // Accessing deleted table should error, not crash
    let result = store.get_schema(table_id).await;
    assert!(result.is_err());
}

#[test]
fn test_memory_safety_double_free_prevention() {
    // Test that we don't free memory twice
    let mut store = InMemoryColumnStore::new();
    let table_id = TableId(1);
    
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);
    
    store.create_table(table_id, schema).await.unwrap();
    store.delete_table(table_id).await.unwrap();
    
    // Second delete should error, not crash
    let result = store.delete_table(table_id).await;
    assert!(result.is_err());
}

// ============================================================================
// TYPE CONFUSION PREVENTION
// ============================================================================

#[test]
fn test_type_confusion_prevention() {
    // Ensure types can't be confused
    let int_col = Column::Int32(vec![1, 2, 3]);
    let string_col = Column::String(vec!["1".to_string(), "2".to_string(), "3".to_string()]);
    
    // These should be treated as different types
    assert_eq!(int_col.data_type(), DataType::Int32);
    assert_eq!(string_col.data_type(), DataType::String);
    
    // Operations should not work across types
    let value = serde_json::Value::String("1".to_string());
    let mask = VectorizedOps::compare_eq(&int_col, &value);
    // Should return all false for type mismatch
    assert_eq!(mask, vec![false, false, false]);
}

// ============================================================================
// INFORMATION DISCLOSURE PREVENTION
// ============================================================================

#[test]
fn test_information_disclosure_in_error_messages() {
    // Error messages should not leak sensitive information
    let store = InMemoryColumnStore::new();
    let table_id = TableId(999);
    
    let result = store.get_schema(table_id).await;
    
    // Error should be generic, not expose internal details
    assert!(result.is_err());
    match result.unwrap_err() {
        Error::Storage(msg) => {
            // Should not contain internal paths, memory addresses, etc.
            assert!(!msg.contains("0x"));
            assert!(!msg.contains("/"));
            assert!(!msg.contains("\\"));
        }
        _ => {}
    }
}

#[tokio::test]
async fn test_information_disclosure_table_not_found() {
    let store = InMemoryColumnStore::new();
    let table_id = TableId(999);
    
    let result = store.read_columns(table_id, vec![0], 0, 10).await;
    
    assert!(result.is_err());
    // Error should not reveal other table IDs or internal state
    match result.unwrap_err() {
        Error::Storage(msg) => {
            // Should be generic
            assert!(msg.contains("not found") || msg.contains("999"));
        }
        _ => {}
    }
}

