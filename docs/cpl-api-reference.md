# Conscience Persistent Loop: API Reference

## Overview

This document provides a comprehensive API reference for the Conscience Persistent Loop (CPL) system. All types, functions, and methods are documented with their signatures, parameters, return values, and behavioral specifications.

## Core Types

### CPLConfig

Configuration structure for initializing a Conscience Persistent Loop instance.

```rust
pub struct CPLConfig {
    pub loop_interval_ms: u64,
    pub enable_global_workspace: bool,
    pub enable_background_daemon: bool,
    pub enable_dreaming: bool,
    pub working_memory_capacity: usize,
    pub enable_attention: bool,
    pub enable_narrative: bool,
    pub enable_memory_bridge: bool,
    pub enable_persistence: bool,
    pub persistence_dir: Option<String>,
}
```

**Fields:**

- `loop_interval_ms`: Loop execution interval in milliseconds. Must be greater than 0.
- `enable_global_workspace`: Enable the Global Workspace Model component.
- `enable_background_daemon`: Enable background unconscious processing.
- `enable_dreaming`: Enable offline experience replay (dreaming loop).
- `working_memory_capacity`: Maximum number of items in working memory. Must be greater than 0.
- `enable_attention`: Enable attention routing component.
- `enable_narrative`: Enable narrative generation for identity formation.
- `enable_memory_bridge`: Enable episodic-semantic memory consolidation.
- `enable_persistence`: Enable state persistence to disk.
- `persistence_dir`: Optional directory path for persistence. Must be a valid, safe path.

**Default Values:**

```rust
impl Default for CPLConfig {
    fn default() -> Self {
        CPLConfig {
            loop_interval_ms: 100,
            enable_global_workspace: true,
            enable_background_daemon: true,
            enable_dreaming: true,
            working_memory_capacity: 7,
            enable_attention: true,
            enable_narrative: true,
            enable_memory_bridge: true,
            enable_persistence: false,
            persistence_dir: None,
        }
    }
}
```

### ConsciencePersistentLoop

Main orchestrator for the cognitive consciousness system.

```rust
pub struct ConsciencePersistentLoop {
    // Private fields
}
```

**Methods:**

#### new

Creates a new CPL instance.

```rust
pub fn new(brain: Arc<CognitiveBrain>, config: CPLConfig) -> Self
```

**Parameters:**

- `brain`: Shared reference to the cognitive brain instance.
- `config`: Configuration for the CPL.

**Returns:** A new `ConsciencePersistentLoop` instance.

**Errors:** None (constructor cannot fail).

#### initialize

Initializes all enabled cognitive systems.

```rust
pub async fn initialize(&self) -> Result<()>
```

**Parameters:** None (uses `self`).

**Returns:** `Result<()>` indicating success or failure.

**Errors:**

- Returns `Error::Storage` if `loop_interval_ms` is 0.
- Returns `Error::Storage` if `working_memory_capacity` is 0.

**Behavior:**

- Initializes Global Workspace if enabled.
- Initializes Background Daemon if enabled.
- Initializes Memory Bridge if enabled.
- Initializes Narrative Generator if enabled.
- Initializes Attention Router if enabled.
- Initializes Dreaming Loop if enabled.
- Loads persisted state if persistence is enabled.

#### start

Starts the persistent loop execution.

```rust
pub async fn start(self: Arc<Self>) -> Result<()>
```

**Parameters:**

- `self`: Must be called with `Arc<Self>` for proper `Send` semantics.

**Returns:** `Result<()>` indicating success or failure.

**Errors:**

- Returns `Error::Storage("CPL is already running")` if the loop is already running.
- Returns `Error::Storage("Loop interval must be > 0")` if interval is invalid.

**Behavior:**

- Spawns an asynchronous task that runs the main loop.
- Loop executes at the configured interval.
- Loop continues until `stop()` is called.

**Concurrency:** This method spawns a new task and returns immediately. The loop runs concurrently.

#### stop

Stops the persistent loop execution.

```rust
pub async fn stop(&self) -> Result<()>
```

**Parameters:** None (uses `self`).

**Returns:** `Result<()>` indicating success or failure.

**Errors:** None (always succeeds).

**Behavior:**

- Sets the running flag to false.
- Persists state if persistence is enabled.
- The loop will exit on the next iteration.

#### id

Returns the unique identifier for this CPL instance.

```rust
pub fn id(&self) -> &str
```

**Returns:** A string slice containing the UUID of the CPL instance.

#### brain

