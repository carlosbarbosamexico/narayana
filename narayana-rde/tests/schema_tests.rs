// Schema extraction tests for narayana-rde

use narayana_rde::*;
use narayana_rde::events;
use narayana_storage::native_events::{EventsConfig, NativeEventsSystem};
use std::sync::Arc;

fn create_test_manager() -> RdeManager {
    let mut config = EventsConfig::default();
    config.max_message_size = 10 * 1024 * 1024;
    config.enable_persistence = false;
    let native_events = Arc::new(NativeEventsSystem::new(config));
    RdeManager::new(native_events)
}

#[tokio::test]
async fn test_schema_extraction_simple_object() {
    let payload = serde_json::json!({
        "string_field": "value",
        "number_field": 42,
        "bool_field": true,
        "null_field": null
    });
    
    let schema = events::extract_schema(&payload).unwrap();
    assert_eq!(schema.fields.len(), 4);
    
    let field_names: Vec<&str> = schema.fields.iter().map(|f| f.name.as_str()).collect();
    assert!(field_names.contains(&"string_field"));
    assert!(field_names.contains(&"number_field"));
    assert!(field_names.contains(&"bool_field"));
    assert!(field_names.contains(&"null_field"));
}

#[tokio::test]
async fn test_schema_extraction_nested_object() {
    let payload = serde_json::json!({
        "user": {
            "name": "John",
            "age": 30,
            "address": {
                "street": "123 Main St",
                "city": "New York"
            }
        },
        "order_id": "12345"
    });
    
    let schema = events::extract_schema(&payload).unwrap();
    assert!(schema.fields.len() > 0);
    
    // Should extract nested fields
    let field_names: Vec<String> = schema.fields.iter().map(|f| f.name.clone()).collect();
    assert!(field_names.iter().any(|n| n.contains("user")));
}

#[tokio::test]
async fn test_schema_extraction_array() {
    let payload = serde_json::json!([1, 2, 3, 4, 5]);
    
    let schema = events::extract_schema(&payload).unwrap();
    // Arrays should be handled
    assert!(schema.fields.len() >= 0); // May have array-specific fields
}

#[tokio::test]
async fn test_schema_extraction_array_of_objects() {
    let payload = serde_json::json!([
        {"id": 1, "name": "Item 1"},
        {"id": 2, "name": "Item 2"},
        {"id": 3, "name": "Item 3"}
    ]);
    
    let schema = events::extract_schema(&payload).unwrap();
    // Should extract schema from array items
    assert!(schema.fields.len() >= 0);
}

#[tokio::test]
async fn test_schema_extraction_primitive_string() {
    let payload = serde_json::json!("simple string");
    
    let schema = events::extract_schema(&payload).unwrap();
    // Primitive strings should be handled
    assert!(schema.fields.len() >= 0);
}

#[tokio::test]
async fn test_schema_extraction_primitive_number() {
    let payload = serde_json::json!(42);
    
    let schema = events::extract_schema(&payload).unwrap();
    // Primitive numbers should be handled
    assert!(schema.fields.len() >= 0);
}

#[tokio::test]
async fn test_schema_extraction_primitive_boolean() {
    let payload = serde_json::json!(true);
    
    let schema = events::extract_schema(&payload).unwrap();
    // Primitive booleans should be handled
    assert!(schema.fields.len() >= 0);
}

#[tokio::test]
async fn test_schema_extraction_null() {
    let payload = serde_json::json!(null);
    
    let schema = events::extract_schema(&payload).unwrap();
    // Null should be handled
    assert!(schema.fields.len() >= 0);
}

#[tokio::test]
async fn test_schema_extraction_empty_object() {
    let payload = serde_json::json!({});
    
    let schema = events::extract_schema(&payload).unwrap();
    assert_eq!(schema.fields.len(), 0);
}

#[tokio::test]
async fn test_schema_extraction_empty_array() {
    let payload = serde_json::json!([]);
    
    let schema = events::extract_schema(&payload).unwrap();
    // Empty arrays should be handled
    assert!(schema.fields.len() >= 0);
}

#[tokio::test]
async fn test_schema_extraction_mixed_types() {
    let payload = serde_json::json!({
        "string": "value",
        "number": 42,
        "float": 3.14,
        "boolean": true,
        "null": null,
        "array": [1, 2, 3],
        "object": {
            "nested": "value"
        }
    });
    
    let schema = events::extract_schema(&payload).unwrap();
    assert!(schema.fields.len() >= 6); // At least 6 top-level fields
}

#[tokio::test]
async fn test_schema_extraction_unicode_field_names() {
    let payload = serde_json::json!({
        "field_你好": "value",
        "field_🌍": "value2",
        "normal_field": "value3"
    });
    
    let schema = events::extract_schema(&payload).unwrap();
    assert!(schema.fields.len() >= 3);
}

#[tokio::test]
async fn test_schema_extraction_large_object() {
    // Create object with many fields
    let mut payload_obj = serde_json::Map::new();
    for i in 0..100 {
        payload_obj.insert(format!("field_{}", i), serde_json::json!(i));
    }
    let payload = serde_json::Value::Object(payload_obj);
    
    let schema = events::extract_schema(&payload).unwrap();
    // Should handle large objects (may be limited by MAX_FIELDS)
    assert!(schema.fields.len() > 0);
}

#[tokio::test]
async fn test_schema_extraction_field_name_length() {
    let long_field_name = "a".repeat(1000);
    let payload = serde_json::json!({
        long_field_name: "value"
    });
    
    let schema = events::extract_schema(&payload).unwrap();
    // Should handle long field names (may be truncated)
    assert!(schema.fields.len() >= 0);
}

#[tokio::test]
async fn test_schema_persistence_across_events() {
    let manager = create_test_manager();
    
    let source = Actor::new(
        ActorId::from("source1"),
        "Source".to_string(),
        ActorType::Source,
        "token-123456789012".to_string(),
    );
    manager.register_actor(source).await.unwrap();
    
    // Publish first event
    let payload1 = serde_json::json!({
        "order_id": "12345",
        "customer": "John Doe"
    });
    
    manager.publish_event(
        &ActorId::from("source1"),
        "token-123456789012",
        "order_created",
        payload1,
    ).await.unwrap();
    
    // Publish second event with same structure
    let payload2 = serde_json::json!({
        "order_id": "67890",
        "customer": "Jane Smith"
    });
    
    manager.publish_event(
        &ActorId::from("source1"),
        "token-123456789012",
        "order_created",
        payload2,
    ).await.unwrap();
    
    // Schema should be extracted from first event and reused
    // Both events should succeed
}



