//! Scene understanding and analysis

use crate::error::VisionError;
use crate::models::{ClipModel, SceneEmbedding, SceneDescription};
use crate::processing::TrackedObject;
use opencv::prelude::Mat;
use std::sync::Arc;
use tracing::debug;

/// Optional LLM integration for enhanced descriptions
/// This is a function that takes a base description and returns an enhanced one
pub type LLMProviderFn = Arc<dyn Fn(String) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, VisionError>> + Send>> + Send + Sync>;
pub type LLMProvider = Option<LLMProviderFn>;

/// Scene analyzer for high-level understanding
pub struct SceneAnalyzer {
    clip: Arc<ClipModel>,
    llm_provider: LLMProvider,
}

impl SceneAnalyzer {
    /// Create a new scene analyzer
    pub fn new(clip: Arc<ClipModel>) -> Self {
        Self {
            clip,
            llm_provider: None,
        }
    }

    /// Create a new scene analyzer with LLM integration
    pub fn with_llm(clip: Arc<ClipModel>, llm_provider: LLMProvider) -> Self {
        Self {
            clip,
            llm_provider,
        }
    }

    /// Set LLM provider (brain-controlled)
    pub fn set_llm_provider(&mut self, provider: LLMProvider) {
        self.llm_provider = provider;
    }

    /// Analyze scene and generate description
    pub async fn analyze_scene(
        &self,
        frame: &Mat,
        tracked_objects: &[TrackedObject],
    ) -> Result<SceneDescription, VisionError> {
        debug!("Analyzing scene with {} tracked objects", tracked_objects.len());

        // Generate scene embedding
        let embedding = self.clip.encode_image(frame)?;

        // Build description from tracked objects
        let mut description_parts = Vec::new();
        let mut tags = Vec::new();

        if tracked_objects.is_empty() {
            description_parts.push("No objects detected".to_string());
        } else {
            description_parts.push(format!("Scene contains {} objects:", tracked_objects.len()));
            
            for obj in tracked_objects {
                let class_name = &obj.object.class_name;
                description_parts.push(format!("- {} (confidence: {:.2})", class_name, obj.object.confidence));
                tags.push(class_name.clone());
            }
        }

        let mut description = description_parts.join("\n");

        // Match to common scene types
        let scene_types = vec![
            "indoor room",
            "outdoor street",
            "office",
            "kitchen",
            "living room",
            "outdoor nature",
        ];

        let similarities = self.clip.match_text(&embedding, &scene_types.iter().map(|s| *s).collect::<Vec<_>>())?;
        let max_similarity = similarities.iter().cloned().fold(0.0f32, f32::max);
        let confidence = max_similarity;

        // Enhance description with LLM if available (brain-controlled)
        // Sanitize input to prevent prompt injection
        if let Some(llm_fn) = &self.llm_provider {
            let base_description = description.clone();
            
            // Sanitize description to prevent prompt injection
            // Remove control characters and limit length
            let sanitized: String = base_description
                .chars()
                .filter(|c| !c.is_control() || c.is_whitespace())
                .take(2000) // Limit length to prevent DoS
                .collect();
            
            match llm_fn(sanitized).await {
                Ok(enhanced) => {
                    // Limit enhanced description length
                    description = enhanced.chars().take(5000).collect();
                    debug!("LLM enhanced scene description");
                }
                Err(e) => {
                    debug!("LLM enhancement failed: {}, using base description", e);
                }
            }
        }

        Ok(SceneDescription {
            description,
            confidence,
            tags,
        })
    }

    /// Get scene embedding for semantic search
    pub fn get_embedding(&self, frame: &Mat) -> Result<SceneEmbedding, VisionError> {
        self.clip.encode_image(frame)
    }
}