Returns a reference to the cognitive brain.

```rust
pub fn brain(&self) -> &Arc<CognitiveBrain>
```

**Returns:** A reference to the shared cognitive brain instance.

#### working_memory

Returns a reference to the working memory scratchpad.

```rust
pub fn working_memory(&self) -> &Arc<WorkingMemoryScratchpad>
```

**Returns:** A reference to the working memory instance.

#### subscribe_events

Creates a new event receiver for CPL events.

```rust
pub fn subscribe_events(&self) -> broadcast::Receiver<CPLEvent>
```

**Returns:** A broadcast receiver that will receive CPL events.

**Behavior:**

- Events are broadcast to all subscribers.
- If a receiver falls behind, older events may be dropped.
- Events include loop iterations, attention shifts, narrative updates, etc.

#### is_running

Checks if the CPL loop is currently running.

```rust
pub fn is_running(&self) -> bool
```

**Returns:** `true` if the loop is running, `false` otherwise.

## Event Types

### CPLEvent

Enumeration of events emitted by the CPL system.

```rust
pub enum CPLEvent {
    LoopIteration {
        iteration: u64,
        timestamp: u64,
    },
    AttentionShifted {
        from: Option<String>,
        to: String,
        timestamp: u64,
        salience: f64,
    },
    NarrativeUpdated {
        narrative_id: String,
        coherence: f64,
        timestamp: u64,
    },
    MemoryConsolidated {
        episodic_id: String,
        semantic_id: String,
        timestamp: u64,
    },
    DreamingReplay {
        experiences_replayed: usize,
        timestamp: u64,
    },
}
```

**Variants:**

- `LoopIteration`: Emitted on each loop iteration.
- `AttentionShifted`: Emitted when attention focus changes.
- `NarrativeUpdated`: Emitted when the narrative is updated.
- `MemoryConsolidated`: Emitted when an episodic memory is consolidated to semantic.
- `DreamingReplay`: Emitted when the dreaming loop performs replay.

## Component APIs

### GlobalWorkspace

#### new

```rust
pub fn new(brain: Arc<CognitiveBrain>, event_sender: broadcast::Sender<CPLEvent>) -> Self
```

Creates a new Global Workspace instance.

#### process_broadcast

```rust
pub async fn process_broadcast(&self) -> Result<()>
```

Executes one cycle of the global workspace model:

1. Computes competition scores for all candidates.
2. Selects winners (highest scores up to capacity).
3. Updates workspace with new conscious content.
4. Broadcasts to all systems.
5. Records integration events.

### BackgroundDaemon

#### new

```rust
pub fn new(brain: Arc<CognitiveBrain>, event_sender: broadcast::Sender<CPLEvent>) -> Self
```

Creates a new Background Daemon instance.

#### process

```rust
pub async fn process(&self) -> Result<()>
```

Executes one cycle of background processing:

1. Memory consolidation (forgetting curves, strength updates).
2. Pattern detection from experiences.
3. Association formation.
4. Process queued items.

### WorkingMemoryScratchpad

#### new

```rust
pub fn new(brain: Arc<CognitiveBrain>, capacity: usize) -> Self
```

Creates a new working memory scratchpad.

**Parameters:**

- `brain`: Reference to the cognitive brain.
- `capacity`: Maximum number of items (typically 7±2).

#### update

```rust
pub async fn update(&self) -> Result<()>
```

Updates working memory:

1. Applies temporal decay to all entries.
2. Removes entries with low activation (< 0.1).
3. Enforces capacity limit (promotes lowest activation to episodic).
4. Sorts by activation (highest first).

#### add

```rust
pub async fn add(&self, content_id: String, content_type: ScratchpadContentType, context: serde_json::Value) -> Result<()>
```

Adds content to working memory.

**Parameters:**

- `content_id`: Unique identifier for the content.
- `content_type`: Type of content (Memory, Thought, Experience).
- `context`: JSON context data.

**Behavior:**

- If content already exists, boosts activation.
- If at capacity, removes lowest activation item (after promoting to episodic).
- New entries start with activation 0.8.

#### access

```rust
pub async fn access(&self, content_id: &str) -> Result<Option<ScratchpadEntry>>
```

Accesses content in working memory, boosting its activation.

**Returns:** `Some(entry)` if found, `None` otherwise.

### MemoryBridge

#### new

```rust
pub fn new(brain: Arc<CognitiveBrain>, working_memory: Arc<WorkingMemoryScratchpad>, event_sender: broadcast::Sender<CPLEvent>) -> Self
```

