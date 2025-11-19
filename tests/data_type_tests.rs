use narayana_core::{column::Column, schema::DataType};
use narayana_query::vectorized::VectorizedOps;
use narayana_storage::{ColumnStore, InMemoryColumnStore};
use narayana_core::{schema::{Schema, Field}, types::TableId};

#[tokio::test]
async fn test_int8_operations() {
    let column = Column::Int8(vec![1, 2, 3, 4, 5]);
    assert_eq!(column.len(), 5);
    assert_eq!(column.data_type(), DataType::Int8);
}

#[tokio::test]
async fn test_int16_operations() {
    let column = Column::Int16(vec![1, 2, 3, 4, 5]);
    assert_eq!(column.len(), 5);
    assert_eq!(column.data_type(), DataType::Int16);
}

#[tokio::test]
async fn test_int32_operations() {
    let column = Column::Int32(vec![1, 2, 3, 4, 5]);
    assert_eq!(column.len(), 5);
    assert_eq!(column.data_type(), DataType::Int32);
    
    let value = serde_json::Value::Number(3.into());
    let mask = VectorizedOps::compare_eq(&column, &value);
    assert_eq!(mask, vec![false, false, true, false, false]);
}

#[tokio::test]
async fn test_int64_operations() {
    let column = Column::Int64(vec![1, 2, 3, 4, 5]);
    assert_eq!(column.len(), 5);
    assert_eq!(column.data_type(), DataType::Int64);
    
    let sum = VectorizedOps::sum(&column);
    assert_eq!(sum, Some(serde_json::Value::Number(15.into())));
}

#[tokio::test]
async fn test_uint8_operations() {
    let column = Column::UInt8(vec![1, 2, 3, 4, 5]);
    assert_eq!(column.len(), 5);
    assert_eq!(column.data_type(), DataType::UInt8);
}

#[tokio::test]
async fn test_uint16_operations() {
    let column = Column::UInt16(vec![1, 2, 3, 4, 5]);
    assert_eq!(column.len(), 5);
    assert_eq!(column.data_type(), DataType::UInt16);
}

#[tokio::test]
async fn test_uint32_operations() {
    let column = Column::UInt32(vec![1, 2, 3, 4, 5]);
    assert_eq!(column.len(), 5);
    assert_eq!(column.data_type(), DataType::UInt32);
}

#[tokio::test]
async fn test_uint64_operations() {
    let column = Column::UInt64(vec![1, 2, 3, 4, 5]);
    assert_eq!(column.len(), 5);
    assert_eq!(column.data_type(), DataType::UInt64);
    
    let value = serde_json::Value::Number(3.into());
    let mask = VectorizedOps::compare_eq(&column, &value);
    assert_eq!(mask, vec![false, false, true, false, false]);
}

