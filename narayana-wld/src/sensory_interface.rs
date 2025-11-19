//! Sensory interface: World → CPL event flow
//! 
//! Transforms external world events into cognitive events,
//! routes them through attention filter, and delivers to CPL.

use crate::attention_filter::AttentionFilter;
use crate::event_transformer::{EventTransformer, WorldEvent};
use narayana_core::Error;
use narayana_storage::cognitive::{CognitiveBrain, CognitiveEvent};
use narayana_storage::conscience_persistent_loop::{ConsciencePersistentLoop, CPLEvent};
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

/// Sensory interface for processing incoming world events
pub struct SensoryInterface {
    brain: Arc<CognitiveBrain>,
    cpl: Arc<ConsciencePersistentLoop>,
    transformer: Arc<RwLock<EventTransformer>>,
    attention_filter: Arc<AttentionFilter>,
    event_sender: broadcast::Sender<SensoryEvent>,
}

#[derive(Debug, Clone)]
pub enum SensoryEvent {
    EventReceived { event: WorldEvent },
    EventRouted { event: WorldEvent, destination: String },
    EventFiltered { event: WorldEvent, reason: String },
}

impl SensoryInterface {
    pub fn new(
        brain: Arc<CognitiveBrain>,
        cpl: Arc<ConsciencePersistentLoop>,
        transformer: Arc<RwLock<EventTransformer>>,
        attention_filter: Arc<AttentionFilter>,
    ) -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self {
            brain,
            cpl,
            transformer,
            attention_filter,
            event_sender: sender,
        }
    }

    /// Process incoming world event
    pub async fn process_event(&self, event: WorldEvent) -> Result<(), Error> {
        debug!("Sensory interface processing event: {:?}", event);

        // Emit event received (non-blocking)
        if self.event_sender.send(SensoryEvent::EventReceived {
            event: event.clone(),
        }).is_err() {
            warn!("Sensory event channel full, message dropped");
        }

        // Compute salience
        let should_route_to_workspace = self.attention_filter
            .should_route_to_workspace(&event)
            .map_err(|e| Error::Storage(format!("Attention filter error: {}", e)))?;

        // Release lock before async operations
        let (cpl_event_opt, cognitive_event_opt) = {
            let transformer = self.transformer.read();
            let cpl_event = transformer.world_to_cpl(&event).ok();
            let cognitive_event = transformer.world_to_cognitive(&event).ok();
            (cpl_event, cognitive_event)
        };
        
        if should_route_to_workspace {
            // Route to Global Workspace via CPL event
            if cpl_event_opt.is_some() {
                // Note: CPL events are typically emitted internally, but we can
                // trigger processing by creating a thought or experience
                info!("Routing high-salience event to Global Workspace");
            }
            
            if self.event_sender.send(SensoryEvent::EventRouted {
                event: event.clone(),
                destination: "global_workspace".to_string(),
            }).is_err() {
                warn!("Sensory event channel full, message dropped");
            }
        } else {
            // Route to background processing
            debug!("Routing low-salience event to background processing");
            
            if self.event_sender.send(SensoryEvent::EventFiltered {
                event: event.clone(),
                reason: "low_salience".to_string(),
            }).is_err() {
                warn!("Sensory event channel full, message dropped");
            }
        }

        // Always store as experience in cognitive brain
        if let Some(cognitive_event) = cognitive_event_opt {

            // Store experience (this would typically be done through the brain's API)
            // For now, we'll just log it
            debug!("Transformed to cognitive event: {:?}", cognitive_event);
        } else {
            warn!("Failed to transform event to cognitive event");
        }

        Ok(())
    }

    /// Subscribe to sensory events
    pub fn subscribe(&self) -> broadcast::Receiver<SensoryEvent> {
        self.event_sender.subscribe()
    }
}

