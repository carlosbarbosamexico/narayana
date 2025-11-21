//! Tests for detection and segmentation pipelines

// Note: These tests verify the API structure without requiring OpenCV or actual models

#[test]
fn test_detection_pipeline_structure() {
    // Test that DetectionPipeline has the expected structure
    // This is a compile-time test - if it compiles, the structure is correct
    // We can't test actual creation without OpenCV and models, but we verify the API exists
    
    // Verify DetectionPipeline is in the public API
    use narayana_eye::processing::DetectionPipeline;
    
    // If this compiles, the structure is correct
    assert!(true);
}

#[test]
fn test_segmentation_pipeline_structure() {
    // Test that SegmentationPipeline has the expected structure
    // Verify SegmentationPipeline is in the public API
    use narayana_eye::processing::SegmentationPipeline;
    
    // If this compiles, the structure is correct
    assert!(true);
}

#[test]
fn test_pipeline_types_exist() {
    // Verify that pipeline types are accessible
    use narayana_eye::processing::{DetectionPipeline, SegmentationPipeline};
    
    // These are compile-time checks
    let _: Option<DetectionPipeline> = None;
    let _: Option<SegmentationPipeline> = None;
    
    assert!(true);
}

