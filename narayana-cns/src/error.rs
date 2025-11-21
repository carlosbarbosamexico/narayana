//! Error types for narayana-cns

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CnsError {
    #[error("Component error: {0}")]
    Component(String),
    
    #[error("Registry error: {0}")]
    Registry(String),
    
    #[error("Capability error: {0}")]
    Capability(String),
    
    #[error("Safety error: {0}")]
    Safety(String),
    
    #[error("Routing error: {0}")]
    Routing(String),
    
    #[error("Transport error: {0}")]
    Transport(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Unknown CNS error: {0}")]
    Unknown(String),
}

impl From<narayana_core::Error> for CnsError {
    fn from(err: narayana_core::Error) -> Self {
        CnsError::Unknown(format!("Core error: {}", err))
    }
}

