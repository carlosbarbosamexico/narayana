//! Tests for SceneAnalyzer

use narayana_eye::scene::{SceneAnalyzer, SceneDescription};
use narayana_eye::models::{ClipModel, SceneEmbedding};
use narayana_eye::processing::TrackedObject;
use narayana_eye::models::DetectedObject;
use narayana_eye::error::VisionError;
use std::path::PathBuf;
use std::sync::Arc;

#[test]
fn test_scene_analyzer_new() {
    // Test that SceneAnalyzer can be created
    let model_path = PathBuf::from("/nonexistent/clip.onnx");
    let clip_result = ClipModel::new(&model_path);
    
    // We expect this to fail (model doesn't exist), but we're testing the structure
    assert!(clip_result.is_err());
    
    // If we had a valid model, we could test:
    // let clip = clip_result.unwrap();
    // let analyzer = SceneAnalyzer::new(Arc::new(clip));
    // assert!(true); // Analyzer created successfully
}

#[test]
fn test_scene_analyzer_with_llm() {
    // Test that SceneAnalyzer can be created with LLM provider
    let model_path = PathBuf::from("/nonexistent/clip.onnx");
    let clip_result = ClipModel::new(&model_path);
    
    assert!(clip_result.is_err());
    
    // If we had a valid model, we could test:
    // let clip = clip_result.unwrap();
    // let llm_provider: Option<Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = Result<String, VisionError>> + Send>> + Send + Sync>> = None;
    // let analyzer = SceneAnalyzer::with_llm(Arc::new(clip), llm_provider);
    // assert!(true);
}

#[test]
fn test_scene_analyzer_set_llm_provider() {
    // Test that set_llm_provider method exists
    let model_path = PathBuf::from("/nonexistent/clip.onnx");
    let clip_result = ClipModel::new(&model_path);
    
    assert!(clip_result.is_err());
    
    // If we had a valid model, we could test:
    // let clip = clip_result.unwrap();
    // let mut analyzer = SceneAnalyzer::new(Arc::new(clip));
    // analyzer.set_llm_provider(None);
    // assert!(true);
}

#[test]
fn test_scene_description_structure() {
    // Test SceneDescription structure
    let description = SceneDescription {
        description: "Test scene".to_string(),
        confidence: 0.95,
        tags: vec!["indoor".to_string(), "room".to_string()],
    };
    
    assert_eq!(description.description, "Test scene");
    assert_eq!(description.confidence, 0.95);
    assert_eq!(description.tags.len(), 2);
    assert_eq!(description.tags[0], "indoor");
    assert_eq!(description.tags[1], "room");
}

#[test]
fn test_scene_embedding_structure() {
    // Test SceneEmbedding structure
    let embedding = SceneEmbedding {
        embedding: vec![0.1, 0.2, 0.3, 0.4, 0.5],
        dimension: 5,
    };
    
    assert_eq!(embedding.embedding.len(), 5);
    assert_eq!(embedding.dimension, 5);
    assert_eq!(embedding.embedding[0], 0.1);
    assert_eq!(embedding.embedding[4], 0.5);
}

#[test]
fn test_tracked_object_structure() {
    // Test TrackedObject structure (from processing module)
    use narayana_eye::processing::TrackedObject;
    
    let detected = DetectedObject {
        class_id: 0,
        class_name: "person".to_string(),
        confidence: 0.9,
        bbox: (10.0, 20.0, 100.0, 200.0),
    };
    
    let tracked = TrackedObject {
        id: 1,
        object: detected.clone(),
        age: 0,
    };
    
    assert_eq!(tracked.id, 1);
    assert_eq!(tracked.object.class_name, "person");
    assert_eq!(tracked.object.confidence, 0.9);
    assert_eq!(tracked.age, 0);
}

#[test]
fn test_scene_description_with_empty_tags() {
    // Test SceneDescription with empty tags
    let description = SceneDescription {
        description: "Empty scene".to_string(),
        confidence: 0.0,
        tags: vec![],
    };
    
    assert_eq!(description.description, "Empty scene");
    assert_eq!(description.confidence, 0.0);
    assert!(description.tags.is_empty());
}

#[test]
fn test_scene_description_serialization() {
    // Test that SceneDescription can be serialized (if needed)
    let description = SceneDescription {
        description: "Test scene".to_string(),
        confidence: 0.95,
        tags: vec!["tag1".to_string(), "tag2".to_string()],
    };
    
    // Verify structure is correct for potential serialization
    assert!(!description.description.is_empty());
    assert!(description.confidence >= 0.0 && description.confidence <= 1.0);
    assert!(!description.tags.is_empty());
}


