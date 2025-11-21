//! Object detection pipeline

use crate::error::VisionError;
use crate::models::{YoloModel, DetectedObject};
use opencv::prelude::Mat;
use std::sync::Arc;
use tracing::debug;

/// Object detection pipeline
pub struct DetectionPipeline {
    yolo: Arc<YoloModel>,
}

impl DetectionPipeline {
    /// Create a new detection pipeline
    pub fn new(yolo: Arc<YoloModel>) -> Self {
        Self { yolo }
    }

    /// Process frame and detect objects
    pub fn detect(&self, frame: &Mat) -> Result<Vec<DetectedObject>, VisionError> {
        debug!("Running object detection on frame");
        let detections = self.yolo.detect(frame)?;
        debug!("Detected {} objects", detections.len());
        Ok(detections)
    }
}


