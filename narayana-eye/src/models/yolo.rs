//! YOLO object detection model

use crate::error::VisionError;
use crate::utils::mat_to_chw_tensor;
use ort::{Session, Value, Environment};
use opencv::prelude::Mat;
use opencv::imgproc;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn, debug};

/// COCO class names (80 classes)
pub const COCO_CLASSES: &[&str] = &[
    "person", "bicycle", "car", "motorcycle", "airplane", "bus", "train", "truck", "boat",
    "traffic light", "fire hydrant", "stop sign", "parking meter", "bench", "bird", "cat",
    "dog", "horse", "sheep", "cow", "elephant", "bear", "zebra", "giraffe", "backpack",
    "umbrella", "handbag", "tie", "suitcase", "frisbee", "skis", "snowboard", "sports ball",
    "kite", "baseball bat", "baseball glove", "skateboard", "surfboard", "tennis racket",
    "bottle", "wine glass", "cup", "fork", "knife", "spoon", "bowl", "banana", "apple",
    "sandwich", "orange", "broccoli", "carrot", "hot dog", "pizza", "donut", "cake", "chair",
    "couch", "potted plant", "bed", "dining table", "toilet", "tv", "laptop", "mouse",
    "remote", "keyboard", "cell phone", "microwave", "oven", "toaster", "sink", "refrigerator",
    "book", "clock", "vase", "scissors", "teddy bear", "hair drier", "toothbrush",
];

/// Detected object
#[derive(Debug, Clone)]
pub struct DetectedObject {
    pub class_id: usize,
    pub class_name: String,
    pub confidence: f32,
    pub bbox: (f32, f32, f32, f32), // x, y, width, height
}

/// YOLO model for object detection
pub struct YoloModel {
    session: Arc<Session>,
    input_size: (u32, u32),
}

impl YoloModel {
    /// Create a new YOLO model
    pub fn new(model_path: &Path) -> Result<Self, VisionError> {
        let environment = Environment::builder()
            .with_name("narayana-eye")
            .build()
            .map_err(|e| VisionError::Ort(format!("Failed to create ONNX environment: {}", e)))?;

        let session = Session::builder()
            .with_execution_providers([ort::ExecutionProvider::CPU(Default::default())])
            .commit_from_file(model_path)
            .map_err(|e| VisionError::Ort(format!("Failed to load YOLO model: {}", e)))?;

        info!("YOLO model loaded from {:?}", model_path);

        Ok(Self {
            session: Arc::new(session),
            input_size: (640, 640), // YOLO standard input size
        })
    }

    /// Detect objects in frame
    pub fn detect(&self, frame: &Mat) -> Result<Vec<DetectedObject>, VisionError> {
        debug!("Running YOLO detection on frame");
        
        // Preprocess frame
        let input = self.preprocess(frame)?;

        // Run inference
        let outputs = self.session.run(vec![input])
            .map_err(|e| VisionError::Ort(format!("YOLO inference failed: {}", e)))?;

        // Postprocess outputs
        let detections = self.postprocess(&outputs, frame)?;

        Ok(detections)
    }

    /// Preprocess frame for YOLO input
    fn preprocess(&self, frame: &Mat) -> Result<Value, VisionError> {
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

        // Convert BGR to RGB and normalize
        let mut rgb = Mat::default();
        opencv::imgproc::cvt_color(&resized, &mut rgb, opencv::imgproc::COLOR_BGR2RGB, 0)
            .map_err(|e| VisionError::OpenCv(format!("Failed to convert color: {}", e)))?;

        // Convert to float32 and normalize to [0, 1]
        let mut float_mat = Mat::default();
        rgb.convert_to(&mut float_mat, opencv::core::CV_32F, 1.0 / 255.0, 0.0)
            .map_err(|e| VisionError::OpenCv(format!("Failed to convert to float: {}", e)))?;

        // Extract pixel data and reshape to [1, 3, H, W]
        let input_shape = vec![1, 3, self.input_size.1 as i64, self.input_size.0 as i64];
        let mut input_data = mat_to_chw_tensor(&float_mat, self.input_size.0, self.input_size.1)?;
        
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
        if input_data.len() == chw_size {
            batched_data[..chw_size].copy_from_slice(&input_data);
        }
        
        let input = Value::from_array(
            ort::ndarray::Array::from_shape_vec(
                input_shape.as_slice(),
                batched_data
            ).map_err(|e| VisionError::Ort(format!("Failed to create input array: {}", e)))?
        ).map_err(|e| VisionError::Ort(format!("Failed to create input value: {}", e)))?;

        Ok(input)
    }

