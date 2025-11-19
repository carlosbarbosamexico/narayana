// Comprehensive tests for timestamp and date handling

use narayana_core::{
    column::Column,
    types::Timestamp,
    schema::DataType,
};

#[test]
fn test_timestamp_column_operations() {
    let column = Column::Timestamp(vec![1000, 2000, 3000]);
    assert_eq!(column.len(), 3);
    assert_eq!(column.data_type(), DataType::Timestamp);
}

#[test]
fn test_timestamp_boundary_values() {
    let column = Column::Timestamp(vec![
        0,
        i64::MAX,
        i64::MIN,
    ]);
    
    assert_eq!(column.len(), 3);
}

#[test]
fn test_timestamp_ordering() {
    let timestamps = vec![
        Timestamp(1000),
        Timestamp(2000),
        Timestamp(3000),
    ];
    
    assert!(timestamps[1] > timestamps[0]);
    assert!(timestamps[2] > timestamps[1]);
    assert_eq!(timestamps[0], Timestamp(1000));
}

#[test]
fn test_timestamp_creation() {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let ts1 = Timestamp(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );
    std::thread::sleep(std::time::Duration::from_millis(10));
    let ts2 = Timestamp(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );
    
    assert!(ts2 >= ts1);
}

#[test]
fn test_date_column_operations() {
    let column = Column::Date(vec![1, 2, 3]);
    assert_eq!(column.len(), 3);
    assert_eq!(column.data_type(), DataType::Date);
}

#[test]
fn test_date_boundary_values() {
    let column = Column::Date(vec![
        i32::MIN,
        0,
        i32::MAX,
    ]);
    
    assert_eq!(column.len(), 3);
}

#[test]
fn test_timestamp_date_conversion() {
    // Test that timestamps and dates are handled separately
    let timestamp_col = Column::Timestamp(vec![1000]);
    let date_col = Column::Date(vec![1]);
    
    assert_eq!(timestamp_col.data_type(), DataType::Timestamp);
    assert_eq!(date_col.data_type(), DataType::Date);
}

#[test]
fn test_timestamp_epoch_values() {
    // Test common epoch values
    let column = Column::Timestamp(vec![
        0, // Unix epoch
        946684800, // 2000-01-01
        1609459200, // 2021-01-01
    ]);
    
    assert_eq!(column.len(), 3);
}

#[test]
fn test_timestamp_negative_values() {
    // Negative timestamps (before epoch)
    let column = Column::Timestamp(vec![
        -1000,
        -946684800, // Before 1970
    ]);
    
    assert_eq!(column.len(), 2);
}

#[test]
fn test_date_negative_values() {
    // Negative dates
    let column = Column::Date(vec![
        -1,
        -365,
    ]);
    
    assert_eq!(column.len(), 2);
}

#[test]
fn test_timestamp_large_values() {
    // Very large timestamp values (far future)
    let column = Column::Timestamp(vec![
        i64::MAX,
        9999999999, // Year 2286
    ]);
    
    assert_eq!(column.len(), 2);
}

