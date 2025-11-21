//! Safety validation example

use narayana_cns::{
    CentralNervousSystem, CnsConfig, ComponentInfo, ComponentId, ComponentType,
    Capability, TransportConfig, TransportType, SafetyLevel, SafetyLimits,
    SafetyRule, SafetyRuleType,
};
use narayana_wld::event_transformer::WorldAction;
use std::collections::HashMap;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    // Create CNS configuration
    let config = CnsConfig {
        default_safety_level: SafetyLevel::Critical, // Use critical safety level
        heartbeat_timeout_secs: 5,
        enable_capability_routing: true,
        enable_load_balancing: true,
        max_action_queue_size: 1000,
        enable_emergency_stop: true,
        action_timeout_ms: 5000,
    };
    
    // Create CNS
    let cns = CentralNervousSystem::new(config)?;
    
    // Create hand actuator with strict safety limits
    let mut hand_actuator = ComponentInfo::new(
        ComponentId::generate(),
        "hand_actuator_1".to_string(),
        ComponentType::Actuator,
        vec![Capability::Simple("move".to_string())],
        TransportConfig {
            transport_type: TransportType::Http,
            config: HashMap::new(),
        },
    );
    
    // Set strict safety limits
    hand_actuator.safety_limits = Some(SafetyLimits {
        max_velocity: Some(5.0),  // Max 5 units/sec
        max_force: Some(30.0),    // Max 30 units
        max_range: Some(50.0),    // Max 50 units
        allowed_commands: vec!["move".to_string()],
        forbidden_commands: vec!["emergency_stop".to_string()],
        emergency_stop_enabled: true,
        safety_level: SafetyLevel::Critical,
    });
    
    // Register component
    cns.register_component(hand_actuator.clone())?;
    
    // Add safety rules
    let mut validator = cns.safety_validator().write();
    validator.add_rule(SafetyRule {
        name: "velocity_limit".to_string(),
        rule_type: SafetyRuleType::VelocityLimit,
        config: HashMap::new(),
        enabled: true,
    });
    validator.add_rule(SafetyRule {
        name: "force_limit".to_string(),
        rule_type: SafetyRuleType::ForceLimit,
        config: HashMap::new(),
        enabled: true,
    });
    drop(validator);
    
    // Test safe action
    println!("Testing safe action...");
    let safe_action = WorldAction::ActuatorCommand {
        target: "hand_actuator_1".to_string(),
        command: json!({
            "command": "move",
            "velocity": 3.0,  // Within limit
            "force": 20.0,    // Within limit
        }),
    };
    
    let validator = cns.safety_validator().read();
    let validation = validator.validate_action(&safe_action, &hand_actuator);
    drop(validator);
    
    println!("Safe action validation:");
    println!("  Is safe: {}", validation.is_safe);
    println!("  Safety score: {:.2}", validation.safety_score);
    println!("  Reasons: {:?}", validation.reasons);
    
    // Test unsafe action (exceeds velocity limit)
    println!("\nTesting unsafe action (exceeds velocity limit)...");
    let unsafe_action = WorldAction::ActuatorCommand {
        target: "hand_actuator_1".to_string(),
        command: json!({
            "command": "move",
            "velocity": 10.0,  // Exceeds max_velocity of 5.0
            "force": 20.0,
        }),
    };
    
    let validator = cns.safety_validator().read();
    let validation = validator.validate_action(&unsafe_action, &hand_actuator);
    drop(validator);
    
    println!("Unsafe action validation:");
    println!("  Is safe: {}", validation.is_safe);
    println!("  Safety score: {:.2}", validation.safety_score);
    println!("  Reasons: {:?}", validation.reasons);
    println!("  Emergency stop: {}", validation.emergency_stop);
    
    // Test emergency stop
    println!("\nTesting emergency stop...");
    let mut validator = cns.safety_validator().write();
    validator.trigger_emergency_stop();
    
    let validation = validator.validate_action(&safe_action, &hand_actuator);
    println!("Action validation with emergency stop active:");
    println!("  Is safe: {}", validation.is_safe);
    println!("  Emergency stop: {}", validation.emergency_stop);
    
    // Clear emergency stop
    validator.clear_emergency_stop();
    println!("\nEmergency stop cleared");
    
    Ok(())
}

