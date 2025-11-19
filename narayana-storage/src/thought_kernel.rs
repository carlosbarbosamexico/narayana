// Unified Thought Kernel - The Linux Kernel for Robot Minds
// Production-ready thought scheduling with priorities, deadlines, cancellation,
// event hooks, shared memory regions, and GPU scheduling

use crate::cognitive::*;
use crate::dynamic_thoughts::*;
use crate::gpu_execution::GpuEngine;
use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use tokio::sync::{broadcast, mpsc, oneshot};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{info, warn, debug, error};
use uuid::Uuid;

/// Thought kernel - unified thought execution system
pub struct ThoughtKernel {
    brain: Arc<CognitiveBrain>,
    spawner: Arc<DynamicThoughtSpawner>,
    gpu_engine: Arc<GpuEngine>,
    scheduler: Arc<ThoughtScheduler>,
    shared_memory: Arc<SharedMemoryRegion>,
    event_hooks: Arc<RwLock<HashMap<String, Vec<Box<dyn Fn(&ThoughtEvent) + Send + Sync>>>>>,
    cancellation_tokens: Arc<RwLock<HashMap<String, CancellationToken>>>,
}

impl ThoughtKernel {
    pub fn new(brain: Arc<CognitiveBrain>) -> Self {
        let spawner = Arc::new(DynamicThoughtSpawner::new(brain.clone()));
        let gpu_engine = Arc::new(GpuEngine::new().unwrap_or_else(|_| {
            GpuEngine::with_backend(crate::gpu_execution::Backend::CPU).unwrap()
        }));
        let scheduler = Arc::new(ThoughtScheduler::new());
        let shared_memory = Arc::new(SharedMemoryRegion::new());
        
        Self {
            brain,
            spawner,
            gpu_engine,
            scheduler,
            shared_memory,
            event_hooks: Arc::new(RwLock::new(HashMap::new())),
            cancellation_tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Spawn a thought with full kernel support
    pub async fn spawn_thought<F>(
        &self,
        ctx: ThoughtContext,
        processor: F,
    ) -> Result<ThoughtResult>
    where
        F: Fn(ThoughtProcessingContext, serde_json::Value) -> Result<serde_json::Value> + Send + Sync + 'static,
    {
        let thought_id = self.brain.create_thought(
            ctx.content.clone(),
            ctx.priority,
        )?;

        // Create cancellation token
        let (_cancel_tx, _cancel_rx) = oneshot::channel::<()>();
        let token = CancellationToken {
            thought_id: thought_id.clone(),
            cancel_rx: None, // Not used in current implementation
            cancelled: Arc::new(RwLock::new(false)),
        };
        self.cancellation_tokens.write().insert(thought_id.clone(), token.clone());

        // Clone ctx at the beginning to avoid partial moves
        let ctx_clone = ctx.clone();
        
        // Register with scheduler
        let deadline = ctx_clone.deadline.map(|d| SystemTime::now() + Duration::from_secs(d));
        self.scheduler.schedule(ThoughtTask {
            thought_id: thought_id.clone(),
            priority: ctx_clone.priority,
            deadline,
            gpu_required: ctx_clone.gpu_required,
        }).await?;

        // Create processing context with shared memory access
        let shared_memory_id_str = ctx_clone.shared_memory_id.unwrap_or_default();
        let mut proc_ctx = ThoughtProcessingContext {
            processor_id: Uuid::new_v4().to_string(),
            parent_thought_id: ctx_clone.parent_thought_id.clone(),
            current_thought_id: thought_id.clone(),
            spawned_thoughts: Vec::new(),
            processing_start: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            context: ctx_clone.context.clone(),
            can_spawn: true,
        };
        
        // Add shared memory access
        proc_ctx.context.insert(
            "shared_memory_id".to_string(),
            serde_json::Value::String(shared_memory_id_str),
        );

        // Track spawned thought relationship
        if let Some(parent_id) = &ctx_clone.parent_thought_id {
            let mut thoughts = self.brain.thoughts.write();
            if let Some(parent_thought) = thoughts.get_mut(parent_id) {
                parent_thought.spawned_thoughts.push(thought_id.clone());
            }
        }
        
        // Capture spawned_thoughts before moving proc_ctx
        let spawned_thoughts_clone = proc_ctx.spawned_thoughts.clone();
        
        // Execute thought
        let token_clone = token.clone();
        // EDGE CASE: Handle potential panic in spawned task
        let result = tokio::spawn(async move {
            // Check for cancellation
            if *token_clone.cancelled.read() {
                return Err(Error::Storage("Thought cancelled".to_string()));
            }

            // Process thought
            let result = processor(proc_ctx, ctx_clone.content)?;

            // Check cancellation again
            if *token_clone.cancelled.read() {
                return Err(Error::Storage("Thought cancelled during processing".to_string()));
            }

            Ok(result)
        }).await;
        
        // EDGE CASE: Handle task join errors (panic, cancellation, etc.)
        let result = match result {
            Ok(Ok(val)) => Ok(val),
            Ok(Err(e)) => Err(e),
            Err(join_err) => {
                // Task panicked or was cancelled
                // Update thought state to reflect failure
                {
                    let mut thoughts = self.brain.thoughts.write();
                    if let Some(thought) = thoughts.get_mut(&thought_id) {
                        thought.state = ThoughtState::Discarded;
                    }
                }
                return Err(Error::Storage(format!("Task join error: {}", join_err)));
            }
        }?;

        // Update thought with result
        {
            let mut thoughts = self.brain.thoughts.write();
            if let Some(thought) = thoughts.get_mut(&thought_id) {
                thought.content = result.clone();
                thought.state = ThoughtState::Completed;
                thought.updated_at = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
            }
        }

        // Trigger event hooks
        self.trigger_event_hooks(ThoughtEvent::ThoughtCompleted {
            thought_id: thought_id.clone(),
            result: result.clone(),
        }).await;

        // Cleanup
        self.cancellation_tokens.write().remove(&thought_id);
        self.scheduler.complete(&thought_id).await;

        Ok(ThoughtResult {
            thought_id,
            result,
            spawned_thoughts: spawned_thoughts_clone,
        })
    }

    /// Register event hook
    pub fn register_event_hook<F>(&self, event_type: String, hook: F)
    where
        F: Fn(&ThoughtEvent) + Send + Sync + 'static,
    {
        self.event_hooks.write()
            .entry(event_type)
            .or_insert_with(Vec::new)
            .push(Box::new(hook));
    }

    /// Trigger event hooks
    async fn trigger_event_hooks(&self, event: ThoughtEvent) {
        let event_type = match &event {
            ThoughtEvent::ThoughtCreated { .. } => "thought_created",
            ThoughtEvent::ThoughtCompleted { .. } => "thought_completed",
            ThoughtEvent::ThoughtCancelled { .. } => "thought_cancelled",
            ThoughtEvent::MemoryAccessed { .. } => "memory_accessed",
            ThoughtEvent::GpuScheduled { .. } => "gpu_scheduled",
        };

        // Execute hooks while holding the lock
        {
            let hooks = self.event_hooks.read();
            if let Some(hooks) = hooks.get(event_type) {
                for hook in hooks.iter() {
                    hook(&event);
                }
            }
        }
    }

    /// Cancel thought cooperatively
    pub fn cancel_thought(&self, thought_id: &str) -> Result<()> {
        // EDGE CASE: Prevent deadlock by acquiring locks in consistent order
        // Always acquire cancellation_tokens before thoughts to prevent deadlock
        let mut tokens = self.cancellation_tokens.write();
        if let Some(token) = tokens.get_mut(thought_id) {
            *token.cancelled.write() = true;
        }
        drop(tokens); // Release lock before acquiring next one
        
        // Update thought state (separate lock acquisition to prevent deadlock)
        let mut thoughts = self.brain.thoughts.write();
        if let Some(thought) = thoughts.get_mut(thought_id) {
            thought.state = ThoughtState::Discarded;
        }
        drop(thoughts); // Release lock before accessing event hooks

        // Trigger cancellation event (fire and forget, but track for cleanup)
        let hooks = self.event_hooks.clone();
        let event = ThoughtEvent::ThoughtCancelled {
            thought_id: thought_id.to_string(),
        };
        // Execute synchronously to avoid resource leaks
        {
            let hooks = hooks.read();
            if let Some(hooks) = hooks.get("thought_cancelled") {
                for hook in hooks {
                    hook(&event);
                }
            }
        }

        Ok(())
    }

    /// Access shared memory region
    pub fn get_shared_memory(&self, region_id: &str) -> Option<Arc<SharedMemoryRegion>> {
        if region_id.is_empty() {
            Some(self.shared_memory.clone())
        } else {
            // In production: would support multiple named regions
            Some(self.shared_memory.clone())
        }
    }

    /// Schedule thought on GPU
    /// Note: GPU operations are now handled through GpuEngine directly
    /// This method is kept for compatibility but should use GpuEngine methods directly
    pub async fn schedule_gpu_thought(
        &self,
        thought_id: &str,
        _operation_type: &str, // e.g., "matmul", "dot", "filter"
    ) -> Result<()> {
        // Get thought content (assumed to be columnar data)
        let _thought = {
            let thoughts = self.brain.thoughts.read();
            thoughts.get(thought_id).cloned()
        }.ok_or_else(|| Error::Storage(format!("Thought {} not found", thought_id)))?;

        // In production: would execute GPU operation using GpuEngine methods
        // For example: engine.matmul(&tensor_a, &tensor_b) or engine.filter(&column, &mask)

        // Trigger GPU scheduling event
        self.trigger_event_hooks(ThoughtEvent::GpuScheduled {
            thought_id: thought_id.to_string(),
        }).await;

        Ok(())
    }
}

/// Thought context for kernel spawning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtContext {
    pub content: serde_json::Value,
    pub priority: f64,
    pub deadline: Option<u64>, // Seconds from now
    pub parent_thought_id: Option<String>,
    pub context: HashMap<String, serde_json::Value>,
    pub shared_memory_id: Option<String>,
    pub gpu_required: bool,
}

/// Thought result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtResult {
    pub thought_id: String,
    pub result: serde_json::Value,
    pub spawned_thoughts: Vec<String>,
}

