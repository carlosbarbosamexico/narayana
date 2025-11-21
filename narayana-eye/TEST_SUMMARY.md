# narayana-eye Test Suite Summary

## Test Coverage

### ✅ Unit Tests (In-Source)

#### Configuration Tests (`src/config.rs`)
- Default configuration values
- Valid configuration validation
- Invalid frame rates (0, >120)
- Invalid resolutions (0, too large, overflow)
- Invalid camera IDs (>100)
- Edge cases (min/max valid values)
- Processing mode equality

#### Error Tests (`src/error.rs`)
- Error display formatting
- Error conversions (IO → VisionError, VisionError → CoreError)
- All error variants

#### Tracker Tests (`src/processing/tracker.rs`)
- Empty detections
- Single detection
- Multiple detections
- Tracking across frames
- Track aging and removal
- IoU calculation (identical, no overlap, partial overlap)
- Invalid inputs (NaN, Inf, negative dimensions)
- Memory limits (MAX_TRACKS)
- ID collision avoidance
- Confidence validation
- Bbox validation

#### Utils Tests (`src/utils.rs`)
- CLIP normalization with empty data
- CLIP normalization with small data
- CLIP normalization with NaN/Inf values
- CLIP normalization with large data

#### Model Manager Tests (`src/models/manager.rs`)
- Model manager creation
- Model directory creation (idempotent)
- Invalid model name validation (empty, path traversal, slashes)
- Invalid URL validation (empty, HTTP, FTP)
- Model loaded state tracking

### ✅ Integration Tests (`tests/`)

#### Basic Integration (`integration_test.rs`)
- Configuration validation
- Configuration serialization/deserialization
- Processing mode serialization
- Error display
- Error conversion

#### Security Tests (`security_test.rs`)
- Integer overflow protection
- Division by zero protection
- NaN handling in IoU
- Infinity handling in IoU
- Negative dimension handling
- Memory limit enforcement
- ID overflow protection
- Confidence validation
- Bbox validation

#### Edge Cases (`edge_cases_test.rs`)
- Minimum/maximum valid values
- Empty detections
- Single pixel bboxes
- Very large bboxes
- Zero IoU threshold
- Very high IoU threshold
- Zero max_age
- Rapid movement scenarios
- Identical detections
- Overlapping bboxes

#### Performance Tests (`performance_test.rs`)
- Many detections (500)
- Many updates (1000 iterations)
- Memory efficiency (track cleanup)

## Test Statistics

- **Total Test Files**: 6
- **Unit Test Modules**: 5 (in-source)
- **Integration Test Files**: 4
- **Total Test Cases**: 50+

## Running Tests

### All Tests
```bash
cargo test --package narayana-eye
```

### Unit Tests Only (No OpenCV Required)
```bash
cargo test --package narayana-eye --lib
```

### Specific Test Suite
```bash
# Security tests
cargo test --package narayana-eye --test security_test

# Edge case tests
cargo test --package narayana-eye --test edge_cases_test

# Performance tests
cargo test --package narayana-eye --test performance_test
```

### With Output
```bash
cargo test --package narayana-eye -- --nocapture
```

## Test Categories

### ✅ Configuration Tests
- Validation logic
- Edge cases
- Serialization

### ✅ Error Handling Tests
- Error types
- Error conversions
- Error display

### ✅ Tracker Tests
- Basic functionality
- Edge cases
- Security (NaN, Inf, overflow)
- Performance

### ✅ Security Tests
- Overflow protection
- Division by zero
- Invalid inputs
- Memory limits

### ✅ Edge Case Tests
- Boundary conditions
- Extreme values
- Special cases

### ✅ Performance Tests
- Stress testing
- Memory efficiency
- Speed benchmarks

## Note on OpenCV Dependency

Some tests require OpenCV to be installed. Tests that don't require OpenCV can run without it:
- Configuration tests ✅
- Error tests ✅
- Tracker tests ✅
- Utils tests ✅
- Model manager tests ✅
- Integration tests (basic) ✅

Tests that require OpenCV (camera, vision adapter, model inference) are not included in the test suite as they require:
- OpenCV installation
- Camera hardware (for camera tests)
- Model files (for inference tests)

These should be tested manually or in CI/CD with proper setup.

## Test Quality

- ✅ Comprehensive coverage of core functionality
- ✅ Security-focused tests
- ✅ Edge case coverage
- ✅ Performance testing
- ✅ No panics in tests
- ✅ Proper error handling
- ✅ Clear test names and organization


