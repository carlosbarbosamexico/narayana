//! Tests for safety validation

use narayana_cns::{
    SafetyValidator, SafetyLevel, SafetyLimits, ComponentInfo, ComponentId,
    ComponentType, Capability, TransportConfig, TransportType,
    SafetyRule, SafetyRuleType,
};
use narayana_wld::event_transformer::WorldAction;
use std::collections::HashMap;
use serde_json::json;

#[test]
fn test_safe_action_validation() {
    let validator = SafetyValidator::new(SafetyLevel::Production);
    
    let component = create_test_component();
    
    let action = WorldAction::ActuatorCommand {
        target: "test_component".to_string(),
        command: json!({
            "command": "move",
            "velocity": 3.0,
        }),
    };
    
    let validation = validator.validate_action(&action, &component);
    assert!(validation.is_safe);
    assert!(validation.safety_score > 0.5);
}

#[test]
fn test_unsafe_action_validation() {
    let validator = SafetyValidator::new(SafetyLevel::Production);
    
    let mut component = create_test_component();
    component.safety_limits = Some(SafetyLimits {
        max_velocity: Some(5.0),
        max_force: Some(30.0),
        max_range: Some(50.0),
        allowed_commands: vec!["move".to_string()],
        forbidden_commands: vec![],
        emergency_stop_enabled: true,
        safety_level: SafetyLevel::Production,
    });
    
    let action = WorldAction::ActuatorCommand {
        target: "test_component".to_string(),
        command: json!({
            "command": "move",
            "velocity": 10.0, // Exceeds max_velocity
        }),
    };
    
    let validation = validator.validate_action(&action, &component);
    assert!(!validation.is_safe);
    assert!(validation.safety_score < 0.5);
}

#[test]
fn test_emergency_stop() {
    let mut validator = SafetyValidator::new(SafetyLevel::Production);
    
    let component = create_test_component();
    
    let action = WorldAction::ActuatorCommand {
        target: "test_component".to_string(),
        command: json!({
            "command": "move",
            "velocity": 3.0,
        }),
    };
    
    // Trigger emergency stop
    validator.trigger_emergency_stop();
    assert!(validator.is_emergency_stop_active());
    
    let validation = validator.validate_action(&action, &component);
    assert!(!validation.is_safe);
    assert!(validation.emergency_stop);
    
    // Clear emergency stop
    validator.clear_emergency_stop();
    assert!(!validator.is_emergency_stop_active());
}

#[test]
fn test_safety_rules() {
    let mut validator = SafetyValidator::new(SafetyLevel::Production);
    
    validator.add_rule(SafetyRule {
        name: "velocity_limit".to_string(),
        rule_type: SafetyRuleType::VelocityLimit,
        config: HashMap::new(),
        enabled: true,
    });
    
    let mut component = create_test_component();
    component.safety_limits = Some(SafetyLimits {
        max_velocity: Some(5.0),
        max_force: None,
        max_range: None,
        allowed_commands: vec![],
        forbidden_commands: vec![],
        emergency_stop_enabled: false,
        safety_level: SafetyLevel::Production,
    });
    
    let action = WorldAction::ActuatorCommand {
        target: "test_component".to_string(),
        command: json!({
            "command": "move",
            "velocity": 10.0, // Exceeds limit
        }),
    };
    
    let validation = validator.validate_action(&action, &component);
    assert!(!validation.is_safe);
}

fn create_test_component() -> ComponentInfo {
    ComponentInfo::new(
        ComponentId::generate(),
        "test_component".to_string(),
        ComponentType::Actuator,
        vec![Capability::Simple("move".to_string())],
        TransportConfig {
            transport_type: TransportType::Http,
            config: HashMap::new(),
        },
    )
}

