// JSON and semi-structured data support - ClickHouse limitation

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// JSON column type for semi-structured data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonColumn {
    pub values: Vec<JsonValue>,
}

impl JsonColumn {
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Extract nested value using JSONPath
    pub fn extract(&self, path: &str) -> Vec<Option<JsonValue>> {
        self.values.iter()
            .map(|v| extract_json_path(v, path))
            .collect()
    }

    /// Filter by JSONPath condition
    pub fn filter(&self, path: &str, condition: JsonCondition) -> Vec<bool> {
        self.values.iter()
            .map(|v| {
                if let Some(value) = extract_json_path(v, path) {
                    match condition {
                        JsonCondition::Eq(ref expected) => &value == expected,
                        JsonCondition::Ne(ref expected) => &value != expected,
                        JsonCondition::Gt(ref expected) => compare_json(&value, expected) > 0,
                        JsonCondition::Lt(ref expected) => compare_json(&value, expected) < 0,
                        JsonCondition::Contains(ref expected) => {
                            if let Some(arr) = value.as_array() {
                                arr.contains(expected)
                            } else {
                                false
                            }
                        }
                        JsonCondition::Exists => true,
                    }
                } else {
                    false
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub enum JsonCondition {
    Eq(JsonValue),
    Ne(JsonValue),
    Gt(JsonValue),
    Lt(JsonValue),
    Contains(JsonValue),
    Exists,
}

/// Extract value from JSON using JSONPath
fn extract_json_path(json: &JsonValue, path: &str) -> Option<JsonValue> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = json;

    for part in parts {
        if part.starts_with('[') && part.ends_with(']') {
            // Array access
            // SECURITY: Validate part length before slicing to prevent panic
            if part.len() < 3 {
                return None; // Need at least "[0]" format
            }
            let index_str = &part[1..part.len()-1];
            if let Ok(index) = index_str.parse::<usize>() {
                if let Some(arr) = current.as_array() {
                    // SECURITY: Bounds check before accessing array (get() already does this safely)
                    current = arr.get(index)?;
                } else {
                    return None;
                }
            }
        } else {
            // Object access
            if let Some(obj) = current.as_object() {
                current = obj.get(part)?;
            } else {
                return None;
            }
        }
    }

    Some(current.clone())
}

/// Compare JSON values
fn compare_json(a: &JsonValue, b: &JsonValue) -> i32 {
    match (a, b) {
        (JsonValue::Number(n1), JsonValue::Number(n2)) => {
            if let (Some(f1), Some(f2)) = (n1.as_f64(), n2.as_f64()) {
                (f1 - f2).signum() as i32
            } else {
                0
            }
        }
        (JsonValue::String(s1), JsonValue::String(s2)) => {
            s1.cmp(s2) as i32
        }
        _ => 0,
    }
}

/// XML support (semi-structured)
pub struct XmlColumn {
    pub values: Vec<String>, // XML strings
}

impl XmlColumn {
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
        }
    }

    /// Extract value using XPath
    pub fn extract_xpath(&self, xpath: &str) -> Vec<Option<String>> {
        // In production, would use XML parser
        self.values.iter()
            .map(|_| None)
            .collect()
    }
}

/// Flexible schema for semi-structured data
pub struct FlexibleSchema {
    pub fields: HashMap<String, FieldType>,
    pub allow_extra: bool,
}

#[derive(Debug, Clone)]
pub enum FieldType {
    Required(DataType),
    Optional(DataType),
    Json,      // JSON object
    JsonArray, // JSON array
    Xml,       // XML
}

use crate::schema::DataType;

impl FlexibleSchema {
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
            allow_extra: true,
        }
    }

    /// Add required field
    pub fn required(mut self, name: &str, data_type: DataType) -> Self {
        self.fields.insert(name.to_string(), FieldType::Required(data_type));
        self
    }

    /// Add optional field
    pub fn optional(mut self, name: &str, data_type: DataType) -> Self {
        self.fields.insert(name.to_string(), FieldType::Optional(data_type));
        self
    }

    /// Add JSON field
    pub fn json(mut self, name: &str) -> Self {
        self.fields.insert(name.to_string(), FieldType::Json);
        self
    }

    /// Validate data against schema
    pub fn validate(&self, data: &HashMap<String, JsonValue>) -> Result<(), String> {
        for (name, field_type) in &self.fields {
            match field_type {
                FieldType::Required(_) => {
                    if !data.contains_key(name) {
                        return Err(format!("Required field '{}' is missing", name));
                    }
                }
                FieldType::Optional(_) => {
                    // Optional fields are OK to be missing
                }
                FieldType::Json | FieldType::JsonArray => {
                    // JSON fields are flexible
                }
                FieldType::Xml => {
                    // XML fields are flexible
                }
            }
        }

        if !self.allow_extra {
            for key in data.keys() {
                if !self.fields.contains_key(key) {
                    return Err(format!("Extra field '{}' not allowed", key));
                }
            }
        }

        Ok(())
    }
}

use std::result::Result;

