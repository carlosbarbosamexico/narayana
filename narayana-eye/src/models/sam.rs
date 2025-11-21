//! SAM (Segment Anything Model) for instance segmentation

use crate::error::VisionError;
use crate::utils::mat_to_chw_tensor;
use ort::{Session, Value, Environment};
use opencv::prelude::Mat;
use opencv::imgproc;
use std::path::Path;
use std::sync::Arc;
use tracing::info;

/// Segmentation mask
#[derive(Debug, Clone)]
pub struct SegmentationMask {
    pub mask: Vec<Vec<bool>>, // 2D boolean mask
    pub bbox: (f32, f32, f32, f32), // x, y, width, height
    pub confidence: f32,
}

/// SAM model for instance segmentation
pub struct SamModel {
    session: Arc<Session>,
    input_size: (u32, u32),
}

impl SamModel {
    /// Create a new SAM model
    pub fn new(model_path: &Path) -> Result<Self, VisionError> {
        let environment = Environment::builder()
            .with_name("narayana-eye")
            .build()
            .map_err(|e| VisionError::Ort(format!("Failed to create ONNX environment: {}", e)))?;

        let session = Session::builder()
            .with_execution_providers([ort::ExecutionProvider::CPU(Default::default())])
            .commit_from_file(model_path)
            .map_err(|e| VisionError::Ort(format!("Failed to load SAM model: {}", e)))?;

        info!("SAM model loaded from {:?}", model_path);

        Ok(Self {
            session: Arc::new(session),
            input_size: (1024, 1024), // SAM standard input size
        })
    }

    /// Segment objects in frame
    pub fn segment(&self, frame: &Mat, prompts: &[(f32, f32)]) -> Result<Vec<SegmentationMask>, VisionError> {
        if prompts.is_empty() {
            return Ok(vec![]);
        }

        // Preprocess frame and prompts
        let inputs = self.preprocess(frame, prompts)?;

        // Run inference
        let outputs = self.session.run(inputs)
            .map_err(|e| VisionError::Ort(format!("SAM inference failed: {}", e)))?;

        // Postprocess outputs
        let masks = self.postprocess(&outputs, frame, prompts)?;

        Ok(masks)
    }

    /// Preprocess frame and prompts for SAM input
    fn preprocess(&self, frame: &Mat, prompts: &[(f32, f32)]) -> Result<Vec<Value>, VisionError> {
        let (width, height) = (frame.cols(), frame.rows());
        
        // Resize frame to model input size
        let mut resized = Mat::default();
        imgproc::resize(
            frame,
            &mut resized,
            opencv::core::Size::new(self.input_size.0 as i32, self.input_size.1 as i32),
            0.0,
            0.0,
            imgproc::INTER_LINEAR,
        ).map_err(|e| VisionError::OpenCv(format!("Failed to resize frame: {}", e)))?;

        // Convert BGR to RGB
        let mut rgb = Mat::default();
        opencv::imgproc::cvt_color(&resized, &mut rgb, opencv::imgproc::COLOR_BGR2RGB, 0)
            .map_err(|e| VisionError::OpenCv(format!("Failed to convert color: {}", e)))?;

        // Normalize to [0, 1]
        let mut float_mat = Mat::default();
        rgb.convert_to(&mut float_mat, opencv::core::CV_32F, 1.0 / 255.0, 0.0)
            .map_err(|e| VisionError::OpenCv(format!("Failed to convert to float: {}", e)))?;

        // Create image input tensor [1, 3, H, W]
        let input_shape = vec![1, 3, self.input_size.1 as i64, self.input_size.0 as i64];
        let mut image_data = mat_to_chw_tensor(&float_mat, self.input_size.0, self.input_size.1)?;
        
        // Add batch dimension: [3, H, W] -> [1, 3, H, W]
        // Prevent integer overflow
        let total_size = input_shape.iter()
            .try_fold(1i64, |acc, &dim| acc.checked_mul(dim))
            .ok_or_else(|| VisionError::Ort("Input shape would overflow".to_string()))?;
        
        if total_size > 100_000_000 {
            return Err(VisionError::Ort("Input tensor too large (max 100M elements)".to_string()));
        }
        
        let mut batched_data = vec![0.0f32; total_size as usize];
        let chw_size = input_shape[1]
            .checked_mul(input_shape[2])
            .and_then(|p| p.checked_mul(input_shape[3]))
            .ok_or_else(|| VisionError::Ort("CHW size calculation overflow".to_string()))? as usize;
        if image_data.len() == chw_size {
            batched_data[..chw_size].copy_from_slice(&image_data);
        }
        
        let image_input = Value::from_array(
            ort::ndarray::Array::from_shape_vec(
                input_shape.as_slice(),
                batched_data
            ).map_err(|e| VisionError::Ort(format!("Failed to create image array: {}", e)))?
        ).map_err(|e| VisionError::Ort(format!("Failed to create image value: {}", e)))?;

        // Create point coordinates input [1, num_points, 2]
        // Limit number of points to prevent excessive processing
        const MAX_PROMPTS: usize = 100;
        let num_points = prompts.len().min(MAX_PROMPTS);
        if num_points == 0 {
            return Err(VisionError::Processing("No prompts provided".to_string()));
        }
        
        // Prevent division by zero
        if width <= 0 || height <= 0 {
            return Err(VisionError::Processing("Invalid frame dimensions for SAM preprocessing".to_string()));
        }
        
        let mut point_data = Vec::new();
        for (x, y) in prompts.iter().take(num_points) {
            // Validate input coordinates are finite
            if !x.is_finite() || !y.is_finite() {
                continue; // Skip invalid prompts
            }
            
            // Normalize coordinates to [0, 1] based on original frame size
            let norm_x = (*x / width as f32).clamp(0.0, 1.0);
            let norm_y = (*y / height as f32).clamp(0.0, 1.0);
            point_data.push(norm_x);
            point_data.push(norm_y);
        }
        
        // Ensure we have at least one valid point
        if point_data.is_empty() {
            return Err(VisionError::Processing("No valid prompts provided".to_string()));
        }
        
        // Update num_points to match actual valid points
        let num_points = point_data.len() / 2;
        if num_points == 0 {
            return Err(VisionError::Processing("No valid point data".to_string()));
        }
        
        let point_shape = vec![1, num_points as i64, 2];

        let point_input = Value::from_array(
            ort::ndarray::Array::from_shape_vec(
                point_shape.as_slice(),
                point_data
            ).map_err(|e| VisionError::Ort(format!("Failed to create point array: {}", e)))?
        ).map_err(|e| VisionError::Ort(format!("Failed to create point value: {}", e)))?;

        // Create point labels (all positive prompts = 1)
        let label_shape = vec![1, num_points as i64];
        let label_data = vec![1.0f32; num_points];
        
        let label_input = Value::from_array(
            ort::ndarray::Array::from_shape_vec(
                label_shape.as_slice(),
                label_data
            ).map_err(|e| VisionError::Ort(format!("Failed to create label array: {}", e)))?
        ).map_err(|e| VisionError::Ort(format!("Failed to create label value: {}", e)))?;

        Ok(vec![image_input, point_input, label_input])
    }

