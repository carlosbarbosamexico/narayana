# narayana-me: Virtual Avatar System

## Overview

`narayana-me` is a Rust crate that provides a unified interface for integrating virtual 3D avatar systems into the Narayana framework. The module implements a broker pattern to abstract differences between multiple avatar provider APIs, enabling seamless switching between providers without code changes.

## Architecture

### Core Components

#### AvatarBroker
The central facade component that provides a unified API for avatar operations. It manages provider lifecycle, stream initialization, and command routing. All provider-specific implementations are abstracted through the `AvatarProvider` trait.

#### AvatarProvider Trait
An asynchronous trait defining the standard interface that all avatar providers must implement:

```rust
async fn initialize(&mut self, config: &AvatarConfig) -> Result<(), AvatarError>;
async fn start_stream(&mut self) -> Result<AvatarStream, AvatarError>;
async fn stop_stream(&mut self) -> Result<(), AvatarError>;
async fn send_audio(&self, audio_data: Vec<u8>) -> Result<(), AvatarError>;
async fn set_expression(&self, expression: Expression, intensity: f64) -> Result<(), AvatarError>;
async fn set_gesture(&self, gesture: Gesture, duration_ms: u64) -> Result<(), AvatarError>;
async fn update_emotion(&self, emotion: Emotion, intensity: f64) -> Result<(), AvatarError>;
```

#### AvatarConfig
Configuration structure containing:
- `enabled`: Boolean flag to enable/disable avatar rendering
- `provider`: Provider type selection (enum)
- `provider_config`: Optional JSON for provider-specific settings
- `expression_sensitivity`: Floating point multiplier (0.0-1.0) for expression intensity
- `animation_speed`: Speed multiplier (0.5-2.0) for animation playback
- `websocket_port`: Port for WebSocket bridge (optional, auto-assigned if None)
- `enable_lip_sync`: Boolean flag for lip synchronization
- `enable_gestures`: Boolean flag for gesture support
- `avatar_id`: Provider-specific avatar model identifier

### Supported Providers

The module supports multiple avatar providers through feature flags:

1. **Beyond Presence** (`beyond-presence` feature)
   - Hyper-realistic avatars with frame-accurate lip sync
   - API endpoint: configurable via `BEYOND_PRESENCE_BASE_URL`
   - Authentication: `BEYOND_PRESENCE_API_KEY` environment variable

2. **LiveAvatar** (`live-avatar` feature)
   - Real-time avatar rendering
   - API endpoint: configurable via `LIVE_AVATAR_BASE_URL`
   - Authentication: `LIVE_AVATAR_API_KEY` environment variable

3. **Ready Player Me** (`ready-player-me` feature)
   - Customizable avatar platform
   - API endpoint: configurable via `READY_PLAYER_ME_BASE_URL`
   - Authentication: `READY_PLAYER_ME_API_KEY` environment variable

4. **Avatar SDK** (`avatar-sdk` feature)
   - Selfie-based avatar generation
   - API endpoint: configurable via `AVATAR_SDK_BASE_URL`
   - Authentication: `AVATAR_SDK_API_KEY` environment variable

5. **Open Avatar Chat** (`open-avatar-chat` feature)
   - Open-source avatar platform
   - API endpoint: configurable via `OPEN_AVATAR_CHAT_BASE_URL`
   - Authentication: `OPEN_AVATAR_CHAT_API_KEY` environment variable (optional)

### AvatarBridge

A WebSocket server component that broadcasts avatar state updates to connected clients. The bridge:
- Accepts connections on a configurable port (default: 8081)
- Broadcasts avatar commands (expressions, gestures, state changes)
- Limits concurrent connections to 10,000 clients
- Implements message size validation (1MB maximum per message)
- Supports automatic cleanup of disconnected clients

### AvatarAdapter

Implements the `ProtocolAdapter` trait from `narayana-wld` to integrate avatar commands with the World Action system. The adapter:
- Listens to `WorldAction` events from the CPL
- Extracts avatar configuration from CPL settings
- Translates world actions into avatar commands
- Handles emotion-to-expression mapping

