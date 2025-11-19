// Query DSL helpers

use crate::elegant::{QueryBuilder, FilterBuilder, OrderByBuilder, Value, QueryResult};
use crate::connection::Connection;
use std::result::Result as StdResult;
use std::sync::Arc;

/// Query DSL helpers

/// Select from table
pub trait SelectFrom {
    fn from(self, table: &str, connection: Arc<dyn Connection>) -> QueryBuilder;
}

impl SelectFrom for Vec<&str> {
    fn from(self, table: &str, connection: Arc<dyn Connection>) -> QueryBuilder {
        QueryBuilder::new(table.to_string(), connection).select(&self)
    }
}

/// Beautiful query examples and helpers

/// Example: Beautiful query syntax
/// 
/// ```rust
/// use narayana_api::elegant::*;
/// 
/// // Beautiful and intuitive
/// let result = db
///     .database("analytics")
///     .table("events")
///     .query()
///     .select(&["id", "name", "timestamp"])
///     .where("timestamp")
///     .gte(1000)
///     .and()
///     .where("status")
///     .eq("active")
///     .order_by("timestamp")
///     .desc()
///     .limit(100)
///     .execute()
///     .await?;
/// ```
pub mod examples {
    // This module contains examples of beautiful API usage
}

/// Fluent query builder extensions
impl QueryBuilder {
    /// Chain multiple where clauses with AND
    pub fn and(mut self) -> Self {
        // In production, would combine filters with AND
        self
    }

    /// Chain multiple where clauses with OR
    pub fn or(mut self) -> Self {
        // In production, would combine filters with OR
        self
    }
}

impl FilterBuilder {
    /// Chain with AND
    pub fn and(self) -> QueryBuilder {
        // Return query builder for chaining (clone handles connection)
        self.query
    }
}

/// Beautiful aggregation builder
pub struct AggregateBuilder {
    query: QueryBuilder,
    aggregates: Vec<AggregateExpr>,
    group_by: Vec<String>,
}

impl AggregateBuilder {
    pub fn new(query: QueryBuilder) -> Self {
        Self {
            query,
            aggregates: Vec::new(),
            group_by: Vec::new(),
        }
    }

    /// Count
    pub fn count(mut self, column: Option<&str>) -> Self {
        self.aggregates.push(AggregateExpr::Count {
            column: column.map(|s| s.to_string()),
        });
        self
    }

    /// Sum
    pub fn sum(mut self, column: &str) -> Self {
        self.aggregates.push(AggregateExpr::Sum {
            column: column.to_string(),
        });
        self
    }

    /// Average
    pub fn avg(mut self, column: &str) -> Self {
        self.aggregates.push(AggregateExpr::Avg {
            column: column.to_string(),
        });
        self
    }

    /// Min
    pub fn min(mut self, column: &str) -> Self {
        self.aggregates.push(AggregateExpr::Min {
            column: column.to_string(),
        });
        self
    }

    /// Max
    pub fn max(mut self, column: &str) -> Self {
        self.aggregates.push(AggregateExpr::Max {
            column: column.to_string(),
        });
        self
    }

    /// Group by
    pub fn group_by(mut self, columns: &[&str]) -> Self {
        self.group_by = columns.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Execute aggregation
    pub async fn execute(self) -> StdResult<QueryResult, narayana_core::Error> {
        // In production, would execute aggregation
        Ok(QueryResult {
            columns: vec![],
            rows: vec![],
        })
    }
}

#[derive(Debug, Clone)]
enum AggregateExpr {
    Count { column: Option<String> },
    Sum { column: String },
    Avg { column: String },
    Min { column: String },
    Max { column: String },
}

