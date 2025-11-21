//! Action router for capability-based routing

use crate::component::{ComponentInfo, ComponentId};
use crate::capability::Capability;
use crate::registry::ComponentRegistry;
#[cfg(feature = "wld-integration")]
use narayana_wld::event_transformer::WorldAction;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;

/// Routing strategy
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingStrategy {
    /// Route to first available component
    FirstAvailable,
    /// Route to least loaded component
    LeastLoaded,
    /// Route to all matching components (broadcast)
    Broadcast,
    /// Route to specific component by ID
    Specific(ComponentId),
}

/// Component load tracking
#[derive(Debug, Clone)]
struct ComponentLoad {
    /// Number of pending actions
    pending_actions: usize,
    /// Last action timestamp
    last_action_time: u64,
}

/// Action router
pub struct ActionRouter {
    registry: Arc<ComponentRegistry>,
    load_tracker: parking_lot::RwLock<HashMap<ComponentId, ComponentLoad>>,
    enable_load_balancing: bool,
}

impl ActionRouter {
    /// Create new action router
    pub fn new(registry: Arc<ComponentRegistry>, enable_load_balancing: bool) -> Self {
        Self {
            registry,
            load_tracker: parking_lot::RwLock::new(HashMap::new()),
            enable_load_balancing,
        }
    }
    
    /// Route action to appropriate component(s)
    #[cfg(feature = "wld-integration")]
    pub fn route_action(
        &self,
        action: &WorldAction,
        strategy: Option<RoutingStrategy>,
    ) -> Result<Vec<ComponentId>, String> {
        match action {
            WorldAction::ActuatorCommand { target, command } => {
                // Determine routing strategy
                let strategy = strategy.unwrap_or_else(|| {
                    // Try to extract capability from command
                    if let Some(capability) = Self::extract_capability_from_command(command) {
                        if self.enable_load_balancing {
                            RoutingStrategy::LeastLoaded
                        } else {
                            RoutingStrategy::FirstAvailable
                        }
                    } else {
                        // Fallback to specific target if provided
                        if !target.is_empty() {
                            if let Some(component) = self.registry.get_by_name(target) {
                                RoutingStrategy::Specific(component.id.clone())
                            } else {
                                RoutingStrategy::FirstAvailable
                            }
                        } else {
                            RoutingStrategy::FirstAvailable
                        }
                    }
                });
                
                match strategy {
                    RoutingStrategy::Specific(component_id) => {
                        // Route to specific component
                        if let Some(component) = self.registry.get(&component_id) {
                            if component.is_available() {
                                self.track_action(&component_id);
                                Ok(vec![component_id])
                            } else {
                                Err(format!("Component '{}' is not available", component_id.as_str()))
                            }
                        } else {
                            Err(format!("Component '{}' not found", component_id.as_str()))
                        }
                    }
                    RoutingStrategy::FirstAvailable => {
                        // Find first available component with matching capability
                        if let Some(capability) = Self::extract_capability_from_command(command) {
                            let components = self.registry.find_by_capability(&capability);
                            if let Some(component) = components.first() {
                                self.track_action(&component.id);
                                Ok(vec![component.id.clone()])
                            } else {
                                Err(format!("No available component with capability '{}'", capability.name()))
                            }
                        } else if !target.is_empty() {
                            // Fallback to target name
                            if let Some(component) = self.registry.get_by_name(target) {
                                if component.is_available() {
                                    self.track_action(&component.id);
                                    Ok(vec![component.id.clone()])
                                } else {
                                    Err(format!("Component '{}' is not available", target))
                                }
                            } else {
                                Err(format!("Component '{}' not found", target))
                            }
                        } else {
                            Err("No target specified and no capability found in command".to_string())
                        }
                    }
                    RoutingStrategy::LeastLoaded => {
                        // Find least loaded component with matching capability
                        if let Some(capability) = Self::extract_capability_from_command(command) {
                            let components = self.registry.find_by_capability(&capability);
                            if components.is_empty() {
                                return Err(format!("No available component with capability '{}'", capability.name()));
                            }
                            
                            let component = if self.enable_load_balancing {
                                self.select_least_loaded(&components)
                            } else {
                                components.first()
                            };
                            
                            if let Some(component) = component {
                                self.track_action(&component.id);
                                Ok(vec![component.id.clone()])
                            } else {
                                Err("No suitable component found".to_string())
                            }
                        } else {
                            Err("No capability found in command for load balancing".to_string())
                        }
                    }
                    RoutingStrategy::Broadcast => {
                        // Route to all matching components
                        if let Some(capability) = Self::extract_capability_from_command(command) {
                            let components = self.registry.find_by_capability(&capability);
                            let component_ids: Vec<ComponentId> = components
                                .iter()
                                .map(|c| c.id.clone())
                                .collect();
                            
                            for component_id in &component_ids {
                                self.track_action(component_id);
                            }
                            
                            if component_ids.is_empty() {
                                Err(format!("No available component with capability '{}'", capability.name()))
                            } else {
                                Ok(component_ids)
                            }
                        } else {
                            Err("No capability found in command for broadcast".to_string())
                        }
                    }
                }
            }
            _ => {
                // Non-actuator commands don't need routing
                Ok(Vec::new())
            }
        }
    }
    
    /// Select least loaded component
    fn select_least_loaded<'a>(&self, components: &'a [ComponentInfo]) -> Option<&'a ComponentInfo> {
        let load_tracker = self.load_tracker.read();
        
        // Collect loads first to avoid lifetime issues
        let loads: Vec<usize> = components.iter()
            .map(|comp| {
                load_tracker
                    .get(&comp.id)
                    .map(|load| load.pending_actions)
                    .unwrap_or(0)
            })
            .collect();
        
        // Find index of minimum load
        loads.iter()
            .enumerate()
            .min_by_key(|(_, &load)| load)
            .map(|(idx, _)| &components[idx])
    }
    
    /// Track action for load balancing
    fn track_action(&self, component_id: &ComponentId) {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let mut load_tracker = self.load_tracker.write();
        let load = load_tracker.entry(component_id.clone()).or_insert_with(|| ComponentLoad {
            pending_actions: 0,
            last_action_time: now,
        });
        
        load.pending_actions = load.pending_actions.saturating_add(1);
        load.last_action_time = now;
    }
    
    /// Mark action as completed (for load tracking)
    pub fn action_completed(&self, component_id: &ComponentId) {
        let mut load_tracker = self.load_tracker.write();
        if let Some(load) = load_tracker.get_mut(component_id) {
            load.pending_actions = load.pending_actions.saturating_sub(1);
        }
    }
    
    /// Get component load
    pub fn get_component_load(&self, component_id: &ComponentId) -> usize {
        let load_tracker = self.load_tracker.read();
        load_tracker
            .get(component_id)
            .map(|load| load.pending_actions)
            .unwrap_or(0)
    }
}

