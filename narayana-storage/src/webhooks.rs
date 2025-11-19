// Native Webhooks - Set up for database, column, row, record, etc.
// Payload format: JSON, TOML, or fully customized

use narayana_core::{Error, Result, types::{TableId, ColumnId}};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::broadcast;
use reqwest::Client;
use tracing::{info, warn, error};
use crate::security_limits::{*, validate_string_length, validate_collection_size};

/// Webhook event types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebhookEventType {
    Insert,
    Update,
    Delete,
    Create,      // Table/database creation
    Drop,        // Table/database drop
    Alter,       // Schema alteration
    Query,       // Query execution
    Transaction, // Transaction commit/rollback
    Custom(String), // Custom event type
}

/// Webhook scope (what triggers the webhook)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebhookScope {
    Database { db_name: String },
    Table { db_name: String, table_name: String },
    Column { db_name: String, table_name: String, column_name: String },
    Row { db_name: String, table_name: String, row_id: u64 },
    Record { db_name: String, table_name: String, record_id: String },
    Global, // All events
}

/// Payload format
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PayloadFormat {
    Json,
    Toml,
    Custom { template: String }, // Custom template with placeholders
}

/// Webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub id: String,
    pub name: String,
    pub url: String,
    pub scope: WebhookScope,
    pub events: Vec<WebhookEventType>,
    pub format: PayloadFormat,
    pub headers: HashMap<String, String>,
    pub secret: Option<String>, // For HMAC signature
    pub enabled: bool,
    pub retry_count: u32,
    pub timeout_seconds: u64,
    pub created_at: u64,
    pub updated_at: u64,
}

impl WebhookConfig {
    pub fn new(
        name: String,
        url: String,
        scope: WebhookScope,
        events: Vec<WebhookEventType>,
        format: PayloadFormat,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            url,
            scope,
            events,
            format,
            headers: HashMap::new(),
            secret: None,
            enabled: true,
            retry_count: 3,
            timeout_seconds: 30,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            updated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Check if webhook should trigger for this event
    pub fn should_trigger(&self, event_type: &WebhookEventType, scope: &WebhookScope) -> bool {
        if !self.enabled {
            return false;
        }

        // Check if event type matches
        if !self.events.contains(event_type) {
            return false;
        }

        // Check if scope matches
        match (&self.scope, scope) {
            (WebhookScope::Global, _) => true,
            (WebhookScope::Database { db_name: db1 }, WebhookScope::Database { db_name: db2 }) => {
                db1 == db2
            }
            (WebhookScope::Table { db_name: db1, table_name: t1 }, 
             WebhookScope::Table { db_name: db2, table_name: t2 }) => {
                db1 == db2 && t1 == t2
            }
            (WebhookScope::Column { db_name: db1, table_name: t1, column_name: c1 },
             WebhookScope::Column { db_name: db2, table_name: t2, column_name: c2 }) => {
                db1 == db2 && t1 == t2 && c1 == c2
            }
            (WebhookScope::Row { db_name: db1, table_name: t1, row_id: r1 },
             WebhookScope::Row { db_name: db2, table_name: t2, row_id: r2 }) => {
                db1 == db2 && t1 == t2 && r1 == r2
            }
            (WebhookScope::Record { db_name: db1, table_name: t1, record_id: rec1 },
             WebhookScope::Record { db_name: db2, table_name: t2, record_id: rec2 }) => {
                db1 == db2 && t1 == t2 && rec1 == rec2
            }
            // Database-level webhook triggers for all tables in that database
            (WebhookScope::Database { db_name: db1 }, WebhookScope::Table { db_name: db2, .. }) => {
                db1 == db2
            }
            (WebhookScope::Database { db_name: db1 }, WebhookScope::Column { db_name: db2, .. }) => {
                db1 == db2
            }
            (WebhookScope::Database { db_name: db1 }, WebhookScope::Row { db_name: db2, .. }) => {
                db1 == db2
            }
            (WebhookScope::Database { db_name: db1 }, WebhookScope::Record { db_name: db2, .. }) => {
                db1 == db2
            }
            // Table-level webhook triggers for all columns/rows in that table
            (WebhookScope::Table { db_name: db1, table_name: t1 },
             WebhookScope::Column { db_name: db2, table_name: t2, .. }) => {
                db1 == db2 && t1 == t2
            }
            (WebhookScope::Table { db_name: db1, table_name: t1 },
             WebhookScope::Row { db_name: db2, table_name: t2, .. }) => {
                db1 == db2 && t1 == t2
            }
            (WebhookScope::Table { db_name: db1, table_name: t1 },
             WebhookScope::Record { db_name: db2, table_name: t2, .. }) => {
                db1 == db2 && t1 == t2
            }
            _ => false,
        }
    }
}

/// Webhook payload builder
pub struct WebhookPayloadBuilder {
    format: PayloadFormat,
    data: HashMap<String, JsonValue>,
}

impl WebhookPayloadBuilder {
    pub fn new(format: PayloadFormat) -> Self {
        Self {
            format,
            data: HashMap::new(),
        }
    }

