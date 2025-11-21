//! Error types for narayana-spk

use narayana_core::Error as CoreError;
use thiserror::Error;

/// Speech synthesis errors
#[derive(Error, Debug)]
pub enum SpeechError {
    #[error("Synthesizer error: {0}")]
    Synthesizer(String),

    #[error("Engine error: {0}")]
    Engine(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Core error: {0}")]
    Core(#[from] CoreError),
}

impl From<SpeechError> for CoreError {
    fn from(err: SpeechError) -> Self {
        CoreError::Storage(format!("Speech error: {}", err))
    }
}


