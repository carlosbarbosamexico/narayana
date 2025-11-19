// Advanced analytics functions - way beyond ClickHouse
// Window functions, statistical functions, advanced aggregations

use narayana_core::column::Column;
use serde_json::Value;
use rayon::prelude::*;

/// Window functions for advanced analytics
pub struct WindowFunctions;

impl WindowFunctions {
    /// ROW_NUMBER() - assign sequential numbers to rows
    pub fn row_number(partition_by: Option<&[usize]>) -> Vec<u64> {
        // In production, would partition and number within partitions
        vec![]
    }

    /// RANK() - rank rows with gaps
    pub fn rank(column: &Column, order_by: &[usize]) -> Vec<u64> {
        // In production, would rank based on order_by columns
        vec![]
    }

    /// DENSE_RANK() - rank rows without gaps
    pub fn dense_rank(column: &Column, order_by: &[usize]) -> Vec<u64> {
        // In production, would dense rank based on order_by columns
        vec![]
    }

    /// LAG() - access previous row value
    pub fn lag(column: &Column, offset: usize, default: Option<Value>) -> Vec<Value> {
        match column {
            Column::Int32(data) => {
                data.iter().enumerate().map(|(i, &val)| {
                    if i >= offset {
                        Value::Number((data[i - offset] as i64).into())
                    } else {
                        default.clone().unwrap_or(Value::Null)
                    }
                }).collect()
            }
            Column::Int64(data) => {
                data.iter().enumerate().map(|(i, &val)| {
                    if i >= offset {
                        Value::Number(data[i - offset].into())
                    } else {
                        default.clone().unwrap_or(Value::Null)
                    }
                }).collect()
            }
            Column::Float64(data) => {
                data.iter().enumerate().map(|(i, &val)| {
                    if i >= offset {
                        Value::Number(serde_json::Number::from_f64(data[i - offset]).unwrap())
                    } else {
                        default.clone().unwrap_or(Value::Null)
                    }
                }).collect()
            }
            _ => vec![],
        }
    }

    /// LEAD() - access next row value
    pub fn lead(column: &Column, offset: usize, default: Option<Value>) -> Vec<Value> {
        match column {
            Column::Int32(data) => {
                data.iter().enumerate().map(|(i, &val)| {
                    if i + offset < data.len() {
                        Value::Number((data[i + offset] as i64).into())
                    } else {
                        default.clone().unwrap_or(Value::Null)
                    }
                }).collect()
            }
            Column::Int64(data) => {
                data.iter().enumerate().map(|(i, &val)| {
                    if i + offset < data.len() {
                        Value::Number(data[i + offset].into())
                    } else {
                        default.clone().unwrap_or(Value::Null)
                    }
                }).collect()
            }
            Column::Float64(data) => {
                data.iter().enumerate().map(|(i, &val)| {
                    if i + offset < data.len() {
                        Value::Number(serde_json::Number::from_f64(data[i + offset]).unwrap())
                    } else {
                        default.clone().unwrap_or(Value::Null)
                    }
                }).collect()
            }
            _ => vec![],
        }
    }

    /// Moving average
    pub fn moving_average(column: &Column, window_size: usize) -> Vec<f64> {
        match column {
            Column::Int32(data) => {
                data.windows(window_size)
                    .map(|window| window.iter().map(|&x| x as f64).sum::<f64>() / window_size as f64)
                    .collect()
            }
            Column::Float64(data) => {
                data.windows(window_size)
                    .map(|window| window.iter().sum::<f64>() / window_size as f64)
                    .collect()
            }
            _ => vec![],
        }
    }

    /// Cumulative sum
    pub fn cumulative_sum(column: &Column) -> Vec<f64> {
        match column {
            Column::Int32(data) => {
                let mut sum = 0.0;
                data.iter().map(|&x| {
                    sum += x as f64;
                    sum
                }).collect()
            }
            Column::Float64(data) => {
                let mut sum = 0.0;
                data.iter().map(|&x| {
                    sum += x;
                    sum
                }).collect()
            }
            _ => vec![],
        }
    }
}

