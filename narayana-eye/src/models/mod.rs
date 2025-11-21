//! Vision model management and inference

pub mod manager;
pub mod yolo;
pub mod sam;
pub mod clip;

pub use manager::ModelManager;
pub use yolo::{YoloModel, DetectedObject};
pub use sam::SamModel;
pub use clip::ClipModel;

