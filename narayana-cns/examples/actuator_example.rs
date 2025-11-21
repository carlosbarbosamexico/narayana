//! Actuator control example with capability-based routing

use narayana_cns::{
    CentralNervousSystem, CnsConfig, ComponentInfo, ComponentId, ComponentType,
    Capability, TransportConfig, TransportType, SafetyLevel, SafetyLimits,
};
use narayana_wld::event_transformer::WorldAction;
use std::collections::HashMap;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    // Create CNS configuration
    let config = CnsConfig {
        default_safety_level: SafetyLevel::Production,
        heartbeat_timeout_secs: 5,
        enable_capability_routing: true,
        enable_load_balancing: true,
        max_action_queue_size: 1000,
        enable_emergency_stop: true,
        action_timeout_ms: 5000,
    };
    
    // Create CNS
    let cns = CentralNervousSystem::new(config)?;
    
    // Create hand actuator with safety limits
    let mut hand_actuator = ComponentInfo::new(
        ComponentId::generate(),
        "hand_actuator_1".to_string(),
        ComponentType::Actuator,
        vec![
            Capability::Simple("move".to_string()),
            Capability::Simple("grasp".to_string()),
        ],
        TransportConfig {
            transport_type: TransportType::Http,
            config: {
                let mut map = HashMap::new();
                map.insert("url".to_string(), json!("http://localhost:8080/hand"));
                map
            },
        },
    );
    
    // Set safety limits
    hand_actuator.safety_limits = Some(SafetyLimits {
        max_velocity: Some(10.0), // 10 units/sec
        max_force: Some(50.0),    // 50 units
        max_range: Some(100.0),   // 100 units
        allowed_commands: vec!["move".to_string(), "grasp".to_string(), "release".to_string()],
        forbidden_commands: vec!["emergency_stop".to_string()],
        emergency_stop_enabled: true,
        safety_level: SafetyLevel::Production,
    });
    
    // Register component
    println!("Registering hand actuator with safety limits...");
    cns.register_component(hand_actuator.clone())?;
    
    // Create an action to move the hand
    let move_action = WorldAction::ActuatorCommand {
        target: "hand_actuator_1".to_string(),
        command: json!({
            "command": "move",
            "capability": "move",
            "position": 50.0,
            "velocity": 5.0,
        }),
    };
    
    println!("\nRouting action to hand actuator...");
    let router = cns.router();
    let component_ids = router.route_action(&move_action, None)
        .map_err(|e| format!("Routing error: {}", e))?;
    
    println!("Action routed to {} component(s)", component_ids.len());
    for component_id in component_ids {
        println!("  - Component ID: {}", component_id.as_str());
    }
    
    // Test capability-based routing
    let grasp_action = WorldAction::ActuatorCommand {
        target: "".to_string(), // Empty target - use capability routing
        command: json!({
            "command": "grasp",
            "capability": "grasp",
            "force": 20.0,
        }),
    };
    
    println!("\nRouting action by capability...");
    let component_ids = router.route_action(&grasp_action, None)
        .map_err(|e| format!("Routing error: {}", e))?;
    
    println!("Action routed to {} component(s) by capability", component_ids.len());
    
    Ok(())
}

