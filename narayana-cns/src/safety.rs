//! Safety interlock and validation system

use crate::component::{ComponentInfo, ComponentId};
use crate::capability::Capability;
#[cfg(feature = "wld-integration")]
use narayana_wld::event_transformer::WorldAction;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Safety validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyValidation {
    /// Whether action is safe to execute
    pub is_safe: bool,
    /// Safety score (0.0 = dangerous, 1.0 = safe)
    pub safety_score: f64,
    /// Reasons for validation decision
    pub reasons: Vec<String>,
    /// Whether emergency stop should be triggered
    pub emergency_stop: bool,
}

/// Safety limits for a component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyLimits {
    /// Maximum velocity (units depend on component)
    pub max_velocity: Option<f64>,
    /// Maximum force/torque
    pub max_force: Option<f64>,
    /// Maximum range/movement distance
    pub max_range: Option<f64>,
    /// Allowed commands (whitelist)
    pub allowed_commands: Vec<String>,
    /// Forbidden commands (blacklist)
    pub forbidden_commands: Vec<String>,
    /// Emergency stop enabled
    pub emergency_stop_enabled: bool,
    /// Safety level
    pub safety_level: SafetyLevel,
}

/// Safety level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SafetyLevel {
    /// Development mode - minimal safety checks
    Development,
    /// Production mode - standard safety checks
    Production,
    /// Critical mode - maximum safety checks
    Critical,
}

impl Default for SafetyLevel {
    fn default() -> Self {
        SafetyLevel::Production
    }
}

/// Safety rule for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyRule {
    /// Rule name
    pub name: String,
    /// Rule type
    pub rule_type: SafetyRuleType,
    /// Rule configuration
    pub config: HashMap<String, JsonValue>,
    /// Whether rule is enabled
    pub enabled: bool,
}

/// Safety rule type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SafetyRuleType {
    /// Velocity limit check
    VelocityLimit,
    /// Force limit check
    ForceLimit,
    /// Range limit check
    RangeLimit,
    /// Command whitelist check
    CommandWhitelist,
    /// Command blacklist check
    CommandBlacklist,
    /// Emergency stop check
    EmergencyStop,
    /// Custom rule
    Custom(String),
}

/// Safety validator
pub struct SafetyValidator {
    /// Global safety rules
    rules: Vec<SafetyRule>,
    /// Component-specific safety limits
    component_limits: HashMap<ComponentId, SafetyLimits>,
    /// Emergency stop state
    emergency_stop_active: bool,
    /// Default safety level
    default_safety_level: SafetyLevel,
}

impl SafetyValidator {
    /// Create new safety validator
    pub fn new(default_safety_level: SafetyLevel) -> Self {
        Self {
            rules: Vec::new(),
            component_limits: HashMap::new(),
            emergency_stop_active: false,
            default_safety_level,
        }
    }
    
    /// Add safety rule
    pub fn add_rule(&mut self, rule: SafetyRule) {
        self.rules.push(rule);
    }
    
    /// Set component safety limits
    pub fn set_component_limits(&mut self, component_id: ComponentId, limits: SafetyLimits) {
        self.component_limits.insert(component_id, limits);
    }
    
    /// Trigger emergency stop
    pub fn trigger_emergency_stop(&mut self) {
        self.emergency_stop_active = true;
    }
    
    /// Clear emergency stop
    pub fn clear_emergency_stop(&mut self) {
        self.emergency_stop_active = false;
    }
    
    /// Check if emergency stop is active
    pub fn is_emergency_stop_active(&self) -> bool {
        self.emergency_stop_active
    }
    
    /// Validate action for a component
    #[cfg(feature = "wld-integration")]
    pub fn validate_action(
        &self,
        action: &WorldAction,
        component: &ComponentInfo,
    ) -> SafetyValidation {
        let mut reasons = Vec::new();
        let mut safety_score = 1.0;
        
        // Check emergency stop
        if self.emergency_stop_active {
            return SafetyValidation {
                is_safe: false,
                safety_score: 0.0,
                reasons: vec!["Emergency stop is active".to_string()],
                emergency_stop: true,
            };
        }
        
        // Get component safety limits
        let limits = component.safety_limits.as_ref()
            .or_else(|| self.component_limits.get(&component.id));
        
        // Extract command from action
        let (target, command) = match action {
            WorldAction::ActuatorCommand { target, command } => {
                (target.as_str(), command)
            }
            _ => {
                // Non-actuator commands are generally safe
                return SafetyValidation {
                    is_safe: true,
                    safety_score: 1.0,
                    reasons: vec!["Non-actuator command".to_string()],
                    emergency_stop: false,
                };
            }
        };
        
        // Validate target matches component
        if target != component.id.as_str() && target != component.name {
            return SafetyValidation {
                is_safe: false,
                safety_score: 0.0,
                reasons: vec![format!("Target '{}' does not match component", target)],
                emergency_stop: false,
            };
        }
        
        // Apply safety rules
        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }
            
