//! Transport layer abstraction

use crate::error::CnsError;
use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    /// Transport type
    pub transport_type: TransportType,
    /// Transport-specific configuration
    pub config: HashMap<String, JsonValue>,
}

/// Transport type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransportType {
    /// HTTP transport
    Http,
    /// WebSocket transport
    WebSocket,
    /// MQTT transport
    Mqtt,
    /// Serial port transport
    Serial,
    /// I2C transport
    I2C,
    /// SPI transport
    SPI,
    /// CAN bus transport
    Can,
    /// Modbus transport
    Modbus,
    /// Custom transport
    Custom(String),
}

/// Transport trait for pluggable communication
#[async_trait]
pub trait Transport: Send + Sync {
    /// Get transport type
    fn transport_type(&self) -> TransportType;
    
    /// Connect to component
    async fn connect(&mut self, config: &TransportConfig) -> Result<(), CnsError>;
    
    /// Disconnect from component
    async fn disconnect(&mut self) -> Result<(), CnsError>;
    
    /// Send data to component
    async fn send(&mut self, data: &Bytes) -> Result<(), CnsError>;
    
    /// Receive data from component
    async fn receive(&mut self) -> Result<Option<Bytes>, CnsError>;
    
    /// Check if transport is connected
    fn is_connected(&self) -> bool;
}

/// Transport registry for managing transports
pub struct TransportRegistry {
    transports: HashMap<TransportType, Box<dyn Transport>>,
}

impl TransportRegistry {
    /// Create new transport registry
    pub fn new() -> Self {
        Self {
            transports: HashMap::new(),
        }
    }
    
    /// Register a transport
    pub fn register(&mut self, transport: Box<dyn Transport>) {
        let transport_type = transport.transport_type();
        self.transports.insert(transport_type, transport);
    }
    
    /// Get transport by type
    pub fn get(&self, transport_type: TransportType) -> Option<&dyn Transport> {
        self.transports.get(&transport_type).map(|t| t.as_ref())
    }
    
    /// Get mutable transport by type
    pub fn get_mut(&mut self, transport_type: TransportType) -> Option<&mut Box<dyn Transport>> {
        self.transports.get_mut(&transport_type)
    }
}

impl Default for TransportRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Placeholder transport implementations
// Full implementations would go in separate modules

/// HTTP transport (placeholder)
pub struct HttpTransport {
    connected: bool,
}

#[async_trait]
impl Transport for HttpTransport {
    fn transport_type(&self) -> TransportType {
        TransportType::Http
    }
    
    async fn connect(&mut self, _config: &TransportConfig) -> Result<(), CnsError> {
        self.connected = true;
        Ok(())
    }
    
    async fn disconnect(&mut self) -> Result<(), CnsError> {
        self.connected = false;
        Ok(())
    }
    
    async fn send(&mut self, _data: &Bytes) -> Result<(), CnsError> {
        if !self.connected {
            return Err(CnsError::Transport("Not connected".to_string()));
        }
        // Placeholder - would use reqwest in full implementation
        Ok(())
    }
    
    async fn receive(&mut self) -> Result<Option<Bytes>, CnsError> {
        if !self.connected {
            return Err(CnsError::Transport("Not connected".to_string()));
        }
        // Placeholder
        Ok(None)
    }
    
    fn is_connected(&self) -> bool {
        self.connected
    }
}

impl HttpTransport {
    pub fn new() -> Self {
        Self { connected: false }
    }
}

