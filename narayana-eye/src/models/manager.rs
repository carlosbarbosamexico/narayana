//! Model manager with auto-download functionality

use crate::error::VisionError;
use crate::config::VisionConfig;
use std::path::{Path, PathBuf};
use std::fs;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{info, warn};
use sha2::{Sha256, Digest};
use hex;

/// Model URLs and checksums
/// Note: These are example URLs. In production, use verified model repositories.
const YOLO_V8_URL: &str = "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8n.onnx";
const YOLO_V8_CHECKSUM: &str = ""; // Checksum validation can be added later

const SAM_VIT_B_URL: &str = "https://dl.fbaipublicfiles.com/segment_anything/sam_vit_b_01ec64.pth";
const SAM_VIT_B_CHECKSUM: &str = ""; // Note: SAM models are typically .pth (PyTorch), need ONNX conversion

const CLIP_VIT_B_32_URL: &str = "https://openaipublic.azureedge.net/clip/models/40d365715913c9da985793124b1dde49adaa2322/CLIP-ViT-B-32.pt";
const CLIP_VIT_B_32_CHECKSUM: &str = ""; // Note: CLIP models are typically .pt (PyTorch), need ONNX conversion

/// Model manager for downloading and managing vision models
pub struct ModelManager {
    config: Arc<VisionConfig>,
    models_loaded: Arc<RwLock<std::collections::HashMap<String, bool>>>,
}

impl ModelManager {
    /// Create a new model manager
    pub fn new(config: Arc<VisionConfig>) -> Self {
        Self {
            config,
            models_loaded: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Ensure model directory exists
    pub fn ensure_model_dir(&self) -> Result<PathBuf, VisionError> {
        let model_path = &self.config.model_path;
        if !model_path.exists() {
            fs::create_dir_all(model_path)
                .map_err(|e| VisionError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to create model directory: {}", e)
                )))?;
            info!("Created model directory: {:?}", model_path);
        }
        Ok(model_path.clone())
    }

    /// Download model if not present
    pub async fn ensure_model(&self, model_name: &str, url: &str, checksum: &str) -> Result<PathBuf, VisionError> {
        // Validate model name to prevent path traversal
        if model_name.is_empty() || model_name.len() > 255 {
            return Err(VisionError::Model("Invalid model name".to_string()));
        }
        
        // Prevent path traversal attacks
        if model_name.contains("..") || model_name.contains("/") || model_name.contains("\\") {
            return Err(VisionError::Model("Model name contains invalid characters".to_string()));
        }
        
        // Validate URL
        if url.is_empty() || url.len() > 2048 {
            return Err(VisionError::Model("Invalid URL".to_string()));
        }
        
        // Only allow HTTPS URLs for security
        if !url.starts_with("https://") {
            return Err(VisionError::Model("Only HTTPS URLs are allowed for model downloads".to_string()));
        }
        
        self.ensure_model_dir()?;
        
        let model_path = self.config.model_path.join(model_name);
        
        // Additional path validation - ensure model_path is within model_dir
        if !model_path.starts_with(&self.config.model_path) {
            return Err(VisionError::Model("Path traversal detected".to_string()));
        }
        
        // Check if model already exists
        if model_path.exists() {
            info!("Model {} already exists at {:?}", model_name, model_path);
            return Ok(model_path);
        }

        info!("Downloading model {} from {}", model_name, url);
        
        // Download model with size limit and timeout
        const MAX_MODEL_SIZE: usize = 2_000_000_000; // 2GB max
        const DOWNLOAD_TIMEOUT_SECS: u64 = 3600; // 1 hour max
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(DOWNLOAD_TIMEOUT_SECS))
            .build()
            .map_err(|e| VisionError::Network(e))?;
        
        let response = client.get(url).send().await
            .map_err(|e| VisionError::Network(e))?;
        
        if !response.status().is_success() {
            return Err(VisionError::Model(format!("Failed to download model: HTTP {}", response.status())));
        }
        
        // Check content length
        if let Some(content_length) = response.content_length() {
            if content_length > MAX_MODEL_SIZE as u64 {
                return Err(VisionError::Model(format!("Model too large: {} bytes (max {} bytes)", 
                    content_length, MAX_MODEL_SIZE)));
            }
        }

        let bytes = response.bytes().await
            .map_err(|e| VisionError::Network(e))?;
        
        // Validate downloaded size
        if bytes.len() > MAX_MODEL_SIZE {
            return Err(VisionError::Model(format!("Downloaded model too large: {} bytes (max {} bytes)", 
                bytes.len(), MAX_MODEL_SIZE)));
        }
        
