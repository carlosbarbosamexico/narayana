// Tests for workers system - Cloudflare Workers-style edge computing

use narayana_storage::workers::*;
use narayana_storage::column_store::InMemoryColumnStore;
use narayana_storage::database_manager::DatabaseManager;
use narayana_storage::ColumnStore;
use std::collections::HashMap;
use std::sync::Arc;

#[test]
fn test_worker_environment_creation() {
    let env = WorkerEnvironment {
        id: "test-worker".to_string(),
        name: "Test Worker".to_string(),
        code: "export default { fetch: () => new Response('Hello') }".to_string(),
        route: "/test/*".to_string(),
        bindings: HashMap::new(),
        limits: WorkerLimits::default(),
        regions: Vec::new(),
        created_at: 0,
        updated_at: 0,
        version: 1,
        active: true,
    };
    
    assert_eq!(env.id, "test-worker");
    assert_eq!(env.name, "Test Worker");
    assert!(env.active);
}

#[test]
fn test_worker_limits_default() {
    let limits = WorkerLimits::default();
    assert_eq!(limits.cpu_time_ms, 50);
    assert_eq!(limits.memory_bytes, 128 * 1024 * 1024);
    assert_eq!(limits.timeout_ms, 30000);
    assert_eq!(limits.max_subrequests, 50);
}

#[test]
fn test_worker_limits_custom() {
    let limits = WorkerLimits {
        cpu_time_ms: 100,
        memory_bytes: 256 * 1024 * 1024,
        timeout_ms: 60000,
        max_subrequests: 100,
        max_request_size: 50 * 1024 * 1024,
        max_response_size: 50 * 1024 * 1024,
    };
    
    assert_eq!(limits.cpu_time_ms, 100);
    assert_eq!(limits.memory_bytes, 256 * 1024 * 1024);
    assert_eq!(limits.timeout_ms, 60000);
}

#[test]
fn test_binding_value_env_var() {
    let binding = BindingValue::EnvVar {
        value: "test-value".to_string(),
    };
    
    match binding {
        BindingValue::EnvVar { value } => assert_eq!(value, "test-value"),
        _ => panic!("Expected EnvVar binding"),
    }
}

#[test]
fn test_binding_value_database() {
    let binding = BindingValue::Database {
        name: "myapp".to_string(),
        database: "production".to_string(),
    };
    
    match binding {
        BindingValue::Database { name, database } => {
            assert_eq!(name, "myapp");
            assert_eq!(database, "production");
        }
        _ => panic!("Expected Database binding"),
    }
}

#[test]
fn test_binding_value_kv_store() {
    let binding = BindingValue::KvStore {
        name: "my-kv".to_string(),
    };
    
    match binding {
        BindingValue::KvStore { name } => assert_eq!(name, "my-kv"),
        _ => panic!("Expected KvStore binding"),
    }
}

#[test]
fn test_binding_value_service() {
    let binding = BindingValue::Service {
        name: "auth-service".to_string(),
        url: "https://auth.example.com".to_string(),
    };
    
    match binding {
        BindingValue::Service { name, url } => {
            assert_eq!(name, "auth-service");
            assert_eq!(url, "https://auth.example.com");
        }
        _ => panic!("Expected Service binding"),
    }
}

#[test]
fn test_worker_request_creation() {
    let request = WorkerRequest {
        method: "GET".to_string(),
        url: "/test".to_string(),
        headers: HashMap::new(),
        body: None,
        query: HashMap::new(),
        client_ip: Some("127.0.0.1".to_string()),
        request_id: "req-123".to_string(),
        worker_id: "worker-123".to_string(),
        edge_location: Some("us-east-1".to_string()),
    };
    
    assert_eq!(request.method, "GET");
    assert_eq!(request.url, "/test");
    assert_eq!(request.worker_id, "worker-123");
}

#[test]
fn test_worker_response_creation() {
    let metrics = ExecutionMetrics {
        cpu_time_ms: 10,
        memory_bytes: 1024,
        execution_time_ms: 15,
        subrequests: 2,
        request_size: 100,
        response_size: 200,
    };
    
    let response = WorkerResponse {
        status: 200,
        headers: HashMap::new(),
        body: b"Hello".to_vec(),
        metrics,
    };
    
    assert_eq!(response.status, 200);
    assert_eq!(response.body, b"Hello");
    assert_eq!(response.metrics.cpu_time_ms, 10);
}