    pub fn add_field(mut self, key: &str, value: JsonValue) -> Self {
        self.data.insert(key.to_string(), value);
        self
    }

    pub fn add_event_type(mut self, event_type: &WebhookEventType) -> Self {
        self.data.insert(
            "event_type".to_string(),
            serde_json::to_value(event_type)
                .unwrap_or_else(|_| serde_json::Value::String(format!("{:?}", event_type))),
        );
        self
    }

    pub fn add_timestamp(mut self) -> Self {
        self.data.insert(
            "timestamp".to_string(),
            JsonValue::Number(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
                    .into(),
            ),
        );
        self
    }

    pub fn add_data(mut self, data: JsonValue) -> Self {
        self.data.insert("data".to_string(), data);
        self
    }

    /// Build payload string based on format
    pub fn build(self) -> Result<String> {
        match &self.format {
            PayloadFormat::Json => {
                serde_json::to_string(&self.data)
                    .map_err(|e| Error::Serialization(format!("Failed to serialize JSON: {}", e)))
            }
            PayloadFormat::Toml => {
                toml::to_string(&self.data)
                    .map_err(|e| Error::Serialization(format!("Failed to serialize TOML: {}", e)))
            }
            PayloadFormat::Custom { template } => {
                // SECURITY: Validate template size to prevent DoS
                if template.len() > 1_000_000 {
                    return Err(Error::Storage("Template size exceeds maximum (1MB)".to_string()));
                }
                
                let mut payload = template.clone();
                for (key, value) in &self.data {
                    // SECURITY: Validate key to prevent injection
                    if key.contains("{{") || key.contains("}}") {
                        return Err(Error::Storage(format!(
                            "Invalid placeholder key '{}': cannot contain placeholder syntax",
                            key
                        )));
                    }
                    
                    let placeholder = format!("{{{{{}}}}}", key);
                    let value_str = match value {
                        JsonValue::String(s) => {
                            // SECURITY: Limit string length to prevent DoS
                            if s.len() > 100_000 {
                                return Err(Error::Storage(format!(
                                    "Value string length {} exceeds maximum (100KB)",
                                    s.len()
                                )));
                            }
                            s.clone()
                        }
                        JsonValue::Number(n) => n.to_string(),
                        JsonValue::Bool(b) => b.to_string(),
                        JsonValue::Null => "null".to_string(),
                        _ => {
                            // SECURITY: Limit JSON serialization size
                            let json_str = serde_json::to_string(value)
                                .unwrap_or_else(|_| "null".to_string());
                            if json_str.len() > 100_000 {
                                return Err(Error::Storage(format!(
                                    "Value JSON size {} exceeds maximum (100KB)",
                                    json_str.len()
                                )));
                            }
                            json_str
                        }
                    };
                    payload = payload.replace(&placeholder, &value_str);
                    
                    // SECURITY: Prevent payload explosion
                    if payload.len() > 10_000_000 {
                        return Err(Error::Storage("Payload size exceeds maximum (10MB)".to_string()));
                    }
                }
                Ok(payload)
            }
        }
    }
}

/// Webhook event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    pub event_type: WebhookEventType,
    pub scope: WebhookScope,
    pub data: JsonValue,
    pub timestamp: u64,
}

/// Webhook manager
pub struct WebhookManager {
    webhooks: Arc<RwLock<HashMap<String, WebhookConfig>>>,
    scoped_webhooks: Arc<RwLock<HashMap<String, Vec<String>>>>, // scope -> webhook_ids
    client: Client,
    event_sender: broadcast::Sender<WebhookEvent>,
}

