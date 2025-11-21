//! Comprehensive audio capture example
//! Demonstrates the full-featured audio capture system with all advanced features

use narayana_sc::*;
use bytes::Bytes;
use tokio::time::{sleep, Duration};
use tracing::{info, error, warn};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting comprehensive audio capture example...");

    // Create comprehensive audio configuration
    let mut audio_config = AudioConfig::default();
    audio_config.enabled = true;
    audio_config.sample_rate = 44100;
    audio_config.channels = 1;
    audio_config.buffer_size = 8192;
    
    // Enable all analysis features
    audio_config.analysis.enable_fft = true;
    audio_config.analysis.enable_energy = true;
    audio_config.analysis.enable_zcr = true;
    audio_config.analysis.enable_spectral = true;
    audio_config.analysis.enable_pitch = true;
    audio_config.analysis.fft_window_size = 2048;
    
    // Enable all advanced capture features
    audio_config.capture.noise_reduction = true;
    audio_config.capture.agc = true;
    audio_config.capture.echo_cancellation = true;
    audio_config.capture.beamforming = false; // Requires multiple channels
    audio_config.capture.spatial_audio = false;
    audio_config.capture.low_latency = true;
    audio_config.capture.buffer_strategy = "ring".to_string();
    audio_config.capture.ring_buffer_size = 8192;
    
    // Enable LLM voice-to-text (if available)
    audio_config.enable_llm_vtt = false; // Set to true if LLM integration is enabled
    
    // Validate configuration
    audio_config.validate()
        .map_err(|e| format!("Invalid audio config: {}", e))?;

    info!("Creating comprehensive audio capture system...");

    // Create comprehensive capture system
    let capture_system = ComprehensiveAudioCapture::new(audio_config)
        .map_err(|e| format!("Failed to create capture system: {}", e))?;

    info!("Capture system created successfully");

    // Simulate processing some audio data
    // In a real scenario, this would come from the microphone
    info!("Processing sample audio data...");

    // Create sample audio (sine wave at 440 Hz)
    let sample_rate = 44100.0;
    let frequency = 440.0;
    let samples: Vec<f32> = (0..4096)
        .map(|i| {
            let t = i as f32 / sample_rate;
            (2.0 * std::f32::consts::PI * frequency * t).sin() * 0.5
        })
        .collect();
    
    let bytes: Vec<u8> = samples.iter()
        .flat_map(|&s| s.to_le_bytes().to_vec())
        .collect();
    
    let audio_data = Bytes::from(bytes);

    // Process the audio comprehensively
    match capture_system.process_comprehensive(&audio_data) {
        Ok(processed) => {
            info!("Audio processed successfully");
            info!("  Samples processed: {}", processed.samples.len());
            info!("  Is voice: {}", processed.is_voice);
            info!("  Latency: {:.2} ms", processed.latency_ms);
            
            info!("Analysis results:");
            info!("  Energy: {:.6}", processed.analysis.energy);
            info!("  Zero-crossing rate: {:.6}", processed.analysis.zero_crossing_rate);
            info!("  Spectral centroid: {:.2} Hz", processed.analysis.spectral_centroid);
            info!("  Spectral rolloff: {:.2} Hz", processed.analysis.spectral_rolloff);
            
            if let Some(pitch) = processed.analysis.pitch {
                info!("  Detected pitch: {:.2} Hz", pitch);
            }
            
            if !processed.analysis.dominant_frequencies.is_empty() {
                info!("  Dominant frequencies:");
                for (i, freq) in processed.analysis.dominant_frequencies.iter().take(5).enumerate() {
                    info!("    {}: {:.2} Hz", i + 1, freq);
                }
            }
        }
        Err(e) => {
            error!("Failed to process audio: {}", e);
            return Err(e.into());
        }
    }

    // Get statistics
    let stats = capture_system.get_stats();
    info!("\nCapture statistics:");
    info!("  Total samples processed: {}", stats.total_samples_processed);
    info!("  Total events detected: {}", stats.total_events_detected);
    info!("  Voice activity detected: {}", stats.voice_activity_detected);
    info!("  Noise reduced samples: {}", stats.noise_reduced_samples);
    info!("  AGC adjustments: {}", stats.agc_adjustments);
    info!("  Average latency: {:.2} ms", stats.average_latency_ms);

    info!("\nComprehensive capture example completed!");

    Ok(())
}