#[test]
fn test_execution_metrics() {
    let metrics = ExecutionMetrics {
        cpu_time_ms: 5,
        memory_bytes: 512,
        execution_time_ms: 10,
        subrequests: 1,
        request_size: 50,
        response_size: 100,
    };
    
    assert_eq!(metrics.cpu_time_ms, 5);
    assert_eq!(metrics.memory_bytes, 512);
    assert_eq!(metrics.execution_time_ms, 10);
}

#[tokio::test]
async fn test_mock_javascript_runtime_validate_code() {
    let runtime = MockJavaScriptRuntime;
    
    // Valid code
    assert!(runtime.validate_code("export default { fetch: () => new Response('Hello') }").is_ok());
    
    // Empty code should fail
    assert!(runtime.validate_code("").is_err());
    
    // Whitespace only should fail
    assert!(runtime.validate_code("   ").is_err());
}

#[tokio::test]
async fn test_mock_javascript_runtime_execute() {
    let runtime = MockJavaScriptRuntime;
    let storage = Arc::new(InMemoryColumnStore::new());
    let db_manager = Arc::new(DatabaseManager::new());
    
    let env = WorkerEnvironment {
        id: "test-worker".to_string(),
        name: "Test Worker".to_string(),
        code: "export default { fetch: () => new Response('Hello') }".to_string(),
        route: "/test/*".to_string(),
        bindings: HashMap::new(),
        limits: WorkerLimits::default(),
        regions: Vec::new(),
        created_at: 0,
        updated_at: 0,
        version: 1,
        active: true,
    };
    
    let request = WorkerRequest {
        method: "GET".to_string(),
        url: "/test".to_string(),
        headers: HashMap::new(),
        body: None,
        query: HashMap::new(),
        client_ip: None,
        request_id: "req-123".to_string(),
        worker_id: "worker-123".to_string(),
        edge_location: None,
    };
    
    let ctx = WorkerExecutionContext::new(env, request, storage, db_manager);
    let response = runtime.execute(ctx).await.unwrap();
    
    assert_eq!(response.status, 200);
    assert!(!response.body.is_empty());
    assert_eq!(response.metrics.execution_time_ms, 0); // May be 0 for fast execution
}

#[tokio::test]
async fn test_worker_manager_creation() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    // Should create successfully
    assert_eq!(manager.list_workers(None).len(), 0);
}

#[tokio::test]
async fn test_worker_manager_deploy_worker() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    let worker_id = manager.deploy_worker(
        "test-worker".to_string(),
        "export default { fetch: () => new Response('Hello') }".to_string(),
        "/test/*".to_string(),
        HashMap::new(),
        None,
        Vec::new(),
    ).await.unwrap();
    
    assert!(!worker_id.is_empty());
    
    let worker = manager.get_worker(&worker_id).unwrap();
    assert_eq!(worker.name, "test-worker");
    assert_eq!(worker.route, "/test/*");
    assert!(worker.active);
}

#[tokio::test]
async fn test_worker_manager_deploy_worker_with_bindings() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    let mut bindings = HashMap::new();
    bindings.insert("DB".to_string(), BindingValue::Database {
        name: "myapp".to_string(),
        database: "production".to_string(),
    });
    bindings.insert("API_KEY".to_string(), BindingValue::EnvVar {
        value: "secret-key".to_string(),
    });
    
    let worker_id = manager.deploy_worker(
        "worker-with-bindings".to_string(),
        "export default { fetch: () => new Response('Hello') }".to_string(),
        "/api/*".to_string(),
        bindings.clone(),
        None,
        Vec::new(),
    ).await.unwrap();
    
    let worker = manager.get_worker(&worker_id).unwrap();
    assert_eq!(worker.bindings.len(), 2);
    assert!(worker.bindings.contains_key("DB"));
    assert!(worker.bindings.contains_key("API_KEY"));
}

#[tokio::test]
async fn test_worker_manager_deploy_worker_with_limits() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    let limits = WorkerLimits {
        cpu_time_ms: 100,
        memory_bytes: 256 * 1024 * 1024,
        timeout_ms: 60000,
        max_subrequests: 100,
        max_request_size: 50 * 1024 * 1024,
        max_response_size: 50 * 1024 * 1024,
    };
    
    let worker_id = manager.deploy_worker(
        "limited-worker".to_string(),
        "export default { fetch: () => new Response('Hello') }".to_string(),
        "/limited/*".to_string(),
        HashMap::new(),
        Some(limits.clone()),
        Vec::new(),
    ).await.unwrap();
    
    let worker = manager.get_worker(&worker_id).unwrap();
    assert_eq!(worker.limits.cpu_time_ms, 100);
    assert_eq!(worker.limits.memory_bytes, 256 * 1024 * 1024);
    assert_eq!(worker.limits.timeout_ms, 60000);
}

