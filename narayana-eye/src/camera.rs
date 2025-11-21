//! USB webcam capture and management

use crate::error::VisionError;
use crate::config::VisionConfig;
use opencv::{
    prelude::*,
    videoio::{VideoCapture, CAP_ANY, CAP_PROP_FRAME_WIDTH, CAP_PROP_FRAME_HEIGHT, CAP_PROP_FPS},
    core::Mat,
};
use tokio::sync::mpsc;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{info, warn, error};

/// Camera manager for USB webcam capture
pub struct CameraManager {
    config: Arc<VisionConfig>,
    capture: Arc<RwLock<Option<VideoCapture>>>,
    is_running: Arc<RwLock<bool>>,
}

impl CameraManager {
    /// Create a new camera manager
    pub fn new(config: Arc<VisionConfig>) -> Self {
        Self {
            config,
            capture: Arc::new(RwLock::new(None)),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Initialize camera
    pub fn initialize(&self) -> Result<(), VisionError> {
        // Check if already initialized and running
        {
            let capture_guard = self.capture.read();
            if capture_guard.is_some() && *self.is_running.read() {
                // Already initialized and running, skip
                return Ok(());
            }
        }

        // Close existing capture if any (cleanup before reinitializing)
        {
            let mut capture_guard = self.capture.write();
            if capture_guard.is_some() {
                *capture_guard = None;
            }
        }

        let mut capture = VideoCapture::new(self.config.camera_id as i32, CAP_ANY)
            .map_err(|e| VisionError::Camera(format!("Failed to open camera {}: {}", self.config.camera_id, e)))?;

        if !capture.is_opened()
            .map_err(|e| VisionError::Camera(format!("Camera {} not opened: {}", self.config.camera_id, e)))? {
            return Err(VisionError::Camera(format!("Camera {} failed to open", self.config.camera_id)));
        }

        // Set resolution (with validation)
        let width = self.config.resolution.0 as f64;
        let height = self.config.resolution.1 as f64;
        let fps = self.config.frame_rate as f64;

        if width <= 0.0 || height <= 0.0 || fps <= 0.0 {
            return Err(VisionError::Camera("Invalid camera resolution or frame rate".to_string()));
        }

        capture.set(CAP_PROP_FRAME_WIDTH, width)
            .map_err(|e| VisionError::Camera(format!("Failed to set width: {}", e)))?;
        capture.set(CAP_PROP_FRAME_HEIGHT, height)
            .map_err(|e| VisionError::Camera(format!("Failed to set height: {}", e)))?;
        capture.set(CAP_PROP_FPS, fps)
            .map_err(|e| VisionError::Camera(format!("Failed to set FPS: {}", e)))?;

        *self.capture.write() = Some(capture);
        info!("Camera {} initialized at {}x{} @ {}fps", 
            self.config.camera_id, 
            self.config.resolution.0, 
            self.config.resolution.1,
            self.config.frame_rate);

        Ok(())
    }

    /// Start frame capture stream
    pub fn start_stream(&self) -> Result<mpsc::Receiver<Mat>, VisionError> {
        // Use atomic check-and-set to prevent race conditions
        {
            let mut is_running = self.is_running.write();
            if *is_running {
                return Err(VisionError::Camera("Camera stream already running".to_string()));
            }
            *is_running = true;
        }

        // Re-initialize camera if needed (outside of write lock to prevent deadlock)
        {
            let capture_guard = self.capture.read();
            if capture_guard.is_none() {
                drop(capture_guard); // Release read lock before calling initialize
                self.initialize()?;
            }
        }

        // Use larger buffer to prevent blocking, but still bounded to prevent memory exhaustion
        const FRAME_BUFFER_SIZE: usize = 30; // ~1 second at 30fps
        let (tx, rx) = mpsc::channel(FRAME_BUFFER_SIZE);
        let config = self.config.clone();
        let capture = self.capture.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            // Prevent division by zero
            let frame_rate = if config.frame_rate == 0 { 1 } else { config.frame_rate };
            let frame_interval = std::time::Duration::from_secs_f64(1.0 / frame_rate as f64);

            loop {
                if !*is_running.read() {
                    break;
                }

                let start = std::time::Instant::now();

                let frame_result = {
                    let capture_guard = capture.read();
                    if let Some(ref cap) = *capture_guard {
                        let mut frame = Mat::default();
                        cap.read(&mut frame).map(|_| frame)
                    } else {
                        Err(opencv::Error::new(0, "Camera not available".to_string()))
                    }
                };

                match frame_result {
                    Ok(frame) => {
                        if tx.send(frame).await.is_err() {
                            warn!("Frame receiver dropped, stopping camera stream");
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Camera read error: {}", e);
                        // Use exponential backoff with max retries
                        // Store retry count in a thread-safe way
                        use std::sync::atomic::{AtomicU32, Ordering};
                        static RETRY_COUNT: AtomicU32 = AtomicU32::new(0);
                        
                        let retries = RETRY_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
                        if retries > 10 {
                            error!("Too many camera read errors ({}), stopping stream", retries);
                            break;
                        }
                        
                        // Exponential backoff: 100ms, 200ms, 400ms, etc., max 5s
                        let backoff_ms = (100 * (1 << retries.min(5))).min(5000);
                        tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                        
                        // Try to reinitialize camera if it's not available
                        let capture_guard = capture.read();
                        if capture_guard.is_none() {
                            drop(capture_guard);
                            // Create a new camera manager instance for reinitialization
                            // This avoids deadlock from trying to get write lock while holding read lock
                            if let Err(init_err) = CameraManager::new(config.clone()).initialize() {
                                error!("Failed to reinitialize camera: {}", init_err);
                            } else {
                                // Update the capture in the shared state
                                let mut capture_write = capture.write();
                                if capture_write.is_none() {
                                    if let Ok(new_capture) = VideoCapture::new(config.camera_id as i32, CAP_ANY) {
                                        if new_capture.is_opened().unwrap_or(false) {
                                            *capture_write = Some(new_capture);
                                            RETRY_COUNT.store(0, Ordering::Relaxed);
                                        }
                                    }
                                }
                            }
                        } else {
                            // Camera is available, reset retry count
                            RETRY_COUNT.store(0, Ordering::Relaxed);
                        }
                    }
                }

                let elapsed = start.elapsed();
                if elapsed < frame_interval {
                    tokio::time::sleep(frame_interval - elapsed).await;
                }
            }

            {
                let mut is_running = self.is_running.write();
                *is_running = false;
            }
            info!("Camera stream stopped");
        });

        info!("Camera stream started");
        Ok(rx)
    }

    /// Capture a single frame
    pub fn capture_frame(&self) -> Result<Mat, VisionError> {
        let capture_guard = self.capture.read();
        let capture = capture_guard.as_ref()
            .ok_or_else(|| VisionError::Camera("Camera not initialized".to_string()))?;

        let mut frame = Mat::default();
        capture.read(&mut frame)
            .map_err(|e| VisionError::Camera(format!("Failed to read frame: {}", e)))?;

        Ok(frame)
    }

    /// Stop camera stream
    pub fn stop(&self) {
        *self.is_running.write() = false;
        *self.capture.write() = None;
        info!("Camera stopped");
    }

    /// Check if camera is running
    pub fn is_running(&self) -> bool {
        *self.is_running.read()
    }
}

impl Drop for CameraManager {
    fn drop(&mut self) {
        self.stop();
    }
}

