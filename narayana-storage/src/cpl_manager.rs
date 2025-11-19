// CPL Manager - Multi-instance support
// Manages multiple CPL instances with isolated state

use crate::cognitive::CognitiveBrain;
use crate::conscience_persistent_loop::{ConsciencePersistentLoop, CPLConfig, CPLEvent};
use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

/// CPL Manager - Manages multiple CPL instances
pub struct CPLManager {
    cpls: Arc<RwLock<HashMap<String, Arc<ConsciencePersistentLoop>>>>,
    shared_brain: Option<Arc<CognitiveBrain>>, // Optional shared brain
    default_config: CPLConfig,
}

impl CPLManager {
    /// Create new CPL Manager
    pub fn new(default_config: CPLConfig) -> Self {
        Self {
            cpls: Arc::new(RwLock::new(HashMap::new())),
            shared_brain: None,
            default_config,
        }
    }
    
    /// Set shared brain (optional - CPLs can share or have separate brains)
    pub fn set_shared_brain(&mut self, brain: Arc<CognitiveBrain>) {
        self.shared_brain = Some(brain);
    }
    
    /// Spawn a new CPL instance
    pub async fn spawn_cpl(&self, config: Option<CPLConfig>) -> Result<String> {
        let cpl_id = Uuid::new_v4().to_string();
        let config = config.unwrap_or_else(|| self.default_config.clone());
        
        // Create brain (shared or new)
        let brain = if let Some(ref shared) = self.shared_brain {
            shared.clone()
        } else {
            Arc::new(CognitiveBrain::new())
        };
        
        // Create CPL
        let cpl = Arc::new(ConsciencePersistentLoop::new(brain, config));
        
        // Initialize
        if let Err(e) = cpl.initialize().await {
            error!("Failed to initialize CPL {}: {}", cpl_id, e);
            return Err(e);
        }
        
        // Store
        self.cpls.write().insert(cpl_id.clone(), cpl);
        
        info!("Spawned CPL {}", cpl_id);
        Ok(cpl_id)
    }
    
    /// Start a CPL instance
    pub async fn start_cpl(&self, cpl_id: &str) -> Result<()> {
        let cpls = self.cpls.read();
        if let Some(cpl) = cpls.get(cpl_id) {
            cpl.clone().start().await
        } else {
            Err(Error::Storage(format!("CPL {} not found", cpl_id)))
        }
    }
    
    /// Stop a CPL instance
    pub async fn stop_cpl(&self, cpl_id: &str) -> Result<()> {
        let cpls = self.cpls.read();
        if let Some(cpl) = cpls.get(cpl_id) {
            cpl.stop().await
        } else {
            Err(Error::Storage(format!("CPL {} not found", cpl_id)))
        }
    }
    
    /// Remove a CPL instance
    pub async fn remove_cpl(&self, cpl_id: &str) -> Result<()> {
        let mut cpls = self.cpls.write();
        
        if let Some(cpl) = cpls.get(cpl_id) {
            // Stop before removing
            if cpl.is_running() {
                if let Err(e) = cpl.stop().await {
                    warn!("Failed to stop CPL before removal: {}", e);
                }
            }
        }
        
        if cpls.remove(cpl_id).is_some() {
            info!("Removed CPL {}", cpl_id);
            Ok(())
        } else {
            Err(Error::Storage(format!("CPL {} not found", cpl_id)))
        }
    }
    
    /// Get a CPL instance
    pub fn get_cpl(&self, cpl_id: &str) -> Option<Arc<ConsciencePersistentLoop>> {
        self.cpls.read().get(cpl_id).cloned()
    }
    
    /// List all CPL IDs
    pub fn list_cpls(&self) -> Vec<String> {
        self.cpls.read().keys().cloned().collect()
    }
    
    /// Get CPL count
    pub fn count(&self) -> usize {
        self.cpls.read().len()
    }
    
    /// Start all CPLs
    pub async fn start_all(&self) -> Result<()> {
        let cpls = self.cpls.read();
        let mut errors = Vec::new();
        
        for (id, cpl) in cpls.iter() {
            if let Err(e) = cpl.clone().start().await {
                error!("Failed to start CPL {}: {}", id, e);
                errors.push((id.clone(), e));
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(Error::Storage(format!("Failed to start {} CPLs", errors.len())))
        }
    }
    
    /// Stop all CPLs
    pub async fn stop_all(&self) -> Result<()> {
        let cpls = self.cpls.read();
        let mut errors = Vec::new();
        
        for (id, cpl) in cpls.iter() {
            if let Err(e) = cpl.stop().await {
                error!("Failed to stop CPL {}: {}", id, e);
                errors.push((id.clone(), e));
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(Error::Storage(format!("Failed to stop {} CPLs", errors.len())))
        }
    }
    
    /// Broadcast message to all CPLs (cross-CPL communication)
    pub async fn broadcast(&self, message: CPLEvent) -> Result<()> {
        let cpls = self.cpls.read();
        
        for (id, cpl) in cpls.iter() {
            // Each CPL can subscribe to events
            // For now, we'll just log - in production would use proper message passing
            debug!("Broadcasting to CPL {}", id);
        }
        
        Ok(())
    }
}

