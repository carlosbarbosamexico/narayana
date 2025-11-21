//! Security-focused tests for narayana-eye

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
fn test_config_validation_overflow_protection() {
    let mut config = VisionConfig::default();
    
    // Test integer overflow protection
    config.resolution = (u32::MAX, 2);
    assert!(config.validate().is_err());
    
    config.resolution = (2, u32::MAX);
    assert!(config.validate().is_err());
}

#[test]
fn test_config_validation_division_by_zero_protection() {
    let mut config = VisionConfig::default();
    
    // Test division by zero protection
    config.frame_rate = 0;
    assert!(config.validate().is_err());
    
    config.frame_rate = 30;
    config.resolution = (0, 480);
    assert!(config.validate().is_err());
}

#[test]
fn test_tracker_iou_nan_protection() {
    let tracker = ObjectTracker::new(30, 0.3);
    
    // Test NaN handling
    let bbox1 = (f32::NAN, 10.0, 50.0, 50.0);
    let bbox2 = (10.0, 10.0, 50.0, 50.0);
    let iou = tracker.compute_iou(&bbox1, &bbox2);
    assert_eq!(iou, 0.0);
    assert!(iou.is_finite());
}

#[test]
fn test_tracker_iou_inf_protection() {
    let tracker = ObjectTracker::new(30, 0.3);
    
    // Test Infinity handling
    let bbox1 = (f32::INFINITY, 10.0, 50.0, 50.0);
    let bbox2 = (10.0, 10.0, 50.0, 50.0);
    let iou = tracker.compute_iou(&bbox1, &bbox2);
    assert_eq!(iou, 0.0);
    assert!(iou.is_finite());
}

#[test]
fn test_tracker_iou_negative_dimensions() {
    let tracker = ObjectTracker::new(30, 0.3);
    
    // Test negative dimension handling
    let bbox1 = (10.0, 10.0, -50.0, 50.0);
    let bbox2 = (10.0, 10.0, 50.0, 50.0);
    let iou = tracker.compute_iou(&bbox1, &bbox2);
    assert_eq!(iou, 0.0);
}

#[test]
fn test_tracker_memory_limit() {
    let tracker = ObjectTracker::new(30, 0.3);
    
    // Create many detections to test memory limit
    let mut detections = Vec::new();
    for i in 0..2000 {
        detections.push(create_detection(
            0,
            "person",
            0.9,
            (i as f32 * 500.0, 10.0, 50.0, 50.0),
        ));
    }
    
    let tracks = tracker.update(&detections);
    // Should be limited to MAX_TRACKS (1000)
    assert!(tracks.len() <= 1000);
}

#[test]
fn test_tracker_id_overflow_protection() {
    let tracker = ObjectTracker::new(30, 0.3);
    
    // Create many detections to test ID overflow handling
    let mut detections = Vec::new();
    for i in 0..100 {
        detections.push(create_detection(
            0,
            "person",
            0.9,
            (i as f32 * 500.0, 10.0, 50.0, 50.0),
        ));
    }
    
    let tracks = tracker.update(&detections);
    // All IDs should be valid (not 0, finite)
    for track in &tracks {
        assert_ne!(track.id, 0);
    }
}

#[test]
fn test_tracker_confidence_validation() {
    let tracker = ObjectTracker::new(30, 0.3);
    
    // Test with invalid confidence values
    let detections = vec![
        create_detection(0, "person", f32::NAN, (10.0, 10.0, 50.0, 50.0)),
        create_detection(0, "person", f32::INFINITY, (100.0, 10.0, 50.0, 50.0)),
        create_detection(0, "person", -1.0, (200.0, 10.0, 50.0, 50.0)),
        create_detection(0, "person", 2.0, (300.0, 10.0, 50.0, 50.0)),
    ];
    
    let tracks = tracker.update(&detections);
    // Should handle invalid confidences gracefully
    for track in &tracks {
        assert!(track.object.confidence.is_finite());
    }
}

#[test]
fn test_tracker_bbox_validation() {
    let tracker = ObjectTracker::new(30, 0.3);
    
    // Test with invalid bbox values
    let detections = vec![
        create_detection(0, "person", 0.9, (f32::NAN, 10.0, 50.0, 50.0)),
        create_detection(0, "person", 0.9, (10.0, f32::INFINITY, 50.0, 50.0)),
        create_detection(0, "person", 0.9, (10.0, 10.0, -50.0, 50.0)),
    ];
    
    let tracks = tracker.update(&detections);
    // Should handle invalid bboxes gracefully
    for track in &tracks {
        let (x, y, w, h) = track.object.bbox;
        assert!(x.is_finite());
        assert!(y.is_finite());
        assert!(w.is_finite() && w >= 0.0);
        assert!(h.is_finite() && h >= 0.0);
    }
}


