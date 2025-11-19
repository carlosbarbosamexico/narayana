// Tests for true columnar format

use narayana_storage::columnar_format::*;
use narayana_core::schema::DataType;

#[test]
fn test_write_uint8() {
    let data = vec![1u8, 2, 3, 4, 5];
    let bytes = ColumnarFormat::write_uint8(&data);
    assert_eq!(bytes.len(), 5);
    assert_eq!(bytes[0], 1);
    assert_eq!(bytes[4], 5);
}

#[test]
fn test_read_uint8() {
    let data = vec![1u8, 2, 3, 4, 5];
    let bytes = ColumnarFormat::write_uint8(&data);
    let read = ColumnarFormat::read_uint8(&bytes);
    assert_eq!(read, data);
}

#[test]
fn test_write_int32() {
    let data = vec![1i32, 2, 3, 4, 5];
    let bytes = ColumnarFormat::write_int32(&data);
    assert_eq!(bytes.len(), 20); // 5 * 4 bytes
}

#[test]
fn test_read_int32() {
    let data = vec![1i32, 2, 3, 4, 5];
    let bytes = ColumnarFormat::write_int32(&data);
    let read = ColumnarFormat::read_int32(&bytes).unwrap();
    assert_eq!(read, data);
}

#[test]
fn test_write_int64() {
    let data = vec![1i64, 2, 3];
    let bytes = ColumnarFormat::write_int64(&data);
    assert_eq!(bytes.len(), 24); // 3 * 8 bytes
}

#[test]
fn test_read_int64() {
    let data = vec![1i64, 2, 3];
    let bytes = ColumnarFormat::write_int64(&data);
    let read = ColumnarFormat::read_int64(&bytes).unwrap();
    assert_eq!(read, data);
}

#[test]
fn test_write_float64() {
    let data = vec![1.0f64, 2.0, 3.0];
    let bytes = ColumnarFormat::write_float64(&data);
    assert_eq!(bytes.len(), 24); // 3 * 8 bytes
}

#[test]
fn test_read_float64() {
    let data = vec![1.0f64, 2.0, 3.0];
    let bytes = ColumnarFormat::write_float64(&data);
    let read = ColumnarFormat::read_float64(&bytes).unwrap();
    assert_eq!(read, data);
}

#[test]
fn test_write_boolean() {
    let data = vec![true, false, true, false, true, false, true, false, true];
    let bytes = ColumnarFormat::write_boolean(&data);
    assert_eq!(bytes.len(), 2); // 9 booleans = 2 bytes (8 + 1)
}

#[test]
fn test_read_boolean() {
    let data = vec![true, false, true, false, true];
    let bytes = ColumnarFormat::write_boolean(&data);
    let read = ColumnarFormat::read_boolean(&bytes, 5);
    assert_eq!(read, data);
}

#[test]
fn test_write_strings() {
    let data = vec!["hello".to_string(), "world".to_string()];
    let (offsets, strings) = ColumnarFormat::write_strings(&data).unwrap();
    assert_eq!(offsets.len(), 12); // 3 * 4 bytes (0, 5, 10)
    assert_eq!(strings.len(), 10); // "helloworld"
}

#[test]
fn test_read_strings() {
    let data = vec!["hello".to_string(), "world".to_string()];
    let (offsets, strings) = ColumnarFormat::write_strings(&data).unwrap();
    let read = ColumnarFormat::read_strings(&offsets, &strings).unwrap();
    assert_eq!(read, data);
}

#[test]
fn test_calculate_size_uint8() {
    let size = ColumnarSizeCalculator::calculate_size(DataType::UInt8, 1_000_000_000);
    assert_eq!(size, 1_000_000_000); // Exactly 1GB
}

#[test]
fn test_calculate_size_int32() {
    let size = ColumnarSizeCalculator::calculate_size(DataType::Int32, 1000);
    assert_eq!(size, 4000); // 1000 * 4 bytes
}

#[test]
fn test_calculate_size_int64() {
    let size = ColumnarSizeCalculator::calculate_size(DataType::Int64, 1000);
    assert_eq!(size, 8000); // 1000 * 8 bytes
}

#[test]
fn test_calculate_size_boolean() {
    let size = ColumnarSizeCalculator::calculate_size(DataType::Boolean, 1000);
    assert_eq!(size, 125); // (1000 + 7) / 8
}

#[test]
fn test_verify_size() {
    let data = vec![1u8, 2, 3, 4, 5];
    let bytes = ColumnarFormat::write_uint8(&data);
    assert!(ColumnarSizeCalculator::verify_size(DataType::UInt8, &bytes, 5));
}

