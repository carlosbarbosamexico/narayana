//! Performance and stress tests for narayana-eye

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
fn test_tracker_performance_many_detections() {
    let tracker = ObjectTracker::new(30, 0.3);
    
    // Create many detections
    let mut detections = Vec::new();
    for i in 0..500 {
        detections.push(create_detection(
            0,
            "person",
            0.9,
            (i as f32 * 10.0, 10.0, 50.0, 50.0),
        ));
    }
    
    let start = std::time::Instant::now();
    let tracks = tracker.update(&detections);
    let elapsed = start.elapsed();
    
    assert!(tracks.len() > 0);
    // Should complete in reasonable time (< 1 second)
    assert!(elapsed.as_secs() < 1);
}

#[test]
fn test_tracker_performance_many_updates() {
    let tracker = ObjectTracker::new(30, 0.3);
    
    let detections = vec![
        create_detection(0, "person", 0.9, (10.0, 10.0, 50.0, 50.0)),
    ];
    
    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let _ = tracker.update(&detections);
    }
    let elapsed = start.elapsed();
    
    // Should complete 1000 updates in reasonable time (< 1 second)
    assert!(elapsed.as_secs() < 1);
}

#[test]
fn test_tracker_memory_efficiency() {
    let tracker = ObjectTracker::new(5, 0.3);
    
    // Create tracks
    let detections = vec![
        create_detection(0, "person", 0.9, (10.0, 10.0, 50.0, 50.0)),
    ];
    let _ = tracker.update(&detections);
    
    // Age tracks beyond max_age
    for _ in 0..10 {
        let _ = tracker.update(&[]);
    }
    
    // Tracks should be cleaned up
    let tracks = tracker.get_tracks();
    assert_eq!(tracks.len(), 0);
}


