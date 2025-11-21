# Test Coverage Summary for narayana-eye

This document summarizes all tests written for the `narayana-eye` crate.

## Test Files

### Unit Tests (in-source)

1. **`src/config.rs`** - Configuration validation and serialization tests
   - Default config values
   - Valid/invalid frame rates, resolutions, camera IDs
   - Integer overflow protection
   - Serialization/deserialization

2. **`src/error.rs`** - Error type tests
   - Error display formatting
   - Error conversion (IO, Core errors)
   - Error propagation

3. **`src/models/manager.rs`** - Model manager tests
   - Model directory creation
   - Invalid model names (path traversal protection)
   - Invalid URLs (HTTPS-only enforcement)
   - Model loading tracking

4. **`src/processing/tracker.rs`** - Object tracker tests
   - Tracker initialization
   - Detection matching and tracking
   - Track aging and retention
   - ID generation and wrapping
   - IoU computation (overlapping, non-overlapping, zero-area, invalid inputs)
   - Max tracks limit

5. **`src/utils.rs`** - Image preprocessing utility tests
   - `mat_to_rgb_tensor` (valid inputs, invalid dimensions, overflow)
   - `mat_to_chw_tensor` (channel-first conversion)
   - `apply_clip_normalization` (normalization, NaN/Inf handling)

### Integration Tests (`tests/`)

1. **`camera_test.rs`** - Camera manager tests
   - CameraManager creation
   - Initialization structure
   - Start stream structure
   - Capture frame structure
   - Stop functionality
   - Double initialization handling
   - Double start stream prevention

2. **`pipeline_test.rs`** - Detection and segmentation pipeline tests
   - DetectionPipeline structure verification
   - SegmentationPipeline structure verification
   - Pipeline type accessibility

3. **`scene_analyzer_test.rs`** - Scene analyzer tests
   - SceneAnalyzer structure verification
   - SceneDescription structure and validation
   - SceneEmbedding structure
   - TrackedObject structure
   - Empty tags handling
   - Serialization readiness

4. **`integration_test.rs`** - Core integration tests
   - Config validation and serialization
   - Processing mode serialization
   - Error display and conversion
   - VisionAdapter initialization
   - VisionAdapter start/stop (RealTime and OnDemand modes)
   - Double start prevention
   - ProtocolAdapter trait implementation
   - LLM manager integration
   - Event subscription

2. **`download_model_test.rs`** - Download model binary tests
   - Model download helper for YOLO, SAM, CLIP
   - Case-insensitive model names
   - Unknown model error handling
   - Empty string handling
   - Model manager method existence

3. **`security_test.rs`** - Security-focused tests
   - Config resolution overflow protection
   - Camera ID limits
   - Model manager path traversal exploits
   - Large/small file exploits
   - Object tracker ID wrapping
   - Max tracks memory exhaustion protection

4. **`edge_cases_test.rs`** - Edge case and boundary condition tests
   - Config min/max valid values
   - Empty detections
   - Zero-area bounding boxes
   - Invalid float inputs (NaN, Inf, negative)
   - Zero/negative dimensions
   - Empty RGB data
   - CLIP normalization edge cases
   - Model manager checksum validation

5. **`performance_test.rs`** - Performance benchmarks
   - Config validation performance (10,000 iterations)
   - Object tracker with many detections (1,000)
   - Object tracker with max tracks and many detections

6. **`model_manager_test.rs`** - Placeholder (tests moved to `src/models/manager.rs`)

## Test Statistics

- **Unit tests**: ~50+ tests across 5 modules
- **Integration tests**: ~50+ tests across 9 test files
- **Total**: ~100+ tests

## Test Categories

### Functionality Tests
- Configuration management
- Model downloading and management
- Object detection and tracking
- Image preprocessing
- Vision adapter lifecycle

### Security Tests
- Input validation
- Path traversal protection
- URL validation (HTTPS-only)
- File size limits
- Integer overflow prevention
- Memory exhaustion protection

### Edge Case Tests
- Boundary conditions
- Empty inputs
- Invalid inputs (NaN, Inf, negative)
- Zero dimensions
- Maximum limits

### Performance Tests
- Large-scale operations
- Repeated operations
- Memory usage under load

## Running Tests

```bash
# Run all unit tests (no OpenCV required)
cargo test --package narayana-eye --lib

# Run all integration tests (requires OpenCV)
cargo test --package narayana-eye --test '*'

# Run specific test file
cargo test --package narayana-eye --test integration_test

# Run with output
cargo test --package narayana-eye --lib -- --nocapture
```

## Note on OpenCV Dependency

Some tests require OpenCV to be installed. Tests that don't require OpenCV can be run with:
```bash
cargo test --package narayana-eye --lib
```

Tests that require OpenCV (integration tests, camera tests) will fail if OpenCV is not installed. See `narayana-eye/README.md` for OpenCV installation instructions.

## Coverage Areas

✅ Configuration validation and serialization  
✅ Error handling and propagation  
✅ Model downloading and management  
✅ Object tracking  
✅ Image preprocessing utilities  
✅ Vision adapter lifecycle  
✅ Camera management  
✅ Detection and segmentation pipelines  
✅ Scene analysis and understanding  
✅ Security hardening  
✅ Edge cases and boundary conditions  
✅ Performance under load  

## Missing Coverage (Future Work)

- Camera capture tests (requires physical camera or mock)
- Model inference tests (requires downloaded models)
- Scene analysis tests (requires CLIP model)
- LLM integration tests (requires LLM service)
- End-to-end vision pipeline tests (requires all components)

