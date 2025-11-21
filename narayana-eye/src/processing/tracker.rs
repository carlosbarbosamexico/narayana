//! Multi-object tracking

use crate::models::DetectedObject;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{debug, warn};

/// Tracked object with ID
#[derive(Debug, Clone)]
pub struct TrackedObject {
    pub id: u64,
    pub object: DetectedObject,
    pub age: u32, // Frames since first detection
}

/// Multi-object tracker
pub struct ObjectTracker {
    next_id: Arc<RwLock<u64>>,
    tracks: Arc<RwLock<HashMap<u64, TrackedObject>>>,
    max_age: u32,
    iou_threshold: f32,
}

impl ObjectTracker {
    /// Create a new object tracker
    pub fn new(max_age: u32, iou_threshold: f32) -> Self {
        Self {
            next_id: Arc::new(RwLock::new(1)),
            tracks: Arc::new(RwLock::new(HashMap::new())),
            max_age,
            iou_threshold,
        }
    }

    /// Update tracker with new detections
    pub fn update(&self, detections: &[DetectedObject]) -> Vec<TrackedObject> {
        let mut tracks = self.tracks.write();
        let mut next_id = self.next_id.write();

        // Age existing tracks
        for track in tracks.values_mut() {
            track.age += 1;
        }

        // Match detections to existing tracks using IoU
        let mut matched = vec![false; detections.len()];
        let mut track_ids: Vec<Option<u64>> = vec![None; detections.len()];

        for (det_idx, detection) in detections.iter().enumerate() {
            let mut best_match: Option<(u64, f32)> = None;

            for (track_id, track) in tracks.iter() {
                if track.age > self.max_age {
                    continue;
                }

                let iou = self.compute_iou(&detection.bbox, &track.object.bbox);
                if iou > self.iou_threshold {
                    if let Some((_, best_iou)) = best_match {
                        if iou > best_iou {
                            best_match = Some((*track_id, iou));
                        }
                    } else {
                        best_match = Some((*track_id, iou));
                    }
                }
            }

            if let Some((track_id, _)) = best_match {
                // Update existing track
                if let Some(track) = tracks.get_mut(&track_id) {
                    track.object = detection.clone();
                    track.age = 0;
                    matched[det_idx] = true;
                    track_ids[det_idx] = Some(track_id);
                }
            }
        }

        // Create new tracks for unmatched detections
        // Limit number of tracks to prevent memory exhaustion
        const MAX_TRACKS: usize = 1000;
        if tracks.len() >= MAX_TRACKS {
            // Remove oldest tracks if we're at the limit
            let mut sorted_tracks: Vec<(u64, u32)> = tracks.iter()
                .map(|(id, track)| (*id, track.age))
                .collect();
            sorted_tracks.sort_by_key(|(_, age)| *age);
            
            // Remove oldest 10% of tracks
            let remove_count = (MAX_TRACKS / 10).max(1);
            for (id, _) in sorted_tracks.iter().take(remove_count) {
                tracks.remove(id);
            }
        }
        
        for (det_idx, detection) in detections.iter().enumerate() {
            if !matched[det_idx] {
                // Find next available track ID, avoiding collisions
                let mut track_id = *next_id;
                let mut attempts = 0;
                while tracks.contains_key(&track_id) && attempts < 1000 {
                    *next_id = next_id.wrapping_add(1);
                    if *next_id == 0 {
                        *next_id = 1; // Skip 0
                    }
                    track_id = *next_id;
                    attempts += 1;
                }
                
                if attempts >= 1000 {
                    warn!("Could not find available track ID, skipping detection");
                    continue;
                }

                let track = TrackedObject {
                    id: track_id,
                    object: detection.clone(),
                    age: 0,
                };

                tracks.insert(track_id, track);
                track_ids[det_idx] = Some(track_id);
                
                // Advance next_id for next iteration
                *next_id = next_id.wrapping_add(1);
                if *next_id == 0 {
                    *next_id = 1; // Skip 0
                }
            }
        }

        // Remove old tracks
        tracks.retain(|_, track| track.age <= self.max_age);

        // Return all active tracks
        let active_tracks: Vec<TrackedObject> = tracks.values()
            .filter(|t| t.age <= self.max_age)
            .cloned()
            .collect();

        debug!("Tracking {} objects", active_tracks.len());
        active_tracks
    }

    /// Compute IoU (Intersection over Union) between two bounding boxes
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

