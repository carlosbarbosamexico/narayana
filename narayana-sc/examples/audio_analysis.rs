//! Audio analysis example
//! Demonstrates how to analyze audio data directly without capture

use narayana_sc::*;
use bytes::Bytes;
use std::f32::consts::PI;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Audio Analysis Example");
    println!("======================");

    // Create analysis configuration
    let mut analysis_config = AnalysisConfig::default();
    analysis_config.enable_fft = true;
    analysis_config.enable_energy = true;
    analysis_config.enable_zcr = true;
    analysis_config.enable_spectral = true;
    analysis_config.enable_pitch = true;
    analysis_config.fft_window_size = 2048;

    // Create audio analyzer
    let mut analyzer = AudioAnalyzer::new(analysis_config, 44100)
        .map_err(|e| format!("Failed to create analyzer: {}", e))?;

    println!("\n1. Analyzing silence...");
    analyze_silence(&mut analyzer)?;

    println!("\n2. Analyzing pure tone (440 Hz)...");
    analyze_tone(&mut analyzer, 440.0)?;

    println!("\n3. Analyzing multiple tones...");
    analyze_multiple_tones(&mut analyzer)?;

    println!("\n4. Analyzing white noise...");
    analyze_noise(&mut analyzer)?;

    println!("\nAnalysis examples completed!");

    Ok(())
}

fn analyze_silence(analyzer: &mut AudioAnalyzer) -> Result<(), Box<dyn std::error::Error>> {
    // Create silence (all zeros)
    let samples: Vec<f32> = vec![0.0; 2048];
    let bytes: Vec<u8> = samples.iter()
        .flat_map(|&s| s.to_le_bytes().to_vec())
        .collect();
    
    let audio_data = Bytes::from(bytes);
    let analysis = analyzer.analyze(&audio_data)?;
    
    println!("  Energy: {:.6}", analysis.energy);
    println!("  Zero-crossing rate: {:.6}", analysis.zero_crossing_rate);
    println!("  Spectral centroid: {:.2} Hz", analysis.spectral_centroid);
    
    Ok(())
}

fn analyze_tone(analyzer: &mut AudioAnalyzer, frequency: f32) -> Result<(), Box<dyn std::error::Error>> {
    // Create a pure sine wave
    let sample_rate = 44100.0;
    let samples: Vec<f32> = (0..2048)
        .map(|i| {
            let t = i as f32 / sample_rate;
            (2.0 * PI * frequency * t).sin() * 0.5
        })
        .collect();
    
    let bytes: Vec<u8> = samples.iter()
        .flat_map(|&s| s.to_le_bytes().to_vec())
        .collect();
    
    let audio_data = Bytes::from(bytes);
    let analysis = analyzer.analyze(&audio_data)?;
    
    println!("  Energy: {:.6}", analysis.energy);
    println!("  Zero-crossing rate: {:.6}", analysis.zero_crossing_rate);
    println!("  Spectral centroid: {:.2} Hz", analysis.spectral_centroid);
    println!("  Spectral rolloff: {:.2} Hz", analysis.spectral_rolloff);
    
    if let Some(pitch) = analysis.pitch {
        println!("  Detected pitch: {:.2} Hz (expected ~{:.2} Hz)", pitch, frequency);
        let error = (pitch - frequency).abs();
        println!("  Error: {:.2} Hz", error);
    } else {
        println!("  Pitch: Not detected");
    }
    
    if !analysis.dominant_frequencies.is_empty() {
        println!("  Dominant frequencies:");
        for (i, freq) in analysis.dominant_frequencies.iter().take(5).enumerate() {
            println!("    {}: {:.2} Hz", i + 1, freq);
        }
    }
    
    Ok(())
}

fn analyze_multiple_tones(analyzer: &mut AudioAnalyzer) -> Result<(), Box<dyn std::error::Error>> {
    // Create a signal with multiple frequencies
    let sample_rate = 44100.0;
    let frequencies = vec![440.0, 880.0, 1320.0]; // A4, A5, E6
    let amplitudes = vec![0.5, 0.3, 0.2];
    
    let samples: Vec<f32> = (0..2048)
        .map(|i| {
            let t = i as f32 / sample_rate;
            frequencies.iter().zip(amplitudes.iter())
                .map(|(freq, amp)| (2.0 * PI * freq * t).sin() * amp)
                .sum::<f32>()
        })
        .collect();
    
    let bytes: Vec<u8> = samples.iter()
        .flat_map(|&s| s.to_le_bytes().to_vec())
        .collect();
    
    let audio_data = Bytes::from(bytes);
    let analysis = analyzer.analyze(&audio_data)?;
    
    println!("  Energy: {:.6}", analysis.energy);
    println!("  Spectral centroid: {:.2} Hz", analysis.spectral_centroid);
    
    if !analysis.dominant_frequencies.is_empty() {
        println!("  Dominant frequencies detected:");
        for (i, freq) in analysis.dominant_frequencies.iter().take(5).enumerate() {
            println!("    {}: {:.2} Hz", i + 1, freq);
        }
    }
    
    Ok(())
}

fn analyze_noise(analyzer: &mut AudioAnalyzer) -> Result<(), Box<dyn std::error::Error>> {
    // Create white noise
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut samples: Vec<f32> = Vec::new();
    let mut hasher = DefaultHasher::new();
    
    for i in 0..2048 {
        i.hash(&mut hasher);
        let hash = hasher.finish();
        // Convert to f32 in range [-0.3, 0.3]
        let sample = ((hash % 600) as f32 / 1000.0) - 0.3;
        samples.push(sample);
    }
    
    let bytes: Vec<u8> = samples.iter()
        .flat_map(|&s| s.to_le_bytes().to_vec())
        .collect();
    
    let audio_data = Bytes::from(bytes);
    let analysis = analyzer.analyze(&audio_data)?;
    
    println!("  Energy: {:.6}", analysis.energy);
    println!("  Zero-crossing rate: {:.6}", analysis.zero_crossing_rate);
    println!("  Spectral centroid: {:.2} Hz", analysis.spectral_centroid);
    println!("  Spectral rolloff: {:.2} Hz", analysis.spectral_rolloff);
    
    // Noise should have high zero-crossing rate
    assert!(analysis.zero_crossing_rate > 0.1, "Noise should have high ZCR");
    
    Ok(())
}

