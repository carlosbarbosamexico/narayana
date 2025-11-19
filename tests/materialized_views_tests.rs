// Tests for materialized views

use narayana_query::materialized_views::*;
use narayana_core::types::TableId;
use narayana_query::plan::QueryPlan;

#[test]
fn test_materialized_view_manager_creation() {
    let manager = MaterializedViewManager::new();
    // Should create successfully
}

#[test]
fn test_create_view() {
    let manager = MaterializedViewManager::new();
    let view_id = TableId(1);
    let query_plan = QueryPlan::new(
        narayana_query::plan::PlanNode::Scan {
            table_id: 1,
            column_ids: vec![0],
            filter: None,
        },
        narayana_core::schema::Schema::new(vec![]),
    );
    
    manager.create_view(
        "test_view".to_string(),
        view_id,
        query_plan,
        vec![TableId(1)],
        RefreshStrategy::Manual,
    ).unwrap();
}

#[test]
fn test_create_duplicate_view() {
    let manager = MaterializedViewManager::new();
    let view_id = TableId(1);
    let query_plan = QueryPlan::new(
        narayana_query::plan::PlanNode::Scan {
            table_id: 1,
            column_ids: vec![0],
            filter: None,
        },
        narayana_core::schema::Schema::new(vec![]),
    );
    
    manager.create_view(
        "test_view".to_string(),
        view_id,
        query_plan.clone(),
        vec![TableId(1)],
        RefreshStrategy::Manual,
    ).unwrap();
    
    let result = manager.create_view(
        "test_view".to_string(),
        view_id,
        query_plan,
        vec![TableId(1)],
        RefreshStrategy::Manual,
    );
    assert!(result.is_err());
}

#[tokio::test]
async fn test_refresh_view() {
    let manager = MaterializedViewManager::new();
    let view_id = TableId(1);
    let query_plan = QueryPlan::new(
        narayana_query::plan::PlanNode::Scan {
            table_id: 1,
            column_ids: vec![0],
            filter: None,
        },
        narayana_core::schema::Schema::new(vec![]),
    );
    
    manager.create_view(
        "test_view".to_string(),
        view_id,
        query_plan,
        vec![TableId(1)],
        RefreshStrategy::Manual,
    ).unwrap();
    
    manager.refresh_view("test_view").await.unwrap();
}

#[test]
fn test_list_views() {
    let manager = MaterializedViewManager::new();
    let view_id = TableId(1);
    let query_plan = QueryPlan::new(
        narayana_query::plan::PlanNode::Scan {
            table_id: 1,
            column_ids: vec![0],
            filter: None,
        },
        narayana_core::schema::Schema::new(vec![]),
    );
    
    manager.create_view(
        "view1".to_string(),
        view_id,
        query_plan.clone(),
        vec![TableId(1)],
        RefreshStrategy::Manual,
    ).unwrap();
    
    let views = manager.list_views();
    assert_eq!(views.len(), 1);
}

#[test]
fn test_drop_view() {
    let manager = MaterializedViewManager::new();
    let view_id = TableId(1);
    let query_plan = QueryPlan::new(
        narayana_query::plan::PlanNode::Scan {
            table_id: 1,
            column_ids: vec![0],
            filter: None,
        },
        narayana_core::schema::Schema::new(vec![]),
    );
    
    manager.create_view(
        "test_view".to_string(),
        view_id,
        query_plan,
        vec![TableId(1)],
        RefreshStrategy::Manual,
    ).unwrap();
    
    manager.drop_view("test_view").unwrap();
    let views = manager.list_views();
    assert_eq!(views.len(), 0);
}

