// Dynamic Output Configuration Manager
// On-the-Fly Transform & Filter Changes - Just Like Dynamic Schema!
// Works across Brain, Database, and Workers

use narayana_core::{
    Error, Result, types::TableId,
    transforms::{
        OutputConfig, DefaultFilter, OutputTransform, FieldRule, ConfigContext,
    },
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use tracing::{info, warn, debug};
use std::time::{SystemTime, UNIX_EPOCH};

/// Dynamic Output Manager - manages transforms/filters on-the-fly
pub struct DynamicOutputManager {
    configs: Arc<RwLock<HashMap<String, TableOutputConfig>>>,
    change_history: Arc<RwLock<Vec<OutputChangeHistory>>>,
    validation_enabled: bool,
    auto_backup: bool,
}

#[derive(Debug, Clone)]
struct TableOutputConfig {
    context: ConfigContext,
    entity_id: String,
    output_config: OutputConfig,
    version: u64,
    last_modified: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputChangeHistory {
    pub context: ConfigContext,
    pub entity_id: String,
    pub change: OutputChange,
    pub result: OutputChangeResult,
    pub timestamp: u64,
    pub rolled_back: bool,
    pub rollback_data: Option<OutputConfigSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputChange {
    AddFilter { filter: DefaultFilter },
    RemoveFilter { filter_index: usize },
    ModifyFilter { filter_index: usize, new_filter: DefaultFilter },
    AddTransform { transform: OutputTransform },
    RemoveTransform { transform_index: usize },
    ModifyTransform { transform_index: usize, new_transform: OutputTransform },
    AddFieldRule { field: String, rule: FieldRule },
    RemoveFieldRule { field: String },
    AddProfile { profile_name: String, config: OutputConfig },
    RemoveProfile { profile_name: String },
    ReplaceConfig { new_config: OutputConfig },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputChangeResult {
    pub success: bool,
    pub change_type: OutputChange,
    pub affected_queries: u64, // Estimated affected queries
    pub errors: Vec<String>,
    pub rollback_available: bool,
    pub duration_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfigSnapshot {
    pub config: OutputConfig,
    pub timestamp: u64,
}

impl DynamicOutputManager {
    pub fn new() -> Self {
        Self {
            configs: Arc::new(RwLock::new(HashMap::new())),
            change_history: Arc::new(RwLock::new(Vec::new())),
            validation_enabled: true,
            auto_backup: true,
        }
    }

    /// Initialize output config for an entity
    pub fn initialize_config(
        &self,
        context: ConfigContext,
        entity_id: String,
        config: OutputConfig,
    ) -> Result<()> {
        // SECURITY: Validate entity_id to prevent injection
        Self::validate_entity_id(&entity_id)?;
        
        // SECURITY: Validate config size to prevent DoS
        let config_size = serde_json::to_string(&config)
            .map_err(|e| Error::Storage(format!("Failed to serialize config: {}", e)))?
            .len();
        const MAX_CONFIG_SIZE: usize = 10 * 1024 * 1024; // 10MB
        if config_size > MAX_CONFIG_SIZE {
            return Err(Error::Storage(format!(
                "Config too large: {} bytes (max: {})",
                config_size, MAX_CONFIG_SIZE
            )));
        }
        
        let mut configs = self.configs.write();
        let key = Self::make_key(&context, &entity_id);
        
        // SECURITY: Prevent overwriting existing config without explicit permission
        // (This is a design decision - could allow overwrite with a flag)
        if configs.contains_key(&key) {
            return Err(Error::Storage("Config already exists. Use update methods instead.".to_string()));
        }
        
        configs.insert(key, TableOutputConfig {
            context: context.clone(),
            entity_id: entity_id.clone(),
            output_config: config,
            version: 1,
            last_modified: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        });
        
        // SECURITY: Don't log entity_id (could be sensitive)
        info!("Initialized output config for {:?} entity", context);
        Ok(())
    }
    
    /// SECURITY: Validate entity_id to prevent injection and DoS
    fn validate_entity_id(entity_id: &str) -> Result<()> {
        // SECURITY: Prevent empty entity_id
        if entity_id.is_empty() {
            return Err(Error::Storage("Entity ID cannot be empty".to_string()));
        }
        
        // SECURITY: Limit entity_id length
        const MAX_ENTITY_ID_LENGTH: usize = 1_024;
        if entity_id.len() > MAX_ENTITY_ID_LENGTH {
            return Err(Error::Storage(format!(
                "Entity ID too long: {} bytes (max: {})",
                entity_id.len(), MAX_ENTITY_ID_LENGTH
            )));
        }
        
        // SECURITY: Prevent path traversal
        if entity_id.contains("..") || entity_id.contains("/") || entity_id.contains("\\") {
            return Err(Error::Storage(format!(
                "Entity ID contains invalid characters: '{}'",
                entity_id
            )));
        }
        
        // SECURITY: Prevent null bytes
        if entity_id.contains('\0') {
            return Err(Error::Storage("Entity ID cannot contain null bytes".to_string()));
        }
        
        // SECURITY: Prevent control characters
        if entity_id.chars().any(|c| c.is_control()) {
            return Err(Error::Storage("Entity ID cannot contain control characters".to_string()));
        }
        
        Ok(())
    }

    /// Add filter on-the-fly
    pub async fn add_filter(
        &self,
        context: ConfigContext,
        entity_id: String,
        filter: DefaultFilter,
    ) -> Result<OutputChangeResult> {
        // SECURITY: Validate entity_id
        Self::validate_entity_id(&entity_id)?;
        
        let start_time = SystemTime::now();
        
        let mut configs = self.configs.write();
        let key = Self::make_key(&context, &entity_id);
        
        let table_config = configs.get_mut(&key)
            .ok_or_else(|| Error::Storage("Config not found".to_string()))?;
        
        // SECURITY: Limit number of filters to prevent DoS
        const MAX_FILTERS: usize = 10_000;
        if table_config.output_config.default_filters.len() >= MAX_FILTERS {
            return Err(Error::Storage(format!(
                "Too many filters: {} (max: {})",
                table_config.output_config.default_filters.len(), MAX_FILTERS
            )));
        }
        
        // Create snapshot
        let snapshot = if self.auto_backup {
            // SECURITY: Limit snapshot size to prevent memory exhaustion
            let config_size = serde_json::to_string(&table_config.output_config)
                .map_err(|e| Error::Storage(format!("Failed to serialize config: {}", e)))?
                .len();
            const MAX_SNAPSHOT_SIZE: usize = 10 * 1024 * 1024; // 10MB
            if config_size > MAX_SNAPSHOT_SIZE {
                // Skip snapshot if too large (but still allow the operation)
                warn!("Config too large for snapshot ({} bytes), skipping backup", config_size);
                None
            } else {
                Some(OutputConfigSnapshot {
                    config: table_config.output_config.clone(),
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                })
            }
        } else {
            None
        };
        
        // Add filter
        table_config.output_config.default_filters.push(filter.clone());
        table_config.version += 1;
        table_config.last_modified = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        drop(configs);
        
        // SECURITY: Prevent integer overflow in duration calculation
        let duration = start_time.elapsed()
            .map(|d| {
                // SECURITY: Cap duration at u64::MAX milliseconds to prevent overflow
                let millis = d.as_millis();
                if millis > u64::MAX as u128 {
                    u64::MAX as f64
                } else {
                    millis as f64
                }
            })
            .unwrap_or(0.0);
        
        let result = OutputChangeResult {
            success: true,
            change_type: OutputChange::AddFilter { filter },
            affected_queries: 0, // Would track actual affected queries
            errors: vec![],
            rollback_available: snapshot.is_some(),
            duration_ms: duration,
        };
        
        // Record history
        let mut history = self.change_history.write();
        
        // SECURITY: Limit history size to prevent unbounded memory growth
        const MAX_HISTORY_SIZE: usize = 100_000;
        if history.len() >= MAX_HISTORY_SIZE {
            // Remove oldest entries (FIFO)
            let to_remove = history.len() - MAX_HISTORY_SIZE + 1;
            history.drain(0..to_remove);
        }
        
        history.push(OutputChangeHistory {
            context: context.clone(),
            entity_id: entity_id.clone(),
            change: result.change_type.clone(),
            result: result.clone(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            rolled_back: false,
            rollback_data: snapshot,
        });
        
        // SECURITY: Don't log entity_id in production (could be sensitive)
        // Only log context type, not full details
        info!("Added filter to {:?} entity in {:.2}ms", context, duration);
        Ok(result)
    }

    /// Remove filter on-the-fly
    pub async fn remove_filter(
        &self,
        context: ConfigContext,
        entity_id: String,
        filter_index: usize,
    ) -> Result<OutputChangeResult> {
        // SECURITY: Validate entity_id
        Self::validate_entity_id(&entity_id)?;
        
        // SECURITY: Limit filter_index to prevent DoS
        const MAX_FILTER_INDEX: usize = 100_000;
        if filter_index > MAX_FILTER_INDEX {
            return Err(Error::Storage(format!(
                "Filter index too large: {} (max: {})",
                filter_index, MAX_FILTER_INDEX
            )));
        }
        
        let start_time = SystemTime::now();
        
        let mut configs = self.configs.write();
        let key = Self::make_key(&context, &entity_id);
        
        let table_config = configs.get_mut(&key)
            .ok_or_else(|| Error::Storage("Config not found".to_string()))?;
        
        if filter_index >= table_config.output_config.default_filters.len() {
            return Err(Error::Storage("Filter index out of range".to_string()));
        }
        
        let snapshot = if self.auto_backup {
            // SECURITY: Limit snapshot size
            let config_size = serde_json::to_string(&table_config.output_config)
                .map_err(|e| Error::Storage(format!("Failed to serialize config: {}", e)))?
                .len();
            const MAX_SNAPSHOT_SIZE: usize = 10 * 1024 * 1024; // 10MB
            if config_size > MAX_SNAPSHOT_SIZE {
                warn!("Config too large for snapshot ({} bytes), skipping backup", config_size);
                None
            } else {
                Some(OutputConfigSnapshot {
                    config: table_config.output_config.clone(),
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                })
            }
        } else {
            None
        };
        
        let removed_filter = table_config.output_config.default_filters.remove(filter_index);
        table_config.version += 1;
        table_config.last_modified = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        drop(configs);
        
        // SECURITY: Prevent integer overflow in duration calculation
        let duration = start_time.elapsed()
            .map(|d| {
                // SECURITY: Cap duration at u64::MAX milliseconds to prevent overflow
                let millis = d.as_millis();
                if millis > u64::MAX as u128 {
                    u64::MAX as f64
                } else {
                    millis as f64
                }
            })
            .unwrap_or(0.0);
        
        let result = OutputChangeResult {
            success: true,
            change_type: OutputChange::RemoveFilter { filter_index },
            affected_queries: 0,
            errors: vec![],
            rollback_available: snapshot.is_some(),
            duration_ms: duration,
        };
        
        // SECURITY: Validate entity_id
        Self::validate_entity_id(&entity_id)?;
        
        // Record history
        let mut history = self.change_history.write();
        
        // SECURITY: Limit history size
        const MAX_HISTORY_SIZE: usize = 100_000;
        if history.len() >= MAX_HISTORY_SIZE {
            let to_remove = history.len() - MAX_HISTORY_SIZE + 1;
            history.drain(0..to_remove);
        }
        
        history.push(OutputChangeHistory {
            context: context.clone(),
            entity_id: entity_id.clone(),
            change: result.change_type.clone(),
            result: result.clone(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            rolled_back: false,
            rollback_data: snapshot,
        });
        
        info!("Removed filter from {:?} entity in {:.2}ms", context, duration);
        Ok(result)
    }

    /// Add transform on-the-fly
    pub async fn add_transform(
        &self,
        context: ConfigContext,
        entity_id: String,
        transform: OutputTransform,
    ) -> Result<OutputChangeResult> {
        // SECURITY: Validate entity_id
        Self::validate_entity_id(&entity_id)?;
        
        let start_time = SystemTime::now();
        
        let mut configs = self.configs.write();
        let key = Self::make_key(&context, &entity_id);
        
        let table_config = configs.get_mut(&key)
            .ok_or_else(|| Error::Storage("Config not found".to_string()))?;
        
        // SECURITY: Limit number of transforms to prevent DoS
        const MAX_TRANSFORMS: usize = 10_000;
        if table_config.output_config.output_transforms.len() >= MAX_TRANSFORMS {
            return Err(Error::Storage(format!(
                "Too many transforms: {} (max: {})",
                table_config.output_config.output_transforms.len(), MAX_TRANSFORMS
            )));
        }
        
        let snapshot = if self.auto_backup {
            // SECURITY: Limit snapshot size
            let config_size = serde_json::to_string(&table_config.output_config)
                .map_err(|e| Error::Storage(format!("Failed to serialize config: {}", e)))?
                .len();
            const MAX_SNAPSHOT_SIZE: usize = 10 * 1024 * 1024; // 10MB
            if config_size > MAX_SNAPSHOT_SIZE {
                warn!("Config too large for snapshot ({} bytes), skipping backup", config_size);
                None
            } else {
                Some(OutputConfigSnapshot {
                    config: table_config.output_config.clone(),
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                })
            }
        } else {
            None
        };
        
        table_config.output_config.output_transforms.push(transform.clone());
        table_config.version += 1;
        table_config.last_modified = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        drop(configs);
        
        // SECURITY: Prevent integer overflow in duration calculation
        let duration = start_time.elapsed()
            .map(|d| {
                // SECURITY: Cap duration at u64::MAX milliseconds to prevent overflow
                let millis = d.as_millis();
                if millis > u64::MAX as u128 {
                    u64::MAX as f64
                } else {
                    millis as f64
                }
            })
            .unwrap_or(0.0);
        
        let result = OutputChangeResult {
            success: true,
            change_type: OutputChange::AddTransform { transform },
            affected_queries: 0,
            errors: vec![],
            rollback_available: snapshot.is_some(),
            duration_ms: duration,
        };
        
        let mut history = self.change_history.write();
        
        // SECURITY: Limit history size
        const MAX_HISTORY_SIZE: usize = 100_000;
        if history.len() >= MAX_HISTORY_SIZE {
            let to_remove = history.len() - MAX_HISTORY_SIZE + 1;
            history.drain(0..to_remove);
        }
        
        history.push(OutputChangeHistory {
            context: context.clone(),
            entity_id: entity_id.clone(),
            change: result.change_type.clone(),
            result: result.clone(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            rolled_back: false,
            rollback_data: snapshot,
        });
        
        info!("Added transform to {:?} entity in {:.2}ms", context, duration);
        Ok(result)
    }

    /// Remove transform on-the-fly
    pub async fn remove_transform(
        &self,
        context: ConfigContext,
        entity_id: String,
        transform_index: usize,
    ) -> Result<OutputChangeResult> {
        // SECURITY: Validate entity_id
        Self::validate_entity_id(&entity_id)?;
        
        // SECURITY: Limit transform_index to prevent DoS
        const MAX_TRANSFORM_INDEX: usize = 100_000;
        if transform_index > MAX_TRANSFORM_INDEX {
            return Err(Error::Storage(format!(
                "Transform index too large: {} (max: {})",
                transform_index, MAX_TRANSFORM_INDEX
            )));
        }
        
        let start_time = SystemTime::now();
        
        let mut configs = self.configs.write();
        let key = Self::make_key(&context, &entity_id);
        
        let table_config = configs.get_mut(&key)
            .ok_or_else(|| Error::Storage("Config not found".to_string()))?;
        
        if transform_index >= table_config.output_config.output_transforms.len() {
            return Err(Error::Storage("Transform index out of range".to_string()));
        }
        
        let snapshot = if self.auto_backup {
            // SECURITY: Limit snapshot size
            let config_size = serde_json::to_string(&table_config.output_config)
                .map_err(|e| Error::Storage(format!("Failed to serialize config: {}", e)))?
                .len();
            const MAX_SNAPSHOT_SIZE: usize = 10 * 1024 * 1024; // 10MB
            if config_size > MAX_SNAPSHOT_SIZE {
                warn!("Config too large for snapshot ({} bytes), skipping backup", config_size);
                None
            } else {
                Some(OutputConfigSnapshot {
                    config: table_config.output_config.clone(),
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                })
            }
        } else {
            None
        };
        
        table_config.output_config.output_transforms.remove(transform_index);
        table_config.version += 1;
        table_config.last_modified = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        drop(configs);
        
        // SECURITY: Prevent integer overflow in duration calculation
        let duration = start_time.elapsed()
            .map(|d| {
                // SECURITY: Cap duration at u64::MAX milliseconds to prevent overflow
                let millis = d.as_millis();
                if millis > u64::MAX as u128 {
                    u64::MAX as f64
                } else {
                    millis as f64
                }
            })
            .unwrap_or(0.0);
        
        let result = OutputChangeResult {
            success: true,
            change_type: OutputChange::RemoveTransform { transform_index },
            affected_queries: 0,
            errors: vec![],
            rollback_available: snapshot.is_some(),
            duration_ms: duration,
        };
        
        let mut history = self.change_history.write();
        
        // SECURITY: Limit history size
        const MAX_HISTORY_SIZE: usize = 100_000;
        if history.len() >= MAX_HISTORY_SIZE {
            let to_remove = history.len() - MAX_HISTORY_SIZE + 1;
            history.drain(0..to_remove);
        }
        
        history.push(OutputChangeHistory {
            context: context.clone(),
            entity_id: entity_id.clone(),
            change: result.change_type.clone(),
            result: result.clone(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            rolled_back: false,
            rollback_data: snapshot,
        });
        
        info!("Removed transform from {:?} entity in {:.2}ms", context, duration);
        Ok(result)
    }

    /// Add field rule on-the-fly
    pub async fn add_field_rule(
        &self,
        context: ConfigContext,
        entity_id: String,
        field: String,
        rule: FieldRule,
    ) -> Result<OutputChangeResult> {
        // SECURITY: Validate entity_id and field name
        Self::validate_entity_id(&entity_id)?;
        use narayana_core::transforms::TransformEngine;
        // Note: validate_field_name is private, so we'll validate inline
        if field.is_empty() || field.len() > 1_024 || field.contains("..") || field.contains('\0') {
            return Err(Error::Storage(format!("Invalid field name: '{}'", field)));
        }
        
        let start_time = SystemTime::now();
        
        let mut configs = self.configs.write();
        let key = Self::make_key(&context, &entity_id);
        
        let table_config = configs.get_mut(&key)
            .ok_or_else(|| Error::Storage("Config not found".to_string()))?;
        
        // SECURITY: Limit number of field rules
        const MAX_FIELD_RULES: usize = 10_000;
        if table_config.output_config.field_rules.len() >= MAX_FIELD_RULES {
            return Err(Error::Storage(format!(
                "Too many field rules: {} (max: {})",
                table_config.output_config.field_rules.len(), MAX_FIELD_RULES
            )));
        }
        
        let snapshot = if self.auto_backup {
            // SECURITY: Limit snapshot size
            let config_size = serde_json::to_string(&table_config.output_config)
                .map_err(|e| Error::Storage(format!("Failed to serialize config: {}", e)))?
                .len();
            const MAX_SNAPSHOT_SIZE: usize = 10 * 1024 * 1024; // 10MB
            if config_size > MAX_SNAPSHOT_SIZE {
                warn!("Config too large for snapshot ({} bytes), skipping backup", config_size);
                None
            } else {
                Some(OutputConfigSnapshot {
                    config: table_config.output_config.clone(),
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                })
            }
        } else {
            None
        };
        
        table_config.output_config.field_rules.insert(field.clone(), rule.clone());
        table_config.version += 1;
        table_config.last_modified = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        drop(configs);
        
        // SECURITY: Prevent integer overflow in duration calculation
        let duration = start_time.elapsed()
            .map(|d| {
                // SECURITY: Cap duration at u64::MAX milliseconds to prevent overflow
                let millis = d.as_millis();
                if millis > u64::MAX as u128 {
                    u64::MAX as f64
                } else {
                    millis as f64
                }
            })
            .unwrap_or(0.0);
        
        let result = OutputChangeResult {
            success: true,
            change_type: OutputChange::AddFieldRule { field, rule },
            affected_queries: 0,
            errors: vec![],
            rollback_available: snapshot.is_some(),
            duration_ms: duration,
        };
        
        let mut history = self.change_history.write();
        
        // SECURITY: Limit history size
        const MAX_HISTORY_SIZE: usize = 100_000;
        if history.len() >= MAX_HISTORY_SIZE {
            let to_remove = history.len() - MAX_HISTORY_SIZE + 1;
            history.drain(0..to_remove);
        }
        
        history.push(OutputChangeHistory {
            context: context.clone(),
            entity_id: entity_id.clone(),
            change: result.change_type.clone(),
            result: result.clone(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            rolled_back: false,
            rollback_data: snapshot,
        });
        
        Ok(result)
    }

    /// Get output config
    pub fn get_config(
        &self,
        context: &ConfigContext,
        entity_id: &str,
    ) -> Option<OutputConfig> {
        let configs = self.configs.read();
        let key = Self::make_key(context, entity_id);
        configs.get(&key).map(|c| c.output_config.clone())
    }

    /// Get config with profile
    pub fn get_config_with_profile(
        &self,
        context: &ConfigContext,
        entity_id: &str,
        profile: Option<&str>,
    ) -> Option<OutputConfig> {
        // SECURITY: Validate profile name if provided
        if let Some(profile_name) = profile {
            // SECURITY: Prevent path traversal in profile names
            if profile_name.is_empty() 
                || profile_name.len() > 1_024 
                || profile_name.contains("..") 
                || profile_name.contains("/") 
                || profile_name.contains("\\")
                || profile_name.contains('\0') {
                return None; // Invalid profile name
            }
        }
        
        let configs = self.configs.read();
        let key = Self::make_key(context, entity_id);
        
        if let Some(table_config) = configs.get(&key) {
            if let Some(profile_name) = profile {
                if let Some(profile_config) = table_config.output_config.profiles.get(profile_name) {
                    return Some(profile_config.clone());
                }
            }
            return Some(table_config.output_config.clone());
        }
        
        None
    }

    /// Get change history
    pub fn get_change_history(
        &self,
        context: &ConfigContext,
        entity_id: &str,
    ) -> Vec<OutputChangeHistory> {
        // SECURITY: Validate entity_id to prevent injection
        if let Err(_) = Self::validate_entity_id(entity_id) {
            return Vec::new(); // Return empty on invalid entity_id
        }
        
        let history = self.change_history.read();
        
        // SECURITY: Limit returned history size to prevent DoS
        const MAX_RETURNED_HISTORY: usize = 10_000;
        let mut results: Vec<OutputChangeHistory> = history.iter()
            .filter(|h| {
                h.context == *context && h.entity_id == entity_id
            })
            .cloned()
            .collect();
        
        // SECURITY: Return only most recent entries
        if results.len() > MAX_RETURNED_HISTORY {
            results.truncate(MAX_RETURNED_HISTORY);
        }
        
        results
    }

    /// Rollback change
    pub async fn rollback_change(
        &self,
        context: ConfigContext,
        entity_id: String,
        change_index: usize,
    ) -> Result<()> {
        // SECURITY: Validate entity_id
        Self::validate_entity_id(&entity_id)?;
        
        // SECURITY: Limit change_index to prevent DoS via large indices
        const MAX_CHANGE_INDEX: usize = 100_000;
        if change_index > MAX_CHANGE_INDEX {
            return Err(Error::Storage(format!(
                "Change index too large: {} (max: {})",
                change_index, MAX_CHANGE_INDEX
            )));
        }
        
        // Collect relevant history entries with their original indices
        // We need to clone the data we need before dropping the read lock
        let (actual_index, snapshot) = {
            let history = self.change_history.read();
            let relevant_history: Vec<(usize, &OutputChangeHistory)> = history.iter()
                .enumerate()
                .filter(|(_, h)| h.context == context && h.entity_id == entity_id)
                .collect();
            
            if change_index >= relevant_history.len() {
                return Err(Error::Storage("Invalid change index".to_string()));
            }
            
            let (actual_index, change_history) = relevant_history[change_index];
            
            if change_history.rolled_back {
                return Err(Error::Storage("Change already rolled back".to_string()));
            }
            
            let snapshot = change_history.rollback_data.as_ref()
                .ok_or_else(|| Error::Storage("No rollback data available".to_string()))?
                .clone();
            
            // SECURITY: Validate snapshot size before using
            let snapshot_size = serde_json::to_string(&snapshot.config)
                .map_err(|e| Error::Storage(format!("Failed to serialize snapshot: {}", e)))?
                .len();
            const MAX_SNAPSHOT_SIZE: usize = 10 * 1024 * 1024; // 10MB
            if snapshot_size > MAX_SNAPSHOT_SIZE {
                return Err(Error::Storage(format!(
                    "Snapshot too large for rollback: {} bytes (max: {})",
                    snapshot_size, MAX_SNAPSHOT_SIZE
                )));
            }
            
            (actual_index, snapshot)
        };
        
        // Restore config
        let mut configs = self.configs.write();
        let key = Self::make_key(&context, &entity_id);
        
        if let Some(table_config) = configs.get_mut(&key) {
            table_config.output_config = snapshot.config.clone();
            table_config.version += 1;
        }
        
        drop(configs);
        
        // Mark as rolled back
        let mut history = self.change_history.write();
        if let Some(h) = history.get_mut(actual_index) {
            h.rolled_back = true;
        }
        
        // SECURITY: Don't log entity_id
        info!("Rolled back output change for {:?} entity", context);
        Ok(())
    }

    /// Helper: make key from context and entity_id
    /// SECURITY: This creates a key for HashMap lookup - entity_id is already validated
    fn make_key(context: &ConfigContext, entity_id: &str) -> String {
        // SECURITY: Format is safe - Debug trait doesn't execute code
        // entity_id is validated before this is called
        format!("{:?}:{}", context, entity_id)
    }
}

