//! Configuration for narayana-eye

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Processing mode for vision pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessingMode {
    /// Continuous real-time processing
    RealTime,
    /// Process frames only on demand
    OnDemand,
}

/// Vision system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionConfig {
    /// USB camera device index (0, 1, 2, etc.)
    pub camera_id: u32,
    /// Target frame rate (frames per second)
    pub frame_rate: u32,
    /// Camera resolution (width, height)
    pub resolution: (u32, u32),
    /// Enable object detection
    pub enable_detection: bool,
    /// Enable instance segmentation
    pub enable_segmentation: bool,
    /// Enable object tracking
    pub enable_tracking: bool,
    /// Enable scene understanding
    pub enable_scene_understanding: bool,
    /// Enable LLM integration for descriptions (brain-controlled)
    pub llm_integration: bool,
    /// Path to store models
    pub model_path: PathBuf,
    /// Processing mode
    pub processing_mode: ProcessingMode,
}

impl Default for VisionConfig {
    fn default() -> Self {
        let model_path = dirs::home_dir()
            .map(|mut p| {
                p.push(".narayana");
                p.push("models");
                p
            })
            .unwrap_or_else(|| PathBuf::from("./models"));

        Self {
            camera_id: 0,
            frame_rate: 30,
            resolution: (640, 480),
            enable_detection: true,
            enable_segmentation: false,
            enable_tracking: true,
            enable_scene_understanding: true,
            llm_integration: false,
            model_path,
            processing_mode: ProcessingMode::RealTime,
        }
    }
}

impl VisionConfig {
    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.frame_rate == 0 || self.frame_rate > 120 {
            return Err("Frame rate must be between 1 and 120".to_string());
        }

        if self.resolution.0 == 0 || self.resolution.1 == 0 {
            return Err("Resolution must be non-zero".to_string());
        }

        if self.resolution.0 > 7680 || self.resolution.1 > 4320 {
            return Err("Resolution too large (max 8K)".to_string());
        }
        
        // Check for potential overflow in pixel calculations
        let total_pixels = self.resolution.0
            .checked_mul(self.resolution.1)
            .ok_or_else(|| "Resolution would cause integer overflow".to_string())?;
        
        if total_pixels > 100_000_000 {
            return Err("Resolution too large (max 100M pixels)".to_string());
        }
        
        // Validate camera_id is reasonable (prevent negative or extremely large values)
        if self.camera_id > 100 {
            return Err("Camera ID too large (max 100)".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = VisionConfig::default();
        assert_eq!(config.camera_id, 0);
        assert_eq!(config.frame_rate, 30);
        assert_eq!(config.resolution, (640, 480));
        assert!(config.enable_detection);
        assert!(!config.enable_segmentation);
        assert!(config.enable_tracking);
        assert!(config.enable_scene_understanding);
        assert!(!config.llm_integration);
        assert_eq!(config.processing_mode, ProcessingMode::RealTime);
    }

    #[test]
    fn test_config_validation_valid() {
        let config = VisionConfig {
            camera_id: 0,
            frame_rate: 30,
            resolution: (640, 480),
            enable_detection: true,
            enable_segmentation: false,
            enable_tracking: true,
            enable_scene_understanding: true,
            llm_integration: false,
            model_path: PathBuf::from("./models"),
            processing_mode: ProcessingMode::RealTime,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_frame_rate_zero() {
        let mut config = VisionConfig::default();
        config.frame_rate = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_frame_rate_too_high() {
        let mut config = VisionConfig::default();
        config.frame_rate = 121;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_resolution_zero() {
        let mut config = VisionConfig::default();
        config.resolution = (0, 480);
        assert!(config.validate().is_err());
        
        config.resolution = (640, 0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_resolution_too_large() {
        let mut config = VisionConfig::default();
        config.resolution = (7681, 4320);
        assert!(config.validate().is_err());
        
        config.resolution = (7680, 4321);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_resolution_overflow() {
        let mut config = VisionConfig::default();
        // Use values that would overflow when multiplied
        config.resolution = (u32::MAX, 2);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_resolution_max_pixels() {
        let mut config = VisionConfig::default();
        // 100M pixels = 10000 x 10000
        config.resolution = (10001, 10000);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_camera_id_too_large() {
        let mut config = VisionConfig::default();
        config.camera_id = 101;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_edge_cases() {
        let mut config = VisionConfig::default();
        
        // Valid edge cases
        config.frame_rate = 1;
        config.resolution = (1, 1);
        config.camera_id = 100;
        assert!(config.validate().is_ok());
        
        config.frame_rate = 120;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_processing_mode_equality() {
        assert_eq!(ProcessingMode::RealTime, ProcessingMode::RealTime);
        assert_eq!(ProcessingMode::OnDemand, ProcessingMode::OnDemand);
        assert_ne!(ProcessingMode::RealTime, ProcessingMode::OnDemand);
    }
}
