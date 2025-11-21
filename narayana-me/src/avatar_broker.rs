//! Avatar broker - unified API for avatar providers

use crate::config::{AvatarConfig, Expression, Gesture, Emotion};
use crate::error::AvatarError;
use async_trait::async_trait;
use bytes::Bytes;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Avatar stream information
pub struct AvatarStream {
    pub stream_id: String,
    pub client_url: String,
    pub handle: Box<dyn std::any::Any + Send + Sync>,
}

/// Trait for avatar providers
#[async_trait]
pub trait AvatarProvider: Send + Sync {
    async fn initialize(&mut self, config: &AvatarConfig) -> Result<(), AvatarError>;
    async fn start_stream(&mut self) -> Result<AvatarStream, AvatarError>;
    async fn stop_stream(&mut self) -> Result<(), AvatarError>;
    async fn send_audio(&self, audio_data: Vec<u8>) -> Result<(), AvatarError>;
    async fn set_expression(&self, expression: Expression, intensity: f64) -> Result<(), AvatarError>;
    async fn set_gesture(&self, gesture: Gesture, duration_ms: u64) -> Result<(), AvatarError>;
    async fn update_emotion(&self, emotion: Emotion, intensity: f64) -> Result<(), AvatarError>;
    fn provider_name(&self) -> &str;
    
    // Multimodal capabilities
    /// Send video frame for vision processing (if enable_vision is true)
    async fn send_video_frame(&self, frame_data: Vec<u8>, width: u32, height: u32) -> Result<(), AvatarError>;
    /// Get audio output for TTS playback (if enable_tts is true)
    async fn get_audio_output(&self) -> Result<Option<Vec<u8>>, AvatarError>;
    /// Check if provider supports vision
    fn supports_vision(&self) -> bool { false }
    /// Check if provider supports audio input
    fn supports_audio_input(&self) -> bool { false }
    /// Check if provider supports TTS
    fn supports_tts(&self) -> bool { false }
}

/// Avatar broker - unified API facade for avatar providers
pub struct AvatarBroker {
    provider_type: crate::config::AvatarProviderType,
    provider: Arc<RwLock<Option<Arc<RwLock<Box<dyn AvatarProvider>>>>>>,
    stream: Arc<RwLock<Option<AvatarStream>>>,
    config: Arc<AvatarConfig>,
}

impl AvatarBroker {
    /// Create a new avatar broker
    pub fn new(config: AvatarConfig) -> Result<Self, AvatarError> {
        config.validate().map_err(|e| AvatarError::Config(e))?;
        Ok(Self {
            provider_type: config.provider.clone(),
            provider: Arc::new(RwLock::new(None)),
            stream: Arc::new(RwLock::new(None)),
            config: Arc::new(config),
        })
    }

    /// Initialize the avatar provider
    pub async fn initialize(&self) -> Result<(), AvatarError> {
        if !self.config.enabled {
            return Ok(());
        }

        // Check if already initialized (idempotent)
        {
            let provider_guard = self.provider.read().await;
            if provider_guard.is_some() {
                warn!("Provider already initialized, skipping");
                return Ok(());
            }
        }

        let provider = self.create_provider().await?;
        let provider_arc = Arc::new(RwLock::new(provider));
        
        {
            let mut provider_guard = provider_arc.write().await;
            provider_guard.initialize(&self.config).await?;
        }
        
        *self.provider.write().await = Some(provider_arc);
        info!("Avatar provider initialized: {:?}", self.provider_type);
        Ok(())
    }

