use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    Int8,
    Int16,
    Int32,
    Int64,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float32,
    Float64,
    Boolean,
    String,
    Binary,
    Timestamp,
    Date,
    Json, // JSON data type for semi-structured data
    Nullable(Box<DataType>),
    Array(Box<DataType>),
    Map(Box<DataType>, Box<DataType>),
}

impl DataType {
    pub fn size(&self) -> Option<usize> {
        match self {
            DataType::Int8 | DataType::UInt8 | DataType::Boolean => Some(1),
            DataType::Int16 | DataType::UInt16 => Some(2),
            DataType::Int32 | DataType::UInt32 | DataType::Float32 => Some(4),
            DataType::Int64 | DataType::UInt64 | DataType::Float64 | DataType::Timestamp | DataType::Date => Some(8),
            DataType::String | DataType::Binary | DataType::Json | DataType::Nullable(_) | DataType::Array(_) | DataType::Map(_, _) => None,
        }
    }

    pub fn is_fixed_size(&self) -> bool {
        self.size().is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub default_value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Schema {
    pub fields: Vec<Field>,
    pub field_map: HashMap<String, usize>,
}

impl<'de> Deserialize<'de> for Schema {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct SchemaHelper {
            fields: Vec<Field>,
            #[serde(default)]
            field_map: Option<HashMap<String, usize>>,
        }
        
        let helper = SchemaHelper::deserialize(deserializer)?;
        
        // If field_map is provided, use it; otherwise generate from fields
        let field_map = if let Some(map) = helper.field_map {
            map
        } else {
            helper.fields
                .iter()
                .enumerate()
                .map(|(idx, field)| (field.name.clone(), idx))
                .collect()
        };
        
        Ok(Schema {
            fields: helper.fields,
            field_map,
        })
    }
}

impl Schema {
    pub fn new(fields: Vec<Field>) -> Self {
        let field_map: HashMap<String, usize> = fields
            .iter()
            .enumerate()
            .map(|(idx, field)| (field.name.clone(), idx))
            .collect();

        Self { fields, field_map }
    }

    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.field_map.get(name).copied()
    }

    pub fn field(&self, name: &str) -> Option<&Field> {
        self.field_index(name).map(|idx| &self.fields[idx])
    }

    pub fn len(&self) -> usize {
        self.fields.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_type_size() {
        assert_eq!(DataType::Int8.size(), Some(1));
        assert_eq!(DataType::Int32.size(), Some(4));
        assert_eq!(DataType::Int64.size(), Some(8));
        assert_eq!(DataType::String.size(), None);
    }

    #[test]
    fn test_data_type_is_fixed_size() {
        assert!(DataType::Int32.is_fixed_size());
        assert!(!DataType::String.is_fixed_size());
        assert!(!DataType::Nullable(Box::new(DataType::Int32)).is_fixed_size());
    }

    #[test]
    fn test_schema_creation() {
        let fields = vec![
            Field {
                name: "id".to_string(),
                data_type: DataType::Int64,
                nullable: false,
                default_value: None,
            },
            Field {
                name: "name".to_string(),
                data_type: DataType::String,
                nullable: false,
                default_value: None,
            },
        ];

        let schema = Schema::new(fields);
        assert_eq!(schema.len(), 2);
        assert_eq!(schema.field_index("id"), Some(0));
        assert_eq!(schema.field_index("name"), Some(1));
        assert_eq!(schema.field_index("nonexistent"), None);
    }

    #[test]
    fn test_schema_field_access() {
        let schema = Schema::new(vec![
            Field {
                name: "id".to_string(),
                data_type: DataType::Int64,
                nullable: false,
                default_value: None,
            },
        ]);

        assert!(schema.field("id").is_some());
        assert_eq!(schema.field("id").unwrap().name, "id");
        assert!(schema.field("nonexistent").is_none());
    }
}