    /// Postprocess YOLO outputs
    fn postprocess(&self, outputs: &[Value], original_frame: &Mat) -> Result<Vec<DetectedObject>, VisionError> {
        if outputs.is_empty() {
            return Ok(vec![]);
        }

        let output = &outputs[0];
        let output_array = output.try_extract_tensor::<f32>()
            .map_err(|e| VisionError::Ort(format!("Failed to extract output tensor: {}", e)))?;

        let shape = output_array.shape();
        debug!("YOLO output shape: {:?}", shape);

        // YOLO output format: [batch, num_detections, 85] where 85 = [x, y, w, h, conf, class_probs...]
        // Simplified postprocessing - extract detections
        let mut detections = Vec::new();
        let confidence_threshold = 0.5;
        let nms_threshold = 0.4;

        if shape.len() >= 2 {
            let num_detections = shape[1];
            let num_classes = COCO_CLASSES.len();

            // Validate num_detections is reasonable
            let max_detections = num_detections.min(100).min(shape[0] as i64);
            if max_detections <= 0 {
                return Ok(vec![]);
            }
            
            for i in 0..max_detections {
                let i_usize = i as usize;
                // Validate index is within bounds
                if i_usize >= num_detections as usize {
                    break;
                }
                
                // Extract confidence and class
                // Simplified: assume output format matches YOLO standard
                let conf_idx = 4; // confidence is at index 4
                if let Some(conf) = output_array.get([0, i_usize, conf_idx]) {
                    if *conf > confidence_threshold {
                        // Find class with highest probability
                        let mut max_class = 0;
                        let mut max_prob = 0.0f32;
                        
                        for class_idx in 0..num_classes.min(80) {
                            let prob_idx = 5 + class_idx; // class probs start at index 5
                            // Validate prob_idx is within expected range
                            if prob_idx >= 85 {
                                break; // YOLO output format is [x, y, w, h, conf, 80 class_probs] = 85 total
                            }
                            if let Some(prob) = output_array.get([0, i_usize, prob_idx]) {
                                if *prob > max_prob {
                                    max_prob = *prob;
                                    max_class = class_idx;
                                }
                            }
                        }

                        if max_prob > confidence_threshold && max_class < COCO_CLASSES.len() {
                            // Extract bounding box (normalized coordinates)
                            // Validate indices are within bounds
                            let x = output_array.get([0, i_usize, 0]).copied().unwrap_or(0.0);
                            let y = output_array.get([0, i_usize, 1]).copied().unwrap_or(0.0);
                            let w = output_array.get([0, i_usize, 2]).copied().unwrap_or(0.0);
                            let h = output_array.get([0, i_usize, 3]).copied().unwrap_or(0.0);

                            // Validate bbox values (should be normalized [0, 1])
                            if !x.is_finite() || !y.is_finite() || !w.is_finite() || !h.is_finite() {
                                continue; // Skip invalid detections
                            }
                            
                            if x < 0.0 || x > 1.0 || y < 0.0 || y > 1.0 || w < 0.0 || w > 1.0 || h < 0.0 || h > 1.0 {
                                continue; // Skip out-of-range detections
                            }

                            // Convert to pixel coordinates
                            let frame_width = original_frame.cols() as f32;
                            let frame_height = original_frame.rows() as f32;
                            
                            // Validate frame dimensions
                            if frame_width <= 0.0 || frame_height <= 0.0 {
                                continue;
                            }
                            
                            let bbox_x = (x * frame_width).max(0.0);
                            let bbox_y = (y * frame_height).max(0.0);
                            let bbox_w = (w * frame_width).min(frame_width - bbox_x);
                            let bbox_h = (h * frame_height).min(frame_height - bbox_y);
                            
                            // Ensure bbox is valid
                            if bbox_w <= 0.0 || bbox_h <= 0.0 {
                                continue;
                            }

                            detections.push(DetectedObject {
                                class_id: max_class,
                                class_name: COCO_CLASSES[max_class].to_string(),
                                confidence: max_prob,
                                bbox: (bbox_x, bbox_y, bbox_w, bbox_h),
                            });
                        }
                    }
                }
            }
        }

        // Apply Non-Maximum Suppression (simplified)
        detections = self.apply_nms(detections, nms_threshold);

        debug!("YOLO detected {} objects", detections.len());
        Ok(detections)
    }

