//! Configuration for audio capture and analysis

use serde::{Deserialize, Serialize};

/// Audio capture and analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioConfig {
    /// Enable audio capture (off by default)
    pub enabled: bool,

    /// Capture configuration
    pub capture: CaptureConfig,

    /// Analysis configuration
    pub analysis: AnalysisConfig,

    /// Enable LLM integration for voice-to-text
    pub enable_llm_vtt: bool,

    /// Buffer size for audio samples (in frames)
    pub buffer_size: usize,

    /// Sample rate (Hz)
    pub sample_rate: u32,

    /// Number of audio channels
    pub channels: u16,
}

/// Audio capture configuration - 2025 enhanced
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureConfig {
    /// Device name (None = default device)
    pub device_name: Option<String>,

    /// Enable continuous capture
    pub continuous: bool,

    /// Maximum capture duration in seconds (0 = unlimited)
    pub max_duration_secs: u64,

    /// Enable AI-driven noise reduction (2025 feature)
    pub noise_reduction: bool,

    /// Adaptive automatic gain control (2025 feature)
    pub agc: bool,

    /// Enable spatial audio capture (multi-channel for 3D audio)
    pub spatial_audio: bool,

    /// Number of spatial audio channels (for immersive sound)
    pub spatial_channels: u16,

    /// Low-latency mode (optimize for real-time processing)
    pub low_latency: bool,

    /// Buffer strategy: "ring" (zero-copy), "queue" (traditional)
    pub buffer_strategy: String,

    /// Ring buffer size (for zero-copy streaming)
    pub ring_buffer_size: usize,

    /// Enable echo cancellation (for voice applications)
    pub echo_cancellation: bool,

    /// Enable beamforming (for directional audio capture)
    pub beamforming: bool,
}

/// Audio analysis configuration - 2025 enhanced with AI features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    /// Enable frequency analysis (FFT)
    pub enable_fft: bool,

    /// FFT window size
    pub fft_window_size: usize,

    /// Enable spectral analysis
    pub enable_spectral: bool,

    /// Enable energy/amplitude analysis
    pub enable_energy: bool,

    /// Enable zero-crossing rate analysis
    pub enable_zcr: bool,

    /// Enable pitch detection
    pub enable_pitch: bool,

    /// Analysis interval in milliseconds
    pub analysis_interval_ms: u64,

    /// Enable AI-driven sound event detection (2025 feature)
    pub enable_sound_event_detection: bool,

    /// Open-vocabulary sound classification (2025: DASM-like)
    pub open_vocabulary_detection: bool,

    /// Enable real-time neural acoustic transfer (2025 feature)
    pub neural_acoustic_transfer: bool,

    /// Parallel processing for analysis (2025: use all cores)
    pub parallel_processing: bool,

    /// Enable spatial audio analysis (3D sound field)
    pub spatial_analysis: bool,

    /// Adaptive analysis (AI adjusts based on audio content)
    pub adaptive_analysis: bool,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            capture: CaptureConfig::default(),
            analysis: AnalysisConfig::default(),
            enable_llm_vtt: false,
            buffer_size: 4096,
            sample_rate: 44100,
            channels: 1,
        }
    }
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            device_name: None,
            continuous: true,
            max_duration_secs: 0,
            noise_reduction: false,
            agc: false,
            spatial_audio: false,
            spatial_channels: 2, // Stereo default
            low_latency: true, // 2025: Low latency by default
            buffer_strategy: "ring".to_string(), // 2025: Zero-copy ring buffers
            ring_buffer_size: 8192, // Optimized for low latency
            echo_cancellation: false,
            beamforming: false,
        }
    }
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            enable_fft: true,
            fft_window_size: 2048,
            enable_spectral: true,
            enable_energy: true,
            enable_zcr: true,
            enable_pitch: false,
            analysis_interval_ms: 100,
            enable_sound_event_detection: false, // AI feature, opt-in
            open_vocabulary_detection: false, // 2025: DASM-like detection
            neural_acoustic_transfer: false, // 2025: Real-time acoustic modeling
            parallel_processing: true, // 2025: Use all cores by default
            spatial_analysis: false, // 3D audio analysis
            adaptive_analysis: true, // 2025: AI adapts to content
        }
    }
}

