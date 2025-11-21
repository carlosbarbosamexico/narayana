//! LiveAvatar provider implementation

use crate::avatar_broker::{AvatarProvider, AvatarStream};
use crate::config::{AvatarConfig, Expression, Gesture, Emotion};
use crate::error::AvatarError;
use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;
use url::Url as UrlUrl;
use tracing::{info, warn, debug};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use bytes::Bytes;

/// LiveAvatar provider
pub struct LiveAvatarProvider {
    config: AvatarConfig,
    api_key: String,
    base_url: String,
    client: Arc<Client>,
    ws_stream: Arc<RwLock<Option<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
    stream_id: Option<String>,
}

impl LiveAvatarProvider {
    /// Create a new LiveAvatar provider
    pub async fn new(config: AvatarConfig) -> Result<Self, AvatarError> {
        // Get API key from environment variable
        let api_key = std::env::var("LIVE_AVATAR_API_KEY")
            .map_err(|_| AvatarError::Config("LIVE_AVATAR_API_KEY environment variable not set".to_string()))?;

        // Validate API key
        if api_key.is_empty() || api_key.len() > 512 {
            return Err(AvatarError::Config("Invalid API key length".to_string()));
        }
        if api_key.chars().any(|c| c.is_control()) {
            return Err(AvatarError::Config("API key contains invalid characters".to_string()));
        }

        // Get base URL from environment or use default
        let base_url = std::env::var("LIVE_AVATAR_BASE_URL")
            .unwrap_or_else(|_| "https://api.liveavatar.ai/v1".to_string());

        // Validate base URL
        if !base_url.starts_with("https://") {
            return Err(AvatarError::Config("Base URL must use HTTPS".to_string()));
        }
        if base_url.len() > 2048 {
            return Err(AvatarError::Config("Base URL too long".to_string()));
        }

        // Validate URL format
        if UrlUrl::parse(&base_url).is_err() {
            return Err(AvatarError::Config("Invalid base URL format".to_string()));
        }

        // Create HTTP client with timeout
        let client = Arc::new(
            Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| AvatarError::Network(format!("Failed to create HTTP client: {}", e)))?
        );

        Ok(Self {
            config,
            api_key,
            base_url,
            client,
            ws_stream: Arc::new(RwLock::new(None)),
            stream_id: None,
        })
    }

    /// Map expression to LiveAvatar format
    fn map_expression(&self, expression: &Expression) -> Result<String, AvatarError> {
        match expression {
            Expression::Neutral => Ok("neutral".to_string()),
            Expression::Happy => Ok("happy".to_string()),
            Expression::Sad => Ok("sad".to_string()),
            Expression::Angry => Ok("angry".to_string()),
            Expression::Surprised => Ok("surprised".to_string()),
            Expression::Thinking => Ok("thinking".to_string()),
            Expression::Confused => Ok("confused".to_string()),
            Expression::Excited => Ok("excited".to_string()),
            Expression::Tired => Ok("tired".to_string()),
            Expression::Recognition => Ok("recognizing".to_string()),
            Expression::Custom(s) => {
                if s.is_empty() || s.len() > 256 {
                    return Err(AvatarError::Config("Invalid custom expression string".to_string()));
                }
                if s.chars().any(|c| !c.is_alphanumeric() && c != '-' && c != '_') {
                    return Err(AvatarError::Config("Custom expression contains invalid characters".to_string()));
                }
                Ok(s.clone())
            },
        }
    }
}

#[async_trait]
impl AvatarProvider for LiveAvatarProvider {
    async fn initialize(&mut self, _config: &AvatarConfig) -> Result<(), AvatarError> {
        info!("Initializing LiveAvatar provider");

        // Test connection with health check
        let test_url = format!("{}/health", self.base_url);
        let response = self.client
            .get(&test_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| AvatarError::Api(format!("Failed to connect to LiveAvatar API: {}", e)))?;

        if !response.status().is_success() {
            warn!("LiveAvatar API health check returned: {}", response.status());
        }

        info!("LiveAvatar provider initialized");
        Ok(())
    }