#[tokio::test]
async fn test_worker_manager_deploy_worker_with_regions() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    let regions = vec!["us-east-1".to_string(), "eu-west-1".to_string()];
    
    let worker_id = manager.deploy_worker(
        "regional-worker".to_string(),
        "export default { fetch: () => new Response('Hello') }".to_string(),
        "/regional/*".to_string(),
        HashMap::new(),
        None,
        regions.clone(),
    ).await.unwrap();
    
    let worker = manager.get_worker(&worker_id).unwrap();
    assert_eq!(worker.regions.len(), 2);
    assert!(worker.regions.contains(&"us-east-1".to_string()));
    assert!(worker.regions.contains(&"eu-west-1".to_string()));
}

#[tokio::test]
async fn test_worker_manager_deploy_invalid_code() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    let result = manager.deploy_worker(
        "invalid-worker".to_string(),
        "".to_string(), // Empty code should fail
        "/invalid/*".to_string(),
        HashMap::new(),
        None,
        Vec::new(),
    ).await;
    
    assert!(result.is_err());
}

#[tokio::test]
async fn test_worker_manager_deploy_invalid_route() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    let result = manager.deploy_worker(
        "invalid-route-worker".to_string(),
        "export default { fetch: () => new Response('Hello') }".to_string(),
        "".to_string(), // Empty route should fail
        HashMap::new(),
        None,
        Vec::new(),
    ).await;
    
    assert!(result.is_err());
}

#[tokio::test]
async fn test_worker_manager_update_worker() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    let worker_id = manager.deploy_worker(
        "update-worker".to_string(),
        "export default { fetch: () => new Response('Hello') }".to_string(),
        "/update/*".to_string(),
        HashMap::new(),
        None,
        Vec::new(),
    ).await.unwrap();
    
    let updated_code = "export default { fetch: () => new Response('Updated') }".to_string();
    
    manager.update_worker(
        &worker_id,
        Some(updated_code.clone()),
        None,
        None,
        None,
        None,
    ).await.unwrap();
    
    let worker = manager.get_worker(&worker_id).unwrap();
    assert_eq!(worker.code, updated_code);
    assert_eq!(worker.version, 2); // Version should increment
}

#[tokio::test]
async fn test_worker_manager_update_worker_route() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    let worker_id = manager.deploy_worker(
        "route-worker".to_string(),
        "export default { fetch: () => new Response('Hello') }".to_string(),
        "/old/*".to_string(),
        HashMap::new(),
        None,
        Vec::new(),
    ).await.unwrap();
    
    manager.update_worker(
        &worker_id,
        None,
        Some("/new/*".to_string()),
        None,
        None,
        None,
    ).await.unwrap();
    
    let worker = manager.get_worker(&worker_id).unwrap();
    assert_eq!(worker.route, "/new/*");
}

#[tokio::test]
async fn test_worker_manager_update_worker_bindings() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    let worker_id = manager.deploy_worker(
        "bindings-worker".to_string(),
        "export default { fetch: () => new Response('Hello') }".to_string(),
        "/bindings/*".to_string(),
        HashMap::new(),
        None,
        Vec::new(),
    ).await.unwrap();
    
    let mut new_bindings = HashMap::new();
    new_bindings.insert("NEW_KEY".to_string(), BindingValue::EnvVar {
        value: "new-value".to_string(),
    });
    
    manager.update_worker(
        &worker_id,
        None,
        None,
        Some(new_bindings.clone()),
        None,
        None,
    ).await.unwrap();
    
    let worker = manager.get_worker(&worker_id).unwrap();
    assert_eq!(worker.bindings.len(), 1);
    assert!(worker.bindings.contains_key("NEW_KEY"));
}

