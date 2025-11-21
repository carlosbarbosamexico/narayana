# narayana-eye Test Suite

## Test Organization

### Unit Tests (in `src/`)
- `config.rs` - Configuration validation tests
- `error.rs` - Error type tests
- `utils.rs` - Utility function tests
- `processing/tracker.rs` - Object tracker tests

### Integration Tests (in `tests/`)
- `integration_test.rs` - Basic integration tests
- `security_test.rs` - Security-focused tests (overflow, NaN, etc.)
- `edge_cases_test.rs` - Edge case handling tests
- `model_manager_test.rs` - Model manager tests
- `performance_test.rs` - Performance and stress tests

## Running Tests

### Run all tests
```bash
cargo test --package narayana-eye
```

### Run specific test suite
```bash
# Unit tests only
cargo test --package narayana-eye --lib

# Integration tests only
cargo test --package narayana-eye --test '*'

# Security tests
cargo test --package narayana-eye --test security_test

# Edge case tests
cargo test --package narayana-eye --test edge_cases_test
```

### Run with output
```bash
cargo test --package narayana-eye -- --nocapture
```

## Test Coverage

### Configuration Tests
- ✅ Default configuration
- ✅ Valid configuration
- ✅ Invalid frame rates (0, >120)
- ✅ Invalid resolutions (0, too large, overflow)
- ✅ Invalid camera IDs
- ✅ Edge cases (min/max values)
- ✅ Serialization/deserialization

### Error Tests
- ✅ Error display formatting
- ✅ Error conversions (IO, Core)
- ✅ All error variants

### Tracker Tests
- ✅ Empty detections
- ✅ Single detection
- ✅ Multiple detections
- ✅ Tracking across frames
- ✅ Track aging
- ✅ IoU calculation (identical, no overlap, partial)
- ✅ Invalid inputs (NaN, Inf, negative)
- ✅ Memory limits
- ✅ ID collision avoidance
- ✅ Confidence validation
- ✅ Bbox validation

### Security Tests
- ✅ Overflow protection
- ✅ Division by zero protection
- ✅ NaN/Inf handling
- ✅ Negative dimension handling
- ✅ Memory limits
- ✅ ID overflow protection

### Edge Case Tests
- ✅ Empty inputs
- ✅ Single pixel bboxes
- ✅ Very large bboxes
- ✅ Zero IoU threshold
- ✅ Very high IoU threshold
- ✅ Zero max_age
- ✅ Rapid movement
- ✅ Identical detections
- ✅ Overlapping bboxes

### Performance Tests
- ✅ Many detections
- ✅ Many updates
- ✅ Memory efficiency

## Note on OpenCV Dependency

Some tests may require OpenCV to be installed. Tests that don't require OpenCV (config, error, tracker, utils) can run without it.

To install OpenCV:
- macOS: `brew install opencv`
- Ubuntu/Debian: `sudo apt-get install libopencv-dev`

See the main README for more details.


