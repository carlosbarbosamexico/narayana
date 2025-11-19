use crate::schema::DataType;
use serde::{Deserialize, Serialize};

/// Columnar data representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Column {
    Int8(Vec<i8>),
    Int16(Vec<i16>),
    Int32(Vec<i32>),
    Int64(Vec<i64>),
    UInt8(Vec<u8>),
    UInt16(Vec<u16>),
    UInt32(Vec<u32>),
    UInt64(Vec<u64>),
    Float32(Vec<f32>),
    Float64(Vec<f64>),
    Boolean(Vec<bool>),
    String(Vec<String>),
    Binary(Vec<Vec<u8>>),
    Timestamp(Vec<i64>),
    Date(Vec<i32>),
}

impl Column {
    pub fn len(&self) -> usize {
        match self {
            Column::Int8(v) => v.len(),
            Column::Int16(v) => v.len(),
            Column::Int32(v) => v.len(),
            Column::Int64(v) => v.len(),
            Column::UInt8(v) => v.len(),
            Column::UInt16(v) => v.len(),
            Column::UInt32(v) => v.len(),
            Column::UInt64(v) => v.len(),
            Column::Float32(v) => v.len(),
            Column::Float64(v) => v.len(),
            Column::Boolean(v) => v.len(),
            Column::String(v) => v.len(),
            Column::Binary(v) => v.len(),
            Column::Timestamp(v) => v.len(),
            Column::Date(v) => v.len(),
        }
    }

    pub fn data_type(&self) -> DataType {
        match self {
            Column::Int8(_) => DataType::Int8,
            Column::Int16(_) => DataType::Int16,
            Column::Int32(_) => DataType::Int32,
            Column::Int64(_) => DataType::Int64,
            Column::UInt8(_) => DataType::UInt8,
            Column::UInt16(_) => DataType::UInt16,
            Column::UInt32(_) => DataType::UInt32,
            Column::UInt64(_) => DataType::UInt64,
            Column::Float32(_) => DataType::Float32,
            Column::Float64(_) => DataType::Float64,
            Column::Boolean(_) => DataType::Boolean,
            Column::String(_) => DataType::String,
            Column::Binary(_) => DataType::Binary,
            Column::Timestamp(_) => DataType::Timestamp,
            Column::Date(_) => DataType::Date,
        }
    }

    /// Append another column to this one (must be same type)
    pub fn append(&self, other: &Column) -> crate::Result<Column> {
        match (self, other) {
            (Column::Int8(a), Column::Int8(b)) => {
                let mut result = a.clone();
                result.extend_from_slice(b);
                Ok(Column::Int8(result))
            }
            (Column::Int16(a), Column::Int16(b)) => {
                let mut result = a.clone();
                result.extend_from_slice(b);
                Ok(Column::Int16(result))
            }
            (Column::Int32(a), Column::Int32(b)) => {
                let mut result = a.clone();
                result.extend_from_slice(b);
                Ok(Column::Int32(result))
            }
            (Column::Int64(a), Column::Int64(b)) => {
                let mut result = a.clone();
                result.extend_from_slice(b);
                Ok(Column::Int64(result))
            }
            (Column::UInt8(a), Column::UInt8(b)) => {
                let mut result = a.clone();
                result.extend_from_slice(b);
                Ok(Column::UInt8(result))
            }
            (Column::UInt16(a), Column::UInt16(b)) => {
                let mut result = a.clone();
                result.extend_from_slice(b);
                Ok(Column::UInt16(result))
            }
            (Column::UInt32(a), Column::UInt32(b)) => {
                let mut result = a.clone();
                result.extend_from_slice(b);
                Ok(Column::UInt32(result))
            }
            (Column::UInt64(a), Column::UInt64(b)) => {
                let mut result = a.clone();
                result.extend_from_slice(b);
                Ok(Column::UInt64(result))
            }
            (Column::Float32(a), Column::Float32(b)) => {
                let mut result = a.clone();
                result.extend_from_slice(b);
                Ok(Column::Float32(result))
            }
            (Column::Float64(a), Column::Float64(b)) => {
                let mut result = a.clone();
                result.extend_from_slice(b);
                Ok(Column::Float64(result))
            }
            (Column::Boolean(a), Column::Boolean(b)) => {
                let mut result = a.clone();
                result.extend_from_slice(b);
                Ok(Column::Boolean(result))
            }
            (Column::String(a), Column::String(b)) => {
                let mut result = a.clone();
                result.extend_from_slice(b);
                Ok(Column::String(result))
            }
            (Column::Binary(a), Column::Binary(b)) => {
                let mut result = a.clone();
                result.extend_from_slice(b);
                Ok(Column::Binary(result))
            }
            (Column::Timestamp(a), Column::Timestamp(b)) => {
                let mut result = a.clone();
                result.extend_from_slice(b);
                Ok(Column::Timestamp(result))
            }
            (Column::Date(a), Column::Date(b)) => {
                let mut result = a.clone();
                result.extend_from_slice(b);
                Ok(Column::Date(result))
            }
            _ => Err(crate::Error::Storage("Column type mismatch".to_string())),
        }
    }

