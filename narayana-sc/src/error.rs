//! Error types for narayana-sc

use narayana_core::Error as CoreError;
use thiserror::Error;

/// Audio capture and analysis errors
#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Audio capture error: {0}")]
    Capture(String),

    #[error("Audio analysis error: {0}")]
    Analysis(String),

    #[error("Audio device error: {0}")]
    Device(String),

    #[error("Audio format error: {0}")]
    Format(String),

    #[error("LLM integration error: {0}")]
    Llm(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Core error: {0}")]
    Core(#[from] CoreError),
}

impl From<AudioError> for CoreError {
    fn from(err: AudioError) -> Self {
        CoreError::Storage(format!("Audio error: {}", err))
    }
}

