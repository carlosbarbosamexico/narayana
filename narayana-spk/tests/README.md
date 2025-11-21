# narayana-spk Test Suite

This directory contains comprehensive tests for the `narayana-spk` speech synthesis package.

## Test Files

### `config_test.rs`
Tests for configuration validation, including:
- Default values
- Rate, volume, and pitch validation
- API configuration validation
- Voice configuration validation

### `cpl_integration_test.rs`
Tests for Conscience Persistent Loop (CPL) integration:
- Speech config extraction from CPL
- JSON deserialization
- Default fallback behavior

### `integration_test.rs`
Integration tests for the speech adapter:
- Adapter creation and lifecycle
- Protocol adapter interface
- World action handling

### `security_test.rs`
Security-focused tests:
- Input length limits
- Path traversal prevention
- Cache size limits
- API endpoint validation
- Retry configuration validation
- Language and voice name validation

### `edge_cases_test.rs`
Edge case and boundary condition tests:
- Boundary values (min/max)
- Just over/under boundaries
- Unicode handling
- Special characters
- Empty values
- Maximum length values

### `synthesizer_test.rs`
Tests for the speech synthesizer:
- Creation with various configs
- Text validation
- Cache key generation
- UTF-8 boundary safety
- Audio size limits

### `input_validation_test.rs`
Input validation and sanitization tests:
- Command injection prevention
- XSS prevention
- Null byte injection
- Oversized input handling
- Unicode text handling
- UTF-8 boundary truncation

### `cache_test.rs`
Cache management tests:
- Cache size limits
- Cache cleanup triggers
- Cache entry size tracking
- Cache key generation consistency
- Invalid entry removal
- Timestamp ordering (LRU)

### `error_handling_test.rs`
Error handling and recovery tests:
- Error type display
- Error propagation
- Validation error messages
- Error recovery
- Multiple validation errors

### `performance_test.rs`
Performance and resource usage tests:
- Config validation performance
- Large text validation performance
- Cache key generation performance
- Memory usage limits
- Concurrent access safety
- String operations performance
- Resource limits enforcement

### `concurrency_test.rs`
Concurrency and thread safety tests:
- Thread safety of config
- Concurrent adapter creation
- Concurrent validation
- Deadlock prevention
- Send/Sync trait implementation
- Concurrent error handling

## Running Tests

Run all tests:
```bash
cargo test --package narayana-spk
```

Run specific test file:
```bash
cargo test --package narayana-spk --test security_test
cargo test --package narayana-spk --test edge_cases_test
cargo test --package narayana-spk --test input_validation_test
```

Run with output:
```bash
cargo test --package narayana-spk -- --nocapture
```

## Test Coverage

The test suite covers:
- ✅ Configuration validation (6 tests)
- ✅ Input sanitization (14 tests)
- ✅ Security vulnerabilities (injection, XSS, path traversal) (21 tests)
- ✅ Resource limits (memory, cache, queue) (10 tests)
- ✅ Edge cases and boundary conditions (17 tests)
- ✅ Error handling (12 tests)
- ✅ UTF-8 safety (multiple tests)
- ✅ Integer overflow protection (multiple tests)
- ✅ Command injection prevention (14 tests)
- ✅ Cache management (10 tests)
- ✅ Performance characteristics (10 tests)
- ✅ Concurrency and thread safety (7 tests)
- ✅ CPL integration (3 tests)
- ✅ Synthesizer functionality (13 tests)
- ✅ Integration with narayana-wld (7 tests)

**Total: 130 tests across 11 test files**

## Notes

- Some tests may require TTS engines to be available (native TTS)
- Tests are designed to pass even when engines are unavailable
- Security tests verify that malicious input is handled safely
- Edge case tests ensure the system handles unusual but valid inputs

