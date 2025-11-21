//! Utility functions for vision processing

use opencv::prelude::Mat;
use crate::error::VisionError;

/// Extract pixel data from OpenCV Mat and convert to RGB float32 tensor
pub fn mat_to_rgb_tensor(
    mat: &Mat,
    target_width: u32,
    target_height: u32,
) -> Result<Vec<f32>, VisionError> {
    // Validate input dimensions
    if target_width == 0 || target_height == 0 {
        return Err(VisionError::Processing("Target dimensions cannot be zero".to_string()));
    }
    
    // Check for potential overflow
    let total_pixels = target_width
        .checked_mul(target_height)
        .and_then(|p| p.checked_mul(3))
        .ok_or_else(|| VisionError::Processing("Target dimensions too large, would overflow".to_string()))?;
    
    if total_pixels > 100_000_000 {
        return Err(VisionError::Processing("Target dimensions too large (max 100M pixels)".to_string()));
    }
    
    let (width, height) = (mat.cols(), mat.rows());
    
    if width <= 0 || height <= 0 {
        return Err(VisionError::Processing("Invalid image dimensions".to_string()));
    }

    // Check if Mat is already float32 (normalized) or u8 (needs normalization)
    let depth = mat.depth();
    let is_float = depth == opencv::core::CV_32F || depth == opencv::core::CV_32FC3;
    
    // Get raw pixel data
    let mat_data = if is_float {
        // For float32, we need to use at_2d or similar
        // For now, use data_bytes which works for both
        mat.data_bytes()
            .map_err(|e| VisionError::OpenCv(format!("Failed to get Mat data: {}", e)))?
    } else {
        mat.data_bytes()
            .map_err(|e| VisionError::OpenCv(format!("Failed to get Mat data: {}", e)))?
    };

    // Calculate bytes per pixel
    let channels = mat.channels();
    if channels <= 0 || channels > 4 {
        return Err(VisionError::Processing(format!("Invalid channel count: {}", channels)));
    }
    
    let bytes_per_pixel = if is_float {
        channels.checked_mul(4).ok_or_else(|| {
            VisionError::Processing("Channel count * 4 would overflow".to_string())
        })? // float32 = 4 bytes
    } else {
        channels // u8 = 1 byte
    } as i32;
    
    let row_stride = width
        .checked_mul(bytes_per_pixel)
        .ok_or_else(|| VisionError::Processing("Row stride calculation overflow".to_string()))?;

    // Allocate output tensor [3, H, W] in RGB format
    let total_pixels = total_pixels as usize;
    let mut tensor_data = vec![0.0f32; total_pixels];

    // Resize and convert BGR to RGB
    for y in 0..target_height.min(height as u32) {
        for x in 0..target_width.min(width as u32) {
            // Calculate source coordinates (simple nearest neighbor)
            // Division by zero already checked above
            let src_x = (x as f32 * width as f32 / target_width as f32) as i32;
            let src_y = (y as f32 * height as f32 / target_height as f32) as i32;
            
            // Clamp to valid range to prevent out-of-bounds access
            // Ensure width and height are positive before subtraction
            let src_x = if width > 0 {
                src_x.max(0).min(width - 1)
            } else {
                0
            };
            let src_y = if height > 0 {
                src_y.max(0).min(height - 1)
            } else {
                0
            };
            
            // src_x and src_y are already clamped above
            let src_idx = (src_y as i64)
                .checked_mul(row_stride as i64)
                .and_then(|s| s.checked_add((src_x * bytes_per_pixel) as i64))
                .and_then(|idx| usize::try_from(idx).ok())
                .ok_or_else(|| VisionError::Processing("Source index calculation overflow".to_string()))?;
            
            if src_idx < mat_data.len() {
                if is_float {
                    // Float32 data (already normalized)
                    // Need 12 bytes for 3 channels * 4 bytes each
                    if src_idx + 11 < mat_data.len() {
                        // Read as f32 (4 bytes each)
                        let b = f32::from_le_bytes([
                            mat_data[src_idx],
                            mat_data[src_idx + 1],
                            mat_data[src_idx + 2],
                            mat_data[src_idx + 3],
                        ]);
                        let g = f32::from_le_bytes([
                            mat_data[src_idx + 4],
                            mat_data[src_idx + 5],
                            mat_data[src_idx + 6],
                            mat_data[src_idx + 7],
                        ]);
                        let r = f32::from_le_bytes([
                            mat_data[src_idx + 8],
                            mat_data[src_idx + 9],
                            mat_data[src_idx + 10],
                            mat_data[src_idx + 11],
                        ]);
                        
                        // Store in RGB order (validate indices)
                        let out_idx = (y as u64)
                            .checked_mul(target_width as u64)
                            .and_then(|p| p.checked_add(x as u64))
                            .and_then(|p| p.checked_mul(3))
                            .and_then(|idx| usize::try_from(idx).ok())
                            .ok_or_else(|| VisionError::Processing("Output index calculation overflow".to_string()))?;
                        
                        if out_idx + 2 < tensor_data.len() {
                            tensor_data[out_idx] = r;
                            tensor_data[out_idx + 1] = g;
                            tensor_data[out_idx + 2] = b;
                        }
                    }
                } else {
                    // U8 data (needs normalization)
                    // Need at least 3 bytes for BGR
                    let required_bytes = channels as usize;
                    if src_idx + required_bytes.saturating_sub(1) < mat_data.len() {
                        // Extract BGR values (safely)
                        let b = if src_idx < mat_data.len() { mat_data[src_idx] as f32 } else { 0.0 };
                        let g = if src_idx + 1 < mat_data.len() { mat_data[src_idx + 1] as f32 } else { 0.0 };
                        let r = if src_idx + 2 < mat_data.len() { mat_data[src_idx + 2] as f32 } else { 0.0 };
                        
                        // Normalize to [0, 1]
                        let r_norm = r / 255.0;
                        let g_norm = g / 255.0;
                        let b_norm = b / 255.0;
                        
                        // Store in RGB order (validate indices)
                        let out_idx = (y as u64)
                            .checked_mul(target_width as u64)
                            .and_then(|p| p.checked_add(x as u64))
                            .and_then(|p| p.checked_mul(3))
                            .and_then(|idx| usize::try_from(idx).ok())
                            .ok_or_else(|| VisionError::Processing("Output index calculation overflow".to_string()))?;
                        
                        if out_idx + 2 < tensor_data.len() {
                            tensor_data[out_idx] = r_norm;
                            tensor_data[out_idx + 1] = g_norm;
                            tensor_data[out_idx + 2] = b_norm;
                        }
                    }
                }
            }
        }
    }

    Ok(tensor_data)
}

