//! Speech adapter for narayana-wld integration

use crate::config::{SpeechConfig, VoiceConfig};
use crate::error::SpeechError;
use crate::synthesizer::SpeechSynthesizer;
use bytes::Bytes;
use narayana_wld::protocol_adapters::ProtocolAdapter;
use narayana_wld::world_broker::WorldBrokerHandle;
use narayana_wld::event_transformer::{WorldEvent, WorldAction};
use narayana_core::Error;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tracing::{info, warn, error, debug};

/// Speech adapter implementing ProtocolAdapter for narayana-wld
pub struct SpeechAdapter {
    config: Arc<SpeechConfig>,
    synthesizer: Arc<RwLock<Option<Arc<SpeechSynthesizer>>>>,
    event_sender: Arc<RwLock<Option<broadcast::Sender<WorldEvent>>>>,
    is_running: Arc<RwLock<bool>>,
    processing_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    request_receiver: Arc<RwLock<Option<mpsc::Receiver<SpeechRequest>>>>,
}

struct SpeechRequest {
    text: String,
    config: crate::config::VoiceConfig,
    response_sender: mpsc::Sender<SpeechResponse>,
}

struct SpeechResponse {
    audio: Result<bytes::Bytes, crate::error::SpeechError>,
}

impl SpeechAdapter {
    /// Create a new speech adapter
    pub fn new(config: SpeechConfig) -> Result<Self, Error> {
        config.validate()
            .map_err(|e| Error::Storage(format!("Invalid speech config: {}", e)))?;

        // Only create synthesizer if enabled
        let synthesizer = if config.enabled {
            match SpeechSynthesizer::new(config.clone()) {
                Ok(synth) => {
                    info!("Speech synthesizer initialized");
                    Some(Arc::new(synth))
                }
                Err(e) => {
                    warn!("Failed to initialize speech synthesizer: {}", e);
                    None
                }
            }
        } else {
            info!("Speech synthesis disabled in config");
            None
        };

        Ok(Self {
            config: Arc::new(config),
            synthesizer: Arc::new(RwLock::new(synthesizer)),
            event_sender: Arc::new(RwLock::new(None)),
            is_running: Arc::new(RwLock::new(false)),
            processing_handle: Arc::new(RwLock::new(None)),
            request_receiver: Arc::new(RwLock::new(None)),
        })
    }
}

#[async_trait]
impl ProtocolAdapter for SpeechAdapter {
    fn protocol_name(&self) -> &str {
        "speech"
    }

    async fn start(&self, _broker: WorldBrokerHandle) -> Result<(), Error> {
        // Check if already running
        {
            let mut is_running = self.is_running.write();
            if *is_running {
                return Err(Error::Storage("Speech adapter already running".to_string()));
            }
            *is_running = true;
        }

        info!("Starting speech adapter");

        // Create event channel
        const EVENT_BUFFER_SIZE: usize = 1000;
        let (sender, _) = broadcast::channel(EVENT_BUFFER_SIZE);
        
        // Set event sender - if this fails, rollback is_running
        *self.event_sender.write() = Some(sender);

        // Synthesizer is ready (processing is done synchronously)
        if self.synthesizer.read().is_some() {
            info!("Speech synthesizer ready");
        }

        info!("Speech adapter started successfully");
        Ok(())
    }

    async fn stop(&self) -> Result<(), Error> {
        {
            let mut is_running = self.is_running.write();
            if !*is_running {
                return Ok(()); // Already stopped
            }
            *is_running = false;
        }

        // Stop processing task
        let handle_opt = {
            let mut guard = self.processing_handle.write();
            guard.take()
        };
        
        if let Some(handle) = handle_opt {
            handle.abort();
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(1),
                handle
            ).await;
        }

        // Clear event sender
        *self.event_sender.write() = None;

