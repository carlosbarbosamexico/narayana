# narayana-me: 3D Virtual Avatar for CPL

Renders a realistic 3D virtual avatar representing a Conscience Persistent Loop (CPL), enabling users to visually interact with their cognitive agents.

## Features

- **Unified Avatar API**: Pluggable provider system (Beyond Presence, LiveAvatar, Ready Player Me, etc.)
- **Real-time Lip Sync**: Synchronized facial animation with speech output from `narayana-spk`
- **CPL Event Integration**: Avatar responds to CPL events (emotions, thoughts, memories)
- **Expression System**: Maps emotions and cognitive states to facial expressions
- **Gesture Support**: Hand and body gestures for enhanced communication
- **WebSocket Bridge**: Real-time streaming to web clients
- **Beyond Presence Provider**: Hyper-realistic avatar support (currently implemented)

## Quick Start

```rust
use narayana_me::{AvatarConfig, AvatarBroker, AvatarProviderType};

// Create avatar config
let mut config = AvatarConfig::default();
config.enabled = true;
config.provider = AvatarProviderType::BeyondPresence;
config.enable_lip_sync = true;

// Create and initialize broker
let broker = AvatarBroker::new(config)?;
broker.initialize().await?;

// Start stream
let client_url = broker.start_stream().await?;
println!("Avatar stream: {}", client_url);

// Set expression
broker.set_expression(Expression::Happy, 0.8).await?;

// Update emotion (from CPL)
broker.update_emotion(Emotion::Joy, 0.7).await?;
```

## Configuration

```rust
pub struct AvatarConfig {
    pub enabled: bool,                      // Enable avatar rendering
    pub provider: AvatarProviderType,       // Provider (BeyondPresence, etc.)
    pub expression_sensitivity: f64,        // 0.0-1.0, default 0.7
    pub animation_speed: f64,               // 0.5-2.0, default 1.0
    pub websocket_port: Option<u16>,        // Optional custom port
    pub enable_lip_sync: bool,              // Enable lip sync
    pub enable_gestures: bool,              // Enable gestures
    pub avatar_id: Option<String>,          // Provider-specific avatar ID
}
```

## Integration with CPL

The avatar integrates with CPL through the `AvatarAdapter`, which implements `ProtocolAdapter` for `narayana-wld`:

```rust
use narayana_me::{create_avatar_adapter_from_cpl, AvatarConfig};
use narayana_storage::conscience_persistent_loop::CPLConfig;

let mut cpl_config = CPLConfig::default();
cpl_config.enable_avatar = true;

// Avatar config can be set via JSON
cpl_config.avatar_config = Some(serde_json::json!({
    "provider": "BeyondPresence",
    "enable_lip_sync": true,
    "expression_sensitivity": 0.8,
}));

if let Ok(Some(adapter)) = create_avatar_adapter_from_cpl(&cpl_config) {
    // Register adapter with WorldBroker
    broker.register_adapter(Box::new(adapter));
}
```

## Beyond Presence Provider

The Beyond Presence Genesis 1.0 provider offers hyper-realistic avatars with:
- Frame-accurate lip sync (<100ms latency)
- High-resolution facial rendering
- Natural head motion and expressions
- Audio-driven animation

### API Key Setup

Set the API key via environment variable:
```bash
export BEYOND_PRESENCE_API_KEY="sk-d4qXnCSYoSwOIQO0o_-ayq920Peu3k2iTE3nuxEf9U8"
```

Or use the default key in the code (for development only).

## Architecture

```
AvatarAdapter (ProtocolAdapter)
    ↓
AvatarBroker (Unified API)
    ↓
AvatarProvider (Beyond Presence, LiveAvatar, etc.)
    ↓
Beyond Presence API / WebSocket Stream
```

## WebSocket Bridge

The bridge streams avatar updates to web clients:

```rust
let bridge = AvatarBridge::new(broker_arc, 8081);
bridge.start().await?;

// Clients connect to: ws://localhost:8081/avatar/ws
```

## Examples

```bash
# Run basic example
cargo run --example basic_avatar --package narayana-me --features beyond-presence
```

## Testing

```bash
# Run tests
cargo test --package narayana-me --features beyond-presence
```

## Status

✅ Core architecture implemented
✅ AvatarBroker with unified API
✅ Beyond Presence provider (basic implementation)
✅ CPL integration
✅ WebSocket bridge
✅ Configuration and validation
⚠️ Web frontend (React Three Fiber) - TODO
⚠️ Full Beyond Presence API integration - needs API docs

## Future Enhancements

- Additional avatar providers (LiveAvatar, Ready Player Me)
- Web frontend with React Three Fiber
- Custom avatar model upload
- VR/AR support
- Multi-avatar support (multiple CPLs)

