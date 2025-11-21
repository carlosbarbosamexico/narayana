# Test Suite for narayana-sc

## Test Files

### 1. `security_test.rs`
**Purpose**: Security-focused tests for input validation, resource limits, and edge cases.

**Coverage**:
- Empty audio data handling
- Invalid audio length handling
- Oversized audio data handling
- Configuration validation
- NaN/Inf handling
- Buffer size validation
- Sample rate validation

**Tests**: 7 tests

### 2. `unit_tests.rs`
**Purpose**: Unit tests for individual components and functions.

**Coverage**:
- AudioConfig creation and validation
- CaptureConfig creation and validation
- AnalysisConfig creation and validation
- AudioAnalyzer creation and basic operations
- LlmAudioProcessor creation
- Error type testing
- Configuration serialization

**Tests**: ~15 tests

### 3. `integration_tests.rs`
**Purpose**: Integration tests for component interactions and full pipelines.

**Coverage**:
- AudioAdapter creation from config
- Full audio analysis pipeline
- Configuration validation chain
- Different sample rates
- Different window sizes
- Silence detection
- Noise analysis

**Tests**: ~8 tests

### 4. `edge_case_tests.rs`
**Purpose**: Tests for edge cases and boundary conditions.

**Coverage**:
- Minimum/maximum valid audio sizes
- Oversized audio rejection
- NaN/Inf value handling
- Extreme value handling
- Boundary value testing for configs
- Empty spectrum handling
- Single frequency detection

**Tests**: ~12 tests

### 5. `performance_tests.rs`
**Purpose**: Performance and throughput tests.

**Coverage**:
- Analysis performance benchmarks
- Throughput testing
- Config validation performance
- Large buffer processing

**Tests**: ~4 tests

### 6. `advanced_features_tests.rs`
**Purpose**: Tests for advanced audio processing features.

**Coverage**:
- AdvancedAudioProcessor creation
- Voice activity detection
- ComprehensiveAudioCapture creation
- Streaming buffer operations
- Event-based processing
- Adaptive stream controller

**Tests**: ~6 tests

## Running Tests

### Run all tests
```bash
cargo test --package narayana-sc
```

### Run specific test file
```bash
cargo test --package narayana-sc --test security_test
cargo test --package narayana-sc --test unit_tests
cargo test --package narayana-sc --test integration_tests
cargo test --package narayana-sc --test edge_case_tests
cargo test --package narayana-sc --test performance_tests
cargo test --package narayana-sc --test advanced_features_tests
```

### Run with output
```bash
cargo test --package narayana-sc -- --nocapture
```

### Run specific test
```bash
cargo test --package narayana-sc test_name
```

## Test Coverage

**Total Tests**: ~52 tests covering:
- ✅ Security vulnerabilities
- ✅ Input validation
- ✅ Edge cases
- ✅ Boundary conditions
- ✅ Integration scenarios
- ✅ Performance benchmarks
- ✅ Advanced features

## Test Status

All tests are designed to:
- ✅ Pass when code is correct
- ✅ Fail when bugs are introduced
- ✅ Cover security-critical paths
- ✅ Validate edge cases
- ✅ Measure performance

**Note**: Some tests may be skipped or show warnings if audio hardware is not available (e.g., AudioAdapter tests). This is expected behavior.

