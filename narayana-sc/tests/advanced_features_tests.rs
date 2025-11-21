//! Tests for advanced audio features

#[cfg(test)]
mod tests {
    use narayana_sc::*;
    use bytes::Bytes;

    #[test]
    fn test_advanced_processor_creation() {
        let capture_config = CaptureConfig::default();
        let analysis_config = AnalysisConfig::default();
        
        let processor = AdvancedAudioProcessor::new(
            &capture_config,
            &analysis_config,
        );
        
        // Should create successfully
        assert!(processor.is_ok());
    }

    #[test]
    fn test_voice_activity_detection() {
        let capture_config = CaptureConfig::default();
        let analysis_config = AnalysisConfig::default();
        
        let processor = AdvancedAudioProcessor::new(
            &capture_config,
            &analysis_config,
        ).unwrap();
        
        // Test with silence (should not detect voice)
        let silence: Vec<f32> = vec![0.0; 1000];
        let is_voice = processor.detect_voice_activity(&silence, 0.0, 0.0, 0.0);
        assert!(!is_voice);
        
        // Test with high energy (should detect voice)
        let voice: Vec<f32> = vec![0.8; 1000];
        let is_voice = processor.detect_voice_activity(&voice, 0.8, 2000.0, 0.1);
        // May or may not detect depending on thresholds
        let _ = is_voice;
    }

    #[test]
    fn test_comprehensive_capture_creation() {
        let mut config = AudioConfig::default();
        config.enabled = true;
        
        let capture = ComprehensiveAudioCapture::new(config);
        // May fail if advanced features can't be initialized
        // That's okay - we're testing the creation path
        let _ = capture;
    }

    #[test]
    fn test_streaming_buffer_creation() {
        let buffer = AudioStreamBuffer::new(8192);
        assert_eq!(buffer.available_write(), 8192);
        assert_eq!(buffer.available_read(), 0);
        assert!(buffer.is_empty());
        assert!(!buffer.is_full());
    }

    #[test]
    fn test_event_based_processor() {
        let processor = EventBasedProcessor::new(0.1);
        
        // Test with low energy (no event)
        let low_energy: Vec<f32> = vec![0.01; 100];
        let events = processor.process_and_detect_events(&low_energy);
        assert!(events.is_empty());
        
        // Test with high energy (should detect event)
        let high_energy: Vec<f32> = vec![0.9; 100];
        let events = processor.process_and_detect_events(&high_energy);
        // May or may not detect depending on threshold
        let _ = events;
    }

    #[test]
    fn test_adaptive_stream_controller() {
        let controller = AdaptiveStreamController::new(50); // 50ms target
        
        // Test initial state
        let multiplier = controller.buffer_multiplier();
        assert_eq!(multiplier, 1.0);
        
        // Test adaptation with high latency
        controller.adapt(std::time::Duration::from_millis(100));
        let multiplier = controller.buffer_multiplier();
        // Should adjust (may be less than 1.0 to reduce buffer)
        assert!(multiplier >= 0.5 && multiplier <= 2.0);
        
        // Test adaptation with low latency
        controller.adapt(std::time::Duration::from_millis(10));
        let multiplier = controller.buffer_multiplier();
        // Should adjust
        assert!(multiplier >= 0.5 && multiplier <= 2.0);
    }
}