#[tokio::test]
async fn test_worker_manager_update_worker_limits() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    let worker_id = manager.deploy_worker(
        "limits-worker".to_string(),
        "export default { fetch: () => new Response('Hello') }".to_string(),
        "/limits/*".to_string(),
        HashMap::new(),
        None,
        Vec::new(),
    ).await.unwrap();
    
    let new_limits = WorkerLimits {
        cpu_time_ms: 200,
        memory_bytes: 512 * 1024 * 1024,
        timeout_ms: 120000,
        max_subrequests: 200,
        max_request_size: 100 * 1024 * 1024,
        max_response_size: 100 * 1024 * 1024,
    };
    
    manager.update_worker(
        &worker_id,
        None,
        None,
        None,
        Some(new_limits.clone()),
        None,
    ).await.unwrap();
    
    let worker = manager.get_worker(&worker_id).unwrap();
    assert_eq!(worker.limits.cpu_time_ms, 200);
    assert_eq!(worker.limits.memory_bytes, 512 * 1024 * 1024);
}

#[tokio::test]
async fn test_worker_manager_update_worker_regions() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    let worker_id = manager.deploy_worker(
        "regions-worker".to_string(),
        "export default { fetch: () => new Response('Hello') }".to_string(),
        "/regions/*".to_string(),
        HashMap::new(),
        None,
        vec!["us-east-1".to_string()],
    ).await.unwrap();
    
    let new_regions = vec!["eu-west-1".to_string(), "ap-southeast-1".to_string()];
    
    manager.update_worker(
        &worker_id,
        None,
        None,
        None,
        None,
        Some(new_regions.clone()),
    ).await.unwrap();
    
    let worker = manager.get_worker(&worker_id).unwrap();
    assert_eq!(worker.regions.len(), 2);
    assert!(worker.regions.contains(&"eu-west-1".to_string()));
    assert!(worker.regions.contains(&"ap-southeast-1".to_string()));
}

#[tokio::test]
async fn test_worker_manager_update_nonexistent_worker() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    let result = manager.update_worker(
        "nonexistent-worker",
        None,
        None,
        None,
        None,
        None,
    ).await;
    
    assert!(result.is_err());
}

#[tokio::test]
async fn test_worker_manager_update_invalid_code() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    let worker_id = manager.deploy_worker(
        "invalid-code-worker".to_string(),
        "export default { fetch: () => new Response('Hello') }".to_string(),
        "/invalid/*".to_string(),
        HashMap::new(),
        None,
        Vec::new(),
    ).await.unwrap();
    
    let result = manager.update_worker(
        &worker_id,
        Some("".to_string()), // Empty code should fail
        None,
        None,
        None,
        None,
    ).await;
    
    assert!(result.is_err());
}

#[tokio::test]
async fn test_worker_manager_delete_worker() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    let worker_id = manager.deploy_worker(
        "delete-worker".to_string(),
        "export default { fetch: () => new Response('Hello') }".to_string(),
        "/delete/*".to_string(),
        HashMap::new(),
        None,
        Vec::new(),
    ).await.unwrap();
    
    manager.delete_worker(&worker_id).await.unwrap();
    
    assert!(manager.get_worker(&worker_id).is_none());
}

#[tokio::test]
async fn test_worker_manager_delete_nonexistent_worker() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    let result = manager.delete_worker("nonexistent-worker").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_worker_manager_list_workers() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    manager.deploy_worker(
        "worker-1".to_string(),
        "export default { fetch: () => new Response('1') }".to_string(),
        "/worker1/*".to_string(),
        HashMap::new(),
        None,
        Vec::new(),
    ).await.unwrap();
    
    manager.deploy_worker(
        "worker-2".to_string(),
        "export default { fetch: () => new Response('2') }".to_string(),
        "/worker2/*".to_string(),
        HashMap::new(),
        None,
        Vec::new(),
    ).await.unwrap();
    
    let workers = manager.list_workers(None);
    assert_eq!(workers.len(), 2);
}

#[tokio::test]
async fn test_worker_manager_list_workers_filter_active() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    let worker_id1 = manager.deploy_worker(
        "active-worker".to_string(),
        "export default { fetch: () => new Response('Active') }".to_string(),
        "/active/*".to_string(),
        HashMap::new(),
        None,
        Vec::new(),
    ).await.unwrap();
    
    let worker_id2 = manager.deploy_worker(
        "inactive-worker".to_string(),
        "export default { fetch: () => new Response('Inactive') }".to_string(),
        "/inactive/*".to_string(),
        HashMap::new(),
        None,
        Vec::new(),
    ).await.unwrap();
    
    // Deactivate second worker
    if let Some(mut worker) = manager.workers.get(&worker_id2) {
        worker.active = false;
    }
    
    let active_filter = WorkerFilter {
        active: Some(true),
        region: None,
    };
    let active_workers = manager.list_workers(Some(active_filter));
    assert_eq!(active_workers.len(), 1);
    assert_eq!(active_workers[0].name, "active-worker");
}

