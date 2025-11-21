//! Tests for action routing

use narayana_cns::{
    ComponentRegistry, ActionRouter, ComponentInfo, ComponentId, ComponentType,
    Capability, TransportConfig, TransportType,
};
use narayana_wld::event_transformer::WorldAction;
use std::collections::HashMap;
use std::sync::Arc;
use serde_json::json;

#[test]
fn test_route_to_specific_component() {
    let registry = Arc::new(ComponentRegistry::new(5));
    let router = ActionRouter::new(registry.clone(), false);
    
    let component = ComponentInfo::new(
        ComponentId::generate(),
        "test_component".to_string(),
        ComponentType::Actuator,
        vec![Capability::Simple("move".to_string())],
        TransportConfig {
            transport_type: TransportType::Http,
            config: HashMap::new(),
        },
    );
    
    let component_id = component.id.clone();
    registry.register(component).unwrap();
    
    let action = WorldAction::ActuatorCommand {
        target: "test_component".to_string(),
        command: json!({
            "command": "move",
        }),
    };
    
    let component_ids = router.route_action(&action, None).unwrap();
    assert_eq!(component_ids.len(), 1);
    assert_eq!(component_ids[0], component_id);
}

#[test]
fn test_route_by_capability() {
    let registry = Arc::new(ComponentRegistry::new(5));
    let router = ActionRouter::new(registry.clone(), false);
    
    let component1 = ComponentInfo::new(
        ComponentId::generate(),
        "component1".to_string(),
        ComponentType::Actuator,
        vec![Capability::Simple("move".to_string())],
        TransportConfig {
            transport_type: TransportType::Http,
            config: HashMap::new(),
        },
    );
    
    let component2 = ComponentInfo::new(
        ComponentId::generate(),
        "component2".to_string(),
        ComponentType::Actuator,
        vec![Capability::Simple("grasp".to_string())],
        TransportConfig {
            transport_type: TransportType::Http,
            config: HashMap::new(),
        },
    );
    
    registry.register(component1).unwrap();
    registry.register(component2).unwrap();
    
    let action = WorldAction::ActuatorCommand {
        target: "".to_string(),
        command: json!({
            "command": "move",
            "capability": "move",
        }),
    };
    
    let component_ids = router.route_action(&action, None).unwrap();
    assert_eq!(component_ids.len(), 1);
}

#[test]
fn test_route_to_unavailable_component() {
    let registry = Arc::new(ComponentRegistry::new(5));
    let router = ActionRouter::new(registry.clone(), false);
    
    let mut component = ComponentInfo::new(
        ComponentId::generate(),
        "test_component".to_string(),
        ComponentType::Actuator,
        vec![Capability::Simple("move".to_string())],
        TransportConfig {
            transport_type: TransportType::Http,
            config: HashMap::new(),
        },
    );
    
    use narayana_cns::ComponentState;
    component.state = ComponentState::Unavailable;
    
    registry.register(component).unwrap();
    
    let action = WorldAction::ActuatorCommand {
        target: "test_component".to_string(),
        command: json!({
            "command": "move",
        }),
    };
    
    assert!(router.route_action(&action, None).is_err());
}