    /// Start the avatar stream
    pub async fn start_stream(&self) -> Result<String, AvatarError> {
        // Check if stream already started (idempotent)
        {
            let stream_guard = self.stream.read().await;
            if let Some(ref existing_stream) = *stream_guard {
                warn!("Stream already started: {}, returning existing URL", existing_stream.stream_id);
                return Ok(existing_stream.client_url.clone());
            }
        }

        let provider_arc = {
            let provider_guard = self.provider.read().await;
            provider_guard.as_ref().map(Arc::clone)
        };

        if let Some(provider_arc) = provider_arc {
            let stream = {
                let mut provider_guard = provider_arc.write().await;
                provider_guard.start_stream().await?
            };
            let client_url = stream.client_url.clone();
            let stream_id = stream.stream_id.clone();
            *self.stream.write().await = Some(stream);
            info!("Avatar stream started: {}", stream_id);
            Ok(client_url)
        } else {
            Err(AvatarError::Broker("Provider not initialized".to_string()))
        }
    }

    /// Stop the avatar stream
    pub async fn stop_stream(&self) -> Result<(), AvatarError> {
        let stream_opt = {
            let mut stream_guard = self.stream.write().await;
            stream_guard.take()
        };

        if stream_opt.is_none() {
            warn!("Attempted to stop stream when no stream is active");
            return Ok(()); // Idempotent
        }

        let provider_arc = {
            let provider_guard = self.provider.read().await;
            provider_guard.as_ref().map(Arc::clone)
        };

        if let Some(provider_arc) = provider_arc {
            let mut provider_guard = provider_arc.write().await;
            provider_guard.stop_stream().await?;
        }

        info!("Avatar stream stopped");
        Ok(())
    }

    /// Send audio data for lip sync
    pub async fn send_audio(&self, audio_data: Vec<u8>) -> Result<(), AvatarError> {
        if !self.config.enable_lip_sync {
            return Ok(());
        }

        if audio_data.is_empty() {
            return Ok(());
        }

        // Validate audio data size
        const MAX_AUDIO_SIZE: usize = 10 * 1024 * 1024; // 10MB max
        if audio_data.len() > MAX_AUDIO_SIZE {
            warn!("Audio data too large ({} bytes, max {} bytes), rejecting", audio_data.len(), MAX_AUDIO_SIZE);
            return Err(AvatarError::Config(format!("Audio data too large (max {} bytes)", MAX_AUDIO_SIZE)));
        }

        let provider_arc = {
            let provider_guard = self.provider.read().await;
            provider_guard.as_ref().map(Arc::clone)
        };

        if let Some(provider_arc) = provider_arc {
            let provider_guard = provider_arc.read().await;
            provider_guard.send_audio(audio_data).await
        } else {
            Err(AvatarError::Broker("Provider not initialized".to_string()))
        }
    }

    /// Set facial expression
    pub async fn set_expression(&self, expression: Expression, intensity: f64) -> Result<(), AvatarError> {
        // Validate intensity
        if !intensity.is_finite() {
            return Err(AvatarError::Config("Intensity must be a finite number".to_string()));
        }

        // Clamp intensity before applying sensitivity
        let intensity = intensity.clamp(-10.0, 10.0);
        let intensity = (intensity * self.config.expression_sensitivity).clamp(0.0, 1.0);

        let provider_arc = {
            let provider_guard = self.provider.read().await;
            provider_guard.as_ref().map(Arc::clone)
        };

        if let Some(provider_arc) = provider_arc {
            let provider_guard = provider_arc.read().await;
            provider_guard.set_expression(expression, intensity).await
        } else {
            Err(AvatarError::Broker("Provider not initialized".to_string()))
        }
    }

    /// Set gesture
    pub async fn set_gesture(&self, gesture: Gesture, duration_ms: u64) -> Result<(), AvatarError> {
        if !self.config.enable_gestures {
            return Ok(());
        }

        // Validate duration
        const MAX_GESTURE_DURATION_MS: u64 = 300_000; // 5 minutes max
        let duration_ms = duration_ms.min(MAX_GESTURE_DURATION_MS);

        let provider_arc = {
            let provider_guard = self.provider.read().await;
            provider_guard.as_ref().map(Arc::clone)
        };

        if let Some(provider_arc) = provider_arc {
            let provider_guard = provider_arc.read().await;
            provider_guard.set_gesture(gesture, duration_ms).await
        } else {
            Err(AvatarError::Broker("Provider not initialized".to_string()))
        }
    }