#[tokio::test]
async fn test_worker_manager_list_workers_filter_region() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    manager.deploy_worker(
        "us-worker".to_string(),
        "export default { fetch: () => new Response('US') }".to_string(),
        "/us/*".to_string(),
        HashMap::new(),
        None,
        vec!["us-east-1".to_string()],
    ).await.unwrap();
    
    manager.deploy_worker(
        "eu-worker".to_string(),
        "export default { fetch: () => new Response('EU') }".to_string(),
        "/eu/*".to_string(),
        HashMap::new(),
        None,
        vec!["eu-west-1".to_string()],
    ).await.unwrap();
    
    let region_filter = WorkerFilter {
        active: None,
        region: Some("us-east-1".to_string()),
    };
    let us_workers = manager.list_workers(Some(region_filter));
    assert_eq!(us_workers.len(), 1);
    assert_eq!(us_workers[0].name, "us-worker");
}

#[tokio::test]
async fn test_worker_manager_find_worker_by_route() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    manager.deploy_worker(
        "route-worker".to_string(),
        "export default { fetch: () => new Response('Route') }".to_string(),
        "/api/*".to_string(),
        HashMap::new(),
        None,
        Vec::new(),
    ).await.unwrap();
    
    let storage = Arc::new(InMemoryColumnStore::new());
    let db_manager = Arc::new(DatabaseManager::new());
    
    let request = WorkerRequest {
        method: "GET".to_string(),
        url: "/api/users".to_string(),
        headers: HashMap::new(),
        body: None,
        query: HashMap::new(),
        client_ip: None,
        request_id: "req-123".to_string(),
        worker_id: String::new(),
        edge_location: None,
    };
    
    let worker = manager.find_worker_by_route("/api/users", &None);
    assert!(worker.is_some());
    assert_eq!(worker.unwrap().route, "/api/*");
}

#[tokio::test]
async fn test_worker_manager_find_worker_by_route_wildcard() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    manager.deploy_worker(
        "wildcard-worker".to_string(),
        "export default { fetch: () => new Response('Wildcard') }".to_string(),
        "*".to_string(), // Match all
        HashMap::new(),
        None,
        Vec::new(),
    ).await.unwrap();
    
    let worker = manager.find_worker_by_route("/any/path", &None);
    assert!(worker.is_some());
    assert_eq!(worker.unwrap().route, "*");
}

#[tokio::test]
async fn test_worker_manager_find_worker_by_route_no_match() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    manager.deploy_worker(
        "specific-worker".to_string(),
        "export default { fetch: () => new Response('Specific') }".to_string(),
        "/specific/*".to_string(),
        HashMap::new(),
        None,
        Vec::new(),
    ).await.unwrap();
    
    let worker = manager.find_worker_by_route("/different/path", &None);
    assert!(worker.is_none());
}

#[tokio::test]
async fn test_worker_manager_find_worker_inactive() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    let worker_id = manager.deploy_worker(
        "inactive-worker".to_string(),
        "export default { fetch: () => new Response('Inactive') }".to_string(),
        "/inactive/*".to_string(),
        HashMap::new(),
        None,
        Vec::new(),
    ).await.unwrap();
    
    // Deactivate worker
    if let Some(mut worker) = manager.workers.get(&worker_id) {
        worker.active = false;
    }
    
    let worker = manager.find_worker_by_route("/inactive/test", &None);
    assert!(worker.is_none()); // Inactive workers should not be found
}

#[tokio::test]
async fn test_worker_manager_find_worker_by_region() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    manager.deploy_worker(
        "regional-worker".to_string(),
        "export default { fetch: () => new Response('Regional') }".to_string(),
        "/regional/*".to_string(),
        HashMap::new(),
        None,
        vec!["us-east-1".to_string(), "eu-west-1".to_string()],
    ).await.unwrap();
    
    // Should match in us-east-1
    let worker = manager.find_worker_by_route("/regional/test", &Some("us-east-1".to_string()));
    assert!(worker.is_some());
    
    // Should match in eu-west-1
    let worker = manager.find_worker_by_route("/regional/test", &Some("eu-west-1".to_string()));
    assert!(worker.is_some());
    
    // Should not match in different region
    let worker = manager.find_worker_by_route("/regional/test", &Some("ap-southeast-1".to_string()));
    assert!(worker.is_none());
}