        info!("Speech adapter stopped");
        Ok(())
    }

    async fn send_action(&self, action: WorldAction) -> Result<(), Error> {
        // Handle speech commands
        match action {
            WorldAction::ActuatorCommand { target, command } => {
                // Validate target length to prevent DoS
                if target.len() > 256 {
                    warn!("Target name too long, ignoring");
                    return Ok(());
                }
                
                if target == "speech" || target.starts_with("speech_") {
                    // Validate command JSON size
                    let command_size = serde_json::to_string(&command)
                        .map(|s| s.len())
                        .unwrap_or(0);
                    const MAX_COMMAND_SIZE: usize = 200_000; // 200KB max
                    if command_size > MAX_COMMAND_SIZE {
                        warn!("Command too large ({} bytes, max {} bytes), ignoring", command_size, MAX_COMMAND_SIZE);
                        return Ok(());
                    }
                    
                    debug!("Received speech command: {:?}", command);
                    
                    // Parse command with validation
                    if let Some(text) = command.get("text")
                        .and_then(|v| v.as_str())
                        .or_else(|| command.get("message").and_then(|v| v.as_str())) {
                        
                        // Validate and sanitize text
                        if text.is_empty() {
                            warn!("Empty text in speech command, ignoring");
                            return Ok(());
                        }
                        
                        // Check for null bytes
                        if text.contains('\0') {
                            warn!("Text contains null bytes, ignoring");
                            return Ok(());
                        }
                        
                        // Limit text length and prepare text to speak
                        // Ensure we don't break UTF-8 boundaries when truncating
                        let text_to_speak = if text.len() > 100_000 {
                            warn!("Text too long ({} bytes, max 100KB), truncating", text.len());
                            // Find the last valid UTF-8 boundary before 100KB
                            let mut truncate_at = 100_000;
                            while !text.is_char_boundary(truncate_at) && truncate_at > 0 {
                                truncate_at -= 1;
                            }
                            &text[..truncate_at]
                        } else {
                            text
                        };
                        
                        // Clone synthesizer reference to avoid holding lock across await
                        let synth_opt = {
                            let synth_guard = self.synthesizer.read();
                            synth_guard.as_ref().map(|s| Arc::clone(s))
                        };
                        
                        if let Some(synth) = synth_opt {
                            // Synthesize speech
                            let audio_result = synth.speak(text_to_speak).await;
                            
                            match audio_result {
                                Ok(audio) => {
                                    info!("Speech synthesized successfully: {} bytes", audio.len());
                                    
                                    // Send event
                                    let event_opt = {
                                        let sender_guard = self.event_sender.read();
                                        sender_guard.as_ref().map(|s| s.clone())
                                    };
                                    
                                    if let Some(sender) = event_opt {
                                        // Sanitize text for JSON (limit length, remove control chars)
                                        let sanitized_text: String = text_to_speak
                                            .chars()
                                            .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
                                            .take(1000) // Limit in event
                                            .collect();
                                        
                                        // Safely convert timestamp to u64, handling overflow and negative values
                                        let timestamp = chrono::Utc::now()
                                            .timestamp_nanos_opt()
                                            .and_then(|ts| {
                                                if ts >= 0 {
                                                    ts.try_into().ok() // Convert i64 to u64
                                                } else {
                                                    None // Negative timestamps not supported
                                                }
                                            })
                                            .unwrap_or(0u64);
                                        
                                        let event = WorldEvent::SensorData {
                                            source: "speech".to_string(),
                                            data: json!({
                                                "type": "audio",
                                                "status": "synthesized",
                                                "text": sanitized_text,
                                                "text_length": text_to_speak.len(),
                                                "audio_size": audio.len(),
                                                "timestamp": timestamp,
                                            }),
                                            timestamp,
                                        };
                                        
                                        // Use try_send to avoid blocking
                                        if sender.send(event).is_err() {
                                            warn!("Failed to send speech event (channel full)");
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Speech synthesis failed: {}", e);
                                }
                            }
                        } else {
                            warn!("Speech synthesizer not available");
                        }
                    } else {
                        warn!("Speech command missing 'text' or 'message' field");
                    }
                }
            }
            _ => {
                // Other actions not handled by speech adapter
            }
        }

        Ok(())
    }

    fn subscribe_events(&self) -> broadcast::Receiver<WorldEvent> {
        if let Some(ref sender) = *self.event_sender.read() {
            sender.subscribe()
        } else {
            // Return a closed channel if not started
            let (_, receiver) = broadcast::channel(1);
            receiver
        }
    }
}

