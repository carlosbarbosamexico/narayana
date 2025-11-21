//! API-based TTS engine implementations
//! Supports OpenAI, Google Cloud, and Amazon Polly

use crate::config::VoiceConfig;
use crate::error::SpeechError;
use crate::engines::TtsEngine;
use async_trait::async_trait;
use bytes::Bytes;
use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn, error};
use url::Url;

/// API TTS engine configuration
pub struct ApiTtsEngine {
    engine_type: ApiEngineType,
    client: Client,
    endpoint: String,
    api_key: Option<String>,
    model: Option<String>,
    timeout: Duration,
    retry_config: crate::config::RetryConfig,
    custom_engine_name: Option<String>, // For custom engines
    rate: u32,   // Speech rate (0-500 WPM)
    volume: f32, // Volume (0.0-1.0)
    pitch: f32,  // Pitch (-1.0 to 1.0)
}

#[derive(Debug, Clone)]
enum ApiEngineType {
    OpenAi,
    GoogleCloud,
    AmazonPolly,
    Custom,
}

impl ApiTtsEngine {
    /// Create a new OpenAI TTS engine
    pub fn new_openai(
        endpoint: String,
        api_key: Option<String>,
        model: Option<String>,
        timeout_secs: u64,
        retry_config: crate::config::RetryConfig,
    ) -> Result<Self, SpeechError> {
        Self::new_openai_with_config(endpoint, api_key, model, timeout_secs, retry_config, 150, 0.8, 0.0)
    }
    
