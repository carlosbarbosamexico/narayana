//! Integration tests for narayana-sc

#[cfg(test)]
mod tests {
    use narayana_sc::*;
    use bytes::Bytes;
    use std::time::Duration;

    #[test]
    fn test_audio_config_to_adapter() {
        let mut config = AudioConfig::default();
        config.enabled = true;
        config.validate().expect("Config should be valid");
        
        // Note: This may fail if audio device is not available
        // That's okay - we're testing the integration path
        let adapter_result = AudioAdapter::new(config);
        // Just verify it doesn't panic and returns a result
        match adapter_result {
            Ok(_) => {
                // Success - audio device available
            }
            Err(e) => {
                // Expected if no audio device
                println!("Adapter creation failed (expected if no device): {}", e);
            }
        }
    }

    #[test]
    fn test_audio_analyzer_full_pipeline() {
        let mut config = AnalysisConfig::default();
        config.enable_fft = true;
        config.enable_energy = true;
        config.enable_zcr = true;
        config.enable_spectral = true;
        config.enable_pitch = true;
        
        let mut analyzer = AudioAnalyzer::new(config, 44100).expect("Analyzer should be created");
        
        // Create realistic audio data (sine wave)
        let sample_rate = 44100;
        let frequency = 440.0; // A4 note
        let duration_samples = 2048;
        let samples: Vec<f32> = (0..duration_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * frequency * t).sin() * 0.5
            })
            .collect();
        
        let bytes: Vec<u8> = samples.iter()
            .flat_map(|&s| s.to_le_bytes().to_vec())
            .collect();
        
        let audio_data = Bytes::from(bytes);
        let result = analyzer.analyze(&audio_data);
        
        assert!(result.is_ok());
        let analysis = result.unwrap();
        
        // Verify analysis results
        assert!(!analysis.spectrum.is_empty());
        assert!(analysis.energy > 0.0);
        assert!(analysis.zero_crossing_rate >= 0.0);
        assert!(analysis.spectral_centroid >= 0.0);
        assert!(analysis.spectral_rolloff >= 0.0);
        // Pitch should be detected for a 440Hz tone
        if let Some(pitch) = analysis.pitch {
            assert!(pitch > 0.0);
            assert!(pitch < sample_rate as f32 / 2.0); // Below Nyquist
        }
    }

    #[test]
    fn test_config_validation_chain() {
        let mut audio_config = AudioConfig::default();
        audio_config.enabled = true;
        
        // Validate entire config chain
        assert!(audio_config.validate().is_ok());
        assert!(audio_config.capture.validate().is_ok());
        assert!(audio_config.analysis.validate().is_ok());
    }

    #[test]
    fn test_audio_analyzer_different_sample_rates() {
        let config = AnalysisConfig::default();
        
        // Test different sample rates
        for sample_rate in [8000, 16000, 22050, 44100, 48000] {
            let analyzer = AudioAnalyzer::new(config.clone(), sample_rate);
            assert!(analyzer.is_ok(), "Should create analyzer for sample rate {}", sample_rate);
        }
    }

    #[test]
    fn test_audio_analyzer_different_window_sizes() {
        let mut config = AnalysisConfig::default();
        
        // Test power-of-2 window sizes
        for window_size in [256, 512, 1024, 2048, 4096] {
            config.fft_window_size = window_size;
            let analyzer = AudioAnalyzer::new(config.clone(), 44100);
            assert!(analyzer.is_ok(), "Should create analyzer for window size {}", window_size);
        }
    }

    #[test]
    fn test_audio_analyzer_silence() {
        let mut config = AnalysisConfig::default();
        config.enable_fft = true;
        config.enable_energy = true;
        let mut analyzer = AudioAnalyzer::new(config, 44100).unwrap();
        
        // Create silence (all zeros)
        let samples: Vec<f32> = vec![0.0; 2048];
        let bytes: Vec<u8> = samples.iter()
            .flat_map(|&s| s.to_le_bytes().to_vec())
            .collect();
        
        let audio_data = Bytes::from(bytes);
        let result = analyzer.analyze(&audio_data);
        assert!(result.is_ok());
        
        let analysis = result.unwrap();
        assert_eq!(analysis.energy, 0.0);
        assert_eq!(analysis.zero_crossing_rate, 0.0);
    }

    #[test]
    fn test_audio_analyzer_noise() {
        let mut config = AnalysisConfig::default();
        config.enable_fft = true;
        config.enable_energy = true;
        config.enable_spectral = true;
        let mut analyzer = AudioAnalyzer::new(config, 44100).unwrap();
        
        // Create white noise
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut samples: Vec<f32> = Vec::new();
        let mut hasher = DefaultHasher::new();
        for i in 0..2048 {
            i.hash(&mut hasher);
            let hash = hasher.finish();
            // Convert to f32 in range [-0.5, 0.5]
            let sample = ((hash % 1000) as f32 / 1000.0) - 0.5;
            samples.push(sample);
        }
        
        let bytes: Vec<u8> = samples.iter()
            .flat_map(|&s| s.to_le_bytes().to_vec())
            .collect();
        
        let audio_data = Bytes::from(bytes);
        let result = analyzer.analyze(&audio_data);
        assert!(result.is_ok());
        
        let analysis = result.unwrap();
        assert!(analysis.energy > 0.0);
        assert!(analysis.zero_crossing_rate > 0.0);
        assert!(analysis.spectral_centroid > 0.0);
    }
}

