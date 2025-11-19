// Worker API endpoints for NarayanaDB

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use tracing::warn;
use narayana_storage::{
    workers::*,
    cognitive::CognitiveBrain,
    ColumnStore,
    database_manager::DatabaseManager,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Worker API state
#[derive(Clone)]
pub struct WorkerApiState {
    pub worker_manager: Arc<WorkerManager>,
    pub storage: Arc<dyn ColumnStore>,
    pub db_manager: Arc<DatabaseManager>,
    pub brain: Arc<CognitiveBrain>,
}

/// Deploy worker request
#[derive(Debug, Deserialize)]
pub struct DeployWorkerRequest {
    pub name: String,
    pub code: String,
    pub route: String,
    pub bindings: Option<HashMap<String, BindingValue>>,
    pub limits: Option<WorkerLimits>,
    pub regions: Option<Vec<String>>,
    /// Allowed URLs whitelist for fetch requests
    /// Examples:
    /// - ["http://localhost:*"] - Allow any port on localhost
    /// - ["http://127.0.0.1:8080"] - Allow specific URL
    /// - ["http://docker-host:5000"] - Allow Docker hostname
    pub allowed_urls: Option<Vec<String>>,
}

/// Deploy worker response
#[derive(Debug, Serialize)]
pub struct DeployWorkerResponse {
    pub worker_id: String,
    pub message: String,
}

/// Update worker request
#[derive(Debug, Deserialize)]
pub struct UpdateWorkerRequest {
    pub code: Option<String>,
    pub route: Option<String>,
    pub bindings: Option<HashMap<String, BindingValue>>,
    pub limits: Option<WorkerLimits>,
    pub regions: Option<Vec<String>>,
    pub active: Option<bool>,
    /// Allowed URLs whitelist for fetch requests
    pub allowed_urls: Option<Vec<String>>,
}

/// Worker response
#[derive(Debug, Serialize)]
pub struct WorkerResponse {
    pub worker: WorkerEnvironment,
}

/// List workers response
#[derive(Debug, Serialize)]
pub struct ListWorkersResponse {
    pub workers: Vec<WorkerEnvironment>,
    pub total: usize,
}

/// Execute worker request
#[derive(Debug, Deserialize)]
pub struct ExecuteWorkerRequest {
    pub worker_id: String,
    pub method: Option<String>,
    pub body: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub query: Option<HashMap<String, String>>,
    pub edge_location: Option<String>,
}

/// Execute worker response
#[derive(Debug, Serialize)]
pub struct ExecuteWorkerResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub metrics: ExecutionMetrics,
}

/// Edge locations response
#[derive(Debug, Serialize)]
pub struct EdgeLocationsResponse {
    pub locations: Vec<EdgeLocation>,
}

/// Create worker API router
pub fn create_worker_router(state: WorkerApiState) -> Router {
    Router::new()
        .route("/workers", post(deploy_worker))
        .route("/workers", get(list_workers))
        .route("/workers/:worker_id", get(get_worker))
        .route("/workers/:worker_id", put(update_worker))
        .route("/workers/:worker_id", delete(delete_worker))
        .route("/workers/:worker_id/execute", post(execute_worker))
        .route("/workers/:worker_id/execute", get(execute_worker_get))
        .route("/workers/execute/:route", post(execute_worker_by_route))
        .route("/workers/execute/:route", get(execute_worker_by_route_get))
        .route("/workers/edge-locations", get(get_edge_locations))
        .with_state(state)
}

/// Deploy worker endpoint
async fn deploy_worker(
    State(state): State<WorkerApiState>,
    Json(request): Json<DeployWorkerRequest>,
) -> Result<Json<DeployWorkerResponse>, StatusCode> {
    let worker_id = state
        .worker_manager
        .deploy_worker(
            request.name,
            request.code,
            request.route,
            request.bindings.unwrap_or_default(),
            request.limits,
            request.regions.unwrap_or_default(),
            request.allowed_urls,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(DeployWorkerResponse {
        worker_id: worker_id.clone(),
        message: format!("Worker {} deployed successfully", worker_id),
    }))
}

