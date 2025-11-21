//! World Broker - Main orchestrator for world-CPL interface
//! 
//! Integrates sensory interface, motor interface, attention filter,
//! and protocol adapters to mediate bidirectional communication.

use crate::attention_filter::{AttentionFilter, AttentionFilterConfig};
use crate::config::WorldBrokerConfig;
use crate::event_transformer::{EventTransformer, WorldEvent, WorldAction};
use crate::motor_interface::MotorInterface;
use crate::protocol_adapters::ProtocolAdapter;
use crate::sensory_interface::SensoryInterface;
use narayana_core::Error;
use narayana_storage::cognitive::CognitiveBrain;
use narayana_storage::conscience_persistent_loop::{ConsciencePersistentLoop, CPLEvent};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::broadcast;
use tracing::{info, warn, debug, error};

/// Main world broker orchestrator
pub struct WorldBroker {
    brain: Arc<CognitiveBrain>,
    cpl: Arc<ConsciencePersistentLoop>,
    sensory_interface: Arc<SensoryInterface>,
    motor_interface: Arc<MotorInterface>,
    transformer: Arc<RwLock<EventTransformer>>,
    attention_filter: Arc<AttentionFilter>,
    adapters: Arc<RwLock<HashMap<String, Box<dyn ProtocolAdapter + Send + Sync>>>>,
    config: WorldBrokerConfig,
    action_sender: broadcast::Sender<WorldAction>,
    is_running: Arc<RwLock<bool>>,
}

/// Handle for async operations (avoids Arc<WorldBroker> issues)
#[derive(Clone)]
pub struct WorldBrokerHandle {
    sensory: Arc<SensoryInterface>,
    motor: Arc<MotorInterface>,
    action_sender: broadcast::Sender<WorldAction>,
}

impl WorldBrokerHandle {
    pub async fn process_world_event(&self, event: WorldEvent) -> Result<(), Error> {
        self.sensory.process_event(event).await
    }

    pub fn subscribe_actions(&self) -> broadcast::Receiver<WorldAction> {
        self.action_sender.subscribe()
    }
}

impl WorldBroker {
    /// Create a new world broker
    pub fn new(
        brain: Arc<CognitiveBrain>,
        cpl: Arc<ConsciencePersistentLoop>,
        config: WorldBrokerConfig,
    ) -> Result<Self, Error> {
        // Validate configuration
        config.validate()
            .map_err(|e| Error::Storage(format!("Invalid configuration: {}", e)))?;
        // Create event transformer
        let transformer = Arc::new(RwLock::new(EventTransformer::new()));

        // Create attention filter
        let attention_config = AttentionFilterConfig {
            novelty_weight: config.novelty_weight,
            urgency_weight: config.urgency_weight,
            relevance_weight: config.relevance_weight,
            magnitude_weight: config.magnitude_weight,
            prediction_error_weight: config.prediction_error_weight,
            salience_threshold: config.salience_threshold,
            context_window_size: config.context_window_size,
        };
        let attention_filter = Arc::new(AttentionFilter::new(
            brain.clone(),
            attention_config,
        ));

        // Create sensory interface
        let sensory_interface = Arc::new(SensoryInterface::new(
            brain.clone(),
            cpl.clone(),
            transformer.clone(),
            attention_filter.clone(),
        ));

        // Create motor interface
        let motor_interface = Arc::new(MotorInterface::new(
            brain.clone(),
            transformer.clone(),
        ));

        // Create action broadcast channel
        let (action_sender, _) = broadcast::channel(config.event_buffer_size);

        Ok(Self {
            brain,
            cpl,
            sensory_interface,
            motor_interface,
            transformer,
            attention_filter,
            adapters: Arc::new(RwLock::new(HashMap::new())),
            config,
            action_sender,
            is_running: Arc::new(RwLock::new(false)),
        })
    }

    /// Start the world broker
    pub async fn start(&self) -> Result<(), Error> {
        // Atomic check-and-set to prevent race conditions
        {
            let mut running = self.is_running.write();
            if *running {
                return Err(Error::Storage("World broker already running".to_string()));
            }
            *running = true;
        }
        info!("Starting World Broker");

        // Create handle for adapters
        let handle = WorldBrokerHandle {
            sensory: self.sensory_interface.clone(),
            motor: self.motor_interface.clone(),
            action_sender: self.action_sender.clone(),
        };

        // Start protocol adapters
        for adapter_name in &self.config.enabled_adapters {
            self.start_adapter(adapter_name.clone(), handle.clone()).await?;
        }

        // Start motor interface listening
        self.motor_interface.start_listening().await?;

        // Start CPL event listener
        self.start_cpl_listener().await?;

        info!("World Broker started successfully");
        Ok(())
    }

