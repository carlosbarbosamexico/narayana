//! Custom TTS engine implementation
//! Allows users to provide their own TTS engine implementations

use crate::config::VoiceConfig;
use crate::error::SpeechError;
use crate::engines::TtsEngine;
use async_trait::async_trait;
use bytes::Bytes;
use std::sync::Arc;
use tracing::warn;

/// Custom TTS engine wrapper
/// Allows users to provide their own TTS engine implementation
pub struct CustomTtsEngine {
    name: String,
    synthesize_fn: Arc<dyn Fn(&str, &VoiceConfig) -> Result<Bytes, SpeechError> + Send + Sync>,
    list_voices_fn: Arc<dyn Fn() -> Result<Vec<String>, SpeechError> + Send + Sync>,
    is_available_fn: Arc<dyn Fn() -> bool + Send + Sync>,
}

impl CustomTtsEngine {
    /// Create a new custom TTS engine
    pub fn new<F1, F2, F3>(
        name: String,
        synthesize_fn: F1,
        list_voices_fn: F2,
        is_available_fn: F3,
    ) -> Self
    where
        F1: Fn(&str, &VoiceConfig) -> Result<Bytes, SpeechError> + Send + Sync + 'static,
        F2: Fn() -> Result<Vec<String>, SpeechError> + Send + Sync + 'static,
        F3: Fn() -> bool + Send + Sync + 'static,
    {
        Self {
            name,
            synthesize_fn: Arc::new(synthesize_fn),
            list_voices_fn: Arc::new(list_voices_fn),
            is_available_fn: Arc::new(is_available_fn),
        }
    }

    /// Create a custom engine from async functions
    /// 
    /// # Safety
    /// This method should only be used when the custom engine will be called from
    /// a synchronous context or when you're certain the runtime handle won't cause deadlocks.
    /// For async contexts, prefer using the async TtsEngine trait directly.
    pub fn from_async<F1, F2, F3>(
        name: String,
        synthesize_fn: F1,
        list_voices_fn: F2,
        is_available_fn: F3,
    ) -> Result<Self, SpeechError>
    where
        F1: Fn(&str, &VoiceConfig) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Bytes, SpeechError>> + Send>> + Send + Sync + 'static,
        F2: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>, SpeechError>> + Send>> + Send + Sync + 'static,
        F3: Fn() -> bool + Send + Sync + 'static,
    {
        // Try to get current runtime handle, but don't create a new one
        // Creating a new runtime can cause deadlocks if called from within async context
        let handle = tokio::runtime::Handle::try_current()
            .map_err(|_| SpeechError::Engine(
                "No tokio runtime available. Custom async engines must be used within a tokio runtime context.".to_string()
            ))?;

        // WARNING: Using block_on from within an async context can cause deadlocks.
        // This implementation assumes the custom engine will be called from a blocking context
        // (e.g., from synchronous code or via spawn_blocking). If called directly from async
        // code, it may deadlock. Users should prefer using the TtsEngine trait directly for async contexts.

        let synthesize_fn = Arc::new(synthesize_fn);
        let synthesize_wrapper = {
            let handle = handle.clone();
            let synthesize_fn = synthesize_fn.clone();
            Arc::new(move |text: &str, config: &VoiceConfig| -> Result<Bytes, SpeechError> {
                let future = synthesize_fn(text, config);
                // Use block_on - this is safe if called from blocking context
                // but may deadlock if called from async context
                handle.block_on(future)
            })
        };

        let list_voices_fn = Arc::new(list_voices_fn);
        let list_voices_wrapper = {
            let handle = handle.clone();
            let list_voices_fn = list_voices_fn.clone();
            Arc::new(move || -> Result<Vec<String>, SpeechError> {
                let future = list_voices_fn();
                handle.block_on(future)
            })
        };

        Ok(Self {
            name,
            synthesize_fn: synthesize_wrapper,
            list_voices_fn: list_voices_wrapper,
            is_available_fn: Arc::new(is_available_fn),
        })
    }
}

#[async_trait]
impl TtsEngine for CustomTtsEngine {
    async fn synthesize(&self, text: &str, config: &VoiceConfig) -> Result<Bytes, SpeechError> {
        // Validate input
        if text.is_empty() {
            return Err(SpeechError::Engine("Text cannot be empty".to_string()));
        }

        if text.len() > 100_000 {
            return Err(SpeechError::Engine("Text too long (max 100KB)".to_string()));
        }

        // Call custom synthesize function
        (self.synthesize_fn)(text, config)
    }

    async fn list_voices(&self) -> Result<Vec<String>, SpeechError> {
        (self.list_voices_fn)()
    }

    fn is_available(&self) -> bool {
        (self.is_available_fn)()
    }

    fn name(&self) -> &str {
        &self.name
    }
}


