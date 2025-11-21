# narayana-eye: Vision Interface for narayana-wld

A state-of-the-art machine vision system that provides object detection, segmentation, tracking, and scene understanding capabilities for robots. Integrates seamlessly with narayana-wld as a protocol adapter to provide vision events to the cognitive system.

## Overview

narayana-eye implements a complete vision pipeline using 2025 state-of-the-art models:

- **Object Detection**: YOLO v8/v9 for real-time object detection with COCO classes
- **Instance Segmentation**: SAM (Segment Anything Model) for precise object segmentation
- **Object Tracking**: Multi-object tracking across frames using IoU-based matching
- **Scene Understanding**: CLIP for semantic scene understanding and analysis
- **LLM Integration**: Optional brain-controlled scene description enhancement

## Features

- **USB Webcam Support**: Direct integration with USB cameras via OpenCV/v4l2
- **Real-time Processing**: Continuous frame processing with configurable frame rates
- **On-demand Processing**: Process frames only when requested
- **Auto-download Models**: Automatically downloads pre-trained models on first use
- **Brain Integration**: Optional LLM integration for enhanced scene descriptions
- **Event-driven**: Emits vision events to narayana-wld for cognitive processing

## Architecture

### Components

1. **VisionAdapter**: Protocol adapter implementing `ProtocolAdapter` trait for narayana-wld
2. **CameraManager**: Handles USB webcam capture and frame streaming
3. **ModelManager**: Manages model loading and auto-download
4. **DetectionPipeline**: YOLO-based object detection
5. **SegmentationPipeline**: SAM-based instance segmentation
6. **ObjectTracker**: Multi-object tracking across frames
7. **SceneAnalyzer**: CLIP-based scene understanding with optional LLM enhancement

### Event Flow

**Camera → Vision → World:**
1. Camera captures frame
2. Vision pipeline processes frame (detection, segmentation, tracking, scene understanding)
3. VisionAdapter emits `WorldEvent::SensorData` with vision data
4. Events flow to narayana-wld for cognitive processing

## Usage

### Basic Setup

```rust
use narayana_eye::{VisionAdapter, VisionConfig, ProcessingMode};
use narayana_wld::{WorldBroker, WorldBrokerConfig};
use narayana_storage::cognitive::CognitiveBrain;
use narayana_storage::conscience_persistent_loop::{ConsciencePersistentLoop, CPLConfig};
use std::sync::Arc;

// Create cognitive brain and CPL
let brain = Arc::new(CognitiveBrain::new());
let cpl_config = CPLConfig::default();
let cpl = Arc::new(ConsciencePersistentLoop::new(brain.clone(), cpl_config));

// Create vision configuration
let vision_config = VisionConfig {
    camera_id: 0,
    frame_rate: 30,
    resolution: (640, 480),
    enable_detection: true,
    enable_segmentation: false,
    enable_tracking: true,
    enable_scene_understanding: true,
    llm_integration: false,
    model_path: std::path::PathBuf::from("./models"),
    processing_mode: ProcessingMode::RealTime,
};

// Create vision adapter
let vision_adapter = Box::new(VisionAdapter::new(vision_config)?);

// Create world broker
let config = WorldBrokerConfig::default();
let broker = WorldBroker::new(brain, cpl, config)?;

// Register vision adapter
broker.register_adapter(vision_adapter);

// Start broker (this will start vision processing)
broker.start().await?;
```

### With LLM Integration

```rust
use narayana_llm::LLMManager;

// Create LLM manager
let llm_manager = Arc::new(LLMManager::new());
// Configure LLM provider (e.g., OpenAI, Anthropic, etc.)
// ...

// Create vision adapter with LLM integration
let mut vision_config = VisionConfig::default();
vision_config.llm_integration = true;

let mut vision_adapter = VisionAdapter::new(vision_config)?;
vision_adapter.set_llm_manager(Some(llm_manager));

// Register and start as above
```

## Configuration

### VisionConfig

```rust
pub struct VisionConfig {
    pub camera_id: u32,                    // USB camera index (0, 1, 2, ...)
    pub frame_rate: u32,                    // Target FPS (1-120)
    pub resolution: (u32, u32),             // Camera resolution (width, height)
    pub enable_detection: bool,             // Enable object detection
    pub enable_segmentation: bool,         // Enable instance segmentation
    pub enable_tracking: bool,              // Enable object tracking
    pub enable_scene_understanding: bool,  // Enable scene understanding
    pub llm_integration: bool,              // Enable LLM enhancement (brain-controlled)
    pub model_path: PathBuf,               // Path to store models
    pub processing_mode: ProcessingMode,    // RealTime or OnDemand
}
```

### Processing Modes

- **RealTime**: Continuous processing, emits events at configured frame rate
- **OnDemand**: Processes frames only when requested via API/command

## Models

Pre-trained models are automatically downloaded on first use:

- **YOLO v8**: Object detection (COCO classes)
- **SAM**: Instance segmentation
- **CLIP**: Scene understanding

Models are stored in `~/.narayana/models/` by default (configurable via `model_path`).

## Vision Events

Vision events are emitted as `WorldEvent::SensorData` with the following structure:

```json
{
  "timestamp": 1234567890,
  "camera_id": 0,
  "detections": [
    {
      "class_id": 0,
      "class_name": "person",
      "confidence": 0.95,
      "bbox": [100, 200, 150, 300]
    }
  ],
  "tracks": [
    {
      "id": 1,
      "class_id": 0,
      "class_name": "person",
      "confidence": 0.95,
      "bbox": [100, 200, 150, 300],
      "age": 5
    }
  ],
  "masks": [
    {
      "bbox": [100, 200, 150, 300],
      "confidence": 0.95
    }
  ],
  "scene": {
    "description": "Scene contains 2 objects:\n- person (confidence: 0.95)\n- car (confidence: 0.87)",
    "confidence": 0.82,
    "tags": ["person", "car"]
  }
}
```

## Integration with narayana-wld

narayana-eye integrates with narayana-wld as a protocol adapter:

1. Implements `ProtocolAdapter` trait
2. Emits vision events via `WorldEvent::SensorData`
3. Receives camera control commands via `WorldAction::ActuatorCommand`
4. Events flow through sensory interface to cognitive brain

## Dependencies

- **OpenCV**: Camera capture and image processing
- **ONNX Runtime**: Model inference (YOLO, SAM, CLIP)
- **tokio**: Async runtime
- **narayana-wld**: World broker integration
- **narayana-llm**: Optional LLM integration

## Requirements

### System Dependencies

- **OpenCV**: Required for camera capture and image processing
  - macOS: `brew install opencv`
  - Ubuntu/Debian: `sudo apt-get install libopencv-dev`
  - See [opencv-rust documentation](https://github.com/twistedfall/opencv-rust#getting-opencv) for details

- **ONNX Runtime**: Required for model inference
  - Automatically handled by the `ort` crate, but may require system libraries
  - See [ort documentation](https://github.com/pykeio/ort) for details

### Hardware Requirements

- USB webcam connected
- Sufficient RAM for model loading (models are loaded into memory)
- CPU with AVX support recommended for better performance
- GPU support optional (can be enabled via ONNX Runtime providers)

## Future Enhancements

- GPU acceleration support
- Multiple camera support
- Video file input
- IP camera support (RTSP/HTTP streams)
- Custom model loading
- Advanced tracking algorithms (DeepSORT, etc.)

## License

Apache-2.0 (same as narayana project)

