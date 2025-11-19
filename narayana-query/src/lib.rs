pub mod executor;
pub mod plan;
pub mod operators;
pub mod vectorized;
pub mod optimizer;
pub mod hot_path;
pub mod advanced_optimizer;
pub mod materialized_views;
pub mod advanced_analytics;
pub mod ai_analytics;
pub mod ml_integration;
pub mod autocomplete;

pub use executor::QueryExecutor;
pub use plan::{QueryPlan, PlanNode};
pub use optimizer::QueryOptimizer;

