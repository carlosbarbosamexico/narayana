// Authentication and Setup API Tests
// Tests for the authentication system, setup flow, and protected endpoints

use narayana_server::http::ApiState;
use narayana_storage::{
    ColumnStore,
    InMemoryColumnStore,
    database_manager::DatabaseManager,
    human_search::HumanSearchEngine,
    webhooks::WebhookManager,
    workers::WorkerManager,
    cognitive::CognitiveBrain,
    query_learning::QueryLearningEngine,
};
use narayana_server::security::TokenManager;
use axum::{
    body::{Body, HttpBody},
    http::{Request, StatusCode},
    Router,
};
use tower::ServiceExt;
use serde_json::{json, Value};
use std::sync::Arc;

// Helper to create test state
fn create_test_state() -> ApiState {
    let storage: Arc<dyn ColumnStore> = Arc::new(InMemoryColumnStore::new());
    let db_manager = Arc::new(DatabaseManager::new());
    let search_engine = Arc::new(HumanSearchEngine::new());
    let webhook_manager = Arc::new(WebhookManager::new());
    // For tests, we need a runtime - use a simple mock
    use narayana_storage::workers::{WorkerManager, WorkerRuntime, WorkerExecutionContext, WorkerResponse};
    struct MockRuntime;
    #[async_trait::async_trait]
    impl WorkerRuntime for MockRuntime {
        async fn execute(&self, _ctx: WorkerExecutionContext) -> anyhow::Result<WorkerResponse> {
            Ok(WorkerResponse {
                status: 200,
                headers: std::collections::HashMap::new(),
                body: b"Mock response".to_vec(),
                metrics: narayana_storage::workers::ExecutionMetrics {
                    cpu_time_ms: 0,
                    memory_bytes: 0,
                    execution_time_ms: 0,
                    subrequests: 0,
                    request_size: 0,
                    response_size: 0,
                },
            })
        }
        
        fn validate_code(&self, code: &str) -> anyhow::Result<()> {
            if code.is_empty() {
                anyhow::bail!("Code cannot be empty")
            } else {
                Ok(())
            }
        }
        
        fn name(&self) -> &str {
            "MockRuntime"
        }
    }
    let runtime: Arc<dyn WorkerRuntime> = Arc::new(MockRuntime);
    let worker_manager = Arc::new(WorkerManager::new(runtime));
    let brain = Arc::new(CognitiveBrain::new());
    let query_learning = Arc::new(QueryLearningEngine::new());
    let token_manager = Arc::new(TokenManager::new("test_secret_key_for_testing_only".to_string()));

    ApiState {
        storage,
        db_manager,
        search_engine,
        webhook_manager,
        worker_manager,
        brain,
        query_learning,
        ws_state: None,
        token_manager,
    }
}

// Helper to create test router
fn create_test_router(state: ApiState) -> Router {
    narayana_server::http::create_router(state)
}

#[tokio::test]
async fn test_setup_check_no_table() {
    let state = create_test_state();
    let app = create_test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/setup/check")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["setup_required"], true);
}

#[tokio::test]
async fn test_setup_create_first_user() {
    let state = create_test_state();
    let app = create_test_router(state);

    // First, check setup is required
    let check_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/setup/check")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(check_response.status(), StatusCode::OK);

    // Perform setup
    let setup_request = json!({
        "name": "Test Admin",
        "username": "testadmin",
        "password": "testpassword123"
    });

    let setup_response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/setup")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&setup_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(setup_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(setup_response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert!(json["user_id"].is_string());
}

#[tokio::test]
async fn test_setup_duplicate_username() {
    let state = create_test_state();
    let app = create_test_router(state);

    // First setup
    let setup_request1 = json!({
        "name": "Test Admin",
        "username": "testadmin",
        "password": "testpassword123"
    });

    let response1 = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/setup")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&setup_request1).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response1.status(), StatusCode::OK);

    // Try to setup again with same username
    let setup_request2 = json!({
        "name": "Another Admin",
        "username": "testadmin",
        "password": "differentpassword"
    });

    let response2 = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/setup")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&setup_request2).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response2.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_setup_validation() {
    let state = create_test_state();
    let app = create_test_router(state);

    // Test short username
    let request = json!({
        "name": "Test",
        "username": "ab",
        "password": "testpassword123"
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/setup")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Test short password
    let request = json!({
        "name": "Test",
        "username": "testuser",
        "password": "short"
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/setup")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_login_success() {
    let state = create_test_state();
    let app = create_test_router(state);

    // Setup first
    let setup_request = json!({
        "name": "Test Admin",
        "username": "testadmin",
        "password": "testpassword123"
    });

    let setup_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/setup")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&setup_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(setup_response.status(), StatusCode::OK);

    // Now login
    let login_request = json!({
        "username": "testadmin",
        "password": "testpassword123"
    });

    let login_response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/login")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&login_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(login_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(login_response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert!(json["token"].is_string());
}

#[tokio::test]
async fn test_login_invalid_credentials() {
    let state = create_test_state();
    let app = create_test_router(state);

    // Setup first
    let setup_request = json!({
        "name": "Test Admin",
        "username": "testadmin",
        "password": "testpassword123"
    });

    let setup_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/setup")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&setup_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(setup_response.status(), StatusCode::OK);

    // Try login with wrong password
    let login_request = json!({
        "username": "testadmin",
        "password": "wrongpassword"
    });

    let login_response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/login")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&login_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(login_response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_protected_endpoint_without_token() {
    let state = create_test_state();
    let app = create_test_router(state);

    // Try to access protected endpoint without token
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/tables")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_protected_endpoint_with_valid_token() {
    let state = create_test_state();
    let app = create_test_router(state);

    // Setup and login to get token
    let setup_request = json!({
        "name": "Test Admin",
        "username": "testadmin",
        "password": "testpassword123"
    });

    let setup_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/setup")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&setup_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(setup_response.status(), StatusCode::OK);

    let login_request = json!({
        "username": "testadmin",
        "password": "testpassword123"
    });

    let login_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/login")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&login_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(login_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(login_response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let token = json["token"].as_str().unwrap();

    // Now access protected endpoint with token
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/tables")
                .header("authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_protected_endpoint_with_invalid_token() {
    let state = create_test_state();
    let app = create_test_router(state);

    // Try to access protected endpoint with invalid token
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/tables")
                .header("authorization", "Bearer invalid_token_here")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_protected_endpoint_case_insensitive_bearer() {
    let state = create_test_state();
    let app = create_test_router(state);

    // Setup and login
    let setup_request = json!({
        "name": "Test Admin",
        "username": "testadmin",
        "password": "testpassword123"
    });

    let setup_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/setup")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&setup_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(setup_response.status(), StatusCode::OK);

    let login_request = json!({
        "username": "testadmin",
        "password": "testpassword123"
    });

    let login_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/login")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&login_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(login_response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let token = json["token"].as_str().unwrap();

    // Test case-insensitive "bearer"
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/tables")
                .header("authorization", format!("bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

