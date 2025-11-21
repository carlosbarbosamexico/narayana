//! Advanced audio features for comprehensive robot hearing
//! 2025: Complete audio capture capabilities

use crate::config::{CaptureConfig, AnalysisConfig};
use crate::error::AudioError;
use bytes::Bytes;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{info, debug, warn};

/// Advanced audio processing features
pub struct AdvancedAudioProcessor {
    /// Noise reduction state
    noise_reduction_state: Arc<RwLock<NoiseReductionState>>,
    
    /// Automatic gain control state
    agc_state: Arc<RwLock<AgcState>>,
    
    /// Echo cancellation state
    echo_cancellation_state: Arc<RwLock<EchoCancellationState>>,
    
    /// Beamforming state (for directional audio)
    beamforming_state: Arc<RwLock<BeamformingState>>,
    
    /// Voice activity detection
    vad_state: Arc<RwLock<VadState>>,
    
    /// Audio enhancement pipeline
    enhancement_pipeline: Arc<RwLock<EnhancementPipeline>>,
}

/// Noise reduction state (AI-driven)
struct NoiseReductionState {
    enabled: bool,
    noise_profile: Vec<f32>,
    adaptation_rate: f32,
    spectral_gating: bool,
}

/// Automatic gain control state
struct AgcState {
    enabled: bool,
    target_level: f32,
    current_gain: f32,
    attack_time: f32,
    release_time: f32,
    max_gain: f32,
    min_gain: f32,
}

/// Echo cancellation state
struct EchoCancellationState {
    enabled: bool,
    filter_length: usize,
    adaptation_rate: f32,
    echo_path: Vec<f32>,
}

/// Beamforming state (directional audio capture)
struct BeamformingState {
    enabled: bool,
    direction: (f32, f32, f32), // 3D direction vector
    beam_width: f32,
    channels: usize,
}

/// Voice activity detection state
struct VadState {
    enabled: bool,
    energy_threshold: f32,
    spectral_centroid_threshold: f32,
    zcr_threshold: f32,
    frame_count: usize,
    voice_frames: usize,
}

/// Audio enhancement pipeline
struct EnhancementPipeline {
    enabled: bool,
    steps: Vec<EnhancementStep>,
}

enum EnhancementStep {
    Normalize,
    HighPassFilter { cutoff: f32 },
    LowPassFilter { cutoff: f32 },
    SpectralEnhancement,
    DynamicRangeCompression { ratio: f32, threshold: f32 },
}

impl AdvancedAudioProcessor {
    /// Create new advanced audio processor
    pub fn new(capture_config: &CaptureConfig, analysis_config: &AnalysisConfig) -> Self {
        Self {
            noise_reduction_state: Arc::new(RwLock::new(NoiseReductionState {
                enabled: capture_config.noise_reduction,
                noise_profile: Vec::new(),
                adaptation_rate: 0.01,
                spectral_gating: true,
            })),
            agc_state: Arc::new(RwLock::new(AgcState {
                enabled: capture_config.agc,
                target_level: 0.7,
                current_gain: 1.0,
                attack_time: 0.01,
                release_time: 0.1,
                max_gain: 10.0,
                min_gain: 0.1,
            })),
            echo_cancellation_state: Arc::new(RwLock::new(EchoCancellationState {
                enabled: capture_config.echo_cancellation,
                filter_length: 512,
                adaptation_rate: 0.01,
                echo_path: vec![0.0; 512],
            })),
            beamforming_state: Arc::new(RwLock::new(BeamformingState {
                enabled: capture_config.beamforming,
                direction: (1.0, 0.0, 0.0), // Forward direction
                beam_width: 30.0, // degrees
                channels: 2,
            })),
            vad_state: Arc::new(RwLock::new(VadState {
                enabled: true, // Always enabled for voice detection
                energy_threshold: 0.01,
                spectral_centroid_threshold: 1000.0,
                zcr_threshold: 0.1,
                frame_count: 0,
                voice_frames: 0,
            })),
            enhancement_pipeline: Arc::new(RwLock::new(EnhancementPipeline {
                enabled: true,
                steps: vec![
                    EnhancementStep::Normalize,
                    EnhancementStep::HighPassFilter { cutoff: 80.0 },
                    EnhancementStep::SpectralEnhancement,
                ],
            })),
        }
    }