    /// Apply Non-Maximum Suppression
    fn apply_nms(&self, mut detections: Vec<DetectedObject>, iou_threshold: f32) -> Vec<DetectedObject> {
        if detections.is_empty() {
            return detections;
        }

        // Sort by confidence (descending)
        // Validate all confidences are finite before sorting
        detections.retain(|d| d.confidence.is_finite() && d.confidence >= 0.0 && d.confidence <= 1.0);
        detections.sort_by(|a, b| {
            b.confidence.partial_cmp(&a.confidence)
                .unwrap_or_else(|| {
                    // If comparison fails (NaN), put NaN last
                    if a.confidence.is_nan() {
                        std::cmp::Ordering::Greater
                    } else if b.confidence.is_nan() {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Equal
                    }
                })
        });

        let mut keep = Vec::new();
        let mut suppressed = vec![false; detections.len()];

        for i in 0..detections.len() {
            if suppressed[i] {
                continue;
            }

            keep.push(detections[i].clone());

            for j in (i + 1)..detections.len() {
                if suppressed[j] {
                    continue;
                }

                let iou = self.compute_iou(&detections[i].bbox, &detections[j].bbox);
                if iou > iou_threshold {
                    suppressed[j] = true;
                }
            }
        }

        keep
    }

    /// Compute IoU between two bounding boxes
    fn compute_iou(&self, bbox1: &(f32, f32, f32, f32), bbox2: &(f32, f32, f32, f32)) -> f32 {
        let (x1, y1, w1, h1) = bbox1;
        let (x2, y2, w2, h2) = bbox2;

        // Validate inputs are finite and non-negative
        if !x1.is_finite() || !y1.is_finite() || !w1.is_finite() || !h1.is_finite() ||
           !x2.is_finite() || !y2.is_finite() || !w2.is_finite() || !h2.is_finite() {
            return 0.0;
        }
        
        if *w1 < 0.0 || *h1 < 0.0 || *w2 < 0.0 || *h2 < 0.0 {
            return 0.0;
        }

        let x1_min = *x1;
        let y1_min = *y1;
        let x1_max = x1 + w1;
        let y1_max = y1 + h1;

        let x2_min = *x2;
        let y2_min = *y2;
        let x2_max = x2 + w2;
        let y2_max = y2 + h2;

        let inter_x_min = x1_min.max(x2_min);
        let inter_y_min = y1_min.max(y2_min);
        let inter_x_max = x1_max.min(x2_max);
        let inter_y_max = y1_max.min(y2_max);

        if inter_x_max <= inter_x_min || inter_y_max <= inter_y_min {
            return 0.0;
        }

        let inter_area = (inter_x_max - inter_x_min) * (inter_y_max - inter_y_min);
        let area1 = w1 * h1;
        let area2 = w2 * h2;
        let union_area = area1 + area2 - inter_area;

        if union_area <= 0.0 || !union_area.is_finite() {
            return 0.0;
        }

        let iou = inter_area / union_area;
        if iou.is_finite() && iou >= 0.0 && iou <= 1.0 {
            iou
        } else {
            0.0
        }
    }
}
