# Test Coverage for narayana-me

## Test Suites

### 1. Integration Tests (`tests/integration_test.rs`)
Comprehensive integration tests covering:
- ✅ Configuration validation (edge cases, boundaries)
- ✅ Port validation
- ✅ Avatar ID validation
- ✅ Provider config validation
- ✅ Broker creation and lifecycle
- ✅ Idempotent operations
- ✅ Expression/gesture/emotion mapping
- ✅ Serialization/deserialization
- ✅ Disabled features (no-op behavior)

**Test Count**: 30+ tests

### 2. Concurrent Tests (`tests/concurrent_test.rs`)
Concurrent access and race condition tests:
- ✅ Concurrent broker creation
- ✅ Concurrent expression updates
- ✅ Concurrent gesture updates
- ✅ Concurrent get_client_url calls
- ✅ Concurrent stop_stream calls (idempotent)

**Test Count**: 5 tests

### 3. Error Handling Tests (`tests/error_handling_test.rs`)
Error handling and edge cases:
- ✅ Invalid configuration errors
- ✅ Operations without initialization
- ✅ Oversized audio rejection
- ✅ Invalid intensity values (NaN, Infinity)
- ✅ Unsupported provider errors
- ✅ Feature flag validation

**Test Count**: 10+ tests

### 4. Unit Tests (`src/avatar_broker.rs::tests`)
Internal unit tests:
- ✅ Broker internal structure
- ✅ Mock provider integration

**Test Count**: 1+ tests

## Coverage Areas

### Configuration Validation
- ✅ Expression sensitivity bounds (0.0-1.0)
- ✅ Animation speed bounds (0.5-2.0)
- ✅ WebSocket port validation (1-65535)
- ✅ Avatar ID validation (length, characters)
- ✅ Provider config validation (type, size, depth)

### Broker Lifecycle
- ✅ Creation with valid/invalid config
- ✅ Initialization (enabled/disabled)
- ✅ Idempotent initialization
- ✅ Stream start/stop
- ✅ Idempotent stream operations

### Expression/Gesture/Emotion
- ✅ Expression mapping from emotions
- ✅ Intensity clamping
- ✅ Duration limits
- ✅ Serialization/deserialization
- ✅ Custom expressions/gestures

### Error Handling
- ✅ Invalid input validation
- ✅ Uninitialized provider errors
- ✅ Oversized data rejection
- ✅ Invalid numeric values (NaN, Infinity)
- ✅ Unsupported provider errors

### Concurrent Access
- ✅ Thread-safe operations
- ✅ Concurrent updates
- ✅ No race conditions
- ✅ Idempotent operations under concurrency

## Running Tests

```bash
# Run all tests
cargo test --package narayana-me --features beyond-presence

# Run specific test suite
cargo test --package narayana-me --features beyond-presence --test integration_test
cargo test --package narayana-me --features beyond-presence --test concurrent_test
cargo test --package narayana-me --features beyond-presence --test error_handling_test

# Run unit tests
cargo test --package narayana-me --features beyond-presence --lib

# Run with output
cargo test --package narayana-me --features beyond-presence -- --nocapture

# Run single test
cargo test --package narayana-me --features beyond-presence test_name
```

## Test Status

✅ **All tests passing**
- Integration tests: ✅
- Concurrent tests: ✅
- Error handling tests: ✅
- Unit tests: ✅

## Future Test Additions

- [ ] Mock provider tests (with full mock implementation)
- [ ] Bridge WebSocket tests
- [ ] Adapter integration tests
- [ ] CPL integration tests
- [ ] Performance/benchmark tests
- [ ] End-to-end tests with real API (optional)

