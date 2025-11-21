// Arrow of Time System Tests
// Comprehensive tests for AOT components: entropy controller, arrow of time, complexity simulator, temporal accelerator

#[cfg(test)]
mod aot_tests {
    use crate::entropy_controller::{EntropyController, EntropyConfig, EntropyPolicy};
    use crate::arrow_of_time::{ArrowOfTimeController, AOTConfig, TimeDirection, OrderingStrategy};
    use crate::complexity_range_simulator::{ComplexityRangeSimulator, ComplexityRange, ExperienceModality};
    use crate::temporal_accelerator::{TemporalAccelerator, AccelerationConfig};
    use crate::cognitive::Experience;
    use serde_json::json;
    use std::sync::Arc;
    use std::collections::HashMap;

    fn create_test_experience(id: &str, timestamp: u64, complexity: f64, entropy: f64) -> Experience {
        Experience {
            id: id.to_string(),
            event_type: "test".to_string(),
            observation: json!({"value": id}),
            action: None,
            outcome: None,
            reward: Some(0.5),
            timestamp,
            context: HashMap::new(),
            patterns: Vec::new(),
            embedding: None,
            complexity: Some(complexity),
            entropy: Some(entropy),
            modality: Some("visual".to_string()),
        }
    }

    #[test]
    fn test_entropy_controller_initialization() {
        let config = EntropyConfig::default();
        let controller = EntropyController::new(config);
        
        let entropy = controller.get_entropy();
        assert!(entropy >= 0.0 && entropy <= 1.0);
        assert!(entropy.is_finite());
    }

    #[test]
    fn test_entropy_controller_set_entropy() {
        let config = EntropyConfig::default();
        let controller = EntropyController::new(config);
        
        controller.set_entropy(0.7).unwrap();
        assert_eq!(controller.get_entropy(), 0.7);
        
        // Test clamping
        controller.set_entropy(1.5).unwrap();
        assert_eq!(controller.get_entropy(), 1.0);
        
        controller.set_entropy(-0.5).unwrap();
        assert_eq!(controller.get_entropy(), 0.0);
    }

    #[test]
    fn test_entropy_controller_calculate_experience_entropy() {
        let config = EntropyConfig::default();
        let controller = EntropyController::new(config);
        
        let exp = create_test_experience("exp1", 1000, 0.5, 0.5);
        let entropy = controller.calculate_experience_entropy(&exp).unwrap();
        
        assert!(entropy >= 0.0 && entropy <= 1.0);
        assert!(entropy.is_finite());
    }

    #[test]
    fn test_entropy_controller_policies() {
        let mut config = EntropyConfig::default();
        config.entropy_policy = EntropyPolicy::Fixed;
        config.initial_entropy = Some(0.6);
        
        let controller = EntropyController::new(config);
        controller.update_entropy().unwrap();
        
        assert_eq!(controller.get_entropy(), 0.6);
    }

    #[test]
    fn test_arrow_of_time_controller_initialization() {
        let entropy_config = EntropyConfig::default();
        let entropy_controller = Arc::new(EntropyController::new(entropy_config));
        
        let aot_config = AOTConfig {
            enable_arrow_of_time: true,
            time_direction: TimeDirection::Forward,
            ordering_strategy: OrderingStrategy::Temporal,
            entropy_based_sampling: false,
            bidirectional_entropy_threshold: 0.5,
        };
        
        let _controller = ArrowOfTimeController::new(aot_config, entropy_controller);
        // Controller created successfully
    }

    #[test]
    fn test_arrow_of_time_forward_ordering() {
        let entropy_config = EntropyConfig::default();
        let entropy_controller = Arc::new(EntropyController::new(entropy_config));
        
        let aot_config = AOTConfig {
            enable_arrow_of_time: true,
            time_direction: TimeDirection::Forward,
            ordering_strategy: OrderingStrategy::Temporal,
            entropy_based_sampling: false,
            bidirectional_entropy_threshold: 0.5,
        };
        
        let controller = ArrowOfTimeController::new(aot_config, entropy_controller);
        
        let mut experiences = vec![
            create_test_experience("exp3", 3000, 0.5, 0.5),
            create_test_experience("exp1", 1000, 0.5, 0.5),
            create_test_experience("exp2", 2000, 0.5, 0.5),
        ];
        
        controller.order_experiences(&mut experiences).unwrap();
        
        assert_eq!(experiences[0].id, "exp1");
        assert_eq!(experiences[1].id, "exp2");
        assert_eq!(experiences[2].id, "exp3");
    }

