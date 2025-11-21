//! Central Nervous System service

use crate::component::{ComponentInfo, ComponentId, ComponentState};
use crate::registry::{ComponentRegistry, RegistryEvent};
use crate::router::ActionRouter;
use crate::safety::{SafetyValidator, SafetyLevel};
use crate::config::CnsConfig;
use crate::error::CnsError;
#[cfg(feature = "wld-integration")]
use narayana_wld::event_transformer::{WorldAction, WorldEvent};
#[cfg(feature = "wld-integration")]
use narayana_wld::world_broker::WorldBrokerHandle;
use narayana_storage::conscience_persistent_loop::CPLEvent;
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::broadcast;
use tokio::time::{interval, Duration};
use tracing::{info, warn, debug, error};

/// Central Nervous System service
pub struct CentralNervousSystem {
    config: Arc<CnsConfig>,
    registry: Arc<ComponentRegistry>,
    router: Arc<ActionRouter>,
    safety_validator: Arc<RwLock<SafetyValidator>>,
    #[cfg(feature = "wld-integration")]
    action_sender: broadcast::Sender<WorldAction>,
    is_running: Arc<RwLock<bool>>,
    health_check_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl CentralNervousSystem {
    /// Create new CNS service
    pub fn new(config: CnsConfig) -> Result<Self, CnsError> {
        config.validate()
            .map_err(|e| CnsError::Config(e))?;
        
        let registry = Arc::new(ComponentRegistry::new(config.heartbeat_timeout_secs));
        let router = Arc::new(ActionRouter::new(
            registry.clone(),
            config.enable_load_balancing,
        ));
        
        let safety_validator = Arc::new(RwLock::new(SafetyValidator::new(
            config.default_safety_level,
        )));
        
        #[cfg(feature = "wld-integration")]
        let (action_sender, _) = broadcast::channel(config.max_action_queue_size);
        #[cfg(not(feature = "wld-integration"))]
        let _action_sender: broadcast::Sender<()> = broadcast::channel(0).0; // Placeholder
        
        Ok(Self {
            config: Arc::new(config),
            registry,
            router,
            safety_validator,
            #[cfg(feature = "wld-integration")]
            action_sender,
            is_running: Arc::new(RwLock::new(false)),
            health_check_handle: Arc::new(RwLock::new(None)),
        })
    }
    
    /// Start CNS service
    #[cfg(feature = "wld-integration")]
    pub async fn start(&self, broker_handle: WorldBrokerHandle) -> Result<(), CnsError> {
        {
            let mut is_running = self.is_running.write();
            if *is_running {
                return Err(CnsError::Registry("CNS already running".to_string()));
            }
            *is_running = true;
        }
        
        info!("Starting Central Nervous System");
        
        // Start health check task
        let registry = self.registry.clone();
        let heartbeat_timeout = self.config.heartbeat_timeout_secs;
        let is_running = self.is_running.clone();
        
        let health_check_handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));
            