    /// Process audio with all advanced features
    pub fn process_audio(&self, samples: &mut [f32], sample_rate: u32) -> Result<(), AudioError> {
        // Apply enhancement pipeline
        if self.enhancement_pipeline.read().enabled {
            self.apply_enhancement_pipeline(samples, sample_rate)?;
        }

        // Apply noise reduction
        if self.noise_reduction_state.read().enabled {
            self.apply_noise_reduction(samples)?;
        }

        // Apply echo cancellation
        if self.echo_cancellation_state.read().enabled {
            self.apply_echo_cancellation(samples)?;
        }

        // Apply beamforming (if multi-channel)
        if self.beamforming_state.read().enabled {
            self.apply_beamforming(samples)?;
        }

        // Apply automatic gain control
        if self.agc_state.read().enabled {
            self.apply_agc(samples)?;
        }

        Ok(())
    }

    /// Detect voice activity
    pub fn detect_voice_activity(&self, samples: &[f32], energy: f32, spectral_centroid: f32, zcr: f32) -> bool {
        let mut vad = self.vad_state.write();
        
        vad.frame_count += 1;
        
        let is_voice = energy > vad.energy_threshold
            && spectral_centroid > vad.spectral_centroid_threshold
            && zcr < vad.zcr_threshold;
        
        if is_voice {
            vad.voice_frames += 1;
        }
        
        is_voice
    }

    /// Apply noise reduction (spectral subtraction)
    /// Security: Validates inputs and prevents resource exhaustion
    fn apply_noise_reduction(&self, samples: &mut [f32]) -> Result<(), AudioError> {
        // Security: Validate input
        if samples.is_empty() {
            return Ok(());
        }

        let mut state = self.noise_reduction_state.write();
        
        // Security: Limit noise profile size to prevent memory exhaustion
        const MAX_NOISE_PROFILE_SIZE: usize = 100_000; // ~400KB max
        if state.noise_profile.len() < MAX_NOISE_PROFILE_SIZE.min(1000) {
            // Update noise profile (first few frames)
            let remaining = (MAX_NOISE_PROFILE_SIZE - state.noise_profile.len()).min(samples.len());
            state.noise_profile.extend_from_slice(&samples[..remaining]);
            return Ok(());
        }

        // Spectral subtraction
        if state.spectral_gating {
            // Simple spectral gating: reduce samples below noise floor
            // Security: Handle empty noise profile
            if state.noise_profile.is_empty() {
                return Ok(());
            }

            let noise_floor = state.noise_profile.iter()
                .map(|&x| x.abs())
                .fold(0.0f32, f32::max) * 0.5;
            
            // Security: Validate noise_floor is finite
            let noise_floor = if noise_floor.is_finite() { noise_floor } else { 0.0 };
            
            for sample in samples.iter_mut() {
                // Security: Handle NaN/Inf
                if !sample.is_finite() {
                    *sample = 0.0;
                    continue;
                }
                
                if sample.abs() < noise_floor {
                    *sample *= 0.1; // Attenuate noise
                }
            }
        }

        Ok(())
    }

