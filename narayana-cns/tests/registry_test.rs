//! Tests for component registry

use narayana_cns::{
    ComponentRegistry, ComponentInfo, ComponentId, ComponentType,
    Capability, TransportConfig, TransportType,
};
use std::collections::HashMap;
use serde_json::json;

#[test]
fn test_register_component() {
    let registry = ComponentRegistry::new(5);
    
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
    
    assert!(registry.register(component.clone()).is_ok());
    assert_eq!(registry.count(), 1);
}

#[test]
fn test_unregister_component() {
    let registry = ComponentRegistry::new(5);
    
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
    
    assert!(registry.unregister(&component_id).is_ok());
    assert_eq!(registry.count(), 0);
}

#[test]
fn test_find_by_capability() {
    let registry = ComponentRegistry::new(5);
    
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
    
    let move_capability = Capability::Simple("move".to_string());
    let components = registry.find_by_capability(&move_capability);
    
    assert_eq!(components.len(), 1);
    assert_eq!(components[0].name, "component1");
}

#[test]
fn test_find_by_type() {
    let registry = ComponentRegistry::new(5);
    
    let actuator = ComponentInfo::new(
        ComponentId::generate(),
        "actuator".to_string(),
        ComponentType::Actuator,
        vec![Capability::Simple("move".to_string())],
        TransportConfig {
            transport_type: TransportType::Http,
            config: HashMap::new(),
        },
    );
    
    let sensor = ComponentInfo::new(
        ComponentId::generate(),
        "sensor".to_string(),
        ComponentType::Sensor,
        vec![Capability::Simple("sense".to_string())],
        TransportConfig {
            transport_type: TransportType::Http,
            config: HashMap::new(),
        },
    );
    
    registry.register(actuator).unwrap();
    registry.register(sensor).unwrap();
    
    let actuators = registry.find_by_type(ComponentType::Actuator);
    assert_eq!(actuators.len(), 1);
    
    let sensors = registry.find_by_type(ComponentType::Sensor);
    assert_eq!(sensors.len(), 1);
}

#[test]
fn test_duplicate_registration() {
    let registry = ComponentRegistry::new(5);
    
    let component_id = ComponentId::generate();
    let component1 = ComponentInfo::new(
        component_id.clone(),
        "test".to_string(),
        ComponentType::Actuator,
        vec![Capability::Simple("move".to_string())],
        TransportConfig {
            transport_type: TransportType::Http,
            config: HashMap::new(),
        },
    );
    
    let component2 = ComponentInfo::new(
        component_id,
        "test2".to_string(),
        ComponentType::Actuator,
        vec![Capability::Simple("move".to_string())],
        TransportConfig {
            transport_type: TransportType::Http,
            config: HashMap::new(),
        },
    );
    
    assert!(registry.register(component1).is_ok());
    assert!(registry.register(component2).is_err());
}