            loop {
                interval.tick().await;
                
                if !*is_running.read() {
                    break;
                }
                
                // Check health of all components
                let unhealthy = registry.get_unhealthy_components();
                for component_id in unhealthy {
                    warn!("Component '{}' is unhealthy, marking as unavailable", component_id.as_str());
                    let _ = registry.update_state(&component_id, ComponentState::Unavailable);
                }
            }
        });
        
        *self.health_check_handle.write() = Some(health_check_handle);
        
        // Subscribe to CPL actions
        let mut action_receiver = broker_handle.subscribe_actions();
        let router = self.router.clone();
        let safety_validator = self.safety_validator.clone();
        let action_sender = self.action_sender.clone();
        let registry = self.registry.clone();
        let is_running = self.is_running.clone();
        
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    action_result = action_receiver.recv() => {
                        match action_result {
                            Ok(action) => {
                                // Route action through CNS
                                if let Err(e) = Self::process_action(
                                    &action,
                                    &router,
                                    &safety_validator,
                                    &registry,
                                    &action_sender,
                                ).await {
                                    warn!("Failed to process action: {}", e);
                                }
                            }
                            Err(_) => {
                                // Channel closed or lagged
                                break;
                            }
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(100)) => {
                        if !*is_running.read() {
                            break;
                        }
                    }
                }
            }
        });
        
        info!("Central Nervous System started");
        Ok(())
    }
    
    /// Stop CNS service
    pub async fn stop(&self) -> Result<(), CnsError> {
        {
            let mut is_running = self.is_running.write();
            if !*is_running {
                return Ok(());
            }
            *is_running = false;
        }
        
        // Stop health check
        if let Some(handle) = self.health_check_handle.write().take() {
            handle.abort();
        }
        
        info!("Central Nervous System stopped");
        Ok(())
    }
    
    /// Process action through CNS pipeline
    #[cfg(feature = "wld-integration")]
    async fn process_action(
        action: &WorldAction,
        router: &Arc<ActionRouter>,
        safety_validator: &Arc<RwLock<SafetyValidator>>,
        registry: &Arc<ComponentRegistry>,
        action_sender: &broadcast::Sender<WorldAction>,
    ) -> Result<(), CnsError> {
        // Extract target component
        let (target, command) = match action {
            WorldAction::ActuatorCommand { target, command } => {
                (target, command)
            }
            _ => {
                // Non-actuator commands don't need CNS processing
                return Ok(());
            }
        };
        
        // Find target component
        let component = if !target.is_empty() {
            registry.get_by_name(target)
                .or_else(|| {
                    // Try as component ID
                    registry.get(&ComponentId::from(target.as_str()))
                })
        } else {
            None
        };
        
        let component = match component {
            Some(comp) => comp,
            None => {
                // Try capability-based routing
                if let Some(capability) = ActionRouter::extract_capability_from_command(command) {
                    let components = registry.find_by_capability(&capability);
                    if let Some(comp) = components.first() {
                        comp.clone()
                    } else {
                        return Err(CnsError::Routing(format!(
                            "No component found for capability '{}'",
                            capability.name()
                        )));
                    }
                } else {
                    return Err(CnsError::Routing("No target component specified".to_string()));
                }
            }
        };
        
        // Safety validation
        let safety_validator_guard = safety_validator.read();
        let validation = safety_validator_guard.validate_action(action, &component);
        drop(safety_validator_guard);
        
        if !validation.is_safe {
            error!("Action failed safety validation: {:?}", validation.reasons);
            if validation.emergency_stop {
                let mut validator = safety_validator.write();
                validator.trigger_emergency_stop();
            }
            return Err(CnsError::Safety(format!(
                "Action unsafe: {}",
                validation.reasons.join(", ")
            )));
        }
        
        // Route action
        let component_ids = router.route_action(action, None)
            .map_err(|e| CnsError::Routing(e))?;
        
        // Dispatch to components
        for component_id in component_ids {
            // Create targeted action
            let targeted_action = WorldAction::ActuatorCommand {
                target: component_id.as_str().to_string(),
                command: command.clone(),
            };
            
            // Broadcast action (components will pick it up)
            if action_sender.send(targeted_action).is_err() {
                warn!("Action broadcast channel full, dropping action");
            }
        }
        
        Ok(())
    }
    
    /// Register a component
    pub fn register_component(&self, component: ComponentInfo) -> Result<(), CnsError> {
        self.registry.register(component)
    }
    
    /// Unregister a component
    pub fn unregister_component(&self, component_id: &ComponentId) -> Result<(), CnsError> {
        self.registry.unregister(component_id)
    }
    
    /// Get component by ID
    pub fn get_component(&self, component_id: &ComponentId) -> Option<ComponentInfo> {
        self.registry.get(component_id)
    }
    
    /// Find components by capability
    pub fn find_by_capability(&self, capability: &crate::capability::Capability) -> Vec<ComponentInfo> {
        self.registry.find_by_capability(capability)
    }
    
    /// Get safety validator
    pub fn safety_validator(&self) -> &Arc<RwLock<SafetyValidator>> {
        &self.safety_validator
    }
    
    /// Get registry
    pub fn registry(&self) -> &Arc<ComponentRegistry> {
        &self.registry
    }
    
    /// Get router
    pub fn router(&self) -> &Arc<ActionRouter> {
        &self.router
    }
    
    /// Subscribe to actions
    #[cfg(feature = "wld-integration")]
    pub fn subscribe_actions(&self) -> broadcast::Receiver<WorldAction> {
        self.action_sender.subscribe()
    }
    
    /// Subscribe to registry events
    pub fn subscribe_registry_events(&self) -> broadcast::Receiver<RegistryEvent> {
        self.registry.subscribe_events()
    }
}

// Helper function to extract capability (needs to be accessible)
impl ActionRouter {
    pub fn extract_capability_from_command(command: &serde_json::Value) -> Option<crate::capability::Capability> {
        // Try to extract capability name
        if let Some(cap_name) = command.get("capability")
            .or_else(|| command.get("action"))
            .or_else(|| command.get("command"))
            .and_then(|v| v.as_str())
        {
            return Some(crate::capability::Capability::Simple(cap_name.to_string()));
        }
        
        // Try structured capability
        if let Some(cap_obj) = command.get("capability").and_then(|v| v.as_object()) {
            if let Some(name) = cap_obj.get("name").and_then(|v| v.as_str()) {
                return Some(crate::capability::Capability::Simple(name.to_string()));
            }
        }
        
        None
    }
}

