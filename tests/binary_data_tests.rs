// Comprehensive tests for binary data handling

use narayana_core::column::Column;

#[test]
fn test_binary_empty() {
    let column = Column::Binary(vec![]);
    assert_eq!(column.len(), 0);
}

#[test]
fn test_binary_single_byte() {
    let column = Column::Binary(vec![vec![42]]);
    assert_eq!(column.len(), 1);
}

#[test]
fn test_binary_null_bytes() {
    let column = Column::Binary(vec![
        vec![0, 0, 0],
        vec![1, 2, 3],
    ]);
    assert_eq!(column.len(), 2);
}

#[test]
fn test_binary_all_bytes() {
    // Test all possible byte values
    let all_bytes: Vec<u8> = (0..=255).collect();
    let column = Column::Binary(vec![all_bytes]);
    assert_eq!(column.len(), 1);
}

#[test]
fn test_binary_very_large() {
    let large_binary = vec![42u8; 1_000_000];
    let column = Column::Binary(vec![large_binary]);
    assert_eq!(column.len(), 1);
}

#[test]
fn test_binary_many_entries() {
    let binaries: Vec<Vec<u8>> = (0..1000).map(|i| vec![i as u8; 100]).collect();
    let column = Column::Binary(binaries);
    assert_eq!(column.len(), 1000);
}

#[test]
fn test_binary_variable_lengths() {
    let column = Column::Binary(vec![
        vec![1],
        vec![1, 2],
        vec![1, 2, 3],
        vec![1, 2, 3, 4, 5],
    ]);
    assert_eq!(column.len(), 4);
}

#[test]
fn test_binary_random_data() {
    // Random binary data
    let random: Vec<u8> = (0..1000).map(|i| ((i * 7919) % 256) as u8).collect();
    let column = Column::Binary(vec![random]);
    assert_eq!(column.len(), 1);
}

#[test]
fn test_binary_unicode_as_binary() {
    // Unicode strings stored as binary
    let unicode_bytes = "世界🌍".as_bytes().to_vec();
    let column = Column::Binary(vec![unicode_bytes]);
    assert_eq!(column.len(), 1);
}

#[test]
fn test_binary_compression_candidate() {
    // Highly repetitive binary data (good for compression)
    let repetitive = vec![0u8; 10000];
    let column = Column::Binary(vec![repetitive]);
    assert_eq!(column.len(), 1);
}

