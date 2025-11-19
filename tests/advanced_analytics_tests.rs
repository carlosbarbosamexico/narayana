// Tests for advanced analytics

use narayana_query::advanced_analytics::*;
use narayana_core::column::Column;

#[test]
fn test_window_functions_row_number() {
    let row_numbers = WindowFunctions::row_number(None);
    // Should return empty for now
    assert_eq!(row_numbers.len(), 0);
}

#[test]
fn test_window_functions_lag() {
    let column = Column::Int32(vec![1, 2, 3, 4, 5]);
    let lagged = WindowFunctions::lag(&column, 1, None);
    
    assert_eq!(lagged.len(), 5);
    assert_eq!(lagged[0], serde_json::Value::Null); // First element has no previous
    assert_eq!(lagged[1], serde_json::Value::Number(1.into())); // Previous value
}

#[test]
fn test_window_functions_lead() {
    let column = Column::Int32(vec![1, 2, 3, 4, 5]);
    let led = WindowFunctions::lead(&column, 1, None);
    
    assert_eq!(led.len(), 5);
    assert_eq!(led[0], serde_json::Value::Number(2.into())); // Next value
    assert_eq!(led[4], serde_json::Value::Null); // Last element has no next
}

#[test]
fn test_window_functions_moving_average() {
    let column = Column::Int32(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    let moving_avg = WindowFunctions::moving_average(&column, 3);
    
    assert_eq!(moving_avg.len(), 8); // 10 - 3 + 1 = 8
    assert!((moving_avg[0] - 2.0).abs() < 0.001); // (1+2+3)/3 = 2
}

#[test]
fn test_window_functions_cumulative_sum() {
    let column = Column::Int32(vec![1, 2, 3, 4, 5]);
    let cumsum = WindowFunctions::cumulative_sum(&column);
    
    assert_eq!(cumsum.len(), 5);
    assert_eq!(cumsum[0], 1.0);
    assert_eq!(cumsum[1], 3.0);
    assert_eq!(cumsum[4], 15.0);
}

#[test]
fn test_statistical_functions_stddev() {
    let column = Column::Int32(vec![1, 2, 3, 4, 5]);
    let stddev = StatisticalFunctions::stddev(&column);
    assert!(stddev.is_some());
    assert!(stddev.unwrap() > 0.0);
}

#[test]
fn test_statistical_functions_variance() {
    let column = Column::Int32(vec![1, 2, 3, 4, 5]);
    let variance = StatisticalFunctions::variance(&column);
    assert!(variance.is_some());
    assert!(variance.unwrap() > 0.0);
}

#[test]
fn test_statistical_functions_median() {
    let column = Column::Int32(vec![1, 2, 3, 4, 5]);
    let median = StatisticalFunctions::median(&column);
    assert_eq!(median, Some(3.0));
}

#[test]
fn test_statistical_functions_percentile() {
    let column = Column::Int32(vec![1, 2, 3, 4, 5]);
    let p50 = StatisticalFunctions::percentile(&column, 0.5);
    assert_eq!(p50, Some(3.0));
    
    let p95 = StatisticalFunctions::percentile(&column, 0.95);
    assert_eq!(p95, Some(5.0));
}

#[test]
fn test_statistical_functions_correlation() {
    let x = Column::Float64(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    let y = Column::Float64(vec![2.0, 4.0, 6.0, 8.0, 10.0]);
    
    let correlation = StatisticalFunctions::correlation(&x, &y);
    assert!(correlation.is_some());
    assert!((correlation.unwrap() - 1.0).abs() < 0.001); // Perfect correlation
}

#[test]
fn test_approximate_aggregations_approx_distinct() {
    let column = Column::Int32(vec![1, 2, 2, 3, 3, 3]);
    let distinct = ApproximateAggregations::approx_distinct(&column, 12);
    assert!(distinct > 0);
}

#[test]
fn test_approximate_aggregations_top_k() {
    let column = Column::Int32(vec![1, 1, 1, 2, 2, 3]);
    let top_2 = ApproximateAggregations::top_k(&column, 2);
    assert_eq!(top_2.len(), 2);
    assert_eq!(top_2[0].1, 3); // Count of 1
}

#[test]
fn test_time_series_functions_ema() {
    let column = Column::Float64(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    let ema = TimeSeriesFunctions::ema(&column, 0.5);
    
    assert_eq!(ema.len(), 5);
    assert_eq!(ema[0], 1.0); // First value is unchanged
}

#[test]
fn test_time_series_functions_rate_of_change() {
    let column = Column::Float64(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    let roc = TimeSeriesFunctions::rate_of_change(&column);
    
    assert_eq!(roc.len(), 4); // n-1 values
    assert_eq!(roc[0], 1.0); // 2.0 - 1.0
}