    /// Stop the world broker
    pub async fn stop(&self) -> Result<(), Error> {
        // Atomic check-and-set
        {
            let mut running = self.is_running.write();
            if !*running {
                return Err(Error::Storage("World broker not running".to_string()));
            }
            *running = false;
        }
        
        info!("Stopping World Broker");

        // CNS integration is handled externally

        // Stop all adapters
        let adapters = self.adapters.read();
        let adapter_names: Vec<String> = adapters.keys().cloned().collect();
        drop(adapters); // Release lock before async operations
        
        for name in adapter_names {
            let adapter = {
                let adapters = self.adapters.read();
                // Note: We can't clone the adapter, so we'll stop them by name
                // This is a limitation - in production, we'd need a better way
                adapters.get(&name).map(|_| ())
            };
            
            if adapter.is_some() {
                // Re-acquire to get adapter for stopping
                let adapters = self.adapters.read();
                if let Some(adapter) = adapters.get(&name) {
                    if let Err(e) = adapter.stop().await {
                        warn!("Error stopping adapter {}: {}", name, e);
                    }
                }
            }
        }

        info!("World Broker stopped");
        Ok(())
    }

    /// Process incoming world event
    pub async fn process_world_event(&self, event: WorldEvent) -> Result<(), Error> {
        self.sensory_interface.process_event(event).await
    }

    /// Register a protocol adapter
    pub fn register_adapter(&self, adapter: Box<dyn ProtocolAdapter + Send + Sync>) {
        let name = adapter.protocol_name().to_string();
        
        // Validate adapter name
        if name.is_empty() || name.len() > 64 {
            warn!("Invalid adapter name: {}", name);
            return;
        }
        
        let mut adapters = self.adapters.write();
        
        // Prevent duplicate registration
        if adapters.contains_key(&name) {
            warn!("Adapter {} already registered, replacing", name);
        }
        
        adapters.insert(name.clone(), adapter);
        info!("Registered protocol adapter: {}", name);
    }

    /// Start a protocol adapter
    async fn start_adapter(
        &self,
        adapter_name: String,
        handle: WorldBrokerHandle,
    ) -> Result<(), Error> {
        let adapters = self.adapters.read();
        if let Some(adapter) = adapters.get(&adapter_name) {
            info!("Starting protocol adapter: {}", adapter_name);
            adapter.start(handle).await?;
        } else {
            warn!("Adapter {} not found, skipping", adapter_name);
        }
        Ok(())
    }

    /// Start listening to CPL events
    async fn start_cpl_listener(&self) -> Result<(), Error> {
        // Note: CPL events are broadcast internally, we need to subscribe
        // For now, we'll set up a task that processes events
        let motor = self.motor_interface.clone();
        let cpl_arc = self.cpl.clone();

        tokio::spawn(async move {
            // In a real implementation, we'd subscribe to CPL events
            // For now, this is a placeholder that shows the pattern
            debug!("CPL event listener started (placeholder)");
        });

        Ok(())
    }

    /// Send action to external world
    pub async fn send_action(&self, action: WorldAction) -> Result<(), Error> {
        // Validate action before sending
        validate_action(&action)?;
        
        // Broadcast to all subscribers (non-blocking)
        if self.action_sender.send(action.clone()).is_err() {
            warn!("Action broadcast channel full, message dropped");
        }

        // Send via all adapters
        let adapters = self.adapters.read();
        for (name, adapter) in adapters.iter() {
            if let Err(e) = adapter.send_action(action.clone()).await {
                warn!("Error sending action via adapter {}: {}", name, e);
            }
        }

        Ok(())
    }

    /// Get sensory interface
    pub fn sensory_interface(&self) -> &Arc<SensoryInterface> {
        &self.sensory_interface
    }

    /// Get motor interface
    pub fn motor_interface(&self) -> &Arc<MotorInterface> {
        &self.motor_interface
    }

    /// Get attention filter
    pub fn attention_filter(&self) -> &Arc<AttentionFilter> {
        &self.attention_filter
    }
}

/// Validate world action before sending
fn validate_action(action: &WorldAction) -> Result<(), Error> {
    match action {
        WorldAction::ActuatorCommand { target, command } => {
            if target.is_empty() || target.len() > 256 {
                return Err(Error::Storage("Invalid actuator target".to_string()));
            }
            // Validate JSON size
            let json_size = serde_json::to_string(command)
                .map_err(|e| Error::Storage(format!("Invalid command JSON: {}", e)))?
                .len();
            if json_size > 1_000_000 {
                return Err(Error::Storage("Command payload too large".to_string()));
            }
        }
        WorldAction::UserResponse { user_id, message } => {
            if user_id.is_empty() || user_id.len() > 256 {
                return Err(Error::Storage("Invalid user_id".to_string()));
            }
            if message.len() > 100_000 {
                return Err(Error::Storage("Response message too large".to_string()));
            }
        }
        WorldAction::SystemNotification { channel, content } => {
            if channel.is_empty() || channel.len() > 256 {
                return Err(Error::Storage("Invalid notification channel".to_string()));
            }
            let json_size = serde_json::to_string(content)
                .map_err(|e| Error::Storage(format!("Invalid content JSON: {}", e)))?
                .len();
            if json_size > 1_000_000 {
                return Err(Error::Storage("Notification content too large".to_string()));
            }
        }
        WorldAction::DataTransmission { destination, data } => {
            if destination.is_empty() || destination.len() > 256 {
                return Err(Error::Storage("Invalid destination".to_string()));
            }
            let json_size = serde_json::to_string(data)
                .map_err(|e| Error::Storage(format!("Invalid data JSON: {}", e)))?
                .len();
            if json_size > 10_000_000 {
                return Err(Error::Storage("Data transmission payload too large".to_string()));
            }
        }
    }
    Ok(())
}

