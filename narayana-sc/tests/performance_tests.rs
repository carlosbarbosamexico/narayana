//! Performance tests for narayana-sc

#[cfg(test)]
mod tests {
    use narayana_sc::*;
    use bytes::Bytes;
    use std::time::Instant;

    #[test]
    fn test_audio_analyzer_performance() {
        let mut config = AnalysisConfig::default();
        config.enable_fft = true;
        config.enable_energy = true;
        config.enable_zcr = true;
        config.enable_spectral = true;
        config.enable_pitch = true;
        
        let mut analyzer = AudioAnalyzer::new(config, 44100).unwrap();
        
        // Create realistic audio data
        let samples: Vec<f32> = (0..2048)
            .map(|i| {
                let t = i as f32 / 44100.0;
                (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5
            })
            .collect();
        
        let bytes: Vec<u8> = samples.iter()
            .flat_map(|&s| s.to_le_bytes().to_vec())
            .collect();
        
        let audio_data = Bytes::from(bytes);
        
        // Measure analysis time
        let start = Instant::now();
        let result = analyzer.analyze(&audio_data);
        let duration = start.elapsed();
        
        assert!(result.is_ok());
        
        // Analysis should complete in reasonable time (< 100ms for 2048 samples)
        assert!(duration.as_millis() < 100, "Analysis took too long: {:?}", duration);
    }

    #[test]
    fn test_audio_analyzer_throughput() {
        let mut config = AnalysisConfig::default();
        config.enable_fft = true;
        config.enable_energy = true;
        let mut analyzer = AudioAnalyzer::new(config, 44100).unwrap();
        
        // Create audio data
        let samples: Vec<f32> = vec![0.5; 2048];
        let bytes: Vec<u8> = samples.iter()
            .flat_map(|&s| s.to_le_bytes().to_vec())
            .collect();
        
        let audio_data = Bytes::from(bytes);
        
        // Process multiple times
        let iterations = 100;
        let start = Instant::now();
        
        for _ in 0..iterations {
            let _ = analyzer.analyze(&audio_data);
        }
        
        let duration = start.elapsed();
        let avg_time = duration / iterations;
        
        // Average time should be reasonable
        assert!(avg_time.as_millis() < 50, "Average analysis time too high: {:?}", avg_time);
    }

    #[test]
    fn test_config_validation_performance() {
        let config = AudioConfig::default();
        
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = config.validate();
        }
        let duration = start.elapsed();
        
        // Validation should be very fast
        assert!(duration.as_millis() < 100, "Validation too slow: {:?}", duration);
    }

    #[test]
    fn test_large_audio_processing() {
        let mut config = AnalysisConfig::default();
        config.enable_fft = true;
        let mut analyzer = AudioAnalyzer::new(config, 44100).unwrap();
        
        // Large audio buffer (but within limits)
        let samples: Vec<f32> = vec![0.0; 100000]; // ~100KB
        let bytes: Vec<u8> = samples.iter()
            .flat_map(|&s| s.to_le_bytes().to_vec())
            .collect();
        
        let audio_data = Bytes::from(bytes);
        
        let start = Instant::now();
        let result = analyzer.analyze(&audio_data);
        let duration = start.elapsed();
        
        assert!(result.is_ok());
        // Should handle large buffers efficiently
        assert!(duration.as_secs() < 1, "Large buffer processing too slow: {:?}", duration);
    }
}

