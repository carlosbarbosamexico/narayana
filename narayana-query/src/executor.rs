use async_trait::async_trait;
use narayana_core::{Error, Result, column::Column, schema::Schema, types::TableId};
use narayana_storage::ColumnStore;
use crate::plan::{QueryPlan, PlanNode, Filter};
use crate::operators::{FilterOperator, ProjectOperator};
use tracing::{info, debug};

#[async_trait]
pub trait QueryExecutor: Send + Sync {
    async fn execute(&self, plan: QueryPlan) -> Result<Vec<Column>>;
}

pub struct DefaultQueryExecutor<S: ColumnStore> {
    pub store: S,
}

impl<S: ColumnStore> DefaultQueryExecutor<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }
}

#[async_trait]
impl<S: ColumnStore> QueryExecutor for DefaultQueryExecutor<S> {
    async fn execute(&self, plan: QueryPlan) -> Result<Vec<Column>> {
        info!("Executing query plan");
        self.execute_node(&plan.root, TableId(0)).await
    }
}

impl<S: ColumnStore> DefaultQueryExecutor<S> {
    fn execute_node<'a>(&'a self, node: &'a PlanNode, table_id: TableId) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<Column>>> + Send + 'a>> {
        let self_ref = self;
        let node_ref = node;
        Box::pin(async move {
            match node_ref {
            PlanNode::Scan { table_id, column_ids, filter: _ } => {
                debug!("Executing scan on table {} for columns {:?}", table_id, column_ids);
                let columns = self_ref.store
                    .read_columns(narayana_core::types::TableId(*table_id), column_ids.clone(), 0, usize::MAX)
                    .await?;
                Ok(columns)
            }
            PlanNode::Filter { predicate, input } => {
                debug!("Executing filter");
                // Recursive call - need to box it
                let input_columns = Self::execute_node(self_ref, input, table_id).await?;
                let schema = self_ref.store.get_schema(table_id).await?;
                let filter_op = FilterOperator::new(predicate.clone(), schema);
                filter_op.apply(&input_columns)
            }
            PlanNode::Project { columns, input } => {
                debug!("Executing project on columns {:?}", columns);
                let input_columns = Self::execute_node(self_ref, input, table_id).await?;
                let schema = self_ref.store.get_schema(table_id).await?;
                let project_op = ProjectOperator::new(columns.clone(), schema)?;
                Ok(project_op.apply(&input_columns))
            }
            PlanNode::Limit { limit, offset: _, input } => {
                debug!("Executing limit: {}", limit);
                let mut columns = Self::execute_node(self_ref, input, table_id).await?;
                // Apply limit to all columns
                for col in &mut columns {
                    match col {
                        Column::Int32(data) => {
                            data.truncate(*limit);
                        }
                        Column::Int64(data) => {
                            data.truncate(*limit);
                        }
                        Column::UInt64(data) => {
                            data.truncate(*limit);
                        }
                        Column::Float64(data) => {
                            data.truncate(*limit);
                        }
                        Column::String(data) => {
                            data.truncate(*limit);
                        }
                        Column::Boolean(data) => {
                            data.truncate(*limit);
                        }
                        _ => {}
                    }
                }
                Ok(columns)
            }
            _ => Err(Error::Query("Unsupported plan node".to_string())),
            }
        })
    }
}

