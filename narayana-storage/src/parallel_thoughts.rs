// Parallel Thought Processing - Multiple Thoughts Simultaneously
// For Next-Generation Robot Brains
// Enhanced with Dynamic Thought Spawning

use crate::cognitive::*;
use crate::dynamic_thoughts::*;
use narayana_core::{Error, Result};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

/// Parallel thought processor - runs multiple thoughts simultaneously
/// Enhanced with dynamic thought spawning
pub struct ParallelThoughtProcessor {
    brain: Arc<CognitiveBrain>,
    active_threads: Arc<RwLock<HashMap<String, String>>>, // thought_id -> thread_id
    max_parallel_thoughts: usize,
    dynamic_spawner: Arc<DynamicThoughtSpawner>,
}

impl ParallelThoughtProcessor {
    pub fn new(brain: Arc<CognitiveBrain>, max_parallel_thoughts: usize) -> Self {
        let dynamic_spawner = Arc::new(DynamicThoughtSpawner::new(brain.clone()));
        
        // Enable automatic spawning
        let spawner_clone = dynamic_spawner.clone();
        tokio::spawn(async move {
            spawner_clone.enable_automatic_spawning().await;
        });

        Self {
            brain,
            active_threads: Arc::new(RwLock::new(HashMap::new())),
            max_parallel_thoughts,
            dynamic_spawner,
        }
    }

    /// Process a thought in parallel with dynamic spawning support
    pub async fn process_thought_parallel<F>(
        &self,
        thought_id: String,
        processor: F,
    ) -> Result<Vec<String>>
    where
        F: Fn(ThoughtProcessingContext, serde_json::Value) -> Result<serde_json::Value> + Send + Sync + 'static,
    {
        // Check if we can start a new thought
        let current_threads = self.active_threads.read().await.len();
        if current_threads >= self.max_parallel_thoughts {
            return Err(narayana_core::Error::Storage(
                "Max parallel thoughts reached".to_string(),
            ));
        }

        let brain = self.brain.clone();
        let thought_id_clone = thought_id.clone();
        let active_threads = self.active_threads.clone();
        let spawner = self.dynamic_spawner.clone();

        // Track in active threads before processing
        active_threads.write().await.insert(thought_id_clone.clone(), thought_id_clone.clone());

        // Spawn async task for true parallel execution
        let thought_id_for_task = thought_id_clone.clone();
        let processor = Arc::new(processor); // Wrap in Arc to share across tasks
        let processor_clone = processor.clone();
        let brain_clone = brain.clone();
        let spawner_clone = spawner.clone();
        let active_threads_clone = active_threads.clone();

        let handle = tokio::spawn(async move {
               // Get thought from brain
               let thought = {
                   let thoughts = brain_clone.thoughts.read();
                   thoughts.get(&thought_id_for_task).cloned()
               };

               if let Some(thought) = thought {
                   // Create processing context
                   let now = std::time::SystemTime::now()
                       .duration_since(std::time::UNIX_EPOCH)
                       .unwrap_or_default()
                       .as_secs();

                   let mut context = ThoughtProcessingContext {
                       processor_id: format!("proc_{}", thought_id_for_task),
                       parent_thought_id: None,
                       current_thought_id: thought_id_for_task.clone(),
                       spawned_thoughts: Vec::new(),
                       processing_start: now,
                       context: std::collections::HashMap::new(),
                       can_spawn: true,
                   };

                   // Process thought (can spawn new thoughts during processing)
                   let result = match (*processor_clone)(context.clone(), thought.content.clone()) {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::error!("Error processing thought {}: {:?}", thought_id_for_task, e);
                        return Err(e);
                    }
                };

                // Update thought with result
                {
                    let mut thoughts = brain_clone.thoughts.write();
                    if let Some(t) = thoughts.get_mut(&thought_id_for_task) {
                        t.content = result;
                        t.state = ThoughtState::Completed;
                        t.updated_at = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                    }
                }

                // Get spawned thoughts from context
                Ok(context.spawned_thoughts)
            } else {
                Err(narayana_core::Error::Storage(format!("Thought {} not found", thought_id_for_task)))
            }
        });

        // Wait for completion and get spawned thoughts
        let spawned = match handle.await {
            Ok(Ok(spawned)) => spawned,
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(narayana_core::Error::Storage(format!("Task join error: {:?}", e))),
        };

        // Remove from active threads
        active_threads.write().await.remove(&thought_id_clone);

        Ok(spawned)
    }

    /// Process multiple thoughts concurrently using rayon for CPU-bound work
    pub fn process_thoughts_parallel_cpu<F>(
        &self,
        thought_ids: Vec<String>,
        processor: F,
    ) -> Result<Vec<(String, serde_json::Value)>>
    where
        F: Fn(serde_json::Value) -> Result<serde_json::Value> + Send + Sync,
    {
        use rayon::prelude::*;

        // Get all thoughts
        let thoughts: Vec<(String, serde_json::Value)> = {
            let thoughts_guard = self.brain.thoughts.read();
            thought_ids.iter()
                .filter_map(|id| {
                    thoughts_guard.get(id).map(|t| (id.clone(), t.content.clone()))
                })
                .collect()
        };

        // Process in parallel using rayon
        let results: Result<Vec<_>> = thoughts.into_par_iter()
            .map(|(id, content)| {
                processor(content)
                    .map(|result| (id, result))
            })
            .collect();

        results
    }

    /// Spawn thought on-the-fly during processing
    pub fn spawn_thought_during_processing(
        &self,
        processor_id: &str,
        spawn_request: ThoughtSpawnRequest,
    ) -> Result<String> {
        self.dynamic_spawner.spawn_thought_during_processing(processor_id, spawn_request)
    }

    /// Get dynamic spawner
    pub fn spawner(&self) -> Arc<DynamicThoughtSpawner> {
        self.dynamic_spawner.clone()
    }

    /// Get active thought count
    pub async fn active_thought_count(&self) -> usize {
        self.active_threads.read().await.len()
    }
}

/// Thought scheduler - schedules and prioritizes thoughts
pub struct ThoughtScheduler {
    brain: Arc<CognitiveBrain>,
    queue: Arc<RwLock<Vec<ScheduledThought>>>,
}

#[derive(Debug, Clone)]
struct ScheduledThought {
    thought_id: String,
    priority: f64,
    scheduled_at: u64,
}

impl ThoughtScheduler {
    pub fn new(brain: Arc<CognitiveBrain>) -> Self {
        Self {
            brain,
            queue: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Schedule a thought
    pub async fn schedule(&self, thought_id: String, priority: f64) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let scheduled = ScheduledThought {
            thought_id,
            priority,
            scheduled_at: now,
        };

        let mut queue = self.queue.write().await;
        queue.push(scheduled);
        queue.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap_or(std::cmp::Ordering::Equal));

        Ok(())
    }

    /// Get next thought to process
    pub async fn next(&self) -> Option<String> {
        let mut queue = self.queue.write().await;
        queue.pop().map(|s| s.thought_id)
    }
}