    /// Create a new OpenAI TTS engine with rate/volume/pitch
    pub fn new_openai_with_config(
        endpoint: String,
        api_key: Option<String>,
        model: Option<String>,
        timeout_secs: u64,
        retry_config: crate::config::RetryConfig,
        rate: u32,
        volume: f32,
        pitch: f32,
    ) -> Result<Self, SpeechError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| SpeechError::Engine(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            engine_type: ApiEngineType::OpenAi,
            client,
            endpoint: endpoint.trim_end_matches('/').to_string(),
            api_key,
            model: model.or(Some("tts-1".to_string())), // Default OpenAI model
            timeout: Duration::from_secs(timeout_secs),
            retry_config,
            custom_engine_name: None,
            rate,
            volume,
            pitch,
        })
    }

    /// Create a new Google Cloud TTS engine
    pub fn new_google_cloud(
        endpoint: String,
        api_key: Option<String>,
        model: Option<String>,
        timeout_secs: u64,
        retry_config: crate::config::RetryConfig,
    ) -> Result<Self, SpeechError> {
        Self::new_google_cloud_with_config(endpoint, api_key, model, timeout_secs, retry_config, 150, 0.8, 0.0)
    }
    
    /// Create a new Google Cloud TTS engine with rate/volume/pitch
    pub fn new_google_cloud_with_config(
        endpoint: String,
        api_key: Option<String>,
        model: Option<String>,
        timeout_secs: u64,
        retry_config: crate::config::RetryConfig,
        rate: u32,
        volume: f32,
        pitch: f32,
    ) -> Result<Self, SpeechError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| SpeechError::Engine(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            engine_type: ApiEngineType::GoogleCloud,
            client,
            endpoint: endpoint.trim_end_matches('/').to_string(),
            api_key,
            model,
            timeout: Duration::from_secs(timeout_secs),
            retry_config,
            custom_engine_name: None,
            rate,
            volume,
            pitch,
        })
    }

    /// Create a new Amazon Polly TTS engine
    pub fn new_amazon_polly(
        endpoint: String,
        api_key: Option<String>,
        model: Option<String>,
        timeout_secs: u64,
        retry_config: crate::config::RetryConfig,
    ) -> Result<Self, SpeechError> {
        Self::new_amazon_polly_with_config(endpoint, api_key, model, timeout_secs, retry_config, 150, 0.8, 0.0)
    }
    
    /// Create a new Amazon Polly TTS engine with rate/volume/pitch
    pub fn new_amazon_polly_with_config(
        endpoint: String,
        api_key: Option<String>,
        model: Option<String>,
        timeout_secs: u64,
        retry_config: crate::config::RetryConfig,
        rate: u32,
        volume: f32,
        pitch: f32,
    ) -> Result<Self, SpeechError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| SpeechError::Engine(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            engine_type: ApiEngineType::AmazonPolly,
            client,
            endpoint: endpoint.trim_end_matches('/').to_string(),
            api_key,
            model: model.or(Some("standard".to_string())), // Default Polly engine
            timeout: Duration::from_secs(timeout_secs),
            retry_config,
            custom_engine_name: None,
            rate,
            volume,
            pitch,
        })
    }

    /// Create a new custom API TTS engine
    pub fn new_custom(
        endpoint: String,
        api_key: Option<String>,
        model: Option<String>,
        timeout_secs: u64,
        retry_config: crate::config::RetryConfig,
        engine_name: String,
    ) -> Result<Self, SpeechError> {
        Self::new_custom_with_config(endpoint, api_key, model, timeout_secs, retry_config, engine_name, 150, 0.8, 0.0)
    }
    
    /// Create a new custom API TTS engine with rate/volume/pitch
    pub fn new_custom_with_config(
        endpoint: String,
        api_key: Option<String>,
        model: Option<String>,
        timeout_secs: u64,
        retry_config: crate::config::RetryConfig,
        engine_name: String,
        rate: u32,
        volume: f32,
        pitch: f32,
    ) -> Result<Self, SpeechError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| SpeechError::Engine(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            engine_type: ApiEngineType::Custom,
            client,
            endpoint: endpoint.trim_end_matches('/').to_string(),
            api_key,
            model,
            timeout: Duration::from_secs(timeout_secs),
            retry_config,
            custom_engine_name: Some(engine_name),
            rate,
            volume,
            pitch,
        })
    }

    /// Synthesize using OpenAI TTS API
    async fn synthesize_openai(&self, text: &str, voice_config: &VoiceConfig) -> Result<Bytes, SpeechError> {
        // Get API key from config or environment
        let api_key = if let Some(ref key) = self.api_key {
            key.clone()
        } else if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            key
        } else {
            return Err(SpeechError::Engine("OpenAI API key not provided".to_string()));
        };

        // Select voice based on config
        let voice = voice_config.name.as_deref()
            .or_else(|| {
                match voice_config.gender {
                    Some(crate::config::VoiceGender::Female) => Some("alloy"),
                    Some(crate::config::VoiceGender::Male) => Some("echo"),
                    _ => Some("alloy"),
                }
            })
            .unwrap_or("alloy");

        let model = self.model.as_deref().unwrap_or("tts-1");

        // Note: OpenAI TTS API supports speed parameter (0.25 to 4.0)
        // Calculate speed from rate (OpenAI speed: 0.25 to 4.0)
        // Rate 0-500 WPM maps to speed 0.25-4.0
        // Default rate 150 WPM = speed 1.0
        let speed = self.calculate_openai_speed();
        
        let request_body = json!({
            "model": model,
            "input": text,
            "voice": voice,
            "response_format": "mp3",
            "speed": speed,
        });

        let url = format!("{}/v1/audio/speech", self.endpoint);

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", &api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| SpeechError::Engine(format!("OpenAI API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(SpeechError::Engine(format!("OpenAI API error ({}): {}", status, error_text)));
        }

        let audio_bytes = response.bytes()
            .await
            .map_err(|e| SpeechError::Engine(format!("Failed to read audio response: {}", e)))?;

        Ok(audio_bytes)
    }

    /// Synthesize using Google Cloud TTS API
    async fn synthesize_google_cloud(&self, text: &str, voice_config: &VoiceConfig) -> Result<Bytes, SpeechError> {
        // Get API key from config or environment
        let api_key = if let Some(ref key) = self.api_key {
            key.clone()
        } else if let Ok(key) = std::env::var("GOOGLE_CLOUD_API_KEY") {
            key
        } else {
            return Err(SpeechError::Engine("Google Cloud API key not provided".to_string()));
        };

        // Select voice based on config
        let voice_name = if let Some(ref name) = voice_config.name {
            name.clone()
        } else {
            format!("{}-Standard-{}", 
                voice_config.language.replace("-", "_"),
                match voice_config.gender {
                    Some(crate::config::VoiceGender::Female) => "A",
                    Some(crate::config::VoiceGender::Male) => "B",
                    _ => "A",
                }
            )
        };

        let request_body = json!({
            "input": {
                "text": text
            },
            "voice": {
                "languageCode": voice_config.language,
                "name": voice_name,
                "ssmlGender": match voice_config.gender {
                    Some(crate::config::VoiceGender::Female) => "FEMALE",
                    Some(crate::config::VoiceGender::Male) => "MALE",
                    _ => "NEUTRAL",
                }
            },
            "audioConfig": {
                "audioEncoding": "MP3",
                "speakingRate": self.calculate_speaking_rate(), // Based on rate config
                "volumeGainDb": self.calculate_volume_gain_db(), // Based on volume config
                "pitch": self.calculate_pitch_semitones(), // Based on pitch config
            }
        });

        let url = format!("{}/v1/text:synthesize?key={}", self.endpoint, &api_key);

        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| SpeechError::Engine(format!("Google Cloud API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(SpeechError::Engine(format!("Google Cloud API error ({}): {}", status, error_text)));
        }

        let response_json: serde_json::Value = response.json()
            .await
            .map_err(|e| SpeechError::Engine(format!("Failed to parse Google Cloud response: {}", e)))?;

        let audio_content = response_json.get("audioContent")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SpeechError::Engine("Missing audioContent in Google Cloud response".to_string()))?;

        // Decode base64 audio
        use base64::{Engine as _, engine::general_purpose};
        let audio_bytes = general_purpose::STANDARD.decode(audio_content)
            .map_err(|e| SpeechError::Engine(format!("Failed to decode base64 audio: {}", e)))?;

        Ok(Bytes::from(audio_bytes))
    }

    /// Synthesize using Amazon Polly TTS API
    async fn synthesize_amazon_polly(&self, text: &str, voice_config: &VoiceConfig) -> Result<Bytes, SpeechError> {
        // Amazon Polly requires AWS credentials and signature v4 signing
        // We'll use a simplified HTTP approach that works with some Polly-compatible endpoints
        // For full AWS Polly, users should use aws-sdk-polly
        
        // Get API key from config or environment
        let api_key = if let Some(ref key) = self.api_key {
            key.clone()
        } else if let Ok(key) = std::env::var("AWS_ACCESS_KEY_ID") {
            key
        } else {
            return Err(SpeechError::Engine("AWS credentials not provided".to_string()));
        };

        // Get secret key for signing (if available)
        let _secret_key = std::env::var("AWS_SECRET_ACCESS_KEY").ok();

        // Select voice based on config
        let voice_id = if let Some(ref name) = voice_config.name {
            name.clone()
        } else {
            format!("{}-{}", 
                voice_config.language.split('-').next().unwrap_or("en"),
                match voice_config.gender {
                    Some(crate::config::VoiceGender::Female) => "Joanna",
                    Some(crate::config::VoiceGender::Male) => "Matthew",
                    _ => "Joanna",
                }
            )
        };

        // Use a simplified approach: try to call Polly-compatible endpoint
        // Note: Full AWS signature v4 requires aws-sdk-polly
        // This implementation works with Polly-compatible services that accept simple API keys
        
        // Amazon Polly supports SSML for rate/volume/pitch control
        // Use SSML prosody attributes for full control:
        // Rate: x-slow, slow, medium, fast, x-fast, or percentage (e.g., "120%")
        // Volume: silent, x-soft, soft, medium, loud, x-loud, or dB (e.g., "+6dB")
        // Pitch: x-low, low, medium, high, x-high, or semitones (e.g., "+5st")
        
        // Convert rate to SSML prosody rate attribute
        let rate_attr = if self.rate == 0 {
            "x-slow"
        } else if self.rate < 100 {
            "slow"
        } else if self.rate <= 150 {
            "medium"
        } else if self.rate <= 250 {
            "fast"
        } else {
            "x-fast"
        };
        
        // Convert volume to SSML prosody volume attribute
        // Map 0.0-1.0 to silent-x-loud
        let volume_attr = if self.volume <= 0.0 {
            "silent"
        } else if self.volume < 0.3 {
            "x-soft"
        } else if self.volume < 0.6 {
            "soft"
        } else if self.volume <= 0.8 {
            "medium"
        } else if self.volume < 0.95 {
            "loud"
        } else {
            "x-loud"
        };
        
        // Convert pitch to SSML prosody pitch attribute
        // Map -1.0 to 1.0 to x-low to x-high
        let pitch_attr = if self.pitch <= -0.7 {
            "x-low"
        } else if self.pitch < -0.3 {
            "low"
        } else if self.pitch <= 0.3 {
            "medium"
        } else if self.pitch < 0.7 {
            "high"
        } else {
            "x-high"
        };
        
        // Escape XML special characters in text
        let escaped_text = text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;");
        
        // Build SSML with prosody attributes
        let ssml_text = format!(
            r#"<speak><prosody rate="{}" volume="{}" pitch="{}">{}</prosody></speak>"#,
            rate_attr, volume_attr, pitch_attr, escaped_text
        );
        
        // Use SSML for full rate/volume/pitch control
        let request_body = json!({
            "Text": ssml_text,
            "OutputFormat": "mp3",
            "VoiceId": voice_id,
            "TextType": "ssml",
            "SampleRate": "22050",
            "Engine": "neural", // Use neural engine for better quality
        });

        let url = format!("{}/v1/speech", self.endpoint);

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", &api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| SpeechError::Engine(format!("Amazon Polly API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            
            // If it's a 403/401, suggest using AWS SDK
            if status == 403 || status == 401 {
                return Err(SpeechError::Engine(format!(
                    "Amazon Polly authentication failed. Full AWS Polly requires aws-sdk-polly for signature v4 signing. \
                     Error: {} (status: {})",
                    error_text, status
                )));
            }
            
            return Err(SpeechError::Engine(format!("Amazon Polly API error ({}): {}", status, error_text)));
        }

        let audio_bytes = response.bytes()
            .await
            .map_err(|e| SpeechError::Engine(format!("Failed to read audio response: {}", e)))?;

        Ok(audio_bytes)
    }

    /// Synthesize using custom API endpoint
    async fn synthesize_custom(&self, text: &str, voice_config: &VoiceConfig) -> Result<Bytes, SpeechError> {
        // Validate input text
        if text.is_empty() {
            return Err(SpeechError::Engine("Text cannot be empty".to_string()));
        }
        if text.len() > 100_000 {
            return Err(SpeechError::Engine("Text too long (max 100KB)".to_string()));
        }
        if text.contains('\0') {
            return Err(SpeechError::Engine("Text contains null bytes".to_string()));
        }

        // Validate endpoint URL
        let endpoint_url = url::Url::parse(&self.endpoint)
            .map_err(|e| SpeechError::Engine(format!("Invalid endpoint URL: {}", e)))?;
        
        // Only allow HTTP/HTTPS protocols
        match endpoint_url.scheme() {
            "http" | "https" => {},
            scheme => return Err(SpeechError::Engine(format!(
                "Unsupported URL scheme: {}. Only http:// and https:// are allowed.",
                scheme
            ))),
        }

        // Validate voice name if provided
        if let Some(ref voice_name) = voice_config.name {
            if voice_name.len() > 256 {
                return Err(SpeechError::Engine("Voice name too long (max 256 chars)".to_string()));
            }
            if voice_name.contains('\0') || voice_name.contains('\n') || voice_name.contains('\r') {
                return Err(SpeechError::Engine("Voice name contains invalid characters".to_string()));
            }
        }

        // Get API key from config or environment
        let api_key = if let Some(ref key) = self.api_key {
            key.clone()
        } else {
            // Try common environment variable names
            std::env::var("API_KEY")
                .or_else(|_| std::env::var("CUSTOM_TTS_API_KEY"))
                .unwrap_or_else(|_| String::new())
        };

        // Build request body - generic format that works with most TTS APIs
        let mut request_body = json!({
            "text": text,
            "format": "mp3",
        });

        // Add voice configuration if available
        if let Some(ref voice_name) = voice_config.name {
            request_body["voice"] = json!(voice_name);
        }
        request_body["language"] = json!(voice_config.language);

        if let Some(ref model) = self.model {
            // Validate model name
            if model.len() > 256 {
                return Err(SpeechError::Engine("Model name too long (max 256 chars)".to_string()));
            }
            request_body["model"] = json!(model);
        }

        // Try common TTS API endpoints
        let mut url = self.endpoint.clone();
        
        // If endpoint doesn't have a path, try common paths
        if !url.contains("/v1/") && !url.contains("/api/") && !url.contains("/tts") {
            // Try common endpoint patterns
            if url.ends_with("/") {
                url.push_str("v1/synthesize");
            } else {
                url.push_str("/v1/synthesize");
            }
        }

        // Re-validate the constructed URL
        let final_url = url::Url::parse(&url)
            .map_err(|e| SpeechError::Engine(format!("Invalid constructed URL: {}", e)))?;
        
        if !matches!(final_url.scheme(), "http" | "https") {
            return Err(SpeechError::Engine("Invalid URL scheme after path construction".to_string()));
        }

        let mut request = self.client
            .post(&url)
            .header("Content-Type", "application/json");

        // Add authorization if API key is provided
        // Only set one header to avoid conflicts - prefer Bearer token
        if !api_key.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", &api_key));
        }

        let response = request
            .json(&request_body)
            .send()
            .await
            .map_err(|e| SpeechError::Engine(format!("Custom TTS API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            // Limit error text size to prevent DoS
            let error_text = response.text().await
                .map(|s| {
                    if s.len() > 1000 {
                        // Use char iterator to avoid UTF-8 boundary issues
                        let truncated: String = s.chars().take(1000).collect();
                        format!("{}...", truncated)
                    } else {
                        s
                    }
                })
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(SpeechError::Engine(format!(
                "Custom TTS API error ({}): {}. Check endpoint URL and API key.",
                status, error_text
            )));
        }

        // Get content length if available to check size before reading
        const MAX_RESPONSE_SIZE: usize = 10 * 1024 * 1024; // 10MB max
        if let Some(content_length) = response.content_length() {
            if content_length > MAX_RESPONSE_SIZE as u64 {
                return Err(SpeechError::Engine(format!(
                    "Response too large ({} bytes, max {} bytes)",
                    content_length, MAX_RESPONSE_SIZE
                )));
            }
        }

        // Try to read as audio bytes directly
        let audio_bytes = response.bytes()
            .await
            .map_err(|e| SpeechError::Engine(format!("Failed to read audio response: {}", e)))?;

        // Enforce size limit even if content-length wasn't provided
        if audio_bytes.len() > MAX_RESPONSE_SIZE {
            return Err(SpeechError::Engine(format!(
                "Response too large ({} bytes, max {} bytes)",
                audio_bytes.len(), MAX_RESPONSE_SIZE
            )));
        }

        // If response is JSON, try to extract audio from common fields
        // More reliable JSON detection: check for JSON-like start and try parsing
        if audio_bytes.len() > 2 && audio_bytes[0] == b'{' && audio_bytes[audio_bytes.len() - 1] == b'}' {
            if let Ok(json_response) = serde_json::from_slice::<serde_json::Value>(&audio_bytes) {
                // Try common audio response fields
                if let Some(audio_base64) = json_response.get("audio")
                    .or_else(|| json_response.get("data"))
                    .or_else(|| json_response.get("audioContent"))
                    .and_then(|v| v.as_str()) {
                    // Validate base64 string length
                    if audio_base64.len() > MAX_RESPONSE_SIZE {
                        return Err(SpeechError::Engine("Base64 audio string too long".to_string()));
                    }
                    use base64::{Engine as _, engine::general_purpose};
                    let decoded = general_purpose::STANDARD.decode(audio_base64)
                        .map_err(|e| SpeechError::Engine(format!("Failed to decode base64 audio: {}", e)))?;
                    
                    // Validate decoded size
                    if decoded.len() > MAX_RESPONSE_SIZE {
                        return Err(SpeechError::Engine("Decoded audio too large".to_string()));
                    }
                    
                    return Ok(Bytes::from(decoded));
                }
            }
        }

        Ok(audio_bytes)
    }

    /// Retry wrapper for API calls
    async fn retry_request<F, Fut>(&self, f: F) -> Result<Bytes, SpeechError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<Bytes, SpeechError>>,
    {
        let mut delay = self.retry_config.initial_delay_ms;
        let mut last_error = None;

        for attempt in 0..=self.retry_config.max_retries {
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.retry_config.max_retries {
                        debug!("TTS API request failed, retrying in {}ms (attempt {}/{})", 
                            delay, attempt + 1, self.retry_config.max_retries);
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        // Use checked arithmetic to prevent overflow in exponential backoff
                        delay = delay.checked_mul(2)
                            .map(|d| d.min(self.retry_config.max_delay_ms))
                            .unwrap_or(self.retry_config.max_delay_ms); // Fallback to max if overflow
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| SpeechError::Engine("Unknown error".to_string())))
    }
}

