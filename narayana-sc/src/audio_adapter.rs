//! Audio adapter for narayana-wld integration

use crate::audio_analyzer::{AudioAnalyzer, AudioAnalysis};
use crate::audio_capture::AudioCapture;
use crate::config::AudioConfig;
use crate::error::AudioError;
use crate::llm_integration::LlmAudioProcessor;
use crate::advanced_features::AdvancedAudioProcessor;
use bytes::Bytes;
use narayana_core::Error;
use narayana_wld::protocol_adapters::ProtocolAdapter;
use narayana_wld::world_broker::WorldBrokerHandle;
use narayana_wld::event_transformer::{WorldEvent, WorldAction};
use async_trait::async_trait;
use parking_lot::RwLock;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tracing::{info, warn, error, debug};
use futures::future;

/// Audio adapter implementing ProtocolAdapter for narayana-wld
/// 2025: Enhanced with advanced audio processing
pub struct AudioAdapter {
    config: Arc<AudioConfig>,
    capture: Arc<RwLock<Option<Arc<AudioCapture>>>>,
    analyzer: Arc<RwLock<Option<Arc<AudioAnalyzer>>>>,
    llm_processor: Arc<LlmAudioProcessor>,
    advanced_processor: Arc<RwLock<Option<AdvancedAudioProcessor>>>,
    event_sender: Arc<RwLock<Option<broadcast::Sender<WorldEvent>>>>,
    is_running: Arc<RwLock<bool>>,
    processing_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    audio_receiver: Arc<RwLock<Option<mpsc::Receiver<Bytes>>>>,
}

