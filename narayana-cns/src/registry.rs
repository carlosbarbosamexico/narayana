//! Component registry for registration and discovery

use crate::component::{ComponentInfo, ComponentId, ComponentType, ComponentState};
use crate::capability::Capability;
use crate::error::CnsError;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, warn, debug};

/// Component registry event
#[derive(Debug, Clone)]
pub enum RegistryEvent {
    /// Component registered
    ComponentRegistered {
        component_id: ComponentId,
        component_name: String,
    },
    /// Component unregistered
    ComponentUnregistered {
        component_id: ComponentId,
    },
    /// Component state changed
    ComponentStateChanged {
        component_id: ComponentId,
        old_state: ComponentState,
        new_state: ComponentState,
    },
    /// Component heartbeat received
    ComponentHeartbeat {
        component_id: ComponentId,
    },
}

/// Component registry
pub struct ComponentRegistry {
    /// Registered components
    components: Arc<RwLock<HashMap<ComponentId, ComponentInfo>>>,
    /// Component index by name
    by_name: Arc<RwLock<HashMap<String, ComponentId>>>,
    /// Component index by capability
    by_capability: Arc<RwLock<HashMap<Capability, Vec<ComponentId>>>>,
    /// Component index by type
    by_type: Arc<RwLock<HashMap<ComponentType, Vec<ComponentId>>>>,
    /// Event broadcaster
    event_sender: broadcast::Sender<RegistryEvent>,
    /// Heartbeat timeout in seconds
    heartbeat_timeout_secs: u64,
}

