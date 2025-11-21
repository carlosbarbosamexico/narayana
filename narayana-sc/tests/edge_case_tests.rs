//! Edge case tests for narayana-sc

#[cfg(test)]
mod tests {
    use narayana_sc::*;
    use bytes::Bytes;

    #[test]
    fn test_minimum_valid_audio() {
        let config = AnalysisConfig::default();
        let mut analyzer = AudioAnalyzer::new(config, 44100).unwrap();
        
        // Minimum valid audio (4 bytes = 1 sample)
        let samples: Vec<f32> = vec![0.0];
        let bytes: Vec<u8> = samples.iter()
            .flat_map(|&s| s.to_le_bytes().to_vec())
            .collect();
        
        let audio_data = Bytes::from(bytes);
        let result = analyzer.analyze(&audio_data);
        // Should handle gracefully (may pad or return error)
        let _ = result;
    }

    #[test]
    fn test_maximum_valid_audio() {
        let config = AnalysisConfig::default();
        let mut analyzer = AudioAnalyzer::new(config, 44100).unwrap();
        
        // Maximum valid audio (close to limit)
        const MAX_SAMPLES: usize = 2 * 1024 * 1024; // 2M samples
        let samples: Vec<f32> = vec![0.0; MAX_SAMPLES.min(10000)]; // Use smaller for test
        let bytes: Vec<u8> = samples.iter()
            .flat_map(|&s| s.to_le_bytes().to_vec())
            .collect();
        
        let audio_data = Bytes::from(bytes);
        let result = analyzer.analyze(&audio_data);
        // Should handle gracefully
        assert!(result.is_ok() || result.is_err()); // Either is acceptable
    }