    /// Update emotion (maps to expression)
    pub async fn update_emotion(&self, emotion: Emotion, intensity: f64) -> Result<(), AvatarError> {
        // Validate intensity
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

    /// Get current client URL
    pub async fn get_client_url(&self) -> Option<String> {
        self.stream.read().await.as_ref().map(|s| s.client_url.clone())
    }

    /// Create provider based on config
    async fn create_provider(&self) -> Result<Box<dyn AvatarProvider>, AvatarError> {
        match self.provider_type {
            crate::config::AvatarProviderType::BeyondPresence => {
                #[cfg(feature = "beyond-presence")]
                {
                    Ok(Box::new(crate::providers::beyond_presence::BeyondPresenceProvider::new(
                        (*self.config).clone(),
                    ).await?))
                }
                #[cfg(not(feature = "beyond-presence"))]
                {
                    Err(AvatarError::Provider(
                        "Beyond Presence provider not enabled. Enable 'beyond-presence' feature.".to_string()
                    ))
                }
            }
            crate::config::AvatarProviderType::LiveAvatar => {
                Ok(Box::new(crate::providers::live_avatar::LiveAvatarProvider::new(
                    (*self.config).clone(),
                ).await?))
            }
            crate::config::AvatarProviderType::ReadyPlayerMe => {
                Ok(Box::new(crate::providers::ready_player_me::ReadyPlayerMeProvider::new(
                    (*self.config).clone(),
                ).await?))
            }
            crate::config::AvatarProviderType::AvatarSDK => {
                Ok(Box::new(crate::providers::avatar_sdk::AvatarSDKProvider::new(
                    (*self.config).clone(),
                ).await?))
            }
            crate::config::AvatarProviderType::OpenAvatarChat => {
                Ok(Box::new(crate::providers::open_avatar_chat::OpenAvatarChatProvider::new(
                    (*self.config).clone(),
                ).await?))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AvatarError;

    struct MockProvider {
        initialized: bool,
        stream_started: bool,
    }

    #[async_trait]
    impl AvatarProvider for MockProvider {
        async fn initialize(&mut self, _config: &AvatarConfig) -> Result<(), AvatarError> {
            self.initialized = true;
            Ok(())
        }

        async fn start_stream(&mut self) -> Result<AvatarStream, AvatarError> {
            if !self.initialized {
                return Err(AvatarError::Broker("Not initialized".to_string()));
            }
            self.stream_started = true;
            Ok(AvatarStream {
                stream_id: "test_stream".to_string(),
                client_url: "ws://test/stream".to_string(),
                handle: Box::new(()),
            })
        }

        async fn stop_stream(&mut self) -> Result<(), AvatarError> {
            self.stream_started = false;
            Ok(())
        }

        async fn send_audio(&self, _audio_data: Vec<u8>) -> Result<(), AvatarError> {
            Ok(())
        }

        async fn set_expression(&self, _expression: Expression, _intensity: f64) -> Result<(), AvatarError> {
            Ok(())
        }

        async fn set_gesture(&self, _gesture: Gesture, _duration_ms: u64) -> Result<(), AvatarError> {
            Ok(())
        }

        async fn update_emotion(&self, _emotion: Emotion, _intensity: f64) -> Result<(), AvatarError> {
            Ok(())
        }

        fn provider_name(&self) -> &str {
            "MockProvider"
        }
    }

    #[tokio::test]
    async fn test_avatar_broker_internal_structure() {
        let config = AvatarConfig::default();
        let broker = AvatarBroker::new(config).unwrap();
        
        // Test internal methods work
        assert!(broker.get_client_url().await.is_none());
    }
}
