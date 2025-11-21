//! Multimodal capabilities for avatar (vision, audio input, TTS)

use crate::error::AvatarError;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::broadcast;
use tracing::warn;

/// Vision frame data
#[derive(Debug, Clone)]
pub struct VisionFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub timestamp: u64,
}

/// Audio input sample
#[derive(Debug, Clone)]
pub struct AudioSample {
    pub data: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u8,
    pub timestamp: u64,
}

/// TTS audio output
#[derive(Debug, Clone)]
pub struct TTSAudio {
    pub data: Vec<u8>,
    pub format: AudioFormat,
    pub sample_rate: u32,
}

/// Audio format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    Wav,
    Pcm,
    Opus,
}

/// Multimodal manager for avatar
pub struct MultimodalManager {
    vision_sender: broadcast::Sender<VisionFrame>,
    audio_input_sender: broadcast::Sender<AudioSample>,
    tts_audio_sender: broadcast::Sender<TTSAudio>,
}

impl MultimodalManager {
    pub fn new() -> Self {
        let (vision_sender, _) = broadcast::channel(100);
        let (audio_input_sender, _) = broadcast::channel(1000);
        let (tts_audio_sender, _) = broadcast::channel(100);
        
        Self {
            vision_sender,
            audio_input_sender,
            tts_audio_sender,
        }
    }
    
    /// Send TTS audio output
    pub fn send_tts_audio(&self, audio: TTSAudio) -> Result<(), AvatarError> {
        if self.tts_audio_sender.send(audio).is_err() {
            warn!("TTS audio broadcast channel full, dropping audio");
        }
        Ok(())
    }

    /// Send vision frame
    pub fn send_vision_frame(&self, frame: VisionFrame) -> Result<(), AvatarError> {
        if self.vision_sender.send(frame).is_err() {
            warn!("Vision frame broadcast channel full, dropping frame");
        }
        Ok(())
    }

    /// Send audio input sample
    pub fn send_audio_input(&self, sample: AudioSample) -> Result<(), AvatarError> {
        if self.audio_input_sender.send(sample).is_err() {
            warn!("Audio input broadcast channel full, dropping sample");
        }
        Ok(())
    }

    /// Subscribe to vision frames
    pub fn subscribe_vision(&self) -> broadcast::Receiver<VisionFrame> {
        self.vision_sender.subscribe()
    }

    /// Subscribe to audio input
    pub fn subscribe_audio_input(&self) -> broadcast::Receiver<AudioSample> {
        self.audio_input_sender.subscribe()
    }

    /// Subscribe to TTS audio output
    pub fn subscribe_tts_audio(&self) -> broadcast::Receiver<TTSAudio> {
        self.tts_audio_sender.subscribe()
    }
}

