// Tests for webhooks

use narayana_storage::webhooks::*;
use narayana_core::types::TableId;
use std::collections::HashMap;

#[test]
fn test_webhook_config_creation() {
    let config = WebhookConfig::new(
        "test-webhook".to_string(),
        "https://example.com/webhook".to_string(),
        WebhookScope::Global,
        vec![WebhookEventType::Insert, WebhookEventType::Update],
        PayloadFormat::Json,
    );
    
    assert_eq!(config.name, "test-webhook");
    assert_eq!(config.url, "https://example.com/webhook");
    assert!(config.enabled);
}

#[test]
fn test_webhook_should_trigger() {
    let config = WebhookConfig::new(
        "test-webhook".to_string(),
        "https://example.com/webhook".to_string(),
        WebhookScope::Global,
        vec![WebhookEventType::Insert],
        PayloadFormat::Json,
    );
    
    assert!(config.should_trigger(&WebhookEventType::Insert, &WebhookScope::Global));
    assert!(!config.should_trigger(&WebhookEventType::Delete, &WebhookScope::Global));
}

#[test]
fn test_webhook_scope_database() {
    let config = WebhookConfig::new(
        "db-webhook".to_string(),
        "https://example.com/webhook".to_string(),
        WebhookScope::Database { db_name: "test_db".to_string() },
        vec![WebhookEventType::Insert],
        PayloadFormat::Json,
    );
    
    assert!(config.should_trigger(
        &WebhookEventType::Insert,
        &WebhookScope::Database { db_name: "test_db".to_string() }
    ));
    
    assert!(config.should_trigger(
        &WebhookEventType::Insert,
        &WebhookScope::Table { db_name: "test_db".to_string(), table_name: "users".to_string() }
    ));
    
    assert!(!config.should_trigger(
        &WebhookEventType::Insert,
        &WebhookScope::Database { db_name: "other_db".to_string() }
    ));
}

#[test]
fn test_webhook_scope_table() {
    let config = WebhookConfig::new(
        "table-webhook".to_string(),
        "https://example.com/webhook".to_string(),
        WebhookScope::Table { db_name: "test_db".to_string(), table_name: "users".to_string() },
        vec![WebhookEventType::Insert],
        PayloadFormat::Json,
    );
    
    assert!(config.should_trigger(
        &WebhookEventType::Insert,
        &WebhookScope::Table { db_name: "test_db".to_string(), table_name: "users".to_string() }
    ));
    
    assert!(config.should_trigger(
        &WebhookEventType::Insert,
        &WebhookScope::Row { db_name: "test_db".to_string(), table_name: "users".to_string(), row_id: 1 }
    ));
    
    assert!(!config.should_trigger(
        &WebhookEventType::Insert,
        &WebhookScope::Table { db_name: "test_db".to_string(), table_name: "posts".to_string() }
    ));
}

#[test]
fn test_webhook_scope_column() {
    let config = WebhookConfig::new(
        "column-webhook".to_string(),
        "https://example.com/webhook".to_string(),
        WebhookScope::Column {
            db_name: "test_db".to_string(),
            table_name: "users".to_string(),
            column_name: "email".to_string(),
        },
        vec![WebhookEventType::Update],
        PayloadFormat::Json,
    );
    
    assert!(config.should_trigger(
        &WebhookEventType::Update,
        &WebhookScope::Column {
            db_name: "test_db".to_string(),
            table_name: "users".to_string(),
            column_name: "email".to_string(),
        }
    ));
    
    assert!(!config.should_trigger(
        &WebhookEventType::Update,
        &WebhookScope::Column {
            db_name: "test_db".to_string(),
            table_name: "users".to_string(),
            column_name: "name".to_string(),
        }
    ));
}

#[test]
fn test_webhook_payload_builder_json() {
    let payload = WebhookPayloadBuilder::new(PayloadFormat::Json)
        .add_event_type(&WebhookEventType::Insert)
        .add_timestamp()
        .add_data(serde_json::json!({"id": 1, "name": "test"}))
        .build()
        .unwrap();
    
    assert!(payload.contains("event_type"));
    assert!(payload.contains("timestamp"));
    assert!(payload.contains("data"));
}

#[test]
fn test_webhook_payload_builder_toml() {
    let payload = WebhookPayloadBuilder::new(PayloadFormat::Toml)
        .add_event_type(&WebhookEventType::Insert)
        .add_timestamp()
        .add_data(serde_json::json!({"id": 1}))
        .build()
        .unwrap();
    
    assert!(payload.contains("event_type"));
    assert!(payload.contains("timestamp"));
}

#[test]
fn test_webhook_payload_builder_custom() {
    let template = "Event: {{{event_type}}} at {{{timestamp}}} with data: {{{data}}}".to_string();
    let payload = WebhookPayloadBuilder::new(PayloadFormat::Custom { template })
        .add_event_type(&WebhookEventType::Insert)
        .add_timestamp()
        .add_data(serde_json::json!("test"))
        .build()
        .unwrap();
    
    assert!(payload.contains("Event:"));
    assert!(payload.contains("Insert"));
}

#[test]
fn test_webhook_manager_creation() {
    let manager = WebhookManager::new();
    // Should create successfully
}

#[test]
fn test_webhook_manager_create() {
    let manager = WebhookManager::new();
    let config = WebhookConfig::new(
        "test-webhook".to_string(),
        "https://example.com/webhook".to_string(),
        WebhookScope::Global,
        vec![WebhookEventType::Insert],
        PayloadFormat::Json,
    );
    
    let id = manager.create_webhook(config).unwrap();
    assert!(!id.is_empty());
}

