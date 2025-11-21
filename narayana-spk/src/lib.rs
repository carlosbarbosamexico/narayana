//! narayana-spk: Speech synthesis for robots
//! 
//! Provides text-to-speech capabilities with:
//! - Native TTS engines (platform-specific)
//! - Optional API-based TTS providers
//! - Integration with narayana-wld for brain-controlled speech
//! - Configurable and off by default

pub mod error;
pub mod config;
pub mod engines;
pub mod speech_adapter;
pub mod synthesizer;
pub mod cpl_integration;

pub use error::SpeechError;
pub use config::{SpeechConfig, VoiceConfig, TtsEngine};
pub use speech_adapter::SpeechAdapter;
pub use synthesizer::SpeechSynthesizer;
pub use cpl_integration::{speech_config_from_cpl, create_speech_adapter_from_cpl};
pub use engines::TtsEngine as TtsEngineTrait;