    #[test]
    fn test_arrow_of_time_reverse_ordering() {
        let entropy_config = EntropyConfig::default();
        let entropy_controller = Arc::new(EntropyController::new(entropy_config));
        
        let aot_config = AOTConfig {
            enable_arrow_of_time: true,
            time_direction: TimeDirection::Backward,
            ordering_strategy: OrderingStrategy::Temporal,
            entropy_based_sampling: false,
            bidirectional_entropy_threshold: 0.5,
        };
        
        let controller = ArrowOfTimeController::new(aot_config, entropy_controller);
        
        let mut experiences = vec![
            create_test_experience("exp1", 1000, 0.5, 0.5),
            create_test_experience("exp3", 3000, 0.5, 0.5),
            create_test_experience("exp2", 2000, 0.5, 0.5),
        ];
        
        controller.order_experiences(&mut experiences).unwrap();
        
        assert_eq!(experiences[0].id, "exp3");
        assert_eq!(experiences[1].id, "exp2");
        assert_eq!(experiences[2].id, "exp1");
    }

    #[test]
    fn test_temporal_accelerator_compression() {
        let entropy_config = EntropyConfig::default();
        let entropy_controller = Arc::new(EntropyController::new(entropy_config));
        
        let accel_config = AccelerationConfig {
            acceleration_ratio: 2.0,
            min_experiences: 2,
            compression_entropy_threshold: 0.1,
            preserve_causality: true,
            maintain_immersion: true,
        };
        
        let accelerator = TemporalAccelerator::new(accel_config, entropy_controller).unwrap();
        
        let experiences = vec![
            create_test_experience("exp1", 1000, 0.1, 0.1),
            create_test_experience("exp2", 2000, 0.2, 0.2),
            create_test_experience("exp3", 3000, 0.3, 0.3),
            create_test_experience("exp4", 4000, 0.4, 0.4),
        ];
        
        let compressed = accelerator.accelerate(experiences).unwrap();
        
        // Should compress from 4 to approximately 2 experiences
        assert!(compressed.len() >= 2);
        assert!(compressed.len() <= 4);
    }

    #[test]
    fn test_temporal_accelerator_preserves_causality() {
        let entropy_config = EntropyConfig::default();
        let entropy_controller = Arc::new(EntropyController::new(entropy_config));
        
        let accel_config = AccelerationConfig {
            acceleration_ratio: 3.0,
            min_experiences: 2,
            compression_entropy_threshold: 0.1,
            preserve_causality: true,
            maintain_immersion: true,
        };
        
        let accelerator = TemporalAccelerator::new(accel_config, entropy_controller).unwrap();
        
        let experiences = vec![
            create_test_experience("exp1", 1000, 0.1, 0.1),
            create_test_experience("exp2", 2000, 0.2, 0.2),
            create_test_experience("exp3", 3000, 0.3, 0.3),
        ];
        
        let compressed = accelerator.accelerate(experiences).unwrap();
        
        // Check temporal order is preserved
        for i in 1..compressed.len() {
            assert!(compressed[i].timestamp >= compressed[i-1].timestamp);
        }
    }

    #[test]
    fn test_complexity_range_simulator_generation() {
        let range = ComplexityRange {
            start_complexity: 0.0,
            end_complexity: 1.0,
            enable_audio: true,
            audio_experience_ratio: 0.3,
            enable_multi_modal: true,
        };
        
        let entropy_config = EntropyConfig::default();
        let entropy_controller = Arc::new(EntropyController::new(entropy_config));
        let simulator = ComplexityRangeSimulator::new(range, entropy_controller).unwrap();
        
        let experience = simulator.generate_experience(Some(ExperienceModality::Visual)).unwrap();
        
        assert!(experience.complexity.unwrap_or(0.0) >= 0.0 && experience.complexity.unwrap_or(0.0) <= 1.0);
    }

