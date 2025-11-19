use narayana_core::{Error, Result, column::Column, schema::Schema};
use crate::plan::{PlanNode, Filter};
use crate::vectorized::VectorizedOps;

pub struct ScanOperator {
    table_id: u64,
    column_ids: Vec<u32>,
    schema: Schema,
}

impl ScanOperator {
    pub fn new(table_id: u64, column_ids: Vec<u32>, schema: Schema) -> Self {
        Self {
            table_id,
            column_ids,
            schema,
        }
    }
}

pub struct FilterOperator {
    predicate: Filter,
    input_schema: Schema,
}

impl FilterOperator {
    pub fn new(predicate: Filter, input_schema: Schema) -> Self {
        Self {
            predicate,
            input_schema,
        }
    }

    pub fn apply(&self, columns: &[Column]) -> Result<Vec<Column>> {
        let mask = self.evaluate_predicate(columns)?;
        Ok(columns.iter().map(|col| VectorizedOps::filter(col, &mask)).collect())
    }

    fn evaluate_predicate(&self, columns: &[Column]) -> Result<Vec<bool>> {
        match &self.predicate {
            Filter::Eq { column, value } => {
                let col_idx = self.input_schema
                    .field_index(column)
                    .ok_or_else(|| Error::Query(format!("Column not found: {}", column)))?;
                let column = &columns[col_idx];
                Ok(VectorizedOps::compare_eq(column, value))
            }
            Filter::Gt { column, value } => {
                let col_idx = self.input_schema
                    .field_index(column)
                    .ok_or_else(|| Error::Query(format!("Column not found: {}", column)))?;
                let column = &columns[col_idx];
                Ok(VectorizedOps::compare_gt(column, value))
            }
            Filter::Lt { column, value } => {
                let col_idx = self.input_schema
                    .field_index(column)
                    .ok_or_else(|| Error::Query(format!("Column not found: {}", column)))?;
                let column = &columns[col_idx];
                Ok(VectorizedOps::compare_lt(column, value))
            }
            Filter::And { left, right } => {
                let left_mask = self.evaluate_predicate_for_filter(left, columns)?;
                let right_mask = self.evaluate_predicate_for_filter(right, columns)?;
                Ok(left_mask.iter().zip(right_mask.iter()).map(|(a, b)| *a && *b).collect())
            }
            Filter::Or { left, right } => {
                let left_mask = self.evaluate_predicate_for_filter(left, columns)?;
                let right_mask = self.evaluate_predicate_for_filter(right, columns)?;
                Ok(left_mask.iter().zip(right_mask.iter()).map(|(a, b)| *a || *b).collect())
            }
            _ => Err(Error::Query("Unsupported filter predicate".to_string())),
        }
    }

    fn evaluate_predicate_for_filter(&self, filter: &Filter, columns: &[Column]) -> Result<Vec<bool>> {
        match filter {
            Filter::Eq { column, value } => {
                let col_idx = self.input_schema
                    .field_index(column)
                    .ok_or_else(|| Error::Query(format!("Column not found: {}", column)))?;
                let column = &columns[col_idx];
                Ok(VectorizedOps::compare_eq(column, value))
            }
            Filter::Gt { column, value } => {
                let col_idx = self.input_schema
                    .field_index(column)
                    .ok_or_else(|| Error::Query(format!("Column not found: {}", column)))?;
                let column = &columns[col_idx];
                Ok(VectorizedOps::compare_gt(column, value))
            }
            Filter::Lt { column, value } => {
                let col_idx = self.input_schema
                    .field_index(column)
                    .ok_or_else(|| Error::Query(format!("Column not found: {}", column)))?;
                let column = &columns[col_idx];
                Ok(VectorizedOps::compare_lt(column, value))
            }
            _ => Err(Error::Query("Unsupported filter predicate".to_string())),
        }
    }
}

pub struct ProjectOperator {
    column_indices: Vec<usize>,
    output_schema: Schema,
}

impl ProjectOperator {
    pub fn new(column_names: Vec<String>, input_schema: Schema) -> Result<Self> {
        let column_indices: Result<Vec<usize>> = column_names
            .iter()
            .map(|name| {
                input_schema
                    .field_index(name)
                    .ok_or_else(|| Error::Query(format!("Column not found: {}", name)))
            })
            .collect();

        let column_indices = column_indices?;
        let output_fields: Vec<_> = column_indices
            .iter()
            .map(|&idx| input_schema.fields[idx].clone())
            .collect();
        let output_schema = Schema::new(output_fields);

        Ok(Self {
            column_indices,
            output_schema,
        })
    }

