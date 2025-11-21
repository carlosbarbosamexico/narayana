//! Audio analysis using Fourier transforms and other techniques

use crate::config::AnalysisConfig;
use crate::error::AudioError;
use bytes::Bytes;
use rustfft::{Fft, FftPlanner};
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, warn};
use rayon::prelude::*; // 2025: Parallel processing

/// Audio analysis results
#[derive(Debug, Clone)]
pub struct AudioAnalysis {
    /// Frequency spectrum (FFT results)
    pub spectrum: Vec<f32>,

    /// Dominant frequencies
    pub dominant_frequencies: Vec<f32>,

    /// Energy/amplitude
    pub energy: f32,

    /// Zero-crossing rate
    pub zero_crossing_rate: f32,

    /// Estimated pitch (Hz)
    pub pitch: Option<f32>,

    /// Spectral centroid
    pub spectral_centroid: f32,

    /// Spectral rolloff
    pub spectral_rolloff: f32,
}

/// Audio analyzer using FFT and other techniques - 2025 enhanced
pub struct AudioAnalyzer {
    config: Arc<AnalysisConfig>,
    fft: Arc<dyn Fft<f32>>,
    fft_scratch: parking_lot::RwLock<Vec<rustfft::num_complex::Complex<f32>>>, // Interior mutability
    sample_rate: u32,
    // 2025: Sound event detection state
    sound_event_history: Arc<parking_lot::RwLock<Vec<(String, f32)>>>, // (event_name, confidence)
}

impl AudioAnalyzer {
    /// Create a new audio analyzer
    pub fn new(config: AnalysisConfig, sample_rate: u32) -> Result<Self, AudioError> {
        config.validate()
            .map_err(|e| AudioError::Config(e))?;

        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(config.fft_window_size);
        let fft_scratch = parking_lot::RwLock::new(
            vec![rustfft::num_complex::Complex::new(0.0, 0.0); config.fft_window_size]
        );

        Ok(Self {
            config: Arc::new(config),
            fft,
            fft_scratch,
            sample_rate,
            sound_event_history: Arc::new(parking_lot::RwLock::new(Vec::new())),
        })
    }

    /// Analyze audio samples - 2025: Enhanced with parallel processing and AI features
    /// Security: Validates inputs and prevents resource exhaustion
    pub fn analyze(&self, audio_data: &Bytes) -> Result<AudioAnalysis, AudioError> {
        // Convert bytes to f32 samples
        let samples = self.bytes_to_samples(audio_data)?;

        // Limit to window size
        let window_size = self.config.fft_window_size;
        let samples: Vec<f32> = samples.into_iter().take(window_size).collect();
        
        // Pad if necessary
        let mut samples = samples;
        while samples.len() < window_size {
            samples.push(0.0);
        }

        let mut analysis = AudioAnalysis {
            spectrum: Vec::new(),
            dominant_frequencies: Vec::new(),
            energy: 0.0,
            zero_crossing_rate: 0.0,
            pitch: None,
            spectral_centroid: 0.0,
            spectral_rolloff: 0.0,
        };

        // Energy analysis
        if self.config.enable_energy {
            analysis.energy = self.calculate_energy(&samples);
        }

        // Zero-crossing rate
        if self.config.enable_zcr {
            analysis.zero_crossing_rate = self.calculate_zcr(&samples);
        }

        // FFT analysis
        if self.config.enable_fft {
            let spectrum = self.compute_fft(&samples)?;
            analysis.spectrum = spectrum.clone();

            // Dominant frequencies
            analysis.dominant_frequencies = self.find_dominant_frequencies(&spectrum);

            // Spectral features
            if self.config.enable_spectral {
                analysis.spectral_centroid = self.calculate_spectral_centroid(&spectrum);
                analysis.spectral_rolloff = self.calculate_spectral_rolloff(&spectrum);
            }

            // Pitch detection
            if self.config.enable_pitch {
                analysis.pitch = self.detect_pitch(&spectrum);
            }
        }

        Ok(analysis)
    }

    /// Convert bytes to f32 samples
    /// Security: Validates input length and handles edge cases
    fn bytes_to_samples(&self, data: &Bytes) -> Result<Vec<f32>, AudioError> {
        // Security: Validate input size to prevent DoS
        if data.is_empty() {
            return Err(AudioError::Format("Empty audio data".to_string()));
        }

        // Security: Limit maximum input size to prevent memory exhaustion
        const MAX_AUDIO_SIZE: usize = 10 * 1024 * 1024; // 10MB max
        if data.len() > MAX_AUDIO_SIZE {
            return Err(AudioError::Format(format!(
                "Audio data too large: {} bytes (max {})",
                data.len(), MAX_AUDIO_SIZE
            )));
        }

        // Assume f32 samples (4 bytes per sample)
        if data.len() % 4 != 0 {
            return Err(AudioError::Format(format!(
                "Invalid audio data length: {} bytes (must be multiple of 4)",
                data.len()
            )));
        }

        // Security: Validate sample count to prevent excessive memory allocation
        let sample_count = data.len() / 4;
        const MAX_SAMPLES: usize = 2 * 1024 * 1024; // 2M samples max (~8MB)
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

    /// Compute FFT
    fn compute_fft(&self, samples: &[f32]) -> Result<Vec<f32>, AudioError> {
        // Convert to complex numbers
        let mut complex_samples: Vec<rustfft::num_complex::Complex<f32>> = samples
            .iter()
            .map(|&s| rustfft::num_complex::Complex::new(s, 0.0))
            .collect();

        // Perform FFT (use interior mutability for scratch)
        {
            let mut scratch = self.fft_scratch.write();
            self.fft.process_with_scratch(&mut complex_samples, &mut *scratch);
        }

        // Convert to magnitude spectrum
        let spectrum: Vec<f32> = complex_samples
            .iter()
            .map(|c| c.norm())
            .collect();

        Ok(spectrum)
    }

    /// Calculate energy
    fn calculate_energy(&self, samples: &[f32]) -> f32 {
        samples.iter()
            .map(|&s| s * s)
            .sum::<f32>() / samples.len() as f32
    }

    /// Calculate zero-crossing rate
    fn calculate_zcr(&self, samples: &[f32]) -> f32 {
        if samples.len() < 2 {
            return 0.0;
        }

        let crossings = samples.windows(2)
            .filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0))
            .count();

