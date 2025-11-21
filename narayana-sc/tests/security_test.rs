//! Security tests for narayana-sc
//! Tests for input validation, resource limits, and edge cases

#[cfg(test)]
mod tests {
    use narayana_sc::*;
    use bytes::Bytes;

    #[test]
    fn test_empty_audio_data() {
        let config = AudioConfig::default();
        let analyzer = AudioAnalyzer::new(config.analysis.clone(), config.sample_rate).unwrap();
        
        let empty_data = Bytes::from(vec![]);
        let result = analyzer.analyze(&empty_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_audio_length() {
        let config = AudioConfig::default();
        let analyzer = AudioAnalyzer::new(config.analysis.clone(), config.sample_rate).unwrap();
        
        // Length not multiple of 4
        let invalid_data = Bytes::from(vec![0u8; 5]);
        let result = analyzer.analyze(&invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_oversized_audio_data() {
        let config = AudioConfig::default();
        let analyzer = AudioAnalyzer::new(config.analysis.clone(), config.sample_rate).unwrap();
        
        // Create data larger than MAX_AUDIO_SIZE (10MB)
        let oversized_data = Bytes::from(vec![0u8; 11 * 1024 * 1024]);
        let result = analyzer.analyze(&oversized_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_validation() {
        let mut config = AudioConfig::default();
        
        // Test invalid buffer size
        config.buffer_size = 0;
        assert!(config.validate().is_err());
        
        config.buffer_size = 100000; // Too large
        assert!(config.validate().is_err());
        
        // Test invalid sample rate
        config.buffer_size = 4096;
        config.sample_rate = 0;
        assert!(config.validate().is_err());
        
        config.sample_rate = 200000; // Too high
        assert!(config.validate().is_err());
        
        // Test invalid channels
        config.sample_rate = 44100;
        config.channels = 0;
        assert!(config.validate().is_err());
        
        config.channels = 10; // Too many
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_capture_config_validation() {
        let mut config = CaptureConfig::default();
        
        // Test invalid spatial channels
        config.spatial_channels = 0;
        assert!(config.validate().is_err());
        
        config.spatial_channels = 50; // Too many
        assert!(config.validate().is_err());
        
        // Test invalid ring buffer size
        config.spatial_channels = 2;
        config.ring_buffer_size = 0;
        assert!(config.validate().is_err());
        
        config.ring_buffer_size = 100000; // Too large
        assert!(config.validate().is_err());
        
        // Test invalid buffer strategy
        config.ring_buffer_size = 8192;
        config.buffer_strategy = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_analysis_config_validation() {
        let mut config = AnalysisConfig::default();
        
        // Test invalid FFT window size
        config.fft_window_size = 0;
        assert!(config.validate().is_err());
        
        config.fft_window_size = 100000; // Too large
        assert!(config.validate().is_err());
        
        // Test non-power-of-2 FFT window
        config.fft_window_size = 1000; // Not power of 2
        assert!(config.validate().is_err());
        
        // Test invalid analysis interval
        config.fft_window_size = 2048;
        config.analysis_interval_ms = 0;
        assert!(config.validate().is_err());
        
        config.analysis_interval_ms = 20000; // Too large
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_nan_inf_handling() {
        let config = AudioConfig::default();
        let analyzer = AudioAnalyzer::new(config.analysis.clone(), config.sample_rate).unwrap();
        
        // Create data with NaN/Inf values
        let mut data = vec![0u8; 16];
        // Set some bytes to create NaN
        data[0] = 0xFF;
        data[1] = 0xFF;
        data[2] = 0xFF;
        data[3] = 0x7F;
        
        let bytes = Bytes::from(data);
        // Should handle gracefully without panicking
        let _result = analyzer.analyze(&bytes);
    }
}

