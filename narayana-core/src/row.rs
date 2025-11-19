use crate::schema::DataType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    Float32(f32),
    Float64(f64),
    Boolean(bool),
    String(String),
    Binary(Vec<u8>),
    Timestamp(i64),
    Date(i32),
    Null,
    Array(Vec<Value>),
}

impl Value {
    pub fn data_type(&self) -> DataType {
        match self {
            Value::Int8(_) => DataType::Int8,
            Value::Int16(_) => DataType::Int16,
            Value::Int32(_) => DataType::Int32,
            Value::Int64(_) => DataType::Int64,
            Value::UInt8(_) => DataType::UInt8,
            Value::UInt16(_) => DataType::UInt16,
            Value::UInt32(_) => DataType::UInt32,
            Value::UInt64(_) => DataType::UInt64,
            Value::Float32(_) => DataType::Float32,
            Value::Float64(_) => DataType::Float64,
            Value::Boolean(_) => DataType::Boolean,
            Value::String(_) => DataType::String,
            Value::Binary(_) => DataType::Binary,
            Value::Timestamp(_) => DataType::Timestamp,
            Value::Date(_) => DataType::Date,
            Value::Null => DataType::Nullable(Box::new(DataType::Int32)),
            Value::Array(_) => DataType::Array(Box::new(DataType::Int32)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    pub values: Vec<Value>,
}

impl Row {
    pub fn new(values: Vec<Value>) -> Self {
        Self { values }
    }

    pub fn get(&self, index: usize) -> Option<&Value> {
        self.values.get(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_row_creation() {
        let row = Row::new(vec![
            Value::Int64(42),
            Value::String("test".to_string()),
        ]);
        assert_eq!(row.values.len(), 2);
    }

    #[test]
    fn test_row_get() {
        let row = Row::new(vec![
            Value::Int64(42),
            Value::String("test".to_string()),
        ]);
        assert!(matches!(row.get(0), Some(Value::Int64(42))));
        assert!(matches!(row.get(1), Some(Value::String(_))));
        assert!(row.get(2).is_none());
    }

    #[test]
    fn test_value_data_type() {
        assert_eq!(Value::Int32(42).data_type(), DataType::Int32);
        assert_eq!(Value::String("test".to_string()).data_type(), DataType::String);
        assert_eq!(Value::Boolean(true).data_type(), DataType::Boolean);
    }
}