            match rule.rule_type {
                SafetyRuleType::VelocityLimit => {
                    if let Some(limits) = limits {
                        if let Some(max_vel) = limits.max_velocity {
                            if let Some(vel) = Self::extract_velocity(command) {
                                if vel > max_vel {
                                    safety_score = 0.0;
                                    reasons.push(format!(
                                        "Velocity {} exceeds limit {}",
                                        vel, max_vel
                                    ));
                                }
                            }
                        }
                    }
                }
                SafetyRuleType::ForceLimit => {
                    if let Some(limits) = limits {
                        if let Some(max_force) = limits.max_force {
                            if let Some(force) = Self::extract_force(command) {
                                if force > max_force {
                                    safety_score = 0.0;
                                    reasons.push(format!(
                                        "Force {} exceeds limit {}",
                                        force, max_force
                                    ));
                                }
                            }
                        }
                    }
                }
                SafetyRuleType::RangeLimit => {
                    if let Some(limits) = limits {
                        if let Some(max_range) = limits.max_range {
                            if let Some(range) = Self::extract_range(command) {
                                if range > max_range {
                                    safety_score = 0.0;
                                    reasons.push(format!(
                                        "Range {} exceeds limit {}",
                                        range, max_range
                                    ));
                                }
                            }
                        }
                    }
                }
                SafetyRuleType::CommandWhitelist => {
                    if let Some(limits) = limits {
                        if !limits.allowed_commands.is_empty() {
                            if let Some(cmd_name) = Self::extract_command_name(command) {
                                if !limits.allowed_commands.contains(&cmd_name) {
                                    safety_score = 0.0;
                                    reasons.push(format!(
                                        "Command '{}' not in whitelist",
                                        cmd_name
                                    ));
                                }
                            }
                        }
                    }
                }
                SafetyRuleType::CommandBlacklist => {
                    if let Some(limits) = limits {
                        if let Some(cmd_name) = Self::extract_command_name(command) {
                            if limits.forbidden_commands.contains(&cmd_name) {
                                safety_score = 0.0;
                                reasons.push(format!(
                                    "Command '{}' is blacklisted",
                                    cmd_name
                                ));
                            }
                        }
                    }
                }
                SafetyRuleType::EmergencyStop => {
                    if let Some(limits) = limits {
                        if limits.emergency_stop_enabled {
                            // Check for emergency stop conditions
                            if let Some(cmd_name) = Self::extract_command_name(command) {
                                if cmd_name == "emergency_stop" || cmd_name == "stop" {
                                    return SafetyValidation {
                                        is_safe: false,
                                        safety_score: 0.0,
                                        reasons: vec!["Emergency stop command".to_string()],
                                        emergency_stop: true,
                                    };
                                }
                            }
                        }
                    }
                }
                SafetyRuleType::Custom(_) => {
                    // Custom rules not implemented here
                }
            }
        }
        
        // Check safety level
        let safety_level = limits
            .map(|l| l.safety_level)
            .unwrap_or(self.default_safety_level);
        
        match safety_level {
            SafetyLevel::Development => {
                // Minimal checks - allow most actions
                if safety_score < 0.3 {
                    safety_score = 0.5; // Reduce severity in dev mode
                }
            }
            SafetyLevel::Production => {
                // Standard checks - enforce limits
            }
            SafetyLevel::Critical => {
                // Maximum safety - any issue fails
                if safety_score < 1.0 {
                    safety_score = 0.0;
                }
            }
        }
        
        SafetyValidation {
            is_safe: safety_score > 0.5,
            safety_score,
            reasons: if reasons.is_empty() {
                vec!["Action validated".to_string()]
            } else {
                reasons
            },
            emergency_stop: safety_score == 0.0 && safety_level == SafetyLevel::Critical,
        }
    }
    
    /// Extract velocity from command JSON
    fn extract_velocity(command: &JsonValue) -> Option<f64> {
        command.get("velocity")
            .or_else(|| command.get("vel"))
            .or_else(|| command.get("speed"))
            .and_then(|v| v.as_f64())
    }
    
    /// Extract force from command JSON
    fn extract_force(command: &JsonValue) -> Option<f64> {
        command.get("force")
            .or_else(|| command.get("torque"))
            .and_then(|v| v.as_f64())
    }
    
    /// Extract range from command JSON
    fn extract_range(command: &JsonValue) -> Option<f64> {
        command.get("range")
            .or_else(|| command.get("distance"))
            .or_else(|| command.get("position"))
            .and_then(|v| v.as_f64())
    }
    
    /// Extract command name from command JSON
    fn extract_command_name(command: &JsonValue) -> Option<String> {
        command.get("command")
            .or_else(|| command.get("action"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
}

