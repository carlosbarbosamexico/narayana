//! Speech synthesizer with caching and queue management

use crate::config::{SpeechConfig, VoiceConfig};
use crate::engines::TtsEngine;
use crate::engines::native::NativeTtsEngine;
use crate::error::SpeechError;
use bytes::Bytes;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Semaphore;
use tracing::{info, debug, warn};

/// Speech synthesizer with caching and queue management
pub struct SpeechSynthesizer {
    config: Arc<SpeechConfig>,
    engine: Arc<dyn TtsEngine>,
    cache: Arc<RwLock<HashMap<String, CachedAudio>>>,
    // Queue management
    queue_semaphore: Arc<Semaphore>,
}

#[derive(Clone)]
struct CachedAudio {
    audio: Bytes,
    timestamp: chrono::DateTime<chrono::Utc>,
    size_bytes: usize,
}

impl SpeechSynthesizer {
    /// Create a new speech synthesizer
    pub fn new(config: SpeechConfig) -> Result<Self, SpeechError> {
        config.validate()
            .map_err(|e| SpeechError::Config(e))?;

        if !config.enabled {
            return Err(SpeechError::Config("Speech synthesis is disabled".to_string()));
        }

        // Initialize engine based on config
        // Clone engine to avoid partial move issues
        let engine_type = config.engine.clone();
        let engine: Arc<dyn TtsEngine> = match engine_type {
            crate::config::TtsEngine::Native => {
                let native_engine = NativeTtsEngine::new_with_config(
                    config.rate,
                    config.volume,
                    config.pitch,
                )?;
                if !native_engine.is_available() {
                    return Err(SpeechError::Engine("Native TTS engine not available".to_string()));
                }
                Arc::new(native_engine)
            }
            crate::config::TtsEngine::OpenAi => {
                let api_config = config.api_config.as_ref()
                    .ok_or_else(|| SpeechError::Engine("API config required for OpenAI TTS".to_string()))?;
                
                let engine = crate::engines::api::ApiTtsEngine::new_openai_with_config(
                    api_config.endpoint.clone(),
                    api_config.api_key.clone(),
                    api_config.model.clone(),
                    api_config.timeout_secs,
                    api_config.retry_config.clone(),
                    config.rate,
                    config.volume,
                    config.pitch,
                )?;
                
                if !engine.is_available() {
                    return Err(SpeechError::Engine("OpenAI TTS not available (API key missing)".to_string()));
                }
                Arc::new(engine)
            }
            crate::config::TtsEngine::GoogleCloud => {
                let api_config = config.api_config.as_ref()
                    .ok_or_else(|| SpeechError::Engine("API config required for Google Cloud TTS".to_string()))?;
                
                let engine = crate::engines::api::ApiTtsEngine::new_google_cloud_with_config(
                    api_config.endpoint.clone(),
                    api_config.api_key.clone(),
                    api_config.model.clone(),
                    api_config.timeout_secs,
                    api_config.retry_config.clone(),
                    config.rate,
                    config.volume,
                    config.pitch,
                )?;
                
                if !engine.is_available() {
                    return Err(SpeechError::Engine("Google Cloud TTS not available (API key missing)".to_string()));
                }
                Arc::new(engine)
            }
            crate::config::TtsEngine::AmazonPolly => {
                let api_config = config.api_config.as_ref()
                    .ok_or_else(|| SpeechError::Engine("API config required for Amazon Polly TTS".to_string()))?;
                
                let engine = crate::engines::api::ApiTtsEngine::new_amazon_polly_with_config(
                    api_config.endpoint.clone(),
                    api_config.api_key.clone(),
                    api_config.model.clone(),
                    api_config.timeout_secs,
                    api_config.retry_config.clone(),
                    config.rate,
                    config.volume,
                    config.pitch,
                )?;
                
                if !engine.is_available() {
                    return Err(SpeechError::Engine("Amazon Polly TTS not available (AWS credentials missing)".to_string()));
                }
                Arc::new(engine)
            }
            crate::config::TtsEngine::Piper => {
                // Piper requires model path or voices directory
                // For now, try to create with default paths
                let piper_engine = crate::engines::piper::PiperTtsEngine::new_with_config(
                    None, // Try to find in PATH
                    None, // No explicit model path
                    None, // No explicit voices directory
                    config.rate,
                    config.volume,
                    config.pitch,
                )?;
                
                if !piper_engine.is_available() {
                    return Err(SpeechError::Engine("Piper TTS not available (executable not found)".to_string()));
                }
                Arc::new(piper_engine)
            }
            crate::config::TtsEngine::Custom(engine_name) => {
                // Custom engines need to be registered via a registry
                // For now, we'll support custom API endpoints via ApiTtsConfig
                // Users can create custom engines by implementing TtsEngine trait
                // and registering them through a custom engine registry
                
                // Check if this is a custom API endpoint
                if let Some(api_config) = &config.api_config {
                    // Create a custom API engine
                    let engine = crate::engines::api::ApiTtsEngine::new_custom_with_config(
                        api_config.endpoint.clone(),
                        api_config.api_key.clone(),
                        api_config.model.clone(),
                        api_config.timeout_secs,
                        api_config.retry_config.clone(),
                        engine_name.clone(),
                        config.rate,
                        config.volume,
                        config.pitch,
                    )?;
                    
                    if !engine.is_available() {
                        return Err(SpeechError::Engine(format!(
                            "Custom TTS engine '{}' not available (check API config)",
                            engine_name
                        )));
                    }
                    Arc::new(engine)
                } else {
                    return Err(SpeechError::Engine(format!(
                        "Custom TTS engine '{}' requires api_config. Provide endpoint and API key.",
                        engine_name
                    )));
                }
            }
        };

        // Create semaphore for queue management (limits concurrent requests)
        let queue_size = config.queue_size;
        let queue_semaphore = Arc::new(Semaphore::new(queue_size));

        Ok(Self {
            config: Arc::new(config),
            engine,
            cache: Arc::new(RwLock::new(HashMap::new())),
            queue_semaphore,
        })
    }

