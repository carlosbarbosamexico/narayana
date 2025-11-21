//! Capability model for component capabilities

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Capability - supports both simple and structured forms
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Capability {
    /// Simple string-based capability (e.g., "move", "grasp")
    Simple(String),
    /// Structured capability with schema
    Structured(StructuredCapability),
}

impl std::hash::Hash for Capability {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Capability::Simple(s) => {
                state.write_u8(0);
                s.hash(state);
            }
            Capability::Structured(sc) => {
                state.write_u8(1);
                sc.name.hash(state);
                sc.version.hash(state);
                // Note: metadata is not hashed as HashMap doesn't implement Hash
            }
        }
    }
}

impl Capability {
    /// Check if this capability matches another
    pub fn matches(&self, other: &Capability) -> bool {
        match (self, other) {
            (Capability::Simple(a), Capability::Simple(b)) => a == b,
            (Capability::Structured(a), Capability::Structured(b)) => {
                a.name == b.name && a.version == b.version
            }
            (Capability::Simple(a), Capability::Structured(b)) => {
                a == &b.name
            }
            (Capability::Structured(a), Capability::Simple(b)) => {
                &a.name == b
            }
        }
    }
    
    /// Get capability name
    pub fn name(&self) -> &str {
        match self {
            Capability::Simple(name) => name,
            Capability::Structured(s) => &s.name,
        }
    }
    
    /// Check if capability is compatible (version check)
    pub fn is_compatible(&self, other: &Capability) -> bool {
        match (self, other) {
            (Capability::Simple(a), Capability::Simple(b)) => a == b,
            (Capability::Structured(a), Capability::Structured(b)) => {
                if a.name != b.name {
                    return false;
                }
                // Version compatibility: same major version
                let a_major = a.version.split('.').next().unwrap_or("0");
                let b_major = b.version.split('.').next().unwrap_or("0");
                a_major == b_major
            }
            _ => self.matches(other),
        }
    }
}

/// Structured capability with schema
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredCapability {
    /// Capability name
    pub name: String,
    /// Capability version (semver)
    pub version: String,
    /// Parameter definitions
    pub parameters: Vec<Parameter>,
    /// Constraints
    pub constraints: Vec<Constraint>,
    /// Metadata
    pub metadata: HashMap<String, JsonValue>,
}

/// Parameter definition
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Parameter {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub param_type: ParameterType,
    /// Whether parameter is required
    pub required: bool,
    /// Default value (if any)
    pub default: Option<JsonValue>,
    /// Description
    pub description: Option<String>,
}

/// Parameter type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ParameterType {
    String,
    Integer,
    Float,
    Boolean,
    Array,
    Object,
    Custom(String), // Custom type name
}

/// Constraint on capability or parameter
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Constraint {
    /// Constraint type
    pub constraint_type: ConstraintType,
    /// Target (parameter name or "capability")
    pub target: String,
    /// Constraint value
    pub value: JsonValue,
}

/// Constraint type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConstraintType {
    /// Minimum value
    Min,
    /// Maximum value
    Max,
    /// Allowed values
    Enum,
    /// Pattern (for strings)
    Pattern,
    /// Custom constraint
    Custom(String),
}

/// Capability matcher for finding components by capability
pub struct CapabilityMatcher;

impl CapabilityMatcher {
    /// Find components that match a required capability
    pub fn find_matching<'a>(
        required: &Capability,
        components: &'a [crate::component::ComponentInfo],
    ) -> Vec<&'a crate::component::ComponentInfo> {
        components
            .iter()
            .filter(|comp| {
                comp.is_available() && comp.capabilities.iter().any(|c| c.matches(required))
            })
            .collect()
    }
    
    /// Find components that are compatible with a required capability
    pub fn find_compatible<'a>(
        required: &Capability,
        components: &'a [crate::component::ComponentInfo],
    ) -> Vec<&'a crate::component::ComponentInfo> {
        components
            .iter()
            .filter(|comp| {
                comp.is_available() && comp.capabilities.iter().any(|c| c.is_compatible(required))
            })
            .collect()
    }
    
    /// Validate that a capability can handle a command
    pub fn validate_command(
        capability: &Capability,
        command: &JsonValue,
    ) -> Result<(), String> {
        match capability {
            Capability::Simple(_) => {
                // Simple capabilities accept any command
                Ok(())
            }
            Capability::Structured(s) => {
                // Validate against parameter schema
                if let Some(obj) = command.as_object() {
                    for param in &s.parameters {
                        if param.required {
                            if !obj.contains_key(&param.name) {
                                return Err(format!("Required parameter '{}' missing", param.name));
                            }
                        }
                        
                        // Type validation
                        if let Some(value) = obj.get(&param.name) {
                            if !Self::validate_parameter_type(value, &param.param_type) {
                                return Err(format!(
                                    "Parameter '{}' has wrong type",
                                    param.name
                                ));
                            }
                            
                            // Constraint validation
                            for constraint in &s.constraints {
                                if constraint.target == param.name {
                                    if let Err(e) = Self::validate_constraint(value, constraint) {
                                        return Err(e);
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(())
            }
        }
    }
    
    fn validate_parameter_type(value: &JsonValue, param_type: &ParameterType) -> bool {
        match param_type {
            ParameterType::String => value.is_string(),
            ParameterType::Integer => value.is_i64() || value.is_u64(),
            ParameterType::Float => value.is_f64() || value.is_i64() || value.is_u64(),
            ParameterType::Boolean => value.is_boolean(),
            ParameterType::Array => value.is_array(),
            ParameterType::Object => value.is_object(),
            ParameterType::Custom(_) => true, // Custom types not validated here
        }
    }
    
    fn validate_constraint(value: &JsonValue, constraint: &Constraint) -> Result<(), String> {
        match constraint.constraint_type {
            ConstraintType::Min => {
                if let Some(num) = value.as_f64() {
                    if let Some(min) = constraint.value.as_f64() {
                        if num < min {
                            return Err(format!("Value {} below minimum {}", num, min));
                        }
                    }
                }
            }
            ConstraintType::Max => {
                if let Some(num) = value.as_f64() {
                    if let Some(max) = constraint.value.as_f64() {
                        if num > max {
                            return Err(format!("Value {} above maximum {}", num, max));
                        }
                    }
                }
            }
            ConstraintType::Enum => {
                if let Some(allowed) = constraint.value.as_array() {
                    if !allowed.contains(value) {
                        return Err(format!("Value not in allowed enum values"));
                    }
                }
            }
            ConstraintType::Pattern => {
                // Pattern validation would require regex
                // For now, skip
            }
            ConstraintType::Custom(_) => {
                // Custom constraints not validated here
            }
        }
        Ok(())
    }
}

