//! Motor interface: CPL → World action flow
//! 
//! Receives cognitive events from CPL, transforms them to world actions,
//! and routes them to appropriate protocol adapters.

use crate::event_transformer::{EventTransformer, WorldAction};
use narayana_core::Error;
use narayana_storage::cognitive::{CognitiveBrain, CognitiveEvent};
use narayana_storage::conscience_persistent_loop::{ConsciencePersistentLoop, CPLEvent};
use narayana_storage::talking_cricket::{TalkingCricket, AssessmentContext};
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};
use std::collections::HashMap;

/// Motor interface for processing outgoing actions
pub struct MotorInterface {
    brain: Arc<CognitiveBrain>,
    transformer: Arc<RwLock<EventTransformer>>,
    action_sender: broadcast::Sender<WorldAction>,
    action_queue: Arc<RwLock<Vec<WorldAction>>>,
    talking_cricket: Arc<RwLock<Option<Arc<TalkingCricket>>>>, // Optional moral guide
}

impl MotorInterface {
    pub fn new(
        brain: Arc<CognitiveBrain>,
        transformer: Arc<RwLock<EventTransformer>>,
    ) -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self {
            brain,
            transformer,
            action_sender: sender,
            action_queue: Arc::new(RwLock::new(Vec::new())),
            talking_cricket: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Set Talking Cricket for moral assessment (optional)
    pub fn set_talking_cricket(&self, tc: Arc<TalkingCricket>) {
        *self.talking_cricket.write() = Some(tc);
        info!("Talking Cricket set on motor interface");
    }
    
    /// Remove Talking Cricket
    pub fn remove_talking_cricket(&self) {
        *self.talking_cricket.write() = None;
        info!("Talking Cricket removed from motor interface");
    }

    /// Process cognitive event and generate world action if needed
    pub async fn process_cognitive_event(&self, event: &CognitiveEvent) -> Result<(), Error> {
        debug!("Motor interface processing cognitive event: {:?}", event);

        let transformer = self.transformer.read();
        if let Some(action) = transformer.cognitive_to_world(event)? {
            self.queue_action(action).await?;
        }

        Ok(())
    }

    /// Process CPL event and generate world action if needed
    pub async fn process_cpl_event(&self, event: &CPLEvent) -> Result<(), Error> {
        debug!("Motor interface processing CPL event: {:?}", event);

        let transformer = self.transformer.read();
        if let Some(action) = transformer.cpl_to_world(event)? {
            self.queue_action(action).await?;
        }

        Ok(())
    }

    /// Queue action for execution
    pub async fn queue_action(&self, action: WorldAction) -> Result<(), Error> {
        // Check if Talking Cricket is attached and assess action
        let tc_opt = {
            let guard = self.talking_cricket.read();
            guard.as_ref().map(|tc| tc.clone())
        };
        
        if let Some(tc) = tc_opt {
            if tc.is_attached() {
                // Build full CPL context (memories, experiences, thoughts)
                let context = match tc.build_cpl_context(None).await {
                    Ok(ctx) => Some(ctx),
                    Err(e) => {
                        warn!("Failed to build CPL context for moral assessment: {}, using minimal context", e);
                        None
                    }
                };
                
                // Assess action with full CPL context
                match tc.assess_action(&action, context.as_ref()).await {
                    Ok(assessment) => {
                        // Emit event if CPL event sender is available
                        // (This would need to be passed in or accessed differently)
                        
                        // Apply veto if needed
                        if assessment.should_veto {
                            warn!("Action vetoed by Talking Cricket: {} (score: {:.2})", 
                                assessment.reasoning, assessment.moral_score);
                            return Ok(()); // Don't queue the action
                        }
                        
                        // Adjust action priority based on influence_weight
                        // (This would modify the action or queue priority)
                        info!("Action assessed by Talking Cricket: score={:.2}, influence={:.2}", 
                            assessment.moral_score, assessment.influence_weight);
                    }
                    Err(e) => {
                        warn!("Talking Cricket assessment error: {}, proceeding with action", e);
                    }
                }
            }
        }
        
        info!("Queuing world action: {:?}", action);
        
        // Prevent unbounded queue growth
        const MAX_QUEUE_SIZE: usize = 10_000;
        {
            let mut queue = self.action_queue.write();
            if queue.len() >= MAX_QUEUE_SIZE {
                warn!("Action queue full, dropping oldest action");
                queue.remove(0); // Remove oldest
            }
            queue.push(action.clone());
        }

        // Broadcast action (non-blocking, drops if channel full)
        if self.action_sender.send(action).is_err() {
            warn!("Action broadcast channel full, message dropped");
        }
        Ok(())
    }

    /// Get next action from queue
    pub fn pop_action(&self) -> Option<WorldAction> {
        let mut queue = self.action_queue.write();
        if queue.is_empty() {
            None
        } else {
            Some(queue.remove(0))
        }
    }

    /// Subscribe to actions
    pub fn subscribe(&self) -> broadcast::Receiver<WorldAction> {
        self.action_sender.subscribe()
    }

    /// Start listening to cognitive brain events
    pub async fn start_listening(&self) -> Result<(), Error> {
        let mut receiver = self.brain.subscribe();
        
        // Clone necessary components for async task
        let transformer = self.transformer.clone();
        let action_sender = self.action_sender.clone();
        let action_queue = self.action_queue.clone();
        
        // Spawn task to listen for cognitive events
        tokio::spawn(async move {
            const MAX_QUEUE_SIZE: usize = 10_000;
            loop {
                match receiver.recv().await {
                    Ok(event) => {
                        // Release lock before async operations
                        let action_opt = {
                            let transformer_guard = transformer.read();
                            transformer_guard.cognitive_to_world(&event).ok().flatten()
                        };
                        
                        if let Some(action) = action_opt {
                            // Prevent unbounded queue growth
                            {
                                let mut queue = action_queue.write();
                                if queue.len() >= MAX_QUEUE_SIZE {
                                    queue.remove(0); // Remove oldest
                                }
                                queue.push(action.clone());
                            }
                            
                            // Non-blocking send
                            if action_sender.send(action).is_err() {
                                warn!("Action broadcast channel full, message dropped");
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        warn!("Cognitive brain event channel closed");
                        break;
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!("Motor interface lagged, skipped {} events", skipped);
                    }
                }
            }
        });

        Ok(())
    }
}

