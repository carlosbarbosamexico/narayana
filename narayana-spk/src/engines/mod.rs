//! TTS engine implementations

pub mod native;
pub mod api;
pub mod piper;
pub mod custom;

use crate::error::SpeechError;
use async_trait::async_trait;
use bytes::Bytes;

/// Trait for TTS engines
#[async_trait]
pub trait TtsEngine: Send + Sync {
    /// Synthesize text to speech audio
    async fn synthesize(&self, text: &str, config: &crate::config::VoiceConfig) -> Result<Bytes, SpeechError>;

    /// Get available voices
    async fn list_voices(&self) -> Result<Vec<String>, SpeechError>;

    /// Check if engine is available
    fn is_available(&self) -> bool;

    /// Get engine name
    fn name(&self) -> &str;
}