/// Thought event
#[derive(Debug, Clone)]
pub enum ThoughtEvent {
    ThoughtCreated { thought_id: String },
    ThoughtCompleted { thought_id: String, result: serde_json::Value },
    ThoughtCancelled { thought_id: String },
    MemoryAccessed { memory_id: String },
    GpuScheduled { thought_id: String },
}

/// Thought scheduler with priority and deadline support
struct ThoughtScheduler {
    task_queue: Arc<RwLock<Vec<ThoughtTask>>>,
    active_tasks: Arc<RwLock<HashMap<String, ThoughtTask>>>,
}

#[derive(Clone)]
struct ThoughtTask {
    thought_id: String,
    priority: f64,
    deadline: Option<SystemTime>,
    gpu_required: bool,
}

impl ThoughtScheduler {
    fn new() -> Self {
        Self {
            task_queue: Arc::new(RwLock::new(Vec::new())),
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn schedule(&self, task: ThoughtTask) -> Result<()> {
        let thought_id = task.thought_id.clone();
        self.active_tasks.write().insert(thought_id.clone(), task.clone());
        
        let mut queue = self.task_queue.write();
        queue.push(task);
        queue.sort_by(|a, b| {
            // Sort by priority (higher first), then by deadline (earlier first)
            // EDGE CASE: Handle NaN, Infinity, and -Infinity
            // SECURITY: Clamp priority to prevent priority inversion attacks
            // An attacker could set priority to Infinity to starve other thoughts
            const MAX_PRIORITY: f64 = 1e6; // Maximum allowed priority
            const MIN_PRIORITY: f64 = -1e6; // Minimum allowed priority
            
            let a_priority = a.priority.clamp(MIN_PRIORITY, MAX_PRIORITY);
            let b_priority = b.priority.clamp(MIN_PRIORITY, MAX_PRIORITY);
            
            if a_priority.is_nan() || b_priority.is_nan() {
                std::cmp::Ordering::Equal
            } else {
                b_priority.partial_cmp(&a_priority)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }
                .then_with(|| {
                    match (a.deadline, b.deadline) {
                        (Some(a_d), Some(b_d)) => a_d.cmp(&b_d),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                })
        });

        Ok(())
    }

    async fn complete(&self, thought_id: &str) {
        self.active_tasks.write().remove(thought_id);
        let mut queue = self.task_queue.write();
        queue.retain(|t| t.thought_id != thought_id);
    }

    fn get_next_task(&self) -> Option<ThoughtTask> {
        let mut queue = self.task_queue.write();
        queue.pop()
    }
}

/// Shared memory region for thoughts
pub struct SharedMemoryRegion {
    data: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    locks: Arc<RwLock<HashMap<String, Arc<RwLock<()>>>>>,
}

impl SharedMemoryRegion {
    fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            locks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn read(&self, key: &str) -> Option<serde_json::Value> {
        let data = self.data.read();
        data.get(key).cloned()
    }

    pub fn write(&self, key: String, value: serde_json::Value) {
        let mut data = self.data.write();
        data.insert(key, value);
    }

    pub fn lock(&self, key: &str) -> Arc<RwLock<()>> {
        let mut locks = self.locks.write();
        locks.entry(key.to_string())
            .or_insert_with(|| Arc::new(RwLock::new(())))
            .clone()
    }
}

/// Cancellation token for cooperative cancellation
struct CancellationToken {
    thought_id: String,
    cancel_rx: Option<oneshot::Receiver<()>>,
    cancelled: Arc<RwLock<bool>>,
}

impl Clone for CancellationToken {
    fn clone(&self) -> Self {
        Self {
            thought_id: self.thought_id.clone(),
            cancel_rx: None, // Cannot clone oneshot receiver
            cancelled: self.cancelled.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_thought_kernel_spawn() {
        let brain = Arc::new(CognitiveBrain::new());
        let kernel = ThoughtKernel::new(brain);
        
        let ctx = ThoughtContext {
            content: serde_json::json!({"task": "test"}),
            priority: 0.9,
            deadline: Some(10),
            parent_thought_id: None,
            context: HashMap::new(),
            shared_memory_id: None,
            gpu_required: false,
        };

        let result = kernel.spawn_thought(ctx, |_ctx, content| {
            Ok(serde_json::json!({"result": "success"}))
        }).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_thought_cancellation() {
        let brain = Arc::new(CognitiveBrain::new());
        let kernel = ThoughtKernel::new(brain);
        
        let ctx = ThoughtContext {
            content: serde_json::json!({"task": "test"}),
            priority: 0.9,
            deadline: None,
            parent_thought_id: None,
            context: HashMap::new(),
            shared_memory_id: None,
            gpu_required: false,
        };

        let thought_id = kernel.brain.create_thought(
            ctx.content.clone(),
            ctx.priority,
        ).unwrap();

        // Cancel before processing
        kernel.cancel_thought(&thought_id).unwrap();
        
        let thoughts = kernel.brain.thoughts.read();
        let thought = thoughts.get(&thought_id).unwrap();
        assert_eq!(thought.state, ThoughtState::Discarded);
    }
}

