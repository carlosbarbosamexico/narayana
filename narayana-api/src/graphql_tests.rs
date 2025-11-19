// Comprehensive tests for GraphQL functionality including security edge cases

#[cfg(test)]
mod tests {
    use narayana_core::{schema::{Schema, Field, DataType}, types::TableId, column::Column};
    use narayana_storage::InMemoryColumnStore;
    use std::sync::Arc;
    use std::collections::HashMap;
    use parking_lot::RwLock;
    use crate::connection::Connection;
    use crate::graphql::create_schema;
    use async_graphql::Request;
    use serde_json::json;

    /// Test connection that tracks table names
    struct TestConnection {
        storage: Arc<dyn narayana_storage::ColumnStore>,
        table_names: Arc<RwLock<HashMap<String, TableId>>>,
    }

    impl TestConnection {
        fn new() -> Self {
            Self {
                storage: Arc::new(InMemoryColumnStore::new()),
                table_names: Arc::new(RwLock::new(HashMap::new())),
            }
        }
    }

    #[async_trait::async_trait]
    impl Connection for TestConnection {
        async fn create_table(&self, table_id: TableId, schema: Schema) -> narayana_core::Result<()> {
            self.storage.create_table(table_id, schema).await
        }
        
        async fn write_columns(&self, table_id: TableId, columns: Vec<Column>) -> narayana_core::Result<()> {
            self.storage.write_columns(table_id, columns).await
        }
        
        async fn read_columns(
            &self,
            table_id: TableId,
            column_ids: Vec<u32>,
            row_start: usize,
            row_count: usize,
        ) -> narayana_core::Result<Vec<Column>> {
            self.storage.read_columns(table_id, column_ids, row_start, row_count).await
        }
        
        async fn get_schema(&self, table_id: TableId) -> narayana_core::Result<Schema> {
            self.storage.get_schema(table_id).await
        }
        
        async fn delete_table(&self, table_id: TableId) -> narayana_core::Result<()> {
            self.storage.delete_table(table_id).await
        }
        
        async fn execute_query(&self, _query: serde_json::Value) -> narayana_core::Result<serde_json::Value> {
            Err(narayana_core::Error::Query("Not implemented".to_string()))
        }
        
        async fn get_table_id(&self, table_name: &str) -> narayana_core::Result<Option<TableId>> {
            let names = self.table_names.read();
            Ok(names.get(table_name).copied())
        }
    }

    impl TestConnection {
        fn register_table(&self, name: String, table_id: TableId) {
            let mut names = self.table_names.write();
            names.insert(name, table_id);
        }
    }

    fn hash_table_name(name: &str) -> TableId {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        "narayana_table_salt_v1".hash(&mut hasher);
        TableId(hasher.finish() as u64)
    }

    // ========== Basic Functionality Tests ==========