    pub fn apply(&self, columns: &[Column]) -> Vec<Column> {
        self.column_indices.iter().map(|&idx| columns[idx].clone()).collect()
    }

    pub fn output_schema(&self) -> &Schema {
        &self.output_schema
    }
}

/// Join operator for combining two tables
pub struct JoinOperator {
    join_type: JoinType,
    left_key: String,
    right_key: String,
    left_schema: Schema,
    right_schema: Schema,
}

#[derive(Debug, Clone)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

impl JoinOperator {
    pub fn new(
        join_type: JoinType,
        left_key: String,
        right_key: String,
        left_schema: Schema,
        right_schema: Schema,
    ) -> Result<Self> {
        // Validate keys exist in schemas
        left_schema.field_index(&left_key)
            .ok_or_else(|| Error::Query(format!("Left join key not found: {}", left_key)))?;
        right_schema.field_index(&right_key)
            .ok_or_else(|| Error::Query(format!("Right join key not found: {}", right_key)))?;

        Ok(Self {
            join_type,
            left_key,
            right_key,
            left_schema,
            right_schema,
        })
    }

    pub fn apply(&self, left_columns: &[Column], right_columns: &[Column]) -> Result<Vec<Column>> {
        let left_key_idx = self.left_schema.field_index(&self.left_key)
            .ok_or_else(|| Error::Query("Left key not found".to_string()))?;
        let right_key_idx = self.right_schema.field_index(&self.right_key)
            .ok_or_else(|| Error::Query("Right key not found".to_string()))?;

        // Build hash map for right side (hash join)
        let mut right_map: std::collections::HashMap<u64, Vec<usize>> = std::collections::HashMap::new();
        
        let right_key_col = &right_columns[right_key_idx];
        let right_len = right_key_col.len();
        
        for i in 0..right_len {
            let key = self.hash_value(right_key_col, i)?;
            right_map.entry(key).or_insert_with(Vec::new).push(i);
        }

        // Perform join
        let left_key_col = &left_columns[left_key_idx];
        let left_len = left_key_col.len();
        let mut result_indices: Vec<(usize, Option<usize>)> = Vec::new();

        for left_idx in 0..left_len {
            let key = self.hash_value(left_key_col, left_idx)?;
            if let Some(right_indices) = right_map.get(&key) {
                for &right_idx in right_indices {
                    // Verify actual match (hash collision check)
                    if self.values_match(left_key_col, left_idx, right_key_col, right_idx)? {
                        result_indices.push((left_idx, Some(right_idx)));
                    }
                }
            } else {
                // No match - handle based on join type
                match self.join_type {
                    JoinType::Inner => {} // Skip
                    JoinType::Left | JoinType::Full => {
                        result_indices.push((left_idx, None));
                    }
                    JoinType::Right => {} // Skip
                }
            }
        }

        // Build result columns
        let mut result_columns: Vec<Column> = Vec::new();
        
        // Add left columns
        for col in left_columns {
            let mut result_data = Vec::new();
            for (left_idx, _) in &result_indices {
                result_data.push(self.get_value(col, *left_idx)?);
            }
            result_columns.push(self.create_column_from_values(result_data)?);
        }

        // Add right columns
        for col in right_columns {
            let mut result_data = Vec::new();
            for (_, right_idx_opt) in &result_indices {
                if let Some(right_idx) = right_idx_opt {
                    result_data.push(self.get_value(col, *right_idx)?);
                } else {
                    result_data.push(None); // NULL for outer join
                }
            }
            result_columns.push(self.create_column_from_values(result_data)?);
        }

        Ok(result_columns)
    }

