// Tests for elegant API

use narayana_api::elegant::*;

#[test]
fn test_narayana_creation() {
    let narayana = Narayana::new();
    // Should create builder
}

#[test]
fn test_narayana_builder_url() {
    let builder = Narayana::new()
        .url("http://localhost:8080");
    // Should set URL
}

#[test]
fn test_narayana_builder_timeout() {
    let builder = Narayana::new()
        .timeout(30);
    // Should set timeout
}

#[test]
fn test_narayana_builder_max_connections() {
    let builder = Narayana::new()
        .max_connections(100);
    // Should set max connections
}

#[tokio::test]
async fn test_narayana_builder_build() {
    let narayana = Narayana::new()
        .url("http://localhost:8080")
        .build()
        .await
        .unwrap();
    // Should build successfully
}

#[tokio::test]
async fn test_database_creation() {
    let narayana = Narayana::new().build().await.unwrap();
    let db = narayana.database("test_db");
    assert_eq!(db.name, "test_db");
}

#[tokio::test]
async fn test_database_create_table() {
    let narayana = Narayana::new().build().await.unwrap();
    let db = narayana.database("test_db");
    let builder = db.create_table("users");
    // Should create table builder
}

#[tokio::test]
async fn test_table_builder_field() {
    let narayana = Narayana::new().build().await.unwrap();
    let db = narayana.database("test_db");
    let builder = db.create_table("users")
        .field("id", narayana_core::schema::DataType::Int64);
    // Should add field
}

#[tokio::test]
async fn test_table_builder_int() {
    let narayana = Narayana::new().build().await.unwrap();
    let db = narayana.database("test_db");
    let builder = db.create_table("users")
        .int("id");
    // Should add int field
}

#[tokio::test]
async fn test_table_builder_string() {
    let narayana = Narayana::new().build().await.unwrap();
    let db = narayana.database("test_db");
    let builder = db.create_table("users")
        .string("name");
    // Should add string field
}

#[tokio::test]
async fn test_table_builder_float() {
    let narayana = Narayana::new().build().await.unwrap();
    let db = narayana.database("test_db");
    let builder = db.create_table("users")
        .float("score");
    // Should add float field
}

#[tokio::test]
async fn test_table_builder_bool() {
    let narayana = Narayana::new().build().await.unwrap();
    let db = narayana.database("test_db");
    let builder = db.create_table("users")
        .bool("active");
    // Should add bool field
}

#[tokio::test]
async fn test_table_builder_timestamp() {
    let narayana = Narayana::new().build().await.unwrap();
    let db = narayana.database("test_db");
    let builder = db.create_table("users")
        .timestamp("created_at");
    // Should add timestamp field
}

#[tokio::test]
async fn test_table_builder_nullable() {
    let narayana = Narayana::new().build().await.unwrap();
    let db = narayana.database("test_db");
    let builder = db.create_table("users")
        .int_nullable("optional_id")
        .string_nullable("optional_name");
    // Should add nullable fields
}

#[tokio::test]
async fn test_table_query() {
    let narayana = Narayana::new().build().await.unwrap();
    let db = narayana.database("test_db");
    let table = db.table("users");
    let query = table.query();
    // Should create query builder
}

#[tokio::test]
async fn test_query_builder_select() {
    let narayana = Narayana::new().build().await.unwrap();
    let db = narayana.database("test_db");
    let table = db.table("users");
    let query = table.query()
        .select(&["id", "name"]);
    // Should set select columns
}

#[tokio::test]
async fn test_query_builder_where() {
    let narayana = Narayana::new().build().await.unwrap();
    let db = narayana.database("test_db");
    let table = db.table("users");
    let filter = table.query()
        .where_clause("id");
    // Should create filter builder
}

#[tokio::test]
async fn test_query_builder_order_by() {
    let narayana = Narayana::new().build().await.unwrap();
    let db = narayana.database("test_db");
    let table = db.table("users");
    let query = table.query()
        .order_by("id");
    // Should create order by builder
}

#[tokio::test]
async fn test_query_builder_limit() {
    let narayana = Narayana::new().build().await.unwrap();
    let db = narayana.database("test_db");
    let table = db.table("users");
    let query = table.query()
        .limit(10);
    // Should set limit
}

#[tokio::test]
async fn test_table_insert() {
    let narayana = Narayana::new().build().await.unwrap();
    let db = narayana.database("test_db");
    let table = db.table("users");
    let insert = table.insert();
    // Should create insert builder
}

#[tokio::test]
async fn test_table_update() {
    let narayana = Narayana::new().build().await.unwrap();
    let db = narayana.database("test_db");
    let table = db.table("users");
    let update = table.update();
    // Should create update builder
}

#[tokio::test]
async fn test_table_delete() {
    let narayana = Narayana::new().build().await.unwrap();
    let db = narayana.database("test_db");
    let table = db.table("users");
    let delete = table.delete();
    // Should create delete builder
}