#[async_trait]
impl TtsEngine for ApiTtsEngine {
    async fn synthesize(&self, text: &str, config: &VoiceConfig) -> Result<Bytes, SpeechError> {
        // Validate input
        if text.is_empty() {
            return Err(SpeechError::Engine("Text cannot be empty".to_string()));
        }

        if text.len() > 100_000 {
            return Err(SpeechError::Engine("Text too long (max 100KB)".to_string()));
        }

        // Retry wrapper
        self.retry_request(|| async {
            match self.engine_type {
                ApiEngineType::OpenAi => self.synthesize_openai(text, config).await,
                ApiEngineType::GoogleCloud => self.synthesize_google_cloud(text, config).await,
                ApiEngineType::AmazonPolly => self.synthesize_amazon_polly(text, config).await,
                ApiEngineType::Custom => self.synthesize_custom(text, config).await,
            }
        }).await
    }

    async fn list_voices(&self) -> Result<Vec<String>, SpeechError> {
        match self.engine_type {
            ApiEngineType::OpenAi => {
                // OpenAI has fixed voices
                Ok(vec![
                    "alloy".to_string(),
                    "echo".to_string(),
                    "fable".to_string(),
                    "onyx".to_string(),
                    "nova".to_string(),
                    "shimmer".to_string(),
                ])
            }
            ApiEngineType::GoogleCloud => {
                self.list_voices_google_cloud().await
            }
            ApiEngineType::AmazonPolly => {
                self.list_voices_amazon_polly().await
            }
            ApiEngineType::Custom => {
                // For custom APIs, we can't list voices without knowing the API structure
                // Return empty list - users should configure voices explicitly
                Ok(vec![])
            }
        }
    }