#[tokio::test]
async fn test_float32_operations() {
    let column = Column::Float32(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    assert_eq!(column.len(), 5);
    assert_eq!(column.data_type(), DataType::Float32);
}

#[tokio::test]
async fn test_float64_operations() {
    let column = Column::Float64(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    assert_eq!(column.len(), 5);
    assert_eq!(column.data_type(), DataType::Float64);
    
    let value = serde_json::Value::Number(serde_json::Number::from_f64(3.0).unwrap());
    let mask = VectorizedOps::compare_eq(&column, &value);
    assert_eq!(mask[2], true);
}

#[tokio::test]
async fn test_boolean_operations() {
    let column = Column::Boolean(vec![true, false, true, false]);
    assert_eq!(column.len(), 4);
    assert_eq!(column.data_type(), DataType::Boolean);
    
    let value = serde_json::Value::Bool(true);
    let mask = VectorizedOps::compare_eq(&column, &value);
    assert_eq!(mask, vec![true, false, true, false]);
}

#[tokio::test]
async fn test_string_operations() {
    let column = Column::String(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    assert_eq!(column.len(), 3);
    assert_eq!(column.data_type(), DataType::String);
    
    let value = serde_json::Value::String("b".to_string());
    let mask = VectorizedOps::compare_eq(&column, &value);
    assert_eq!(mask, vec![false, true, false]);
}

#[tokio::test]
async fn test_binary_operations() {
    let column = Column::Binary(vec![vec![1, 2, 3], vec![4, 5, 6]]);
    assert_eq!(column.len(), 2);
    assert_eq!(column.data_type(), DataType::Binary);
}

#[tokio::test]
async fn test_timestamp_operations() {
    let column = Column::Timestamp(vec![1000, 2000, 3000]);
    assert_eq!(column.len(), 3);
    assert_eq!(column.data_type(), DataType::Timestamp);
}

#[tokio::test]
async fn test_date_operations() {
    let column = Column::Date(vec![1, 2, 3]);
    assert_eq!(column.len(), 3);
    assert_eq!(column.data_type(), DataType::Date);
}

#[tokio::test]
async fn test_all_types_in_schema() {
    let schema = Schema::new(vec![
        Field { name: "int8".to_string(), data_type: DataType::Int8, nullable: false, default_value: None },
        Field { name: "int16".to_string(), data_type: DataType::Int16, nullable: false, default_value: None },
        Field { name: "int32".to_string(), data_type: DataType::Int32, nullable: false, default_value: None },
        Field { name: "int64".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
        Field { name: "uint8".to_string(), data_type: DataType::UInt8, nullable: false, default_value: None },
        Field { name: "uint16".to_string(), data_type: DataType::UInt16, nullable: false, default_value: None },
        Field { name: "uint32".to_string(), data_type: DataType::UInt32, nullable: false, default_value: None },
        Field { name: "uint64".to_string(), data_type: DataType::UInt64, nullable: false, default_value: None },
        Field { name: "float32".to_string(), data_type: DataType::Float32, nullable: false, default_value: None },
        Field { name: "float64".to_string(), data_type: DataType::Float64, nullable: false, default_value: None },
        Field { name: "boolean".to_string(), data_type: DataType::Boolean, nullable: false, default_value: None },
        Field { name: "string".to_string(), data_type: DataType::String, nullable: false, default_value: None },
        Field { name: "binary".to_string(), data_type: DataType::Binary, nullable: false, default_value: None },
        Field { name: "timestamp".to_string(), data_type: DataType::Timestamp, nullable: false, default_value: None },
        Field { name: "date".to_string(), data_type: DataType::Date, nullable: false, default_value: None },
    ]);
    
    assert_eq!(schema.len(), 15);
    assert_eq!(schema.field_index("int8"), Some(0));
    assert_eq!(schema.field_index("date"), Some(14));
}

#[tokio::test]
async fn test_nullable_type() {
    let nullable_int = DataType::Nullable(Box::new(DataType::Int32));
    assert!(!nullable_int.is_fixed_size());
    assert_eq!(nullable_int.size(), None);
}

#[tokio::test]
async fn test_array_type() {
    let array_int = DataType::Array(Box::new(DataType::Int32));
    assert!(!array_int.is_fixed_size());
    assert_eq!(array_int.size(), None);
}

#[tokio::test]
async fn test_map_type() {
    let map_type = DataType::Map(Box::new(DataType::String), Box::new(DataType::Int32));
    assert!(!map_type.is_fixed_size());
    assert_eq!(map_type.size(), None);
}

#[tokio::test]
async fn test_nested_types() {
    let nested = DataType::Nullable(Box::new(DataType::Array(Box::new(DataType::Int32))));
    assert!(!nested.is_fixed_size());
}

#[tokio::test]
async fn test_type_size_calculation() {
    assert_eq!(DataType::Int8.size(), Some(1));
    assert_eq!(DataType::Int16.size(), Some(2));
    assert_eq!(DataType::Int32.size(), Some(4));
    assert_eq!(DataType::Int64.size(), Some(8));
    assert_eq!(DataType::UInt8.size(), Some(1));
    assert_eq!(DataType::UInt16.size(), Some(2));
    assert_eq!(DataType::UInt32.size(), Some(4));
    assert_eq!(DataType::UInt64.size(), Some(8));
    assert_eq!(DataType::Float32.size(), Some(4));
    assert_eq!(DataType::Float64.size(), Some(8));
    assert_eq!(DataType::Boolean.size(), Some(1));
    assert_eq!(DataType::Timestamp.size(), Some(8));
    assert_eq!(DataType::Date.size(), Some(8));
}

#[tokio::test]
async fn test_store_all_types() {
    let store = InMemoryColumnStore::new();
    let table_id = TableId(1);
    
    let schema = Schema::new(vec![
        Field { name: "int32".to_string(), data_type: DataType::Int32, nullable: false, default_value: None },
        Field { name: "int64".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
        Field { name: "float64".to_string(), data_type: DataType::Float64, nullable: false, default_value: None },
        Field { name: "string".to_string(), data_type: DataType::String, nullable: false, default_value: None },
        Field { name: "boolean".to_string(), data_type: DataType::Boolean, nullable: false, default_value: None },
    ]);
    
    store.create_table(table_id, schema).await.unwrap();
    
    let columns = vec![
        Column::Int32(vec![1, 2, 3]),
        Column::Int64(vec![4, 5, 6]),
        Column::Float64(vec![1.1, 2.2, 3.3]),
        Column::String(vec!["a".to_string(), "b".to_string(), "c".to_string()]),
        Column::Boolean(vec![true, false, true]),
    ];
    
    store.write_columns(table_id, columns).await.unwrap();
    let read = store.read_columns(table_id, vec![0, 1, 2, 3, 4], 0, 3).await.unwrap();
    assert_eq!(read.len(), 5);
}

