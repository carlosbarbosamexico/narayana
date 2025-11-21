//! Basic component registration example

use narayana_cns::{
    CentralNervousSystem, CnsConfig, ComponentInfo, ComponentId, ComponentType,
    Capability, TransportConfig, TransportType, SafetyLevel,
};
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
    
    // Create a hand actuator component
    let hand_actuator = ComponentInfo::new(
        ComponentId::generate(),
        "hand_actuator_1".to_string(),
        ComponentType::Actuator,
        vec![
            Capability::Simple("move".to_string()),
            Capability::Simple("grasp".to_string()),
            Capability::Simple("release".to_string()),
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
    
    // Register component
    println!("Registering hand actuator...");
    cns.register_component(hand_actuator.clone())?;
    println!("Hand actuator registered: {}", hand_actuator.id.as_str());
    
    // Find components by capability
    let move_capability = Capability::Simple("move".to_string());
    let components = cns.find_by_capability(&move_capability);
    println!("Found {} components with 'move' capability", components.len());
    
    // Get component by ID
    if let Some(component) = cns.get_component(&hand_actuator.id) {
        println!("Component found: {} ({})", component.name, component.id.as_str());
        println!("  Type: {:?}", component.component_type);
        println!("  Capabilities: {:?}", component.capabilities);
    }
    
    // Unregister component
    println!("\nUnregistering hand actuator...");
    cns.unregister_component(&hand_actuator.id)?;
    println!("Hand actuator unregistered");
    
    Ok(())
}