    /// Get all active tracks
    pub fn get_tracks(&self) -> Vec<TrackedObject> {
        let tracks = self.tracks.read();
        tracks.values()
            .filter(|t| t.age <= self.max_age)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DetectedObject;

    fn create_detection(class_id: usize, class_name: &str, confidence: f32, bbox: (f32, f32, f32, f32)) -> DetectedObject {
        DetectedObject {
            class_id,
            class_name: class_name.to_string(),
            confidence,
            bbox,
        }
    }

    #[test]
    fn test_tracker_new() {
        let tracker = ObjectTracker::new(30, 0.3);
        let tracks = tracker.get_tracks();
        assert_eq!(tracks.len(), 0);
    }

    #[test]
    fn test_tracker_update_empty() {
        let tracker = ObjectTracker::new(30, 0.3);
        let detections = vec![];
        let tracks = tracker.update(&detections);
        assert_eq!(tracks.len(), 0);
    }

    #[test]
    fn test_tracker_update_single_detection() {
        let tracker = ObjectTracker::new(30, 0.3);
        let detections = vec![
            create_detection(0, "person", 0.9, (10.0, 10.0, 50.0, 50.0)),
        ];
        let tracks = tracker.update(&detections);
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].object.class_name, "person");
        assert_eq!(tracks[0].age, 0);
    }

    #[test]
    fn test_tracker_update_multiple_detections() {
        let tracker = ObjectTracker::new(30, 0.3);
        let detections = vec![
            create_detection(0, "person", 0.9, (10.0, 10.0, 50.0, 50.0)),
            create_detection(1, "car", 0.8, (100.0, 100.0, 60.0, 60.0)),
        ];
        let tracks = tracker.update(&detections);
        assert_eq!(tracks.len(), 2);
    }

    #[test]
    fn test_tracker_tracking_across_frames() {
        let tracker = ObjectTracker::new(30, 0.3);
        
        // First frame
        let detections1 = vec![
            create_detection(0, "person", 0.9, (10.0, 10.0, 50.0, 50.0)),
        ];
        let tracks1 = tracker.update(&detections1);
        assert_eq!(tracks1.len(), 1);
        let track_id = tracks1[0].id;
        
        // Second frame - same object slightly moved
        let detections2 = vec![
            create_detection(0, "person", 0.9, (12.0, 12.0, 50.0, 50.0)),
        ];
        let tracks2 = tracker.update(&detections2);
        assert_eq!(tracks2.len(), 1);
        assert_eq!(tracks2[0].id, track_id); // Same track ID
        assert_eq!(tracks2[0].age, 0); // Age reset
    }

    #[test]
    fn test_tracker_track_aging() {
        let tracker = ObjectTracker::new(5, 0.3);
        
        // Create a track
        let detections = vec![
            create_detection(0, "person", 0.9, (10.0, 10.0, 50.0, 50.0)),
        ];
        let _ = tracker.update(&detections);
        
        // Update with no detections (track ages)
        for _ in 0..5 {
            let tracks = tracker.update(&[]);
            assert!(tracks.len() > 0);
        }
        
        // After max_age, track should be removed
        let tracks = tracker.update(&[]);
        assert_eq!(tracks.len(), 0);
    }

    #[test]
    fn test_tracker_iou_identical() {
        let tracker = ObjectTracker::new(30, 0.3);
        let bbox = (10.0, 10.0, 50.0, 50.0);
        let iou = tracker.compute_iou(&bbox, &bbox);
        assert!((iou - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_tracker_iou_no_overlap() {
        let tracker = ObjectTracker::new(30, 0.3);
        let bbox1 = (10.0, 10.0, 50.0, 50.0);
        let bbox2 = (200.0, 200.0, 50.0, 50.0);
        let iou = tracker.compute_iou(&bbox1, &bbox2);
        assert_eq!(iou, 0.0);
    }

    #[test]
    fn test_tracker_iou_partial_overlap() {
        let tracker = ObjectTracker::new(30, 0.3);
        let bbox1 = (10.0, 10.0, 50.0, 50.0);
        let bbox2 = (30.0, 30.0, 50.0, 50.0);
        let iou = tracker.compute_iou(&bbox1, &bbox2);
        assert!(iou > 0.0 && iou < 1.0);
    }

    #[test]
    fn test_tracker_iou_invalid_inputs() {
        let tracker = ObjectTracker::new(30, 0.3);
        
        // NaN inputs
        let bbox1 = (f32::NAN, 10.0, 50.0, 50.0);
        let bbox2 = (10.0, 10.0, 50.0, 50.0);
        assert_eq!(tracker.compute_iou(&bbox1, &bbox2), 0.0);
        
        // Negative dimensions
        let bbox3 = (10.0, 10.0, -50.0, 50.0);
        assert_eq!(tracker.compute_iou(&bbox2, &bbox3), 0.0);
        
        // Infinite values
        let bbox4 = (f32::INFINITY, 10.0, 50.0, 50.0);
        assert_eq!(tracker.compute_iou(&bbox2, &bbox4), 0.0);
    }

    #[test]
    fn test_tracker_max_tracks_limit() {
        let tracker = ObjectTracker::new(30, 0.3);
        
        // Create many detections that won't match (low IoU threshold)
        let mut detections = Vec::new();
        for i in 0..1100 {
            detections.push(create_detection(
                0,
                "person",
                0.9,
                (i as f32 * 200.0, 10.0, 50.0, 50.0),
            ));
        }
        
        let tracks = tracker.update(&detections);
        // Should be limited to MAX_TRACKS (1000)
        assert!(tracks.len() <= 1000);
    }

    #[test]
    fn test_tracker_id_collision_avoidance() {
        let tracker = ObjectTracker::new(30, 0.3);
        
        // Create many detections to test ID collision handling
        let mut detections = Vec::new();
        for i in 0..100 {
            detections.push(create_detection(
                0,
                "person",
                0.9,
                (i as f32 * 200.0, 10.0, 50.0, 50.0),
            ));
        }
        
        let tracks = tracker.update(&detections);
        // All tracks should have unique IDs
        let mut ids: Vec<u64> = tracks.iter().map(|t| t.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), tracks.len());
    }
}