impl AudioConfig {
    /// Validate configuration
    /// Security: Validates all configuration values to prevent resource exhaustion
    pub fn validate(&self) -> Result<(), String> {
        // Security: Validate buffer size
        if self.buffer_size == 0 {
            return Err("Buffer size must be greater than 0".to_string());
        }

        if self.buffer_size > 65536 {
            return Err("Buffer size too large (max 65536)".to_string());
        }

        // Security: Validate sample rate
        if self.sample_rate == 0 {
            return Err("Sample rate must be greater than 0".to_string());
        }

        if self.sample_rate > 192000 {
            return Err("Sample rate too high (max 192000 Hz)".to_string());
        }

        // Security: Validate channel count
        if self.channels == 0 {
            return Err("Number of channels must be greater than 0".to_string());
        }

        if self.channels > 8 {
            return Err("Too many channels (max 8)".to_string());
        }

        // Security: Validate nested configs
        self.analysis.validate()?;
        self.capture.validate()?;

        Ok(())
    }
}

impl CaptureConfig {
    /// Validate capture configuration
    /// Security: Validates all fields to prevent resource exhaustion and injection
    pub fn validate(&self) -> Result<(), String> {
        // Security: Validate device name
        if let Some(ref name) = self.device_name {
            if name.is_empty() {
                return Err("Device name cannot be empty".to_string());
            }
            if name.len() > 256 {
                return Err("Device name too long (max 256 chars)".to_string());
            }
            // Security: Check for potential injection patterns
            if name.contains('\0') {
                return Err("Device name contains null byte".to_string());
            }
        }

        // Security: Validate spatial channels
        if self.spatial_channels == 0 {
            return Err("Spatial channels must be greater than 0".to_string());
        }

        if self.spatial_channels > 32 {
            return Err("Too many spatial channels (max 32)".to_string());
        }

        // Security: Validate buffer strategy
        if !["ring", "queue"].contains(&self.buffer_strategy.as_str()) {
            return Err("Buffer strategy must be 'ring' or 'queue'".to_string());
        }

        // Security: Validate ring buffer size
        if self.ring_buffer_size == 0 {
            return Err("Ring buffer size must be greater than 0".to_string());
        }

        if self.ring_buffer_size > 65536 {
            return Err("Ring buffer size too large (max 65536)".to_string());
        }

        // Ring buffer size should be power of 2 for efficiency
        if self.buffer_strategy == "ring" && !self.ring_buffer_size.is_power_of_two() {
            return Err("Ring buffer size must be a power of 2".to_string());
        }

        // Security: Validate max duration
        if self.max_duration_secs > 86400 {
            return Err("Max duration too large (max 86400 seconds = 24 hours)".to_string());
        }

        Ok(())
    }
}

impl AnalysisConfig {
    /// Validate analysis configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.fft_window_size == 0 {
            return Err("FFT window size must be greater than 0".to_string());
        }

        if self.fft_window_size > 65536 {
            return Err("FFT window size too large (max 65536)".to_string());
        }

        // FFT window size should be a power of 2 for efficiency
        if !self.fft_window_size.is_power_of_two() {
            return Err("FFT window size must be a power of 2".to_string());
        }

        if self.analysis_interval_ms == 0 {
            return Err("Analysis interval must be greater than 0".to_string());
        }

        if self.analysis_interval_ms > 10000 {
            return Err("Analysis interval too large (max 10000 ms)".to_string());
        }

        // 2025: Validate AI features
        if self.open_vocabulary_detection && !self.enable_sound_event_detection {
            return Err("Open vocabulary detection requires sound event detection".to_string());
        }

        Ok(())
    }
}