#[tokio::test]
async fn test_worker_manager_find_worker_global_regions() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    // Worker with empty regions = global
    manager.deploy_worker(
        "global-worker".to_string(),
        "export default { fetch: () => new Response('Global') }".to_string(),
        "/global/*".to_string(),
        HashMap::new(),
        None,
        Vec::new(), // Empty regions = global
    ).await.unwrap();
    
    // Should match in any region
    let worker = manager.find_worker_by_route("/global/test", &Some("us-east-1".to_string()));
    assert!(worker.is_some());
    
    let worker = manager.find_worker_by_route("/global/test", &Some("eu-west-1".to_string()));
    assert!(worker.is_some());
    
    let worker = manager.find_worker_by_route("/global/test", &Some("ap-southeast-1".to_string()));
    assert!(worker.is_some());
}

#[tokio::test]
async fn test_worker_manager_execute_worker() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    let worker_id = manager.deploy_worker(
        "execute-worker".to_string(),
        "export default { fetch: () => new Response('Execute') }".to_string(),
        "/execute/*".to_string(),
        HashMap::new(),
        None,
        Vec::new(),
    ).await.unwrap();
    
    let storage = Arc::new(InMemoryColumnStore::new());
    let db_manager = Arc::new(DatabaseManager::new());
    
    let request = WorkerRequest {
        method: "GET".to_string(),
        url: "/execute/test".to_string(),
        headers: HashMap::new(),
        body: None,
        query: HashMap::new(),
        client_ip: None,
        request_id: "req-123".to_string(),
        worker_id: worker_id.clone(),
        edge_location: None,
    };
    
    let response = manager.execute_worker(request, storage, db_manager).await.unwrap();
    
    assert_eq!(response.status, 200);
    assert!(!response.body.is_empty());
}

#[tokio::test]
async fn test_worker_manager_execute_worker_not_found() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    let storage = Arc::new(InMemoryColumnStore::new());
    let db_manager = Arc::new(DatabaseManager::new());
    
    let request = WorkerRequest {
        method: "GET".to_string(),
        url: "/nonexistent".to_string(),
        headers: HashMap::new(),
        body: None,
        query: HashMap::new(),
        client_ip: None,
        request_id: "req-123".to_string(),
        worker_id: String::new(),
        edge_location: None,
    };
    
    let result = manager.execute_worker(request, storage, db_manager).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_worker_manager_execute_worker_inactive() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    let worker_id = manager.deploy_worker(
        "inactive-execute-worker".to_string(),
        "export default { fetch: () => new Response('Inactive') }".to_string(),
        "/inactive-execute/*".to_string(),
        HashMap::new(),
        None,
        Vec::new(),
    ).await.unwrap();
    
    // Deactivate worker
    if let Some(mut worker) = manager.workers.get(&worker_id) {
        worker.active = false;
    }
    
    let storage = Arc::new(InMemoryColumnStore::new());
    let db_manager = Arc::new(DatabaseManager::new());
    
    let request = WorkerRequest {
        method: "GET".to_string(),
        url: "/inactive-execute/test".to_string(),
        headers: HashMap::new(),
        body: None,
        query: HashMap::new(),
        client_ip: None,
        request_id: "req-123".to_string(),
        worker_id: worker_id.clone(),
        edge_location: None,
    };
    
    let result = manager.execute_worker(request, storage, db_manager).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_worker_manager_execute_worker_request_size_limit() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    let limits = WorkerLimits {
        cpu_time_ms: 50,
        memory_bytes: 128 * 1024 * 1024,
        timeout_ms: 30000,
        max_subrequests: 50,
        max_request_size: 100, // Very small limit
        max_response_size: 100 * 1024 * 1024,
    };
    
    let worker_id = manager.deploy_worker(
        "size-limited-worker".to_string(),
        "export default { fetch: () => new Response('Limited') }".to_string(),
        "/size-limited/*".to_string(),
        HashMap::new(),
        Some(limits),
        Vec::new(),
    ).await.unwrap();
    
    let storage = Arc::new(InMemoryColumnStore::new());
    let db_manager = Arc::new(DatabaseManager::new());
    
    // Create request with body exceeding limit
    let large_body = vec![0u8; 200]; // Exceeds 100 byte limit
    
    let request = WorkerRequest {
        method: "POST".to_string(),
        url: "/size-limited/test".to_string(),
        headers: HashMap::new(),
        body: Some(large_body),
        query: HashMap::new(),
        client_ip: None,
        request_id: "req-123".to_string(),
        worker_id: worker_id.clone(),
        edge_location: None,
    };
    
    let result = manager.execute_worker(request, storage, db_manager).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_worker_manager_edge_locations() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    let location = EdgeLocation {
        id: "test-location".to_string(),
        name: "Test Location".to_string(),
        region: "test-region".to_string(),
        coordinates: Some((0.0, 0.0)),
        active: true,
    };
    
    manager.add_edge_location(location.clone());
    
    let locations = manager.get_edge_locations();
    assert!(!locations.is_empty());
    assert!(locations.iter().any(|l| l.id == "test-location"));
}

