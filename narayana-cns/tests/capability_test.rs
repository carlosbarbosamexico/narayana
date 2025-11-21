//! Tests for capability model

use narayana_cns::{
    Capability, CapabilityMatcher, StructuredCapability, Parameter, ParameterType,
    Constraint, ConstraintType,
};
use serde_json::json;

#[test]
fn test_simple_capability_matching() {
    let cap1 = Capability::Simple("move".to_string());
    let cap2 = Capability::Simple("move".to_string());
    let cap3 = Capability::Simple("grasp".to_string());
    
    assert!(cap1.matches(&cap2));
    assert!(!cap1.matches(&cap3));
}

#[test]
fn test_structured_capability_matching() {
    let cap1 = Capability::Structured(StructuredCapability {
        name: "move".to_string(),
        version: "1.0.0".to_string(),
        parameters: Vec::new(),
        constraints: Vec::new(),
        metadata: std::collections::HashMap::new(),
    });
    
    let cap2 = Capability::Structured(StructuredCapability {
        name: "move".to_string(),
        version: "1.0.0".to_string(),
        parameters: Vec::new(),
        constraints: Vec::new(),
        metadata: std::collections::HashMap::new(),
    });
    
    let cap3 = Capability::Structured(StructuredCapability {
        name: "grasp".to_string(),
        version: "1.0.0".to_string(),
        parameters: Vec::new(),
        constraints: Vec::new(),
        metadata: std::collections::HashMap::new(),
    });
    
    assert!(cap1.matches(&cap2));
    assert!(!cap1.matches(&cap3));
}

#[test]
fn test_capability_compatibility() {
    let cap1 = Capability::Structured(StructuredCapability {
        name: "move".to_string(),
        version: "1.0.0".to_string(),
        parameters: Vec::new(),
        constraints: Vec::new(),
        metadata: std::collections::HashMap::new(),
    });
    
    let cap2 = Capability::Structured(StructuredCapability {
        name: "move".to_string(),
        version: "1.1.0".to_string(), // Same major version
        parameters: Vec::new(),
        constraints: Vec::new(),
        metadata: std::collections::HashMap::new(),
    });
    
    let cap3 = Capability::Structured(StructuredCapability {
        name: "move".to_string(),
        version: "2.0.0".to_string(), // Different major version
        parameters: Vec::new(),
        constraints: Vec::new(),
        metadata: std::collections::HashMap::new(),
    });
    
    assert!(cap1.is_compatible(&cap2));
    assert!(!cap1.is_compatible(&cap3));
}

#[test]
fn test_command_validation() {
    let capability = Capability::Structured(StructuredCapability {
        name: "move".to_string(),
        version: "1.0.0".to_string(),
        parameters: vec![
            Parameter {
                name: "position".to_string(),
                param_type: ParameterType::Float,
                required: true,
                default: None,
                description: Some("Target position".to_string()),
            },
            Parameter {
                name: "velocity".to_string(),
                param_type: ParameterType::Float,
                required: false,
                default: Some(json!(1.0)),
                description: Some("Movement velocity".to_string()),
            },
        ],
        constraints: vec![
            Constraint {
                constraint_type: ConstraintType::Min,
                target: "position".to_string(),
                value: json!(0.0),
            },
            Constraint {
                constraint_type: ConstraintType::Max,
                target: "position".to_string(),
                value: json!(100.0),
            },
        ],
        metadata: std::collections::HashMap::new(),
    });
    
    // Valid command
    let valid_command = json!({
        "position": 50.0,
        "velocity": 5.0,
    });
    
    assert!(CapabilityMatcher::validate_command(&capability, &valid_command).is_ok());
    
    // Invalid command - missing required parameter
    let invalid_command = json!({
        "velocity": 5.0,
    });
    
    assert!(CapabilityMatcher::validate_command(&capability, &invalid_command).is_err());
    
    // Invalid command - violates constraint
    let invalid_command2 = json!({
        "position": 150.0, // Exceeds max
        "velocity": 5.0,
    });
    
    assert!(CapabilityMatcher::validate_command(&capability, &invalid_command2).is_err());
}

