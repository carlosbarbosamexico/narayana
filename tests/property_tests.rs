use proptest::prelude::*;
use narayana_core::column::Column;
use narayana_query::vectorized::VectorizedOps;

proptest! {
    #[test]
    fn test_filter_property(data in prop::collection::vec(-1000i32..1000i32, 1..1000)) {
        let column = Column::Int32(data.clone());
        let mask: Vec<bool> = (0..data.len()).map(|i| i % 2 == 0).collect();
        let filtered = VectorizedOps::filter(&column, &mask);
        
        match filtered {
            Column::Int32(filtered_data) => {
                // Filtered data should be a subset
                assert!(filtered_data.len() <= data.len());
                // All filtered values should be in original
                for val in &filtered_data {
                    assert!(data.contains(val));
                }
            }
            _ => panic!("Expected Int32 column"),
        }
    }

    #[test]
    fn test_sum_property(data in prop::collection::vec(-1000i32..1000i32, 1..1000)) {
        let column = Column::Int32(data.clone());
        let sum = VectorizedOps::sum(&column);
        
        if let Some(serde_json::Value::Number(n)) = sum {
            let sum_value = n.as_i64().unwrap();
            let expected_sum: i64 = data.iter().map(|&x| x as i64).sum();
            assert_eq!(sum_value, expected_sum);
        } else {
            panic!("Expected number result");
        }
    }

    #[test]
    fn test_min_max_property(data in prop::collection::vec(-1000i32..1000i32, 1..1000)) {
        if data.is_empty() {
            return Ok(());
        }
        
        let column = Column::Int32(data.clone());
        let min = VectorizedOps::min(&column);
        let max = VectorizedOps::max(&column);
        
        let expected_min = *data.iter().min().unwrap();
        let expected_max = *data.iter().max().unwrap();
        
        if let Some(serde_json::Value::Number(n)) = min {
            assert_eq!(n.as_i64().unwrap(), expected_min as i64);
        } else {
            panic!("Expected min value");
        }
        
        if let Some(serde_json::Value::Number(n)) = max {
            assert_eq!(n.as_i64().unwrap(), expected_max as i64);
        } else {
            panic!("Expected max value");
        }
    }

    #[test]
    fn test_compare_eq_property(
        data in prop::collection::vec(-1000i32..1000i32, 1..1000),
        value in -1000i32..1000i32
    ) {
        let column = Column::Int32(data.clone());
        let json_value = serde_json::Value::Number(value.into());
        let mask = VectorizedOps::compare_eq(&column, &json_value);
        
        assert_eq!(mask.len(), data.len());
        for (i, &matched) in mask.iter().enumerate() {
            assert_eq!(matched, data[i] == value);
        }
    }

    #[test]
    fn test_compare_gt_property(
        data in prop::collection::vec(-1000i32..1000i32, 1..1000),
        value in -1000i32..1000i32
    ) {
        let column = Column::Int32(data.clone());
        let json_value = serde_json::Value::Number(value.into());
        let mask = VectorizedOps::compare_gt(&column, &json_value);
        
        assert_eq!(mask.len(), data.len());
        for (i, &matched) in mask.iter().enumerate() {
            assert_eq!(matched, data[i] > value);
        }
    }

    #[test]
    fn test_compare_lt_property(
        data in prop::collection::vec(-1000i32..1000i32, 1..1000),
        value in -1000i32..1000i32
    ) {
        let column = Column::Int32(data.clone());
        let json_value = serde_json::Value::Number(value.into());
        let mask = VectorizedOps::compare_lt(&column, &json_value);
        
        assert_eq!(mask.len(), data.len());
        for (i, &matched) in mask.iter().enumerate() {
            assert_eq!(matched, data[i] < value);
        }
    }
}