    async fn start_stream(&mut self) -> Result<AvatarStream, AvatarError> {
        info!("Starting LiveAvatar stream");

        let stream_url = format!("{}/streams/create", self.base_url);
        let avatar_id = self.config.avatar_id.clone().unwrap_or_else(|| "default".to_string());

        // Validate avatar ID
        if avatar_id.len() > 256 || avatar_id.chars().any(|c| !c.is_alphanumeric() && c != '-' && c != '_' && c != '.') {
            return Err(AvatarError::Config("Invalid Avatar ID".to_string()));
        }
        if avatar_id.contains("..") || avatar_id.contains("//") {
            return Err(AvatarError::Config("Invalid Avatar ID (path traversal attempt)".to_string()));
        }

        let payload = serde_json::json!({
            "avatar_id": avatar_id,
            "enable_lip_sync": self.config.enable_lip_sync,
            "enable_expressions": true,
            "enable_gestures": self.config.enable_gestures,
        });

            // Validate payload size (safe serialization)
            const MAX_PAYLOAD_SIZE: usize = 10_000; // 10KB max
            let payload_size = match serde_json::to_string(&payload) {
                Ok(s) => s.len(),
                Err(e) => {
                    warn!("Failed to serialize payload for size check: {}", e);
                    return Err(AvatarError::Api(format!("Failed to serialize payload: {}", e)));
                }
            };
            if payload_size > MAX_PAYLOAD_SIZE {
                return Err(AvatarError::Api(format!("Payload too large (max {} bytes)", MAX_PAYLOAD_SIZE)));
            }

        let response = self.client
            .post(&stream_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| AvatarError::Api(format!("Failed to create stream: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            // Limit error text size to prevent DoS
            const MAX_ERROR_TEXT_SIZE: usize = 10_000; // 10KB max
            let error_text = {
                let bytes = match response.bytes().await {
                    Ok(b) => b,
                    Err(_) => bytes::Bytes::new(), // Empty if read fails
                };
                if bytes.len() > MAX_ERROR_TEXT_SIZE {
                    String::from_utf8_lossy(&bytes[..MAX_ERROR_TEXT_SIZE.min(bytes.len())]).to_string()
                } else {
                    String::from_utf8_lossy(&bytes).to_string()
                }
            };
            return Err(AvatarError::Api(format!("Failed to create stream: {} - {}", status, error_text)));
        }

        // Validate response size (check content-length, but also limit actual read)
        const MAX_RESPONSE_SIZE: u64 = 100 * 1024; // 100KB max
        if let Some(content_length) = response.content_length() {
            if content_length > MAX_RESPONSE_SIZE {
                return Err(AvatarError::Api(format!("Response too large (max {} bytes)", MAX_RESPONSE_SIZE)));
            }
        }
        // For chunked responses or missing content-length, validate after reading

        // Parse response with size limit
        let stream_response: serde_json::Value = {
            let bytes = response.bytes().await
                .map_err(|e| AvatarError::Api(format!("Failed to read response: {}", e)))?;
            
            // Validate actual response size
            if bytes.len() > MAX_RESPONSE_SIZE as usize {
                return Err(AvatarError::Api(format!("Response too large (max {} bytes)", MAX_RESPONSE_SIZE)));
            }
            
            serde_json::from_slice(&bytes)
                .map_err(|e| AvatarError::Api(format!("Failed to parse stream response: {}", e)))?
        };

        let stream_id = stream_response.get("stream_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AvatarError::Api("Missing stream_id in response".to_string()))?
            .to_string();

        if stream_id.is_empty() || stream_id.len() > 256 || stream_id.chars().any(|c| !c.is_alphanumeric() && c != '-' && c != '_') {
            return Err(AvatarError::Api("Invalid stream_id from API".to_string()));
        }

        let ws_url = stream_response.get("websocket_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AvatarError::Api("Missing websocket_url in response".to_string()))?
            .to_string();

        if ws_url.is_empty() || ws_url.len() > 2048 {
            return Err(AvatarError::Api("Invalid websocket_url length from API".to_string()));
        }
        if !ws_url.starts_with("ws://") && !ws_url.starts_with("wss://") {
            return Err(AvatarError::Api("Invalid websocket_url protocol from API".to_string()));
        }
        if UrlUrl::parse(&ws_url).is_err() {
            return Err(AvatarError::Api("Invalid websocket_url format from API".to_string()));
        }

        let (ws_stream, _) = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            connect_async(&ws_url)
        )
        .await
        .map_err(|_| AvatarError::Network("WebSocket connection timed out".to_string()))?
        .map_err(|e| AvatarError::Network(format!("Failed to connect WebSocket: {}", e)))?;

        *self.ws_stream.write().await = Some(ws_stream);
        self.stream_id = Some(stream_id.clone());

        let client_url = format!("ws://localhost:8081/avatar/stream/{}", stream_id);

        info!("LiveAvatar stream started: {}", stream_id);
        Ok(AvatarStream {
            stream_id,
            client_url,
            handle: Box::new(()),
        })
    }

    async fn stop_stream(&mut self) -> Result<(), AvatarError> {
        if self.stream_id.is_none() {
            warn!("Attempted to stop stream when no stream is active");
            return Ok(());
        }

        // Safely get stream_id (avoid panic)
        let stream_id = match self.stream_id.take() {
            Some(id) => id,
            None => {
                warn!("Attempted to stop stream when no stream is active");
                return Ok(());
            }
        };
        info!("Stopping LiveAvatar stream: {}", stream_id);

        let ws_opt = self.ws_stream.write().await.take();
        if let Some(mut ws) = ws_opt {
            let close_result = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                ws.close(None)
            ).await;
            match close_result {
                Ok(Ok(_)) => {
                    debug!("WebSocket closed successfully");
                }
                Ok(Err(e)) => {
                    warn!("WebSocket close error: {}", e);
                }
                Err(_) => {
                    warn!("WebSocket close timed out");
                }
            }
        }

        // Use URL encoding to prevent injection attacks
        let encoded_stream_id = utf8_percent_encode(&stream_id, NON_ALPHANUMERIC).to_string();
        let stop_url = format!("{}/streams/{}/close", self.base_url, encoded_stream_id);
        let _ = self.client
            .post(&stop_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await;

        info!("LiveAvatar stream stopped");
        Ok(())
    }

    async fn send_audio(&self, audio_data: Vec<u8>) -> Result<(), AvatarError> {
        if audio_data.is_empty() {
            return Ok(());
        }

        const MAX_AUDIO_SIZE: usize = 10 * 1024 * 1024;
        if audio_data.len() > MAX_AUDIO_SIZE {
            warn!("Audio data too large ({} bytes, max {} bytes), rejecting", audio_data.len(), MAX_AUDIO_SIZE);
            return Err(AvatarError::Config(format!("Audio data too large (max {} bytes)", MAX_AUDIO_SIZE)));
        }

        if let Some(ref stream_id) = self.stream_id {
            if stream_id.is_empty() || stream_id.len() > 256 || stream_id.chars().any(|c| !c.is_alphanumeric() && c != '-' && c != '_') {
                return Err(AvatarError::Api("Invalid stream_id".to_string()));
            }

            // Use URL encoding to prevent injection attacks
            let encoded_stream_id = utf8_percent_encode(stream_id, NON_ALPHANUMERIC).to_string();
            let audio_url = format!("{}/streams/{}/audio", self.base_url, encoded_stream_id);

            let response = self.client
                .post(&audio_url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "audio/wav")
                .body(audio_data)
                .timeout(std::time::Duration::from_secs(30))
                .send()
                .await
                .map_err(|e| AvatarError::Api(format!("Failed to send audio: {}", e)))?;

            if !response.status().is_success() {
                warn!("Audio send returned non-success status: {}", response.status());
            }
        } else {
            warn!("Cannot send audio: stream not started");
        }

        Ok(())
    }

    async fn set_expression(&self, expression: Expression, intensity: f64) -> Result<(), AvatarError> {
        if !intensity.is_finite() || !(0.0..=1.0).contains(&intensity) {
            warn!("Invalid intensity value: {}, clamping to 0.0-1.0", intensity);
            return Err(AvatarError::Config("Intensity must be between 0.0 and 1.0".to_string()));
        }

        if let Some(ref stream_id) = self.stream_id {
            if stream_id.is_empty() || stream_id.len() > 256 || stream_id.chars().any(|c| !c.is_alphanumeric() && c != '-' && c != '_') {
                return Err(AvatarError::Api("Invalid stream_id".to_string()));
            }

            let expression_id = self.map_expression(&expression)?;
            // Use URL encoding to prevent injection attacks
            let encoded_stream_id = utf8_percent_encode(stream_id, NON_ALPHANUMERIC).to_string();
            let expression_url = format!("{}/streams/{}/expression", self.base_url, encoded_stream_id);

            let payload = serde_json::json!({
                "expression": expression_id,
                "intensity": intensity,
            });

            // Validate payload size (safe serialization)
            const MAX_PAYLOAD_SIZE: usize = 10_000; // 10KB max
            let payload_size = match serde_json::to_string(&payload) {
                Ok(s) => s.len(),
                Err(e) => {
                    warn!("Failed to serialize payload for size check: {}", e);
                    return Err(AvatarError::Api(format!("Failed to serialize payload: {}", e)));
                }
            };
            if payload_size > MAX_PAYLOAD_SIZE {
                return Err(AvatarError::Api(format!("Payload too large (max {} bytes)", MAX_PAYLOAD_SIZE)));
            }

            let response = self.client
                .post(&expression_url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&payload)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
                .map_err(|e| AvatarError::Api(format!("Failed to set expression: {}", e)))?;

            if !response.status().is_success() {
                warn!("Expression set returned non-success status: {}", response.status());
            }
        } else {
            warn!("Cannot set expression: stream not started");
        }

        Ok(())
    }

    async fn set_gesture(&self, gesture: Gesture, duration_ms: u64) -> Result<(), AvatarError> {
        if gesture == Gesture::None {
            return Ok(());
        }

        if let Some(ref stream_id) = self.stream_id {
            if stream_id.is_empty() || stream_id.len() > 256 || stream_id.chars().any(|c| !c.is_alphanumeric() && c != '-' && c != '_') {
                return Err(AvatarError::Api("Invalid stream_id".to_string()));
            }

            let gesture_id = match gesture {
                Gesture::Wave => "wave",
                Gesture::Point => "point",
                Gesture::Nod => "nod",
                Gesture::Shake => "shake",
                Gesture::ThumbsUp => "thumbs_up",
                Gesture::Custom(ref s) => {
                    if s.is_empty() || s.len() > 256 || s.chars().any(|c| !c.is_alphanumeric() && c != '-' && c != '_') {
                        return Err(AvatarError::Config("Invalid custom gesture string".to_string()));
                    }
                    s.as_str()
                },
                Gesture::None => return Ok(()),
            };

            const MAX_GESTURE_DURATION_MS: u64 = 300_000;
            let duration_ms = duration_ms.min(MAX_GESTURE_DURATION_MS);

            // Use URL encoding to prevent injection attacks
            let encoded_stream_id = utf8_percent_encode(stream_id, NON_ALPHANUMERIC).to_string();
            let gesture_url = format!("{}/streams/{}/gesture", self.base_url, encoded_stream_id);

            let payload = serde_json::json!({
                "gesture": gesture_id,
                "duration_ms": duration_ms,
            });

            // Validate payload size (safe serialization)
            const MAX_PAYLOAD_SIZE: usize = 10_000; // 10KB max
            let payload_size = match serde_json::to_string(&payload) {
                Ok(s) => s.len(),
                Err(e) => {
                    warn!("Failed to serialize payload for size check: {}", e);
                    return Err(AvatarError::Api(format!("Failed to serialize payload: {}", e)));
                }
            };
            if payload_size > MAX_PAYLOAD_SIZE {
                return Err(AvatarError::Api(format!("Payload too large (max {} bytes)", MAX_PAYLOAD_SIZE)));
            }

            let response = self.client
                .post(&gesture_url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&payload)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
                .map_err(|e| AvatarError::Api(format!("Failed to set gesture: {}", e)))?;

            if !response.status().is_success() {
                warn!("Gesture set returned non-success status: {}", response.status());
            }
        } else {
            warn!("Cannot set gesture: stream not started");
        }

        Ok(())
    }

    async fn update_emotion(&self, emotion: Emotion, intensity: f64) -> Result<(), AvatarError> {
        if !intensity.is_finite() || intensity < 0.0 || intensity > 1.0 {
            warn!("Invalid emotion intensity: {}, clamping to 0.0-1.0", intensity);
            let intensity = intensity.clamp(0.0, 1.0);
            if !intensity.is_finite() {
                return Err(AvatarError::Config("Emotion intensity must be a finite number".to_string()));
            }
        }

        let expression = emotion.to_expression();
        self.set_expression(expression, intensity).await
    }

           async fn send_video_frame(&self, _frame_data: Vec<u8>, _width: u32, _height: u32) -> Result<(), AvatarError> {
               Ok(())
           }

           async fn get_audio_output(&self) -> Result<Option<Vec<u8>>, AvatarError> {
               Ok(None)
           }

           fn supports_vision(&self) -> bool { false }
           fn supports_audio_input(&self) -> bool { false }
           fn supports_tts(&self) -> bool { false }

           fn provider_name(&self) -> &str {
               "LiveAvatar 1.0"
           }
       }

