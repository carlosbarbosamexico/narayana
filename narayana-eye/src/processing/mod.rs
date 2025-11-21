//! Vision processing pipelines

pub mod detection;
pub mod segmentation;
pub mod tracker;

pub use detection::DetectionPipeline;
pub use segmentation::SegmentationPipeline;
pub use tracker::ObjectTracker;