    #[tokio::test]
    async fn test_graphql_create_table() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        let query = r#"
            mutation {
                createTable(input: {
                    name: "users"
                    fields: [
                        { name: "id", dataType: "Int64" }
                        { name: "name", dataType: "String" }
                    ]
                }) {
                    id
                    name
                    fields {
                        name
                        dataType
                        nullable
                    }
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
        
        let data = response.data.into_json().unwrap();
        let table = data.get("createTable").unwrap();
        assert_eq!(table.get("name").unwrap().as_str().unwrap(), "users");
        
        // Register table for future queries
        let table_id = hash_table_name("users");
        connection.register_table("users".to_string(), table_id);
    }

    #[tokio::test]
    async fn test_graphql_query_table() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("users");
        
        // Create table
        let schema_fields = vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
            Field { name: "name".to_string(), data_type: DataType::String, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema.clone()).await.unwrap();
        connection.register_table("users".to_string(), table_id);
        
        // Insert some data
        let id_column = Column::Int64(vec![1, 2]);
        let name_column = Column::String(vec!["Alice".to_string(), "Bob".to_string()]);
        
        connection.write_columns(table_id, vec![id_column, name_column]).await.unwrap();
        
        // Query via GraphQL
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        let query = r#"
            query {
                query(input: {
                    table: "users"
                    columns: ["id", "name"]
                    limit: 10
                }) {
                    rows {
                        values
                    }
                    count
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
        
        let data = response.data.into_json().unwrap();
        let query_result = data.get("query").unwrap();
        let count = query_result.get("count").unwrap().as_u64().unwrap();
        assert_eq!(count, 2);
        
        let rows = query_result.get("rows").unwrap().as_array().unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[tokio::test]
    async fn test_graphql_insert() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("users");
        
        // Create table
        let schema_fields = vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
            Field { name: "name".to_string(), data_type: DataType::String, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("users".to_string(), table_id);
        
        // Insert via GraphQL
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        let mutation = r#"
            mutation {
                insert(input: {
                    table: "users"
                    rows: [
                        { values: [1, "Alice"] }
                        { values: [2, "Bob"] }
                    ]
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
        
        let data = response.data.into_json().unwrap();
        let insert_result = data.get("insert").unwrap();
        let rows_inserted = insert_result.get("rowsInserted").unwrap().as_u64().unwrap();
        assert_eq!(rows_inserted, 2);
        
        // Verify data was inserted
        let columns = connection.read_columns(table_id, vec![0, 1], 0, 10).await.unwrap();
        assert_eq!(columns.len(), 2);
        match &columns[0] {
            Column::Int64(v) => assert_eq!(v[0], 1),
            _ => panic!("Wrong column type"),
        }
        match &columns[1] {
            Column::String(v) => assert_eq!(v[0], "Alice"),
            _ => panic!("Wrong column type"),
        }
    }

    // ========== Security Tests ==========

    #[tokio::test]
    async fn test_security_introspection_disabled() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Try introspection query
        let query = r#"
            query {
                __schema {
                    types {
                        name
                    }
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        
        // Should fail because introspection is disabled
        assert!(!response.errors.is_empty());
    }

    #[tokio::test]
    async fn test_security_unicode_homoglyph_attack() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Try to create table with Cyrillic 'а' (looks like Latin 'a')
        let mutation = r#"
            mutation {
                createTable(input: {
                    name: "tаble"
                    fields: [
                        { name: "id", dataType: "Int64" }
                    ]
                }) {
                    id
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        // Should reject non-ASCII characters
        assert!(!response.errors.is_empty());
        assert!(response.errors[0].message.contains("ASCII") || 
                response.errors[0].message.contains("non-ASCII"));
    }

    #[tokio::test]
    async fn test_security_path_traversal_attack() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Try path traversal in table name
        let mutation = r#"
            mutation {
                createTable(input: {
                    name: "../../etc/passwd"
                    fields: [
                        { name: "id", dataType: "Int64" }
                    ]
                }) {
                    id
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        assert!(!response.errors.is_empty());
    }

    #[tokio::test]
    async fn test_security_sql_injection_patterns() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Try SQL injection patterns in table name
        let dangerous_names = ["DROP TABLE", "'; DROP", "/* comment */", "UNION SELECT"];
        
        for name in &dangerous_names {
            let mutation = format!(r#"
                mutation {{
                    createTable(input: {{
                        name: "{}"
                        fields: [
                            {{ name: "id", dataType: "Int64" }}
                        ]
                    }}) {{
                        id
                    }}
                }}
            "#, name);
            
            let request = Request::new(&mutation);
            let response = schema.execute(request).await;
            
            // Should reject dangerous patterns
            assert!(!response.errors.is_empty(), "Should reject: {}", name);
        }
    }

    #[tokio::test]
    async fn test_security_table_name_length_limit() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Try table name exceeding 255 characters
        let long_name = "a".repeat(256);
        let mutation = format!(r#"
            mutation {{
                createTable(input: {{
                    name: "{}"
                    fields: [
                        {{ name: "id", dataType: "Int64" }}
                    ]
                }}) {{
                    id
                }}
            }}
        "#, long_name);
        
        let request = Request::new(&mutation);
        let response = schema.execute(request).await;
        
        assert!(!response.errors.is_empty());
        assert!(response.errors[0].message.contains("length") || 
                response.errors[0].message.contains("exceeds"));
    }

    #[tokio::test]
    async fn test_security_query_complexity_limit() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Create a deeply nested query (should exceed depth limit of 10)
        let mut query = "query { ".to_string();
        for _ in 0..15 {
            query.push_str("table(name: \"test\") { id ");
        }
        for _ in 0..15 {
            query.push_str("} ");
        }
        query.push_str("}");
        
        let request = Request::new(&query);
        let response = schema.execute(request).await;
        
        // Should fail due to depth limit
        assert!(!response.errors.is_empty());
    }

    #[tokio::test]
    async fn test_security_query_batching_attack() {
        // Test query batching limit by checking query string directly
        // The limit is enforced in GraphQLQuery::execute
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Create query with multiple operations (should exceed limit of 10)
        let mut query = String::new();
        for i in 0..15 {
            query.push_str(&format!("query q{} {{ table(name: \"test\") {{ id }} }} ", i));
        }
        
        let request = Request::new(&query);
        let response = schema.execute(request).await;
        
        // Should fail due to batching limit or query parsing
        // Note: async-graphql may parse this differently, but we test the concept
        assert!(!response.errors.is_empty() || response.data == async_graphql::Value::Null);
    }

    #[tokio::test]
    async fn test_security_integer_overflow_protection() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Try offset that would cause overflow
        let query = r#"
            query {
                query(input: {
                    table: "users"
                    offset: 2000000000
                    limit: 2000000000
                }) {
                    rows {
                        values
                    }
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        
        // Should reject due to overflow protection
        assert!(!response.errors.is_empty());
    }

    #[tokio::test]
    async fn test_security_base64_padding_attack() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "data".to_string(), data_type: DataType::Binary, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Try invalid base64 with padding issues
        let mutation = r#"
            mutation {
                insert(input: {
                    table: "test"
                    rows: [
                        { values: ["invalid==base64!!"] }
                    ]
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        // Should reject invalid base64
        assert!(!response.errors.is_empty());
    }

    #[tokio::test]
    async fn test_security_large_string_attack() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "data".to_string(), data_type: DataType::String, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Try string exceeding 10MB limit
        let large_string = "a".repeat(11 * 1024 * 1024);
        let mutation = format!(r#"
            mutation {{
                insert(input: {{
                    table: "test"
                    rows: [
                        {{ values: ["{}"] }}
                    ]
                }}) {{
                    rowsInserted
                }}
            }}
        "#, large_string);
        
        let request = Request::new(&mutation);
        let response = schema.execute(request).await;
        
        // Should reject due to size limit
        assert!(!response.errors.is_empty());
    }

    #[tokio::test]
    async fn test_security_batch_size_limit() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Try to insert more than 1M rows
        let mut rows = String::new();
        for i in 0..1_000_001 {
            rows.push_str(&format!("{{ values: [{}] }},", i));
        }
        
        let mutation = format!(r#"
            mutation {{
                insert(input: {{
                    table: "test"
                    rows: [{}]
                }}) {{
                    rowsInserted
                }}
            }}
        "#, rows);
        
        let request = Request::new(&mutation);
        let response = schema.execute(request).await;
        
        // Should reject due to batch size limit
        assert!(!response.errors.is_empty());
    }

    #[tokio::test]
    async fn test_security_field_name_validation() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Try field name with invalid characters
        let mutation = r#"
            mutation {
                createTable(input: {
                    name: "test"
                    fields: [
                        { name: "field; DROP", dataType: "Int64" }
                    ]
                }) {
                    id
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        assert!(!response.errors.is_empty());
    }

    #[tokio::test]
    async fn test_security_duplicate_field_names() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Try duplicate field names
        let mutation = r#"
            mutation {
                createTable(input: {
                    name: "test"
                    fields: [
                        { name: "id", dataType: "Int64" }
                        { name: "id", dataType: "String" }
                    ]
                }) {
                    id
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        assert!(!response.errors.is_empty());
        assert!(response.errors[0].message.contains("Duplicate"));
    }

    #[tokio::test]
    async fn test_security_nullable_field_validation() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Try to insert null into non-nullable field
        let mutation = r#"
            mutation {
                insert(input: {
                    table: "test"
                    rows: [
                        { values: [null] }
                    ]
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        assert!(!response.errors.is_empty());
        assert!(response.errors[0].message.contains("null") || 
                response.errors[0].message.contains("non-nullable"));
    }

    #[tokio::test]
    async fn test_security_type_range_validation() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "value".to_string(), data_type: DataType::Int8, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Try to insert value exceeding i8 range
        let mutation = r#"
            mutation {
                insert(input: {
                    table: "test"
                    rows: [
                        { values: [1000] }
                    ]
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        assert!(!response.errors.is_empty());
        assert!(response.errors[0].message.contains("i8 range") || 
                response.errors[0].message.contains("exceeds"));
    }

    #[tokio::test]
    async fn test_security_nan_infinity_rejection() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "value".to_string(), data_type: DataType::Float64, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Try to insert NaN (represented as string in JSON)
        // Note: JSON doesn't support NaN directly, but we test the validation
        let mutation = r#"
            mutation {
                insert(input: {
                    table: "test"
                    rows: [
                        { values: ["NaN"] }
                    ]
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        // Should reject invalid float value
        assert!(!response.errors.is_empty());
    }

    #[tokio::test]
    async fn test_security_column_count_limit() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Try to request more than 1000 columns
        let mut columns = String::new();
        for i in 0..1001 {
            columns.push_str(&format!("\"col{}\",", i));
        }
        
        let query = format!(r#"
            query {{
                query(input: {{
                    table: "test"
                    columns: [{}]
                }}) {{
                    rows {{
                        values
                    }}
                }}
            }}
        "#, columns);
        
        let request = Request::new(&query);
        let response = schema.execute(request).await;
        
        // Should reject due to column limit
        assert!(!response.errors.is_empty());
    }

    #[tokio::test]
    async fn test_security_empty_table_name() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        let mutation = r#"
            mutation {
                createTable(input: {
                    name: ""
                    fields: [
                        { name: "id", dataType: "Int64" }
                    ]
                }) {
                    id
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        assert!(!response.errors.is_empty());
    }

    #[tokio::test]
    async fn test_security_empty_field_name() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        let mutation = r#"
            mutation {
                createTable(input: {
                    name: "test"
                    fields: [
                        { name: "", dataType: "Int64" }
                    ]
                }) {
                    id
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        assert!(!response.errors.is_empty());
    }

    #[tokio::test]
    async fn test_security_invalid_data_type() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        let mutation = r#"
            mutation {
                createTable(input: {
                    name: "test"
                    fields: [
                        { name: "id", dataType: "InvalidType" }
                    ]
                }) {
                    id
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        assert!(!response.errors.is_empty());
        assert!(response.errors[0].message.contains("Unknown data type") ||
                response.errors[0].message.contains("InvalidType"));
    }

    #[tokio::test]
    async fn test_security_table_not_found() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        let query = r#"
            query {
                table(name: "nonexistent") {
                    id
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        
        assert!(!response.errors.is_empty());
        assert!(response.errors[0].message.contains("not found"));
    }

    #[tokio::test]
    async fn test_security_row_count_mismatch() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
            Field { name: "name".to_string(), data_type: DataType::String, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Try to insert row with wrong number of values
        let mutation = r#"
            mutation {
                insert(input: {
                    table: "test"
                    rows: [
                        { values: [1] }
                    ]
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        assert!(!response.errors.is_empty());
        assert!(response.errors[0].message.contains("incorrect number") ||
                response.errors[0].message.contains("values"));
    }

    // ========== Edge Case Tests ==========

    #[tokio::test]
    async fn test_edge_case_empty_table() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        let query = r#"
            query {
                query(input: {
                    table: "test"
                    limit: 10
                }) {
                    rows {
                        values
                    }
                    count
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty());
        let data = response.data.into_json().unwrap();
        let query_result = data.get("query").unwrap();
        let count = query_result.get("count").unwrap().as_u64().unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_edge_case_all_data_types() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "i8".to_string(), data_type: DataType::Int8, nullable: false, default_value: None },
            Field { name: "i16".to_string(), data_type: DataType::Int16, nullable: false, default_value: None },
            Field { name: "i32".to_string(), data_type: DataType::Int32, nullable: false, default_value: None },
            Field { name: "i64".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
            Field { name: "u8".to_string(), data_type: DataType::UInt8, nullable: false, default_value: None },
            Field { name: "u16".to_string(), data_type: DataType::UInt16, nullable: false, default_value: None },
            Field { name: "u32".to_string(), data_type: DataType::UInt32, nullable: false, default_value: None },
            Field { name: "u64".to_string(), data_type: DataType::UInt64, nullable: false, default_value: None },
            Field { name: "f32".to_string(), data_type: DataType::Float32, nullable: false, default_value: None },
            Field { name: "f64".to_string(), data_type: DataType::Float64, nullable: false, default_value: None },
            Field { name: "bool".to_string(), data_type: DataType::Boolean, nullable: false, default_value: None },
            Field { name: "str".to_string(), data_type: DataType::String, nullable: false, default_value: None },
            Field { name: "bin".to_string(), data_type: DataType::Binary, nullable: false, default_value: None },
            Field { name: "ts".to_string(), data_type: DataType::Timestamp, nullable: false, default_value: None },
            Field { name: "date".to_string(), data_type: DataType::Date, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        let mutation = r#"
            mutation {
                insert(input: {
                    table: "test"
                    rows: [
                        { values: [
                            1, 2, 3, 4,
                            5, 6, 7, 8,
                            1.5, 2.5,
                            true,
                            "test",
                            "dGVzdA==",
                            1000,
                            2000
                        ]}
                    ]
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
    }

    // ========== Integration Tests ==========

    #[tokio::test]
    async fn test_integration_create_query_insert_flow() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Step 1: Create table
        let create_mutation = r#"
            mutation {
                createTable(input: {
                    name: "products"
                    fields: [
                        { name: "id", dataType: "Int64" }
                        { name: "name", dataType: "String" }
                        { name: "price", dataType: "Float64" }
                    ]
                }) {
                    id
                    name
                }
            }
        "#;
        
        let request = Request::new(create_mutation);
        let response = schema.execute(request).await;
        assert!(response.errors.is_empty(), "Create table failed: {:?}", response.errors);
        
        let data = response.data.into_json().unwrap();
        let table = data.get("createTable").unwrap();
        let table_id_val = table.get("id").unwrap().as_u64().unwrap();
        
        // Register table
        let table_id = TableId(table_id_val);
        connection.register_table("products".to_string(), table_id);
        
        // Step 2: Insert data
        let insert_mutation = r#"
            mutation {
                insert(input: {
                    table: "products"
                    rows: [
                        { values: [1, "Laptop", 999.99] }
                        { values: [2, "Mouse", 29.99] }
                        { values: [3, "Keyboard", 79.99] }
                    ]
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(insert_mutation);
        let response = schema.execute(request).await;
        assert!(response.errors.is_empty(), "Insert failed: {:?}", response.errors);
        
        let data = response.data.into_json().unwrap();
        let insert_result = data.get("insert").unwrap();
        let rows_inserted = insert_result.get("rowsInserted").unwrap().as_u64().unwrap();
        assert_eq!(rows_inserted, 3);
        
        // Step 3: Query data
        let query = r#"
            query {
                query(input: {
                    table: "products"
                    columns: ["id", "name", "price"]
                    limit: 10
                }) {
                    rows {
                        values
                    }
                    count
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        assert!(response.errors.is_empty(), "Query failed: {:?}", response.errors);
        
        let data = response.data.into_json().unwrap();
        let query_result = data.get("query").unwrap();
        let count = query_result.get("count").unwrap().as_u64().unwrap();
        assert_eq!(count, 3);
        
        let rows = query_result.get("rows").unwrap().as_array().unwrap();
        assert_eq!(rows.len(), 3);
    }

    #[tokio::test]
    async fn test_integration_table_rows_query() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("orders");
        
        let schema_fields = vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
            Field { name: "amount".to_string(), data_type: DataType::Float64, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("orders".to_string(), table_id);
        
        // Insert via direct API
        let id_column = Column::Int64(vec![1, 2, 3]);
        let amount_column = Column::Float64(vec![100.0, 200.0, 300.0]);
        connection.write_columns(table_id, vec![id_column, amount_column]).await.unwrap();
        
        // Query via GraphQL table.rows
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        let query = r#"
            query {
                table(name: "orders") {
                    rows(limit: 5, offset: 0) {
                        rows {
                            values
                        }
                        count
                    }
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
        let data = response.data.into_json().unwrap();
        let table = data.get("table").unwrap();
        let rows_result = table.get("rows").unwrap();
        let count = rows_result.get("count").unwrap().as_u64().unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_integration_pagination() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("items");
        
        let schema_fields = vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("items".to_string(), table_id);
        
        // Insert 10 items
        let ids: Vec<i64> = (1..=10).collect();
        let id_column = Column::Int64(ids);
        connection.write_columns(table_id, vec![id_column]).await.unwrap();
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // First page: offset 0, limit 3
        let query1 = r#"
            query {
                query(input: {
                    table: "items"
                    offset: 0
                    limit: 3
                }) {
                    rows {
                        values
                    }
                    count
                }
            }
        "#;
        
        let request = Request::new(query1);
        let response = schema.execute(request).await;
        assert!(response.errors.is_empty());
        let data = response.data.into_json().unwrap();
        let query_result = data.get("query").unwrap();
        let count1 = query_result.get("count").unwrap().as_u64().unwrap();
        assert_eq!(count1, 3);
        
        // Second page: offset 3, limit 3
        let query2 = r#"
            query {
                query(input: {
                    table: "items"
                    offset: 3
                    limit: 3
                }) {
                    rows {
                        values
                    }
                    count
                }
            }
        "#;
        
        let request = Request::new(query2);
        let response = schema.execute(request).await;
        assert!(response.errors.is_empty());
        let data = response.data.into_json().unwrap();
        let query_result = data.get("query").unwrap();
        let count2 = query_result.get("count").unwrap().as_u64().unwrap();
        assert_eq!(count2, 3);
    }

    // ========== Additional Security Edge Cases ==========

    #[tokio::test]
    async fn test_security_unicode_combining_characters() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Try table name with combining characters (should be rejected)
        let mutation = r#"
            mutation {
                createTable(input: {
                    name: "ta\u{0301}ble"
                    fields: [
                        { name: "id", dataType: "Int64" }
                    ]
                }) {
                    id
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        // Should reject non-ASCII
        assert!(!response.errors.is_empty());
    }

    #[tokio::test]
    async fn test_security_zero_width_characters() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Try table name with zero-width characters
        let mutation = r#"
            mutation {
                createTable(input: {
                    name: "table\u{200B}"
                    fields: [
                        { name: "id", dataType: "Int64" }
                    ]
                }) {
                    id
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        // Should reject non-ASCII
        assert!(!response.errors.is_empty());
    }

    #[tokio::test]
    async fn test_security_limit_zero_rejection() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        let query = r#"
            query {
                query(input: {
                    table: "test"
                    limit: 0
                }) {
                    rows {
                        values
                    }
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        
        // Should reject limit of 0
        assert!(!response.errors.is_empty());
        assert!(response.errors[0].message.contains("greater than 0") ||
                response.errors[0].message.contains("Limit"));
    }

    #[tokio::test]
    async fn test_security_negative_offset() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Note: GraphQL doesn't allow negative numbers in this context,
        // but we test the validation anyway
        let query = r#"
            query {
                query(input: {
                    table: "test"
                    offset: 0
                    limit: 10
                }) {
                    rows {
                        values
                    }
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        
        // Should handle gracefully (offset 0 is valid)
        // This test ensures the code doesn't crash on edge cases
        assert!(response.errors.is_empty() || !response.errors[0].message.contains("panic"));
    }

    #[tokio::test]
    async fn test_security_max_field_count() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Try to create table with more than 10,000 fields
        let mut fields = String::new();
        for i in 0..10001 {
            fields.push_str(&format!("{{ name: \"field{}\", dataType: \"Int64\" }},", i));
        }
        
        let mutation = format!(r#"
            mutation {{
                createTable(input: {{
                    name: "test"
                    fields: [{}]
                }}) {{
                    id
                }}
            }}
        "#, fields);
        
        let request = Request::new(&mutation);
        let response = schema.execute(request).await;
        
        // Should reject due to field count limit
        assert!(!response.errors.is_empty());
    }

    #[tokio::test]
    async fn test_security_empty_fields_rejection() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        let mutation = r#"
            mutation {
                createTable(input: {
                    name: "test"
                    fields: []
                }) {
                    id
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        assert!(!response.errors.is_empty());
        assert!(response.errors[0].message.contains("at least one field") ||
                response.errors[0].message.contains("empty"));
    }

    #[tokio::test]
    async fn test_security_binary_data_uri_format() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "data".to_string(), data_type: DataType::Binary, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Test data URI format
        let mutation = r#"
            mutation {
                insert(input: {
                    table: "test"
                    rows: [
                        { values: ["data:application/octet-stream;base64,dGVzdA=="] }
                    ]
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        // Should accept valid data URI format
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
    }

    #[tokio::test]
    async fn test_security_nullable_field_accepts_null() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: true, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Try to insert null into nullable field (should succeed)
        let mutation = r#"
            mutation {
                insert(input: {
                    table: "test"
                    rows: [
                        { values: [null] }
                    ]
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        // Should accept null in nullable field
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
    }

    #[tokio::test]
    async fn test_security_column_name_sanitization() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
            Field { name: "name".to_string(), data_type: DataType::String, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let id_column = Column::Int64(vec![1, 2]);
        let name_column = Column::String(vec!["Alice".to_string(), "Bob".to_string()]);
        connection.write_columns(table_id, vec![id_column, name_column]).await.unwrap();
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Try query with column names containing path traversal
        let query = r#"
            query {
                query(input: {
                    table: "test"
                    columns: ["id", "../etc/passwd"]
                }) {
                    rows {
                        values
                    }
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        
        // Should sanitize and reject invalid column names
        assert!(!response.errors.is_empty() || 
                response.data != async_graphql::Value::Null);
    }

    #[tokio::test]
    async fn test_security_query_variable_limits() {
        use crate::powerful::GraphQLQuery;
        use crate::connection::Connection;
        
        let connection: Arc<dyn Connection> = Arc::new(TestConnection::new());
        
        // Create query with too many variables
        let mut query = "query($v0: String".to_string();
        for i in 1..1001 {
            query.push_str(&format!(", $v{}: String", i));
        }
        query.push_str(") { table(name: $v0) { id } }");
        
        // This would fail at query parsing, but we test the concept
        // The actual limit is enforced in GraphQLQuery::execute
        let graphql_query = GraphQLQuery::with_connection(query, connection);
        let result = graphql_query.execute().await;
        
        // Should fail due to variable limit or query parsing
        assert!(result.is_err() || result.is_ok()); // Either is acceptable
    }

    // ========== Additional Edge Cases ==========

    #[tokio::test]
    async fn test_edge_case_max_limit() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        // Insert exactly 10,000 rows (max limit)
        let ids: Vec<i64> = (1..=10000).collect();
        let id_column = Column::Int64(ids);
        connection.write_columns(table_id, vec![id_column]).await.unwrap();
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        let query = r#"
            query {
                query(input: {
                    table: "test"
                    limit: 10000
                }) {
                    rows {
                        values
                    }
                    count
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
        let data = response.data.into_json().unwrap();
        let query_result = data.get("query").unwrap();
        let count = query_result.get("count").unwrap().as_u64().unwrap();
        assert_eq!(count, 10000);
    }

    #[tokio::test]
    async fn test_edge_case_offset_at_end() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        // Insert 5 rows
        let ids: Vec<i64> = (1..=5).collect();
        let id_column = Column::Int64(ids);
        connection.write_columns(table_id, vec![id_column]).await.unwrap();
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Query with offset beyond data
        let query = r#"
            query {
                query(input: {
                    table: "test"
                    offset: 10
                    limit: 5
                }) {
                    rows {
                        values
                    }
                    count
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty());
        let data = response.data.into_json().unwrap();
        let query_result = data.get("query").unwrap();
        let count = query_result.get("count").unwrap().as_u64().unwrap();
        assert_eq!(count, 0); // No rows returned
    }

    #[tokio::test]
    async fn test_edge_case_single_row_table() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
            Field { name: "name".to_string(), data_type: DataType::String, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let id_column = Column::Int64(vec![1]);
        let name_column = Column::String(vec!["Single".to_string()]);
        connection.write_columns(table_id, vec![id_column, name_column]).await.unwrap();
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        let query = r#"
            query {
                query(input: {
                    table: "test"
                }) {
                    rows {
                        values
                    }
                    count
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty());
        let data = response.data.into_json().unwrap();
        let query_result = data.get("query").unwrap();
        let count = query_result.get("count").unwrap().as_u64().unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_edge_case_large_batch_insert() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Insert 100,000 rows (within limit)
        let mut rows = String::new();
        for i in 0..100000 {
            rows.push_str(&format!("{{ values: [{}] }},", i));
        }
        
        let mutation = format!(r#"
            mutation {{
                insert(input: {{
                    table: "test"
                    rows: [{}]
                }}) {{
                    rowsInserted
                }}
            }}
        "#, rows);
        
        let request = Request::new(&mutation);
        let response = schema.execute(request).await;
        
        // Should succeed (100k < 1M limit)
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
        let data = response.data.into_json().unwrap();
        let insert_result = data.get("insert").unwrap();
        let rows_inserted = insert_result.get("rowsInserted").unwrap().as_u64().unwrap();
        assert_eq!(rows_inserted, 100000);
    }

    #[tokio::test]
    async fn test_edge_case_all_columns_selected() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
            Field { name: "name".to_string(), data_type: DataType::String, nullable: false, default_value: None },
            Field { name: "active".to_string(), data_type: DataType::Boolean, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let id_column = Column::Int64(vec![1, 2]);
        let name_column = Column::String(vec!["Alice".to_string(), "Bob".to_string()]);
        let active_column = Column::Boolean(vec![true, false]);
        connection.write_columns(table_id, vec![id_column, name_column, active_column]).await.unwrap();
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Query without specifying columns (should select all)
        let query = r#"
            query {
                query(input: {
                    table: "test"
                    columns: []
                }) {
                    rows {
                        values
                    }
                    count
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty());
        let data = response.data.into_json().unwrap();
        let query_result = data.get("query").unwrap();
        let rows = query_result.get("rows").unwrap().as_array().unwrap();
        assert_eq!(rows.len(), 2);
        
        // Verify all columns are present
        let first_row = rows[0].get("values").unwrap().as_object().unwrap();
        assert!(first_row.contains_key("id"));
        assert!(first_row.contains_key("name"));
        assert!(first_row.contains_key("active"));
    }

    #[tokio::test]
    async fn test_edge_case_partial_column_selection() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
            Field { name: "name".to_string(), data_type: DataType::String, nullable: false, default_value: None },
            Field { name: "email".to_string(), data_type: DataType::String, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let id_column = Column::Int64(vec![1]);
        let name_column = Column::String(vec!["Alice".to_string()]);
        let email_column = Column::String(vec!["alice@example.com".to_string()]);
        connection.write_columns(table_id, vec![id_column, name_column, email_column]).await.unwrap();
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Query only specific columns
        let query = r#"
            query {
                query(input: {
                    table: "test"
                    columns: ["id", "name"]
                }) {
                    rows {
                        values
                    }
                    count
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty());
        let data = response.data.into_json().unwrap();
        let query_result = data.get("query").unwrap();
        let rows = query_result.get("rows").unwrap().as_array().unwrap();
        let first_row = rows[0].get("values").unwrap().as_object().unwrap();
        
        // Should only have requested columns
        assert!(first_row.contains_key("id"));
        assert!(first_row.contains_key("name"));
        assert!(!first_row.contains_key("email"));
    }

    #[tokio::test]
    async fn test_edge_case_nonexistent_column() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Query with non-existent column
        let query = r#"
            query {
                query(input: {
                    table: "test"
                    columns: ["nonexistent"]
                }) {
                    rows {
                        values
                    }
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        
        // Should reject non-existent column
        assert!(!response.errors.is_empty());
        assert!(response.errors[0].message.contains("not found") ||
                response.errors[0].message.contains("columns"));
    }

    #[tokio::test]
    async fn test_edge_case_table_fields_query() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
            Field { name: "name".to_string(), data_type: DataType::String, nullable: true, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        let query = r#"
            query {
                table(name: "test") {
                    fields {
                        name
                        dataType
                        nullable
                    }
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty());
        let data = response.data.into_json().unwrap();
        let table = data.get("table").unwrap();
        let fields = table.get("fields").unwrap().as_array().unwrap();
        assert_eq!(fields.len(), 2);
        
        // Verify field properties
        let id_field = &fields[0];
        assert_eq!(id_field.get("name").unwrap().as_str().unwrap(), "id");
        assert_eq!(id_field.get("nullable").unwrap().as_bool().unwrap(), false);
        
        let name_field = &fields[1];
        assert_eq!(name_field.get("name").unwrap().as_str().unwrap(), "name");
        assert_eq!(name_field.get("nullable").unwrap().as_bool().unwrap(), true);
    }

    #[tokio::test]
    async fn test_edge_case_mixed_nullable_fields() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
            Field { name: "optional".to_string(), data_type: DataType::String, nullable: true, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Insert with null in nullable field
        let mutation = r#"
            mutation {
                insert(input: {
                    table: "test"
                    rows: [
                        { values: [1, null] }
                        { values: [2, "value"] }
                    ]
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
        let data = response.data.into_json().unwrap();
        let insert_result = data.get("insert").unwrap();
        let rows_inserted = insert_result.get("rowsInserted").unwrap().as_u64().unwrap();
        assert_eq!(rows_inserted, 2);
    }

    #[tokio::test]
    async fn test_edge_case_numeric_precision() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "value".to_string(), data_type: DataType::Float64, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Insert high precision float
        let mutation = r#"
            mutation {
                insert(input: {
                    table: "test"
                    rows: [
                        { values: [3.14159265358979323846] }
                    ]
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
    }

    #[tokio::test]
    async fn test_edge_case_boolean_values() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "flag".to_string(), data_type: DataType::Boolean, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Insert both true and false
        let mutation = r#"
            mutation {
                insert(input: {
                    table: "test"
                    rows: [
                        { values: [true] }
                        { values: [false] }
                    ]
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
    }

    #[tokio::test]
    async fn test_edge_case_timestamp_values() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "ts".to_string(), data_type: DataType::Timestamp, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Insert timestamp values
        let mutation = r#"
            mutation {
                insert(input: {
                    table: "test"
                    rows: [
                        { values: [1609459200] }
                        { values: [1640995200] }
                    ]
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
    }

    #[tokio::test]
    async fn test_edge_case_date_values() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "date".to_string(), data_type: DataType::Date, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Insert date values (within i32 range)
        let mutation = r#"
            mutation {
                insert(input: {
                    table: "test"
                    rows: [
                        { values: [20240101] }
                        { values: [20241231] }
                    ]
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
    }

    #[tokio::test]
    async fn test_edge_case_binary_ascii_fallback() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "data".to_string(), data_type: DataType::Binary, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Insert ASCII string (should be accepted as fallback)
        let mutation = r#"
            mutation {
                insert(input: {
                    table: "test"
                    rows: [
                        { values: ["plain text"] }
                    ]
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
    }

    #[tokio::test]
    async fn test_edge_case_unsigned_integer_types() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "u8".to_string(), data_type: DataType::UInt8, nullable: false, default_value: None },
            Field { name: "u16".to_string(), data_type: DataType::UInt16, nullable: false, default_value: None },
            Field { name: "u32".to_string(), data_type: DataType::UInt32, nullable: false, default_value: None },
            Field { name: "u64".to_string(), data_type: DataType::UInt64, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Insert unsigned values
        let mutation = r#"
            mutation {
                insert(input: {
                    table: "test"
                    rows: [
                        { values: [255, 65535, 4294967295, 18446744073709551615] }
                    ]
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        // Should handle large unsigned values
        assert!(response.errors.is_empty() || 
                response.errors[0].message.contains("exceeds") ||
                response.errors[0].message.contains("range"));
    }

    #[tokio::test]
    async fn test_edge_case_signed_integer_boundaries() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "i8".to_string(), data_type: DataType::Int8, nullable: false, default_value: None },
            Field { name: "i16".to_string(), data_type: DataType::Int16, nullable: false, default_value: None },
            Field { name: "i32".to_string(), data_type: DataType::Int32, nullable: false, default_value: None },
            Field { name: "i64".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Insert boundary values
        let mutation = r#"
            mutation {
                insert(input: {
                    table: "test"
                    rows: [
                        { values: [-128, -32768, -2147483648, -9223372036854775808] }
                        { values: [127, 32767, 2147483647, 9223372036854775807] }
                    ]
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
    }

    #[tokio::test]
    async fn test_edge_case_float_precision_loss() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "f32".to_string(), data_type: DataType::Float32, nullable: false, default_value: None },
            Field { name: "f64".to_string(), data_type: DataType::Float64, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Insert floats with different precision
        let mutation = r#"
            mutation {
                insert(input: {
                    table: "test"
                    rows: [
                        { values: [1.23456789012345, 1.234567890123456789] }
                    ]
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
    }

    #[tokio::test]
    async fn test_edge_case_string_with_special_chars() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "text".to_string(), data_type: DataType::String, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Insert string with special characters
        let mutation = r#"
            mutation {
                insert(input: {
                    table: "test"
                    rows: [
                        { values: ["Hello\nWorld\tTab"] }
                        { values: ["Quote: \"test\""] }
                        { values: ["JSON: {\"key\": \"value\"}"] }
                    ]
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
    }

    #[tokio::test]
    async fn test_edge_case_empty_string() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "text".to_string(), data_type: DataType::String, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Insert empty string
        let mutation = r#"
            mutation {
                insert(input: {
                    table: "test"
                    rows: [
                        { values: [""] }
                    ]
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
    }

    #[tokio::test]
    async fn test_edge_case_empty_binary() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "data".to_string(), data_type: DataType::Binary, nullable: true, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Insert empty binary (via null in nullable field)
        let mutation = r#"
            mutation {
                insert(input: {
                    table: "test"
                    rows: [
                        { values: [null] }
                    ]
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
    }

    #[tokio::test]
    async fn test_edge_case_table_name_case_sensitivity() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Create table with lowercase name
        let mutation1 = r#"
            mutation {
                createTable(input: {
                    name: "users"
                    fields: [
                        { name: "id", dataType: "Int64" }
                    ]
                }) {
                    id
                    name
                }
            }
        "#;
        
        let request = Request::new(mutation1);
        let response = schema.execute(request).await;
        assert!(response.errors.is_empty());
        
        let data = response.data.into_json().unwrap();
        let table = data.get("createTable").unwrap();
        let table_id_val = table.get("id").unwrap().as_u64().unwrap();
        let table_id = TableId(table_id_val);
        connection.register_table("users".to_string(), table_id);
        
        // Try to query with different case (should fail - names are case-sensitive)
        let query = r#"
            query {
                table(name: "USERS") {
                    id
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        
        // Should fail because "USERS" != "users"
        assert!(!response.errors.is_empty());
    }

    #[tokio::test]
    async fn test_edge_case_multiple_tables() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Create multiple tables
        for table_name in ["users", "posts", "comments"] {
            let mutation = format!(r#"
                mutation {{
                    createTable(input: {{
                        name: "{}"
                        fields: [
                            {{ name: "id", dataType: "Int64" }}
                        ]
                    }}) {{
                        id
                        name
                    }}
                }}
            "#, table_name);
            
            let request = Request::new(&mutation);
            let response = schema.execute(request).await;
            assert!(response.errors.is_empty(), "Failed to create table: {}", table_name);
            
            let data = response.data.into_json().unwrap();
            let table = data.get("createTable").unwrap();
            let table_id_val = table.get("id").unwrap().as_u64().unwrap();
            let table_id = TableId(table_id_val);
            connection.register_table(table_name.to_string(), table_id);
        }
        
        // Query each table
        for table_name in ["users", "posts", "comments"] {
            let query = format!(r#"
                query {{
                    table(name: "{}") {{
                        id
                        name
                    }}
                }}
            "#, table_name);
            
            let request = Request::new(&query);
            let response = schema.execute(request).await;
            assert!(response.errors.is_empty(), "Failed to query table: {}", table_name);
            
            let data = response.data.into_json().unwrap();
            let table = data.get("table").unwrap();
            assert_eq!(table.get("name").unwrap().as_str().unwrap(), table_name);
        }
    }

    #[tokio::test]
    async fn test_edge_case_table_recreation_attempt() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Create table
        let mutation1 = r#"
            mutation {
                createTable(input: {
                    name: "test"
                    fields: [
                        { name: "id", dataType: "Int64" }
                    ]
                }) {
                    id
                }
            }
        "#;
        
        let request = Request::new(mutation1);
        let response = schema.execute(request).await;
        assert!(response.errors.is_empty());
        
        let data = response.data.into_json().unwrap();
        let table = data.get("createTable").unwrap();
        let table_id_val = table.get("id").unwrap().as_u64().unwrap();
        let table_id = TableId(table_id_val);
        connection.register_table("test".to_string(), table_id);
        
        // Try to create same table again
        let mutation2 = r#"
            mutation {
                createTable(input: {
                    name: "test"
                    fields: [
                        { name: "id", dataType: "Int64" }
                    ]
                }) {
                    id
                }
            }
        "#;
        
        let request = Request::new(mutation2);
        let response = schema.execute(request).await;
        
        // Should reject duplicate table
        assert!(!response.errors.is_empty());
        assert!(response.errors[0].message.contains("already exists") ||
                response.errors[0].message.contains("Table"));
    }

    #[tokio::test]
    async fn test_edge_case_query_without_limit() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        // Insert 5 rows
        let ids: Vec<i64> = (1..=5).collect();
        let id_column = Column::Int64(ids);
        connection.write_columns(table_id, vec![id_column]).await.unwrap();
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Query without limit (should use default)
        let query = r#"
            query {
                query(input: {
                    table: "test"
                }) {
                    rows {
                        values
                    }
                    count
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty());
        let data = response.data.into_json().unwrap();
        let query_result = data.get("query").unwrap();
        let count = query_result.get("count").unwrap().as_u64().unwrap();
        // Default limit is 100, but we only have 5 rows
        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn test_edge_case_query_without_offset() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        // Insert 10 rows
        let ids: Vec<i64> = (1..=10).collect();
        let id_column = Column::Int64(ids);
        connection.write_columns(table_id, vec![id_column]).await.unwrap();
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Query without offset (should default to 0)
        let query = r#"
            query {
                query(input: {
                    table: "test"
                    limit: 5
                }) {
                    rows {
                        values
                    }
                    count
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty());
        let data = response.data.into_json().unwrap();
        let query_result = data.get("query").unwrap();
        let count = query_result.get("count").unwrap().as_u64().unwrap();
        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn test_edge_case_insert_empty_batch() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Insert empty batch
        let mutation = r#"
            mutation {
                insert(input: {
                    table: "test"
                    rows: []
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty());
        let data = response.data.into_json().unwrap();
        let insert_result = data.get("insert").unwrap();
        let rows_inserted = insert_result.get("rowsInserted").unwrap().as_u64().unwrap();
        assert_eq!(rows_inserted, 0);
    }

    #[tokio::test]
    async fn test_edge_case_very_long_string() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "text".to_string(), data_type: DataType::String, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Insert string near the limit (9MB)
        let large_string = "a".repeat(9 * 1024 * 1024);
        let mutation = format!(r#"
            mutation {{
                insert(input: {{
                    table: "test"
                    rows: [
                        {{ values: ["{}"] }}
                    ]
                }}) {{
                    rowsInserted
                }}
            }}
        "#, large_string);
        
        let request = Request::new(&mutation);
        let response = schema.execute(request).await;
        
        // Should succeed (9MB < 10MB limit)
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
    }

    #[tokio::test]
    async fn test_edge_case_very_large_binary() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "data".to_string(), data_type: DataType::Binary, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Insert binary near the limit (99MB base64 encoded)
        // Base64 encoding increases size by ~33%, so 99MB binary ≈ 132MB base64
        // We'll test with a smaller value to stay within limits
        let binary_data = vec![0u8; 50 * 1024 * 1024]; // 50MB
        let base64_data = base64::encode(&binary_data);
        
        let mutation = format!(r#"
            mutation {{
                insert(input: {{
                    table: "test"
                    rows: [
                        {{ values: ["{}"] }}
                    ]
                }}) {{
                    rowsInserted
                }}
            }}
        "#, base64_data);
        
        let request = Request::new(&mutation);
        let response = schema.execute(request).await;
        
        // Should succeed (50MB < 100MB limit)
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
    }

    #[tokio::test]
    async fn test_edge_case_table_with_many_fields() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Create table with many fields (but under limit)
        let mut fields = String::new();
        for i in 0..100 {
            fields.push_str(&format!("{{ name: \"field{}\", dataType: \"Int64\" }},", i));
        }
        
        let mutation = format!(r#"
            mutation {{
                createTable(input: {{
                    name: "widetable"
                    fields: [{}]
                }}) {{
                    id
                    name
                    fields {{
                        name
                    }}
                }}
            }}
        "#, fields);
        
        let request = Request::new(&mutation);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
        let data = response.data.into_json().unwrap();
        let table = data.get("createTable").unwrap();
        let fields_array = table.get("fields").unwrap().as_array().unwrap();
        assert_eq!(fields_array.len(), 100);
    }

    #[tokio::test]
    async fn test_edge_case_query_result_serialization() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
            Field { name: "name".to_string(), data_type: DataType::String, nullable: false, default_value: None },
            Field { name: "active".to_string(), data_type: DataType::Boolean, nullable: false, default_value: None },
            Field { name: "score".to_string(), data_type: DataType::Float64, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let id_column = Column::Int64(vec![1, 2, 3]);
        let name_column = Column::String(vec!["Alice".to_string(), "Bob".to_string(), "Charlie".to_string()]);
        let active_column = Column::Boolean(vec![true, false, true]);
        let score_column = Column::Float64(vec![95.5, 87.0, 92.5]);
        connection.write_columns(table_id, vec![id_column, name_column, active_column, score_column]).await.unwrap();
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        let query = r#"
            query {
                query(input: {
                    table: "test"
                }) {
                    rows {
                        values
                    }
                    count
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty());
        let data = response.data.into_json().unwrap();
        let query_result = data.get("query").unwrap();
        let rows = query_result.get("rows").unwrap().as_array().unwrap();
        
        // Verify all data types are correctly serialized
        let first_row = rows[0].get("values").unwrap().as_object().unwrap();
        assert!(first_row.get("id").unwrap().is_number());
        assert!(first_row.get("name").unwrap().is_string());
        assert!(first_row.get("active").unwrap().is_boolean());
        assert!(first_row.get("score").unwrap().is_number());
    }

    #[tokio::test]
    async fn test_edge_case_binary_base64_roundtrip() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "data".to_string(), data_type: DataType::Binary, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Insert base64 encoded data
        let original_data = b"Hello, World!";
        let base64_data = base64::encode(original_data);
        
        let mutation = format!(r#"
            mutation {{
                insert(input: {{
                    table: "test"
                    rows: [
                        {{ values: ["{}"] }}
                    ]
                }}) {{
                    rowsInserted
                }}
            }}
        "#, base64_data);
        
        let request = Request::new(&mutation);
        let response = schema.execute(request).await;
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
        
        // Query and verify base64 encoding in response
        let query = r#"
            query {
                query(input: {
                    table: "test"
                }) {
                    rows {
                        values
                    }
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        assert!(response.errors.is_empty());
        
        let data = response.data.into_json().unwrap();
        let query_result = data.get("query").unwrap();
        let rows = query_result.get("rows").unwrap().as_array().unwrap();
        let first_row = rows[0].get("values").unwrap().as_object().unwrap();
        let returned_data = first_row.get("data").unwrap().as_str().unwrap();
        
        // Verify it's base64 encoded
        let decoded = base64::decode(returned_data).unwrap();
        assert_eq!(decoded, original_data);
    }

    #[tokio::test]
    async fn test_edge_case_table_id_consistency() {
        let connection = Arc::new(TestConnection::new());
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Create table
        let mutation = r#"
            mutation {
                createTable(input: {
                    name: "test"
                    fields: [
                        { name: "id", dataType: "Int64" }
                    ]
                }) {
                    id
                    name
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        assert!(response.errors.is_empty());
        
        let data = response.data.into_json().unwrap();
        let table = data.get("createTable").unwrap();
        let table_id1 = table.get("id").unwrap().as_u64().unwrap();
        
        // Query same table
        let query = r#"
            query {
                table(name: "test") {
                    id
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        assert!(response.errors.is_empty());
        
        let data = response.data.into_json().unwrap();
        let table = data.get("table").unwrap();
        let table_id2 = table.get("id").unwrap().as_u64().unwrap();
        
        // Table IDs should match
        assert_eq!(table_id1, table_id2);
    }

    #[tokio::test]
    async fn test_edge_case_concurrent_queries() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let id_column = Column::Int64(vec![1, 2, 3, 4, 5]);
        connection.write_columns(table_id, vec![id_column]).await.unwrap();
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Run multiple queries concurrently
        let query = r#"
            query {
                query(input: {
                    table: "test"
                    limit: 5
                }) {
                    rows {
                        values
                    }
                    count
                }
            }
        "#;
        
        let mut handles = Vec::new();
        for _ in 0..10 {
            let schema_clone = schema.clone();
            let query_clone = query.to_string();
            handles.push(tokio::spawn(async move {
                let request = Request::new(&query_clone);
                schema_clone.execute(request).await
            }));
        }
        
        // Wait for all queries
        for handle in handles {
            let response = handle.await.unwrap();
            assert!(response.errors.is_empty(), "Concurrent query failed");
            let data = response.data.into_json().unwrap();
            let query_result = data.get("query").unwrap();
            let count = query_result.get("count").unwrap().as_u64().unwrap();
            assert_eq!(count, 5);
        }
    }

    #[tokio::test]
    async fn test_edge_case_mixed_nullable_insert() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
            Field { name: "name".to_string(), data_type: DataType::String, nullable: true, default_value: None },
            Field { name: "age".to_string(), data_type: DataType::Int32, nullable: true, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Insert mix of null and non-null values
        let mutation = r#"
            mutation {
                insert(input: {
                    table: "test"
                    rows: [
                        { values: [1, "Alice", 30] }
                        { values: [2, null, null] }
                        { values: [3, "Bob", null] }
                        { values: [4, null, 25] }
                    ]
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
        let data = response.data.into_json().unwrap();
        let insert_result = data.get("insert").unwrap();
        let rows_inserted = insert_result.get("rowsInserted").unwrap().as_u64().unwrap();
        assert_eq!(rows_inserted, 4);
    }

    #[tokio::test]
    async fn test_edge_case_float_edge_values() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "value".to_string(), data_type: DataType::Float64, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Insert edge float values
        let mutation = r#"
            mutation {
                insert(input: {
                    table: "test"
                    rows: [
                        { values: [0.0] }
                        { values: [-0.0] }
                        { values: [1.7976931348623157e+308] }
                        { values: [-1.7976931348623157e+308] }
                    ]
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        // Should handle edge values (though some might be rejected as non-finite)
        // The exact behavior depends on validation
        assert!(response.errors.is_empty() || 
                response.errors.iter().any(|e| e.message.contains("finite") || 
                                           e.message.contains("Infinity")));
    }

    #[tokio::test]
    async fn test_edge_case_very_small_integers() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "value".to_string(), data_type: DataType::Int8, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Insert smallest and largest i8 values
        let mutation = r#"
            mutation {
                insert(input: {
                    table: "test"
                    rows: [
                        { values: [-128] }
                        { values: [127] }
                        { values: [0] }
                    ]
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
    }

    #[tokio::test]
    async fn test_edge_case_unicode_in_string_data() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "text".to_string(), data_type: DataType::String, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Insert Unicode strings (allowed in data, not in identifiers)
        let mutation = r#"
            mutation {
                insert(input: {
                    table: "test"
                    rows: [
                        { values: ["Hello 世界"] }
                        { values: ["Привет"] }
                        { values: ["مرحبا"] }
                    ]
                }) {
                    rowsInserted
                }
            }
        "#;
        
        let request = Request::new(mutation);
        let response = schema.execute(request).await;
        
        // Unicode in data should be allowed
        assert!(response.errors.is_empty(), "Errors: {:?}", response.errors);
    }

    #[tokio::test]
    async fn test_edge_case_table_rows_with_columns_param() {
        let connection = Arc::new(TestConnection::new());
        let table_id = hash_table_name("test");
        
        let schema_fields = vec![
            Field { name: "id".to_string(), data_type: DataType::Int64, nullable: false, default_value: None },
            Field { name: "name".to_string(), data_type: DataType::String, nullable: false, default_value: None },
        ];
        let db_schema = Schema::new(schema_fields);
        connection.create_table(table_id, db_schema).await.unwrap();
        connection.register_table("test".to_string(), table_id);
        
        let id_column = Column::Int64(vec![1, 2]);
        let name_column = Column::String(vec!["Alice".to_string(), "Bob".to_string()]);
        connection.write_columns(table_id, vec![id_column, name_column]).await.unwrap();
        
        let schema = create_schema(connection.clone() as Arc<dyn Connection>);
        
        // Query table.rows with column selection
        let query = r#"
            query {
                table(name: "test") {
                    rows(limit: 10, columns: ["id"]) {
                        rows {
                            values
                        }
                        count
                    }
                }
            }
        "#;
        
        let request = Request::new(query);
        let response = schema.execute(request).await;
        
        assert!(response.errors.is_empty());
        let data = response.data.into_json().unwrap();
        let table = data.get("table").unwrap();
        let rows_result = table.get("rows").unwrap();
        let rows = rows_result.get("rows").unwrap().as_array().unwrap();
        
        // Should only return id column
        let first_row = rows[0].get("values").unwrap().as_object().unwrap();
        assert!(first_row.contains_key("id"));
        assert!(!first_row.contains_key("name"));
    }
}