#[test]
fn test_webhook_manager_get() {
    let manager = WebhookManager::new();
    let config = WebhookConfig::new(
        "test-webhook".to_string(),
        "https://example.com/webhook".to_string(),
        WebhookScope::Global,
        vec![WebhookEventType::Insert],
        PayloadFormat::Json,
    );
    
    let id = manager.create_webhook(config.clone()).unwrap();
    let retrieved = manager.get_webhook(&id).unwrap();
    assert_eq!(retrieved.name, config.name);
}

#[test]
fn test_webhook_manager_list() {
    let manager = WebhookManager::new();
    
    let config1 = WebhookConfig::new(
        "webhook-1".to_string(),
        "https://example.com/webhook1".to_string(),
        WebhookScope::Global,
        vec![WebhookEventType::Insert],
        PayloadFormat::Json,
    );
    
    let config2 = WebhookConfig::new(
        "webhook-2".to_string(),
        "https://example.com/webhook2".to_string(),
        WebhookScope::Global,
        vec![WebhookEventType::Update],
        PayloadFormat::Toml,
    );
    
    manager.create_webhook(config1).unwrap();
    manager.create_webhook(config2).unwrap();
    
    let webhooks = manager.list_webhooks();
    assert_eq!(webhooks.len(), 2);
}

#[test]
fn test_webhook_manager_delete() {
    let manager = WebhookManager::new();
    let config = WebhookConfig::new(
        "test-webhook".to_string(),
        "https://example.com/webhook".to_string(),
        WebhookScope::Global,
        vec![WebhookEventType::Insert],
        PayloadFormat::Json,
    );
    
    let id = manager.create_webhook(config).unwrap();
    manager.delete_webhook(&id).unwrap();
    
    assert!(manager.get_webhook(&id).is_none());
}

#[test]
fn test_webhook_manager_enable_disable() {
    let manager = WebhookManager::new();
    let config = WebhookConfig::new(
        "test-webhook".to_string(),
        "https://example.com/webhook".to_string(),
        WebhookScope::Global,
        vec![WebhookEventType::Insert],
        PayloadFormat::Json,
    );
    
    let id = manager.create_webhook(config).unwrap();
    
    manager.disable_webhook(&id).unwrap();
    let webhook = manager.get_webhook(&id).unwrap();
    assert!(!webhook.enabled);
    
    manager.enable_webhook(&id).unwrap();
    let webhook = manager.get_webhook(&id).unwrap();
    assert!(webhook.enabled);
}

#[test]
fn test_webhook_manager_list_by_scope() {
    let manager = WebhookManager::new();
    
    let config1 = WebhookConfig::new(
        "db-webhook".to_string(),
        "https://example.com/webhook1".to_string(),
        WebhookScope::Database { db_name: "test_db".to_string() },
        vec![WebhookEventType::Insert],
        PayloadFormat::Json,
    );
    
    let config2 = WebhookConfig::new(
        "other-webhook".to_string(),
        "https://example.com/webhook2".to_string(),
        WebhookScope::Database { db_name: "other_db".to_string() },
        vec![WebhookEventType::Insert],
        PayloadFormat::Json,
    );
    
    manager.create_webhook(config1).unwrap();
    manager.create_webhook(config2).unwrap();
    
    let scope = WebhookScope::Database { db_name: "test_db".to_string() };
    let webhooks = manager.list_webhooks_by_scope(&scope);
    assert_eq!(webhooks.len(), 1);
}

#[tokio::test]
async fn test_webhook_api_create() {
    use narayana_api::webhooks::*;
    use std::sync::Arc;
    
    let manager = Arc::new(WebhookManager::new());
    let api = WebhookApi::new(manager);
    
    let request = CreateWebhookRequest {
        name: "test-webhook".to_string(),
        url: "https://example.com/webhook".to_string(),
        scope: WebhookScope::Global,
        events: vec![WebhookEventType::Insert],
        format: PayloadFormat::Json,
        headers: None,
        secret: None,
        retry_count: None,
        timeout_seconds: None,
    };
    
    let response = api.create_webhook(request).await.unwrap();
    assert_eq!(response.name, "test-webhook");
}

#[tokio::test]
async fn test_webhook_api_update() {
    use narayana_api::webhooks::*;
    use std::sync::Arc;
    
    let manager = Arc::new(WebhookManager::new());
    let api = WebhookApi::new(manager);
    
    let create_request = CreateWebhookRequest {
        name: "test-webhook".to_string(),
        url: "https://example.com/webhook".to_string(),
        scope: WebhookScope::Global,
        events: vec![WebhookEventType::Insert],
        format: PayloadFormat::Json,
        headers: None,
        secret: None,
        retry_count: None,
        timeout_seconds: None,
    };
    
    let created = api.create_webhook(create_request).await.unwrap();
    
    let update_request = UpdateWebhookRequest {
        name: Some("updated-webhook".to_string()),
        url: None,
        scope: None,
        events: None,
        format: None,
        headers: None,
        secret: None,
        enabled: None,
        retry_count: None,
        timeout_seconds: None,
    };
    
    let updated = api.update_webhook(&created.id, update_request).await.unwrap();
    assert_eq!(updated.name, "updated-webhook");
}

