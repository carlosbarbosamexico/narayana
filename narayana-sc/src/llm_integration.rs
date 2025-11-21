//! LLM integration for voice-to-text

#[cfg(feature = "llm-integration")]
use narayana_llm::LlmEngine;
use crate::error::AudioError;
use bytes::Bytes;
use parking_lot::RwLock;
use std::sync::Arc;
use tracing::{info, warn, debug};

/// LLM-based audio processor for voice-to-text
pub struct LlmAudioProcessor {
    #[cfg(feature = "llm-integration")]
    llm_engine: Arc<RwLock<Option<Arc<dyn LlmEngine>>>>,
    enabled: bool,
}

impl LlmAudioProcessor {
    /// Create a new LLM audio processor
    pub fn new(enabled: bool) -> Self {
        Self {
            #[cfg(feature = "llm-integration")]
            llm_engine: Arc::new(RwLock::new(None)),
            enabled,
        }
    }

    /// Set LLM engine (if LLM integration is enabled)
    #[cfg(feature = "llm-integration")]
    pub fn set_llm_engine(&self, engine: Arc<dyn LlmEngine>) {
        *self.llm_engine.write() = Some(engine);
        info!("LLM engine set for voice-to-text");
    }

    /// Process audio for voice-to-text
    pub async fn process_audio_to_text(&self, audio_data: &Bytes) -> Result<Option<String>, AudioError> {
        if !self.enabled {
            return Ok(None);
        }

        #[cfg(feature = "llm-integration")]
        {
            let engine_guard = self.llm_engine.read();
            if let Some(ref engine) = *engine_guard {
                // Check if engine supports audio input
                // This is a placeholder - actual implementation depends on LLM engine capabilities
                debug!("Processing audio with LLM for voice-to-text");
                
                // Convert audio to text using LLM
                // Note: This requires the LLM engine to support audio input
                // For now, we return None if not supported
                return Ok(None);
            }
        }

        Ok(None)
    }

    /// Check if LLM integration is available
    pub fn is_available(&self) -> bool {
        if !self.enabled {
            return false;
        }

        #[cfg(feature = "llm-integration")]
        {
            let engine_guard = self.llm_engine.read();
            engine_guard.is_some()
        }

        #[cfg(not(feature = "llm-integration"))]
        {
            false
        }
    }
}