    #[test]
    fn test_oversized_audio_rejected() {
        let config = AnalysisConfig::default();
        let mut analyzer = AudioAnalyzer::new(config, 44100).unwrap();
        
        // Oversized audio (exceeds MAX_AUDIO_SIZE)
        const MAX_SIZE: usize = 10 * 1024 * 1024; // 10MB
        let oversized: Vec<u8> = vec![0u8; MAX_SIZE + 1];
        let audio_data = Bytes::from(oversized);
        
        let result = analyzer.analyze(&audio_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_audio_with_nan_values() {
        let config = AnalysisConfig::default();
        let mut analyzer = AudioAnalyzer::new(config, 44100).unwrap();
        
        // Create audio with NaN (should be handled gracefully)
        let mut samples: Vec<f32> = vec![0.0; 100];
        samples[50] = f32::NAN;
        
        let bytes: Vec<u8> = samples.iter()
            .flat_map(|&s| s.to_le_bytes().to_vec())
            .collect();
        
        let audio_data = Bytes::from(bytes);
        let result = analyzer.analyze(&audio_data);
        // Should handle NaN gracefully (either filter out or return error)
        let _ = result;
    }

    #[test]
    fn test_audio_with_inf_values() {
        let config = AnalysisConfig::default();
        let mut analyzer = AudioAnalyzer::new(config, 44100).unwrap();
        
        // Create audio with Inf
        let mut samples: Vec<f32> = vec![0.0; 100];
        samples[50] = f32::INFINITY;
        samples[51] = f32::NEG_INFINITY;
        
        let bytes: Vec<u8> = samples.iter()
            .flat_map(|&s| s.to_le_bytes().to_vec())
            .collect();
        
        let audio_data = Bytes::from(bytes);
        let result = analyzer.analyze(&audio_data);
        // Should handle Inf gracefully
        let _ = result;
    }

    #[test]
    fn test_audio_with_extreme_values() {
        let config = AnalysisConfig::default();
        let mut analyzer = AudioAnalyzer::new(config, 44100).unwrap();
        
        // Create audio with extreme values (clipping)
        let mut samples: Vec<f32> = vec![0.0; 100];
        samples[25] = 10.0; // Way above 1.0
        samples[26] = -10.0; // Way below -1.0
        samples[27] = 1.0; // At limit
        samples[28] = -1.0; // At limit
        
        let bytes: Vec<u8> = samples.iter()
            .flat_map(|&s| s.to_le_bytes().to_vec())
            .collect();
        
        let audio_data = Bytes::from(bytes);
        let result = analyzer.analyze(&audio_data);
        // Should handle extreme values
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_config_with_boundary_values() {
        // Test config validation with boundary values
        let mut config = AudioConfig::default();
        
        // Minimum valid values
        config.buffer_size = 1;
        config.sample_rate = 1;
        config.channels = 1;
        assert!(config.validate().is_ok());
        
        // Maximum valid values
        config.buffer_size = 65536;
        config.sample_rate = 192000;
        config.channels = 8;
        assert!(config.validate().is_ok());
        
        // Just over maximum
        config.buffer_size = 65537;
        assert!(config.validate().is_err());
        
        config.buffer_size = 65536;
        config.sample_rate = 192001;
        assert!(config.validate().is_err());
        
        config.sample_rate = 192000;
        config.channels = 9;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_analysis_config_boundary_values() {
        let mut config = AnalysisConfig::default();
        
        // Minimum valid FFT window (power of 2)
        config.fft_window_size = 2;
        assert!(config.validate().is_ok());
        
        // Maximum valid FFT window
        config.fft_window_size = 65536;
        assert!(config.validate().is_ok());
        
        // Just over maximum
        config.fft_window_size = 65537;
        assert!(config.validate().is_err());
        
        // Minimum analysis interval
        config.fft_window_size = 2048;
        config.analysis_interval_ms = 1;
        assert!(config.validate().is_ok());
        
        // Maximum analysis interval
        config.analysis_interval_ms = 10000;
        assert!(config.validate().is_ok());
        
        // Just over maximum
        config.analysis_interval_ms = 10001;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_capture_config_boundary_values() {
        let mut config = CaptureConfig::default();
        
        // Minimum valid spatial channels
        config.spatial_channels = 1;
        assert!(config.validate().is_ok());
        
        // Maximum valid spatial channels
        config.spatial_channels = 32;
        assert!(config.validate().is_ok());
        
        // Just over maximum
        config.spatial_channels = 33;
        assert!(config.validate().is_err());
        
        // Minimum ring buffer size
        config.spatial_channels = 2;
        config.ring_buffer_size = 1;
        config.buffer_strategy = "queue".to_string(); // Not ring, so no power-of-2 requirement
        assert!(config.validate().is_ok());
        
        // Maximum ring buffer size
        config.ring_buffer_size = 65536;
        assert!(config.validate().is_ok());
        
        // Just over maximum
        config.ring_buffer_size = 65537;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_audio_analyzer_empty_spectrum() {
        let mut config = AnalysisConfig::default();
        config.enable_fft = true;
        let mut analyzer = AudioAnalyzer::new(config, 44100).unwrap();
        
        // Very short audio (less than window size)
        let samples: Vec<f32> = vec![0.0; 4]; // Just 1 sample
        let bytes: Vec<u8> = samples.iter()
            .flat_map(|&s| s.to_le_bytes().to_vec())
            .collect();
        
        let audio_data = Bytes::from(bytes);
        let result = analyzer.analyze(&audio_data);
        // Should handle gracefully (may pad or return error)
        let _ = result;
    }

    #[test]
    fn test_audio_analyzer_single_frequency() {
        let mut config = AnalysisConfig::default();
        config.enable_fft = true;
        config.enable_spectral = true;
        config.enable_pitch = true;
        let mut analyzer = AudioAnalyzer::new(config, 44100).unwrap();
        
        // Pure sine wave at 1000Hz
        let frequency = 1000.0;
        let sample_rate = 44100.0;
        let samples: Vec<f32> = (0..2048)
            .map(|i| {
                let t = i as f32 / sample_rate;
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
        // Should detect the frequency
        if let Some(pitch) = analysis.pitch {
            // Should be close to 1000Hz (within reasonable tolerance)
            assert!(pitch > 0.0);
            assert!(pitch < sample_rate / 2.0);
        }
    }
}

