use narayana_core::schema::Schema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlanNode {
    Scan {
        table_id: u64,
        column_ids: Vec<u32>,
        filter: Option<Filter>,
    },
    Filter {
        predicate: Filter,
        input: Box<PlanNode>,
    },
    Project {
        columns: Vec<String>,
        input: Box<PlanNode>,
    },
    Aggregate {
        group_by: Vec<String>,
        aggregates: Vec<AggregateExpr>,
        input: Box<PlanNode>,
    },
    Join {
        left: Box<PlanNode>,
        right: Box<PlanNode>,
        join_type: JoinType,
        condition: JoinCondition,
    },
    Sort {
        order_by: Vec<OrderBy>,
        input: Box<PlanNode>,
    },
    Limit {
        limit: usize,
        offset: usize,
        input: Box<PlanNode>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Filter {
    Eq { column: String, value: serde_json::Value },
    Ne { column: String, value: serde_json::Value },
    Gt { column: String, value: serde_json::Value },
    Lt { column: String, value: serde_json::Value },
    Gte { column: String, value: serde_json::Value },
    Lte { column: String, value: serde_json::Value },
    And { left: Box<Filter>, right: Box<Filter> },
    Or { left: Box<Filter>, right: Box<Filter> },
    Not { expr: Box<Filter> },
    In { column: String, values: Vec<serde_json::Value> },
    Between { column: String, low: serde_json::Value, high: serde_json::Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregateExpr {
    Count { column: Option<String> },
    Sum { column: String },
    Avg { column: String },
    Min { column: String },
    Max { column: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JoinCondition {
    Equi { left: String, right: String },
    On { predicate: Filter },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBy {
    pub column: String,
    pub ascending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    pub root: PlanNode,
    pub output_schema: Schema,
}

impl QueryPlan {
    pub fn new(root: PlanNode, output_schema: Schema) -> Self {
        Self { root, output_schema }
    }
}