    fn is_available(&self) -> bool {
        // Check if API key is available
        match self.engine_type {
            ApiEngineType::OpenAi => {
                self.api_key.is_some() || std::env::var("OPENAI_API_KEY").is_ok()
            }
            ApiEngineType::GoogleCloud => {
                self.api_key.is_some() || std::env::var("GOOGLE_CLOUD_API_KEY").is_ok()
            }
            ApiEngineType::AmazonPolly => {
                self.api_key.is_some() || std::env::var("AWS_ACCESS_KEY_ID").is_ok()
            }
            ApiEngineType::Custom => {
                // Custom API is available if endpoint is set
                !self.endpoint.is_empty()
            }
        }
    }

    fn name(&self) -> &str {
        match self.engine_type {
            ApiEngineType::OpenAi => "OpenAI TTS",
            ApiEngineType::GoogleCloud => "Google Cloud TTS",
            ApiEngineType::AmazonPolly => "Amazon Polly",
            ApiEngineType::Custom => {
                self.custom_engine_name.as_deref().unwrap_or("Custom API TTS")
            }
        }
    }
}

impl ApiTtsEngine {
    /// Calculate speaking rate for Google Cloud TTS (0.25 to 4.0)
    /// Maps from SpeechConfig.rate (0-500 WPM) to Google Cloud speakingRate
    /// Default 150 WPM = 1.0
    fn calculate_speaking_rate(&self) -> f32 {
        // Map 0-500 WPM to 0.25-4.0
        // 0 WPM -> 0.25, 150 WPM -> 1.0, 500 WPM -> 4.0
        if self.rate <= 150 {
            // 0-150 WPM maps to 0.25-1.0
            0.25 + (self.rate as f32 / 150.0) * 0.75
        } else {
            // 150-500 WPM maps to 1.0-4.0
            1.0 + ((self.rate - 150) as f32 / 350.0) * 3.0
        }.clamp(0.25, 4.0)
    }
    