    fn hash_value(&self, col: &Column, idx: usize) -> Result<u64> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        match col {
            Column::Int32(v) => v[idx].hash(&mut hasher),
            Column::Int64(v) => v[idx].hash(&mut hasher),
            Column::UInt64(v) => v[idx].hash(&mut hasher),
            Column::String(v) => v[idx].hash(&mut hasher),
            _ => return Err(Error::Query("Unsupported column type for join".to_string())),
        }
        Ok(hasher.finish())
    }

    fn values_match(&self, left_col: &Column, left_idx: usize, right_col: &Column, right_idx: usize) -> Result<bool> {
        match (left_col, right_col) {
            (Column::Int32(l), Column::Int32(r)) => Ok(l[left_idx] == r[right_idx]),
            (Column::Int64(l), Column::Int64(r)) => Ok(l[left_idx] == r[right_idx]),
            (Column::UInt64(l), Column::UInt64(r)) => Ok(l[left_idx] == r[right_idx]),
            (Column::String(l), Column::String(r)) => Ok(l[left_idx] == r[right_idx]),
            _ => Err(Error::Query("Type mismatch in join".to_string())),
        }
    }

    fn get_value(&self, col: &Column, idx: usize) -> Result<Option<serde_json::Value>> {
        match col {
            Column::Int32(v) => Ok(Some(serde_json::Value::Number(v[idx].into()))),
            Column::Int64(v) => Ok(Some(serde_json::Value::Number(v[idx].into()))),
            Column::UInt64(v) => Ok(Some(serde_json::Value::Number(v[idx].into()))),
            Column::String(v) => Ok(Some(serde_json::Value::String(v[idx].clone()))),
            _ => Err(Error::Query("Unsupported column type".to_string())),
        }
    }

    fn create_column_from_values(&self, values: Vec<Option<serde_json::Value>>) -> Result<Column> {
        if values.is_empty() {
            return Err(Error::Query("Empty values".to_string()));
        }
        
        // Determine type from first non-null value
        if let Some(Some(first)) = values.first() {
            match first {
                serde_json::Value::Number(n) if n.is_i64() => {
                    Ok(Column::Int64(values.iter().map(|v| v.as_ref().and_then(|v| v.as_i64()).unwrap_or(0)).collect()))
                }
                serde_json::Value::Number(n) if n.is_u64() => {
                    Ok(Column::UInt64(values.iter().map(|v| v.as_ref().and_then(|v| v.as_u64()).unwrap_or(0)).collect()))
                }
                serde_json::Value::String(_) => {
                    Ok(Column::String(values.iter().map(|v| v.as_ref().and_then(|v| v.as_str()).unwrap_or("").to_string()).collect()))
                }
                _ => Err(Error::Query("Unsupported value type".to_string())),
            }
        } else {
            Err(Error::Query("All values are null".to_string()))
        }
    }
}

/// Aggregate operator for grouping and aggregation
pub struct AggregateOperator {
    group_by: Vec<String>,
    aggregates: Vec<AggregateFunction>,
    input_schema: Schema,
}

#[derive(Debug, Clone)]
pub enum AggregateFunction {
    Count { column: Option<String> },
    Sum { column: String },
    Avg { column: String },
    Min { column: String },
    Max { column: String },
}

impl AggregateOperator {
    pub fn new(
        group_by: Vec<String>,
        aggregates: Vec<AggregateFunction>,
        input_schema: Schema,
    ) -> Result<Self> {
        // Validate columns exist
        for col in &group_by {
            input_schema.field_index(col)
                .ok_or_else(|| Error::Query(format!("Group by column not found: {}", col)))?;
        }

        for agg in &aggregates {
            match agg {
                AggregateFunction::Count { column: Some(col) } |
                AggregateFunction::Sum { column: col } |
                AggregateFunction::Avg { column: col } |
                AggregateFunction::Min { column: col } |
                AggregateFunction::Max { column: col } => {
                    input_schema.field_index(col)
                        .ok_or_else(|| Error::Query(format!("Aggregate column not found: {}", col)))?;
                }
                AggregateFunction::Count { column: None } => {}
            }
        }

        Ok(Self {
            group_by,
            aggregates,
            input_schema,
        })
    }