        // Minimum size check (prevent empty/corrupted files)
        if bytes.len() < 1024 {
            return Err(VisionError::Model("Downloaded file too small, likely corrupted".to_string()));
        }

        // Verify checksum if provided
        if !checksum.is_empty() {
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let computed_hash = hex::encode(hasher.finalize());
            
            if computed_hash != checksum {
                return Err(VisionError::Model(format!(
                    "Checksum mismatch for model {}: expected {}, got {}",
                    model_name, checksum, computed_hash
                )));
            }
            info!("Verified checksum for model {}", model_name);
        } else {
            info!("Downloaded {} bytes for model {} (checksum verification skipped)", bytes.len(), model_name);
        }

        // Write to file atomically to prevent corruption
        // Write to temp file first, then rename
        let temp_path = model_path.with_extension(".tmp");
        fs::write(&temp_path, &bytes)
            .map_err(|e| VisionError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to write model file: {}", e)
            )))?;
        
        // Atomic rename
        fs::rename(&temp_path, &model_path)
            .map_err(|e| {
                // Clean up temp file on error
                let _ = fs::remove_file(&temp_path);
                VisionError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to rename model file: {}", e)
                ))
            })?;

        info!("Model {} saved to {:?}", model_name, model_path);
        Ok(model_path)
    }

    /// Get YOLO model path, downloading if needed
    pub async fn get_yolo_model(&self) -> Result<PathBuf, VisionError> {
        self.ensure_model("yolov8n.onnx", YOLO_V8_URL, YOLO_V8_CHECKSUM).await
    }

    /// Get SAM model path, downloading if needed
    pub async fn get_sam_model(&self) -> Result<PathBuf, VisionError> {
        self.ensure_model("sam_vit_b.onnx", SAM_VIT_B_URL, SAM_VIT_B_CHECKSUM).await
    }

    /// Get CLIP model path, downloading if needed
    pub async fn get_clip_model(&self) -> Result<PathBuf, VisionError> {
        self.ensure_model("clip_vit_b32.onnx", CLIP_VIT_B_32_URL, CLIP_VIT_B_32_CHECKSUM).await
    }

    /// Mark model as loaded
    pub fn mark_loaded(&self, model_name: &str) {
        self.models_loaded.write().insert(model_name.to_string(), true);
    }

    /// Check if model is loaded
    pub fn is_loaded(&self, model_name: &str) -> bool {
        self.models_loaded.read().get(model_name).copied().unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_model_manager_new() {
        let config = VisionConfig::default();
        let manager = ModelManager::new(Arc::new(config));
        // Should not panic
        assert!(true);
    }

    #[tokio::test]
    async fn test_model_manager_ensure_model_dir() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = VisionConfig::default();
        config.model_path = temp_dir.path().to_path_buf();
        
        let manager = ModelManager::new(Arc::new(config));
        let result = manager.ensure_model_dir();
        assert!(result.is_ok());
        
        // Should be idempotent
        let result2 = manager.ensure_model_dir();
        assert!(result2.is_ok());
    }

    #[tokio::test]
    async fn test_model_manager_ensure_model_invalid_name() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = VisionConfig::default();
        config.model_path = temp_dir.path().to_path_buf();
        
        let manager = ModelManager::new(Arc::new(config));
        
        // Test invalid model names
        let result = manager.ensure_model("", "https://example.com/model.onnx", "").await;
        assert!(result.is_err());
        
        let result = manager.ensure_model("../evil", "https://example.com/model.onnx", "").await;
        assert!(result.is_err());
        
        let result = manager.ensure_model("model/name", "https://example.com/model.onnx", "").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_model_manager_ensure_model_invalid_url() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = VisionConfig::default();
        config.model_path = temp_dir.path().to_path_buf();
        
        let manager = ModelManager::new(Arc::new(config));
        
        // Test invalid URLs
        let result = manager.ensure_model("model.onnx", "", "").await;
        assert!(result.is_err());
        
        let result = manager.ensure_model("model.onnx", "http://example.com/model.onnx", "").await;
        assert!(result.is_err()); // Only HTTPS allowed
        
        let result = manager.ensure_model("model.onnx", "ftp://example.com/model.onnx", "").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_model_manager_mark_loaded() {
        let config = VisionConfig::default();
        let manager = ModelManager::new(Arc::new(config));
        
        assert!(!manager.is_loaded("test_model"));
        manager.mark_loaded("test_model");
        assert!(manager.is_loaded("test_model"));
    }
}

