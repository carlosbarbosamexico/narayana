// Tests for powerful API

use narayana_api::powerful::*;
use narayana_api::elegant::*;
use tokio_test;

#[test]
fn test_powerful_builder_creation() {
    let builder = NarayanaPowerful::new();
    // Should create successfully
}

#[test]
fn test_powerful_builder_url() {
    let builder = NarayanaPowerful::new()
        .url("http://localhost:8080");
    // Should set URL
}

#[test]
fn test_powerful_builder_enable_features() {
    let builder = NarayanaPowerful::new()
        .enable_graphql()
        .enable_reactive()
        .enable_subscriptions();
    // Should enable features
}

#[test]
fn test_graphql_query_creation() {
    let client = NarayanaPowerful::new();
    let query = client.graphql("query { users { id } }");
    // Should create query
}

#[test]
fn test_graphql_query_variable() {
    let client = NarayanaPowerful::new();
    let query = client.graphql("query { users(id: $id) { id } }")
        .variable("id", serde_json::json!(1));
    // Should add variable
}

#[test]
fn test_batch_operations_creation() {
    let client = NarayanaPowerful::new();
    let batch = client.batch();
    // Should create batch
}

#[test]
fn test_batch_operations_insert() {
    let client = NarayanaPowerful::new();
    let batch = client.batch()
        .insert("users", vec![]);
    // Should add insert operation
}

#[test]
fn test_pipeline_creation() {
    let client = NarayanaPowerful::new();
    let pipeline = client.pipeline();
    // Should create pipeline
}

#[tokio::test]
async fn test_pipeline_query() {
    let client = NarayanaPowerful::new().build().await.unwrap();
    let pipeline = client.pipeline();
    // Should create pipeline
}

#[test]
fn test_advanced_query_builder_creation() {
    let builder = AdvancedQueryBuilder::new("users".to_string());
    assert_eq!(builder.table, "users");
}

#[test]
fn test_advanced_query_builder_select() {
    let builder = AdvancedQueryBuilder::new("users".to_string())
        .select(&["id", "name"]);
    assert_eq!(builder.select.len(), 2);
}

#[test]
fn test_advanced_query_builder_distinct() {
    let builder = AdvancedQueryBuilder::new("users".to_string())
        .distinct();
    assert!(builder.distinct);
}

#[test]
fn test_advanced_query_builder_join() {
    let builder = AdvancedQueryBuilder::new("users".to_string())
        .join("profiles", "users.id = profiles.user_id");
    assert_eq!(builder.joins.len(), 1);
}

#[test]
fn test_advanced_query_builder_group_by() {
    let builder = AdvancedQueryBuilder::new("users".to_string())
        .group_by(&["status"]);
    assert_eq!(builder.group_by.len(), 1);
}

#[test]
fn test_composable_query_creation() {
    let query = ComposableQuery::new();
    // Should create successfully
}

#[test]
fn test_composable_query_select() {
    let query = ComposableQuery::new()
        .select(&["id", "name"]);
    // Should add select
}

#[test]
fn test_composable_query_from() {
    let query = ComposableQuery::new()
        .from("users");
    // Should add from
}

#[test]
fn test_bulk_operations_creation() {
    let bulk = BulkOperations::new();
    // Should create successfully
}

#[test]
fn test_bulk_operations_insert() {
    let bulk = BulkOperations::new()
        .insert("users", vec![]);
    // Should add insert
}

use tokio_test;

#[tokio::test]
async fn test_subscription_creation() {
    let client = NarayanaPowerful::new();
    let subscription = client.subscribe("events");
    // Should create subscription
}

#[tokio::test]
async fn test_subscription_filter() {
    let client = NarayanaPowerful::new();
    let subscription = client.subscribe("events")
        .filter("status", "eq", serde_json::json!("active"));
    // Should add filter
}