/// Extract pixel data and reshape to CHW format [C, H, W]
pub fn mat_to_chw_tensor(
    mat: &Mat,
    target_width: u32,
    target_height: u32,
) -> Result<Vec<f32>, VisionError> {
    let rgb_data = mat_to_rgb_tensor(mat, target_width, target_height)?;
    
    // Reshape from [H*W*3] to [3, H, W] (CHW format)
    let mut chw_data = vec![0.0f32; rgb_data.len()];
    let h = target_height as usize;
    let w = target_width as usize;
    
    for c in 0..3 {
        for y in 0..h {
            for x in 0..w {
                let rgb_idx = (y * w + x) * 3 + c;
                let chw_idx = c * h * w + y * w + x;
                if rgb_idx < rgb_data.len() && chw_idx < chw_data.len() {
                    chw_data[chw_idx] = rgb_data[rgb_idx];
                }
            }
        }
    }
    
    Ok(chw_data)
}

/// Apply CLIP normalization (mean and std per channel)
pub fn apply_clip_normalization(data: &mut [f32]) {
    // CLIP normalization: mean=[0.485, 0.456, 0.406], std=[0.229, 0.224, 0.225]
    let mean = [0.485, 0.456, 0.406];
    let std = [0.229, 0.224, 0.225];
    
    // Assuming CHW format: [C, H, W]
    const CHANNELS: usize = 3;
    
    // Prevent division by zero
    if data.is_empty() || data.len() < CHANNELS {
        return;
    }
    
    let hw = data.len() / CHANNELS;
    if hw == 0 {
        return;
    }
    
    for c in 0..CHANNELS {
        let mean_val = mean[c];
        let std_val = std[c];
        
        // Prevent division by zero
        if std_val == 0.0 {
            continue;
        }
        
        for i in 0..hw {
            let idx = c * hw + i;
            if idx < data.len() {
                let val = data[idx];
                // Check for NaN/Inf before normalization
                if val.is_finite() {
                    data[idx] = (val - mean_val) / std_val;
                    // Ensure result is finite
                    if !data[idx].is_finite() {
                        data[idx] = 0.0;
                    }
                } else {
                    data[idx] = 0.0;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_clip_normalization_empty() {
        let mut data = vec![];
        apply_clip_normalization(&mut data);
        assert_eq!(data.len(), 0);
    }

    #[test]
    fn test_apply_clip_normalization_small() {
        let mut data = vec![0.5; 6]; // 2 pixels * 3 channels
        apply_clip_normalization(&mut data);
        assert_eq!(data.len(), 6);
        // All values should be finite after normalization
        for val in &data {
            assert!(val.is_finite());
        }
    }

    #[test]
    fn test_apply_clip_normalization_with_nan() {
        let mut data = vec![0.5, f32::NAN, 0.5, 0.5, f32::INFINITY, 0.5];
        apply_clip_normalization(&mut data);
        // NaN and Inf should be replaced with 0.0
        for val in &data {
            assert!(val.is_finite());
        }
    }

    #[test]
    fn test_apply_clip_normalization_large() {
        let mut data = vec![0.5; 3072]; // 32x32x3
        apply_clip_normalization(&mut data);
        assert_eq!(data.len(), 3072);
        for val in &data {
            assert!(val.is_finite());
        }
    }
}
