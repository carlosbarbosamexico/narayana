//! Configuration for speech synthesis

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Speech synthesis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SpeechConfig {
    /// Enable speech synthesis (off by default)
    pub enabled: bool,

    /// Preferred TTS engine
    pub engine: TtsEngine,

    /// Voice settings
    pub voice: VoiceConfig,

    /// Speech rate (words per minute, 0-500, default 150)
    pub rate: u32,

    /// Volume (0.0-1.0, default 0.8)
    pub volume: f32,

    /// Pitch adjustment (-1.0 to 1.0, default 0.0)
    pub pitch: f32,

    /// API configuration (if using API-based TTS)
    pub api_config: Option<ApiTtsConfig>,

    /// Cache directory for synthesized audio
    pub cache_dir: PathBuf,

    /// Enable audio caching
    pub enable_cache: bool,

    /// Maximum cache size in MB
    pub max_cache_size_mb: u64,

    /// Queue size for speech requests
    pub queue_size: usize,
}

/// TTS Engine type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TtsEngine {
    /// Native platform TTS (macOS NSSpeechSynthesizer, Linux espeak, Windows SAPI)
    Native,
    /// OpenAI TTS API
    OpenAi,
    /// Google Cloud TTS
    GoogleCloud,
    /// Amazon Polly
    AmazonPolly,
    /// Piper TTS (local neural TTS)
    Piper,
    /// Custom engine
    Custom(String),
}

/// Voice configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VoiceConfig {
    /// Voice name/identifier
    pub name: Option<String>,

    /// Language code (e.g., "en-US", "es-ES")
    pub language: String,

    /// Gender preference
    pub gender: Option<VoiceGender>,

    /// Voice age (if applicable)
    pub age: Option<VoiceAge>,
}

/// Voice gender
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VoiceGender {
    Male,
    Female,
    Neutral,
}

/// Voice age category
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VoiceAge {
    Child,
    Young,
    Adult,
    Elderly,
}

/// API TTS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiTtsConfig {
    /// API endpoint URL
    pub endpoint: String,

    /// API key (optional, can be set via environment)
    pub api_key: Option<String>,

    /// Model/voice ID
    pub model: Option<String>,

    /// Request timeout in seconds
    pub timeout_secs: u64,

    /// Retry configuration
    pub retry_config: RetryConfig,
}

/// Retry configuration for API calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retry attempts
    pub max_retries: u32,

    /// Initial retry delay in milliseconds
    pub initial_delay_ms: u64,

    /// Maximum retry delay in milliseconds
    pub max_delay_ms: u64,
}

impl Default for SpeechConfig {
    fn default() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
            .join("narayana-spk");

        Self {
            enabled: false, // Off by default
            engine: TtsEngine::Native,
            voice: VoiceConfig::default(),
            rate: 150,
            volume: 0.8,
            pitch: 0.0,
            api_config: None,
            cache_dir,
            enable_cache: true,
            max_cache_size_mb: 100,
            queue_size: 100,
        }
    }
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            name: None,
            language: "en-US".to_string(),
            gender: None,
            age: None,
        }
    }
}

impl VoiceConfig {
    /// Validate voice configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate language code format (ISO 639-1 + ISO 3166-1, e.g., "en-US")
        if self.language.is_empty() {
            return Err("Language code cannot be empty".to_string());
        }

        if self.language.len() > 32 {
            return Err("Language code too long (max 32 chars)".to_string());
        }

        // Basic format check: should be like "en-US" or "en"
        if !self.language.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err("Language code contains invalid characters (only alphanumeric and '-' allowed)".to_string());
        }

        // Validate voice name if provided
        if let Some(ref name) = self.name {
            if name.is_empty() {
                return Err("Voice name cannot be empty if provided".to_string());
            }

            if name.len() > 256 {
                return Err("Voice name too long (max 256 chars)".to_string());
            }

            // Check for null bytes and control characters
            if name.chars().any(|c| c == '\0' || c.is_control()) {
                return Err("Voice name contains invalid characters".to_string());
            }
        }

        Ok(())
    }
}

impl RetryConfig {
    /// Validate retry configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.max_retries > 100 {
            return Err("Max retries too large (max 100)".to_string());
        }

        if self.initial_delay_ms > 60_000 {
            return Err("Initial delay too large (max 60000 ms)".to_string());
        }

        if self.max_delay_ms > 300_000 {
            return Err("Max delay too large (max 300000 ms)".to_string());
        }

        if self.initial_delay_ms > self.max_delay_ms {
            return Err("Initial delay cannot be greater than max delay".to_string());
        }

        Ok(())
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
        }
    }
}

impl SpeechConfig {
    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.rate > 500 {
            return Err("Speech rate must be between 0 and 500 WPM".to_string());
        }

        if !(0.0..=1.0).contains(&self.volume) {
            return Err("Volume must be between 0.0 and 1.0".to_string());
        }

        if !(-1.0..=1.0).contains(&self.pitch) {
            return Err("Pitch must be between -1.0 and 1.0".to_string());
        }

        if self.queue_size == 0 {
            return Err("Queue size must be greater than 0".to_string());
        }

        if self.queue_size > 10000 {
            return Err("Queue size too large (max 10000)".to_string());
        }

        // Validate cache directory path (prevent path traversal)
        if self.cache_dir.to_string_lossy().contains("..") {
            return Err("Cache directory path cannot contain '..'".to_string());
        }

        // Limit cache size to prevent resource exhaustion
        const MAX_CACHE_SIZE_MB: u64 = 10_000; // 10GB max
        if self.max_cache_size_mb > MAX_CACHE_SIZE_MB {
            return Err(format!("Cache size too large (max {} MB)", MAX_CACHE_SIZE_MB));
        }

        // Validate voice config
        self.voice.validate()?;

        if let Some(api_config) = &self.api_config {
            if api_config.endpoint.is_empty() {
                return Err("API endpoint cannot be empty".to_string());
            }

            // Validate URL format more strictly
            if !api_config.endpoint.starts_with("https://") {
                return Err("API endpoint must use HTTPS".to_string());
            }

            // Check URL length
            if api_config.endpoint.len() > 2048 {
                return Err("API endpoint URL too long (max 2048 chars)".to_string());
            }

            // Validate URL contains valid characters
            if api_config.endpoint.chars().any(|c| c == '\0' || c.is_control()) {
                return Err("API endpoint contains invalid characters".to_string());
            }

            // Validate model name if provided
            if let Some(ref model) = api_config.model {
                if model.len() > 256 {
                    return Err("API model name too long (max 256 chars)".to_string());
                }
                if model.chars().any(|c| c == '\0' || c.is_control()) {
                    return Err("API model name contains invalid characters".to_string());
                }
            }

            if api_config.timeout_secs == 0 {
                return Err("API timeout must be greater than 0".to_string());
            }

            if api_config.timeout_secs > 300 {
                return Err("API timeout too large (max 300 seconds)".to_string());
            }

            // Validate retry config
            api_config.retry_config.validate()?;
        }

        Ok(())
    }
}