    /// Synthesize text to speech (async, queued)
    /// 
    /// This method uses a queue to limit concurrent synthesis requests.
    /// If the queue is full, the request will wait until a slot becomes available.
    pub async fn speak(&self, text: &str) -> Result<Bytes, SpeechError> {
        self.speak_with_config(text, &self.config.voice).await
    }

    /// Synthesize text with custom voice config
    /// 
    /// This method uses a semaphore-based queue to limit concurrent requests.
    /// The queue size is configured via `SpeechConfig::queue_size`.
    pub async fn speak_with_config(&self, text: &str, voice_config: &VoiceConfig) -> Result<Bytes, SpeechError> {
        // Acquire permit from semaphore (queue management)
        // This will wait if queue is full, preventing resource exhaustion
        let _permit = self.queue_semaphore.acquire().await
            .map_err(|e| SpeechError::Synthesizer(format!("Failed to acquire queue permit: {}", e)))?;
        
        // Drop permit after synthesis completes (automatically released)
        // Use a scope to ensure permit is held during synthesis
        let result = self.synthesize_internal(text, voice_config).await;
        
        // Permit is automatically released when _permit is dropped
        result
    }
    
    /// Internal synthesis method (without queue management)
    async fn synthesize_internal(&self, text: &str, voice_config: &VoiceConfig) -> Result<Bytes, SpeechError> {
        // Validate input
        if text.is_empty() {
            return Err(SpeechError::Synthesizer("Text cannot be empty".to_string()));
        }

        // Check for valid UTF-8
        if text.chars().any(|c| c == '\0') {
            return Err(SpeechError::Synthesizer("Text contains null bytes".to_string()));
        }

        // Limit text length to prevent resource exhaustion
        const MAX_TEXT_LENGTH: usize = 100_000;
        if text.len() > MAX_TEXT_LENGTH {
            return Err(SpeechError::Synthesizer(format!("Text too long (max {} bytes)", MAX_TEXT_LENGTH)));
        }

        // Validate voice config
        if voice_config.language.len() > 32 {
            return Err(SpeechError::Synthesizer("Language code too long (max 32 chars)".to_string()));
        }
        if let Some(ref name) = voice_config.name {
            if name.len() > 256 {
                return Err(SpeechError::Synthesizer("Voice name too long (max 256 chars)".to_string()));
            }
        }

        // Check cache if enabled
        if self.config.enable_cache {
            let cache_key = self.cache_key(text, voice_config);
            let cache_hit = {
                let cache = self.cache.read();
                cache.get(&cache_key).cloned()
            };
            
            if let Some(cached) = cache_hit {
                // Validate cached audio size
                if cached.audio.len() > 10 * 1024 * 1024 { // 10MB max per audio
                    warn!("Cached audio too large, removing from cache and regenerating");
                    // Remove invalid cache entry
                    let mut cache = self.cache.write();
                    cache.remove(&cache_key);
                } else {
                    // Safe string slicing for debug message
                    let preview = if text.len() > 50 {
                        format!("{}...", &text[..50])
                    } else {
                        text.to_string()
                    };
                    debug!("Cache hit for text: {}", preview);
                    return Ok(cached.audio.clone());
                }
            }
        }

        // Synthesize directly
        let audio_result = self.engine.synthesize(text, voice_config).await;

        match audio_result {
            Ok(audio) => {
                // Validate audio size (prevent huge audio files)
                const MAX_AUDIO_SIZE: usize = 10 * 1024 * 1024; // 10MB
                if audio.len() > MAX_AUDIO_SIZE {
                    return Err(SpeechError::Synthesizer(format!(
                        "Generated audio too large ({} bytes, max {} bytes)",
                        audio.len(), MAX_AUDIO_SIZE
                    )));
                }

                // Cache if enabled
                if self.config.enable_cache {
                    let cache_key = self.cache_key(text, voice_config);
                    let size_bytes = audio.len();
                    
                    // Only cache if audio is reasonable size (prevent memory exhaustion)
                    if size_bytes <= MAX_AUDIO_SIZE {
                        {
                            let mut cache = self.cache.write();
                            cache.insert(cache_key, CachedAudio {
                                audio: audio.clone(),
                                timestamp: chrono::Utc::now(),
                                size_bytes,
                            });
                        }
                        self.cleanup_cache();
                    } else {
                        warn!("Audio too large to cache ({} bytes), skipping cache", size_bytes);
                    }
                }
                Ok(audio)
            }
            Err(e) => Err(e),
        }
    }