    /// Calculate volume gain in dB for Google Cloud TTS (-96.0 to 16.0)
    /// Maps from SpeechConfig.volume (0.0-1.0) to Google Cloud volumeGainDb
    fn calculate_volume_gain_db(&self) -> f32 {
        // Map 0.0-1.0 to -96.0 to 16.0 dB
        // 0.0 -> -96.0 (silent), 0.5 -> -40.0, 1.0 -> 16.0 (loud)
        -96.0 + (self.volume * 112.0)
    }
    
    /// Calculate pitch in semitones for Google Cloud TTS (-20.0 to 20.0)
    /// Maps from SpeechConfig.pitch (-1.0 to 1.0) to Google Cloud pitch
    fn calculate_pitch_semitones(&self) -> f32 {
        // Map -1.0 to 1.0 to -20.0 to 20.0 semitones
        self.pitch * 20.0
    }
    
    /// Calculate speed for OpenAI TTS (0.25 to 4.0)
    /// Maps from SpeechConfig.rate (0-500 WPM) to OpenAI speed
    fn calculate_openai_speed(&self) -> f32 {
        // Map 0-500 WPM to 0.25-4.0
        // 0 WPM -> 0.25, 150 WPM -> 1.0, 500 WPM -> 4.0
        if self.rate <= 150 {
            // 0-150 WPM maps to 0.25-1.0
            0.25 + (self.rate as f32 / 150.0) * 0.75
        } else {
            // 150-500 WPM maps to 1.0-4.0
            1.0 + ((self.rate - 150) as f32 / 350.0) * 3.0
        }.clamp(0.25, 4.0)
    }
    
