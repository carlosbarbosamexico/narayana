// Tests for type conversions and compatibility

use narayana_core::{
    schema::DataType,
    column::Column,
    row::Value,
};

#[test]
fn test_value_to_column_conversion() {
    let values = vec![
        Value::Int32(1),
        Value::Int32(2),
        Value::Int32(3),
    ];
    
    // Convert values to column
    let column_data: Vec<i32> = values.iter()
        .filter_map(|v| {
            if let Value::Int32(x) = v {
                Some(*x)
            } else {
                None
            }
        })
        .collect();
    
    let column = Column::Int32(column_data);
    assert_eq!(column.len(), 3);
}

#[test]
fn test_column_to_value_conversion() {
    let column = Column::Int32(vec![1, 2, 3]);
    
    match column {
        Column::Int32(data) => {
            let values: Vec<Value> = data.iter().map(|&x| Value::Int32(x)).collect();
            assert_eq!(values.len(), 3);
        }
        _ => panic!("Expected Int32"),
    }
}

#[test]
fn test_data_type_compatibility() {
    // Test that compatible types can be compared
    assert_eq!(DataType::Int32.size(), Some(4));
    assert_eq!(DataType::UInt32.size(), Some(4));
    // Same size but different types
}

#[test]
fn test_type_promotion() {
    // Test implicit type promotion scenarios
    let int_col = Column::Int32(vec![1, 2, 3]);
    let long_col = Column::Int64(vec![1, 2, 3]);
    
    // These are different types, no automatic promotion
    assert_eq!(int_col.data_type(), DataType::Int32);
    assert_eq!(long_col.data_type(), DataType::Int64);
}