#[tokio::test]
async fn test_worker_manager_route_matching() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    // Test exact match
    assert!(WorkerManager::match_route("/api", "/api"));
    assert!(!WorkerManager::match_route("/api", "/api/users"));
    
    // Test wildcard suffix
    assert!(WorkerManager::match_route("/api/*", "/api"));
    assert!(WorkerManager::match_route("/api/*", "/api/users"));
    assert!(WorkerManager::match_route("/api/*", "/api/users/123"));
    assert!(!WorkerManager::match_route("/api/*", "/other"));
    
    // Test global wildcard
    assert!(WorkerManager::match_route("*", "/any"));
    assert!(WorkerManager::match_route("*", "/any/path"));
    assert!(WorkerManager::match_route("*", "/"));
}

#[tokio::test]
async fn test_worker_manager_validate_route() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = WorkerManager::new(runtime);
    
    // Valid routes
    assert!(WorkerManager::validate_route("/api").is_ok());
    assert!(WorkerManager::validate_route("/api/*").is_ok());
    assert!(WorkerManager::validate_route("*").is_ok());
    
    // Invalid routes
    assert!(WorkerManager::validate_route("").is_err());
}

#[tokio::test]
async fn test_worker_execution_context_get_binding() {
    let storage = Arc::new(InMemoryColumnStore::new());
    let db_manager = Arc::new(DatabaseManager::new());
    
    let mut bindings = HashMap::new();
    bindings.insert("TEST_BINDING".to_string(), BindingValue::EnvVar {
        value: "test-value".to_string(),
    });
    
    let env = WorkerEnvironment {
        id: "context-worker".to_string(),
        name: "Context Worker".to_string(),
        code: "export default { fetch: () => new Response('Context') }".to_string(),
        route: "/context/*".to_string(),
        bindings,
        limits: WorkerLimits::default(),
        regions: Vec::new(),
        created_at: 0,
        updated_at: 0,
        version: 1,
        active: true,
    };
    
    let request = WorkerRequest {
        method: "GET".to_string(),
        url: "/context/test".to_string(),
        headers: HashMap::new(),
        body: None,
        query: HashMap::new(),
        client_ip: None,
        request_id: "req-123".to_string(),
        worker_id: "context-worker".to_string(),
        edge_location: None,
    };
    
    let ctx = WorkerExecutionContext::new(env, request, storage, db_manager);
    
    let binding = ctx.get_binding("TEST_BINDING");
    assert!(binding.is_some());
    
    match binding.unwrap() {
        BindingValue::EnvVar { value } => assert_eq!(value, "test-value"),
        _ => panic!("Expected EnvVar binding"),
    }
    
    // Non-existent binding
    assert!(ctx.get_binding("NONEXISTENT").is_none());
}

#[tokio::test]
async fn test_worker_execution_context_create_response() {
    let storage = Arc::new(InMemoryColumnStore::new());
    let db_manager = Arc::new(DatabaseManager::new());
    
    let env = WorkerEnvironment {
        id: "response-worker".to_string(),
        name: "Response Worker".to_string(),
        code: "export default { fetch: () => new Response('Response') }".to_string(),
        route: "/response/*".to_string(),
        bindings: HashMap::new(),
        limits: WorkerLimits::default(),
        regions: Vec::new(),
        created_at: 0,
        updated_at: 0,
        version: 1,
        active: true,
    };
    
    let request = WorkerRequest {
        method: "GET".to_string(),
        url: "/response/test".to_string(),
        headers: HashMap::new(),
        body: None,
        query: HashMap::new(),
        client_ip: None,
        request_id: "req-123".to_string(),
        worker_id: "response-worker".to_string(),
        edge_location: None,
    };
    
    let ctx = WorkerExecutionContext::new(env, request, storage, db_manager);
    
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    
    let body = b"{\"message\":\"Hello\"}".to_vec();
    let response = ctx.create_response(200, headers.clone(), body.clone());
    
    assert_eq!(response.status, 200);
    assert_eq!(response.headers, headers);
    assert_eq!(response.body, body);
    assert_eq!(response.metrics.response_size, body.len() as u64);
    assert!(response.metrics.execution_time_ms >= 0);
}