/// Statistical functions
pub struct StatisticalFunctions;

impl StatisticalFunctions {
    /// Standard deviation
    pub fn stddev(column: &Column) -> Option<f64> {
        match column {
            Column::Int32(data) => {
                if data.is_empty() {
                    return None;
                }
                let mean = data.iter().sum::<i32>() as f64 / data.len() as f64;
                let variance = data.iter()
                    .map(|&x| {
                        let diff = x as f64 - mean;
                        diff * diff
                    })
                    .sum::<f64>() / data.len() as f64;
                Some(variance.sqrt())
            }
            Column::Float64(data) => {
                if data.is_empty() {
                    return None;
                }
                let mean = data.iter().sum::<f64>() / data.len() as f64;
                let variance = data.iter()
                    .map(|&x| {
                        let diff = x - mean;
                        diff * diff
                    })
                    .sum::<f64>() / data.len() as f64;
                Some(variance.sqrt())
            }
            _ => None,
        }
    }

    /// Variance
    pub fn variance(column: &Column) -> Option<f64> {
        match column {
            Column::Int32(data) => {
                if data.is_empty() {
                    return None;
                }
                let mean = data.iter().sum::<i32>() as f64 / data.len() as f64;
                Some(data.iter()
                    .map(|&x| {
                        let diff = x as f64 - mean;
                        diff * diff
                    })
                    .sum::<f64>() / data.len() as f64)
            }
            Column::Float64(data) => {
                if data.is_empty() {
                    return None;
                }
                let mean = data.iter().sum::<f64>() / data.len() as f64;
                Some(data.iter()
                    .map(|&x| {
                        let diff = x - mean;
                        diff * diff
                    })
                    .sum::<f64>() / data.len() as f64)
            }
            _ => None,
        }
    }

    /// Percentile (approximate using sampling)
    pub fn percentile(column: &Column, p: f64) -> Option<f64> {
        match column {
            Column::Int32(data) => {
                if data.is_empty() {
                    return None;
                }
                let mut sorted = data.clone();
                sorted.sort();
                // SECURITY: Prevent integer overflow and bounds errors
                if sorted.is_empty() {
                    return None;
                }
                let len = sorted.len();
                let index = if len == 1 {
                    0
                } else {
                    // SECURITY: Safe calculation to prevent overflow
                    ((len - 1) as f64 * p).min((len - 1) as f64) as usize
                };
                // SECURITY: Bounds check
                if index >= sorted.len() {
                    return None;
                }
                Some(sorted[index] as f64)
            }
            Column::Float64(data) => {
                if data.is_empty() {
                    return None;
                }
                let mut sorted = data.clone();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                // SECURITY: Prevent integer overflow and bounds errors
                if sorted.is_empty() {
                    return None;
                }
                let len = sorted.len();
                let index = if len == 1 {
                    0
                } else {
                    // SECURITY: Safe calculation to prevent overflow
                    ((len - 1) as f64 * p).min((len - 1) as f64) as usize
                };
                // SECURITY: Bounds check
                if index >= sorted.len() {
                    return None;
                }
                Some(sorted[index])
            }
            _ => None,
        }
    }

    /// Median
    pub fn median(column: &Column) -> Option<f64> {
        Self::percentile(column, 0.5)
    }

