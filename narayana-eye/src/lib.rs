//! narayana-eye: Vision Interface for narayana-wld
//!
//! A state-of-the-art machine vision system that provides object detection,
//! segmentation, tracking, and scene understanding capabilities for robots.
//!
//! Integrates with narayana-wld as a protocol adapter to provide vision
//! events to the cognitive system.

pub mod vision_adapter;
pub mod camera;
pub mod config;
pub mod models;
pub mod processing;
pub mod scene;
pub mod error;
mod utils;

pub use vision_adapter::VisionAdapter;
pub use config::{VisionConfig, ProcessingMode};
pub use error::VisionError;