impl ComponentRegistry {
    /// Create new component registry
    pub fn new(heartbeat_timeout_secs: u64) -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self {
            components: Arc::new(RwLock::new(HashMap::new())),
            by_name: Arc::new(RwLock::new(HashMap::new())),
            by_capability: Arc::new(RwLock::new(HashMap::new())),
            by_type: Arc::new(RwLock::new(HashMap::new())),
            event_sender: sender,
            heartbeat_timeout_secs,
        }
    }
    
    /// Register a component
    pub fn register(&self, component: ComponentInfo) -> Result<(), CnsError> {
        let component_id = component.id.clone();
        let component_name = component.name.clone();
        
        // Validate component
        if component.name.is_empty() {
            return Err(CnsError::Registry("Component name cannot be empty".to_string()));
        }
        
        if component.capabilities.is_empty() {
            return Err(CnsError::Registry("Component must have at least one capability".to_string()));
        }
        
        // Check for duplicate ID
        {
            let components = self.components.read();
            if components.contains_key(&component_id) {
                return Err(CnsError::Registry(format!(
                    "Component with ID '{}' already registered",
                    component_id.as_str()
                )));
            }
        }
        
        // Check for duplicate name
        {
            let by_name = self.by_name.read();
            if by_name.contains_key(&component_name) {
                return Err(CnsError::Registry(format!(
                    "Component with name '{}' already registered",
                    component_name
                )));
            }
        }
        
        // Register component
        {
            let mut components = self.components.write();
            components.insert(component_id.clone(), component.clone());
        }
        
        // Index by name
        {
            let mut by_name = self.by_name.write();
            by_name.insert(component_name.clone(), component_id.clone());
        }
        
        // Index by capabilities
        {
            let mut by_capability = self.by_capability.write();
            for capability in &component.capabilities {
                by_capability
                    .entry(capability.clone())
                    .or_insert_with(Vec::new)
                    .push(component_id.clone());
            }
        }
        
        // Index by type
        {
            let mut by_type = self.by_type.write();
            by_type
                .entry(component.component_type)
                .or_insert_with(Vec::new)
                .push(component_id.clone());
        }
        
        // Broadcast event
        let _ = self.event_sender.send(RegistryEvent::ComponentRegistered {
            component_id: component_id.clone(),
            component_name: component_name.clone(),
        });
        
        info!("Component registered: {} ({})", component_name, component_id.as_str());
        
        Ok(())
    }
    
    /// Unregister a component
    pub fn unregister(&self, component_id: &ComponentId) -> Result<(), CnsError> {
        // Get component info before removing
        let component = {
            let components = self.components.read();
            components.get(component_id).cloned()
        };
        
        let component = component.ok_or_else(|| {
            CnsError::Registry(format!("Component '{}' not found", component_id.as_str()))
        })?;
        
        // Remove from main registry
        {
            let mut components = self.components.write();
            components.remove(component_id);
        }
        
        // Remove from name index
        {
            let mut by_name = self.by_name.write();
            by_name.remove(&component.name);
        }
        
        // Remove from capability index
        {
            let mut by_capability = self.by_capability.write();
            for capability in &component.capabilities {
                if let Some(ids) = by_capability.get_mut(capability) {
                    ids.retain(|id| id != component_id);
                    if ids.is_empty() {
                        by_capability.remove(capability);
                    }
                }
            }
        }
        
        // Remove from type index
        {
            let mut by_type = self.by_type.write();
            if let Some(ids) = by_type.get_mut(&component.component_type) {
                ids.retain(|id| id != component_id);
                if ids.is_empty() {
                    by_type.remove(&component.component_type);
                }
            }
        }
        
        // Broadcast event
        let _ = self.event_sender.send(RegistryEvent::ComponentUnregistered {
            component_id: component_id.clone(),
        });
        
        info!("Component unregistered: {}", component_id.as_str());
        
        Ok(())
    }
    
    /// Get component by ID
    pub fn get(&self, component_id: &ComponentId) -> Option<ComponentInfo> {
        let components = self.components.read();
        components.get(component_id).cloned()
    }
    
    /// Get component by name
    pub fn get_by_name(&self, name: &str) -> Option<ComponentInfo> {
        let by_name = self.by_name.read();
        if let Some(component_id) = by_name.get(name) {
            self.get(component_id)
        } else {
            None
        }
    }
    
    /// Find components by capability
    pub fn find_by_capability(&self, capability: &Capability) -> Vec<ComponentInfo> {
        let by_capability = self.by_capability.read();
        if let Some(ids) = by_capability.get(capability) {
            let components = self.components.read();
            ids.iter()
                .filter_map(|id| components.get(id).cloned())
                .filter(|comp| comp.is_available())
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Find components by type
    pub fn find_by_type(&self, component_type: ComponentType) -> Vec<ComponentInfo> {
        let by_type = self.by_type.read();
        if let Some(ids) = by_type.get(&component_type) {
            let components = self.components.read();
            ids.iter()
                .filter_map(|id| components.get(id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Get all components
    pub fn get_all(&self) -> Vec<ComponentInfo> {
        let components = self.components.read();
        components.values().cloned().collect()
    }
    
    /// Update component state
    pub fn update_state(&self, component_id: &ComponentId, new_state: ComponentState) -> Result<(), CnsError> {
        let old_state = {
            let mut components = self.components.write();
            if let Some(component) = components.get_mut(component_id) {
                let old = component.state.clone();
                component.state = new_state.clone();
                old
            } else {
                return Err(CnsError::Registry(format!("Component '{}' not found", component_id.as_str())));
            }
        };
        
        // Broadcast event
        let _ = self.event_sender.send(RegistryEvent::ComponentStateChanged {
            component_id: component_id.clone(),
            old_state,
            new_state,
        });
        
        Ok(())
    }
    
    /// Update component heartbeat
    pub fn update_heartbeat(&self, component_id: &ComponentId) -> Result<(), CnsError> {
        {
            let mut components = self.components.write();
            if let Some(component) = components.get_mut(component_id) {
                component.update_heartbeat();
            } else {
                return Err(CnsError::Registry(format!("Component '{}' not found", component_id.as_str())));
            }
        }
        
        // Broadcast event
        let _ = self.event_sender.send(RegistryEvent::ComponentHeartbeat {
            component_id: component_id.clone(),
        });
        
        Ok(())
    }
    
    /// Check component health and update state if needed
    pub fn check_health(&self, component_id: &ComponentId) -> bool {
        let is_healthy = {
            let components = self.components.read();
            if let Some(component) = components.get(component_id) {
                component.is_healthy(self.heartbeat_timeout_secs)
            } else {
                false
            }
        };
        
        if !is_healthy {
            // Update state to unavailable if unhealthy
            if let Err(e) = self.update_state(component_id, ComponentState::Unavailable) {
                warn!("Failed to update component state: {}", e);
            }
        }
        
        is_healthy
    }
    
    /// Get all unhealthy components
    pub fn get_unhealthy_components(&self) -> Vec<ComponentId> {
        let components = self.components.read();
        components
            .values()
            .filter(|comp| !comp.is_healthy(self.heartbeat_timeout_secs))
            .map(|comp| comp.id.clone())
            .collect()
    }
    
    /// Subscribe to registry events
    pub fn subscribe_events(&self) -> broadcast::Receiver<RegistryEvent> {
        self.event_sender.subscribe()
    }
    
    /// Get component count
    pub fn count(&self) -> usize {
        let components = self.components.read();
        components.len()
    }
    
    /// Get available component count
    pub fn available_count(&self) -> usize {
        let components = self.components.read();
        components.values().filter(|c| c.is_available()).count()
    }
}

