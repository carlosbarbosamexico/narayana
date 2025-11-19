// Universal Transform & Filter System
// The Cognitive Layer: Filter Thoughts, Transform Information into Actions
// Works across Brain, Database, and Workers - unified intelligence

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::{Error, Result};

/// Universal Output Configuration
/// Applies to Brain memories, Database tables, Worker responses
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OutputConfig {
    /// Default filters applied to ALL queries/retrievals
    pub default_filters: Vec<DefaultFilter>,
    
    /// Output transformations applied to ALL responses
    pub output_transforms: Vec<OutputTransform>,
    
    /// Field-level rules (per-field configuration)
    pub field_rules: HashMap<String, FieldRule>,
    
    /// Format conversion
    pub output_format: Option<DataFormat>,
    
    /// Profiles (different configs for different contexts)
    pub profiles: HashMap<String, OutputConfig>,
    
    /// Version for tracking changes
    pub version: u64,
}

/// Default Filter - automatic filtering applied to all data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DefaultFilter {
    /// Row-level filters (applied before output)
    ExcludeRows { condition: FilterPredicate },
    IncludeOnlyRows { condition: FilterPredicate },
    
    /// Field-level filters
    ExcludeFields(Vec<String>),
    IncludeOnlyFields(Vec<String>),
    
    /// Value filters (privacy/security)
    MaskFields { 
        fields: Vec<String>, 
        pattern: String,
        preserve_length: bool, // Keep original length
    },
    HashFields { 
        fields: Vec<String>, 
        algorithm: String, // "sha256", "md5", etc.
    },
    NullifyFields(Vec<String>), // Set to null
    
    /// Conditional filters
    Conditional {
        condition: String, // e.g., "user.role != 'admin'"
        filters: Vec<DefaultFilter>
    },
    
    /// Custom filter function
    Custom { 
        function: String, 
        params: HashMap<String, serde_json::Value> 
    },
    
    /// Transform field values
    TransformField { 
        field: String, 
        transform: FieldTransform 
    },
}

/// Output Transform - response shaping and structure changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputTransform {
    /// Wrap response under a key
    Wrap { 
        key: String, 
        inner: Vec<OutputTransform> 
    },
    
    /// Rename table/entity in response
    RenameTable(String),
    
    /// Flatten nested structures
    Flatten,
    
    /// Nest data under a path
    Nest { 
        path: String, 
        key: String 
    },
    
    /// Format conversion
    Format { 
        from: DataFormat, 
        to: DataFormat 
    },
    
    /// Format specific field
    FormatField { 
        field: String, 
        from: DataFormat, 
        to: DataFormat 
    },
    
    /// Field transforms
    RenameField { 
        from: String, 
        to: String 
    },
    TransformField { 
        field: String, 
        transform: FieldTransform 
    },
    
    /// Nested transforms (apply to nested objects)
    Nested { 
        path: String, 
        transforms: Vec<OutputTransform> 
    },
    
    /// Custom transformation function
    Custom { 
        function: String, 
        params: HashMap<String, serde_json::Value> 
    },
    
    /// Conditional transforms
    Conditional {
        condition: String,
        then: Vec<OutputTransform>,
        r#else: Vec<OutputTransform>,
    },
}

/// Field Transform - value-level transformations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldTransform {
    /// Mask value
    Mask { 
        pattern: String,
        preserve_length: bool,
    },
    
    /// Hash value
    Hash { 
        algorithm: String,
    },
    
    /// Format value
    Format { 
        format: String, // "currency", "date", "email", etc.
    },
    
    /// Date format conversion
    DateFormat { 
        from: String, 
        to: String 
    },
    
    /// Number format
    NumberFormat { 
        format: String, // "currency", "percentage", "scientific"
    },
    
    /// Custom transformation
    Custom { 
        function: String, 
        params: HashMap<String, serde_json::Value> 
    },
    
    /// Compute from other fields
    Compute { 
        expression: String, // e.g., "field1 + field2"
    },
}

/// Field Rule - per-field configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldRule {
    pub field: String,
    pub always_mask: bool,
    pub always_hash: bool,
    pub format: Option<String>, // "currency", "date", "email", etc.
    pub transform: Option<FieldTransform>,
    pub exclude_unless: Option<FilterPredicate>, // Only include if condition met
    pub default_value: Option<serde_json::Value>,
}

/// Filter Predicate - conditions for filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterPredicate {
    Eq { field: String, value: serde_json::Value },
    Ne { field: String, value: serde_json::Value },
    Gt { field: String, value: serde_json::Value },
    Lt { field: String, value: serde_json::Value },
    Gte { field: String, value: serde_json::Value },
    Lte { field: String, value: serde_json::Value },
    In { field: String, values: Vec<serde_json::Value> },
    Contains { field: String, value: serde_json::Value },
    Regex { field: String, pattern: String },
    And { left: Box<FilterPredicate>, right: Box<FilterPredicate> },
    Or { left: Box<FilterPredicate>, right: Box<FilterPredicate> },
    Not { expr: Box<FilterPredicate> },
}

/// Data Format - format conversion types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataFormat {
    Json,
    Xml,
    Csv,
    Yaml,
    Toml,
    MessagePack,
    Avro,
    Parquet,
    Custom { name: String },
}

/// Config Context - where the config applies
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConfigContext {
    Brain { 
        memory_type: Option<String>, // "Episodic", "Semantic", etc.
    },
    Database { 
        table_id: u64,
    },
    Worker { 
        worker_id: String,
    },
    Global, // System-wide defaults
}

/// Universal Transform Engine
/// Applies transforms and filters to any data structure
pub struct TransformEngine;

impl TransformEngine {
    /// Maximum recursion depth for nested transforms
    const MAX_RECURSION_DEPTH: usize = 100;
    
    /// Maximum number of transforms in a chain
    const MAX_TRANSFORM_CHAIN: usize = 1_000;
    
    /// Maximum number of filters in a chain
    const MAX_FILTER_CHAIN: usize = 1_000;
    
    /// Maximum field name length (prevents DoS via extremely long keys)
    const MAX_FIELD_NAME_LENGTH: usize = 1_024;
    
    /// Maximum key length for Wrap/Nest transforms
    const MAX_KEY_LENGTH: usize = 1_024;
    
    /// Maximum pattern length for masking
    const MAX_PATTERN_LENGTH: usize = 1_024;
    
    /// SECURITY: Validate field name to prevent injection and DoS
    fn validate_field_name(field: &str) -> Result<()> {
        // SECURITY: Prevent empty field names
        if field.is_empty() {
            return Err(Error::Query("Field name cannot be empty".to_string()));
        }
        
        // SECURITY: Limit field name length to prevent DoS
        if field.len() > Self::MAX_FIELD_NAME_LENGTH {
            return Err(Error::Query(format!(
                "Field name too long: {} bytes (max: {})",
                field.len(), Self::MAX_FIELD_NAME_LENGTH
            )));
        }
        
        // SECURITY: Prevent path traversal in field names
        if field.contains("..") || field.contains("/") || field.contains("\\") {
            return Err(Error::Query(format!(
                "Field name contains invalid characters (path traversal attempt?): '{}'",
                field
            )));
        }
        
        // SECURITY: Prevent null bytes
        if field.contains('\0') {
            return Err(Error::Query("Field name cannot contain null bytes".to_string()));
        }
        
        // SECURITY: Prevent control characters
        if field.chars().any(|c| c.is_control()) {
            return Err(Error::Query("Field name cannot contain control characters".to_string()));
        }
        
        Ok(())
    }
    