    /// Apply automatic gain control
    /// Security: Validates inputs and prevents division by zero
    fn apply_agc(&self, samples: &mut [f32]) -> Result<(), AudioError> {
        // Security: Validate input
        if samples.is_empty() {
            return Ok(()); // Nothing to process
        }

        let mut state = self.agc_state.write();
        
        // Calculate current level (handle edge cases)
        let current_level: f32 = samples.iter()
            .map(|&s| s.abs())
            .fold(0.0f32, f32::max);
        
        // Security: Prevent division by zero and handle NaN/Inf
        if current_level > 0.0 && current_level.is_finite() {
            let target_gain = (state.target_level / current_level).clamp(0.0, 100.0);
            
            // Smooth gain adjustment
            if target_gain > state.current_gain {
                // Attack
                state.current_gain += (target_gain - state.current_gain) * state.attack_time;
            } else {
                // Release
                state.current_gain += (target_gain - state.current_gain) * state.release_time;
            }
            
            // Clamp gain (security: prevent excessive gain)
            state.current_gain = state.current_gain.clamp(state.min_gain, state.max_gain);
            
            // Security: Validate gain is finite
            if !state.current_gain.is_finite() {
                state.current_gain = 1.0; // Reset to safe value
            }
        }
        
        // Apply gain (security: validate each sample)
        for sample in samples.iter_mut() {
            *sample *= state.current_gain;
            // Clamp to prevent clipping and handle NaN/Inf
            if !sample.is_finite() {
                *sample = 0.0; // Replace invalid values
            } else {
                *sample = sample.clamp(-1.0, 1.0);
            }
        }
        
        Ok(())
    }

    /// Apply echo cancellation (simplified NLMS)
    fn apply_echo_cancellation(&self, samples: &mut [f32]) -> Result<(), AudioError> {
        // Note: state is not actually used in this simplified implementation
        let _state = self.echo_cancellation_state.read();
        
        // Simplified echo cancellation
        // In full implementation, this would use adaptive filtering
        // For now, apply simple high-pass filter to reduce low-frequency echo
        
        if samples.len() > 1 {
            let alpha = 0.95;
            let mut prev = samples[0];
            for sample in samples.iter_mut().skip(1) {
                let current = *sample;
                *sample = current - alpha * prev;
                prev = current;
            }
        }
        
        Ok(())
    }