## Configuration

### Basic Configuration

```rust
use narayana_me::{AvatarConfig, AvatarProviderType};

let config = AvatarConfig {
    enabled: true,
    provider: AvatarProviderType::BeyondPresence,
    expression_sensitivity: 0.7,
    animation_speed: 1.0,
    enable_lip_sync: true,
    enable_gestures: true,
    avatar_id: Some("avatar-123".to_string()),
    ..Default::default()
};
```

### Provider-Specific Configuration

Provider-specific settings can be passed via `provider_config`:

```rust
use serde_json::json;

let config = AvatarConfig {
    provider: AvatarProviderType::BeyondPresence,
    provider_config: Some(json!({
        "quality": "high",
        "frame_rate": 30,
        "resolution": "1080p"
    })),
    ..Default::default()
};
```

### Validation

Configuration is validated on broker creation:
- `expression_sensitivity`: Must be in range [0.0, 1.0]
- `animation_speed`: Must be in range [0.5, 2.0]
- `websocket_port`: Must be in range [1, 65535] if specified
- `avatar_id`: Maximum 256 characters, alphanumeric plus dash/underscore/dot
- `provider_config`: Maximum 100KB, maximum nesting depth of 32 levels

## Usage

### Initialization

```rust
use narayana_me::AvatarBroker;

// Create broker with configuration
let broker = AvatarBroker::new(config)?;

// Initialize the selected provider
broker.initialize().await?;

// Start avatar stream
let client_url = broker.start_stream().await?;
// client_url: "ws://localhost:8081/avatar/stream/{stream_id}"
```

### Controlling Avatar Expressions

```rust
use narayana_me::{Expression, Gesture, Emotion};

// Set facial expression with intensity (0.0-1.0)
broker.set_expression(Expression::Happy, 0.8).await?;

// Set gesture with duration in milliseconds
broker.set_gesture(Gesture::Wave, 2000).await?;

// Update emotion (automatically maps to expression)
broker.update_emotion(Emotion::Joy, 0.9).await?;
```

### Audio/Lip Sync

```rust
// Send audio data for lip synchronization
let audio_data: Vec<u8> = /* ... */;
broker.send_audio(audio_data).await?;
```

### Available Expressions

- `Neutral`: Resting face
- `Happy`: Smiling expression
- `Sad`: Frowning expression
- `Angry`: Angry expression
- `Surprised`: Surprised expression
- `Thinking`: Contemplative expression
- `Confused`: Confused expression
- `Excited`: Excited expression
- `Tired`: Sleepy expression
- `Recognition`: Understanding/recognition expression
- `Custom(String)`: Provider-specific custom expression

### Available Gestures

- `None`: No gesture
- `Wave`: Hand wave
- `Point`: Pointing gesture
- `Nod`: Head nod
- `Shake`: Head shake
- `ThumbsUp`: Thumbs up gesture
- `Custom(String)`: Provider-specific custom gesture

### Available Emotions

Emotions are automatically mapped to expressions:
- `Joy` → `Happy`
- `Sadness` → `Sad`
- `Anger` → `Angry`
- `Surprise` → `Surprised`
- `Disgust` → `Confused`
- `Neutral` → `Neutral`
- `Recognition` → `Recognition`
- `Thinking` → `Thinking`
- `Interest` → `Thinking`
- Others → `Neutral`

## Security Considerations

### Input Validation

All inputs are validated with strict limits:
- Audio data: Maximum 10MB per transmission
- Expression/Gesture strings: Maximum 256 characters
- Intensity values: Clamped to valid ranges, validated for NaN/Infinity
- Gesture duration: Maximum 300 seconds (5 minutes)
- URL parameters: Percent-encoded to prevent injection
- Provider config: Size and depth limits to prevent DoS

### Resource Limits

- WebSocket connections: Maximum 10,000 concurrent clients
- Message size: Maximum 1MB per WebSocket message
- API response size: Maximum 100KB per response
- Error text size: Maximum 10KB per error response
- Request timeouts: 10-30 seconds depending on operation