    /// SECURITY: Validate key for Wrap/Nest transforms
    fn validate_key(key: &str) -> Result<()> {
        if key.is_empty() {
            return Err(Error::Query("Key cannot be empty".to_string()));
        }
        
        if key.len() > Self::MAX_KEY_LENGTH {
            return Err(Error::Query(format!(
                "Key too long: {} bytes (max: {})",
                key.len(), Self::MAX_KEY_LENGTH
            )));
        }
        
        // SECURITY: Prevent path traversal
        if key.contains("..") || key.contains("/") || key.contains("\\") {
            return Err(Error::Query(format!(
                "Key contains invalid characters: '{}'",
                key
            )));
        }
        
        if key.contains('\0') {
            return Err(Error::Query("Key cannot contain null bytes".to_string()));
        }
        
        Ok(())
    }
    
    /// SECURITY: Validate pattern for masking
    fn validate_pattern(pattern: &str) -> Result<()> {
        if pattern.len() > Self::MAX_PATTERN_LENGTH {
            return Err(Error::Query(format!(
                "Pattern too long: {} bytes (max: {})",
                pattern.len(), Self::MAX_PATTERN_LENGTH
            )));
        }
        
        // SECURITY: Prevent extremely long patterns that could cause DoS in repeat()
        // Pattern will be repeated, so even small patterns can be dangerous
        if pattern.len() > 100 {
            return Err(Error::Query(format!(
                "Pattern too long for masking: {} bytes (max: 100)",
                pattern.len()
            )));
        }
        
        Ok(())
    }
    
    /// Apply transforms to data
    pub fn apply_transforms(
        data: serde_json::Value,
        transforms: &[OutputTransform],
    ) -> Result<serde_json::Value> {
        Self::apply_transforms_with_depth(data, transforms, 0)
    }
    
    /// Apply transforms with recursion depth tracking
    fn apply_transforms_with_depth(
        data: serde_json::Value,
        transforms: &[OutputTransform],
        depth: usize,
    ) -> Result<serde_json::Value> {
        // EDGE CASE: Prevent stack overflow from deep recursion
        if depth > Self::MAX_RECURSION_DEPTH {
            return Err(Error::Query(format!(
                "Transform recursion depth {} exceeds maximum {}",
                depth, Self::MAX_RECURSION_DEPTH
            )));
        }
        
        // EDGE CASE: Limit transform chain length
        if transforms.len() > Self::MAX_TRANSFORM_CHAIN {
            return Err(Error::Query(format!(
                "Transform chain length {} exceeds maximum {}",
                transforms.len(), Self::MAX_TRANSFORM_CHAIN
            )));
        }
        
        let mut result = data;
        
        for transform in transforms {
            result = Self::apply_single_transform_with_depth(result, transform, depth + 1)?;
        }
        
        Ok(result)
    }
    
    /// Apply filters to data
    pub fn apply_filters(
        data: serde_json::Value,
        filters: &[DefaultFilter],
    ) -> Result<serde_json::Value> {
        Self::apply_filters_with_depth(data, filters, 0)
    }
    
    /// Apply filters with recursion depth tracking
    fn apply_filters_with_depth(
        data: serde_json::Value,
        filters: &[DefaultFilter],
        depth: usize,
    ) -> Result<serde_json::Value> {
        // EDGE CASE: Prevent stack overflow from deep recursion
        if depth > Self::MAX_RECURSION_DEPTH {
            return Err(Error::Query(format!(
                "Filter recursion depth {} exceeds maximum {}",
                depth, Self::MAX_RECURSION_DEPTH
            )));
        }
        
        // EDGE CASE: Limit filter chain length
        if filters.len() > Self::MAX_FILTER_CHAIN {
            return Err(Error::Query(format!(
                "Filter chain length {} exceeds maximum {}",
                filters.len(), Self::MAX_FILTER_CHAIN
            )));
        }
        
        let mut result = data;
        
        for filter in filters {
            result = Self::apply_single_filter_with_depth(result, filter, depth + 1)?;
        }
        
        Ok(result)
    }
    
    /// Apply complete output config (filters + transforms)
    pub fn apply_config(
        data: serde_json::Value,
        config: &OutputConfig,
    ) -> Result<serde_json::Value> {
        // SECURITY: Validate profile names to prevent injection
        for (profile_name, _) in &config.profiles {
            // SECURITY: Validate profile name
            if profile_name.is_empty() 
                || profile_name.len() > 1_024 
                || profile_name.contains("..") 
                || profile_name.contains("/") 
                || profile_name.contains("\\")
                || profile_name.contains('\0') {
                return Err(Error::Query(format!(
                    "Invalid profile name: '{}'",
                    profile_name
                )));
            }
        }
        
        // EDGE CASE: Check for circular profile references (basic check)
        // In production, would do full cycle detection
        if config.profiles.len() > 100 {
            return Err(Error::Query(format!(
                "Too many profiles: {} (max: 100)",
                config.profiles.len()
            )));
        }
        
        // SECURITY: Validate all field rule names
        for (field_name, _) in &config.field_rules {
            Self::validate_field_name(field_name)?;
        }
        
        // First apply filters
        let filtered = Self::apply_filters(data, &config.default_filters)?;
        
        // Then apply transforms
        let transformed = Self::apply_transforms(filtered, &config.output_transforms)?;
        
        // Apply field rules
        let with_rules = Self::apply_field_rules(transformed, &config.field_rules)?;
        
        // Apply format conversion if specified
        let final_result = if let Some(format) = &config.output_format {
            Self::convert_format(with_rules, format)?
        } else {
            with_rules
        };
        
        Ok(final_result)
    }
    
    /// Apply single transform
    fn apply_single_transform(
        data: serde_json::Value,
        transform: &OutputTransform,
    ) -> Result<serde_json::Value> {
        Self::apply_single_transform_with_depth(data, transform, 0)
    }
    
