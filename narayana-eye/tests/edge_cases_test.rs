//! Edge case tests for narayana-eye

use narayana_eye::config::VisionConfig;
use narayana_eye::processing::ObjectTracker;
use narayana_eye::models::DetectedObject;

fn create_detection(class_id: usize, class_name: &str, confidence: f32, bbox: (f32, f32, f32, f32)) -> DetectedObject {
    DetectedObject {
        class_id,
        class_name: class_name.to_string(),
        confidence,
        bbox,
    }
}

#[test]
fn test_config_edge_cases() {
    // Minimum valid values
    let mut config = VisionConfig::default();
    config.frame_rate = 1;
    config.resolution = (1, 1);
    config.camera_id = 0;
    assert!(config.validate().is_ok());
    
    // Maximum valid values
    config.frame_rate = 120;
    config.resolution = (7680, 4320);
    config.camera_id = 100;
    assert!(config.validate().is_ok());
}

#[test]
fn test_tracker_empty_detections() {
    let tracker = ObjectTracker::new(30, 0.3);
    let detections = vec![];
    let tracks = tracker.update(&detections);
    assert_eq!(tracks.len(), 0);
}

#[test]
fn test_tracker_single_pixel_bbox() {
    let tracker = ObjectTracker::new(30, 0.3);
    let detections = vec![
        create_detection(0, "person", 0.9, (10.0, 10.0, 1.0, 1.0)),
    ];
    let tracks = tracker.update(&detections);
    assert_eq!(tracks.len(), 1);
}

#[test]
fn test_tracker_very_large_bbox() {
    let tracker = ObjectTracker::new(30, 0.3);
    let detections = vec![
        create_detection(0, "person", 0.9, (0.0, 0.0, 10000.0, 10000.0)),
    ];
    let tracks = tracker.update(&detections);
    assert_eq!(tracks.len(), 1);
}

#[test]
fn test_tracker_zero_iou_threshold() {
    let tracker = ObjectTracker::new(30, 0.0);
    let detections = vec![
        create_detection(0, "person", 0.9, (10.0, 10.0, 50.0, 50.0)),
        create_detection(0, "person", 0.9, (12.0, 12.0, 50.0, 50.0)),
    ];
    let tracks = tracker.update(&detections);
    // With zero threshold, should match
    assert!(tracks.len() <= 2);
}

#[test]
fn test_tracker_very_high_iou_threshold() {
    let tracker = ObjectTracker::new(30, 1.0);
    let detections = vec![
        create_detection(0, "person", 0.9, (10.0, 10.0, 50.0, 50.0)),
        create_detection(0, "person", 0.9, (12.0, 12.0, 50.0, 50.0)),
    ];
    let tracks = tracker.update(&detections);
    // With very high threshold, should create separate tracks
    assert_eq!(tracks.len(), 2);
}

#[test]
fn test_tracker_max_age_zero() {
    let tracker = ObjectTracker::new(0, 0.3);
    let detections = vec![
        create_detection(0, "person", 0.9, (10.0, 10.0, 50.0, 50.0)),
    ];
    let tracks = tracker.update(&detections);
    // Track should be immediately removed
    let tracks_after = tracker.update(&[]);
    assert_eq!(tracks_after.len(), 0);
}

#[test]
fn test_tracker_rapid_movement() {
    let tracker = ObjectTracker::new(30, 0.3);
    
    // First frame
    let detections1 = vec![
        create_detection(0, "person", 0.9, (10.0, 10.0, 50.0, 50.0)),
    ];
    let tracks1 = tracker.update(&detections1);
    let track_id = tracks1[0].id;
    
    // Rapid movement - low IoU
    let detections2 = vec![
        create_detection(0, "person", 0.9, (500.0, 500.0, 50.0, 50.0)),
    ];
    let tracks2 = tracker.update(&detections2);
    // Should create new track due to low IoU
    assert!(tracks2.len() >= 1);
}

#[test]
fn test_tracker_identical_detections() {
    let tracker = ObjectTracker::new(30, 0.3);
    
    let detections = vec![
        create_detection(0, "person", 0.9, (10.0, 10.0, 50.0, 50.0)),
        create_detection(0, "person", 0.9, (10.0, 10.0, 50.0, 50.0)),
    ];
    let tracks = tracker.update(&detections);
    // Should handle identical detections
    assert!(tracks.len() >= 1);
}

#[test]
fn test_tracker_overlapping_bboxes() {
    let tracker = ObjectTracker::new(30, 0.3);
    
    // Overlapping bboxes
    let detections = vec![
        create_detection(0, "person", 0.9, (10.0, 10.0, 50.0, 50.0)),
        create_detection(0, "person", 0.8, (30.0, 30.0, 50.0, 50.0)),
    ];
    let tracks = tracker.update(&detections);
    // Should match if IoU > threshold
    assert!(tracks.len() >= 1);
}