impl WebhookManager {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self {
            webhooks: Arc::new(RwLock::new(HashMap::new())),
            scoped_webhooks: Arc::new(RwLock::new(HashMap::new())),
            client: Client::new(),
            event_sender: sender,
        }
    }

    /// Create a new webhook
    /// SECURITY: Added resource limits to prevent DoS
    pub fn create_webhook(&self, config: WebhookConfig) -> Result<String> {
        // SECURITY: Validate input sizes
        validate_string_length(&config.name, MAX_WEBHOOK_NAME_LENGTH, "Webhook name")?;
        validate_string_length(&config.url, MAX_WEBHOOK_URL_LENGTH, "Webhook URL")?;
        validate_collection_size(&config.events, 100, "Webhook events")?;
        validate_collection_size(&config.headers.keys().collect::<Vec<_>>(), MAX_WEBHOOK_HEADERS, "Webhook headers")?;
        
        // Validate header key/value lengths
        for (key, value) in &config.headers {
            validate_string_length(key, MAX_WEBHOOK_HEADER_KEY_LENGTH, "Webhook header key")?;
            validate_string_length(value, MAX_WEBHOOK_HEADER_VALUE_LENGTH, "Webhook header value")?;
        }
        
        // SECURITY: Check global webhook limit
        let webhooks = self.webhooks.read();
        if webhooks.len() >= MAX_WEBHOOKS_GLOBAL {
            return Err(Error::Storage(format!(
                "Maximum number of webhooks ({}) exceeded",
                MAX_WEBHOOKS_GLOBAL
            )));
        }
        drop(webhooks);
        
        // SECURITY: Check scope-specific limit
        let scope_key = format!("{:?}", config.scope);
        let mut scoped_webhooks = self.scoped_webhooks.write();
        let scope_list = scoped_webhooks.entry(scope_key.clone()).or_insert_with(Vec::new);
        
        if scope_list.len() >= MAX_WEBHOOKS_PER_SCOPE {
            return Err(Error::Storage(format!(
                "Maximum number of webhooks per scope ({}) exceeded",
                MAX_WEBHOOKS_PER_SCOPE
            )));
        }
        
        let id = config.id.clone();
        let mut webhooks = self.webhooks.write();
        
        if webhooks.contains_key(&id) {
            return Err(Error::Storage(format!("Webhook {} already exists", id)));
        }
        
        webhooks.insert(id.clone(), config);
        scope_list.push(id.clone());
        info!("Created webhook: {}", id);
        Ok(id)
    }

    /// Update webhook
    pub fn update_webhook(&self, id: &str, config: WebhookConfig) -> Result<()> {
        let mut webhooks = self.webhooks.write();
        
        if !webhooks.contains_key(id) {
            return Err(Error::Storage(format!("Webhook {} not found", id)));
        }
        
        let mut updated_config = config;
        updated_config.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        webhooks.insert(id.to_string(), updated_config);
        info!("Updated webhook: {}", id);
        Ok(())
    }

    /// Delete webhook
    pub fn delete_webhook(&self, id: &str) -> Result<()> {
        let mut webhooks = self.webhooks.write();
        
        if webhooks.remove(id).is_none() {
            return Err(Error::Storage(format!("Webhook {} not found", id)));
        }
        
        info!("Deleted webhook: {}", id);
        Ok(())
    }

    /// Get webhook
    pub fn get_webhook(&self, id: &str) -> Option<WebhookConfig> {
        self.webhooks.read().get(id).cloned()
    }

    /// List all webhooks
    pub fn list_webhooks(&self) -> Vec<WebhookConfig> {
        self.webhooks.read().values().cloned().collect()
    }

    /// List webhooks by scope
    pub fn list_webhooks_by_scope(&self, scope: &WebhookScope) -> Vec<WebhookConfig> {
        self.webhooks
            .read()
            .values()
            .filter(|webhook| {
                match (&webhook.scope, scope) {
                    (WebhookScope::Global, _) => true,
                    (WebhookScope::Database { db_name: db1 }, WebhookScope::Database { db_name: db2 }) => {
                        db1 == db2
                    }
                    (WebhookScope::Table { db_name: db1, table_name: t1 },
                     WebhookScope::Table { db_name: db2, table_name: t2 }) => {
                        db1 == db2 && t1 == t2
                    }
                    _ => false,
                }
            })
            .cloned()
            .collect()
    }

    /// Trigger webhook for an event
    pub async fn trigger_webhook(&self, event: WebhookEvent) -> Result<()> {
        let webhooks = self.webhooks.read();
        let matching_webhooks: Vec<_> = webhooks
            .values()
            .filter(|webhook| webhook.should_trigger(&event.event_type, &event.scope))
            .cloned()
            .collect();
        drop(webhooks);

        // Trigger all matching webhooks in parallel
        let mut handles = Vec::new();
        for webhook in matching_webhooks {
            let client = self.client.clone();
            let event_clone = event.clone();
            handles.push(tokio::spawn(async move {
                Self::send_webhook(client, webhook, event_clone).await
            }));
        }

        // Wait for all webhooks to complete (or fail)
        for handle in handles {
            if let Err(e) = handle.await {
                warn!("Webhook task error: {}", e);
            }
        }

        Ok(())
    }

    /// Validate webhook URL to prevent SSRF attacks
    fn validate_webhook_url(url: &str) -> Result<()> {
        use crate::security_utils::SecurityUtils;
        SecurityUtils::validate_http_url(url)
    }

    /// Send webhook HTTP request
    async fn send_webhook(
        client: Client,
        webhook: WebhookConfig,
        event: WebhookEvent,
    ) -> Result<()> {
        // SECURITY: Validate URL to prevent SSRF attacks
        Self::validate_webhook_url(&webhook.url)?;
        
        // Build payload
        let payload = WebhookPayloadBuilder::new(webhook.format.clone())
            .add_event_type(&event.event_type)
            .add_timestamp()
            .add_data(event.data)
            .build()?;

        // Clone payload for HMAC signature calculation
        let payload_for_signature = payload.clone();

        // Build request
        let mut request = client
            .post(&webhook.url)
            .timeout(std::time::Duration::from_secs(webhook.timeout_seconds))
            .body(payload);

        // SECURITY: Add headers with validation to prevent header injection
        for (key, value) in &webhook.headers {
            // SECURITY: Validate header names/values to prevent injection
            if key.contains('\r') || key.contains('\n') || value.contains('\r') || value.contains('\n') {
                return Err(Error::Storage(format!(
                    "Invalid header: contains CR/LF characters in key '{}' or value",
                    key
                )));
            }
            
            // SECURITY: Additional validation for dangerous header names
            let key_lower = key.to_lowercase();
            let dangerous_headers = ["host", "content-length", "transfer-encoding", "connection", "upgrade"];
            if dangerous_headers.contains(&key_lower.as_str()) {
                return Err(Error::Storage(format!(
                    "Header '{}' cannot be set - reserved for HTTP protocol",
                    key
                )));
            }
            
            request = request.header(key, value);
        }

        // Add content-type based on format
        match webhook.format {
            PayloadFormat::Json => {
                request = request.header("Content-Type", "application/json");
            }
            PayloadFormat::Toml => {
                request = request.header("Content-Type", "application/toml");
            }
            PayloadFormat::Custom { .. } => {
                request = request.header("Content-Type", "text/plain");
            }
        }

        // Add HMAC signature if secret is provided
        if let Some(secret) = &webhook.secret {
            use hmac::{Hmac, Mac};
            use sha2::Sha256;
            type HmacSha256 = Hmac<Sha256>;
            
            let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
                .map_err(|e| Error::Storage(format!("Invalid secret: {}", e)))?;
            mac.update(payload_for_signature.as_bytes());
            let signature = hex::encode(mac.finalize().into_bytes());
            request = request.header("X-Narayana-Signature", signature);
        }

        // Send request with retries
        let mut last_error = None;
        for attempt in 0..=webhook.retry_count {
            match request.try_clone() {
                Some(req) => {
                    match req.send().await {
                        Ok(response) => {
                            if response.status().is_success() {
                                info!("Webhook {} sent successfully", webhook.id);
                                return Ok(());
                            } else {
                                // SECURITY: Don't expose full response body (could contain sensitive info)
                                let status_code = response.status().as_u16();
                                last_error = Some(format!(
                                    "HTTP {}: Request failed",
                                    status_code
                                ));
                            }
                        }
                        Err(e) => {
                            last_error = Some(format!("Request error: {}", e));
                        }
                    }
                }
                None => {
                    last_error = Some("Request cannot be cloned".to_string());
                    break;
                }
            }

            if attempt < webhook.retry_count {
                tokio::time::sleep(std::time::Duration::from_millis(100 * (attempt + 1) as u64)).await;
            }
        }

        error!("Webhook {} failed after {} retries: {:?}", webhook.id, webhook.retry_count, last_error);
        Err(Error::Storage(format!(
            "Webhook failed: {:?}",
            last_error
        )))
    }

    /// Enable webhook
    pub fn enable_webhook(&self, id: &str) -> Result<()> {
        let mut webhooks = self.webhooks.write();
        if let Some(webhook) = webhooks.get_mut(id) {
            webhook.enabled = true;
            webhook.updated_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            Ok(())
        } else {
            Err(Error::Storage(format!("Webhook {} not found", id)))
        }
    }

    /// Disable webhook
    pub fn disable_webhook(&self, id: &str) -> Result<()> {
        let mut webhooks = self.webhooks.write();
        if let Some(webhook) = webhooks.get_mut(id) {
            webhook.enabled = false;
            webhook.updated_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            Ok(())
        } else {
            Err(Error::Storage(format!("Webhook {} not found", id)))
        }
    }
}

impl Default for WebhookManager {
    fn default() -> Self {
        Self::new()
    }
}