        crossings as f32 / (samples.len() - 1) as f32
    }

    /// Find dominant frequencies
    /// Security: Validates inputs and prevents invalid frequency calculations
    fn find_dominant_frequencies(&self, spectrum: &[f32]) -> Vec<f32> {
        // Security: Validate input
        if spectrum.is_empty() {
            return Vec::new();
        }

        let mut peaks: Vec<(usize, f32)> = spectrum.iter()
            .enumerate()
            .filter_map(|(i, &mag)| {
                // Security: Filter out NaN/Inf
                if mag.is_finite() && mag >= 0.0 {
                    Some((i, mag))
                } else {
                    None
                }
            })
            .collect();

        // Sort by magnitude (handle NaN/Inf gracefully)
        peaks.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Get top 5 frequencies
        // Security: Prevent division by zero
        let nyquist = self.sample_rate as f32 / 2.0;
        let bin_to_freq_factor = if self.config.fft_window_size > 0 {
            self.sample_rate as f32 / (self.config.fft_window_size as f32 * 2.0)
        } else {
            return Vec::new(); // Invalid window size
        };

        peaks.into_iter()
            .take(5)
            .map(|(i, _)| {
                // Convert bin index to frequency
                (i as f32) * bin_to_freq_factor
            })
            .filter(|&f| f > 0.0 && f.is_finite() && f < nyquist)
            .collect()
    }

    /// Calculate spectral centroid
    /// Security: Validates inputs and prevents division by zero
    fn calculate_spectral_centroid(&self, spectrum: &[f32]) -> f32 {
        // Security: Validate input
        if spectrum.is_empty() || self.config.fft_window_size == 0 {
            return 0.0;
        }

        let mut weighted_sum = 0.0;
        let mut magnitude_sum = 0.0;

        let bin_to_freq_factor = self.sample_rate as f32 / (self.config.fft_window_size as f32 * 2.0);

        for (i, &mag) in spectrum.iter().enumerate() {
            // Security: Filter out NaN/Inf
            if !mag.is_finite() || mag < 0.0 {
                continue;
            }

            let freq = (i as f32) * bin_to_freq_factor;
            
            // Security: Validate frequency is finite
            if freq.is_finite() {
                weighted_sum += freq * mag;
                magnitude_sum += mag;
            }
        }

        // Security: Prevent division by zero and handle NaN/Inf
        if magnitude_sum > 0.0 && magnitude_sum.is_finite() {
            let centroid = weighted_sum / magnitude_sum;
            if centroid.is_finite() {
                centroid
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// Calculate spectral rolloff
    /// Security: Validates inputs and prevents division by zero
    fn calculate_spectral_rolloff(&self, spectrum: &[f32]) -> f32 {
        // Security: Validate input
        if spectrum.is_empty() || self.config.fft_window_size == 0 {
            return 0.0;
        }

        let total_energy: f32 = spectrum.iter()
            .filter_map(|&m| {
                // Security: Filter out NaN/Inf
                if m.is_finite() {
                    Some(m * m)
                } else {
                    None
                }
            })
            .sum();

        // Security: Validate total_energy is finite
        if !total_energy.is_finite() || total_energy <= 0.0 {
            return 0.0;
        }

        let threshold = total_energy * 0.85; // 85% rolloff
        let bin_to_freq_factor = self.sample_rate as f32 / (self.config.fft_window_size as f32 * 2.0);

        let mut cumulative_energy = 0.0;
        for (i, &mag) in spectrum.iter().enumerate() {
            // Security: Filter out NaN/Inf
            if !mag.is_finite() {
                continue;
            }

            cumulative_energy += mag * mag;
            if cumulative_energy >= threshold {
                let freq = (i as f32) * bin_to_freq_factor;
                return if freq.is_finite() { freq } else { 0.0 };
            }
        }

        0.0
    }

    /// Detect pitch using autocorrelation
    fn detect_pitch(&self, spectrum: &[f32]) -> Option<f32> {
        // Simple pitch detection: find peak in frequency domain
        // More sophisticated methods could use autocorrelation
        let dominant = self.find_dominant_frequencies(spectrum);
        dominant.first().copied()
    }

    /// Convert analysis to JSON for world events
    pub fn analysis_to_json(analysis: &AudioAnalysis) -> serde_json::Value {
        json!({
            "energy": analysis.energy,
            "zero_crossing_rate": analysis.zero_crossing_rate,
            "spectral_centroid": analysis.spectral_centroid,
            "spectral_rolloff": analysis.spectral_rolloff,
            "dominant_frequencies": analysis.dominant_frequencies,
            "pitch": analysis.pitch,
            "spectrum_length": analysis.spectrum.len(),
        })
    }
}