    /// List voices from Google Cloud TTS API
    async fn list_voices_google_cloud(&self) -> Result<Vec<String>, SpeechError> {
        use tracing::warn;
        
        // Get API key
        let api_key = if let Some(ref key) = self.api_key {
            key.clone()
        } else if let Ok(key) = std::env::var("GOOGLE_CLOUD_API_KEY") {
            key
        } else {
            // Return default voices if API key not available
            return Ok(vec![
                "en-US-Standard-A".to_string(),
                "en-US-Standard-B".to_string(),
                "en-US-Standard-C".to_string(),
                "en-US-Standard-D".to_string(),
            ]);
        };

        let url = format!("{}/v1/voices?key={}", self.endpoint, &api_key);

        let response = self.client
            .get(&url)
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| SpeechError::Engine(format!("Google Cloud voices API request failed: {}", e)))?;

        if !response.status().is_success() {
            // If API call fails, return default voices
            warn!("Failed to list Google Cloud voices, using defaults");
            return Ok(vec![
                "en-US-Standard-A".to_string(),
                "en-US-Standard-B".to_string(),
                "en-US-Standard-C".to_string(),
                "en-US-Standard-D".to_string(),
            ]);
        }

        let response_json: serde_json::Value = response.json()
            .await
            .map_err(|e| SpeechError::Engine(format!("Failed to parse Google Cloud voices response: {}", e)))?;

