# Build, Test, and Launch Guide

## ✅ Build Status

The narayana-me package builds successfully with all features enabled.

## Building

```bash
# Build with Beyond Presence feature
cargo build --package narayana-me --features beyond-presence

# Build release version
cargo build --release --package narayana-me --features beyond-presence
```

## Testing

```bash
# Run all tests
cargo test --package narayana-me --features beyond-presence

# Run specific test
cargo test --package narayana-me --features beyond-presence --lib test_name

# Run integration tests
cargo test --package narayana-me --features beyond-presence --test integration_test
```

## Launching Examples

### Basic Avatar Example

```bash
# Set API key
export BEYOND_PRESENCE_API_KEY="your-api-key"

# Run example
cargo run --example basic_avatar --package narayana-me --features beyond-presence
```

The example will:
1. Create an AvatarBroker with Beyond Presence provider
2. Initialize the provider
3. Attempt to start a stream
4. Test setting expressions and emotions

**Note**: The example may fail if:
- API key is not set
- Beyond Presence API is not available
- Network connection is unavailable

This is expected - the example gracefully handles failures.

## Launching Full System

### 1. Backend (Rust)

```bash
# Set environment variable
export BEYOND_PRESENCE_API_KEY="your-api-key"

# Start narayana-server with avatar support
cargo run --package narayana-server --features beyond-presence

# Or build and run separately
cargo build --release --package narayana-server --features beyond-presence
./target/release/narayana-server
```

### 2. Frontend (React)

```bash
cd narayana-ui

# Install dependencies (if not already done)
npm install

# Start development server
npm run dev

# Or build for production
npm run build
npm run preview
```

### 3. Access

- Frontend: http://localhost:3000
- Avatar WebSocket: ws://localhost:8081/avatar/ws
- API: http://localhost:8080

## Configuration

### CPL Configuration

Enable avatar in CPL config:

```json
{
  "enable_avatar": true,
  "avatar_config": {
    "provider": "BeyondPresence",
    "expression_sensitivity": 0.8,
    "animation_speed": 1.0,
    "websocket_port": 8081,
    "enable_lip_sync": true,
    "enable_gestures": true,
    "avatar_id": "default_avatar_model"
  }
}
```

### Environment Variables

```bash
export BEYOND_PRESENCE_API_KEY="your-api-key"
export BEYOND_PRESENCE_BASE_URL="https://api.beyondpresence.ai/v1"  # optional
```

## Troubleshooting

### Build Errors

1. **Missing dependencies**: Run `cargo update`
2. **Feature not enabled**: Ensure `--features beyond-presence` is used
3. **Compilation errors**: Check Rust version (requires 1.70+)

### Runtime Errors

1. **API Key not set**: Set `BEYOND_PRESENCE_API_KEY` environment variable
2. **Network errors**: Check internet connection and API availability
3. **Port conflicts**: Change `websocket_port` in config or kill process using port 8081

### Test Failures

1. **Integration tests**: May require API key and network connection
2. **Unit tests**: Should pass without API key
3. **Mock tests**: Some tests may need mocking for external APIs

## Status

✅ **Build**: Compiles successfully
✅ **Tests**: All tests pass
✅ **Examples**: Run successfully (with graceful error handling)
✅ **Integration**: Ready for deployment

The system is ready for use! 🚀

