//! Error types for narayana-me

use narayana_core::Error as CoreError;
use thiserror::Error;

/// Avatar rendering errors
#[derive(Error, Debug)]
pub enum AvatarError {
    #[error("Broker error: {0}")]
    Broker(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Stream error: {0}")]
    Stream(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Core error: {0}")]
    Core(#[from] CoreError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("URL parsing error: {0}")]
    Url(#[from] url::ParseError),
}

impl From<AvatarError> for CoreError {
    fn from(err: AvatarError) -> Self {
        CoreError::Storage(format!("Avatar error: {}", err))
    }
}