        // Extract voice names from response
        let voices: Vec<String> = response_json
            .get("voices")
            .and_then(|v| v.as_array())
            .map(|voices_array| {
                voices_array
                    .iter()
                    .filter_map(|voice| {
                        voice.get("name")
                            .and_then(|n| n.as_str())
                            .map(|name| {
                                // Extract voice name (format: "projects/.../voices/voice-name")
                                name.split('/').last().unwrap_or(name).to_string()
                            })
                    })
                    .filter(|name| name.len() <= 256) // Validate length
                    .take(1000) // Limit to prevent memory exhaustion
                    .collect()
            })
            .unwrap_or_else(|| {
                // Fallback to defaults if parsing fails
                vec![
                    "en-US-Standard-A".to_string(),
                    "en-US-Standard-B".to_string(),
                    "en-US-Standard-C".to_string(),
                    "en-US-Standard-D".to_string(),
                ]
            });

        if voices.is_empty() {
            // Return defaults if no voices found
            Ok(vec![
                "en-US-Standard-A".to_string(),
                "en-US-Standard-B".to_string(),
                "en-US-Standard-C".to_string(),
                "en-US-Standard-D".to_string(),
            ])
        } else {
            Ok(voices)
        }
    }
    
    /// List voices from Amazon Polly API
    async fn list_voices_amazon_polly(&self) -> Result<Vec<String>, SpeechError> {
        use tracing::{debug, warn};
        
        // Get API key
        let api_key = if let Some(ref key) = self.api_key {
            key.clone()
        } else if let Ok(key) = std::env::var("AWS_ACCESS_KEY_ID") {
            key
        } else {
            // Return default voices if API key not available
            return Ok(vec![
                "Joanna".to_string(),
                "Matthew".to_string(),
                "Amy".to_string(),
                "Brian".to_string(),
            ]);
        };

        // Try to call AWS Polly DescribeVoices API
        // Note: This requires proper AWS signature v4, but we'll try a simple approach
        let url = format!("{}/v1/voices", self.endpoint);

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", &api_key))
            .header("Content-Type", "application/json")
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(response_json) = resp.json::<serde_json::Value>().await {
                    // Extract voice names from response
                    let voices: Vec<String> = response_json
                        .get("Voices")
                        .and_then(|v| v.as_array())
                        .map(|voices_array| {
                            voices_array
                                .iter()
                                .filter_map(|voice| {
                                    voice.get("Id")
                                        .and_then(|id| id.as_str())
                                        .map(|name| name.to_string())
                                })
                                .filter(|name| name.len() <= 256) // Validate length
                                .take(1000) // Limit to prevent memory exhaustion
                                .collect()
                        })
                        .unwrap_or_else(Vec::new);

                    if !voices.is_empty() {
                        return Ok(voices);
                    }
                }
            }
            _ => {
                // API call failed, use defaults
                debug!("Amazon Polly voices API call failed, using defaults");
            }
        }

        // Return default voices
        Ok(vec![
            "Joanna".to_string(),
            "Matthew".to_string(),
            "Amy".to_string(),
            "Brian".to_string(),
            "Ivy".to_string(),
            "Justin".to_string(),
            "Kendra".to_string(),
            "Kimberly".to_string(),
        ])
    }
}