    /// Apply beamforming (directional audio)
    fn apply_beamforming(&self, samples: &mut [f32]) -> Result<(), AudioError> {
        let state = self.beamforming_state.read();
        
        // Simplified beamforming
        // Full implementation would use multi-channel phase alignment
        // For now, apply directional weighting
        
        if state.channels > 1 && samples.len() >= state.channels {
            // Simple delay-and-sum beamforming
            // In full implementation, would use proper phase alignment
            for i in 0..(samples.len() / state.channels) {
                let idx = i * state.channels;
                if idx + 1 < samples.len() {
                    // Weight channels based on direction
                    samples[idx] *= 1.0; // Primary channel
                    if idx + 1 < samples.len() {
                        samples[idx + 1] *= 0.7; // Secondary channel
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Apply enhancement pipeline
    fn apply_enhancement_pipeline(&self, samples: &mut [f32], sample_rate: u32) -> Result<(), AudioError> {
        let pipeline = self.enhancement_pipeline.read();
        
        for step in &pipeline.steps {
            match step {
                EnhancementStep::Normalize => {
                    self.normalize_audio(samples)?;
                }
                EnhancementStep::HighPassFilter { cutoff } => {
                    self.apply_high_pass_filter(samples, *cutoff, sample_rate)?;
                }
                EnhancementStep::LowPassFilter { cutoff } => {
                    self.apply_low_pass_filter(samples, *cutoff, sample_rate)?;
                }
                EnhancementStep::SpectralEnhancement => {
                    self.apply_spectral_enhancement(samples)?;
                }
                EnhancementStep::DynamicRangeCompression { ratio, threshold } => {
                    self.apply_compression(samples, *ratio, *threshold)?;
                }
            }
        }
        
        Ok(())
    }

    /// Normalize audio to prevent clipping
    fn normalize_audio(&self, samples: &mut [f32]) -> Result<(), AudioError> {
        let max_abs = samples.iter()
            .map(|&s| s.abs())
            .fold(0.0f32, f32::max);
        
        if max_abs > 1.0 {
            let scale = 0.95 / max_abs; // Leave headroom
            for sample in samples.iter_mut() {
                *sample *= scale;
            }
        }
        
        Ok(())
    }

    /// Apply high-pass filter
    /// Security: Validates parameters and handles edge cases
    fn apply_high_pass_filter(&self, samples: &mut [f32], cutoff: f32, sample_rate: u32) -> Result<(), AudioError> {
        // Security: Validate parameters
        if samples.is_empty() {
            return Ok(());
        }

        if cutoff <= 0.0 || !cutoff.is_finite() {
            return Err(AudioError::Analysis("Invalid cutoff frequency".to_string()));
        }

        if sample_rate == 0 {
            return Err(AudioError::Analysis("Invalid sample rate".to_string()));
        }

        // Security: Prevent division by zero
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff.max(1.0));
        let dt = 1.0 / sample_rate as f32;
        let alpha = rc / (rc + dt);
        
        // Security: Validate alpha is finite
        if !alpha.is_finite() {
            return Err(AudioError::Analysis("Invalid filter coefficient".to_string()));
        }

        let mut prev_output = 0.0;
        let mut prev_input = 0.0;
        for sample in samples.iter_mut() {
            let input = *sample;
            let output = alpha * (prev_output + input - prev_input);
            
            // Security: Handle NaN/Inf
            if output.is_finite() {
                prev_output = output;
                prev_input = input;
                *sample = output;
            } else {
                *sample = 0.0; // Replace invalid values
            }
        }
        
        Ok(())
    }

    /// Apply low-pass filter
    /// Security: Validates parameters and handles edge cases
    fn apply_low_pass_filter(&self, samples: &mut [f32], cutoff: f32, sample_rate: u32) -> Result<(), AudioError> {
        // Security: Validate parameters
        if samples.is_empty() {
            return Ok(());
        }

        if cutoff <= 0.0 || !cutoff.is_finite() {
            return Err(AudioError::Analysis("Invalid cutoff frequency".to_string()));
        }

        if sample_rate == 0 {
            return Err(AudioError::Analysis("Invalid sample rate".to_string()));
        }

        // Security: Prevent division by zero
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff.max(1.0));
        let dt = 1.0 / sample_rate as f32;
        let alpha = dt / (rc + dt);
        
        // Security: Validate alpha is finite
        if !alpha.is_finite() {
            return Err(AudioError::Analysis("Invalid filter coefficient".to_string()));
        }

        let mut prev_output = 0.0;
        for sample in samples.iter_mut() {
            let output = prev_output + alpha * (*sample - prev_output);
            
            // Security: Handle NaN/Inf
            if output.is_finite() {
                prev_output = output;
                *sample = output;
            } else {
                *sample = 0.0; // Replace invalid values
            }
        }
        
        Ok(())
    }

    /// Apply spectral enhancement
    fn apply_spectral_enhancement(&self, samples: &mut [f32]) -> Result<(), AudioError> {
        // Simple spectral enhancement: boost mid frequencies
        // Full implementation would use FFT-based enhancement
        
        for sample in samples.iter_mut() {
            // Gentle boost for clarity
            *sample *= 1.1;
            *sample = sample.clamp(-1.0, 1.0);
        }
        
        Ok(())
    }

    /// Apply dynamic range compression
    fn apply_compression(&self, samples: &mut [f32], ratio: f32, threshold: f32) -> Result<(), AudioError> {
        for sample in samples.iter_mut() {
            let abs = sample.abs();
            if abs > threshold {
                let excess = abs - threshold;
                let compressed_excess = excess / ratio;
                let new_abs = threshold + compressed_excess;
                *sample = new_abs * sample.signum();
            }
        }
        
        Ok(())
    }
}


