# narayana-wld: World Broker Interface

A broker interface that mediates bidirectional communication between Conscience Persistent Loops (CPL) and the external world.

## Overview

The `narayana-wld` (world) module implements a sensory-motor interface based on cognitive architecture principles:

- **Global Workspace Theory** (Baars, 1988): Sensory input competes for workspace access
- **Predictive Processing** (Friston, 2010): Actions minimize prediction error
- **Embodied Cognition** (Varela, Thompson, Rosch, 1991): Cognition situated in environment
- **Attention Mechanisms** (Desimone & Duncan, 1995): Salience-based filtering

## Architecture

### Components

1. **WorldBroker**: Main orchestrator managing adapters and event routing
2. **SensoryInterface**: Processes incoming world events → CPL events
3. **MotorInterface**: Processes CPL events → world actions
4. **EventTransformer**: Bidirectional event format conversion
5. **AttentionFilter**: Computes salience and routes high-priority events
6. **ProtocolAdapters**: Pluggable adapters (HTTP, WebSocket, etc.)

### Event Flow

**Inbound (World → CPL):**
1. External event arrives via protocol adapter
2. Adapter converts to `WorldEvent`
3. SensoryInterface transforms to `CognitiveEvent` or `CPLEvent`
4. AttentionFilter computes salience
5. High-salience events → Global Workspace
6. All events → CognitiveBrain for storage

**Outbound (CPL → World):**
1. CPL emits `CPLEvent` or `CognitiveEvent`
2. MotorInterface receives via event subscription
3. EventTransformer converts to `WorldAction`
4. Protocol adapter sends to external system

## Usage

```rust
use narayana_wld::{WorldBroker, WorldBrokerConfig};
use narayana_storage::cognitive::CognitiveBrain;
use narayana_storage::conscience_persistent_loop::{ConsciencePersistentLoop, CPLConfig};
use std::sync::Arc;

// Create cognitive brain and CPL
let brain = Arc::new(CognitiveBrain::new());
let cpl_config = CPLConfig::default();
let cpl = Arc::new(ConsciencePersistentLoop::new(brain.clone(), cpl_config));

// Create world broker
let config = WorldBrokerConfig::default();
let broker = WorldBroker::new(brain, cpl, config)?;

// Start broker
broker.start().await?;

// Process world events
let event = WorldEvent::UserInput {
    user_id: "user1".to_string(),
    input: "Hello".to_string(),
    context: serde_json::json!({}),
};
broker.process_world_event(event).await?;

// Stop broker
broker.stop().await?;
```

## Protocol Adapters

### HTTP Adapter

Receives events via HTTP POST to `/world/events`:

```json
{
  "type": "user_input",
  "user_id": "user1",
  "input": "Hello",
  "context": {}
}
```

### WebSocket Adapter

Bidirectional real-time communication (placeholder implementation).

## Configuration

```rust
let config = WorldBrokerConfig {
    enabled_adapters: vec!["http".to_string(), "websocket".to_string()],
    salience_threshold: 0.5,
    event_buffer_size: 1000,
    enable_attention_filter: true,
    // ... other settings
};
```

## Testing

Run tests with:

```bash
cargo test --package narayana-wld
```

## Academic References

- Baars, B. J. (1988). *A Cognitive Theory of Consciousness*
- Friston, K. (2010). The free-energy principle: a unified brain theory?
- Varela, F. J., Thompson, E., & Rosch, E. (1991). *The Embodied Mind*
- Desimone, R., & Duncan, J. (1995). Neural mechanisms of selective visual attention