/// List workers endpoint
async fn list_workers(
    State(state): State<WorkerApiState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ListWorkersResponse>, StatusCode> {
    let filter = WorkerFilter {
        active: params.get("active").map(|v| v == "true"),
        region: params.get("region").cloned(),
    };

    let workers = state.worker_manager.list_workers(Some(filter));

    Ok(Json(ListWorkersResponse {
        total: workers.len(),
        workers,
    }))
}

/// Get worker endpoint
async fn get_worker(
    State(state): State<WorkerApiState>,
    Path(worker_id): Path<String>,
) -> Result<Json<WorkerResponse>, StatusCode> {
    let worker = state
        .worker_manager
        .get_worker(&worker_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(WorkerResponse { worker }))
}

/// Update worker endpoint
async fn update_worker(
    State(state): State<WorkerApiState>,
    Path(worker_id): Path<String>,
    Json(request): Json<UpdateWorkerRequest>,
) -> Result<Json<WorkerResponse>, StatusCode> {
    state
        .worker_manager
        .update_worker(
            &worker_id,
            request.code,
            request.route,
            request.bindings,
            request.limits,
            request.regions,
            request.allowed_urls,
        )
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Update active status if provided
    // Note: WorkerManager doesn't expose direct field access
    // We need to use update_worker method or redeploy with new active status
    // For now, if active is provided, we'll need to get the worker and update it
    // This is a limitation - WorkerManager needs an update_active method
    if request.active.is_some() {
        // TODO: Implement proper worker update in WorkerManager
        // For now, this functionality is not available via public API
        warn!("Updating worker active status not yet supported via public API");
    }

    let worker = state
        .worker_manager
        .get_worker(&worker_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(WorkerResponse { worker }))
}

/// Delete worker endpoint
async fn delete_worker(
    State(state): State<WorkerApiState>,
    Path(worker_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    state
        .worker_manager
        .delete_worker(&worker_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(serde_json::json!({
        "message": format!("Worker {} deleted successfully", worker_id)
    })))
}

/// Execute worker endpoint (POST)
async fn execute_worker(
    State(state): State<WorkerApiState>,
    Path(worker_id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<ExecuteWorkerRequest>,
) -> Result<Json<ExecuteWorkerResponse>, StatusCode> {
    // Get client IP from headers
    let client_ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    // Extract headers from request or use request headers
    let request_headers = request.headers.unwrap_or_else(|| {
        headers
            .iter()
            .filter_map(|(k, v)| {
                k.as_str()
                    .parse::<String>()
                    .ok()
                    .and_then(|key| v.to_str().ok().map(|val| (key, val.to_string())))
            })
            .collect()
    });

    let worker_request = WorkerRequest {
        method: request.method.unwrap_or_else(|| "POST".to_string()),
        url: format!("/workers/{}", worker_id),
        headers: request_headers,
        body: request.body.map(|b| b.into_bytes()),
        query: request.query.unwrap_or_default(),
        client_ip,
        request_id: Uuid::new_v4().to_string(),
        worker_id: request.worker_id,
        edge_location: request.edge_location,
    };

    let response = state
        .worker_manager
        .execute_worker(worker_request, state.storage.clone(), state.db_manager.clone(), Some(state.brain.clone()))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ExecuteWorkerResponse {
        status: response.status,
        headers: response.headers,
        body: String::from_utf8_lossy(&response.body).to_string(),
        metrics: response.metrics,
    }))
}

/// Execute worker endpoint (GET)
async fn execute_worker_get(
    State(state): State<WorkerApiState>,
    Path(worker_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<Json<ExecuteWorkerResponse>, StatusCode> {
    let client_ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let request_headers = headers
        .iter()
        .filter_map(|(k, v)| {
            k.as_str()
                .parse::<String>()
                .ok()
                .and_then(|key| v.to_str().ok().map(|val| (key, val.to_string())))
        })
        .collect();

    let worker_request = WorkerRequest {
        method: "GET".to_string(),
        url: format!("/workers/{}", worker_id),
        headers: request_headers,
        body: None,
        query: params,
        client_ip,
        request_id: Uuid::new_v4().to_string(),
        worker_id: worker_id.clone(),
        edge_location: None,
    };

    let response = state
        .worker_manager
        .execute_worker(worker_request, state.storage.clone(), state.db_manager.clone(), Some(state.brain.clone()))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ExecuteWorkerResponse {
        status: response.status,
        headers: response.headers,
        body: String::from_utf8_lossy(&response.body).to_string(),
        metrics: response.metrics,
    }))
}

/// Execute worker by route (POST)
async fn execute_worker_by_route(
    State(state): State<WorkerApiState>,
    Path(route): Path<String>,
    headers: HeaderMap,
    body: Option<String>,
) -> Result<Json<ExecuteWorkerResponse>, StatusCode> {
    let client_ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let request_headers = headers
        .iter()
        .filter_map(|(k, v)| {
            k.as_str()
                .parse::<String>()
                .ok()
                .and_then(|key| v.to_str().ok().map(|val| (key, val.to_string())))
        })
        .collect();

    let worker_request = WorkerRequest {
        method: "POST".to_string(),
        url: format!("/{}", route),
        headers: request_headers,
        body: body.map(|b| b.into_bytes()),
        query: HashMap::new(),
        client_ip,
        request_id: Uuid::new_v4().to_string(),
        worker_id: String::new(),
        edge_location: None,
    };

    let response = state
        .worker_manager
        .execute_worker(worker_request, state.storage.clone(), state.db_manager.clone(), Some(state.brain.clone()))
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(ExecuteWorkerResponse {
        status: response.status,
        headers: response.headers,
        body: String::from_utf8_lossy(&response.body).to_string(),
        metrics: response.metrics,
    }))
}

/// Execute worker by route (GET)
async fn execute_worker_by_route_get(
    State(state): State<WorkerApiState>,
    Path(route): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<Json<ExecuteWorkerResponse>, StatusCode> {
    let client_ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let request_headers = headers
        .iter()
        .filter_map(|(k, v)| {
            k.as_str()
                .parse::<String>()
                .ok()
                .and_then(|key| v.to_str().ok().map(|val| (key, val.to_string())))
        })
        .collect();

    let worker_request = WorkerRequest {
        method: "GET".to_string(),
        url: format!("/{}", route),
        headers: request_headers,
        body: None,
        query: params,
        client_ip,
        request_id: Uuid::new_v4().to_string(),
        worker_id: String::new(),
        edge_location: None,
    };

    let response = state
        .worker_manager
        .execute_worker(worker_request, state.storage.clone(), state.db_manager.clone(), Some(state.brain.clone()))
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(ExecuteWorkerResponse {
        status: response.status,
        headers: response.headers,
        body: String::from_utf8_lossy(&response.body).to_string(),
        metrics: response.metrics,
    }))
}

/// Get edge locations endpoint
async fn get_edge_locations(
    State(state): State<WorkerApiState>,
) -> Result<Json<EdgeLocationsResponse>, StatusCode> {
    let locations = state.worker_manager.get_edge_locations();

    Ok(Json(EdgeLocationsResponse { locations }))
}