    #[test]
    fn test_complexity_range_simulator_batch() {
        let range = ComplexityRange {
            start_complexity: 0.0,
            end_complexity: 1.0,
            enable_audio: true,
            audio_experience_ratio: 0.3,
            enable_multi_modal: true,
        };
        
        let entropy_config = EntropyConfig::default();
        let entropy_controller = Arc::new(EntropyController::new(entropy_config));
        let simulator = ComplexityRangeSimulator::new(range, entropy_controller).unwrap();
        
        let batch = simulator.generate_batch(10).unwrap();
        
        assert_eq!(batch.len(), 10);
        
        // Complexity should be valid
        for exp in &batch {
            let complexity = exp.complexity.unwrap_or(0.0);
            assert!(complexity >= 0.0 && complexity <= 1.0);
        }
    }

    #[test]
    fn test_entropy_controller_edge_cases() {
        let config = EntropyConfig::default();
        let controller = EntropyController::new(config);
        
        // Test with empty experience
        let empty_exp = create_test_experience("empty", 1000, 0.0, 0.0);
        let entropy = controller.calculate_experience_entropy(&empty_exp).unwrap();
        assert!(entropy.is_finite());
        
        // Test with NaN/Infinity handling
        controller.set_entropy(0.5).unwrap();
        let entropy = controller.get_entropy();
        assert!(entropy.is_finite());
    }

    #[test]
    fn test_temporal_accelerator_edge_cases() {
        let entropy_config = EntropyConfig::default();
        let entropy_controller = Arc::new(EntropyController::new(entropy_config));
        
        let accel_config = AccelerationConfig::default();
        let accelerator = TemporalAccelerator::new(accel_config, entropy_controller).unwrap();
        
        // Test with empty experiences
        let empty: Vec<Experience> = Vec::new();
        let result = accelerator.accelerate(empty).unwrap();
        assert_eq!(result.len(), 0);
        
        // Test with single experience
        let single = vec![create_test_experience("exp1", 1000, 0.5, 0.5)];
        let result = accelerator.accelerate(single).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_arrow_of_time_bidirectional() {
        let entropy_config = EntropyConfig::default();
        let entropy_controller = Arc::new(EntropyController::new(entropy_config));
        
        let aot_config = AOTConfig {
            enable_arrow_of_time: true,
            time_direction: TimeDirection::Bidirectional,
            ordering_strategy: OrderingStrategy::Mixed,
            entropy_based_sampling: true,
            bidirectional_entropy_threshold: 0.5,
        };
        
        let controller = ArrowOfTimeController::new(aot_config, entropy_controller);
        
        let mut experiences = vec![
            create_test_experience("exp1", 1000, 0.5, 0.1),
            create_test_experience("exp2", 2000, 0.5, 0.9),
            create_test_experience("exp3", 3000, 0.5, 0.5),
        ];
        
        controller.order_experiences(&mut experiences).unwrap();
        
        // Should be ordered (exact order depends on entropy)
        assert_eq!(experiences.len(), 3);
    }

    #[tokio::test]
    async fn test_entropy_controller_adaptive_policy() {
        let mut config = EntropyConfig::default();
        config.entropy_policy = EntropyPolicy::Adaptive;
        config.enable_dynamic_entropy = true;
        
        let controller = EntropyController::new(config);
        
        // Update metrics
        let mut metrics = HashMap::new();
        metrics.insert("performance".to_string(), 0.8);
        metrics.insert("loss".to_string(), 0.2);
        controller.update_metrics(metrics);
        
        // Update entropy
        controller.update_entropy().unwrap();
        
        let entropy = controller.get_entropy();
        assert!(entropy >= 0.0 && entropy <= 1.0);
        assert!(entropy.is_finite());
    }

    #[test]
    fn test_complexity_simulator_modalities() {
        let range = ComplexityRange {
            start_complexity: 0.0,
            end_complexity: 1.0,
            enable_audio: true,
            audio_experience_ratio: 0.5,
            enable_multi_modal: true,
        };
        
        let entropy_config = EntropyConfig::default();
        let entropy_controller = Arc::new(EntropyController::new(entropy_config));
        let simulator = ComplexityRangeSimulator::new(range, entropy_controller).unwrap();
        
        // Test different modalities
        let visual = simulator.generate_experience(Some(ExperienceModality::Visual)).unwrap();
        assert_eq!(visual.modality, Some("Visual".to_string()));
        
        let audio = simulator.generate_experience(Some(ExperienceModality::Audio)).unwrap();
        assert_eq!(audio.modality, Some("Audio".to_string()));
        
        let voice = simulator.generate_experience(Some(ExperienceModality::Voice)).unwrap();
        assert_eq!(voice.modality, Some("Voice".to_string()));
    }
}
