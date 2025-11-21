//! Unit tests for narayana-sc components

#[cfg(test)]
mod tests {
    use narayana_sc::*;
    use bytes::Bytes;

    #[test]
    fn test_audio_config_default() {
        let config = AudioConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.sample_rate, 44100);
        assert_eq!(config.channels, 1);
        assert_eq!(config.buffer_size, 4096);
    }

    #[test]
    fn test_audio_config_validation() {
        let mut config = AudioConfig::default();
        
        // Valid config
        assert!(config.validate().is_ok());
        
        // Invalid buffer size
        config.buffer_size = 0;
        assert!(config.validate().is_err());
        
        config.buffer_size = 100000;
        assert!(config.validate().is_err());
        
        // Invalid sample rate
        config.buffer_size = 4096;
        config.sample_rate = 0;
        assert!(config.validate().is_err());
        
        config.sample_rate = 200000;
        assert!(config.validate().is_err());
        
        // Invalid channels
        config.sample_rate = 44100;
        config.channels = 0;
        assert!(config.validate().is_err());
        
        config.channels = 10;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_capture_config_default() {
        let config = CaptureConfig::default();
        assert_eq!(config.spatial_channels, 2);
        assert!(config.low_latency);
        assert_eq!(config.buffer_strategy, "ring");
    }

    #[test]
    fn test_capture_config_validation() {
        let mut config = CaptureConfig::default();
        
        // Valid config
        assert!(config.validate().is_ok());
        
        // Invalid spatial channels
        config.spatial_channels = 0;
        assert!(config.validate().is_err());
        
        config.spatial_channels = 50;
        assert!(config.validate().is_err());
        
        // Invalid buffer strategy
        config.spatial_channels = 2;
        config.buffer_strategy = "invalid".to_string();
        assert!(config.validate().is_err());
        
        // Invalid ring buffer size
        config.buffer_strategy = "ring".to_string();
        config.ring_buffer_size = 0;
        assert!(config.validate().is_err());
        
        config.ring_buffer_size = 100000;
        assert!(config.validate().is_err());
        
        // Non-power-of-2 for ring buffer
        config.ring_buffer_size = 1000;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_analysis_config_default() {
        let config = AnalysisConfig::default();
        assert_eq!(config.fft_window_size, 2048);
        assert_eq!(config.analysis_interval_ms, 100);
    }

    #[test]
    fn test_analysis_config_validation() {
        let mut config = AnalysisConfig::default();
        
        // Valid config
        assert!(config.validate().is_ok());
        
        // Invalid FFT window size
        config.fft_window_size = 0;
        assert!(config.validate().is_err());
        
        config.fft_window_size = 100000;
        assert!(config.validate().is_err());
        
        // Non-power-of-2 FFT window
        config.fft_window_size = 1000;
        assert!(config.validate().is_err());
        
        // Invalid analysis interval
        config.fft_window_size = 2048;
        config.analysis_interval_ms = 0;
        assert!(config.validate().is_err());
        
        config.analysis_interval_ms = 20000;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_audio_analyzer_creation() {
        let config = AnalysisConfig::default();
        let analyzer = AudioAnalyzer::new(config, 44100);
        assert!(analyzer.is_ok());
    }

    #[test]
    fn test_audio_analyzer_invalid_config() {
        let mut config = AnalysisConfig::default();
        config.fft_window_size = 0; // Invalid
        let analyzer = AudioAnalyzer::new(config, 44100);
        assert!(analyzer.is_err());
    }

    #[test]
    fn test_audio_analyzer_valid_audio() {
        let config = AnalysisConfig::default();
        let mut analyzer = AudioAnalyzer::new(config, 44100).unwrap();
        
        // Create valid audio data (f32 samples, 4 bytes each)
        let samples: Vec<f32> = vec![0.0; 1024];
        let bytes: Vec<u8> = samples.iter()
            .flat_map(|&s| s.to_le_bytes().to_vec())
            .collect();
        
        let audio_data = Bytes::from(bytes);
        let result = analyzer.analyze(&audio_data);
        assert!(result.is_ok());
        
        let analysis = result.unwrap();
        assert_eq!(analysis.spectrum.len(), 0); // FFT disabled by default
        assert_eq!(analysis.energy, 0.0);
    }

    #[test]
    fn test_audio_analyzer_with_fft() {
        let mut config = AnalysisConfig::default();
        config.enable_fft = true;
        config.enable_energy = true;
        let mut analyzer = AudioAnalyzer::new(config, 44100).unwrap();
        
        // Create valid audio data
        let samples: Vec<f32> = vec![0.5; 2048]; // Non-zero samples
        let bytes: Vec<u8> = samples.iter()
            .flat_map(|&s| s.to_le_bytes().to_vec())
            .collect();
        
        let audio_data = Bytes::from(bytes);
        let result = analyzer.analyze(&audio_data);
        assert!(result.is_ok());
        
        let analysis = result.unwrap();
        assert!(!analysis.spectrum.is_empty());
        assert!(analysis.energy > 0.0);
    }

    #[test]
    fn test_llm_processor_creation() {
        let processor = LlmAudioProcessor::new(false);
        assert!(!processor.is_available());
        
        let processor_enabled = LlmAudioProcessor::new(true);
        // Availability depends on feature flag and engine setup
        // Just verify it doesn't panic
        let _ = processor_enabled.is_available();
    }

    #[test]
    fn test_audio_error_types() {
        let config_error = AudioError::Config("test".to_string());
        assert!(format!("{}", config_error).contains("test"));
        
        let capture_error = AudioError::Capture("test".to_string());
        assert!(format!("{}", capture_error).contains("test"));
        
        let analysis_error = AudioError::Analysis("test".to_string());
        assert!(format!("{}", analysis_error).contains("test"));
    }

    #[test]
    fn test_config_serialization() {
        let config = AudioConfig::default();
        let json = serde_json::to_string(&config);
        assert!(json.is_ok());
        
        let json_str = json.unwrap();
        let deserialized: Result<AudioConfig, _> = serde_json::from_str(&json_str);
        assert!(deserialized.is_ok());
    }
}