    /// Generate cache key
    fn cache_key(&self, text: &str, voice_config: &VoiceConfig) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        
        // Limit input to prevent DoS
        let text_bytes = text.as_bytes();
        let text_limit = text_bytes.len().min(100_000);
        hasher.update(&text_bytes[..text_limit]);
        
        let lang_bytes = voice_config.language.as_bytes();
        let lang_limit = lang_bytes.len().min(32);
        hasher.update(&lang_bytes[..lang_limit]);
        
        if let Some(ref name) = voice_config.name {
            let name_bytes = name.as_bytes();
            let name_limit = name_bytes.len().min(256);
            hasher.update(&name_bytes[..name_limit]);
        }
        
        format!("{:x}", hasher.finalize())
    }

    /// Cleanup cache if it exceeds max size
    fn cleanup_cache(&self) {
        // Prevent integer overflow in size calculation
        const MAX_CACHE_SIZE_MB: u64 = 10_000; // 10GB max
        let max_cache_size_mb = self.config.max_cache_size_mb.min(MAX_CACHE_SIZE_MB);
        
        // Use checked arithmetic to prevent overflow
        // If overflow occurs, use a safe default (1GB)
        let max_size_bytes = max_cache_size_mb
            .checked_mul(1024)
            .and_then(|x| x.checked_mul(1024))
            .unwrap_or(1024 * 1024 * 1024) as usize; // Default to 1GB if overflow

        // Calculate total size and collect keys to remove
        let keys_to_remove = {
            let cache = self.cache.read();
            
            // Use checked arithmetic for sum
            // If overflow occurs, assume cache is too large and needs cleanup
            let total_size: usize = cache.values()
                .map(|c| c.size_bytes)
                .try_fold(0usize, |acc, x| acc.checked_add(x))
                .unwrap_or_else(|| {
                    // If overflow, return a value that will trigger cleanup
                    max_size_bytes + 1
                });

            if total_size <= max_size_bytes {
                return; // No cleanup needed
            }

            // Collect entries and sort by timestamp
            // Limit the number of entries we process to prevent memory exhaustion
            const MAX_CACHE_ENTRIES: usize = 100_000;
            let cache_len = cache.len();
            let entries_to_process = cache_len.min(MAX_CACHE_ENTRIES);
            
            let mut entries: Vec<_> = cache.iter()
                .take(entries_to_process)
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            entries.sort_by_key(|(_, cached)| cached.timestamp);

            let mut removed_size = 0usize;
            // Calculate target size as 80% of max using proper percentage calculation
            // Use checked arithmetic to prevent overflow
            let target_size = max_size_bytes
                .checked_mul(80)
                .and_then(|x| x.checked_div(100))
                .unwrap_or(max_size_bytes); // Fallback to max if overflow
            let mut keys_to_remove = Vec::new();
            
            // Limit the number of keys we'll remove to prevent excessive memory allocation
            const MAX_KEYS_TO_REMOVE: usize = 10_000;
            let mut keys_removed_count = 0;

            for (key, cached) in entries {
                // Limit the number of removals
                if keys_removed_count >= MAX_KEYS_TO_REMOVE {
                    break;
                }
                
                // Use checked arithmetic to prevent underflow
                if let Some(remaining) = total_size.checked_sub(removed_size) {
                    if remaining <= target_size {
                        break;
                    }
                } else {
                    break; // Underflow protection
                }
                
                if let Some(new_removed) = removed_size.checked_add(cached.size_bytes) {
                    removed_size = new_removed;
                    keys_to_remove.push(key);
                    keys_removed_count += 1;
                } else {
                    break; // Overflow protection
                }
            }

            keys_to_remove
        };

        // Remove keys
        if !keys_to_remove.is_empty() {
            let mut cache = self.cache.write();
            for key in keys_to_remove {
                cache.remove(&key);
            }
            info!("Cleaned up cache: removed entries to reduce size");
        }
    }
    
    /// Get current queue usage (number of active requests)
    /// Returns the number of permits currently in use
    pub fn queue_usage(&self) -> usize {
        let available = self.queue_semaphore.available_permits();
        self.config.queue_size.saturating_sub(available)
    }
    
    /// Get queue capacity (maximum concurrent requests)
    pub fn queue_capacity(&self) -> usize {
        self.config.queue_size
    }
    
    /// Check if queue is full
    pub fn is_queue_full(&self) -> bool {
        self.queue_semaphore.available_permits() == 0
    }

}

