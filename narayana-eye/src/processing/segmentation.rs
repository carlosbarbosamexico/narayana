//! Instance segmentation pipeline

use crate::error::VisionError;
use crate::models::{SamModel, SegmentationMask};
use opencv::prelude::Mat;
use std::sync::Arc;
use tracing::debug;

/// Instance segmentation pipeline
pub struct SegmentationPipeline {
    sam: Arc<SamModel>,
}

impl SegmentationPipeline {
    /// Create a new segmentation pipeline
    pub fn new(sam: Arc<SamModel>) -> Self {
        Self { sam }
    }

    /// Process frame and segment objects
    pub fn segment(&self, frame: &Mat, prompts: &[(f32, f32)]) -> Result<Vec<SegmentationMask>, VisionError> {
        debug!("Running instance segmentation on frame");
        let masks = self.sam.segment(frame, prompts)?;
        debug!("Generated {} segmentation masks", masks.len());
        Ok(masks)
    }
}