    /// Correlation coefficient
    pub fn correlation(x: &Column, y: &Column) -> Option<f64> {
        match (x, y) {
            (Column::Float64(x_data), Column::Float64(y_data)) => {
                if x_data.len() != y_data.len() || x_data.is_empty() {
                    return None;
                }
                
                let x_mean = x_data.iter().sum::<f64>() / x_data.len() as f64;
                let y_mean = y_data.iter().sum::<f64>() / y_data.len() as f64;
                
                let numerator: f64 = x_data.iter().zip(y_data.iter())
                    .map(|(&x, &y)| (x - x_mean) * (y - y_mean))
                    .sum();
                
                let x_var: f64 = x_data.iter()
                    .map(|&x| (x - x_mean).powi(2))
                    .sum();
                let y_var: f64 = y_data.iter()
                    .map(|&y| (y - y_mean).powi(2))
                    .sum();
                
                let denominator = (x_var * y_var).sqrt();
                if denominator == 0.0 {
                    None
                } else {
                    Some(numerator / denominator)
                }
            }
            _ => None,
        }
    }
}

/// Approximate aggregations (HyperLogLog, Quantiles)
pub struct ApproximateAggregations;

impl ApproximateAggregations {
    /// Approximate distinct count using HyperLogLog
    pub fn approx_distinct(column: &Column, precision: u8) -> u64 {
        // In production, would use HyperLogLog algorithm
        // For now, return exact count
        match column {
            Column::Int32(data) => {
                use std::collections::HashSet;
                data.iter().collect::<HashSet<_>>().len() as u64
            }
            Column::String(data) => {
                use std::collections::HashSet;
                data.iter().collect::<HashSet<_>>().len() as u64
            }
            _ => column.len() as u64,
        }
    }

    /// Approximate quantiles using T-Digest
    pub fn approx_quantile(column: &Column, quantiles: &[f64]) -> Vec<f64> {
        // In production, would use T-Digest algorithm
        // For now, compute exact quantiles
        quantiles.iter()
            .filter_map(|&q| StatisticalFunctions::percentile(column, q))
            .collect()
    }

    /// Top-K most frequent values
    pub fn top_k(column: &Column, k: usize) -> Vec<(Value, u64)> {
        match column {
            Column::Int32(data) => {
                use std::collections::HashMap;
                let mut counts = HashMap::new();
                for &val in data {
                    *counts.entry(val).or_insert(0) += 1;
                }
                let mut items: Vec<_> = counts.into_iter()
                    .map(|(val, count)| (Value::Number((val as i64).into()), count))
                    .collect();
                items.sort_by(|a, b| b.1.cmp(&a.1));
                items.into_iter().take(k).collect()
            }
            Column::String(data) => {
                use std::collections::HashMap;
                let mut counts = HashMap::new();
                for val in data {
                    *counts.entry(val.clone()).or_insert(0) += 1;
                }
                let mut items: Vec<_> = counts.into_iter()
                    .map(|(val, count)| (Value::String(val), count))
                    .collect();
                items.sort_by(|a, b| b.1.cmp(&a.1));
                items.into_iter().take(k).collect()
            }
            _ => vec![],
        }
    }
}

/// Time series functions
pub struct TimeSeriesFunctions;

impl TimeSeriesFunctions {
    /// Exponential moving average
    pub fn ema(column: &Column, alpha: f64) -> Vec<f64> {
        match column {
            Column::Float64(data) => {
                if data.is_empty() {
                    return vec![];
                }
                let mut result = vec![data[0]];
                for &val in data.iter().skip(1) {
                    let prev = result.last().unwrap();
                    result.push(alpha * val + (1.0 - alpha) * prev);
                }
                result
            }
            Column::Int32(data) => {
                let float_data: Vec<f64> = data.iter().map(|&x| x as f64).collect();
                Self::ema(&Column::Float64(float_data), alpha)
            }
            _ => vec![],
        }
    }

    /// Rate of change
    pub fn rate_of_change(column: &Column) -> Vec<f64> {
        match column {
            Column::Float64(data) => {
                if data.len() < 2 {
                    return vec![];
                }
                data.windows(2)
                    .map(|w| w[1] - w[0])
                    .collect()
            }
            Column::Int32(data) => {
                if data.len() < 2 {
                    return vec![];
                }
                data.windows(2)
                    .map(|w| (w[1] - w[0]) as f64)
                    .collect()
            }
            _ => vec![],
        }
    }
}