### Error Handling

All operations return `Result<T, AvatarError>`:
- `AvatarError::Broker`: Broker-level errors (initialization, state)
- `AvatarError::Provider`: Provider-specific errors
- `AvatarError::Api`: API communication errors
- `AvatarError::Network`: Network connectivity errors
- `AvatarError::Config`: Configuration validation errors
- `AvatarError::Stream`: Stream management errors

Operations are idempotent where applicable (e.g., `initialize()`, `start_stream()` can be called multiple times safely).

## Integration with CPL

The module integrates with Conscience Persistent Loop (CPL) through the `AvatarAdapter`:

1. CPL configuration includes avatar settings:
   ```rust
   struct CPLConfig {
       // ... other fields
       enable_avatar: bool,
       avatar_config: Option<AvatarConfig>,
   }
   ```

2. `AvatarAdapter` listens to world actions and translates them to avatar commands

3. Configuration is extracted via `avatar_config_from_cpl()` helper function

## WebSocket Protocol

The AvatarBridge exposes a WebSocket endpoint at `/avatar/ws` (default port 8081). Clients receive JSON messages:

```json
{
  "type": "expression",
  "expression": "happy",
  "intensity": 0.8
}
```

```json
{
  "type": "gesture",
  "gesture": "wave",
  "duration_ms": 2000
}
```

```json
{
  "type": "state",
  "stream_id": "stream-123",
  "status": "active"
}
```

## Thread Safety

All public APIs are designed for concurrent use:
- `AvatarBroker` methods use `Arc<RwLock<>>` internally for thread-safe access
- Provider instances are wrapped in `Arc<RwLock<>>` for shared ownership
- All async operations are `Send` and `Sync` compatible

## Error Recovery

The system implements graceful degradation:
- Failed provider initialization returns errors but doesn't crash
- Stream stop operations are idempotent (safe to call when already stopped)
- WebSocket connection failures are logged but don't prevent other operations
- Provider API failures are logged with warnings but don't crash the broker

## Performance Characteristics

- Provider initialization: O(1) after configuration validation
- Stream start: Network-bound (depends on provider API latency)
- Expression/Gesture updates: O(1) once stream is active
- Audio transmission: O(n) where n is audio data size
- Broadcast operations: O(k) where k is number of connected clients (max 10,000)

## Dependencies

Required crates:
- `async-trait`: For async trait definitions
- `reqwest`: HTTP client for provider APIs
- `tokio`: Async runtime
- `tokio-tungstenite`: WebSocket support
- `serde`: Serialization/deserialization
- `thiserror`: Error handling
- `tracing`: Structured logging
- `percent-encoding`: URL encoding
- `bytes`: Byte buffer handling
- `axum`: WebSocket server framework (for bridge)

## Limitations

1. Provider availability depends on feature flags enabled at compile time
2. Real-time performance is limited by network latency to provider APIs
3. Maximum 10,000 concurrent WebSocket clients
4. Audio format compatibility depends on provider (typically WAV expected)
5. Custom expressions/gestures are provider-specific and may not be portable

## Testing

The module includes unit tests for:
- Configuration validation
- Expression/Gesture/Emotion serialization
- Provider type selection
- Broker state management

Integration tests verify provider initialization and basic operations. Tests require feature flags to be enabled for the respective providers.

## API Reference

See Rust documentation generated via `cargo doc`:

```bash
cargo doc --package narayana-me --features beyond-presence --open
```

Main public exports:
- `AvatarBroker`: Main broker interface
- `AvatarConfig`: Configuration structure
- `AvatarProviderType`: Provider type enum
- `Expression`, `Gesture`, `Emotion`: Enum types for avatar control
- `AvatarError`: Error type hierarchy
- `AvatarBridge`: WebSocket bridge server
- `AvatarAdapter`: CPL integration adapter

## Versioning

The module follows semantic versioning. Breaking changes to the public API will increment the major version number.