Creates a new Memory Bridge instance.

#### process_bridge

```rust
pub async fn process_bridge(&self) -> Result<()>
```

Executes one cycle of memory bridge processing:

1. Identifies episodic memories ready for consolidation.
2. Replays episodic memories (hippocampal replay).
3. Extracts patterns from episodic memories.
4. Consolidates episodic to semantic memories.

### NarrativeGenerator

#### new

```rust
pub fn new(brain: Arc<CognitiveBrain>, event_sender: broadcast::Sender<CPLEvent>) -> Self
```

Creates a new Narrative Generator instance.

#### update_narrative

```rust
pub async fn update_narrative(&self) -> Result<()>
```

Updates the narrative:

1. Extracts key events from recent memories/experiences.
2. Updates identity markers.
3. Constructs narrative from events and markers.
4. Updates narrative.
5. Saves snapshot to history.

### AttentionRouter

#### new

```rust
pub fn new(brain: Arc<CognitiveBrain>, event_sender: broadcast::Sender<CPLEvent>) -> Self
```

Creates a new Attention Router instance.

#### route_attention

```rust
pub async fn route_attention(&self) -> Result<()>
```

Routes attention:

1. Computes salience for all candidates.
2. Allocates attention weights.
3. Updates focus (shifts if needed).

### DreamingLoop

#### new

```rust
pub fn new(brain: Arc<CognitiveBrain>, event_sender: broadcast::Sender<CPLEvent>) -> Self
```

Creates a new Dreaming Loop instance.

#### replay_experiences

```rust
pub async fn replay_experiences(&self) -> Result<()>
```

Replays experiences using epsilon-greedy sampling:

1. Updates replay buffer from brain experiences.
2. Samples batch for replay.
3. Replays experiences (strengthens memories, reinforces patterns).
4. Updates statistics.

## Manager API

### CPLManager

Manages multiple CPL instances.

#### new

```rust
pub fn new(default_config: CPLConfig) -> Self
```

Creates a new CPL manager with a default configuration.

#### spawn_cpl

```rust
pub async fn spawn_cpl(&self, config_override: Option<CPLConfig>) -> Result<String>
```

Spawns a new CPL instance.

**Parameters:**

- `config_override`: Optional configuration override. If `None`, uses default config.

**Returns:** The UUID of the spawned CPL instance.

#### get_cpl

```rust
pub fn get_cpl(&self, id: &str) -> Option<Arc<ConsciencePersistentLoop>>
```

Retrieves a CPL instance by ID.

**Returns:** `Some(cpl)` if found, `None` otherwise.

#### start_cpl

```rust
pub async fn start_cpl(&self, id: &str) -> Result<()>
```

Starts a specific CPL instance.

**Errors:** Returns error if CPL not found or already running.

#### stop_cpl

```rust
pub async fn stop_cpl(&self, id: &str) -> Result<()>
```

Stops a specific CPL instance.

**Errors:** Returns error if CPL not found.

#### start_all

```rust
pub async fn start_all(&self) -> Result<()>
```

Starts all CPL instances.

#### stop_all

```rust
pub async fn stop_all(&self) -> Result<()>
```

Stops all CPL instances.

#### remove_cpl

```rust
pub async fn remove_cpl(&self, id: &str) -> Result<()>
```

Removes a CPL instance from the manager.

**Errors:** Returns error if CPL not found or currently running.

#### count

```rust
pub fn count(&self) -> usize
```

Returns the number of managed CPL instances.

## Error Types

All methods return `Result<T, Error>` where `Error` is from the `narayana_core` crate. Common error types:

- `Error::Storage(String)`: Storage or state-related errors.
- `Error::Serialization(String)`: Serialization/deserialization errors.
- `Error::Deserialization(String)`: Deserialization errors.

## Thread Safety

All CPL components are designed for concurrent access:

- Shared state is protected by `RwLock` or `Arc<RwLock<T>>`.
- Methods are safe to call from multiple threads.
- The main loop runs in a separate async task.

## Performance Considerations

- Loop interval should be tuned based on system load.
- Working memory capacity affects memory usage and processing time.
- History buffer sizes affect memory usage.
- Queue sizes affect memory usage and processing latency.

## Security Considerations

- All file paths are validated to prevent directory traversal.
- Input values are validated (NaN, Infinity, zero denominators).
- Collection sizes are bounded to prevent resource exhaustion.
- Integer operations use saturating arithmetic to prevent overflow.

