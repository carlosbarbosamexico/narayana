// Webhook API - Create, update, delete webhooks through API

use narayana_storage::webhooks::{WebhookScope, WebhookEventType, PayloadFormat, WebhookConfig, WebhookManager, WebhookEvent};
use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Create webhook request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWebhookRequest {
    pub name: String,
    pub url: String,
    pub scope: WebhookScope,
    pub events: Vec<WebhookEventType>,
    pub format: PayloadFormat,
    pub headers: Option<HashMap<String, String>>,
    pub secret: Option<String>,
    pub retry_count: Option<u32>,
    pub timeout_seconds: Option<u64>,
}

/// Update webhook request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWebhookRequest {
    pub name: Option<String>,
    pub url: Option<String>,
    pub scope: Option<WebhookScope>,
    pub events: Option<Vec<WebhookEventType>>,
    pub format: Option<PayloadFormat>,
    pub headers: Option<HashMap<String, String>>,
    pub secret: Option<String>,
    pub enabled: Option<bool>,
    pub retry_count: Option<u32>,
    pub timeout_seconds: Option<u64>,
}

/// Webhook response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookResponse {
    pub id: String,
    pub name: String,
    pub url: String,
    pub scope: WebhookScope,
    pub events: Vec<WebhookEventType>,
    pub format: PayloadFormat,
    pub enabled: bool,
    pub created_at: u64,
    pub updated_at: u64,
}

impl From<WebhookConfig> for WebhookResponse {
    fn from(config: WebhookConfig) -> Self {
        Self {
            id: config.id,
            name: config.name,
            url: config.url,
            scope: config.scope,
            events: config.events,
            format: config.format,
            enabled: config.enabled,
            created_at: config.created_at,
            updated_at: config.updated_at,
        }
    }
}

/// Webhook API client
pub struct WebhookApi {
    manager: Arc<WebhookManager>,
}

impl WebhookApi {
    pub fn new(manager: Arc<WebhookManager>) -> Self {
        Self { manager }
    }

    /// Create webhook
    pub async fn create_webhook(&self, request: CreateWebhookRequest) -> Result<WebhookResponse> {
        let mut config = WebhookConfig::new(
            request.name,
            request.url,
            request.scope,
            request.events,
            request.format,
        );

        if let Some(headers) = request.headers {
            config.headers = headers;
        }

        if let Some(secret) = request.secret {
            config.secret = Some(secret);
        }

        if let Some(retry_count) = request.retry_count {
            config.retry_count = retry_count;
        }

        if let Some(timeout) = request.timeout_seconds {
            config.timeout_seconds = timeout;
        }

        let id = self.manager.create_webhook(config.clone())?;
        Ok(WebhookResponse::from(config))
    }

    /// Update webhook
    pub async fn update_webhook(
        &self,
        id: &str,
        request: UpdateWebhookRequest,
    ) -> Result<WebhookResponse> {
        let mut config = self
            .manager
            .get_webhook(id)
            .ok_or_else(|| Error::Storage(format!("Webhook {} not found", id)))?;

        if let Some(name) = request.name {
            config.name = name;
        }

        if let Some(url) = request.url {
            config.url = url;
        }

        if let Some(scope) = request.scope {
            config.scope = scope;
        }

        if let Some(events) = request.events {
            config.events = events;
        }

        if let Some(format) = request.format {
            config.format = format;
        }

        if let Some(headers) = request.headers {
            config.headers = headers;
        }

        if let Some(secret) = request.secret {
            config.secret = Some(secret);
        }

        if let Some(enabled) = request.enabled {
            config.enabled = enabled;
        }

        if let Some(retry_count) = request.retry_count {
            config.retry_count = retry_count;
        }

        if let Some(timeout) = request.timeout_seconds {
            config.timeout_seconds = timeout;
        }

        self.manager.update_webhook(id, config.clone())?;
        Ok(WebhookResponse::from(config))
    }

    /// Delete webhook
    pub async fn delete_webhook(&self, id: &str) -> Result<()> {
        self.manager.delete_webhook(id)
    }

    /// Get webhook
    pub async fn get_webhook(&self, id: &str) -> Result<WebhookResponse> {
        self.manager
            .get_webhook(id)
            .map(WebhookResponse::from)
            .ok_or_else(|| Error::Storage(format!("Webhook {} not found", id)))
    }

    /// List webhooks
    pub async fn list_webhooks(&self) -> Vec<WebhookResponse> {
        self.manager
            .list_webhooks()
            .into_iter()
            .map(WebhookResponse::from)
            .collect()
    }

    /// List webhooks by scope
    pub async fn list_webhooks_by_scope(&self, scope: &WebhookScope) -> Vec<WebhookResponse> {
        self.manager
            .list_webhooks_by_scope(scope)
            .into_iter()
            .map(WebhookResponse::from)
            .collect()
    }

    /// Enable webhook
    pub async fn enable_webhook(&self, id: &str) -> Result<()> {
        self.manager.enable_webhook(id)
    }

    /// Disable webhook
    pub async fn disable_webhook(&self, id: &str) -> Result<()> {
        self.manager.disable_webhook(id)
    }

    /// Trigger webhook manually (for testing)
    pub async fn trigger_webhook(&self, event: WebhookEvent) -> Result<()> {
        self.manager.trigger_webhook(event).await
    }
}

use std::sync::Arc;