#[tokio::test]
async fn test_worker_runtime_name() {
    let runtime = MockJavaScriptRuntime;
    assert_eq!(runtime.name(), "mock-javascript-runtime");
}

// Concurrent execution tests

#[tokio::test]
async fn test_worker_manager_concurrent_deploy() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = Arc::new(WorkerManager::new(runtime));
    
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let manager = manager.clone();
            tokio::spawn(async move {
                manager.deploy_worker(
                    format!("concurrent-worker-{}", i),
                    "export default { fetch: () => new Response('Concurrent') }".to_string(),
                    format!("/concurrent-{}/*", i),
                    HashMap::new(),
                    None,
                    Vec::new(),
                ).await
            })
        })
        .collect();
    
    let results: Vec<_> = futures::future::join_all(handles).await;
    
    // All should succeed
    for result in results {
        assert!(result.unwrap().is_ok());
    }
    
    assert_eq!(manager.list_workers(None).len(), 10);
}

#[tokio::test]
async fn test_worker_manager_concurrent_execute() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = Arc::new(WorkerManager::new(runtime));
    
    let worker_id = manager.deploy_worker(
        "concurrent-execute-worker".to_string(),
        "export default { fetch: () => new Response('Concurrent Execute') }".to_string(),
        "/concurrent-execute/*".to_string(),
        HashMap::new(),
        None,
        Vec::new(),
    ).await.unwrap();
    
    let storage = Arc::new(InMemoryColumnStore::new());
    let db_manager = Arc::new(DatabaseManager::new());
    
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let manager = manager.clone();
            let storage = storage.clone();
            let db_manager = db_manager.clone();
            let worker_id = worker_id.clone();
            tokio::spawn(async move {
                let request = WorkerRequest {
                    method: "GET".to_string(),
                    url: format!("/concurrent-execute/test-{}", i),
                    headers: HashMap::new(),
                    body: None,
                    query: HashMap::new(),
                    client_ip: None,
                    request_id: format!("req-{}", i),
                    worker_id: worker_id.clone(),
                    edge_location: None,
                };
                manager.execute_worker(request, storage, db_manager).await
            })
        })
        .collect();
    
    let results: Vec<_> = futures::future::join_all(handles).await;
    
    // All should succeed
    for result in results {
        let response = result.unwrap().unwrap();
        assert_eq!(response.status, 200);
    }
}

// Stress tests

#[tokio::test]
async fn test_worker_manager_stress_deploy_delete() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = Arc::new(WorkerManager::new(runtime));
    
    // Deploy and delete many workers
    for i in 0..100 {
        let worker_id = manager.deploy_worker(
            format!("stress-worker-{}", i),
            "export default { fetch: () => new Response('Stress') }".to_string(),
            format!("/stress-{}/*", i),
            HashMap::new(),
            None,
            Vec::new(),
        ).await.unwrap();
        
        manager.delete_worker(&worker_id).await.unwrap();
    }
    
    // Should have no workers left
    assert_eq!(manager.list_workers(None).len(), 0);
}

#[tokio::test]
async fn test_worker_manager_stress_execute() {
    let runtime = Arc::new(MockJavaScriptRuntime);
    let manager = Arc::new(WorkerManager::new(runtime));
    
    let worker_id = manager.deploy_worker(
        "stress-execute-worker".to_string(),
        "export default { fetch: () => new Response('Stress Execute') }".to_string(),
        "/stress-execute/*".to_string(),
        HashMap::new(),
        None,
        Vec::new(),
    ).await.unwrap();
    
    let storage = Arc::new(InMemoryColumnStore::new());
    let db_manager = Arc::new(DatabaseManager::new());
    
    // Execute worker many times
    for i in 0..100 {
        let request = WorkerRequest {
            method: "GET".to_string(),
            url: format!("/stress-execute/test-{}", i),
            headers: HashMap::new(),
            body: None,
            query: HashMap::new(),
            client_ip: None,
            request_id: format!("req-{}", i),
            worker_id: worker_id.clone(),
            edge_location: None,
        };
        
        let response = manager.execute_worker(request, storage.clone(), db_manager.clone()).await.unwrap();
        assert_eq!(response.status, 200);
    }
}