impl AudioAdapter {
    /// Create a new audio adapter
    pub fn new(config: AudioConfig) -> Result<Self, Error> {
        config.validate()
            .map_err(|e| Error::Storage(format!("Invalid audio config: {}", e)))?;

        // Create audio capture if enabled
        let capture: Option<Arc<AudioCapture>> = if config.enabled {
            match AudioCapture::new(
                config.capture.clone(),
                config.sample_rate,
                config.channels,
            ) {
                Ok(cap) => {
                    info!("Audio capture initialized");
                    Some(Arc::new(cap))
                }
                Err(e) => {
                    warn!("Failed to initialize audio capture: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Create audio analyzer
        let analyzer: Option<Arc<AudioAnalyzer>> = if config.analysis.enable_fft || config.analysis.enable_energy {
            match AudioAnalyzer::new(config.analysis.clone(), config.sample_rate) {
                Ok(ana) => {
                    info!("Audio analyzer initialized");
                    Some(Arc::new(ana))
                }
                Err(e) => {
                    warn!("Failed to initialize audio analyzer: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Create LLM processor
        let llm_processor = Arc::new(LlmAudioProcessor::new(config.enable_llm_vtt));

        // Create advanced audio processor for comprehensive capture
        let advanced_processor = if config.enabled {
            Some(AdvancedAudioProcessor::new(&config.capture, &config.analysis))
        } else {
            None
        };

        Ok(Self {
            config: Arc::new(config),
            capture: Arc::new(RwLock::new(capture)),
            analyzer: Arc::new(RwLock::new(analyzer)),
            llm_processor,
            advanced_processor: Arc::new(RwLock::new(advanced_processor)),
            event_sender: Arc::new(RwLock::new(None)),
            is_running: Arc::new(RwLock::new(false)),
            processing_handle: Arc::new(RwLock::new(None)),
            audio_receiver: Arc::new(RwLock::new(None)),
        })
    }
}

#[async_trait]
impl ProtocolAdapter for AudioAdapter {
    fn protocol_name(&self) -> &str {
        "audio"
    }

    async fn start(&self, _broker: WorldBrokerHandle) -> Result<(), Error> {
        {
            let mut is_running = self.is_running.write();
            if *is_running {
                return Err(Error::Storage("Audio adapter already running".to_string()));
            }
            *is_running = true;
        }

        info!("Starting audio adapter");

        // Create event channel
        const EVENT_BUFFER_SIZE: usize = 1000;
        let (sender, _) = broadcast::channel(EVENT_BUFFER_SIZE);
        *self.event_sender.write() = Some(sender);

        // Create audio channel
        let (audio_tx, audio_rx) = mpsc::channel(1000);
        *self.audio_receiver.write() = Some(audio_rx);

        // Start audio capture if available
        {
            let capture_guard = self.capture.read();
            if let Some(ref capture) = *capture_guard {
                if let Err(e) = capture.start(audio_tx.clone()) {
                    warn!("Failed to start audio capture: {}", e);
                }
            }
        }

        // Start processing task
        // Extract receiver before moving into task
        let rx_opt = {
            let mut receiver_guard = self.audio_receiver.write();
            receiver_guard.take()
        };
        
        let analyzer = self.analyzer.clone();
        let llm_processor = self.llm_processor.clone();
        let event_sender = self.event_sender.clone();
        let is_running = self.is_running.clone();
        let config = self.config.clone();

        let handle = tokio::spawn(async move {
            let mut analysis_interval = interval(Duration::from_millis(config.analysis.analysis_interval_ms));
            let mut audio_buffer = Vec::new();

            // Handle receiver properly
            if let Some(mut rx) = rx_opt {
                loop {
                    tokio::select! {
                        // Check if we should stop
                        _ = tokio::time::sleep(Duration::from_millis(100)) => {
                            if !*is_running.read() {
                                break;
                            }
                        }
                        // Receive audio data
                        audio_opt = rx.recv() => {
                            if let Some(audio_data) = audio_opt {
                                audio_buffer.push(audio_data);
                                
                                // Process when buffer is large enough or interval elapsed
                                if audio_buffer.len() >= 10 {
                                    Self::process_audio_batch(
                                        &audio_buffer,
                                        &analyzer,
                                        &llm_processor,
                                        &event_sender,
                                        &config,
                                    ).await;
                                    audio_buffer.clear();
                                }
                            }
                        }
                        // Analysis interval
                        _ = analysis_interval.tick() => {
                            if !audio_buffer.is_empty() {
                                Self::process_audio_batch(
                                    &audio_buffer,
                                    &analyzer,
                                    &llm_processor,
                                    &event_sender,
                                    &config,
                                ).await;
                                audio_buffer.clear();
                            }
                        }
                    }
                }
            } else {
                // No receiver, just run analysis interval
                loop {
                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_millis(100)) => {
                            if !*is_running.read() {
                                break;
                            }
                        }
                        _ = analysis_interval.tick() => {
                            if !audio_buffer.is_empty() {
                                Self::process_audio_batch(
                                    &audio_buffer,
                                    &analyzer,
                                    &llm_processor,
                                    &event_sender,
                                    &config,
                                ).await;
                                audio_buffer.clear();
                            }
                        }
                    }
                }
            }
        });

        *self.processing_handle.write() = Some(handle);
        info!("Audio adapter started successfully");

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

        // Stop audio capture
        if let Some(ref capture) = *self.capture.read() {
            let _ = capture.stop();
        }

        // Stop processing task
        let handle_opt = {
            let mut guard = self.processing_handle.write();
            guard.take()
        };

        if let Some(handle) = handle_opt {
            handle.abort();
            let _ = tokio::time::timeout(
                Duration::from_secs(1),
                handle
            ).await;
        }

        // Clear channels
        *self.event_sender.write() = None;
        *self.audio_receiver.write() = None;

        info!("Audio adapter stopped");
        Ok(())
    }

    async fn send_action(&self, _action: WorldAction) -> Result<(), Error> {
        // Audio adapter doesn't handle actions, only emits events
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

impl AudioAdapter {
    /// Process a batch of audio data - 2025: Enhanced with advanced processing
    async fn process_audio_batch(
        audio_buffer: &[Bytes],
        analyzer: &Arc<RwLock<Option<Arc<AudioAnalyzer>>>>,
        llm_processor: &Arc<LlmAudioProcessor>,
        event_sender: &Arc<RwLock<Option<broadcast::Sender<WorldEvent>>>>,
        config: &Arc<AudioConfig>,
    ) {
        // Combine audio buffer
        let mut combined_audio: Bytes = audio_buffer.iter()
            .flat_map(|b| b.iter().copied())
            .collect::<Vec<u8>>()
            .into();

        // Process with LLM for voice-to-text
        let text_result = if config.enable_llm_vtt {
            llm_processor.process_audio_to_text(&combined_audio).await
        } else {
            Ok(None)
        };

        let text = match text_result {
            Ok(Some(t)) => {
                info!("Voice-to-text: {}", t);
                Some(t)
            }
            Ok(None) => None,
            Err(e) => {
                warn!("LLM voice-to-text error: {}", e);
                None
            }
        };

        // Analyze audio
        let analysis_result = {
            let analyzer_guard = analyzer.read();
            if let Some(ref analyzer) = *analyzer_guard {
                analyzer.analyze(&combined_audio)
            } else {
                Err(AudioError::Analysis("Analyzer not available".to_string()))
            }
        };

        // Emit events
        let sender_guard = event_sender.read();
        if let Some(ref sender) = *sender_guard {
            let timestamp = chrono::Utc::now()
                .timestamp_nanos_opt()
                .and_then(|ts| {
                    if ts >= 0 {
                        ts.try_into().ok()
                    } else {
                        None
                    }
                })
                .unwrap_or(0u64);

            // Emit text event if available
            if let Some(ref text) = text {
                let event = WorldEvent::SensorData {
                    source: "audio".to_string(),
                    data: json!({
                        "type": "voice_to_text",
                        "text": text,
                        "timestamp": timestamp,
                    }),
                    timestamp,
                };

                if sender.send(event).is_err() {
                    debug!("Failed to send voice-to-text event (channel full)");
                }
            }

            // Emit analysis event
            match analysis_result {
                Ok(analysis) => {
                    let event = WorldEvent::SensorData {
                        source: "audio".to_string(),
                        data: AudioAnalyzer::analysis_to_json(&analysis),
                        timestamp,
                    };

                    if sender.send(event).is_err() {
                        debug!("Failed to send audio analysis event (channel full)");
                    }
                }
                Err(e) => {
                    debug!("Audio analysis error: {}", e);
                }
            }
        }
    }
}