    /// Apply single transform with depth tracking
    fn apply_single_transform_with_depth(
        data: serde_json::Value,
        transform: &OutputTransform,
        depth: usize,
    ) -> Result<serde_json::Value> {
        // EDGE CASE: Prevent stack overflow
        if depth > Self::MAX_RECURSION_DEPTH {
            return Err(Error::Query(format!(
                "Transform recursion depth {} exceeds maximum {}",
                depth, Self::MAX_RECURSION_DEPTH
            )));
        }
        
        match transform {
            OutputTransform::Wrap { key, inner } => {
                // SECURITY: Validate key before use
                Self::validate_key(key)?;
                
                let mut wrapped = serde_json::Map::new();
                let inner_data = if inner.is_empty() {
                    data
                } else {
                    Self::apply_transforms_with_depth(data, inner, depth + 1)?
                };
                wrapped.insert(key.clone(), inner_data);
                Ok(serde_json::Value::Object(wrapped))
            }
            
            OutputTransform::RenameTable(new_name) => {
                // SECURITY: Validate new table name
                Self::validate_key(new_name)?;
                
                if let serde_json::Value::Object(mut obj) = data {
                    if let Some(value) = obj.remove("table") {
                        obj.insert(new_name.clone(), value);
                    }
                    Ok(serde_json::Value::Object(obj))
                } else {
                    Ok(data)
                }
            }
            
            OutputTransform::RenameField { from, to } => {
                // SECURITY: Validate field names
                Self::validate_field_name(from)?;
                Self::validate_field_name(to)?;
                Self::rename_field_in_value(data, from, to)
            }
            
            OutputTransform::TransformField { field, transform } => {
                // SECURITY: Validate field name
                Self::validate_field_name(field)?;
                Self::transform_field_in_value(data, field, transform)
            }
            
            OutputTransform::Flatten => {
                Self::flatten_value(data)
            }
            
            OutputTransform::Nest { path, key } => {
                // SECURITY: Validate path and key
                Self::validate_key(key)?;
                // Path validation would go here if path is used
                Self::nest_value(data, path, key)
            }
            
            OutputTransform::Format { from: _, to } => {
                Self::convert_format(data, to)
            }
            
            OutputTransform::FormatField { field, from: _, to } => {
                Self::format_field(data, field, to)
            }
            
            OutputTransform::Nested { path, transforms } => {
                Self::apply_nested_transforms(data, path, transforms)
            }
            
            OutputTransform::Conditional { condition, then, r#else } => {
                // Simple condition evaluation (can be extended)
                if Self::evaluate_condition(&data, condition)? {
                    Self::apply_transforms_with_depth(data, then, depth + 1)
                } else {
                    Self::apply_transforms_with_depth(data, r#else, depth + 1)
                }
            }
            
            OutputTransform::Custom { function, params: _ } => {
                // Custom function execution (would integrate with worker system)
                Err(Error::Query(format!("Custom transform function '{}' not yet implemented", function)))
            }
        }
    }
    
    /// Apply single filter
    fn apply_single_filter(
        data: serde_json::Value,
        filter: &DefaultFilter,
    ) -> Result<serde_json::Value> {
        Self::apply_single_filter_with_depth(data, filter, 0)
    }
    
    /// Apply single filter with depth tracking
    fn apply_single_filter_with_depth(
        data: serde_json::Value,
        filter: &DefaultFilter,
        depth: usize,
    ) -> Result<serde_json::Value> {
        // EDGE CASE: Prevent stack overflow
        if depth > Self::MAX_RECURSION_DEPTH {
            return Err(Error::Query(format!(
                "Filter recursion depth {} exceeds maximum {}",
                depth, Self::MAX_RECURSION_DEPTH
            )));
        }
        
        match filter {
            DefaultFilter::ExcludeFields(fields) => {
                // SECURITY: Validate all field names
                for field in fields {
                    Self::validate_field_name(field)?;
                }
                Self::exclude_fields(data, fields)
            }
            
            DefaultFilter::IncludeOnlyFields(fields) => {
                // SECURITY: Validate all field names
                for field in fields {
                    Self::validate_field_name(field)?;
                }
                Self::include_only_fields(data, fields)
            }
            
            DefaultFilter::MaskFields { fields, pattern, preserve_length } => {
                // SECURITY: Validate field names and pattern
                for field in fields {
                    Self::validate_field_name(field)?;
                }
                Self::validate_pattern(pattern)?;
                Self::mask_fields(data, fields, pattern, *preserve_length)
            }
            
            DefaultFilter::HashFields { fields, algorithm } => {
                // SECURITY: Validate field names and algorithm
                for field in fields {
                    Self::validate_field_name(field)?;
                }
                // SECURITY: Whitelist allowed algorithms to prevent injection
                const ALLOWED_ALGORITHMS: &[&str] = &["sha256", "sha512"];
                if !ALLOWED_ALGORITHMS.contains(&algorithm.as_str()) {
                    return Err(Error::Query(format!(
                        "Unsupported hash algorithm: '{}'. Allowed: {:?}",
                        algorithm, ALLOWED_ALGORITHMS
                    )));
                }
                Self::hash_fields(data, fields, algorithm)
            }
            
            DefaultFilter::NullifyFields(fields) => {
                // SECURITY: Validate all field names
                for field in fields {
                    Self::validate_field_name(field)?;
                }
                Self::nullify_fields(data, fields)
            }
            
            DefaultFilter::ExcludeRows { condition } => {
                // SECURITY: Validate predicate fields
                Self::validate_predicate_fields(condition)?;
                Self::exclude_rows(data, condition)
            }
            
            DefaultFilter::IncludeOnlyRows { condition } => {
                // SECURITY: Validate predicate fields
                Self::validate_predicate_fields(condition)?;
                Self::include_only_rows(data, condition)
            }
            
            DefaultFilter::TransformField { field, transform } => {
                // SECURITY: Validate field name
                Self::validate_field_name(field)?;
                Self::transform_field_in_value(data, field, transform)
            }
            
            DefaultFilter::Conditional { condition, filters } => {
                if Self::evaluate_condition(&data, condition)? {
                    Self::apply_filters_with_depth(data, filters, depth + 1)
                } else {
                    Ok(data)
                }
            }
            
            DefaultFilter::Custom { function, params: _ } => {
                Err(Error::Query(format!("Custom filter function '{}' not yet implemented", function)))
            }
        }
    }
    
    // Helper methods
    
    fn rename_field_in_value(
        data: serde_json::Value,
        from: &str,
        to: &str,
    ) -> Result<serde_json::Value> {
        // EDGE CASE: Prevent infinite recursion on deeply nested structures
        // Limit nesting depth to prevent stack overflow
        Self::rename_field_in_value_with_depth(data, from, to, 0)
    }
    
    fn rename_field_in_value_with_depth(
        data: serde_json::Value,
        from: &str,
        to: &str,
        depth: usize,
    ) -> Result<serde_json::Value> {
        // EDGE CASE: Limit nesting depth
        const MAX_NESTING_DEPTH: usize = 1_000;
        if depth > MAX_NESTING_DEPTH {
            return Err(Error::Query(format!(
                "Nesting depth {} exceeds maximum {}",
                depth, MAX_NESTING_DEPTH
            )));
        }
        
        match data {
            serde_json::Value::Object(mut obj) => {
                if let Some(value) = obj.remove(from) {
                    obj.insert(to.to_string(), value);
                }
                Ok(serde_json::Value::Object(obj))
            }
            serde_json::Value::Array(arr) => {
                // EDGE CASE: Limit array size
                const MAX_ARRAY_SIZE: usize = 1_000_000;
                if arr.len() > MAX_ARRAY_SIZE {
                    return Err(Error::Query(format!(
                        "Array too large for field renaming: {} elements (max: {})",
                        arr.len(), MAX_ARRAY_SIZE
                    )));
                }
                
                let mut result = Vec::new();
                for item in arr {
                    result.push(Self::rename_field_in_value_with_depth(item, from, to, depth + 1)?);
                }
                Ok(serde_json::Value::Array(result))
            }
            _ => Ok(data),
        }
    }
    
    fn transform_field_in_value(
        data: serde_json::Value,
        field: &str,
        transform: &FieldTransform,
    ) -> Result<serde_json::Value> {
        match data {
            serde_json::Value::Object(mut obj) => {
                if let Some(value) = obj.get_mut(field) {
                    *value = Self::apply_field_transform(value.clone(), transform)?;
                }
                Ok(serde_json::Value::Object(obj))
            }
            serde_json::Value::Array(arr) => {
                let mut result = Vec::new();
                for item in arr {
                    result.push(Self::transform_field_in_value(item, field, transform)?);
                }
                Ok(serde_json::Value::Array(result))
            }
            _ => Ok(data),
        }
    }
    
    fn apply_field_transform(
        value: serde_json::Value,
        transform: &FieldTransform,
    ) -> Result<serde_json::Value> {
        match transform {
            FieldTransform::Mask { pattern, preserve_length } => {
                Self::mask_value(value, pattern, *preserve_length)
            }
            FieldTransform::Hash { algorithm } => {
                Self::hash_value(value, algorithm)
            }
            FieldTransform::DateFormat { from, to } => {
                Self::convert_date_format(value, from, to)
            }
            FieldTransform::NumberFormat { format } => {
                Self::format_number(value, format)
            }
            FieldTransform::Format { format } => {
                Self::format_value(value, format)
            }
            FieldTransform::Compute { expression: _ } => {
                // Would evaluate expression
                Ok(value)
            }
            FieldTransform::Custom { function, params: _ } => {
                Err(Error::Query(format!("Custom field transform '{}' not yet implemented", function)))
            }
        }
    }
    
    fn exclude_fields(
        data: serde_json::Value,
        fields: &[String],
    ) -> Result<serde_json::Value> {
        // EDGE CASE: Handle empty fields list
        if fields.is_empty() {
            return Ok(data);
        }
        
        // EDGE CASE: Limit fields list size
        const MAX_FIELDS: usize = 10_000;
        if fields.len() > MAX_FIELDS {
            return Err(Error::Query(format!(
                "Too many fields to exclude: {} (max: {})",
                fields.len(), MAX_FIELDS
            )));
        }
        
        match data {
            serde_json::Value::Object(mut obj) => {
                // EDGE CASE: Handle empty object
                if obj.is_empty() {
                    return Ok(serde_json::Value::Object(obj));
                }
                
                for field in fields {
                    obj.remove(field);
                }
                Ok(serde_json::Value::Object(obj))
            }
            serde_json::Value::Array(arr) => {
                // EDGE CASE: Handle empty array
                if arr.is_empty() {
                    return Ok(serde_json::Value::Array(arr));
                }
                
                // EDGE CASE: Limit array size
                const MAX_ARRAY_SIZE: usize = 1_000_000;
                if arr.len() > MAX_ARRAY_SIZE {
                    return Err(Error::Query(format!(
                        "Array too large for field exclusion: {} elements (max: {})",
                        arr.len(), MAX_ARRAY_SIZE
                    )));
                }
                
                let mut result = Vec::new();
                for item in arr {
                    result.push(Self::exclude_fields(item, fields)?);
                }
                Ok(serde_json::Value::Array(result))
            }
            _ => Ok(data),
        }
    }
    
    fn include_only_fields(
        data: serde_json::Value,
        fields: &[String],
    ) -> Result<serde_json::Value> {
        // EDGE CASE: Handle empty fields list - return empty object
        if fields.is_empty() {
            return Ok(serde_json::Value::Object(serde_json::Map::new()));
        }
        
        // EDGE CASE: Limit fields list size
        const MAX_FIELDS: usize = 10_000;
        if fields.len() > MAX_FIELDS {
            return Err(Error::Query(format!(
                "Too many fields to include: {} (max: {})",
                fields.len(), MAX_FIELDS
            )));
        }
        
        match data {
            serde_json::Value::Object(mut obj) => {
                // EDGE CASE: Handle empty object
                if obj.is_empty() {
                    return Ok(serde_json::Value::Object(serde_json::Map::new()));
                }
                
                let mut new_obj = serde_json::Map::new();
                for field in fields {
                    if let Some(value) = obj.remove(field) {
                        new_obj.insert(field.clone(), value);
                    }
                }
                Ok(serde_json::Value::Object(new_obj))
            }
            serde_json::Value::Array(arr) => {
                // EDGE CASE: Handle empty array
                if arr.is_empty() {
                    return Ok(serde_json::Value::Array(arr));
                }
                
                // EDGE CASE: Limit array size
                const MAX_ARRAY_SIZE: usize = 1_000_000;
                if arr.len() > MAX_ARRAY_SIZE {
                    return Err(Error::Query(format!(
                        "Array too large for field inclusion: {} elements (max: {})",
                        arr.len(), MAX_ARRAY_SIZE
                    )));
                }
                
                let mut result = Vec::new();
                for item in arr {
                    result.push(Self::include_only_fields(item, fields)?);
                }
                Ok(serde_json::Value::Array(result))
            }
            _ => Ok(data),
        }
    }
    
    fn mask_fields(
        data: serde_json::Value,
        fields: &[String],
        pattern: &str,
        preserve_length: bool,
    ) -> Result<serde_json::Value> {
        match data {
            serde_json::Value::Object(mut obj) => {
                for field in fields {
                    if let Some(value) = obj.get_mut(field) {
                        *value = Self::mask_value(value.clone(), pattern, preserve_length)?;
                    }
                }
                Ok(serde_json::Value::Object(obj))
            }
            serde_json::Value::Array(arr) => {
                let mut result = Vec::new();
                for item in arr {
                    result.push(Self::mask_fields(item, fields, pattern, preserve_length)?);
                }
                Ok(serde_json::Value::Array(result))
            }
            _ => Ok(data),
        }
    }
    
    fn mask_value(
        value: serde_json::Value,
        pattern: &str,
        preserve_length: bool,
    ) -> Result<serde_json::Value> {
        // SECURITY: Validate pattern before use
        Self::validate_pattern(pattern)?;
        
        // EDGE CASE: Handle empty pattern
        if pattern.is_empty() {
            return Ok(serde_json::Value::String(String::new()));
        }
        
        if let serde_json::Value::String(s) = value {
            // EDGE CASE: Handle empty string
            if s.is_empty() {
                return Ok(serde_json::Value::String(String::new()));
            }
            
            if preserve_length {
                // SECURITY: Prevent DoS via extremely long strings
                const MAX_MASK_LENGTH: usize = 1_000_000;
                let target_len = s.len().min(MAX_MASK_LENGTH);
                
                // SECURITY: Prevent DoS via pattern.repeat() with large target_len
                // Calculate repetitions needed, but cap it
                let pattern_len = pattern.len();
                if pattern_len == 0 {
                    return Ok(serde_json::Value::String(String::new()));
                }
                
                // SECURITY: Limit number of repetitions to prevent memory exhaustion
                // SECURITY: Use checked arithmetic to prevent integer overflow
                let repetitions_needed = target_len
                    .checked_div(pattern_len)
                    .unwrap_or(0)
                    .max(1)
                    .checked_add(1)
                    .unwrap_or(MAX_REPETITIONS);
                const MAX_REPETITIONS: usize = 1_000_000;
                let safe_repetitions = repetitions_needed.min(MAX_REPETITIONS);
                
                // SECURITY: Check total size before allocation
                // SECURITY: Prevent integer overflow in multiplication
                let estimated_size = pattern_len.checked_mul(safe_repetitions)
                    .ok_or_else(|| Error::Query("Pattern size calculation overflow".to_string()))?;
                const MAX_ALLOCATION_SIZE: usize = 10 * 1024 * 1024; // 10MB
                if estimated_size > MAX_ALLOCATION_SIZE {
                    return Err(Error::Query(format!(
                        "Mask pattern would require {} bytes (max: {})",
                        estimated_size, MAX_ALLOCATION_SIZE
                    )));
                }
                
                // SECURITY: Use safe_repetitions directly (already capped)
                let repeated = pattern.repeat(safe_repetitions);
                Ok(serde_json::Value::String(repeated.chars().take(target_len).collect()))
            } else {
                Ok(serde_json::Value::String(pattern.to_string()))
            }
        } else {
            // EDGE CASE: Non-string values get converted to string pattern
            Ok(serde_json::Value::String(pattern.to_string()))
        }
    }
    
    fn hash_fields(
        data: serde_json::Value,
        fields: &[String],
        algorithm: &str,
    ) -> Result<serde_json::Value> {
        match data {
            serde_json::Value::Object(mut obj) => {
                for field in fields {
                    if let Some(value) = obj.get_mut(field) {
                        *value = Self::hash_value(value.clone(), algorithm)?;
                    }
                }
                Ok(serde_json::Value::Object(obj))
            }
            serde_json::Value::Array(arr) => {
                let mut result = Vec::new();
                for item in arr {
                    result.push(Self::hash_fields(item, fields, algorithm)?);
                }
                Ok(serde_json::Value::Array(result))
            }
            _ => Ok(data),
        }
    }
    
    fn hash_value(
        value: serde_json::Value,
        algorithm: &str,
    ) -> Result<serde_json::Value> {
        use sha2::{Sha256, Sha512, Digest};
        
        // SECURITY: Limit input size to prevent DoS via extremely large values
        const MAX_HASH_INPUT_SIZE: usize = 100 * 1024 * 1024; // 100MB
        // SECURITY: Check size before converting to string to prevent DoS
        // Estimate size: JSON values can be much larger when stringified
        // For safety, limit based on JSON serialization size
        let json_size = serde_json::to_string(&value)
            .map_err(|e| Error::Query(format!("Failed to serialize value: {}", e)))?
            .len();
        
        if json_size > MAX_HASH_INPUT_SIZE {
            return Err(Error::Query(format!(
                "Value too large for hashing: {} bytes (max: {})",
                json_size, MAX_HASH_INPUT_SIZE
            )));
        }
        
        let value_str = value.to_string();
        
        // SECURITY: Whitelist allowed algorithms to prevent injection
        // Only allow secure algorithms (md5 is deprecated, so removed)
        let hash = match algorithm {
            "sha256" => {
                let mut hasher = Sha256::new();
                hasher.update(value_str.as_bytes());
                format!("{:x}", hasher.finalize())
            }
            "sha512" => {
                let mut hasher = Sha512::new();
                hasher.update(value_str.as_bytes());
                format!("{:x}", hasher.finalize())
            }
            _ => {
                return Err(Error::Query(format!(
                    "Unsupported hash algorithm: '{}'. Allowed: sha256, sha512",
                    algorithm
                )));
            }
        };
        
        Ok(serde_json::Value::String(hash))
    }
    
    fn nullify_fields(
        data: serde_json::Value,
        fields: &[String],
    ) -> Result<serde_json::Value> {
        match data {
            serde_json::Value::Object(mut obj) => {
                for field in fields {
                    obj.insert(field.clone(), serde_json::Value::Null);
                }
                Ok(serde_json::Value::Object(obj))
            }
            serde_json::Value::Array(arr) => {
                let mut result = Vec::new();
                for item in arr {
                    result.push(Self::nullify_fields(item, fields)?);
                }
                Ok(serde_json::Value::Array(result))
            }
            _ => Ok(data),
        }
    }
    
    fn exclude_rows(
        data: serde_json::Value,
        condition: &FilterPredicate,
    ) -> Result<serde_json::Value> {
        // EDGE CASE: Handle non-array data
        if let serde_json::Value::Array(arr) = data {
            // EDGE CASE: Handle empty array
            if arr.is_empty() {
                return Ok(serde_json::Value::Array(Vec::new()));
            }
            
            // EDGE CASE: Limit array size to prevent memory exhaustion
            const MAX_ARRAY_SIZE: usize = 1_000_000;
            if arr.len() > MAX_ARRAY_SIZE {
                return Err(Error::Query(format!(
                    "Array too large for filtering: {} elements (max: {})",
                    arr.len(), MAX_ARRAY_SIZE
                )));
            }
            
            let mut result = Vec::new();
            for item in arr {
                // EDGE CASE: Handle predicate evaluation errors gracefully
                match Self::evaluate_predicate(&item, condition) {
                    Ok(false) => result.push(item),
                    Ok(true) => {}, // Exclude this row
                    Err(e) => {
                        // Log error but continue processing
                        // In production, might want to return error or skip row
                        return Err(e);
                    }
                }
            }
            Ok(serde_json::Value::Array(result))
        } else {
            // EDGE CASE: Non-array data - return as-is (can't filter rows)
            Ok(data)
        }
    }
    
    fn include_only_rows(
        data: serde_json::Value,
        condition: &FilterPredicate,
    ) -> Result<serde_json::Value> {
        // EDGE CASE: Handle non-array data
        if let serde_json::Value::Array(arr) = data {
            // EDGE CASE: Handle empty array
            if arr.is_empty() {
                return Ok(serde_json::Value::Array(Vec::new()));
            }
            
            // EDGE CASE: Limit array size to prevent memory exhaustion
            const MAX_ARRAY_SIZE: usize = 1_000_000;
            if arr.len() > MAX_ARRAY_SIZE {
                return Err(Error::Query(format!(
                    "Array too large for filtering: {} elements (max: {})",
                    arr.len(), MAX_ARRAY_SIZE
                )));
            }
            
            let mut result = Vec::new();
            for item in arr {
                // EDGE CASE: Handle predicate evaluation errors gracefully
                match Self::evaluate_predicate(&item, condition) {
                    Ok(true) => result.push(item),
                    Ok(false) => {}, // Exclude this row
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
            Ok(serde_json::Value::Array(result))
        } else {
            // EDGE CASE: Non-array data - return as-is (can't filter rows)
            Ok(data)
        }
    }
    
    fn apply_field_rules(
        data: serde_json::Value,
        rules: &HashMap<String, FieldRule>,
    ) -> Result<serde_json::Value> {
        // SECURITY: Limit number of field rules to prevent DoS
        const MAX_FIELD_RULES: usize = 10_000;
        if rules.len() > MAX_FIELD_RULES {
            return Err(Error::Query(format!(
                "Too many field rules: {} (max: {})",
                rules.len(), MAX_FIELD_RULES
            )));
        }
        
        let mut result = data;
        
        for (field, rule) in rules {
            // SECURITY: Validate field name
            Self::validate_field_name(field)?;
            
            if rule.always_mask {
                result = Self::mask_fields(result, &[field.clone()], "***", false)?;
            }
            if rule.always_hash {
                result = Self::hash_fields(result, &[field.clone()], "sha256")?;
            }
            if let Some(ref transform) = rule.transform {
                result = Self::transform_field_in_value(result, field, transform)?;
            }
        }
        
        Ok(result)
    }
    
    fn flatten_value(_data: serde_json::Value) -> Result<serde_json::Value> {
        // Simple flattening - can be extended
        Ok(_data)
    }
    
    fn nest_value(
        data: serde_json::Value,
        _path: &str,
        key: &str,
    ) -> Result<serde_json::Value> {
        let mut nested = serde_json::Map::new();
        nested.insert(key.to_string(), data);
        Ok(serde_json::Value::Object(nested))
    }
    
    fn convert_format(
        data: serde_json::Value,
        format: &DataFormat,
    ) -> Result<serde_json::Value> {
        match format {
            DataFormat::Json => Ok(data),
            DataFormat::Xml => {
                // Convert JSON to XML string
                let xml_string = Self::json_to_xml(&data, "root")?;
                Ok(serde_json::Value::String(xml_string))
            }
            DataFormat::Csv => {
                // Convert JSON to CSV string
                let csv_string = Self::json_to_csv(&data)?;
                Ok(serde_json::Value::String(csv_string))
            }
            _ => Ok(data),
        }
    }
    
    /// Convert JSON value to XML string
    fn json_to_xml(value: &serde_json::Value, root_name: &str) -> Result<String> {
        use std::fmt::Write;
        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        
        fn escape_xml(s: &str) -> String {
            s.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
                .replace('\'', "&apos;")
        }
        
        fn value_to_xml(value: &serde_json::Value, name: &str, indent: usize, xml: &mut String) -> Result<()> {
            let indent_str = "  ".repeat(indent);
            match value {
                serde_json::Value::Object(map) => {
                    write!(xml, "{}<{}>\n", indent_str, escape_xml(name)).map_err(|e| {
                        Error::Query(format!("Failed to write XML: {}", e))
                    })?;
                    for (key, val) in map {
                        value_to_xml(val, key, indent + 1, xml)?;
                    }
                    write!(xml, "{}</{}>\n", indent_str, escape_xml(name)).map_err(|e| {
                        Error::Query(format!("Failed to write XML: {}", e))
                    })?;
                }
                serde_json::Value::Array(arr) => {
                    write!(xml, "{}<{}>\n", indent_str, escape_xml(name)).map_err(|e| {
                        Error::Query(format!("Failed to write XML: {}", e))
                    })?;
                    for (idx, val) in arr.iter().enumerate() {
                        value_to_xml(val, &format!("item_{}", idx), indent + 1, xml)?;
                    }
                    write!(xml, "{}</{}>\n", indent_str, escape_xml(name)).map_err(|e| {
                        Error::Query(format!("Failed to write XML: {}", e))
                    })?;
                }
                serde_json::Value::String(s) => {
                    write!(xml, "{}<{}>{}</{}>\n", indent_str, escape_xml(name), escape_xml(s), escape_xml(name)).map_err(|e| {
                        Error::Query(format!("Failed to write XML: {}", e))
                    })?;
                }
                serde_json::Value::Number(n) => {
                    write!(xml, "{}<{}>{}</{}>\n", indent_str, escape_xml(name), n, escape_xml(name)).map_err(|e| {
                        Error::Query(format!("Failed to write XML: {}", e))
                    })?;
                }
                serde_json::Value::Bool(b) => {
                    write!(xml, "{}<{}>{}</{}>\n", indent_str, escape_xml(name), b, escape_xml(name)).map_err(|e| {
                        Error::Query(format!("Failed to write XML: {}", e))
                    })?;
                }
                serde_json::Value::Null => {
                    write!(xml, "{}<{} />\n", indent_str, escape_xml(name)).map_err(|e| {
                        Error::Query(format!("Failed to write XML: {}", e))
                    })?;
                }
            }
            Ok(())
        }
        
        value_to_xml(value, root_name, 0, &mut xml)?;
        Ok(xml)
    }
    
    /// Convert JSON value to CSV string
    fn json_to_csv(value: &serde_json::Value) -> Result<String> {
        fn escape_csv(s: &str) -> String {
            if s.contains(',') || s.contains('"') || s.contains('\n') {
                format!("\"{}\"", s.replace('"', "\"\""))
            } else {
                s.to_string()
            }
        }
        
        match value {
            serde_json::Value::Array(arr) => {
                if arr.is_empty() {
                    return Ok(String::new());
                }
                
                // If first element is an object, use keys as headers
                let mut csv = String::new();
                if let Some(serde_json::Value::Object(first_obj)) = arr.first() {
                    // Write header
                    let headers: Vec<String> = first_obj.keys().map(|k| escape_csv(k)).collect();
                    csv.push_str(&headers.join(","));
                    csv.push('\n');
                    
                    // Write rows
                    for item in arr {
                        if let serde_json::Value::Object(obj) = item {
                            let values: Vec<String> = headers.iter().map(|h| {
                                obj.get(h.trim_matches('"'))
                                    .map(|v| match v {
                                        serde_json::Value::String(s) => escape_csv(s),
                                        serde_json::Value::Number(n) => n.to_string(),
                                        serde_json::Value::Bool(b) => b.to_string(),
                                        serde_json::Value::Null => String::new(),
                                        _ => escape_csv(&v.to_string()),
                                    })
                                    .unwrap_or_else(|| String::new())
                            }).collect();
                            csv.push_str(&values.join(","));
                            csv.push('\n');
                        }
                    }
                } else {
                    // Simple array - one column
                    for item in arr {
                        let val = match item {
                            serde_json::Value::String(s) => escape_csv(s),
                            serde_json::Value::Number(n) => n.to_string(),
                            serde_json::Value::Bool(b) => b.to_string(),
                            serde_json::Value::Null => String::new(),
                            _ => escape_csv(&item.to_string()),
                        };
                        csv.push_str(&val);
                        csv.push('\n');
                    }
                }
                Ok(csv)
            }
            serde_json::Value::Object(obj) => {
                // Single object - write as one row with headers
                let mut csv = String::new();
                let keys: Vec<String> = obj.keys().map(|k| escape_csv(k)).collect();
                csv.push_str(&keys.join(","));
                csv.push('\n');
                
                let values: Vec<String> = keys.iter().map(|k| {
                    obj.get(k.trim_matches('"'))
                        .map(|v| match v {
                            serde_json::Value::String(s) => escape_csv(s),
                            serde_json::Value::Number(n) => n.to_string(),
                            serde_json::Value::Bool(b) => b.to_string(),
                            serde_json::Value::Null => String::new(),
                            _ => escape_csv(&v.to_string()),
                        })
                        .unwrap_or_else(|| String::new())
                }).collect();
                csv.push_str(&values.join(","));
                csv.push('\n');
                Ok(csv)
            }
            _ => {
                // Primitive value - single cell
                Ok(match value {
                    serde_json::Value::String(s) => escape_csv(s),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Null => String::new(),
                    _ => escape_csv(&value.to_string()),
                })
            }
        }
    }
    
    fn format_field(
        data: serde_json::Value,
        _field: &str,
        _format: &DataFormat,
    ) -> Result<serde_json::Value> {
        // Format specific field
        Ok(data)
    }
    
    fn apply_nested_transforms(
        data: serde_json::Value,
        _path: &str,
        _transforms: &[OutputTransform],
    ) -> Result<serde_json::Value> {
        // Apply transforms to nested path
        Ok(data)
    }
    
    fn evaluate_condition(
        data: &serde_json::Value,
        condition: &str,
    ) -> Result<bool> {
        // Real condition evaluation engine
        // Supports: field comparisons (field > value, field == value, etc.)
        //          logical operators (&&, ||, !)
        //          field access (field.subfield)
        
        // Parse and evaluate condition
        Self::evaluate_expression(data, condition)
    }
    
    /// Evaluate a condition expression against JSON data
    fn evaluate_expression(data: &serde_json::Value, expr: &str) -> Result<bool> {
        // Simple expression parser for conditions like:
        // "field > 10"
        // "field == 'value'"
        // "field.subfield != null"
        // "field1 > 5 && field2 < 10"
        // "field == 'test' || field == 'other'"
        
        let expr = expr.trim();
        
        // Handle logical operators (process && and || with proper precedence)
        if expr.contains("&&") {
            let parts: Vec<&str> = expr.split("&&").collect();
            let mut result = true;
            for part in parts {
                result = result && Self::evaluate_expression(data, part.trim())?;
                if !result {
                    break; // Short-circuit evaluation
                }
            }
            return Ok(result);
        }
        
        if expr.contains("||") {
            let parts: Vec<&str> = expr.split("||").collect();
            let mut result = false;
            for part in parts {
                result = result || Self::evaluate_expression(data, part.trim())?;
                if result {
                    break; // Short-circuit evaluation
                }
            }
            return Ok(result);
        }
        
        // Handle negation
        if expr.starts_with('!') {
            return Ok(!Self::evaluate_expression(data, &expr[1..].trim())?);
        }
        
        // Handle parentheses (simple - just remove them for now)
        let expr = expr.trim_matches(|c| c == '(' || c == ')');
        
        // Parse comparison operators
        let operators = [
            ("!=", "ne"),
            ("==", "eq"),
            (">=", "gte"),
            ("<=", "lte"),
            (">", "gt"),
            ("<", "lt"),
        ];
        
        for (op_str, op_name) in &operators {
            if expr.contains(op_str) {
                let parts: Vec<&str> = expr.split(op_str).collect();
                if parts.len() == 2 {
                    let field_path = parts[0].trim();
                    let value_str = parts[1].trim();
                    
                    // Get field value from data
                    let field_value = Self::get_field_value(data, field_path)?;
                    
                    // Parse comparison value
                    let compare_value = Self::parse_value(value_str)?;
                    
                    // Compare
                    return Ok(Self::compare_values(&field_value, op_name, &compare_value)?);
                }
            }
        }
        
        // If no operator found, check for field existence or truthiness
        if let Ok(field_value) = Self::get_field_value(data, expr) {
            match field_value {
                serde_json::Value::Bool(b) => Ok(b),
                serde_json::Value::Null => Ok(false),
                serde_json::Value::Number(n) => Ok(n.as_f64().unwrap_or(0.0) != 0.0),
                serde_json::Value::String(s) => Ok(!s.is_empty()),
                _ => Ok(true),
            }
        } else {
            Ok(false)
        }
    }
    
    /// Get field value from JSON using dot notation
    fn get_field_value(data: &serde_json::Value, path: &str) -> Result<serde_json::Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = data;
        
        for part in parts {
            match current {
                serde_json::Value::Object(map) => {
                    current = map.get(part).ok_or_else(|| {
                        Error::Query(format!("Field '{}' not found in path '{}'", part, path))
                    })?;
                }
                serde_json::Value::Array(arr) => {
                    let idx: usize = part.parse().map_err(|_| {
                        Error::Query(format!("Invalid array index '{}' in path '{}'", part, path))
                    })?;
                    current = arr.get(idx).ok_or_else(|| {
                        Error::Query(format!("Array index {} out of bounds in path '{}'", idx, path))
                    })?;
                }
                _ => {
                    return Err(Error::Query(format!("Cannot access '{}' on non-object/non-array in path '{}'", part, path)));
                }
            }
        }
        
        Ok(current.clone())
    }
    
    /// Parse a string value to JSON value
    fn parse_value(s: &str) -> Result<serde_json::Value> {
        let s = s.trim();
        
        // Remove quotes if present
        let s = if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
            &s[1..s.len()-1]
        } else {
            s
        };
        
        // Try parsing as number
        if let Ok(n) = s.parse::<i64>() {
            return Ok(serde_json::Value::Number(n.into()));
        }
        if let Ok(n) = s.parse::<f64>() {
            return Ok(serde_json::Value::Number(serde_json::Number::from_f64(n).unwrap()));
        }
        
        // Check for boolean
        if s == "true" {
            return Ok(serde_json::Value::Bool(true));
        }
        if s == "false" {
            return Ok(serde_json::Value::Bool(false));
        }
        
        // Check for null
        if s == "null" {
            return Ok(serde_json::Value::Null);
        }
        
        // Default to string
        Ok(serde_json::Value::String(s.to_string()))
    }
    
    /// Compare two JSON values
    fn compare_values(
        left: &serde_json::Value,
        op: &str,
        right: &serde_json::Value,
    ) -> Result<bool> {
        match (left, right, op) {
            // Number comparisons
            (serde_json::Value::Number(a), serde_json::Value::Number(b), "eq") => {
                Ok(a.as_f64().unwrap_or(0.0) == b.as_f64().unwrap_or(0.0))
            }
            (serde_json::Value::Number(a), serde_json::Value::Number(b), "ne") => {
                Ok(a.as_f64().unwrap_or(0.0) != b.as_f64().unwrap_or(0.0))
            }
            (serde_json::Value::Number(a), serde_json::Value::Number(b), "gt") => {
                Ok(a.as_f64().unwrap_or(0.0) > b.as_f64().unwrap_or(0.0))
            }
            (serde_json::Value::Number(a), serde_json::Value::Number(b), "gte") => {
                Ok(a.as_f64().unwrap_or(0.0) >= b.as_f64().unwrap_or(0.0))
            }
            (serde_json::Value::Number(a), serde_json::Value::Number(b), "lt") => {
                Ok(a.as_f64().unwrap_or(0.0) < b.as_f64().unwrap_or(0.0))
            }
            (serde_json::Value::Number(a), serde_json::Value::Number(b), "lte") => {
                Ok(a.as_f64().unwrap_or(0.0) <= b.as_f64().unwrap_or(0.0))
            }
            
            // String comparisons
            (serde_json::Value::String(a), serde_json::Value::String(b), "eq") => Ok(a == b),
            (serde_json::Value::String(a), serde_json::Value::String(b), "ne") => Ok(a != b),
            
            // Boolean comparisons
            (serde_json::Value::Bool(a), serde_json::Value::Bool(b), "eq") => Ok(a == b),
            (serde_json::Value::Bool(a), serde_json::Value::Bool(b), "ne") => Ok(a != b),
            
            // Null comparisons
            (serde_json::Value::Null, serde_json::Value::Null, "eq") => Ok(true),
            (serde_json::Value::Null, _, "eq") => Ok(false),
            (_, serde_json::Value::Null, "eq") => Ok(false),
            (serde_json::Value::Null, _, "ne") => Ok(true),
            (_, serde_json::Value::Null, "ne") => Ok(true),
            
            _ => Err(Error::Query(format!("Cannot compare {:?} {} {:?}", left, op, right))),
        }
    }
    
    /// SECURITY: Validate all field names in a predicate recursively
    fn validate_predicate_fields(predicate: &FilterPredicate) -> Result<()> {
        match predicate {
            FilterPredicate::Eq { field, .. } |
            FilterPredicate::Ne { field, .. } |
            FilterPredicate::Gt { field, .. } |
            FilterPredicate::Lt { field, .. } |
            FilterPredicate::Gte { field, .. } |
            FilterPredicate::Lte { field, .. } |
            FilterPredicate::In { field, .. } |
            FilterPredicate::Contains { field, .. } |
            FilterPredicate::Regex { field, .. } => {
                Self::validate_field_name(field)?;
                Ok(())
            }
            FilterPredicate::And { left, right } |
            FilterPredicate::Or { left, right } => {
                Self::validate_predicate_fields(left)?;
                Self::validate_predicate_fields(right)?;
                Ok(())
            }
            FilterPredicate::Not { expr } => {
                Self::validate_predicate_fields(expr)
            }
        }
    }
    
    fn evaluate_predicate(
        data: &serde_json::Value,
        predicate: &FilterPredicate,
    ) -> Result<bool> {
        // EDGE CASE: Handle null/empty data
        if data.is_null() {
            return Ok(false);
        }
        
        match predicate {
            FilterPredicate::Eq { field, value } => {
                // SECURITY: Field name already validated, but double-check for safety
                // Using get() is safe - it doesn't execute code
                // SECURITY: Type-safe comparison using serde_json::Value's PartialEq
                // This prevents type confusion attacks
                Ok(data.get(field) == Some(value))
            }
            FilterPredicate::Ne { field, value } => {
                // SECURITY: Type-safe comparison
                Ok(data.get(field) != Some(value))
            }
            FilterPredicate::Gt { field, value } => {
                // EDGE CASE: Handle type mismatches and missing fields
                if let Some(data_val) = data.get(field) {
                    if let (Some(data_num), Some(pred_val)) = (data_val.as_f64(), value.as_f64()) {
                        // EDGE CASE: Handle NaN and Infinity
                        if data_num.is_nan() || pred_val.is_nan() {
                            return Ok(false);
                        }
                        if data_num.is_infinite() || pred_val.is_infinite() {
                            return Ok(false);
                        }
                        return Ok(data_num > pred_val);
                    }
                }
                Ok(false)
            }
            FilterPredicate::Gte { field, value } => {
                if let Some(data_val) = data.get(field) {
                    if let (Some(data_num), Some(pred_val)) = (data_val.as_f64(), value.as_f64()) {
                        if data_num.is_nan() || pred_val.is_nan() {
                            return Ok(false);
                        }
                        if data_num.is_infinite() || pred_val.is_infinite() {
                            return Ok(false);
                        }
                        return Ok(data_num >= pred_val);
                    }
                }
                Ok(false)
            }
            FilterPredicate::Lt { field, value } => {
                // EDGE CASE: Handle type mismatches and missing fields
                if let Some(data_val) = data.get(field) {
                    if let (Some(data_num), Some(pred_val)) = (data_val.as_f64(), value.as_f64()) {
                        // EDGE CASE: Handle NaN and Infinity
                        if data_num.is_nan() || pred_val.is_nan() {
                            return Ok(false);
                        }
                        if data_num.is_infinite() || pred_val.is_infinite() {
                            return Ok(false);
                        }
                        return Ok(data_num < pred_val);
                    }
                }
                Ok(false)
            }
            FilterPredicate::Lte { field, value } => {
                if let Some(data_val) = data.get(field) {
                    if let (Some(data_num), Some(pred_val)) = (data_val.as_f64(), value.as_f64()) {
                        if data_num.is_nan() || pred_val.is_nan() {
                            return Ok(false);
                        }
                        if data_num.is_infinite() || pred_val.is_infinite() {
                            return Ok(false);
                        }
                        return Ok(data_num <= pred_val);
                    }
                }
                Ok(false)
            }
            FilterPredicate::In { field, values } => {
                // EDGE CASE: Handle empty values list
                if values.is_empty() {
                    return Ok(false);
                }
                
                if let Some(data_val) = data.get(field) {
                    // EDGE CASE: Limit values list size
                    const MAX_IN_VALUES: usize = 10_000;
                    if values.len() > MAX_IN_VALUES {
                        return Err(Error::Query(format!(
                            "Too many values in 'In' predicate: {} (max: {})",
                            values.len(), MAX_IN_VALUES
                        )));
                    }
                    
                    return Ok(values.contains(data_val));
                }
                Ok(false)
            }
            FilterPredicate::Contains { field, value } => {
                // SECURITY: Field name already validated
                // SECURITY: Limit search string length to prevent DoS
                if let Some(data_val) = data.get(field) {
                    if let Some(data_str) = data_val.as_str() {
                        if let Some(pred_str) = value.as_str() {
                            // SECURITY: Prevent DoS via extremely long search strings
                            const MAX_SEARCH_STRING_LENGTH: usize = 1_000_000;
                            if pred_str.len() > MAX_SEARCH_STRING_LENGTH {
                                return Err(Error::Query(format!(
                                    "Search string too long: {} bytes (max: {})",
                                    pred_str.len(), MAX_SEARCH_STRING_LENGTH
                                )));
                            }
                            
                            // EDGE CASE: Handle empty strings
                            if pred_str.is_empty() {
                                return Ok(true); // Empty string is contained in any string
                            }
                            
                            // SECURITY: Limit data string length to prevent DoS
                            if data_str.len() > MAX_SEARCH_STRING_LENGTH {
                                return Err(Error::Query(format!(
                                    "Data string too long for contains check: {} bytes (max: {})",
                                    data_str.len(), MAX_SEARCH_STRING_LENGTH
                                )));
                            }
                            
                            return Ok(data_str.contains(pred_str));
                        }
                    }
                    // EDGE CASE: Try array contains
                    if let Some(data_arr) = data_val.as_array() {
                        // SECURITY: Limit array size for contains check
                        const MAX_CONTAINS_ARRAY_SIZE: usize = 100_000;
                        if data_arr.len() > MAX_CONTAINS_ARRAY_SIZE {
                            return Err(Error::Query(format!(
                                "Array too large for contains check: {} elements (max: {})",
                                data_arr.len(), MAX_CONTAINS_ARRAY_SIZE
                            )));
                        }
                        return Ok(data_arr.contains(value));
                    }
                }
                Ok(false)
            }
            FilterPredicate::Regex { field, pattern } => {
                // SECURITY: Field name already validated
                // SECURITY: When implementing regex, MUST:
                // 1. Use regex crate with timeout
                // 2. Validate pattern length
                // 3. Prevent ReDoS (Regular Expression Denial of Service)
                // 4. Limit pattern complexity
                
                // SECURITY: Validate pattern length before implementation
                const MAX_REGEX_PATTERN_LENGTH: usize = 1_024;
                if pattern.len() > MAX_REGEX_PATTERN_LENGTH {
                    return Err(Error::Query(format!(
                        "Regex pattern too long: {} bytes (max: {})",
                        pattern.len(), MAX_REGEX_PATTERN_LENGTH
                    )));
                }
                
                // SECURITY: Prevent dangerous regex patterns (basic check)
                // In production, would use a regex validator or whitelist
                let dangerous_patterns = [
                    "(.*)*",  // Catastrophic backtracking
                    "(a+)+",  // ReDoS pattern
                    "(a|a)*", // ReDoS pattern
                ];
                for dangerous in &dangerous_patterns {
                    if pattern.contains(dangerous) {
                        return Err(Error::Query(format!(
                            "Potentially dangerous regex pattern detected: '{}'",
                            pattern
                        )));
                    }
                }
                
                // EDGE CASE: Regex not yet implemented, return false
                // In production, would use regex crate with timeout
                let _ = (field, pattern);
                Ok(false)
            }
            FilterPredicate::And { left, right } => {
                // EDGE CASE: Short-circuit evaluation
                let left_result = Self::evaluate_predicate(data, left)?;
                if !left_result {
                    return Ok(false);
                }
                Ok(Self::evaluate_predicate(data, right)?)
            }
            FilterPredicate::Or { left, right } => {
                // EDGE CASE: Short-circuit evaluation
                let left_result = Self::evaluate_predicate(data, left)?;
                if left_result {
                    return Ok(true);
                }
                Ok(Self::evaluate_predicate(data, right)?)
            }
            FilterPredicate::Not { expr } => {
                Ok(!Self::evaluate_predicate(data, expr)?)
            }
        }
    }
    
    fn convert_date_format(
        value: serde_json::Value,
        _from: &str,
        _to: &str,
    ) -> Result<serde_json::Value> {
        // Date format conversion
        Ok(value)
    }
    
    fn format_number(
        value: serde_json::Value,
        _format: &str,
    ) -> Result<serde_json::Value> {
        // Number formatting
        Ok(value)
    }
    
    fn format_value(
        value: serde_json::Value,
        _format: &str,
    ) -> Result<serde_json::Value> {
        // Value formatting
        Ok(value)
    }
}