    /// Slice column to get a range of rows
    pub fn slice(&self, start: usize, count: usize) -> crate::Result<Column> {
        let end = start + count;
        match self {
            Column::Int8(v) => {
                if end > v.len() {
                    return Err(crate::Error::Storage("Slice out of bounds".to_string()));
                }
                Ok(Column::Int8(v[start..end].to_vec()))
            }
            Column::Int16(v) => {
                if end > v.len() {
                    return Err(crate::Error::Storage("Slice out of bounds".to_string()));
                }
                Ok(Column::Int16(v[start..end].to_vec()))
            }
            Column::Int32(v) => {
                if end > v.len() {
                    return Err(crate::Error::Storage("Slice out of bounds".to_string()));
                }
                Ok(Column::Int32(v[start..end].to_vec()))
            }
            Column::Int64(v) => {
                if end > v.len() {
                    return Err(crate::Error::Storage("Slice out of bounds".to_string()));
                }
                Ok(Column::Int64(v[start..end].to_vec()))
            }
            Column::UInt8(v) => {
                if end > v.len() {
                    return Err(crate::Error::Storage("Slice out of bounds".to_string()));
                }
                Ok(Column::UInt8(v[start..end].to_vec()))
            }
            Column::UInt16(v) => {
                if end > v.len() {
                    return Err(crate::Error::Storage("Slice out of bounds".to_string()));
                }
                Ok(Column::UInt16(v[start..end].to_vec()))
            }
            Column::UInt32(v) => {
                if end > v.len() {
                    return Err(crate::Error::Storage("Slice out of bounds".to_string()));
                }
                Ok(Column::UInt32(v[start..end].to_vec()))
            }
            Column::UInt64(v) => {
                if end > v.len() {
                    return Err(crate::Error::Storage("Slice out of bounds".to_string()));
                }
                Ok(Column::UInt64(v[start..end].to_vec()))
            }
            Column::Float32(v) => {
                if end > v.len() {
                    return Err(crate::Error::Storage("Slice out of bounds".to_string()));
                }
                Ok(Column::Float32(v[start..end].to_vec()))
            }
            Column::Float64(v) => {
                if end > v.len() {
                    return Err(crate::Error::Storage("Slice out of bounds".to_string()));
                }
                Ok(Column::Float64(v[start..end].to_vec()))
            }
            Column::Boolean(v) => {
                if end > v.len() {
                    return Err(crate::Error::Storage("Slice out of bounds".to_string()));
                }
                Ok(Column::Boolean(v[start..end].to_vec()))
            }
            Column::String(v) => {
                if end > v.len() {
                    return Err(crate::Error::Storage("Slice out of bounds".to_string()));
                }
                Ok(Column::String(v[start..end].to_vec()))
            }
            Column::Binary(v) => {
                if end > v.len() {
                    return Err(crate::Error::Storage("Slice out of bounds".to_string()));
                }
                Ok(Column::Binary(v[start..end].to_vec()))
            }
            Column::Timestamp(v) => {
                if end > v.len() {
                    return Err(crate::Error::Storage("Slice out of bounds".to_string()));
                }
                Ok(Column::Timestamp(v[start..end].to_vec()))
            }
            Column::Date(v) => {
                if end > v.len() {
                    return Err(crate::Error::Storage("Slice out of bounds".to_string()));
                }
                Ok(Column::Date(v[start..end].to_vec()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_len() {
        let col = Column::Int32(vec![1, 2, 3, 4, 5]);
        assert_eq!(col.len(), 5);
        
        let col = Column::String(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(col.len(), 2);
        
        let col = Column::Boolean(vec![true, false, true]);
        assert_eq!(col.len(), 3);
    }

    #[test]
    fn test_column_data_type() {
        assert_eq!(Column::Int32(vec![]).data_type(), DataType::Int32);
        assert_eq!(Column::Int64(vec![]).data_type(), DataType::Int64);
        assert_eq!(Column::String(vec![]).data_type(), DataType::String);
        assert_eq!(Column::Float64(vec![]).data_type(), DataType::Float64);
        assert_eq!(Column::Boolean(vec![]).data_type(), DataType::Boolean);
    }

    #[test]
    fn test_column_empty() {
        let col = Column::Int32(vec![]);
        assert_eq!(col.len(), 0);
    }
}
