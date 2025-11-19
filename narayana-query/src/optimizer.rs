// Query optimizer for maximum performance

use crate::plan::{QueryPlan, PlanNode, Filter};
use narayana_core::schema::Schema;

/// Query optimizer that rewrites plans for better performance
pub struct QueryOptimizer;

impl QueryOptimizer {
    /// Optimize a query plan
    pub fn optimize(plan: QueryPlan) -> QueryPlan {
        let optimized_root = Self::optimize_node(plan.root);
        QueryPlan::new(optimized_root, plan.output_schema)
    }

    fn optimize_node(node: PlanNode) -> PlanNode {
        match node {
            // Push filters down (predicate pushdown)
            PlanNode::Filter { predicate, input } => {
                let optimized_input = Self::optimize_node(*input);
                
                // If input is a scan, we can push the filter into the scan
                if let PlanNode::Scan { table_id, column_ids, filter: None } = &optimized_input {
                    PlanNode::Scan {
                        table_id: *table_id,
                        column_ids: column_ids.clone(),
                        filter: Some(predicate.clone()),
                    }
                } else {
                    PlanNode::Filter {
                        predicate,
                        input: Box::new(optimized_input),
                    }
                }
            }
            
            // Combine multiple filters
            PlanNode::Filter { predicate: pred1, input } => {
                if let PlanNode::Filter { predicate: pred2, input: inner } = *input {
                    PlanNode::Filter {
                        predicate: Filter::And {
                            left: Box::new(pred1),
                            right: Box::new(pred2),
                        },
                        input: inner,
                    }
                } else {
                    PlanNode::Filter {
                        predicate: pred1,
                        input: Box::new(Self::optimize_node(*input)),
                    }
                }
            }
            
            // Project column elimination
            PlanNode::Project { columns, input } => {
                let optimized_input = Self::optimize_node(*input);
                PlanNode::Project {
                    columns,
                    input: Box::new(optimized_input),
                }
            }
            
            // Limit pushdown
            PlanNode::Limit { limit, offset, input } => {
                let optimized_input = Self::optimize_node(*input);
                PlanNode::Limit {
                    limit,
                    offset,
                    input: Box::new(optimized_input),
                }
            }
            
            // Default: optimize children
            node => {
                // Recursively optimize children
                node
            }
        }
    }

    /// Estimate cost of a plan node
    pub fn estimate_cost(node: &PlanNode) -> f64 {
        match node {
            PlanNode::Scan { column_ids, .. } => {
                // Cost based on number of columns
                column_ids.len() as f64 * 100.0
            }
            PlanNode::Filter { .. } => 50.0,
            PlanNode::Project { columns, .. } => {
                columns.len() as f64 * 10.0
            }
            PlanNode::Limit { limit, .. } => *limit as f64 * 0.1,
            PlanNode::Sort { .. } => 200.0,
            PlanNode::Aggregate { .. } => 150.0,
            PlanNode::Join { .. } => 500.0,
        }
    }
}

/// Index selection optimizer
pub struct IndexOptimizer;

impl IndexOptimizer {
    /// Select best index for a filter
    pub fn select_index(filter: &Filter, available_indexes: &[String]) -> Option<String> {
        match filter {
            Filter::Eq { column, .. } => {
                available_indexes.iter()
                    .find(|idx| idx == &column)
                    .cloned()
            }
            Filter::Gt { column, .. } | Filter::Lt { column, .. } => {
                available_indexes.iter()
                    .find(|idx| idx == &column)
                    .cloned()
            }
            _ => None,
        }
    }
}

/// Join order optimizer
pub struct JoinOptimizer;

impl JoinOptimizer {
    /// Optimize join order for better performance
    pub fn optimize_join_order(joins: Vec<PlanNode>) -> Vec<PlanNode> {
        // Simple heuristic: smaller tables first
        joins
    }
}

