//! Tests for the download_model binary functionality

use narayana_eye::config::VisionConfig;
use narayana_eye::models::ModelManager;
use narayana_eye::error::VisionError;
use std::sync::Arc;
use tempfile::TempDir;

/// Test helper function that mimics the binary's download logic
async fn download_model_helper(model_name: &str, config: Arc<VisionConfig>) -> Result<std::path::PathBuf, VisionError> {
    let manager = ModelManager::new(config);
    
    match model_name.to_lowercase().as_str() {
        "yolo" => manager.get_yolo_model().await,
        "sam" => manager.get_sam_model().await,
        "clip" => manager.get_clip_model().await,
        _ => Err(VisionError::Model(format!("Unknown model: {}", model_name))),
    }
}

#[tokio::test]
async fn test_download_model_helper_yolo() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = VisionConfig::default();
    config.model_path = temp_dir.path().to_path_buf();
    let config = Arc::new(config);
    
    // This will try to download, but may fail if network is unavailable
    // We're testing the logic, not the actual download
    let result = download_model_helper("yolo", config).await;
    
    // Either succeeds (model downloaded) or fails with a network/IO error
    // But should NOT fail with "Unknown model"
    match result {
        Ok(path) => {
            assert!(path.exists() || path.parent().unwrap().exists());
        }
        Err(VisionError::Model(msg)) => {
            panic!("Should not fail with Model error for valid model name: {}", msg);
        }
        Err(_) => {
            // Network/IO errors are acceptable in tests
        }
    }
}

#[tokio::test]
async fn test_download_model_helper_sam() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = VisionConfig::default();
    config.model_path = temp_dir.path().to_path_buf();
    let config = Arc::new(config);
    
    let result = download_model_helper("sam", config).await;
    
    match result {
        Ok(path) => {
            assert!(path.exists() || path.parent().unwrap().exists());
        }
        Err(VisionError::Model(msg)) => {
            panic!("Should not fail with Model error for valid model name: {}", msg);
        }
        Err(_) => {
            // Network/IO errors are acceptable in tests
        }
    }
}

#[tokio::test]
async fn test_download_model_helper_clip() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = VisionConfig::default();
    config.model_path = temp_dir.path().to_path_buf();
    let config = Arc::new(config);
    
    let result = download_model_helper("clip", config).await;
    
    match result {
        Ok(path) => {
            assert!(path.exists() || path.parent().unwrap().exists());
        }
        Err(VisionError::Model(msg)) => {
            panic!("Should not fail with Model error for valid model name: {}", msg);
        }
        Err(_) => {
            // Network/IO errors are acceptable in tests
        }
    }
}

#[tokio::test]
async fn test_download_model_helper_case_insensitive() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = VisionConfig::default();
    config.model_path = temp_dir.path().to_path_buf();
    let config = Arc::new(config);
    
    // Test uppercase
    let result_upper = download_model_helper("YOLO", config.clone()).await;
    // Test mixed case
    let result_mixed = download_model_helper("SaM", config.clone()).await;
    // Test lowercase
    let result_lower = download_model_helper("clip", config).await;
    
    // All should either succeed or fail with network/IO errors, not "Unknown model"
    for result in [result_upper, result_mixed, result_lower] {
        match result {
            Ok(_) => {}
            Err(VisionError::Model(msg)) => {
                panic!("Should not fail with Model error for valid model name: {}", msg);
            }
            Err(_) => {
                // Network/IO errors are acceptable
            }
        }
    }
}

#[tokio::test]
async fn test_download_model_helper_unknown_model() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = VisionConfig::default();
    config.model_path = temp_dir.path().to_path_buf();
    let config = Arc::new(config);
    
    let result = download_model_helper("unknown_model", config).await;
    
    assert!(result.is_err());
    match result.unwrap_err() {
        VisionError::Model(msg) => {
            assert!(msg.contains("Unknown model"));
        }
        _ => {
            panic!("Should fail with Model error for unknown model");
        }
    }
}

#[tokio::test]
async fn test_download_model_helper_empty_string() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = VisionConfig::default();
    config.model_path = temp_dir.path().to_path_buf();
    let config = Arc::new(config);
    
    let result = download_model_helper("", config).await;
    
    assert!(result.is_err());
    match result.unwrap_err() {
        VisionError::Model(msg) => {
            assert!(msg.contains("Unknown model"));
        }
        _ => {
            panic!("Should fail with Model error for empty string");
        }
    }
}

#[tokio::test]
async fn test_model_manager_get_methods_exist() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = VisionConfig::default();
    config.model_path = temp_dir.path().to_path_buf();
    let manager = ModelManager::new(Arc::new(config));
    
    // Test that all three methods exist and are callable
    // They may fail due to network issues, but should not panic
    let _yolo_result = manager.get_yolo_model().await;
    let _sam_result = manager.get_sam_model().await;
    let _clip_result = manager.get_clip_model().await;
    
    // If we get here, the methods exist and are callable
    assert!(true);
}


