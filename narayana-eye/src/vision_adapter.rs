//! Vision adapter for narayana-wld integration

use crate::camera::CameraManager;
use crate::config::{VisionConfig, ProcessingMode};
use crate::error::VisionError;
use crate::models::{ModelManager, YoloModel, SamModel, ClipModel};
use crate::processing::{DetectionPipeline, SegmentationPipeline, ObjectTracker};
use crate::scene::{SceneAnalyzer, LLMProvider};
use narayana_llm::{LLMManager};
use narayana_llm::config::{Message, MessageRole};
use narayana_wld::protocol_adapters::ProtocolAdapter;
use narayana_wld::world_broker::WorldBrokerHandle;
use narayana_wld::event_transformer::{WorldEvent, WorldAction};
use narayana_core::Error;
use async_trait::async_trait;
use opencv::prelude::Mat;
use serde_json::json;
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tracing::{info, warn, error, debug};

/// Vision adapter implementing ProtocolAdapter for narayana-wld
pub struct VisionAdapter {
    config: Arc<VisionConfig>,
    camera: Arc<CameraManager>,
    model_manager: Arc<ModelManager>,
    detection_pipeline: Arc<RwLock<Option<Arc<DetectionPipeline>>>>,
    segmentation_pipeline: Arc<RwLock<Option<Arc<SegmentationPipeline>>>>,
    tracker: Arc<ObjectTracker>,
    scene_analyzer: Arc<RwLock<Option<Arc<SceneAnalyzer>>>>,
    event_sender: Arc<RwLock<Option<broadcast::Sender<WorldEvent>>>>,
    is_running: Arc<RwLock<bool>>,
    frame_receiver: Arc<RwLock<Option<mpsc::Receiver<Mat>>>>,
    llm_manager: Option<Arc<LLMManager>>,
    process_request_sender: Arc<RwLock<Option<mpsc::Sender<()>>>>,
    processing_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    on_demand_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl VisionAdapter {
    /// Create a new vision adapter
    pub fn new(config: VisionConfig) -> Result<Self, Error> {
        config.validate()
            .map_err(|e| Error::Storage(format!("Invalid vision config: {}", e)))?;

        let config = Arc::new(config);
        let camera = Arc::new(CameraManager::new(config.clone()));
        let model_manager = Arc::new(ModelManager::new(config.clone()));
        let tracker = Arc::new(ObjectTracker::new(30, 0.3)); // max_age=30, iou_threshold=0.3

        Ok(Self {
            config: config.clone(),
            camera,
            model_manager,
            detection_pipeline: Arc::new(RwLock::new(None)),
            segmentation_pipeline: Arc::new(RwLock::new(None)),
            tracker,
            scene_analyzer: Arc::new(RwLock::new(None)),
            event_sender: Arc::new(RwLock::new(None)),
            is_running: Arc::new(RwLock::new(false)),
            frame_receiver: Arc::new(RwLock::new(None)),
            llm_manager: None,
            process_request_sender: Arc::new(RwLock::new(None)),
            processing_handle: Arc::new(RwLock::new(None)),
            on_demand_handle: Arc::new(RwLock::new(None)),
        })
    }

    /// Process a single frame on demand
    pub async fn process_frame_on_demand(&self) -> Result<(), VisionError> {
        let frame = self.camera.capture_frame()?;
        process_frame_internal(
            &frame,
            &self.config,
            &self.detection_pipeline,
            &self.segmentation_pipeline,
            &self.tracker,
            &self.scene_analyzer,
            &self.event_sender,
        ).await
    }

    /// Set LLM manager for brain-controlled descriptions
    pub fn set_llm_manager(&mut self, llm_manager: Option<Arc<LLMManager>>) {
        self.llm_manager = llm_manager;
    }

    /// Clone adapter for on-demand processing
    fn clone_for_on_demand(&self) -> VisionAdapterOnDemand {
        VisionAdapterOnDemand {
            camera: self.camera.clone(),
            config: self.config.clone(),
            detection_pipeline: self.detection_pipeline.clone(),
            segmentation_pipeline: self.segmentation_pipeline.clone(),
            tracker: self.tracker.clone(),
            scene_analyzer: self.scene_analyzer.clone(),
            event_sender: self.event_sender.clone(),
        }
    }

    /// Start processing loop
    async fn start_processing_loop(&self) -> Result<(), VisionError> {
        let frame_receiver = self.frame_receiver.read()
            .as_ref()
            .ok_or_else(|| VisionError::Processing("Frame receiver not initialized".to_string()))?
            .clone();

        let config = self.config.clone();
        let detection_pipeline = self.detection_pipeline.clone();
        let segmentation_pipeline = self.segmentation_pipeline.clone();
        let tracker = self.tracker.clone();
        let scene_analyzer = self.scene_analyzer.clone();
        let event_sender = self.event_sender.clone();
        let is_running = self.is_running.clone();

        let handle = tokio::spawn(async move {
            let mut frame_receiver = frame_receiver;
            loop {
                // Check if we should stop
                if !*is_running.read() {
                    break;
                }

                // Use timeout to periodically check is_running
                match tokio::time::timeout(
                    std::time::Duration::from_millis(100),
                    frame_receiver.recv()
                ).await {
                    Ok(Some(frame)) => {
                        if let Err(e) = process_frame_internal(
                            &frame,
                            &config,
                            &detection_pipeline,
                            &segmentation_pipeline,
                            &tracker,
                            &scene_analyzer,
                            &event_sender,
                        ).await {
                            error!("Frame processing error: {}", e);
                        }
                    }
                    Ok(None) => {
                        warn!("Frame receiver closed, stopping processing loop");
                        break;
                    }
                    Err(_) => {
                        // Timeout - check is_running again
                        continue;
                    }
                }
            }

            *is_running.write() = false;
            info!("Camera stream stopped");
        });
        
        // Store handle for cleanup
        *self.processing_handle.write() = Some(handle);

        Ok(())
    }

    /// Initialize models
    async fn initialize_models(&self) -> Result<(), VisionError> {
        info!("Initializing vision models...");

        // Track which models were successfully loaded for rollback
        let mut loaded_models = Vec::new();

        // Load YOLO model if detection is enabled
        if self.config.enable_detection {
            match self.model_manager.get_yolo_model().await {
                Ok(yolo_path) => {
                    match YoloModel::new(&yolo_path) {
                        Ok(yolo) => {
                            let detection = Arc::new(DetectionPipeline::new(Arc::new(yolo)));
                            *self.detection_pipeline.write() = Some(detection);
                            loaded_models.push("yolo");
                            info!("YOLO detection model loaded");
                        }
                        Err(e) => {
                            // Rollback on failure
                            self.rollback_models(&loaded_models);
                            return Err(VisionError::Model(format!("Failed to load YOLO model: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    self.rollback_models(&loaded_models);
                    return Err(e);
                }
            }
        }

        // Load SAM model if segmentation is enabled
        if self.config.enable_segmentation {
            match self.model_manager.get_sam_model().await {
                Ok(sam_path) => {
                    match SamModel::new(&sam_path) {
                        Ok(sam) => {
                            let segmentation = Arc::new(SegmentationPipeline::new(Arc::new(sam)));
                            *self.segmentation_pipeline.write() = Some(segmentation);
                            loaded_models.push("sam");
                            info!("SAM segmentation model loaded");
                        }
                        Err(e) => {
                            self.rollback_models(&loaded_models);
                            return Err(VisionError::Model(format!("Failed to load SAM model: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    self.rollback_models(&loaded_models);
                    return Err(e);
                }
            }
        }

        // Load CLIP model if scene understanding is enabled
        if self.config.enable_scene_understanding {
            match self.model_manager.get_clip_model().await {
                Ok(clip_path) => {
                    match ClipModel::new(&clip_path) {
                        Ok(clip) => {
                            // Create LLM provider if LLM integration is enabled
                            let llm_provider: LLMProvider = if self.config.llm_integration {
                                if let Some(llm_mgr) = &self.llm_manager {
                                    let llm_clone = llm_mgr.clone();
                                    let provider_fn: crate::scene::LLMProviderFn = Arc::new(move |description: String| {
                                        let llm = llm_clone.clone();
                                        Box::pin(async move {
                                            let prompt = format!(
                                                "Describe this scene in natural language based on the following vision analysis:\n\n{}",
                                                description
                                            );
                                            llm.chat(vec![Message {
                                                role: MessageRole::User,
                                                content: prompt,
                                            }], None).await
                                                .map_err(|e| VisionError::Model(format!("LLM error: {}", e)))
                                        })
                                    });
                                    Some(provider_fn)
                                } else {
                                    warn!("LLM integration enabled but LLM manager not provided");
                                    None
                                }
                            } else {
                                None
                            };
                            
                            let analyzer = if llm_provider.is_some() {
                                Arc::new(SceneAnalyzer::with_llm(Arc::new(clip), llm_provider))
                            } else {
                                Arc::new(SceneAnalyzer::new(Arc::new(clip)))
                            };
                            *self.scene_analyzer.write() = Some(analyzer);
                            loaded_models.push("clip");
                            info!("CLIP scene understanding model loaded");
                        }
                        Err(e) => {
                            self.rollback_models(&loaded_models);
                            return Err(VisionError::Model(format!("Failed to load CLIP model: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    self.rollback_models(&loaded_models);
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    /// Rollback loaded models on initialization failure
    fn rollback_models(&self, loaded_models: &[&str]) {
        for model in loaded_models {
            match *model {
                "yolo" => {
                    *self.detection_pipeline.write() = None;
                }
                "sam" => {
                    *self.segmentation_pipeline.write() = None;
                }
                "clip" => {
                    *self.scene_analyzer.write() = None;
                }
                _ => {}
            }
        }
    }
}

/// Helper struct for on-demand processing
struct VisionAdapterOnDemand {
    camera: Arc<CameraManager>,
    config: Arc<VisionConfig>,
    detection_pipeline: Arc<RwLock<Option<Arc<DetectionPipeline>>>>,
    segmentation_pipeline: Arc<RwLock<Option<Arc<SegmentationPipeline>>>>,
    tracker: Arc<ObjectTracker>,
    scene_analyzer: Arc<RwLock<Option<Arc<SceneAnalyzer>>>>,
    event_sender: Arc<RwLock<Option<broadcast::Sender<WorldEvent>>>>,
}

impl VisionAdapterOnDemand {
    async fn process_frame_on_demand(&self) -> Result<(), VisionError> {
        let frame = self.camera.capture_frame()?;
        process_frame_internal(
            &frame,
            &self.config,
            &self.detection_pipeline,
            &self.segmentation_pipeline,
            &self.tracker,
            &self.scene_analyzer,
            &self.event_sender,
        ).await
    }
}

impl VisionAdapter {
    /// Start processing loop
    async fn start_processing_loop(&self) -> Result<(), VisionError> {
        let frame_receiver = self.frame_receiver.read()
            .as_ref()
            .ok_or_else(|| VisionError::Processing("Frame receiver not initialized".to_string()))?
            .clone();

        let config = self.config.clone();
        let detection_pipeline = self.detection_pipeline.clone();
        let segmentation_pipeline = self.segmentation_pipeline.clone();
        let tracker = self.tracker.clone();
        let scene_analyzer = self.scene_analyzer.clone();
        let event_sender = self.event_sender.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            let mut frame_receiver = frame_receiver;
            loop {
                if !*is_running.read() {
                    break;
                }

                match frame_receiver.recv().await {
                    Some(frame) => {
                        if let Err(e) = process_frame_internal(
                            &frame,
                            &config,
                            &detection_pipeline,
                            &segmentation_pipeline,
                            &tracker,
                            &scene_analyzer,
                            &event_sender,
                        ).await {
                            error!("Frame processing error: {}", e);
                        }
                    }
                    None => {
                        warn!("Frame receiver closed, stopping processing loop");
                        break;
                    }
                }
            }
        });

        Ok(())
    }
}

/// Internal frame processing function
async fn process_frame_internal(
    frame: &Mat,
    config: &Arc<VisionConfig>,
    detection_pipeline: &Arc<RwLock<Option<Arc<DetectionPipeline>>>>,
    segmentation_pipeline: &Arc<RwLock<Option<Arc<SegmentationPipeline>>>>,
    tracker: &Arc<ObjectTracker>,
    scene_analyzer: &Arc<RwLock<Option<Arc<SceneAnalyzer>>>>,
    event_sender: &Arc<RwLock<Option<broadcast::Sender<WorldEvent>>>>,
) -> Result<(), VisionError> {
    // Use timestamp_nanos_opt to handle potential overflow gracefully
    let timestamp = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| chrono::Utc::now().timestamp() as i64 * 1_000_000_000) as u64;
    
    let mut vision_data = json!({
        "timestamp": timestamp,
        "camera_id": config.camera_id,
    });

    // Object detection
    let mut detections = Vec::new();
    if config.enable_detection {
        if let Some(detection) = detection_pipeline.read().as_ref() {
            match detection.detect(frame) {
                Ok(dets) => {
                    // Limit detections to prevent JSON serialization DoS
                    const MAX_DETECTIONS: usize = 100;
                    detections = if dets.len() > MAX_DETECTIONS {
                        let mut limited = dets;
                        limited.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
                        limited.truncate(MAX_DETECTIONS);
                        limited
                    } else {
                        dets
                    };
                    
                    let detections_json: Vec<serde_json::Value> = detections.iter()
                        .map(|d| json!({
                            "class_id": d.class_id,
                            "class_name": d.class_name,
                            "confidence": d.confidence,
                            "bbox": [d.bbox.0, d.bbox.1, d.bbox.2, d.bbox.3],
                        }))
                        .collect();
                    vision_data["detections"] = json!(detections_json);
                }
                Err(e) => {
                    warn!("Detection error: {}", e);
                }
            }
        }
    }

    // Object tracking
    let mut tracked_objects = Vec::new();
    if config.enable_tracking && !detections.is_empty() {
        tracked_objects = tracker.update(&detections);
        
        // Limit tracked objects for JSON serialization
        const MAX_TRACKS_JSON: usize = 100;
        if tracked_objects.len() > MAX_TRACKS_JSON {
            tracked_objects.truncate(MAX_TRACKS_JSON);
        }
        
        let tracks_json: Vec<serde_json::Value> = tracked_objects.iter()
            .map(|t| json!({
                "id": t.id,
                "class_id": t.object.class_id,
                "class_name": t.object.class_name,
                "confidence": t.object.confidence,
                "bbox": [t.object.bbox.0, t.object.bbox.1, t.object.bbox.2, t.object.bbox.3],
                "age": t.age,
            }))
            .collect();
        vision_data["tracks"] = json!(tracks_json);
    }

    // Instance segmentation
    if config.enable_segmentation {
        if let Some(segmentation) = segmentation_pipeline.read().as_ref() {
            // Limit prompts to prevent excessive processing
            const MAX_SEGMENTATION_PROMPTS: usize = 50;
            let prompts: Vec<(f32, f32)> = detections.iter()
                .take(MAX_SEGMENTATION_PROMPTS)
                .map(|d| {
                    // Validate bbox before computing center
                    let center_x = if d.bbox.2 > 0.0 && d.bbox.2.is_finite() {
                        d.bbox.0 + d.bbox.2 / 2.0
                    } else {
                        d.bbox.0
                    };
                    let center_y = if d.bbox.3 > 0.0 && d.bbox.3.is_finite() {
                        d.bbox.1 + d.bbox.3 / 2.0
                    } else {
                        d.bbox.1
                    };
                    (center_x, center_y)
                })
                .collect();
            
            match segmentation.segment(frame, &prompts) {
                Ok(masks) => {
                    // Limit masks for JSON serialization
                    const MAX_MASKS_JSON: usize = 50;
                    let limited_masks: Vec<_> = masks.iter().take(MAX_MASKS_JSON).collect();
                    let masks_json: Vec<serde_json::Value> = limited_masks.iter()
                        .map(|m| json!({
                            "bbox": [m.bbox.0, m.bbox.1, m.bbox.2, m.bbox.3],
                            "confidence": m.confidence,
                        }))
                        .collect();
                    vision_data["masks"] = json!(masks_json);
                }
                Err(e) => {
                    warn!("Segmentation error: {}", e);
                }
            }
        }
    }

    // Scene understanding
    if config.enable_scene_understanding {
        if let Some(analyzer) = scene_analyzer.read().as_ref() {
            match analyzer.analyze_scene(frame, &tracked_objects).await {
                Ok(description) => {
                    vision_data["scene"] = json!({
                        "description": description.description,
                        "confidence": description.confidence,
                        "tags": description.tags,
                    });
                }
                Err(e) => {
                    warn!("Scene analysis error: {}", e);
                }
            }
        }
    }

    // Emit vision event
    if let Some(sender) = event_sender.read().as_ref() {
        let event = WorldEvent::SensorData {
            source: format!("camera_{}", config.camera_id),
            data: vision_data,
            timestamp,
        };

        // Try to send event, but don't block if channel is full
        match sender.try_send(event) {
            Ok(_) => {}
            Err(tokio::sync::broadcast::error::TrySendError::Full(_)) => {
                warn!("Vision event channel full, dropping event");
            }
            Err(tokio::sync::broadcast::error::TrySendError::Closed(_)) => {
                warn!("Vision event channel closed");
            }
        }
    }

    Ok(())
}
}

#[async_trait]
impl ProtocolAdapter for VisionAdapter {
    fn protocol_name(&self) -> &str {
        "vision"
    }

    async fn start(&self, broker: WorldBrokerHandle) -> Result<(), Error> {
        // Check if already running (atomic check)
        {
            let mut is_running = self.is_running.write();
            if *is_running {
                return Err(Error::Storage("Vision adapter already running".to_string()));
            }
            *is_running = true;
        }

        info!("Starting vision adapter");

        // Initialize camera (with rollback on failure)
        if let Err(e) = self.camera.initialize() {
            *self.is_running.write() = false;
            return Err(Error::Storage(format!("Camera initialization failed: {}", e)));
        }

        // Initialize models (with rollback on failure)
        if let Err(e) = self.initialize_models().await {
            *self.is_running.write() = false;
            self.camera.stop();
            return Err(Error::Storage(format!("Model initialization failed: {}", e)));
        }

        // Create event channel with reasonable buffer size
        // Larger buffer prevents event loss, but still bounded
        const EVENT_BUFFER_SIZE: usize = 5000;
        let (sender, _) = broadcast::channel(EVENT_BUFFER_SIZE);
        *self.event_sender.write() = Some(sender.clone());

        // Start camera stream (with rollback on failure)
        let frame_receiver = match self.camera.start_stream() {
            Ok(rx) => rx,
            Err(e) => {
                *self.is_running.write() = false;
                self.camera.stop();
                *self.event_sender.write() = None;
                return Err(Error::Storage(format!("Failed to start camera stream: {}", e)));
            }
        };
        *self.frame_receiver.write() = Some(frame_receiver);

        // Start processing based on mode (with rollback on failure)
        match self.config.processing_mode {
            ProcessingMode::RealTime => {
                match self.start_processing_loop().await {
                    Ok(()) => {}
                    Err(e) => {
                        // Rollback on failure
                        *self.is_running.write() = false;
                        self.camera.stop();
                        *self.event_sender.write() = None;
                        *self.frame_receiver.write() = None;
                        return Err(Error::Storage(format!("Failed to start processing loop: {}", e)));
                    }
                }
            }
            ProcessingMode::OnDemand => {
                // On-demand processing: set up command channel
                let (tx, mut rx) = mpsc::channel(10);
                *self.process_request_sender.write() = Some(tx);
                
                let adapter_clone = self.clone_for_on_demand();
                let is_running_clone = self.is_running.clone();
                let handle = tokio::spawn(async move {
                    loop {
                        // Check if we should stop
                        if !*is_running_clone.read() {
                            break;
                        }
                        
                        // Use timeout to periodically check is_running
                        match tokio::time::timeout(
                            std::time::Duration::from_millis(100),
                            rx.recv()
                        ).await {
                            Ok(Some(_)) => {
                                if let Err(e) = adapter_clone.process_frame_on_demand().await {
                                    error!("On-demand frame processing error: {}", e);
                                }
                            }
                            Ok(None) => {
                                // Channel closed
                                break;
                            }
                            Err(_) => {
                                // Timeout - check is_running again
                                continue;
                            }
                        }
                    }
                    info!("On-demand processing task stopped");
                });
                *self.on_demand_handle.write() = Some(handle);
                info!("Vision adapter running in on-demand mode");
            }
        }

        info!("Vision adapter started successfully");
        Ok(())
    }

    async fn stop(&self) -> Result<(), Error> {
        // Set is_running to false first to signal tasks to stop
        {
            let mut is_running = self.is_running.write();
            if !*is_running {
                return Ok(()); // Already stopped
            }
            *is_running = false;
        }

        // Abort processing tasks if they exist
        if let Some(handle) = self.processing_handle.write().take() {
            handle.abort();
            // Wait a bit for task to finish
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(1),
                handle
            ).await;
        }

        if let Some(handle) = self.on_demand_handle.write().take() {
            handle.abort();
            // Wait a bit for task to finish
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(1),
                handle
            ).await;
        }

        // Close process request sender to unblock on-demand task
        if let Some(sender) = self.process_request_sender.write().take() {
            drop(sender); // This will close the channel
        }

        // Stop camera
        self.camera.stop();

        // Clear event sender (this will close the channel)
        *self.event_sender.write() = None;

        // Clear frame receiver
        *self.frame_receiver.write() = None;

        info!("Vision adapter stopped");
        Ok(())
    }

    async fn send_action(&self, action: WorldAction) -> Result<(), Error> {
        // Handle camera control commands
        match action {
            WorldAction::ActuatorCommand { target, command } => {
                if target == format!("camera_{}", self.config.camera_id) {
                    // Handle camera commands (e.g., change resolution, frame rate)
                    debug!("Received camera command: {:?}", command);
                    
                    // Check for on-demand processing request
                    if let Some(cmd_str) = command.get("command").and_then(|v| v.as_str()) {
                        if cmd_str == "process_frame" {
                            // Trigger on-demand frame processing
                            if let Some(sender) = self.process_request_sender.read().as_ref() {
                                if sender.send(()).await.is_err() {
                                    warn!("Failed to send on-demand processing request");
                                }
                            }
                        }
                    }
                }
            }
            WorldAction::Command { command, args } => {
                // Handle direct commands for vision system
                if command == "process_frame" {
                    if let Some(sender) = self.process_request_sender.read().as_ref() {
                        if sender.send(()).await.is_err() {
                            warn!("Failed to send on-demand processing request");
                        }
                    }
                }
            }
            _ => {
                // Other actions not relevant to vision
            }
        }
        Ok(())
    }

    fn subscribe_events(&self) -> broadcast::Receiver<WorldEvent> {
        self.event_sender.read()
            .as_ref()
            .map(|s| s.subscribe())
            .unwrap_or_else(|| {
                let (_, receiver) = broadcast::channel(1);
                receiver
            })
    }
}

