//! Error types for narayana-eye

use thiserror::Error;
use narayana_core::Error as CoreError;

#[derive(Error, Debug)]
pub enum VisionError {
    #[error("Camera error: {0}")]
    Camera(String),

    #[error("Model error: {0}")]
    Model(String),

    #[error("Processing error: {0}")]
    Processing(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("ONNX Runtime error: {0}")]
    Ort(String),

    #[error("OpenCV error: {0}")]
    OpenCv(String),

    #[error("Core error: {0}")]
    Core(#[from] CoreError),
}

impl From<VisionError> for CoreError {
    fn from(err: VisionError) -> Self {
        CoreError::Storage(format!("Vision error: {}", err))
    }
}

impl From<opencv::Error> for VisionError {
    fn from(err: opencv::Error) -> Self {
        VisionError::OpenCv(err.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vision_error_display() {
        let err = VisionError::Camera("Test error".to_string());
        assert!(err.to_string().contains("Camera error"));
        assert!(err.to_string().contains("Test error"));
    }

    #[test]
    fn test_vision_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let vision_err: VisionError = io_err.into();
        match vision_err {
            VisionError::Io(_) => {}
            _ => panic!("Expected Io error"),
        }
    }

    #[test]
    fn test_vision_error_to_core_error() {
        let vision_err = VisionError::Camera("Test".to_string());
        let core_err: CoreError = vision_err.into();
        match core_err {
            CoreError::Storage(msg) => {
                assert!(msg.contains("Vision error"));
                assert!(msg.contains("Test"));
            }
            _ => panic!("Expected Storage error"),
        }
    }

    #[test]
    fn test_all_error_variants() {
        let _ = VisionError::Camera("camera".to_string());
        let _ = VisionError::Model("model".to_string());
        let _ = VisionError::Processing("processing".to_string());
        let _ = VisionError::Config("config".to_string());
        let _ = VisionError::Ort("ort".to_string());
        let _ = VisionError::OpenCv("opencv".to_string());
    }
}
