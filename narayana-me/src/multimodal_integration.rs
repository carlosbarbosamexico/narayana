//! Multimodal integration for avatar (vision, audio, TTS)
//! Integrates narayana-eye, narayana-sc, and narayana-spk

#[cfg(feature = "vision")]
use narayana_eye::{VisionAdapter, VisionConfig, ProcessingMode};
#[cfg(feature = "audio-input")]
use narayana_sc::{AudioAdapter, AudioConfig};
#[cfg(feature = "tts")]
use narayana_spk::{SpeechAdapter, SpeechConfig};
use crate::config::AvatarConfig;
use crate::error::AvatarError;
use crate::multimodal::{MultimodalManager, VisionFrame, AudioSample};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, debug};

/// Multimodal integration manager
pub struct MultimodalIntegration {
    manager: Arc<MultimodalManager>,
    #[cfg(feature = "vision")]
    vision_adapter: Option<Arc<RwLock<Box<VisionAdapter>>>>,
    #[cfg(feature = "audio-input")]
    audio_adapter: Option<Arc<RwLock<Box<AudioAdapter>>>>,
    #[cfg(feature = "tts")]
    tts_adapter: Option<Arc<RwLock<Box<SpeechAdapter>>>>,
}

impl MultimodalIntegration {
    /// Create new multimodal integration
    pub fn new(config: &AvatarConfig) -> Result<Self, AvatarError> {
        let manager = Arc::new(MultimodalManager::new());
        
        // Initialize vision if enabled
        #[cfg(feature = "vision")]
        let vision_adapter = if config.enable_vision {
            let vision_config = VisionConfig {
                camera_id: config.vision_config
                    .as_ref()
                    .and_then(|c| c.get("camera_id"))
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32)
                    .unwrap_or(0),
                frame_rate: config.vision_config
                    .as_ref()
                    .and_then(|c| c.get("fps"))
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32)
                    .unwrap_or(30),
                resolution: (
                    config.vision_config
                        .as_ref()
                        .and_then(|c| c.get("width"))
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u32)
                        .unwrap_or(640),
                    config.vision_config
                        .as_ref()
                        .and_then(|c| c.get("height"))
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u32)
                        .unwrap_or(480),
                ),
                enable_detection: true,
                enable_segmentation: false,
                enable_tracking: true,
                enable_scene_understanding: true,
                llm_integration: false,
                model_path: std::path::PathBuf::from("./models"),
                processing_mode: ProcessingMode::RealTime,
            };
            
            match VisionAdapter::new(vision_config) {
                Ok(adapter) => {
                    info!("Vision adapter initialized for avatar");
                    Some(Arc::new(RwLock::new(Box::new(adapter))))
                }
                Err(e) => {
                    warn!("Failed to initialize vision adapter: {}", e);
                    None
                }
            }
        } else {
            None
        };
        
        #[cfg(not(feature = "vision"))]
        let vision_adapter = None;
        
        // Initialize audio input if enabled
        #[cfg(feature = "audio-input")]
        let audio_adapter = if config.enable_audio_input {
            let audio_config = AudioConfig {
                sample_rate: config.audio_input_config
                    .as_ref()
                    .and_then(|c| c.get("sample_rate"))
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32)
                    .unwrap_or(16000),
                channels: config.audio_input_config
                    .as_ref()
                    .and_then(|c| c.get("channels"))
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u8)
                    .unwrap_or(1),
                ..Default::default()
            };
            
            match AudioAdapter::new(audio_config) {
                Ok(adapter) => {
                    info!("Audio input adapter initialized for avatar");
                    Some(Arc::new(RwLock::new(Box::new(adapter))))
                }
                Err(e) => {
                    warn!("Failed to initialize audio input adapter: {}", e);
                    None
                }
            }
        } else {
            None
        };
        
        #[cfg(not(feature = "audio-input"))]
        let audio_adapter = None;
        
        // Initialize TTS if enabled
        #[cfg(feature = "tts")]
        let tts_adapter = if config.enable_tts {
            let mut speech_config = SpeechConfig::default();
            speech_config.enabled = true;
            
            if let Some(tts_cfg) = &config.tts_config {
                if let Some(rate) = tts_cfg.get("rate").and_then(|v| v.as_u64()) {
                    speech_config.rate = rate as u32;
                }
                if let Some(volume) = tts_cfg.get("volume").and_then(|v| v.as_f64()) {
                    speech_config.volume = volume as f32;
                }
                if let Some(voice_lang) = tts_cfg.get("voice").and_then(|v| v.get("language")).and_then(|v| v.as_str()) {
                    speech_config.voice.language = voice_lang.to_string();
                }
            }
            
            match SpeechAdapter::new(speech_config) {
                Ok(adapter) => {
                    info!("TTS adapter initialized for avatar");
                    Some(Arc::new(RwLock::new(Box::new(adapter))))
                }
                Err(e) => {
                    warn!("Failed to initialize TTS adapter: {}", e);
                    None
                }
            }
        } else {
            None
        };
        
        #[cfg(not(feature = "tts"))]
        let tts_adapter = None;
        
        Ok(Self {
            manager,
            #[cfg(feature = "vision")]
            vision_adapter,
            #[cfg(feature = "audio-input")]
            audio_adapter,
            #[cfg(feature = "tts")]
            tts_adapter,
        })
    }
    
    /// Get multimodal manager
    pub fn manager(&self) -> Arc<MultimodalManager> {
        Arc::clone(&self.manager)
    }
    
    /// Start all enabled adapters
    pub async fn start(&self) -> Result<(), AvatarError> {
        #[cfg(feature = "vision")]
        if let Some(ref adapter) = self.vision_adapter {
            // Vision adapter would need world broker handle
            // For now, just log
            info!("Vision adapter ready");
        }
        
        #[cfg(feature = "audio-input")]
        if let Some(ref adapter) = self.audio_adapter {
            // Audio adapter would need world broker handle
            info!("Audio input adapter ready");
        }
        
        #[cfg(feature = "tts")]
        if let Some(ref adapter) = self.tts_adapter {
            // TTS adapter would need world broker handle
            info!("TTS adapter ready");
        }
        
        Ok(())
    }
    
    /// Process video frame
    pub async fn process_video_frame(&self, frame: VisionFrame) -> Result<(), AvatarError> {
        self.manager.send_vision_frame(frame)?;
        Ok(())
    }
    
    /// Process audio sample
    pub async fn process_audio_sample(&self, sample: AudioSample) -> Result<(), AvatarError> {
        self.manager.send_audio_input(sample)?;
        Ok(())
    }
}

