//! Comprehensive audio capture for robots - "hear it all"
//! 2025: Complete audio capture system with all advanced features

use crate::config::AudioConfig;
use crate::error::AudioError;
use crate::advanced_features::AdvancedAudioProcessor;
use crate::audio_analyzer::AudioAnalyzer;
use bytes::Bytes;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{info, debug, warn};

/// Comprehensive audio capture system
/// Provides complete audio processing pipeline for robots
pub struct ComprehensiveAudioCapture {
    config: Arc<AudioConfig>,
    advanced_processor: Arc<RwLock<Option<AdvancedAudioProcessor>>>,
    analyzer: Arc<RwLock<Option<AudioAnalyzer>>>,
    /// Capture statistics
    stats: Arc<RwLock<CaptureStats>>,
}

/// Capture statistics
#[derive(Debug, Clone)]
pub struct CaptureStats {
    pub total_samples_processed: u64,
    pub total_events_detected: u64,
    pub voice_activity_detected: u64,
    pub noise_reduced_samples: u64,
    pub agc_adjustments: u64,
    pub average_latency_ms: f64,
}

impl ComprehensiveAudioCapture {
    /// Create new comprehensive capture system
    pub fn new(config: AudioConfig) -> Result<Self, AudioError> {
        config.validate()
            .map_err(|e| AudioError::Config(e))?;

        let advanced_processor = if config.enabled {
            Some(AdvancedAudioProcessor::new(&config.capture, &config.analysis))
        } else {
            None
        };

        let analyzer = if config.analysis.enable_fft || config.analysis.enable_energy {
            match AudioAnalyzer::new(config.analysis.clone(), config.sample_rate) {
                Ok(ana) => Some(ana),
                Err(e) => {
                    warn!("Failed to initialize analyzer: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Ok(Self {
            config: Arc::new(config),
            advanced_processor: Arc::new(RwLock::new(advanced_processor)),
            analyzer: Arc::new(RwLock::new(analyzer)),
            stats: Arc::new(RwLock::new(CaptureStats {
                total_samples_processed: 0,
                total_events_detected: 0,
                voice_activity_detected: 0,
                noise_reduced_samples: 0,
                agc_adjustments: 0,
                average_latency_ms: 0.0,
            })),
        })
    }

    /// Process audio comprehensively - "hear it all"
    pub fn process_comprehensive(&self, audio_data: &Bytes) -> Result<ProcessedAudio, AudioError> {
        let start_time = std::time::Instant::now();

        // Convert bytes to f32 samples
        let mut samples = self.bytes_to_samples(audio_data)?;
        let original_samples = samples.len();

        // Update statistics (security: prevent integer overflow)
        {
            let mut stats = self.stats.write();
            stats.total_samples_processed = stats.total_samples_processed
                .saturating_add(original_samples as u64);
        }

        // Apply advanced processing
        if let Some(ref processor) = *self.advanced_processor.read() {
            processor.process_audio(&mut samples, self.config.sample_rate)?;
            
            // Update noise reduction stats (security: prevent integer overflow)
            {
                let mut stats = self.stats.write();
                stats.noise_reduced_samples = stats.noise_reduced_samples
                    .saturating_add(samples.len() as u64);
            }
        }

        // Analyze audio
        let analysis = {
            let analyzer_guard = self.analyzer.read();
            if let Some(ref analyzer) = *analyzer_guard {
                // Convert back to bytes for analysis
                let analysis_bytes: Bytes = samples.iter()
                    .flat_map(|&s| s.to_le_bytes().to_vec())
                    .collect::<Vec<u8>>()
                    .into();
                
                analyzer.analyze(&analysis_bytes)?
            } else {
                return Err(AudioError::Analysis("Analyzer not available".to_string()));
            }
        };

        // Detect voice activity
        let is_voice = if let Some(ref processor) = *self.advanced_processor.read() {
            processor.detect_voice_activity(
                &samples,
                analysis.energy,
                analysis.spectral_centroid,
                analysis.zero_crossing_rate,
            )
        } else {
            false
        };

        if is_voice {
            let mut stats = self.stats.write();
            stats.voice_activity_detected = stats.voice_activity_detected.saturating_add(1);
        }

        // Calculate latency (security: prevent division by zero and handle edge cases)
        let latency = start_time.elapsed();
        {
            let mut stats = self.stats.write();
            let total = stats.total_samples_processed;
            if total > 0 {
                // Security: Prevent division by zero and handle overflow
                let new_latency = latency.as_secs_f64() * 1000.0;
                if new_latency.is_finite() && stats.average_latency_ms.is_finite() {
                    // Weighted average: (old_avg * (n-1) + new) / n
                    stats.average_latency_ms = 
                        (stats.average_latency_ms * (total.saturating_sub(1)) as f64 + new_latency) 
                        / total as f64;
                    
                    // Security: Validate result
                    if !stats.average_latency_ms.is_finite() {
                        stats.average_latency_ms = new_latency; // Fallback to current
                    }
                }
            } else {
                stats.average_latency_ms = latency.as_secs_f64() * 1000.0;
            }
        }

        Ok(ProcessedAudio {
            samples,
            analysis,
            is_voice,
            latency_ms: latency.as_secs_f64() * 1000.0,
        })
    }

    /// Get capture statistics
    pub fn get_stats(&self) -> CaptureStats {
        self.stats.read().clone()
    }

    /// Convert bytes to f32 samples
    /// Security: Validates input length and handles edge cases
    fn bytes_to_samples(&self, data: &Bytes) -> Result<Vec<f32>, AudioError> {
        // Security: Validate input size
        if data.is_empty() {
            return Err(AudioError::Format("Empty audio data".to_string()));
        }

        // Security: Limit maximum input size
        const MAX_AUDIO_SIZE: usize = 10 * 1024 * 1024; // 10MB max
        if data.len() > MAX_AUDIO_SIZE {
            return Err(AudioError::Format(format!(
                "Audio data too large: {} bytes (max {})",
                data.len(), MAX_AUDIO_SIZE
            )));
        }

        if data.len() % 4 != 0 {
            return Err(AudioError::Format(format!(
                "Invalid audio data length: {} bytes (must be multiple of 4)",
                data.len()
            )));
        }

        // Security: Validate sample count
        let sample_count = data.len() / 4;
        const MAX_SAMPLES: usize = 2 * 1024 * 1024; // 2M samples max
        if sample_count > MAX_SAMPLES {
            return Err(AudioError::Format(format!(
                "Too many samples: {} (max {})",
                sample_count, MAX_SAMPLES
            )));
        }

        let samples: Vec<f32> = data.chunks_exact(4)
            .map(|chunk| {
                let bytes = [chunk[0], chunk[1], chunk[2], chunk[3]];
                f32::from_le_bytes(bytes)
            })
            .collect();

        Ok(samples)
    }
}

/// Processed audio result
#[derive(Debug, Clone)]
pub struct ProcessedAudio {
    pub samples: Vec<f32>,
    pub analysis: crate::audio_analyzer::AudioAnalysis,
    pub is_voice: bool,
    pub latency_ms: f64,
}

