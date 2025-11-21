//! narayana-sc: Sound capture and audio analysis for robots
//! 
//! Provides audio capture capabilities with:
//! - System microphone integration
//! - Real-time audio streaming
//! - Audio analysis (Fourier transforms, frequency analysis, etc.)
//! - Optional LLM integration for voice-to-text
//! - Integration with narayana-wld for brain-controlled audio processing
//! - Configurable and flexible architecture

pub mod error;
pub mod config;
pub mod audio_capture;
pub mod audio_analyzer;
pub mod audio_adapter;
pub mod llm_integration;
pub mod cpl_integration;
pub mod streaming; // 2025: Modern streaming architecture
pub mod advanced_features; // Advanced audio processing for comprehensive capture
pub mod comprehensive_capture; // Complete comprehensive capture system

pub use error::AudioError;
pub use config::{AudioConfig, CaptureConfig, AnalysisConfig};
pub use audio_capture::AudioCapture;
pub use audio_analyzer::AudioAnalyzer;
pub use audio_adapter::AudioAdapter;
pub use llm_integration::LlmAudioProcessor;
pub use streaming::{AudioStreamBuffer, EventBasedProcessor, AdaptiveStreamController, AudioEvent, AudioEventType};
pub use advanced_features::AdvancedAudioProcessor;
pub use comprehensive_capture::{ComprehensiveAudioCapture, CaptureStats, ProcessedAudio};

