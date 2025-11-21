use thiserror::Error;

#[derive(Error, Debug)]
pub enum LLMError {
    #[error("Provider error: {0}")]
    Provider(String),

    #[error("API key not set for provider: {0}")]
    MissingApiKey(String),

    #[error("HTTP request failed: {0}")]
    HttpRequest(#[from] reqwest::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid response from provider: {0}")]
    InvalidResponse(String),

    #[error("Rate limit exceeded")]
    RateLimit,

    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Model not available: {0}")]
    ModelNotAvailable(String),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Brain integration error: {0}")]
    BrainIntegration(String),
}

pub type Result<T> = std::result::Result<T, LLMError>;