    /// Postprocess SAM outputs
    fn postprocess(&self, outputs: &[Value], original_frame: &Mat, prompts: &[(f32, f32)]) -> Result<Vec<SegmentationMask>, VisionError> {
        if outputs.is_empty() {
            return Ok(vec![]);
        }

        let mask_output = &outputs[0];
        let mask_array = mask_output.try_extract_tensor::<f32>()
            .map_err(|e| VisionError::Ort(format!("Failed to extract mask tensor: {}", e)))?;

        let shape = mask_array.shape();
        let mut masks = Vec::new();

        // SAM output format: [batch, num_masks, H, W]
        if shape.len() >= 4 {
            let num_masks = shape[1];
            let mask_height = shape[2];
            let mask_width = shape[3];

            // Limit number of masks to prevent memory exhaustion
            const MAX_MASKS: usize = 100;
            let max_masks = num_masks.min(prompts.len() as i64).min(MAX_MASKS as i64);
            
            // Validate mask dimensions
            if mask_height <= 0 || mask_width <= 0 {
                return Err(VisionError::Processing("Invalid mask dimensions".to_string()));
            }
            
            // Prevent excessive memory allocation
            let max_mask_size = 10_000_000; // 10M pixels max
            if (mask_height as usize) * (mask_width as usize) > max_mask_size {
                return Err(VisionError::Processing("Mask too large".to_string()));
            }
            
            for mask_idx in 0..max_masks {
                let mask_idx_usize = mask_idx as usize;
                if mask_idx_usize >= num_masks as usize {
                    break;
                }
                
                // Extract mask data
                let mut mask_data = Vec::new();
                for y in 0..mask_height {
                    let y_usize = y as usize;
                    if y_usize >= mask_height as usize {
                        break;
                    }
                    let mut row = Vec::new();
                    for x in 0..mask_width {
                        let x_usize = x as usize;
                        if x_usize >= mask_width as usize {
                            break;
                        }
                        if let Some(&val) = mask_array.get([0, mask_idx_usize, y_usize, x_usize]) {
                            row.push(val > 0.5 && val.is_finite()); // Threshold at 0.5, ensure finite
                        } else {
                            row.push(false);
                        }
                    }
                    mask_data.push(row);
                }

                // Calculate bounding box from mask
                let mut min_x = mask_width as f32;
                let mut min_y = mask_height as f32;
                let mut max_x = 0.0f32;
                let mut max_y = 0.0f32;

                for (y, row) in mask_data.iter().enumerate() {
                    for (x, &val) in row.iter().enumerate() {
                        if val {
                            min_x = min_x.min(x as f32);
                            min_y = min_y.min(y as f32);
                            max_x = max_x.max(x as f32);
                            max_y = max_y.max(y as f32);
                        }
                    }
                }

                // Scale bbox to original frame size
                // Prevent division by zero
                if mask_width == 0 || mask_height == 0 {
                    continue;
                }
                
                let frame_width = original_frame.cols() as f32;
                let frame_height = original_frame.rows() as f32;
                
                if frame_width <= 0.0 || frame_height <= 0.0 {
                    continue;
                }
                
                let scale_x = frame_width / mask_width as f32;
                let scale_y = frame_height / mask_height as f32;
                
                if !scale_x.is_finite() || !scale_y.is_finite() {
                    continue;
                }

                // Validate bbox dimensions
                let bbox_w = (max_x - min_x) * scale_x;
                let bbox_h = (max_y - min_y) * scale_y;
                
                if bbox_w <= 0.0 || bbox_h <= 0.0 || !bbox_w.is_finite() || !bbox_h.is_finite() {
                    continue;
                }
                
                let bbox_x = (min_x * scale_x).max(0.0).min(frame_width);
                let bbox_y = (min_y * scale_y).max(0.0).min(frame_height);
                let bbox_w = bbox_w.min(frame_width - bbox_x);
                let bbox_h = bbox_h.min(frame_height - bbox_y);
                
                let bbox = (bbox_x, bbox_y, bbox_w, bbox_h);

                masks.push(SegmentationMask {
                    mask: mask_data,
                    bbox,
                    confidence: 0.9, // SAM doesn't provide confidence, use default
                });
            }
        }

        Ok(masks)
    }
}