    pub fn apply(&self, columns: &[Column]) -> Result<Vec<Column>> {
        // Build group keys
        let group_indices: Vec<usize> = self.group_by.iter()
            .map(|col| self.input_schema.field_index(col).unwrap())
            .collect();

        let num_rows = if columns.is_empty() { 0 } else { columns[0].len() };
        
        // Group rows
        let mut groups: std::collections::HashMap<Vec<u64>, Vec<usize>> = std::collections::HashMap::new();
        
        for row_idx in 0..num_rows {
            let mut key = Vec::new();
            for &col_idx in &group_indices {
                let hash = self.hash_value(&columns[col_idx], row_idx)?;
                key.push(hash);
            }
            groups.entry(key).or_insert_with(Vec::new).push(row_idx);
        }

        // Compute aggregates for each group
        let mut result_columns: Vec<Column> = Vec::new();
        
        // Add group by columns
        for &col_idx in &group_indices {
            let mut group_values = Vec::new();
            for group_rows in groups.values() {
                if let Some(&first_row) = group_rows.first() {
                    group_values.push(self.get_value(&columns[col_idx], first_row)?);
                }
            }
            result_columns.push(self.create_column_from_values(group_values)?);
        }

        // Add aggregate columns
        for agg in &self.aggregates {
            let agg_col = match agg {
                AggregateFunction::Count { column: _ } => {
                    Column::UInt64(groups.values().map(|rows| rows.len() as u64).collect())
                }
                AggregateFunction::Sum { column } => {
                    let col_idx = self.input_schema.field_index(column).unwrap();
                    let mut sums = Vec::new();
                    for group_rows in groups.values() {
                        let mut sum = 0.0;
                        for &row_idx in group_rows {
                            sum += self.get_numeric_value(&columns[col_idx], row_idx)?;
                        }
                        sums.push(sum);
                    }
                    Column::Float64(sums)
                }
                AggregateFunction::Avg { column } => {
                    let col_idx = self.input_schema.field_index(column).unwrap();
                    let mut avgs = Vec::new();
                    for group_rows in groups.values() {
                        let mut sum = 0.0;
                        for &row_idx in group_rows {
                            sum += self.get_numeric_value(&columns[col_idx], row_idx)?;
                        }
                        avgs.push(sum / group_rows.len() as f64);
                    }
                    Column::Float64(avgs)
                }
                AggregateFunction::Min { column } => {
                    let col_idx = self.input_schema.field_index(column).unwrap();
                    let mut mins = Vec::new();
                    for group_rows in groups.values() {
                        let mut min_val = f64::MAX;
                        for &row_idx in group_rows {
                            let val = self.get_numeric_value(&columns[col_idx], row_idx)?;
                            if val < min_val {
                                min_val = val;
                            }
                        }
                        mins.push(min_val);
                    }
                    Column::Float64(mins)
                }
                AggregateFunction::Max { column } => {
                    let col_idx = self.input_schema.field_index(column).unwrap();
                    let mut maxs = Vec::new();
                    for group_rows in groups.values() {
                        let mut max_val = f64::MIN;
                        for &row_idx in group_rows {
                            let val = self.get_numeric_value(&columns[col_idx], row_idx)?;
                            if val > max_val {
                                max_val = val;
                            }
                        }
                        maxs.push(max_val);
                    }
                    Column::Float64(maxs)
                }
            };
            result_columns.push(agg_col);
        }

        Ok(result_columns)
    }

    fn hash_value(&self, col: &Column, idx: usize) -> Result<u64> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        match col {
            Column::Int32(v) => v[idx].hash(&mut hasher),
            Column::Int64(v) => v[idx].hash(&mut hasher),
            Column::UInt64(v) => v[idx].hash(&mut hasher),
            Column::String(v) => v[idx].hash(&mut hasher),
            _ => return Err(Error::Query("Unsupported column type for grouping".to_string())),
        }
        Ok(hasher.finish())
    }

    fn get_numeric_value(&self, col: &Column, idx: usize) -> Result<f64> {
        match col {
            Column::Int32(v) => Ok(v[idx] as f64),
            Column::Int64(v) => Ok(v[idx] as f64),
            Column::UInt64(v) => Ok(v[idx] as f64),
            Column::Float64(v) => Ok(v[idx]),
            _ => Err(Error::Query("Not a numeric column".to_string())),
        }
    }

    fn get_value(&self, col: &Column, idx: usize) -> Result<Option<serde_json::Value>> {
        match col {
            Column::Int32(v) => Ok(Some(serde_json::Value::Number(v[idx].into()))),
            Column::Int64(v) => Ok(Some(serde_json::Value::Number(v[idx].into()))),
            Column::UInt64(v) => Ok(Some(serde_json::Value::Number(v[idx].into()))),
            Column::String(v) => Ok(Some(serde_json::Value::String(v[idx].clone()))),
            _ => Err(Error::Query("Unsupported column type".to_string())),
        }
    }

    fn create_column_from_values(&self, values: Vec<Option<serde_json::Value>>) -> Result<Column> {
        if values.is_empty() {
            return Err(Error::Query("Empty values".to_string()));
        }
        
        if let Some(Some(first)) = values.first() {
            match first {
                serde_json::Value::Number(n) if n.is_i64() => {
                    Ok(Column::Int64(values.iter().map(|v| v.as_ref().and_then(|v| v.as_i64()).unwrap_or(0)).collect()))
                }
                serde_json::Value::Number(n) if n.is_u64() => {
                    Ok(Column::UInt64(values.iter().map(|v| v.as_ref().and_then(|v| v.as_u64()).unwrap_or(0)).collect()))
                }
                serde_json::Value::String(_) => {
                    Ok(Column::String(values.iter().map(|v| v.as_ref().and_then(|v| v.as_str()).unwrap_or("").to_string()).collect()))
                }
                _ => Err(Error::Query("Unsupported value type".to_string())),
            }
        } else {
            Err(Error::Query("All values are null".to_string()))
        }
    }
}

