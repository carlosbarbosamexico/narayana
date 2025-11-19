// HTTP server with API routes for UI and database operations

use axum::{
    body::Body,
    extract::{Path, Query, State, Request},
    http::{Response, StatusCode, Uri, HeaderMap},
    middleware::Next,
    response::{IntoResponse, Json},
    routing::{delete, get, post},
    Router,
};
use narayana_storage::{
    ColumnStore,
    database_manager::DatabaseManager,
    human_search::HumanSearchEngine,
    webhooks::WebhookManager,
    workers::WorkerManager,
    cognitive::{CognitiveBrain, MemoryType, ThoughtState, CognitiveEventWithTimestamp, Conflict, MemoryAccessRecord},
};
use narayana_core::{schema::Schema, types::TableId, column::Column};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, error, warn};

// Protected system table name - cannot be accessed via normal API
const PROTECTED_USERS_TABLE: &str = "narayana_ui_users";

/// Rate limiting middleware for auth endpoints
async fn auth_rate_limit_middleware(
    State(state): State<ApiState>,
    request: Request,
    next: Next,
) -> Result<Response<Body>, StatusCode> {
    // Get client IP for rate limiting
    let client_ip = request
        .headers()
        .get("x-forwarded-for")
        .or_else(|| request.headers().get("x-real-ip"))
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown")
        .split(',')
        .next()
        .unwrap_or("unknown")
        .trim()
        .to_string();
    
    // DEVELOPMENT: Skip rate limiting for localhost/127.0.0.1 to allow robot demo
    let is_localhost = client_ip == "127.0.0.1" 
        || client_ip == "localhost" 
        || client_ip == "::1"
        || client_ip.starts_with("127.")
        || client_ip == "unknown"; // unknown usually means localhost when no proxy
    
    if !is_localhost {
        // SECURITY: Rate limit auth endpoints (5 attempts per 15 minutes) for non-localhost
        if let Err(_) = state.rate_limiter.check_rate_limit(&format!("auth:{}", client_ip)).await {
            warn!("Rate limit exceeded for auth endpoint from IP: {}", client_ip);
            let response = Json(ErrorResponse {
                error: "Too many requests. Please try again later.".to_string(),
                code: "RATE_LIMIT_EXCEEDED".to_string(),
            });
            return Ok((StatusCode::TOO_MANY_REQUESTS, response).into_response());
        }
    }
    
    Ok(next.run(request).await)
}

/// API rate limit middleware - rate limits API requests by user
async fn api_rate_limit_middleware(
    State(state): State<ApiState>,
    request: Request,
    next: Next,
) -> Result<Response<Body>, StatusCode> {
    // Get user ID from claims
    let user_id = if let Some(claims) = request.extensions().get::<crate::security::Claims>() {
        claims.sub.clone()
    } else {
        // Should not happen if auth_middleware runs first and attaches claims
        warn!("API rate limit: No claims found (auth middleware missing?)");
        // Fallback to IP-based rate limiting if no user (not ideal but safe)
        request
            .headers()
            .get("x-forwarded-for")
            .or_else(|| request.headers().get("x-real-ip"))
            .and_then(|h| h.to_str().ok())
            .unwrap_or("unknown")
            .split(',')
            .next()
            .unwrap_or("unknown")
            .trim()
            .to_string()
    };

    // Rate limit using api_rate_limiter
    // Use "user:{user_id}" or "ip:{ip}" as the key
    let key = if user_id.contains('.') || user_id == "unknown" {
        format!("ip:{}", user_id)
    } else {
        format!("user:{}", user_id)
    };
    
    if let Err(_) = state.api_rate_limiter.check_rate_limit(&key).await {
         warn!("API rate limit exceeded for: {}", key);
         let response = Json(ErrorResponse {
             error: "API rate limit exceeded. Please slow down.".to_string(),
             code: "RATE_LIMIT_EXCEEDED".to_string(),
         });
         return Ok((StatusCode::TOO_MANY_REQUESTS, response).into_response());
    }

    Ok(next.run(request).await)
}

/// Authentication middleware - validates JWT tokens
async fn auth_middleware(
    State(state): State<ApiState>,
    request: Request,
    next: Next,
) -> Result<Response<Body>, StatusCode> {
    // Extract authorization header (case-insensitive)
    // SECURITY: Check both "authorization" and "Authorization" headers
    let headers = request.headers();
    let auth_header = headers.get("authorization")
        .or_else(|| headers.get("Authorization"))
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // SECURITY: Case-insensitive Bearer prefix check to prevent bypass
    let auth_lower = auth_header.to_lowercase();
    if auth_lower.starts_with("bearer ") {
        // Find the actual position after "bearer " (case-insensitive)
        // SECURITY: Handle case variations like "Bearer ", "BEARER ", "bearer ", etc.
        let bearer_len = if auth_header.len() >= 7 && auth_header[..7].eq_ignore_ascii_case("bearer ") {
            7
        } else {
            // Fallback: find "bearer " case-insensitively
            match auth_lower.find("bearer ") {
                Some(pos) => {
                    // Find the space after "bearer"
                    if let Some(space_pos) = auth_header[pos..].find(' ') {
                        pos + space_pos + 1
                    } else {
                        pos + 7 // "bearer " is 7 chars
                    }
                }
                None => 0
            }
        };
        
        if bearer_len == 0 || bearer_len >= auth_header.len() {
            warn!("Invalid Bearer token format");
            return Err(StatusCode::UNAUTHORIZED);
        }
        
        let token = &auth_header[bearer_len..];
        // SECURITY: Trim whitespace from token to prevent bypass
        let token = token.trim();
        
        // SECURITY: Validate token is not empty
        // EDGE CASE: Handle empty, whitespace-only, control characters
        if token.is_empty() {
            warn!("Empty authentication token");
            return Err(StatusCode::UNAUTHORIZED);
        }
        
        // EDGE CASE: Check for control characters in token
        if token.chars().any(|c| c.is_control() || c == '\0') {
            warn!("Token contains control characters");
            return Err(StatusCode::UNAUTHORIZED);
        }
        
        // EDGE CASE: Check token length (prevent extremely long tokens)
        if token.len() > 4096 {
            warn!("Token too long: {} bytes", token.len());
            return Err(StatusCode::UNAUTHORIZED);
        }
        
        // Verify token signature and expiration
        match state.token_manager.verify_token(token) {
            Ok(claims) => {
                // Token is valid, attach claims to request
                let mut request = request;
                request.extensions_mut().insert(claims);
                Ok(next.run(request).await)
            }
            Err(_) => {
                // Invalid token
                warn!("Invalid authentication token");
                Err(StatusCode::UNAUTHORIZED)
            }
        }
    } else {
        warn!("Missing or invalid Authorization header");
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// Check if a table is the protected users table
fn is_protected_users_table(state: &ApiState, table_id: TableId) -> bool {
    // Check if this table ID corresponds to the protected users table
    if let Some(protected_table_id) = state.db_manager.get_table_by_name("default", PROTECTED_USERS_TABLE) {
        protected_table_id == table_id
    } else {
        false
    }
}

/// Check if a table name is the protected users table
fn is_protected_users_table_name(table_name: &str) -> bool {
    table_name == PROTECTED_USERS_TABLE
}

// API state
#[derive(Clone)]
pub struct ApiState {
    pub storage: Arc<dyn ColumnStore>,
    pub db_manager: Arc<DatabaseManager>,
    pub search_engine: Arc<HumanSearchEngine>,
    pub webhook_manager: Arc<WebhookManager>,
    pub worker_manager: Arc<WorkerManager>,
    pub brain: Arc<CognitiveBrain>,
    pub query_learning: Arc<narayana_storage::query_learning::QueryLearningEngine>,
    pub ws_state: Option<Arc<crate::websocket::WebSocketState>>,
    pub token_manager: Arc<crate::security::TokenManager>,
    pub rate_limiter: Arc<crate::security::RateLimiter>, // For auth endpoints
    pub api_rate_limiter: Arc<crate::security::RateLimiter>, // For API endpoints
}

// Statistics tracking
use std::sync::atomic::{AtomicU64, Ordering};
// Make stats accessible for WebSocket broadcasting
pub static TOTAL_QUERIES: AtomicU64 = AtomicU64::new(0);
pub static TOTAL_ROWS_READ: AtomicU64 = AtomicU64::new(0);
pub static TOTAL_ROWS_INSERTED: AtomicU64 = AtomicU64::new(0);
pub static TOTAL_QUERY_TIME_MS: AtomicU64 = AtomicU64::new(0);

// Response types
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct TablesResponse {
    pub tables: Vec<TableInfo>,
}

#[derive(Debug, Serialize)]
pub struct TableInfo {
    pub id: u64,
    pub name: String,
    pub schema: Option<Schema>,
    pub row_count: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTableRequest {
    pub table_name: String,
    pub schema: Schema,
}

#[derive(Debug, Serialize)]
pub struct CreateTableResponse {
    pub success: bool,
    pub table_id: u64,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InsertRequest {
    pub columns: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct InsertResponse {
    pub success: bool,
    pub rows_inserted: usize,
}

#[derive(Debug, Serialize)]
pub struct QueryResponse {
    pub columns: Vec<serde_json::Value>,
    pub row_count: usize,
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub total_queries: u64,
    pub avg_duration_ms: f64,
    pub total_rows_read: u64,
    pub total_rows_inserted: u64,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

/// SECURITY: Sanitize error messages to prevent information disclosure
/// Returns generic error messages for users, detailed errors only in logs
fn sanitize_error_message(internal_error: &str, error_code: &str) -> String {
    // SECURITY: Don't reveal internal details in user-facing error messages
    match error_code {
        "TABLE_NOT_FOUND" | "INVALID_TABLE_ID" => "Table not found".to_string(),
        "INVALID_CREDENTIALS" | "TOKEN_ERROR" => "Authentication failed".to_string(),
        "SETUP_ALREADY_DONE" => "Setup has already been completed".to_string(),
        "USER_CREATION_ERROR" | "CREATE_USER_ERROR" | "CREATE_DATABASE_ERROR" | 
        "CREATE_USER_TABLE_ERROR" | "INIT_USER_TABLE_ERROR" => "Failed to create user account".to_string(),
        "COLUMN_MISMATCH" | "COLUMN_COUNT_ERROR" | "PARSE_ERROR" => "Invalid data format".to_string(),
        "PAYLOAD_TOO_LARGE" | "COLUMN_TOO_LARGE" => "Request payload too large".to_string(),
        "TOO_MANY_COLUMNS" | "TOO_MANY_PARAMS" => "Too many items in request".to_string(),
        "INVALID_BRAIN_ID" | "INVALID_WEBHOOK_ID" => "Invalid identifier".to_string(),
        "WEBHOOK_NOT_FOUND" => "Webhook not found".to_string(),
        "DELETE_TABLE_ERROR" | "INSERT_ERROR" | "QUERY_ERROR" | "CREATE_TABLE_ERROR" | 
        "CREATE_THOUGHT_ERROR" | "STORE_EXPERIENCE_ERROR" | "CREATE_WEBHOOK_ERROR" | 
        "DELETE_WEBHOOK_ERROR" | "ENABLE_WEBHOOK_ERROR" | "DISABLE_WEBHOOK_ERROR" => {
            "Operation failed".to_string()
        },
        _ => "An error occurred".to_string(), // Generic fallback
    }
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub token: String,
    pub message: String,
}

/// Create HTTP router with all API routes
pub fn create_router(state: ApiState) -> Router {
    use axum::middleware;
    
    // Public routes (no authentication required)
    let public_routes = Router::new()
        // Health check
        .route("/health", get(health_handler))
        // Metrics (Prometheus format)
        .route("/metrics", get(metrics_handler))
        .route("/api/v1/health", get(health_handler));
    
    // Auth routes - setup check is not rate limited (read-only, called frequently)
    // Only login and setup POST endpoints are rate limited
    let setup_check_route = Router::new()
        .route("/api/v1/auth/setup/check", get(check_setup_handler));
    
    // Apply rate limiting only to POST endpoints (login and setup)
    let rate_limited_auth_routes = Router::new()
        .route("/api/v1/auth/setup", post(setup_handler).get(redirect_to_setup_check_handler))
        .route("/api/v1/auth/login", post(login_handler))
        .layer(middleware::from_fn_with_state(state.clone(), auth_rate_limit_middleware));
    
    // Merge rate-limited and non-rate-limited auth routes
    let auth_routes = setup_check_route.merge(rate_limited_auth_routes);
    
    // Protected routes (authentication required)
    let protected_routes = Router::new()
        // API v1 routes
        .route("/api/v1/stats", get(stats_handler))
        .route("/api/v1/tables", get(get_tables_handler).post(create_table_handler))
        .route("/api/v1/tables/:id", delete(delete_table_handler))
        .route("/api/v1/tables/:id/insert", post(insert_data_handler))
        .route("/api/v1/tables/:id/query", get(query_data_handler))
        // Cognitive Brain API (Robot endpoints)
        .route("/api/v1/brains", get(get_brains_handler).post(create_brain_handler))
        .route("/api/v1/brains/:brain_id/thoughts", post(create_thought_handler))
        .route("/api/v1/brains/:brain_id/experiences", post(store_experience_handler))
        .route("/api/v1/brains/:brain_id/memories", get(get_memories_handler))
        .route("/api/v1/brains/:brain_id/thoughts/cancel/:thought_id", post(cancel_thought_handler))
        .route("/api/v1/brains/:brain_id/thoughts/list", get(get_thoughts_handler))
        .route("/api/v1/brains/:brain_id/memory-accesses", get(get_memory_accesses_handler))
        .route("/api/v1/brains/:brain_id/thought-timeline", get(get_thought_timeline_handler))
        .route("/api/v1/brains/:brain_id/conflicts", get(get_conflicts_handler))
        // Workers API
        .route("/api/v1/workers", get(get_workers_handler))
        // Webhooks API
        .route("/api/v1/webhooks", get(get_webhooks_handler).post(create_webhook_handler))
        .route("/api/v1/webhooks/:id", get(get_webhook_handler).delete(delete_webhook_handler))
        .route("/api/v1/webhooks/:id/deliveries", get(get_webhook_deliveries_handler))
        .route("/api/v1/webhooks/:id/enable", post(enable_webhook_handler))
        .route("/api/v1/webhooks/:id/disable", post(disable_webhook_handler))
        // System stats
        .route("/api/v1/system/stats", get(get_system_stats_handler))
        // Schema and seeds management (public endpoints for CLI - no auth required)
        .route("/api/v1/schema/load", post(load_schema_handler))
        .route("/api/v1/schema/seeds", post(load_seeds_handler))
        .route("/api/v1/schema/spawn", post(spawn_schema_handler))
        .layer(middleware::from_fn_with_state(state.clone(), api_rate_limit_middleware))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware));
    
    // Note: Worker API routes from create_worker_router are handled separately
    // They use WorkerApiState which is incompatible with ApiState
    // For now, we use the simple get_workers_handler above
    // Full worker API would need to be integrated differently
    // let worker_router = create_worker_router(worker_api_state);
    // router = router.merge(worker_router); // Can't merge different state types

    // Add WebSocket route if WebSocket state is available
    // Note: WebSocket handler uses its own state type, so we need to handle it differently
    // For now, WebSocket is commented out until we can properly integrate the state
    // if let Some(ws_state) = &state.ws_state {
    //     use crate::websocket::{websocket_handler, WebSocketState};
    //     // WebSocket handler requires Arc<WebSocketState>, not ApiState
    //     // This needs to be handled at a higher level or we need to restructure
    // }

    // Combine public, auth, and protected routes
    let router = public_routes.merge(auth_routes).merge(protected_routes);
    
    router
        // Static files (UI) - catch all
        .fallback(serve_static_handler)
        .with_state(state)
}

#[derive(Debug, Serialize)]
struct SetupCheckResponse {
    setup_required: bool,
    message: String,
}

/// Redirect GET /auth/setup to /auth/setup/check
async fn redirect_to_setup_check_handler(State(state): State<ApiState>) -> impl IntoResponse {
    // If someone tries to GET /auth/setup, redirect them to /auth/setup/check
    check_setup_handler(State(state)).await
}

/// Check if setup is required (users table doesn't exist)
async fn check_setup_handler(State(state): State<ApiState>) -> impl IntoResponse {
    // Check env vars first - if they're set, setup is not required
    let env_user = std::env::var("NARAYANA_ADMIN_USER").ok();
    let env_password = std::env::var("NARAYANA_ADMIN_PASSWORD").ok();
    
    if env_user.is_some() && env_password.is_some() {
        return (StatusCode::OK, Json(SetupCheckResponse {
            setup_required: false,
            message: "Environment variables configured".to_string(),
        })).into_response();
    }
    
    // Check if users table exists in database manager
    let db_id = match state.db_manager.get_database_by_name("default") {
        Some(id) => id,
        None => {
            // No default database, setup required
            return (StatusCode::OK, Json(SetupCheckResponse {
                setup_required: true,
                message: "Initial setup required".to_string(),
            })).into_response();
        }
    };
    
    // Check if narayana_ui_users table exists
    match state.db_manager.get_table_by_name("default", "narayana_ui_users") {
        Some(table_id) => {
            // Table exists, verify it can be read and has users
            // Try to read at least one row to verify the table is not corrupted
            match state.storage.read_columns(table_id, vec![0], 0, 1).await {
                Ok(columns) => {
                    // Table exists and is readable
                    if !columns.is_empty() && columns[0].len() > 0 {
                        // Table has data, setup is complete
                        (StatusCode::OK, Json(SetupCheckResponse {
                            setup_required: false,
                            message: "Users table exists and has data".to_string(),
                        })).into_response()
                    } else {
                        // Table exists but is empty, setup required
                        warn!("Users table exists but is empty, requiring setup");
                        (StatusCode::OK, Json(SetupCheckResponse {
                            setup_required: true,
                            message: "Users table exists but is empty".to_string(),
                        })).into_response()
                    }
                }
                Err(e) => {
                    // Table exists but can't be read (corrupted), require setup
                    error!("Users table exists but cannot be read: {}. Requiring setup.", e);
                    (StatusCode::OK, Json(SetupCheckResponse {
                        setup_required: true,
                        message: format!("Users table exists but is corrupted: {}. Setup required.", e),
                    })).into_response()
                }
            }
        }
        None => {
            // Table doesn't exist, setup required
            (StatusCode::OK, Json(SetupCheckResponse {
                setup_required: true,
                message: "Initial setup required".to_string(),
            })).into_response()
        }
    }
}

#[derive(Debug, Deserialize)]
struct SetupRequest {
    name: String,
    username: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct SetupResponse {
    success: bool,
    message: String,
    user_id: Option<String>,
}

/// Create first admin user and users table
async fn setup_handler(
    State(state): State<ApiState>,
    Json(request): Json<SetupRequest>,
) -> impl IntoResponse {
    info!("Setting up first admin user: {}", request.username);
    
    // Validate input
    if request.name.trim().is_empty() || request.username.trim().is_empty() || request.password.is_empty() {
        let response = Json(ErrorResponse {
            error: "Name, username, and password are required".to_string(),
            code: "INVALID_INPUT".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // Validate username: alphanumeric and underscore, 3-50 chars
    // EDGE CASE: Handle unicode, control characters, and normalization
    let username = request.username.trim();
    
    // EDGE CASE: Check for empty after trim
    if username.is_empty() {
        let response = Json(ErrorResponse {
            error: "Username cannot be empty or whitespace only".to_string(),
            code: "INVALID_USERNAME".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // EDGE CASE: Check byte length vs char length (unicode handling)
    if username.len() < 3 || username.len() > 50 {
        let response = Json(ErrorResponse {
            error: "Username must be between 3 and 50 characters".to_string(),
            code: "INVALID_USERNAME".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // EDGE CASE: Check byte length separately (prevent unicode abuse)
    if username.as_bytes().len() > 50 {
        let response = Json(ErrorResponse {
            error: "Username byte length exceeds maximum".to_string(),
            code: "INVALID_USERNAME".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // EDGE CASE: Check for control characters and unicode normalization issues
    if username.chars().any(|c| c.is_control() || c == '\0') {
        let response = Json(ErrorResponse {
            error: "Username cannot contain control characters".to_string(),
            code: "INVALID_USERNAME".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        let response = Json(ErrorResponse {
            error: "Username can only contain letters, numbers, and underscores".to_string(),
            code: "INVALID_USERNAME".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // Validate password: 8-128 chars
    // EDGE CASE: Handle empty password, control characters, unicode
    if request.password.is_empty() {
        let response = Json(ErrorResponse {
            error: "Password cannot be empty".to_string(),
            code: "INVALID_PASSWORD".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // EDGE CASE: Check byte length (prevent unicode abuse)
    let password_bytes = request.password.as_bytes().len();
    if password_bytes < 8 || password_bytes > 128 {
        let response = Json(ErrorResponse {
            error: "Password must be between 8 and 128 bytes".to_string(),
            code: "INVALID_PASSWORD".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // EDGE CASE: Check for null bytes (could cause issues in some systems)
    if request.password.contains('\0') {
        let response = Json(ErrorResponse {
            error: "Password cannot contain null bytes".to_string(),
            code: "INVALID_PASSWORD".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // Validate name: 1-100 chars
    // EDGE CASE: Handle whitespace-only, unicode, control characters
    let name = request.name.trim();
    
    // EDGE CASE: Check for empty after trim
    if name.is_empty() {
        let response = Json(ErrorResponse {
            error: "Name cannot be empty or whitespace only".to_string(),
            code: "INVALID_NAME".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // EDGE CASE: Check char length
    if name.len() > 100 {
        let response = Json(ErrorResponse {
            error: "Name must be 100 characters or less".to_string(),
            code: "INVALID_NAME".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // EDGE CASE: Check byte length (prevent unicode abuse)
    if name.as_bytes().len() > 200 {
        let response = Json(ErrorResponse {
            error: "Name byte length exceeds maximum".to_string(),
            code: "INVALID_NAME".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // EDGE CASE: Check for control characters
    if name.chars().any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t') {
        let response = Json(ErrorResponse {
            error: "Name cannot contain control characters".to_string(),
            code: "INVALID_NAME".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // Check if env vars are set - if so, setup is not allowed
    let env_user = std::env::var("NARAYANA_ADMIN_USER").ok();
    if env_user.is_some() {
        let response = Json(ErrorResponse {
            error: "Setup is disabled when environment variables are configured".to_string(),
            code: "SETUP_DISABLED".to_string(),
        });
        return (StatusCode::FORBIDDEN, response).into_response();
    }
    
    // Check if users table already exists - prevent duplicate setup
    // This check happens early to prevent race conditions
    if let Some(table_id) = state.db_manager.get_table_by_name("default", "narayana_ui_users") {
        // Table already exists - check if it has any users
        // If empty, we could allow setup, but for security, we'll require manual intervention
        match state.storage.read_columns(table_id, vec![0], 0, 1).await {
            Ok(columns) => {
                if !columns.is_empty() {
                    // Table exists and has data
                    let response = Json(ErrorResponse {
                        error: "Setup has already been completed. Users table exists.".to_string(),
                        code: "SETUP_ALREADY_DONE".to_string(),
                    });
                    return (StatusCode::FORBIDDEN, response).into_response();
                }
                // Table exists but is empty - could be a failed setup
                // For now, still reject to prevent issues
                let response = Json(ErrorResponse {
                    error: "Users table exists but is empty. Please contact administrator.".to_string(),
                    code: "SETUP_ALREADY_DONE".to_string(),
                });
                return (StatusCode::FORBIDDEN, response).into_response();
            }
            Err(_) => {
                // Can't read table - might not be initialized in storage
                // Allow setup to proceed (will fail if table truly exists)
            }
        }
    }
    
    // Create users table schema
    use narayana_core::schema::{Schema, Field, DataType};
    let fields = vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::String,
            nullable: false,
            default_value: None,
        },
        Field {
            name: "name".to_string(),
            data_type: DataType::String,
            nullable: false,
            default_value: None,
        },
        Field {
            name: "username".to_string(),
            data_type: DataType::String,
            nullable: false,
            default_value: None,
        },
        Field {
            name: "password_hash".to_string(),
            data_type: DataType::String,
            nullable: false,
            default_value: None,
        },
        Field {
            name: "is_admin".to_string(),
            data_type: DataType::Boolean,
            nullable: false,
            default_value: Some(serde_json::json!(false)),
        },
        Field {
            name: "created_at".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ];
    
    let schema = Schema::new(fields);
    
    // Create default database if it doesn't exist
    let db_id = match state.db_manager.get_database_by_name("default") {
        Some(id) => id,
        None => {
            match state.db_manager.create_database("default".to_string()) {
                Ok(id) => id,
                Err(e) => {
                    error!("Failed to create default database: {}", e);
                    let response = Json(ErrorResponse {
                        error: sanitize_error_message(&format!("Failed to create database: {}", e), "CREATE_DATABASE_ERROR"),
                        code: "DATABASE_ERROR".to_string(),
                    });
                    return (StatusCode::INTERNAL_SERVER_ERROR, response).into_response();
                }
            }
        }
    };
    
    // Check for duplicate username before creating table
    // (This is a best-effort check - race condition still possible but unlikely)
    // Note: We can't check the table since it doesn't exist yet, but we'll validate after creation
    
    // Create users table in database manager
    let table_id = match state.db_manager.create_table(db_id, "narayana_ui_users".to_string(), schema.clone()) {
        Ok(id) => id,
        Err(e) => {
            // Check if error is because table already exists (race condition)
            if e.to_string().contains("already exists") {
                let response = Json(ErrorResponse {
                    error: "Setup has already been completed. Users table exists.".to_string(),
                    code: "SETUP_ALREADY_DONE".to_string(),
                });
                return (StatusCode::FORBIDDEN, response).into_response();
            }
            error!("Failed to create users table: {}", e);
            let response = Json(ErrorResponse {
                error: sanitize_error_message(&format!("Failed to create users table: {}", e), "CREATE_USER_TABLE_ERROR"),
                code: "TABLE_ERROR".to_string(),
            });
            return (StatusCode::INTERNAL_SERVER_ERROR, response).into_response();
        }
    };
    
    // Initialize table in storage (critical - table must exist in storage too)
    match state.storage.create_table(table_id, schema.clone()).await {
        Ok(_) => {
            info!("Initialized users table in storage");
        }
        Err(e) => {
            error!("Failed to initialize users table in storage: {}", e);
            // Try to clean up - remove from database manager
            // Note: This is best effort, table might remain in db_manager
            if let Err(cleanup_err) = state.db_manager.drop_table(table_id) {
                warn!("Failed to cleanup table after storage init failure: {}", cleanup_err);
            }
            
            let response = Json(ErrorResponse {
                error: sanitize_error_message(&format!("Failed to initialize users table: {}", e), "INIT_USER_TABLE_ERROR"),
                code: "STORAGE_INIT_ERROR".to_string(),
            });
            return (StatusCode::INTERNAL_SERVER_ERROR, response).into_response();
        }
    }
    
    // Hash password (simple hash for now - in production use bcrypt/argon2)
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(request.password.as_bytes());
    let password_hash = format!("{:x}", hasher.finalize());
    
    // Create first user record
    // EDGE CASE: Handle UUID generation failure (extremely unlikely)
    let user_id = uuid::Uuid::new_v4().to_string();
    
    // EDGE CASE: Handle system time before epoch
    let created_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    // EDGE CASE: Ensure created_at is not zero (use current time if zero)
    let created_at = if created_at == 0 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .max(1) // Ensure at least 1
    } else {
        created_at
    };
    
    // Insert user into table
    use narayana_core::column::Column;
    use narayana_core::types::ColumnId;
    
    let id_col = Column::String(vec![user_id.clone()]);
    let name_col = Column::String(vec![name.to_string()]);
    let username_col = Column::String(vec![username.to_string()]);
    let password_col = Column::String(vec![password_hash]);
    let is_admin_col = Column::Boolean(vec![true]);
    let created_at_col = Column::Int64(vec![created_at as i64]);
    
    // Validate all columns have the same length (should be 1 for single user)
    let column_lengths: Vec<usize> = vec![
        id_col.len(),
        name_col.len(),
        username_col.len(),
        password_col.len(),
        is_admin_col.len(),
        created_at_col.len(),
    ];
    
    let columns = vec![
        (ColumnId(0), id_col),
        (ColumnId(1), name_col),
        (ColumnId(2), username_col),
        (ColumnId(3), password_col),
        (ColumnId(4), is_admin_col),
        (ColumnId(5), created_at_col),
    ];
    
    if column_lengths.iter().any(|&len| len != 1) {
        error!("Column length mismatch in user creation: {:?}", column_lengths);
        let response = Json(ErrorResponse {
            error: "Internal error: column length mismatch".to_string(),
            code: "COLUMN_MISMATCH".to_string(),
        });
        return (StatusCode::INTERNAL_SERVER_ERROR, response).into_response();
    }
    
    // SECURITY: Check for duplicate username before inserting
    // This is a best-effort check - race condition still possible but unlikely
    // The table was just created, so it should be empty, but check anyway
    // EDGE CASE: Handle read errors, empty columns, large result sets
    match state.storage.read_columns(table_id, vec![2], 0, 1000).await {
        Ok(existing_columns) => {
            // SECURITY: Safely access first column, handling empty columns
            // EDGE CASE: Check for empty columns vector
            if existing_columns.is_empty() {
                // No existing data, safe to proceed
            } else if let Some(first_col) = existing_columns.get(0) {
                if let narayana_core::column::Column::String(existing_usernames) = first_col {
                    // EDGE CASE: Limit iteration to prevent DoS on large datasets
                    for existing_username in existing_usernames.iter().take(10000) {
                        // EDGE CASE: Handle empty usernames
                        if !existing_username.is_empty() && existing_username.eq_ignore_ascii_case(&username) {
                            error!("Duplicate username detected during setup: {}", username);
                            let response = Json(ErrorResponse {
                                error: "Username already exists".to_string(),
                                code: "DUPLICATE_USERNAME".to_string(),
                            });
                            return (StatusCode::CONFLICT, response).into_response();
                        }
                    }
                }
            }
        }
        Err(e) => {
            // EDGE CASE: Log error but continue (table might not be initialized yet)
            // This is expected for first user, but log for debugging
            warn!("Could not read existing users during setup (this is normal for first user): {}", e);
        }
    }
    
    // Write columns to storage
    // EDGE CASE: Handle write failures, partial writes
    let column_data: Vec<Column> = columns.into_iter().map(|(_, col)| col).collect();
    
    // EDGE CASE: Validate we have the expected number of columns
    if column_data.len() != 6 {
        error!("Invalid column count in user creation: {} (expected: 6)", column_data.len());
        let response = Json(ErrorResponse {
            error: "Internal error: invalid column count".to_string(),
            code: "COLUMN_COUNT_ERROR".to_string(),
        });
        return (StatusCode::INTERNAL_SERVER_ERROR, response).into_response();
    }
    
    match state.storage.write_columns(table_id, column_data).await {
        Ok(_) => {
            info!("Created first admin user: {}", username);
            (StatusCode::OK, Json(SetupResponse {
                success: true,
                message: "Setup completed successfully".to_string(),
                user_id: Some(user_id),
            })).into_response()
        }
        Err(e) => {
            error!("Failed to create user: {}", e);
            // EDGE CASE: Attempt cleanup if write fails
            // Try to drop the table if it was created but write failed
            // This is best-effort - if cleanup fails, admin intervention may be needed
            if let Err(cleanup_err) = state.db_manager.drop_table(table_id) {
                warn!("Failed to cleanup table after write failure: {}", cleanup_err);
            }
            
            let response = Json(ErrorResponse {
                error: sanitize_error_message(&format!("Failed to create user: {}", e), "CREATE_USER_ERROR"),
                code: "CREATE_USER_ERROR".to_string(),
            });
            return (StatusCode::INTERNAL_SERVER_ERROR, response).into_response();
        }
    }
}

/// Login endpoint
async fn login_handler(
    State(state): State<ApiState>,
    Json(request): Json<LoginRequest>,
) -> impl IntoResponse {
    // EDGE CASE: Validate input before processing
    let trimmed_username = request.username.trim();
    if trimmed_username.is_empty() {
        error!("Login attempt with empty username");
        let response = Json(ErrorResponse {
            error: "Invalid username or password".to_string(),
            code: "INVALID_CREDENTIALS".to_string(),
        });
        return (StatusCode::UNAUTHORIZED, response).into_response();
    }
    
    if request.password.is_empty() {
        error!("Login attempt with empty password");
        let response = Json(ErrorResponse {
            error: "Invalid username or password".to_string(),
            code: "INVALID_CREDENTIALS".to_string(),
        });
        return (StatusCode::UNAUTHORIZED, response).into_response();
    }
    
    // EDGE CASE: Check for extremely long credentials (DoS prevention)
    if trimmed_username.len() > 255 || request.password.len() > 128 {
        error!("Login attempt with excessively long credentials");
        let response = Json(ErrorResponse {
            error: "Invalid username or password".to_string(),
            code: "INVALID_CREDENTIALS".to_string(),
        });
        return (StatusCode::UNAUTHORIZED, response).into_response();
    }
    
    // First, check environment variables
    let env_user = std::env::var("NARAYANA_ADMIN_USER").ok();
    let env_password = std::env::var("NARAYANA_ADMIN_PASSWORD").ok();
    
    if let (Some(admin_user), Some(admin_password)) = (env_user, env_password) {
        // EDGE CASE: Handle empty env vars
        if admin_user.is_empty() || admin_password.is_empty() {
            // Treat empty env vars as not set
        } else {
            // Validate against env vars (constant-time comparison)
            let username_match = trimmed_username.len() == admin_user.len() && 
                trimmed_username.bytes().zip(admin_user.bytes()).all(|(a, b)| a == b);
            let password_match = request.password.len() == admin_password.len() && 
                request.password.bytes().zip(admin_password.bytes()).all(|(a, b)| a == b);
        
            if username_match && password_match {
                info!("Successful login for user: {} (env)", trimmed_username);
                // Generate proper JWT token
                // EDGE CASE: Use trimmed username
                match state.token_manager.generate_token(trimmed_username.to_string(), vec!["admin".to_string()]) {
                    Ok(token) => {
                        return (StatusCode::OK, Json(LoginResponse {
                            success: true,
                            token,
                            message: "Login successful".to_string(),
                        })).into_response();
                    }
                    Err(e) => {
                        error!("Failed to generate token: {}", e);
                        let response = Json(ErrorResponse {
                            error: "Authentication failed".to_string(),
                            code: "TOKEN_ERROR".to_string(),
                        });
                        return (StatusCode::INTERNAL_SERVER_ERROR, response).into_response();
                    }
                }
            }
        }
    }
    
    // If env vars not set or don't match, check users table
    let db_id = match state.db_manager.get_database_by_name("default") {
        Some(id) => id,
        None => {
            // No default database, can't authenticate
            error!("Failed login attempt for user: {} (no users table)", request.username);
            let response = Json(ErrorResponse {
                error: "Invalid username or password".to_string(),
                code: "INVALID_CREDENTIALS".to_string(),
            });
            return (StatusCode::UNAUTHORIZED, response).into_response();
        }
    };
    
    let table_id = match state.db_manager.get_table_by_name("default", "narayana_ui_users") {
        Some(id) => id,
        None => {
            // Users table doesn't exist
            error!("Failed login attempt for user: {} (users table not found)", request.username);
            let response = Json(ErrorResponse {
                error: "Invalid username or password".to_string(),
                code: "INVALID_CREDENTIALS".to_string(),
            });
            return (StatusCode::UNAUTHORIZED, response).into_response();
        }
    };
    
    // Query users table to find matching username
    // Schema: id(0), name(1), username(2), password_hash(3), is_admin(4), created_at(5)
    // Read id(0), username(2), password_hash(3), is_admin(4) columns
    match state.storage.read_columns(table_id, vec![0, 2, 3, 4], 0, 1000).await {
        Ok(columns) => {
            if columns.len() < 4 {
                error!("Users table has invalid schema - expected 4 columns, got {}", columns.len());
                let response = Json(ErrorResponse {
                    error: "Invalid username or password".to_string(),
                    code: "INVALID_CREDENTIALS".to_string(),
                });
                return (StatusCode::UNAUTHORIZED, response).into_response();
            }
            
            // Find user by username (case-insensitive)
            // SECURITY: Safely access columns with bounds checking
            // Schema: columns[0] = id, columns[1] = name, columns[2] = username, columns[3] = password_hash, columns[4] = is_admin
            let id_col = &columns[0];
            let username_col = &columns[2]; // Index 2 is username (0=id, 1=name, 2=username, 3=password, 4=is_admin)
            let password_col = &columns[3];
            let is_admin_col = &columns[4];
            
            let mut found_user_id = None;
            let mut found_password_hash = None;
            let mut found_is_admin = None;
            
            match (id_col, username_col, password_col, is_admin_col) {
                (narayana_core::column::Column::String(ids), 
                 narayana_core::column::Column::String(usernames),
                 narayana_core::column::Column::String(passwords),
                 narayana_core::column::Column::Boolean(is_admins)) => {
                    // Validate all columns have the same length
                    // EDGE CASE: Handle empty columns, mismatched lengths
                    let len = usernames.len();
                    
                    // EDGE CASE: Check for empty columns
                    if len > 0 {
                        if ids.len() != len || passwords.len() != len || is_admins.len() != len {
                            error!("Users table has mismatched column lengths: ids={}, usernames={}, passwords={}, is_admins={}", 
                                   ids.len(), usernames.len(), passwords.len(), is_admins.len());
                            let response = Json(ErrorResponse {
                                error: "Invalid username or password".to_string(),
                                code: "INVALID_CREDENTIALS".to_string(),
                            });
                            return (StatusCode::UNAUTHORIZED, response).into_response();
                        }
                        
                        // Find user by username (case-insensitive)
                        // EDGE CASE: Use trimmed username for comparison, handle empty usernames
                        for (idx, stored_username) in usernames.iter().enumerate() {
                            // EDGE CASE: Skip empty usernames
                            if stored_username.is_empty() {
                                continue;
                            }
                            
                            if stored_username.eq_ignore_ascii_case(trimmed_username) {
                                // Bounds check before accessing
                                // EDGE CASE: Double-check bounds even though we validated above
                                if idx < ids.len() && idx < passwords.len() && idx < is_admins.len() {
                                    found_user_id = ids.get(idx).cloned();
                                    found_password_hash = passwords.get(idx).cloned();
                                    found_is_admin = is_admins.get(idx).cloned();
                                } else {
                                    error!("Index out of bounds when accessing user data: idx={}, ids_len={}, passwords_len={}, is_admins_len={}", 
                                           idx, ids.len(), passwords.len(), is_admins.len());
                                }
                                break;
                            }
                        }
                    }
                    // If len == 0, no users in table, authentication will fail at end of function
                }
                _ => {
                    error!("Users table has unexpected column types");
                    let response = Json(ErrorResponse {
                        error: "Invalid username or password".to_string(),
                        code: "INVALID_CREDENTIALS".to_string(),
                    });
                    return (StatusCode::UNAUTHORIZED, response).into_response();
                }
            }
            
            if let (Some(user_id), Some(stored_hash), Some(is_admin)) = (found_user_id, found_password_hash, found_is_admin) {
                // Verify password (constant-time comparison to prevent timing attacks)
                use sha2::{Sha256, Digest};
                let mut hasher = Sha256::new();
                hasher.update(request.password.as_bytes());
                let password_hash = format!("{:x}", hasher.finalize());
                
                // Constant-time comparison (prevent timing attacks)
                // Use bitwise XOR to compare all bytes, then check result
                // EDGE CASE: Handle empty hashes, ensure constant-time comparison
                let mut diff = password_hash.len() != stored_hash.len();
                let min_len = password_hash.len().min(stored_hash.len());
                
                // EDGE CASE: Ensure we always do the same amount of work
                for (a, b) in password_hash.bytes().take(min_len).zip(stored_hash.bytes().take(min_len)) {
                    diff |= a != b;
                }
                
                // EDGE CASE: Always compare remaining bytes to maintain constant time
                // (even if lengths differ, we've already set diff = true)
                
                if !diff {
                    info!("Successful login for user: {} (database)", request.username);
                    // Generate proper JWT token with roles from database
                    let roles = if is_admin {
                        vec!["admin".to_string(), "user".to_string()]
                    } else {
                        vec!["user".to_string()]
                    };
                    match state.token_manager.generate_token(user_id, roles) {
                        Ok(token) => {
                            return (StatusCode::OK, Json(LoginResponse {
                                success: true,
                                token,
                                message: "Login successful".to_string(),
                            })).into_response();
                        }
                        Err(e) => {
                            error!("Failed to generate token: {}", e);
                            let response = Json(ErrorResponse {
                                error: "Authentication failed".to_string(),
                                code: "TOKEN_ERROR".to_string(),
                            });
                            return (StatusCode::INTERNAL_SERVER_ERROR, response).into_response();
                        }
                    }
                }
            } else {
                // User not found - still do password hash to prevent timing attacks
                // (don't reveal whether username exists)
                use sha2::{Sha256, Digest};
                let mut hasher = Sha256::new();
                hasher.update(request.password.as_bytes());
                let _ = format!("{:x}", hasher.finalize());
            }
        }
        Err(e) => {
            // Log detailed error for debugging
            let error_msg = format!("Failed to read users table: {}", e);
            error!("{}", error_msg);
            
            // Check if this is a decompression/deserialization error (corrupted data)
            let is_corruption_error = error_msg.contains("decompression") 
                || error_msg.contains("Deserialization") 
                || error_msg.contains("LZ4")
                || error_msg.contains("corrupted");
            
            if is_corruption_error {
                error!("Users table appears to be corrupted. User should run setup again to recreate the table.");
                // Return a more specific error code for corruption (but still generic message for security)
                let response = Json(ErrorResponse {
                    error: "Invalid username or password".to_string(),
                    code: "DATABASE_ERROR".to_string(),
                });
                return (StatusCode::UNAUTHORIZED, response).into_response();
            }
            
            // For other errors, fall through to generic error
        }
    }
    
    // Authentication failed
    error!("Failed login attempt for user: {}", request.username);
    let response = Json(ErrorResponse {
        error: "Invalid username or password".to_string(),
        code: "INVALID_CREDENTIALS".to_string(),
    });
    (StatusCode::UNAUTHORIZED, response).into_response()
}

/// Health check endpoint
async fn health_handler() -> impl IntoResponse {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Metrics endpoint (Prometheus format)
async fn metrics_handler() -> impl IntoResponse {
    // Return basic Prometheus metrics
    let metrics = r#"# HELP narayana_queries_total Total number of queries
# TYPE narayana_queries_total counter
narayana_queries_total 0

# HELP narayana_queries_duration_seconds Query duration in seconds
# TYPE narayana_queries_duration_seconds histogram
narayana_queries_duration_seconds 0

# HELP narayana_rows_read_total Total rows read
# TYPE narayana_rows_read_total counter
narayana_rows_read_total 0

# HELP narayana_rows_inserted_total Total rows inserted
# TYPE narayana_rows_inserted_total counter
narayana_rows_inserted_total 0
"#;
    
    // SECURITY: Handle response building errors gracefully
    match Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/plain; version=0.0.4")
        .body(Body::from(metrics))
    {
        Ok(response) => response,
        Err(e) => {
            error!("Failed to build metrics response: {}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Internal server error"))
                        .unwrap_or_else(|_| {
                    // Last resort - create minimal error response
                    // EDGE CASE: Handle nested unwrap failure (extremely unlikely)
                    // If this fails, we're in a bad state - return a basic response
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::from("Internal Server Error"))
                        .unwrap_or_else(|_| {
                            // Absolute last resort - create empty response
                            Response::new(Body::from("Error"))
                        })
                })
        }
    }
}

/// Get all tables
async fn get_tables_handler(State(state): State<ApiState>) -> impl IntoResponse {
    // List all tables from database manager, but exclude protected system tables
    let db_id = match state.db_manager.get_database_by_name("default") {
        Some(id) => id,
        None => {
            return Json(TablesResponse { tables: Vec::new() });
        }
    };
    
    let all_tables = match state.db_manager.list_tables(db_id) {
        Ok(tables) => tables,
        Err(_) => {
            return Json(TablesResponse { tables: Vec::new() });
        }
    };
    
    // Filter out protected system tables and get row counts
    let mut tables: Vec<TableInfo> = Vec::new();
    for table_info in all_tables {
        if is_protected_users_table_name(&table_info.name) {
            continue;
        }
        
        // Get row count from storage by reading first column with high limit
        // Note: This reads the full column which may be inefficient for very large tables
        // In production, consider caching row counts or using metadata
        let row_count = if !table_info.schema.fields.is_empty() {
            // Read first column with high limit to get actual row count
            // Using 10M as max limit - tables larger than this may not show accurate counts
            const MAX_COUNT_LIMIT: usize = 10_000_000;
            match state.storage.read_columns(table_info.table_id, vec![0], 0, MAX_COUNT_LIMIT).await {
                Ok(columns) => {
                    if let Some(first_col) = columns.first() {
                        Some(first_col.len() as u64)
                    } else {
                        Some(0)
                    }
                }
                Err(_) => {
                    // If read fails, return None (table might be empty or have issues)
                    None
                }
            }
        } else {
            Some(0) // Empty schema means no rows
        };
        
        tables.push(TableInfo {
            id: table_info.table_id.0,
            name: table_info.name,
            schema: Some(table_info.schema),
            row_count,
        });
    }
    
    Json(TablesResponse { tables })
}

/// Create a new table
async fn create_table_handler(
    State(state): State<ApiState>,
    Json(request): Json<CreateTableRequest>,
) -> impl IntoResponse {
    info!("Creating table: {}", request.table_name);
    
    // SECURITY: Validate table name
    let max_table_name_length: usize = 255;
    if request.table_name.trim().is_empty() {
        let response = Json(ErrorResponse {
            error: "Table name cannot be empty".to_string(),
            code: "INVALID_TABLE_NAME".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    if request.table_name.len() > max_table_name_length {
        let response = Json(ErrorResponse {
            error: format!("Table name too long. Maximum is {} characters", max_table_name_length),
            code: "INVALID_TABLE_NAME".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // SECURITY: Validate table name contains only safe characters
    // EDGE CASE: Handle empty, whitespace-only, unicode, control characters
    let table_name = request.table_name.trim();
    
    // EDGE CASE: Check for empty after trim
    if table_name.is_empty() {
        let response = Json(ErrorResponse {
            error: "Table name cannot be empty or whitespace only".to_string(),
            code: "INVALID_TABLE_NAME".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // EDGE CASE: Check for control characters
    if table_name.chars().any(|c| c.is_control() || c == '\0') {
        let response = Json(ErrorResponse {
            error: "Table name cannot contain control characters".to_string(),
            code: "INVALID_TABLE_NAME".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    if !table_name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        let response = Json(ErrorResponse {
            error: "Table name can only contain letters, numbers, underscores, and hyphens".to_string(),
            code: "INVALID_TABLE_NAME".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // SECURITY: Prevent creation of protected system tables
    if is_protected_users_table_name(&request.table_name) {
        error!("Attempt to create protected system table: {}", request.table_name);
        let response = Json(ErrorResponse {
            error: "Cannot create protected system table".to_string(),
            code: "PROTECTED_TABLE".to_string(),
        });
        return (StatusCode::FORBIDDEN, response).into_response();
    }
    
    // SECURITY: Validate schema
    // EDGE CASE: Handle empty schema, duplicate field names, invalid field names
    let max_fields: usize = 1000;
    
    // EDGE CASE: Check for empty schema
    if request.schema.fields.is_empty() {
        let response = Json(ErrorResponse {
            error: "Schema must have at least one field".to_string(),
            code: "INVALID_SCHEMA".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    if request.schema.fields.len() > max_fields {
        let response = Json(ErrorResponse {
            error: format!("Too many fields. Maximum is {}", max_fields),
            code: "TOO_MANY_FIELDS".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // EDGE CASE: Check for duplicate field names
    use std::collections::HashSet;
    let mut field_names = HashSet::new();
    for field in &request.schema.fields {
        if !field_names.insert(&field.name) {
            let response = Json(ErrorResponse {
                error: "Duplicate field name in schema".to_string(),
                code: "DUPLICATE_FIELD_NAME".to_string(),
            });
            return (StatusCode::BAD_REQUEST, response).into_response();
        }
        
        // EDGE CASE: Validate field name
        if field.name.trim().is_empty() {
            let response = Json(ErrorResponse {
                error: "Field name cannot be empty".to_string(),
                code: "INVALID_FIELD_NAME".to_string(),
            });
            return (StatusCode::BAD_REQUEST, response).into_response();
        }
        
        if field.name.len() > 255 {
            let response = Json(ErrorResponse {
                error: "Field name too long (maximum 255 characters)".to_string(),
                code: "INVALID_FIELD_NAME".to_string(),
            });
            return (StatusCode::BAD_REQUEST, response).into_response();
        }
    }
    
    // Create schema from request
    let schema = request.schema;
    
    // Get or create default database
    let db_id = match state.db_manager.get_database_by_name("default") {
        Some(id) => id,
        None => {
            match state.db_manager.create_database("default".to_string()) {
                Ok(id) => id,
                Err(e) => {
                    error!("Failed to create default database: {}", e);
                    let response = Json(ErrorResponse {
                        error: sanitize_error_message(&format!("Failed to create database: {}", e), "CREATE_DATABASE_ERROR"),
                        code: "DATABASE_ERROR".to_string(),
                    });
                    return (StatusCode::INTERNAL_SERVER_ERROR, response).into_response();
                }
            }
        }
    };
    
    // Generate a unique table ID using timestamp
    // SECURITY: Prevent integer overflow in timestamp calculation
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    
    // SECURITY: Safely convert to u64, preventing overflow
    let table_id_value = if timestamp > u64::MAX as u128 {
        // If timestamp exceeds u64::MAX, use modulo to wrap safely
        (timestamp % (u64::MAX as u128)) as u64
    } else {
        timestamp as u64
    };
    
    // SECURITY: Ensure table ID is not zero (reserved)
    // EDGE CASE: Handle zero timestamp, ensure valid table ID
    let table_id = if table_id_value == 0 {
        // EDGE CASE: If timestamp is 0, use a random fallback
        // Use current time in seconds as fallback
        let fallback = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        TableId(if fallback == 0 { 1 } else { fallback })
    } else {
        TableId(table_id_value)
    };
    
    // Create table in database manager first (so it shows up in list)
    match state.db_manager.create_table(db_id, request.table_name.clone(), schema.clone()) {
        Ok(created_table_id) => {
            // Use the table ID from database manager (it might be different)
            let final_table_id = created_table_id;
            
            // Create table in storage with the same ID
            match state.storage.create_table(final_table_id, schema.clone()).await {
                Ok(_) => {
                    info!("Table {} created with ID {}", request.table_name, final_table_id.0);
                    
                    // Emit database event
                    // TODO: Implement WebSocket event broadcasting when bridge is available
                    // if let Some(ws_state) = &state.ws_state {
                    //     ws_state.bridge.broadcast_database_event(
                    //         "default", // TODO: Get actual database name
                    //         Some(&request.table_name),
                    //         "create",
                    //         serde_json::json!({
                    //             "table_id": final_table_id.0,
                    //             "table_name": request.table_name,
                    //             "schema": schema,
                    //         }),
                    //     );
                    // }
                    
                    (StatusCode::OK, Json(CreateTableResponse {
                        success: true,
                        table_id: final_table_id.0,
                        message: format!("Table '{}' created successfully", request.table_name),
                    })).into_response()
                }
                Err(e) => {
                    error!("Failed to create table in storage: {}", e);
                    // Note: Database manager entry remains, but storage creation failed
                    // This is a partial failure state - table exists in manager but not in storage
                    let response = Json(ErrorResponse {
                        error: sanitize_error_message(&format!("Failed to create table in storage: {}", e), "CREATE_TABLE_ERROR"),
                        code: "CREATE_TABLE_ERROR".to_string(),
                    });
                    (StatusCode::INTERNAL_SERVER_ERROR, response).into_response()
                }
            }
        }
        Err(e) => {
            error!("Failed to create table in database manager: {}", e);
            let response = Json(ErrorResponse {
                error: sanitize_error_message(&format!("Failed to create table: {}", e), "CREATE_TABLE_ERROR"),
                code: "CREATE_TABLE_ERROR".to_string(),
            });
            (StatusCode::INTERNAL_SERVER_ERROR, response).into_response()
        }
    }
}

/// Delete a table
async fn delete_table_handler(
    State(state): State<ApiState>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    // EDGE CASE: Validate table ID is not zero
    if id == 0 {
        let response = Json(ErrorResponse {
            error: "Invalid table ID".to_string(),
            code: "INVALID_TABLE_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    info!("Deleting table: {}", id);
    
    let table_id = TableId(id);
    
    // SECURITY: Validate table exists before attempting deletion
    let db_id = match state.db_manager.get_database_by_name("default") {
        Some(id) => id,
        None => {
            let response = Json(ErrorResponse {
                error: "Table not found".to_string(),
                code: "TABLE_NOT_FOUND".to_string(),
            });
            return (StatusCode::NOT_FOUND, response).into_response();
        }
    };
    
    // Check if table exists
    // EDGE CASE: Handle error from list_tables
    let table_exists = match state.db_manager.list_tables(db_id) {
        Ok(tables) => tables.iter().any(|t| t.table_id == table_id),
        Err(_) => {
            error!("Failed to list tables for database");
            false
        }
    };
    
    if !table_exists {
        let response = Json(ErrorResponse {
            error: "Table not found".to_string(),
            code: "TABLE_NOT_FOUND".to_string(),
        });
        return (StatusCode::NOT_FOUND, response).into_response();
    }
    
    // SECURITY: Prevent deletion of protected system tables
    if is_protected_users_table(&state, table_id) {
        error!("Attempt to delete protected system table: {}", id);
        let response = Json(ErrorResponse {
            error: "Cannot delete protected system table".to_string(),
            code: "PROTECTED_TABLE".to_string(),
        });
        return (StatusCode::FORBIDDEN, response).into_response();
    }
    
    // Delete table from storage
    match state.storage.delete_table(table_id).await {
        Ok(_) => {
            // Emit database event
            // TODO: Implement WebSocket event broadcasting when bridge is available
            // if let Some(ws_state) = &state.ws_state {
            //     ws_state.bridge.broadcast_database_event(
            //         "default", // TODO: Get actual database name
            //         None, // Table name not available after deletion
            //         "drop",
            //         serde_json::json!({
            //             "table_id": id,
            //         }),
            //     );
            // }
            
            (StatusCode::OK, Json(serde_json::json!({
                "success": true,
                "message": format!("Table {} deleted", id)
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to delete table: {}", e);
            let response = Json(ErrorResponse {
                error: sanitize_error_message(&format!("Failed to delete table: {}", e), "DELETE_TABLE_ERROR"),
                code: "DELETE_TABLE_ERROR".to_string(),
            });
            (StatusCode::INTERNAL_SERVER_ERROR, response).into_response()
        }
    }
}

/// Insert data into a table
async fn insert_data_handler(
    State(state): State<ApiState>,
    Path(id): Path<u64>,
    Json(request): Json<InsertRequest>,
) -> impl IntoResponse {
    info!("Inserting data into table: {}", id);
    
    let table_id = TableId(id);
    
    // SECURITY: Validate table exists before inserting
    let db_id = match state.db_manager.get_database_by_name("default") {
        Some(id) => id,
        None => {
            let response = Json(ErrorResponse {
                error: "Table not found".to_string(),
                code: "TABLE_NOT_FOUND".to_string(),
            });
            return (StatusCode::NOT_FOUND, response).into_response();
        }
    };
    
    // Check if table exists and get schema
    let table_info = state.db_manager.list_tables(db_id)
        .ok()
        .and_then(|tables| tables.into_iter().find(|t| t.table_id == table_id));
    
    if table_info.is_none() {
        let response = Json(ErrorResponse {
            error: "Table not found".to_string(),
            code: "TABLE_NOT_FOUND".to_string(),
        });
        return (StatusCode::NOT_FOUND, response).into_response();
    }
    
    // SECURITY: Prevent modification of protected system tables via normal API
    if is_protected_users_table(&state, table_id) {
        error!("Attempt to insert into protected system table: {}", id);
        let response = Json(ErrorResponse {
            error: "Cannot modify protected system table via this endpoint".to_string(),
            code: "PROTECTED_TABLE".to_string(),
        });
        return (StatusCode::FORBIDDEN, response).into_response();
    }
    
    // SECURITY: Validate payload size before processing
    let max_payload_size: usize = 100 * 1024 * 1024; // 100MB
    let max_columns_per_insert: usize = 1000;
    let _max_json_depth: usize = 32; // Reserved for future use
    
    // Check column count
    if request.columns.len() > max_columns_per_insert {
        error!("Too many columns in insert request: {} (max: {})", request.columns.len(), max_columns_per_insert);
        let response = Json(ErrorResponse {
            error: format!("Too many columns. Maximum is {}", max_columns_per_insert),
            code: "TOO_MANY_COLUMNS".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // Convert JSON columns to Column types with size validation
    let mut columns: Vec<Column> = Vec::new();
    let mut total_size: usize = 0;
    
    // EDGE CASE: Check for empty columns array
    if request.columns.is_empty() {
        let response = Json(ErrorResponse {
            error: "No columns provided".to_string(),
            code: "INVALID_COLUMNS".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    for col_json in request.columns {
        // SECURITY: Check JSON size and depth before deserialization
        // SECURITY: Limit JSON string size to prevent DoS during serialization
        // EDGE CASE: Handle serialization failures, overflow in size calculation
        let json_str = match serde_json::to_string(&col_json) {
            Ok(s) => {
                // SECURITY: Check individual JSON string size
                if s.len() > 10 * 1024 * 1024 {
                    error!("Individual column JSON too large: {} bytes", s.len());
                    let response = Json(ErrorResponse {
                        error: "Column data too large".to_string(),
                        code: "COLUMN_TOO_LARGE".to_string(),
                    });
                    return (StatusCode::BAD_REQUEST, response).into_response();
                }
                s
            }
            Err(e) => {
                error!("Failed to serialize column JSON: {}", e);
                let response = Json(ErrorResponse {
                    error: "Invalid column data format".to_string(),
                    code: "PARSE_ERROR".to_string(),
                });
                return (StatusCode::BAD_REQUEST, response).into_response();
            }
        };
        
        // EDGE CASE: Check for overflow when adding to total_size
        total_size = match total_size.checked_add(json_str.len()) {
            Some(new_total) => {
                if new_total > max_payload_size {
                    error!("Insert payload too large: {} bytes (max: {} bytes)", new_total, max_payload_size);
                    let response = Json(ErrorResponse {
                        error: format!("Payload too large. Maximum is {} bytes", max_payload_size),
                        code: "PAYLOAD_TOO_LARGE".to_string(),
                    });
                    return (StatusCode::BAD_REQUEST, response).into_response();
                }
                new_total
            }
            None => {
                // Overflow detected
                error!("Payload size overflow detected");
                let response = Json(ErrorResponse {
                    error: "Payload too large".to_string(),
                    code: "PAYLOAD_TOO_LARGE".to_string(),
                });
                return (StatusCode::BAD_REQUEST, response).into_response();
            }
        };
        
        // Parse column from JSON - Column already implements Deserialize
        match serde_json::from_value::<Column>(col_json) {
            Ok(col) => {
                // SECURITY: Validate column size
                // EDGE CASE: Handle overflow in size calculation
                let col_size = match &col {
                    Column::String(v) => {
                        // EDGE CASE: Check for overflow in sum
                        v.iter().try_fold(0usize, |acc, s| {
                            acc.checked_add(s.len())
                        }).unwrap_or(usize::MAX)
                    },
                    Column::Int8(v) => v.len(),
                    Column::Int16(v) => {
                        // EDGE CASE: Check for overflow
                        v.len().checked_mul(2).unwrap_or(usize::MAX)
                    },
                    Column::Int32(v) => {
                        v.len().checked_mul(4).unwrap_or(usize::MAX)
                    },
                    Column::Int64(v) => {
                        v.len().checked_mul(8).unwrap_or(usize::MAX)
                    },
                    Column::UInt8(v) => v.len(),
                    Column::UInt16(v) => {
                        v.len().checked_mul(2).unwrap_or(usize::MAX)
                    },
                    Column::UInt32(v) => {
                        v.len().checked_mul(4).unwrap_or(usize::MAX)
                    },
                    Column::UInt64(v) => {
                        v.len().checked_mul(8).unwrap_or(usize::MAX)
                    },
                    Column::Float32(v) => {
                        v.len().checked_mul(4).unwrap_or(usize::MAX)
                    },
                    Column::Float64(v) => {
                        v.len().checked_mul(8).unwrap_or(usize::MAX)
                    },
                    Column::Boolean(v) => v.len(),
                    Column::Binary(v) => {
                        // EDGE CASE: Check for overflow in sum
                        v.iter().try_fold(0usize, |acc, b| {
                            acc.checked_add(b.len())
                        }).unwrap_or(usize::MAX)
                    },
                    Column::Timestamp(v) => {
                        v.len().checked_mul(8).unwrap_or(usize::MAX)
                    },
                    Column::Date(v) => {
                        v.len().checked_mul(4).unwrap_or(usize::MAX)
                    },
                };
                
                let max_column_size: usize = 10 * 1024 * 1024; // 10MB per column
                // EDGE CASE: Check for overflow in size calculation
                if col_size > max_column_size || col_size == 0 {
                    // EDGE CASE: col_size == 0 means empty column, which is valid
                    // Only reject if it exceeds max
                    if col_size > max_column_size {
                        error!("Column too large: {} bytes (max: {} bytes)", col_size, max_column_size);
                        let response = Json(ErrorResponse {
                            error: format!("Column too large. Maximum is {} bytes per column", max_column_size),
                            code: "COLUMN_TOO_LARGE".to_string(),
                        });
                        return (StatusCode::BAD_REQUEST, response).into_response();
                    }
                }
                
                columns.push(col);
            }
            Err(e) => {
                error!("Failed to parse column: {}", e);
                let response = Json(ErrorResponse {
                    error: sanitize_error_message(&format!("Failed to parse column: {}", e), "PARSE_ERROR"),
                    code: "PARSE_ERROR".to_string(),
                });
                return (StatusCode::BAD_REQUEST, response).into_response();
            }
        }
    }
    
    if columns.is_empty() {
        let response = Json(ErrorResponse {
            error: "No valid columns provided".to_string(),
            code: "INVALID_COLUMNS".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // SECURITY: Validate column count matches table schema
    if let Some(ref table) = table_info {
        if columns.len() != table.schema.fields.len() {
            error!("Column count mismatch: expected {}, got {}", table.schema.fields.len(), columns.len());
            let response = Json(ErrorResponse {
                error: sanitize_error_message(&format!("Column count mismatch. Expected {} columns, got {}", table.schema.fields.len(), columns.len()), "COLUMN_COUNT_ERROR"),
                code: "COLUMN_COUNT_MISMATCH".to_string(),
            });
            return (StatusCode::BAD_REQUEST, response).into_response();
        }
    }
    
    match state.storage.write_columns(table_id, columns.clone()).await {
        Ok(_) => {
            // EDGE CASE: Handle empty columns, overflow in conversion
            let row_count = columns.first().map(|c| c.len()).unwrap_or(0);
            
            // EDGE CASE: Check for usize to u64 overflow
            let row_count_u64 = if row_count > u64::MAX as usize {
                u64::MAX
            } else {
                row_count as u64
            };
            
            TOTAL_ROWS_INSERTED.fetch_add(row_count_u64, Ordering::Relaxed);
            info!("Inserted {} rows into table {}", row_count, id);
            
            // Emit database event
            // TODO: Implement WebSocket event broadcasting when bridge is available
            // if let Some(ws_state) = &state.ws_state {
            //     ws_state.bridge.broadcast_database_event(
            //         "default", // TODO: Get actual database name
            //         None, // TODO: Get table name from table_id
            //         "insert",
            //         serde_json::json!({
            //             "table_id": id,
            //             "row_count": row_count,
            //         }),
            //     );
            // }
            
            (StatusCode::OK, Json(InsertResponse {
                success: true,
                rows_inserted: row_count,
            })).into_response()
        }
        Err(e) => {
            error!("Failed to insert data: {}", e);
            let response = Json(ErrorResponse {
                error: sanitize_error_message(&format!("Failed to insert data: {}", e), "INSERT_ERROR"),
                code: "INSERT_ERROR".to_string(),
            });
            (StatusCode::INTERNAL_SERVER_ERROR, response).into_response()
        }
    }
}

/// Query data from a table
async fn query_data_handler(
    State(state): State<ApiState>,
    Path(id): Path<u64>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    // EDGE CASE: Validate table ID is not zero
    if id == 0 {
        let response = Json(ErrorResponse {
            error: "Invalid table ID".to_string(),
            code: "INVALID_TABLE_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // SECURITY: Limit number of query parameters to prevent DoS
    const MAX_QUERY_PARAMS: usize = 100;
    if params.len() > MAX_QUERY_PARAMS {
        error!("Too many query parameters: {} (max: {})", params.len(), MAX_QUERY_PARAMS);
        let response = Json(ErrorResponse {
            error: format!("Too many query parameters. Maximum is {}", MAX_QUERY_PARAMS),
            code: "TOO_MANY_PARAMS".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // SECURITY: Validate parameter key and value lengths
    for (key, value) in &params {
        if key.len() > 255 {
            error!("Query parameter key too long: {} chars", key.len());
            let response = Json(ErrorResponse {
                error: "Query parameter key too long".to_string(),
                code: "INVALID_PARAM".to_string(),
            });
            return (StatusCode::BAD_REQUEST, response).into_response();
        }
        if value.len() > 10_000 {
            error!("Query parameter value too long: {} chars", value.len());
            let response = Json(ErrorResponse {
                error: "Query parameter value too long".to_string(),
                code: "INVALID_PARAM".to_string(),
            });
            return (StatusCode::BAD_REQUEST, response).into_response();
        }
    }
    
    info!("Querying table: {}", id);
    
    let table_id = TableId(id);
    
    // SECURITY: Validate table exists before querying
    let db_id = match state.db_manager.get_database_by_name("default") {
        Some(id) => id,
        None => {
            let response = Json(ErrorResponse {
                error: "Table not found".to_string(),
                code: "TABLE_NOT_FOUND".to_string(),
            });
            return (StatusCode::NOT_FOUND, response).into_response();
        }
    };
    
    // Check if table exists
    // EDGE CASE: Handle error from list_tables
    let table_info = match state.db_manager.list_tables(db_id) {
        Ok(tables) => tables.into_iter().find(|t| t.table_id == table_id),
        Err(_) => {
            error!("Failed to list tables for database");
            let response = Json(ErrorResponse {
                error: "Table not found".to_string(),
                code: "TABLE_NOT_FOUND".to_string(),
            });
            return (StatusCode::NOT_FOUND, response).into_response();
        }
    };
    
    if table_info.is_none() {
        let response = Json(ErrorResponse {
            error: "Table not found".to_string(),
            code: "TABLE_NOT_FOUND".to_string(),
        });
        return (StatusCode::NOT_FOUND, response).into_response();
    }
    
    // SECURITY: Prevent querying of protected system tables via normal API
    if is_protected_users_table(&state, table_id) {
        error!("Attempt to query protected system table: {}", id);
        let response = Json(ErrorResponse {
            error: "Cannot query protected system table via this endpoint".to_string(),
            code: "PROTECTED_TABLE".to_string(),
        });
        return (StatusCode::FORBIDDEN, response).into_response();
    }
    
    // Parse query parameters with security validation
    let max_columns: usize = 100;
    let max_limit: usize = 10_000;
    let default_limit: usize = 100;
    
    // SECURITY: Parse column indices with DoS protection
    // EDGE CASE: Handle empty params, duplicate indices, invalid formats
    let column_indices: Vec<u32> = params
        .get("columns")
        .map(|s| {
            // EDGE CASE: Check for empty string
            if s.is_empty() {
                return Vec::new();
            }
            
            // SECURITY: Limit split operations to prevent DoS
            let parts: Vec<&str> = s.split(',').take(max_columns + 1).collect();
            
            // SECURITY: Check if too many parts (DoS attempt)
            if parts.len() > max_columns {
                warn!("Too many column indices in query: {} (max: {})", parts.len(), max_columns);
                return Vec::new(); // Return empty, will be caught by validation below
            }
            
            // EDGE CASE: Use HashSet to deduplicate indices
            use std::collections::HashSet;
            let mut seen = HashSet::new();
            
            parts.into_iter()
                .filter_map(|x| {
                    let trimmed = x.trim();
                    // SECURITY: Limit trimmed length to prevent DoS
                    if trimmed.len() > 20 {
                        warn!("Column index string too long: {} chars", trimmed.len());
                        return None;
                    }
                    
                    // SECURITY: Validate integer parsing and bounds
                    // EDGE CASE: Handle negative numbers, overflow, empty strings
                    if trimmed.is_empty() {
                        return None;
                    }
                    
                    // EDGE CASE: Check for negative sign
                    if trimmed.starts_with('-') {
                        warn!("Negative column index rejected: {}", trimmed);
                        return None;
                    }
                    
                    // EDGE CASE: Check for leading zeros (could be octal interpretation attempt)
                    if trimmed.len() > 1 && trimmed.starts_with('0') && trimmed.chars().nth(1).map_or(false, |c| c.is_ascii_digit()) {
                        warn!("Column index with leading zero rejected: {}", trimmed);
                        return None;
                    }
                    
                    if let Ok(parsed) = trimmed.parse::<u64>() {
                        // EDGE CASE: Check bounds and overflow
                        if parsed <= u32::MAX as u64 {
                            let idx = parsed as u32;
                            // EDGE CASE: Deduplicate indices
                            if seen.insert(idx) {
                                Some(idx)
                            } else {
                                warn!("Duplicate column index ignored: {}", idx);
                                None
                            }
                        } else {
                            warn!("Column index {} exceeds u32::MAX, ignoring", parsed);
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_else(|| vec![0, 1]); // Default to first two columns
    
    // SECURITY: Validate we have at least one column index
    if column_indices.is_empty() {
        let response = Json(ErrorResponse {
            error: "No valid column indices provided".to_string(),
            code: "INVALID_COLUMNS".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // SECURITY: Limit number of columns to prevent DoS
    if column_indices.len() > max_columns {
        error!("Too many columns requested: {} (max: {})", column_indices.len(), max_columns);
        let response = Json(ErrorResponse {
            error: format!("Too many columns requested. Maximum is {}", max_columns),
            code: "TOO_MANY_COLUMNS".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // SECURITY: Parse limit with validation to prevent DoS and edge cases
    let limit = params
        .get("limit")
        .and_then(|s| {
            let trimmed = s.trim();
            // EDGE CASE: Handle empty string, negative numbers, overflow
            if trimmed.is_empty() {
                return None;
            }
            // EDGE CASE: Check for negative sign
            if trimmed.starts_with('-') {
                return None;
            }
            // EDGE CASE: Validate it's a valid number within bounds
            if let Ok(parsed) = trimmed.parse::<u64>() {
                if parsed > 0 && parsed <= max_limit as u64 {
                    // EDGE CASE: Check for usize overflow
                    if parsed <= usize::MAX as u64 {
                        Some(parsed as usize)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        })
        .unwrap_or(default_limit);
    
    // SECURITY: Validate limit is not zero
    if limit == 0 {
        let response = Json(ErrorResponse {
            error: "Limit must be greater than 0".to_string(),
            code: "INVALID_LIMIT".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // Track query start time
    let query_start = std::time::Instant::now();
    
    // SECURITY: Validate column indices are within table bounds
    // EDGE CASE: Handle empty schema, zero columns, overflow
    if let Some(ref table) = table_info {
        // EDGE CASE: Check for empty schema
        if table.schema.fields.is_empty() {
            let response = Json(ErrorResponse {
                error: "Table has no columns".to_string(),
                code: "INVALID_TABLE_SCHEMA".to_string(),
            });
            return (StatusCode::BAD_REQUEST, response).into_response();
        }
        
        // EDGE CASE: Check for usize overflow when converting to u32
        let max_col_index = if table.schema.fields.len() > u32::MAX as usize {
            u32::MAX
        } else {
            table.schema.fields.len() as u32
        };
        
        for &col_idx in &column_indices {
            // EDGE CASE: Check for zero (valid index) and bounds
            if col_idx >= max_col_index {
                error!("Column index {} out of bounds (max: {})", col_idx, max_col_index.saturating_sub(1));
                let response = Json(ErrorResponse {
                    error: "Column index is out of bounds".to_string(),
                    code: "INVALID_COLUMN_INDEX".to_string(),
                });
                return (StatusCode::BAD_REQUEST, response).into_response();
            }
        }
    }
    
    // Read columns from storage
    match state.storage.read_columns(table_id, column_indices.clone(), 0, limit).await {
        Ok(columns) => {
            // Track statistics
            // SECURITY: Safely get row count, handling empty columns gracefully
            // EDGE CASE: Handle empty columns, overflow in conversion
            let row_count = columns.first()
                .map(|c| c.len())
                .unwrap_or(0);
            
            // EDGE CASE: Check for usize to u64 overflow (unlikely but safe)
            let row_count_u64 = if row_count > u64::MAX as usize {
                u64::MAX
            } else {
                row_count as u64
            };
            
            TOTAL_QUERIES.fetch_add(1, Ordering::Relaxed);
            TOTAL_ROWS_READ.fetch_add(row_count_u64, Ordering::Relaxed);
            
            // EDGE CASE: Handle potential overflow in elapsed time
            let query_time_ms = query_start.elapsed().as_millis();
            let query_time_ms_u64 = if query_time_ms > u64::MAX as u128 {
                u64::MAX
            } else {
                query_time_ms as u64
            };
            TOTAL_QUERY_TIME_MS.fetch_add(query_time_ms_u64, Ordering::Relaxed);
            
            // Convert columns to JSON - Column already implements Serialize
            let json_columns: Vec<serde_json::Value> = columns
                .iter()
                .filter_map(|col| {
                    // Serialize column to JSON
                    serde_json::to_value(col).ok()
                })
                .collect();
            
            (StatusCode::OK, Json(QueryResponse {
                columns: json_columns,
                row_count,
            })).into_response()
        }
        Err(e) => {
            error!("Failed to query table: {}", e);
            let response = Json(ErrorResponse {
                error: sanitize_error_message(&format!("Failed to query table: {}", e), "QUERY_ERROR"),
                code: "QUERY_ERROR".to_string(),
            });
            (StatusCode::INTERNAL_SERVER_ERROR, response).into_response()
        }
    }
}

/// Get query statistics
async fn stats_handler(State(state): State<ApiState>) -> impl IntoResponse {
    // Get real statistics from atomic counters and query learning engine
    let total_queries = TOTAL_QUERIES.load(Ordering::Relaxed);
    let total_rows_read = TOTAL_ROWS_READ.load(Ordering::Relaxed);
    let total_rows_inserted = TOTAL_ROWS_INSERTED.load(Ordering::Relaxed);
    let total_query_time = TOTAL_QUERY_TIME_MS.load(Ordering::Relaxed);
    
    let avg_duration_ms = if total_queries > 0 {
        total_query_time as f64 / total_queries as f64
    } else {
        0.0
    };
    
    Json(StatsResponse {
        total_queries,
        avg_duration_ms,
        total_rows_read,
        total_rows_inserted,
    })
}

/// Serve static files (UI) - fallback handler
async fn serve_static_handler(uri: Uri) -> impl IntoResponse {
    use crate::static_files::serve_static;
    serve_static(uri).await
}

// Cognitive Brain API handlers

#[derive(Debug, Serialize, Deserialize)]
struct CreateBrainRequest {
    brain_id: String,
    memory_types: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct CreateBrainResponse {
    success: bool,
    brain_id: String,
    message: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateThoughtRequest {
    content: serde_json::Value,
    priority: f64,
}

#[derive(Debug, Serialize)]
struct CreateThoughtResponse {
    success: bool,
    thought_id: String,
    message: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoreExperienceRequest {
    observation: serde_json::Value,
    action: Option<serde_json::Value>,
    outcome: Option<serde_json::Value>,
    reward: Option<f64>,
}

#[derive(Debug, Serialize)]
struct StoreExperienceResponse {
    success: bool,
    experience_id: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct MemoryResponse {
    id: String,
    memory_type: String,
    content: serde_json::Value,
    created_at: u64,
}

#[derive(Debug, Serialize)]
struct GetMemoriesResponse {
    memories: Vec<MemoryResponse>,
    count: usize,
}

#[derive(Debug, Serialize)]
struct GetThoughtsResponse {
    thoughts: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct GetMemoryAccessesResponse {
    accesses: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct GetThoughtTimelineResponse {
    timeline: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct GetConflictsResponse {
    conflicts: Vec<Conflict>,
}

#[derive(Debug, Serialize)]
struct CancelThoughtResponse {
    success: bool,
    message: String,
}

/// Create a cognitive brain for a robot
async fn create_brain_handler(
    State(state): State<ApiState>,
    Json(request): Json<CreateBrainRequest>,
) -> impl IntoResponse {
    let brain_id = request.brain_id.clone();
    info!("Creating brain: {}", brain_id);
    
    // Brain is already created and shared, just return success
    // In a real implementation, we'd manage multiple brains per brain_id
    (StatusCode::OK, Json(CreateBrainResponse {
        success: true,
        brain_id,
        message: format!("Brain '{}' is ready", request.brain_id),
    })).into_response()
}

/// Create a thought (robot decision)
async fn create_thought_handler(
    State(state): State<ApiState>,
    Path(brain_id): Path<String>,
    Json(request): Json<CreateThoughtRequest>,
) -> impl IntoResponse {
    // SECURITY: Validate brain_id to prevent path traversal/injection
    // EDGE CASE: Handle empty, whitespace-only, unicode, control characters
    let trimmed_brain_id = brain_id.trim();
    
    if trimmed_brain_id.is_empty() {
        let response = Json(ErrorResponse {
            error: "Brain ID cannot be empty or whitespace only".to_string(),
            code: "INVALID_BRAIN_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    if trimmed_brain_id.len() > 255 {
        let response = Json(ErrorResponse {
            error: "Brain ID too long (max 255 characters)".to_string(),
            code: "INVALID_BRAIN_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // EDGE CASE: Check byte length (prevent unicode abuse)
    if trimmed_brain_id.as_bytes().len() > 255 {
        let response = Json(ErrorResponse {
            error: "Brain ID byte length exceeds maximum".to_string(),
            code: "INVALID_BRAIN_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // EDGE CASE: Check for control characters and path traversal
    if trimmed_brain_id.chars().any(|c| c.is_control() || c == '\0' || c == '/' || c == '\\' || c == '.') {
        let response = Json(ErrorResponse {
            error: "Brain ID contains invalid characters".to_string(),
            code: "INVALID_BRAIN_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // SECURITY: Validate brain_id contains only safe characters
    if !trimmed_brain_id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        let response = Json(ErrorResponse {
            error: "Brain ID can only contain letters, numbers, underscores, and hyphens".to_string(),
            code: "INVALID_BRAIN_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }

    // Validate priority
    if !request.priority.is_finite() || request.priority < 0.0 || request.priority > 1.0 {
        let response = Json(ErrorResponse {
            error: "Priority must be a number between 0.0 and 1.0".to_string(),
            code: "INVALID_PRIORITY".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    info!("Creating thought for brain {}: {:?}", brain_id, request.content);
    
    match state.brain.create_thought(request.content, request.priority) {
        Ok(thought_id) => {
            (StatusCode::OK, Json(CreateThoughtResponse {
                success: true,
                thought_id,
                message: "Thought created successfully".to_string(),
            })).into_response()
        }
        Err(e) => {
            error!("Failed to create thought: {}", e);
            let response = Json(ErrorResponse {
                error: sanitize_error_message(&format!("Failed to create thought: {}", e), "CREATE_THOUGHT_ERROR"),
                code: "CREATE_THOUGHT_ERROR".to_string(),
            });
            (StatusCode::INTERNAL_SERVER_ERROR, response).into_response()
        }
    }
}

/// Store an experience (robot learning)
async fn store_experience_handler(
    State(state): State<ApiState>,
    Path(brain_id): Path<String>,
    Json(request): Json<StoreExperienceRequest>,
) -> impl IntoResponse {
    // SECURITY: Validate brain_id to prevent path traversal/injection
    // EDGE CASE: Handle empty, whitespace-only, unicode, control characters
    let trimmed_brain_id = brain_id.trim();
    
    if trimmed_brain_id.is_empty() {
        let response = Json(ErrorResponse {
            error: "Brain ID cannot be empty or whitespace only".to_string(),
            code: "INVALID_BRAIN_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    if trimmed_brain_id.len() > 255 {
        let response = Json(ErrorResponse {
            error: "Brain ID too long (max 255 characters)".to_string(),
            code: "INVALID_BRAIN_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // EDGE CASE: Check byte length (prevent unicode abuse)
    if trimmed_brain_id.as_bytes().len() > 255 {
        let response = Json(ErrorResponse {
            error: "Brain ID byte length exceeds maximum".to_string(),
            code: "INVALID_BRAIN_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // EDGE CASE: Check for control characters and path traversal
    if trimmed_brain_id.chars().any(|c| c.is_control() || c == '\0' || c == '/' || c == '\\' || c == '.') {
        let response = Json(ErrorResponse {
            error: "Brain ID contains invalid characters".to_string(),
            code: "INVALID_BRAIN_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // SECURITY: Validate brain_id contains only safe characters
    if !trimmed_brain_id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        let response = Json(ErrorResponse {
            error: "Brain ID can only contain letters, numbers, underscores, and hyphens".to_string(),
            code: "INVALID_BRAIN_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    info!("Storing experience for brain {}: {:?}", brain_id, request.observation);
    
    match state.brain.store_experience(
        "robot_experience".to_string(),
        request.observation,
        request.action,
        request.outcome,
        request.reward,
        None, // embedding
    ) {
        Ok(experience_id) => {
            (StatusCode::OK, Json(StoreExperienceResponse {
                success: true,
                experience_id,
                message: "Experience stored successfully".to_string(),
            })).into_response()
        }
        Err(e) => {
            error!("Failed to store experience: {}", e);
            let response = Json(ErrorResponse {
                error: sanitize_error_message(&format!("Failed to store experience: {}", e), "STORE_EXPERIENCE_ERROR"),
                code: "STORE_EXPERIENCE_ERROR".to_string(),
            });
            (StatusCode::INTERNAL_SERVER_ERROR, response).into_response()
        }
    }
}

/// Get thoughts (Thought Debugger)
async fn get_thoughts_handler(
    State(state): State<ApiState>,
    Path(brain_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    // Validate brain_id
    if brain_id.trim().is_empty() || brain_id.len() > 255 {
        return (StatusCode::BAD_REQUEST, Json(ErrorResponse {
            error: "Invalid brain ID".to_string(),
            code: "INVALID_BRAIN_ID".to_string(),
        })).into_response();
    }

    let state_filter = params.get("state").map(|s| match s.to_lowercase().as_str() {
        "active" => ThoughtState::Active,
        "paused" => ThoughtState::Paused,
        "completed" => ThoughtState::Completed,
        "merged" => ThoughtState::Merged,
        "discarded" => ThoughtState::Discarded,
        _ => ThoughtState::Active, // Default to active if unknown
    });

    let thoughts = state.brain.get_thoughts_by_state(state_filter);
    
    let thoughts_json: Vec<serde_json::Value> = thoughts.into_iter().map(|t| {
        serde_json::json!({
            "id": t.id,
            "thread_id": t.thread_id,
            "content": t.content,
            "state": format!("{:?}", t.state),
            "created_at": t.created_at,
            "updated_at": t.updated_at,
            "priority": t.priority,
            "associations": t.associations,
        })
    }).collect();

    Json(GetThoughtsResponse { thoughts: thoughts_json }).into_response()
}

/// Get memory accesses (Thought Debugger)
async fn get_memory_accesses_handler(
    State(state): State<ApiState>,
    Path(brain_id): Path<String>,
) -> impl IntoResponse {
    // Validate brain_id
    if brain_id.trim().is_empty() || brain_id.len() > 255 {
        return (StatusCode::BAD_REQUEST, Json(ErrorResponse {
            error: "Invalid brain ID".to_string(),
            code: "INVALID_BRAIN_ID".to_string(),
        })).into_response();
    }

    let accesses = state.brain.get_all_memory_accesses();
    
    let accesses_json: Vec<serde_json::Value> = accesses.into_iter().map(|a| {
        serde_json::json!({
            "memory_id": a.memory_id,
            "access_type": format!("{:?}", a.access_type),
            "timestamp": a.timestamp,
        })
    }).collect();

    Json(GetMemoryAccessesResponse { accesses: accesses_json }).into_response()
}

/// Get thought timeline (Thought Debugger)
async fn get_thought_timeline_handler(
    State(state): State<ApiState>,
    Path(brain_id): Path<String>,
) -> impl IntoResponse {
    // Validate brain_id
    if brain_id.trim().is_empty() || brain_id.len() > 255 {
        return (StatusCode::BAD_REQUEST, Json(ErrorResponse {
            error: "Invalid brain ID".to_string(),
            code: "INVALID_BRAIN_ID".to_string(),
        })).into_response();
    }

    let timeline = state.brain.get_thought_timeline();
    
    let timeline_json: Vec<serde_json::Value> = timeline.into_iter().map(|e| {
        // Extract thought_id based on event type
        let (type_str, thought_id) = match &e.event {
            narayana_storage::cognitive::CognitiveEvent::ThoughtCreated { thought_id } => ("ThoughtCreated", thought_id.clone()),
            narayana_storage::cognitive::CognitiveEvent::ThoughtCompleted { thought_id } => ("ThoughtCompleted", thought_id.clone()),
            narayana_storage::cognitive::CognitiveEvent::ThoughtDiscarded { thought_id } => ("ThoughtDiscarded", thought_id.clone()),
            narayana_storage::cognitive::CognitiveEvent::ThoughtMerged { to, .. } => ("ThoughtMerged", to.clone()),
            narayana_storage::cognitive::CognitiveEvent::MemoryFormed { .. } => ("MemoryFormed", "system".to_string()),
            _ => ("Other", "system".to_string()),
        };

        serde_json::json!({
            "type": type_str,
            "thought_id": thought_id,
            "timestamp": e.timestamp,
            "data": format!("{:?}", e.event),
        })
    }).collect();

    Json(GetThoughtTimelineResponse { timeline: timeline_json }).into_response()
}

/// Get conflicts (Thought Debugger)
async fn get_conflicts_handler(
    State(state): State<ApiState>,
    Path(brain_id): Path<String>,
) -> impl IntoResponse {
    // Validate brain_id
    if brain_id.trim().is_empty() || brain_id.len() > 255 {
        return (StatusCode::BAD_REQUEST, Json(ErrorResponse {
            error: "Invalid brain ID".to_string(),
            code: "INVALID_BRAIN_ID".to_string(),
        })).into_response();
    }

    let conflicts = state.brain.detect_conflicts();
    Json(GetConflictsResponse { conflicts }).into_response()
}

/// Cancel thought (Thought Debugger)
async fn cancel_thought_handler(
    State(state): State<ApiState>,
    Path((brain_id, thought_id)): Path<(String, String)>,
) -> impl IntoResponse {
    // Validate brain_id
    if brain_id.trim().is_empty() || brain_id.len() > 255 {
        return (StatusCode::BAD_REQUEST, Json(ErrorResponse {
            error: "Invalid brain ID".to_string(),
            code: "INVALID_BRAIN_ID".to_string(),
        })).into_response();
    }
    
    // Validate thought_id
    if thought_id.trim().is_empty() || thought_id.len() > 255 {
        return (StatusCode::BAD_REQUEST, Json(ErrorResponse {
            error: "Invalid thought ID".to_string(),
            code: "INVALID_THOUGHT_ID".to_string(),
        })).into_response();
    }

    match state.brain.cancel_thought(&thought_id) {
        Ok(_) => (StatusCode::OK, Json(CancelThoughtResponse {
            success: true,
            message: "Thought cancelled successfully".to_string(),
        })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string(),
            code: "CANCEL_THOUGHT_ERROR".to_string(),
        })).into_response(),
    }
}

/// Get memories (robot recall)
async fn get_memories_handler(
    State(state): State<ApiState>,
    Path(brain_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    // SECURITY: Validate brain_id to prevent path traversal/injection
    // EDGE CASE: Handle empty, whitespace-only, unicode, control characters
    let trimmed_brain_id = brain_id.trim();
    
    if trimmed_brain_id.is_empty() {
        let response = Json(ErrorResponse {
            error: "Brain ID cannot be empty or whitespace only".to_string(),
            code: "INVALID_BRAIN_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    if trimmed_brain_id.len() > 255 {
        let response = Json(ErrorResponse {
            error: "Brain ID too long (max 255 characters)".to_string(),
            code: "INVALID_BRAIN_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // EDGE CASE: Check byte length (prevent unicode abuse)
    if trimmed_brain_id.as_bytes().len() > 255 {
        let response = Json(ErrorResponse {
            error: "Brain ID byte length exceeds maximum".to_string(),
            code: "INVALID_BRAIN_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // EDGE CASE: Check for control characters and path traversal
    if trimmed_brain_id.chars().any(|c| c.is_control() || c == '\0' || c == '/' || c == '\\' || c == '.') {
        let response = Json(ErrorResponse {
            error: "Brain ID contains invalid characters".to_string(),
            code: "INVALID_BRAIN_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // SECURITY: Validate brain_id contains only safe characters
    if !trimmed_brain_id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        let response = Json(ErrorResponse {
            error: "Brain ID can only contain letters, numbers, underscores, and hyphens".to_string(),
            code: "INVALID_BRAIN_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // SECURITY: Limit number of query parameters to prevent DoS
    const MAX_QUERY_PARAMS: usize = 100;
    if params.len() > MAX_QUERY_PARAMS {
        error!("Too many query parameters: {} (max: {})", params.len(), MAX_QUERY_PARAMS);
        let response = Json(ErrorResponse {
            error: format!("Too many query parameters. Maximum is {}", MAX_QUERY_PARAMS),
            code: "TOO_MANY_PARAMS".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // SECURITY: Validate parameter key and value lengths
    for (key, value) in &params {
        if key.len() > 255 {
            error!("Query parameter key too long: {} chars", key.len());
            let response = Json(ErrorResponse {
                error: "Query parameter key too long".to_string(),
                code: "INVALID_PARAM".to_string(),
            });
            return (StatusCode::BAD_REQUEST, response).into_response();
        }
        if value.len() > 10_000 {
            error!("Query parameter value too long: {} chars", value.len());
            let response = Json(ErrorResponse {
                error: "Query parameter value too long".to_string(),
                code: "INVALID_PARAM".to_string(),
            });
            return (StatusCode::BAD_REQUEST, response).into_response();
        }
    }
    
    info!("Getting memories for brain {}: {:?}", brain_id, params);
    
    // SECURITY: Validate and sanitize memory type parameter
    // EDGE CASE: Handle empty, whitespace-only, control characters
    let memory_type_str = params.get("type")
        .map(|s| s.trim())
        .filter(|s| {
            // EDGE CASE: Check for empty, control characters, and length
            !s.is_empty() && 
            s.len() <= 50 && 
            s.as_bytes().len() <= 50 &&
            !s.chars().any(|c| c.is_control() || c == '\0')
        })
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| "episodic".to_string());
    
    // SECURITY: Validate limit parameter to prevent DoS
    let limit = params
        .get("limit")
        .and_then(|s| {
            let trimmed = s.trim();
            // SECURITY: Validate limit is a positive number within bounds
            if let Ok(parsed) = trimmed.parse::<u64>() {
                if parsed > 0 && parsed <= 10000 {
                    Some(parsed as usize)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .unwrap_or(10);
    
    // SECURITY: Validate memory type against whitelist to prevent injection
    let memory_type = match memory_type_str.as_str() {
        "episodic" => MemoryType::Episodic,
        "semantic" => MemoryType::Semantic,
        "procedural" => MemoryType::Procedural,
        "working" => MemoryType::Working,
        "spatial" => MemoryType::Spatial,
        "temporal" => MemoryType::Temporal,
        "associative" => MemoryType::Associative,
        "emotional" => MemoryType::Emotional,
        _ => MemoryType::Episodic,
    };
    
    // Get memories using temporal retrieval with wide range, then filter by type
    // This is a workaround since memories field is private
    // In production, we'd add a public method to retrieve by type
    let start_time = 0u64;
    let end_time = std::u64::MAX;
    
    let memories_result = state.brain.retrieve_memories_temporal(start_time, end_time);
    
    let memories: Vec<MemoryResponse> = match memories_result {
        Ok(all_memories) => {
            all_memories
                .into_iter()
                .filter(|m| m.memory_type == memory_type)
                .take(limit)
                .map(|m| MemoryResponse {
                    id: m.id.clone(),
                    memory_type: format!("{:?}", m.memory_type),
                    content: m.content.clone(),
                    created_at: m.created_at,
                })
                .collect()
        }
        Err(_) => Vec::new(),
    };
    
    let count = memories.len();
    
    (StatusCode::OK, Json(GetMemoriesResponse {
        memories,
        count,
    })).into_response()
}

#[derive(Debug, Serialize)]
struct GetBrainsResponse {
    brains: Vec<BrainInfo>,
    count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct BrainInfo {
    brain_id: String,
    memory_types: Vec<String>,
    created_at: Option<u64>,
}

/// Get all brains
async fn get_brains_handler(State(state): State<ApiState>) -> impl IntoResponse {
    info!("Getting all brains");
    
    // For now, return a single default brain
    // In a real implementation, we'd track multiple brains
    let brains = vec![BrainInfo {
        brain_id: "default".to_string(),
        memory_types: vec![
            "episodic".to_string(),
            "semantic".to_string(),
            "procedural".to_string(),
            "spatial".to_string(),
        ],
        created_at: Some(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()),
    }];
    
    (StatusCode::OK, Json(GetBrainsResponse {
        brains: brains.clone(),
        count: brains.len(),
    })).into_response()
}

#[derive(Debug, Serialize)]
struct GetWorkersResponse {
    workers: Vec<WorkerInfo>,
    count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct WorkerInfo {
    worker_id: String,
    name: String,
    route: String,
    active: bool,
    created_at: Option<u64>,
}

/// Get all workers
async fn get_workers_handler(State(state): State<ApiState>) -> impl IntoResponse {
    info!("Getting all workers");
    
    // Get workers from worker manager
    // Note: This requires accessing the worker manager's internal state
    // For now, return empty list - would need proper API in worker manager
    let workers = Vec::<WorkerInfo>::new();
    
    let count = workers.len();
    (StatusCode::OK, Json(GetWorkersResponse {
        workers,
        count,
    })).into_response()
}

#[derive(Debug, Serialize)]
struct SystemStatsResponse {
    tables: u64,
    brains: usize,
    workers: usize,
    active_connections: usize,
    total_queries: u64,
    avg_latency_ms: f64,
    total_rows_read: u64,
    total_rows_inserted: u64,
}

/// Get comprehensive system statistics
async fn get_system_stats_handler(State(state): State<ApiState>) -> impl IntoResponse {
    info!("Getting system stats");
    
    // Get query stats
    let total_queries = TOTAL_QUERIES.load(Ordering::Relaxed);
    let total_rows_read = TOTAL_ROWS_READ.load(Ordering::Relaxed);
    let total_rows_inserted = TOTAL_ROWS_INSERTED.load(Ordering::Relaxed);
    let total_query_time = TOTAL_QUERY_TIME_MS.load(Ordering::Relaxed);
    
    let avg_latency_ms = if total_queries > 0 {
        total_query_time as f64 / total_queries as f64
    } else {
        0.0
    };
    
    // Get table count (would need proper implementation)
    let tables = 0u64;
    
    // Get brain count (default to 1 for now)
    let brains = 1usize;
    
    // Get worker count (would need proper implementation)
    let workers = 0usize;
    
    // Get active connections (would need WebSocket manager)
    let active_connections = 0usize;
    
    (StatusCode::OK, Json(SystemStatsResponse {
        tables,
        brains,
        workers,
        active_connections,
        total_queries,
        avg_latency_ms,
        total_rows_read,
        total_rows_inserted,
    })).into_response()
}

// Webhook API handlers

#[derive(Debug, Serialize)]
struct GetWebhooksResponse {
    webhooks: Vec<WebhookInfo>,
    count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct WebhookInfo {
    id: String,
    name: String,
    url: String,
    enabled: bool,
    events: Vec<String>,
    scope: String,
    retry_count: u32,
    created_at: u64,
    updated_at: u64,
    total_deliveries: u64,
    successful_deliveries: u64,
    failed_deliveries: u64,
}

/// Get all webhooks
async fn get_webhooks_handler(State(state): State<ApiState>) -> impl IntoResponse {
    info!("Getting all webhooks");
    
    let webhooks = state.webhook_manager.list_webhooks();
    let webhook_infos: Vec<WebhookInfo> = webhooks
        .iter()
        .map(|w| WebhookInfo {
            id: w.id.clone(),
            name: w.name.clone(),
            url: w.url.clone(),
            enabled: w.enabled,
            events: w.events.iter().map(|e| format!("{:?}", e)).collect(),
            scope: format!("{:?}", w.scope),
            retry_count: w.retry_count,
            created_at: w.created_at,
            updated_at: w.updated_at,
            total_deliveries: 0, // TODO: Track in webhook manager
            successful_deliveries: 0,
            failed_deliveries: 0,
        })
        .collect();
    
    (StatusCode::OK, Json(GetWebhooksResponse {
        webhooks: webhook_infos.clone(),
        count: webhook_infos.len(),
    })).into_response()
}

#[derive(Debug, Deserialize)]
struct CreateWebhookRequest {
    name: String,
    url: String,
    events: Vec<String>,
    scope: String,
    secret: Option<String>,
    retry_count: Option<u32>,
}

/// Create a new webhook
async fn create_webhook_handler(
    State(state): State<ApiState>,
    Json(request): Json<CreateWebhookRequest>,
) -> impl IntoResponse {
    info!("Creating webhook: {}", request.name);
    
    // Parse events
    use narayana_storage::webhooks::WebhookEventType;
    let events: Vec<WebhookEventType> = request
        .events
        .iter()
        .map(|e| match e.as_str() {
            "Insert" => WebhookEventType::Insert,
            "Update" => WebhookEventType::Update,
            "Delete" => WebhookEventType::Delete,
            "Create" => WebhookEventType::Create,
            "Drop" => WebhookEventType::Drop,
            "Alter" => WebhookEventType::Alter,
            "Query" => WebhookEventType::Query,
            "Transaction" => WebhookEventType::Transaction,
            _ => WebhookEventType::Custom(e.clone()),
        })
        .collect();
    
    // Parse scope (simplified - would need proper parsing)
    use narayana_storage::webhooks::{WebhookScope, PayloadFormat};
    let scope = if request.scope == "Global" {
        WebhookScope::Global
    } else {
        // For now, default to Global - would need proper parsing
        WebhookScope::Global
    };
    
    let config = narayana_storage::webhooks::WebhookConfig::new(
        request.name,
        request.url,
        scope,
        events,
        PayloadFormat::Json,
    );
    
    match state.webhook_manager.create_webhook(config) {
        Ok(id) => {
            (StatusCode::OK, Json(serde_json::json!({
                "success": true,
                "webhook_id": id,
                "message": "Webhook created successfully"
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to create webhook: {}", e);
            let response = Json(ErrorResponse {
                error: sanitize_error_message(&format!("Failed to create webhook: {}", e), "CREATE_WEBHOOK_ERROR"),
                code: "CREATE_WEBHOOK_ERROR".to_string(),
            });
            (StatusCode::INTERNAL_SERVER_ERROR, response).into_response()
        }
    }
}

/// Get a specific webhook
async fn get_webhook_handler(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // SECURITY: Validate webhook ID to prevent injection
    if id.trim().is_empty() || id.len() > 255 {
        let response = Json(ErrorResponse {
            error: "Invalid webhook ID".to_string(),
            code: "INVALID_WEBHOOK_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // SECURITY: Validate webhook ID contains only safe characters (UUID format typically)
    if !id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        let response = Json(ErrorResponse {
            error: "Invalid webhook ID format".to_string(),
            code: "INVALID_WEBHOOK_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    info!("Getting webhook: {}", id);
    
    match state.webhook_manager.get_webhook(&id) {
        Some(webhook) => {
            let info = WebhookInfo {
                id: webhook.id.clone(),
                name: webhook.name.clone(),
                url: webhook.url.clone(),
                enabled: webhook.enabled,
                events: webhook.events.iter().map(|e| format!("{:?}", e)).collect(),
                scope: format!("{:?}", webhook.scope),
                retry_count: webhook.retry_count,
                created_at: webhook.created_at,
                updated_at: webhook.updated_at,
                total_deliveries: 0,
                successful_deliveries: 0,
                failed_deliveries: 0,
            };
            (StatusCode::OK, Json(info)).into_response()
        }
        None => {
            let response = Json(ErrorResponse {
                error: sanitize_error_message(&format!("Webhook {} not found", id), "WEBHOOK_NOT_FOUND"),
                code: "WEBHOOK_NOT_FOUND".to_string(),
            });
            (StatusCode::NOT_FOUND, response).into_response()
        }
    }
}

/// Delete a webhook
async fn delete_webhook_handler(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // SECURITY: Validate webhook ID
    // EDGE CASE: Handle empty, whitespace-only, unicode, control characters
    let trimmed_id = id.trim();
    
    if trimmed_id.is_empty() {
        let response = Json(ErrorResponse {
            error: "Webhook ID cannot be empty or whitespace only".to_string(),
            code: "INVALID_WEBHOOK_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    if trimmed_id.len() > 255 {
        let response = Json(ErrorResponse {
            error: "Webhook ID too long (max 255 characters)".to_string(),
            code: "INVALID_WEBHOOK_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // EDGE CASE: Check byte length (prevent unicode abuse)
    if trimmed_id.as_bytes().len() > 255 {
        let response = Json(ErrorResponse {
            error: "Webhook ID byte length exceeds maximum".to_string(),
            code: "INVALID_WEBHOOK_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // EDGE CASE: Check for control characters and path traversal
    if trimmed_id.chars().any(|c| c.is_control() || c == '\0' || c == '/' || c == '\\' || c == '.') {
        let response = Json(ErrorResponse {
            error: "Webhook ID contains invalid characters".to_string(),
            code: "INVALID_WEBHOOK_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    if !trimmed_id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        let response = Json(ErrorResponse {
            error: "Webhook ID can only contain letters, numbers, underscores, and hyphens".to_string(),
            code: "INVALID_WEBHOOK_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    info!("Deleting webhook: {}", id);
    
    match state.webhook_manager.delete_webhook(&id) {
        Ok(_) => {
            (StatusCode::OK, Json(serde_json::json!({
                "success": true,
                "message": format!("Webhook {} deleted", id)
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to delete webhook: {}", e);
            let response = Json(ErrorResponse {
                error: sanitize_error_message(&format!("Failed to delete webhook: {}", e), "DELETE_WEBHOOK_ERROR"),
                code: "DELETE_WEBHOOK_ERROR".to_string(),
            });
            (StatusCode::INTERNAL_SERVER_ERROR, response).into_response()
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct DeliveryInfo {
    id: String,
    webhook_id: String,
    status: String, // "pending", "processing", "success", "failed"
    attempt: u32,
    max_attempts: u32,
    created_at: u64,
    completed_at: Option<u64>,
    error: Option<String>,
    response_status: Option<u16>,
    duration_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
struct GetDeliveriesResponse {
    deliveries: Vec<DeliveryInfo>,
    count: usize,
    total: usize,
}

/// Get webhook delivery history
async fn get_webhook_deliveries_handler(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    // SECURITY: Validate webhook ID
    // EDGE CASE: Handle empty, whitespace-only, unicode, control characters
    let trimmed_id = id.trim();
    
    if trimmed_id.is_empty() {
        let response = Json(ErrorResponse {
            error: "Webhook ID cannot be empty or whitespace only".to_string(),
            code: "INVALID_WEBHOOK_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    if trimmed_id.len() > 255 {
        let response = Json(ErrorResponse {
            error: "Webhook ID too long (max 255 characters)".to_string(),
            code: "INVALID_WEBHOOK_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // EDGE CASE: Check byte length (prevent unicode abuse)
    if trimmed_id.as_bytes().len() > 255 {
        let response = Json(ErrorResponse {
            error: "Webhook ID byte length exceeds maximum".to_string(),
            code: "INVALID_WEBHOOK_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // EDGE CASE: Check for control characters and path traversal
    if trimmed_id.chars().any(|c| c.is_control() || c == '\0' || c == '/' || c == '\\' || c == '.') {
        let response = Json(ErrorResponse {
            error: "Webhook ID contains invalid characters".to_string(),
            code: "INVALID_WEBHOOK_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    if !trimmed_id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        let response = Json(ErrorResponse {
            error: "Webhook ID can only contain letters, numbers, underscores, and hyphens".to_string(),
            code: "INVALID_WEBHOOK_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // SECURITY: Limit number of query parameters to prevent DoS
    const MAX_QUERY_PARAMS: usize = 100;
    if params.len() > MAX_QUERY_PARAMS {
        error!("Too many query parameters: {} (max: {})", params.len(), MAX_QUERY_PARAMS);
        let response = Json(ErrorResponse {
            error: format!("Too many query parameters. Maximum is {}", MAX_QUERY_PARAMS),
            code: "TOO_MANY_PARAMS".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // SECURITY: Validate parameter key and value lengths
    for (key, value) in &params {
        if key.len() > 255 {
            error!("Query parameter key too long: {} chars", key.len());
            let response = Json(ErrorResponse {
                error: "Query parameter key too long".to_string(),
                code: "INVALID_PARAM".to_string(),
            });
            return (StatusCode::BAD_REQUEST, response).into_response();
        }
        if value.len() > 10_000 {
            error!("Query parameter value too long: {} chars", value.len());
            let response = Json(ErrorResponse {
                error: "Query parameter value too long".to_string(),
                code: "INVALID_PARAM".to_string(),
            });
            return (StatusCode::BAD_REQUEST, response).into_response();
        }
    }
    
    info!("Getting deliveries for webhook: {}", id);
    
    // SECURITY: Validate limit parameter to prevent DoS
    let limit = params
        .get("limit")
        .and_then(|s| {
            let trimmed = s.trim();
            // SECURITY: Validate limit is a positive number within bounds
            if let Ok(parsed) = trimmed.parse::<u64>() {
                if parsed > 0 && parsed <= 10000 {
                    Some(parsed as usize)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .unwrap_or(50);
    
    // TODO: Implement delivery tracking in webhook manager
    // For now, return empty list
    let deliveries = Vec::<DeliveryInfo>::new();
    
    let count = deliveries.len();
    (StatusCode::OK, Json(GetDeliveriesResponse {
        deliveries,
        count,
        total: 0,
    })).into_response()
}

/// Enable a webhook
async fn enable_webhook_handler(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // SECURITY: Validate webhook ID
    // EDGE CASE: Handle empty, whitespace-only, unicode, control characters
    let trimmed_id = id.trim();
    
    if trimmed_id.is_empty() {
        let response = Json(ErrorResponse {
            error: "Webhook ID cannot be empty or whitespace only".to_string(),
            code: "INVALID_WEBHOOK_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    if trimmed_id.len() > 255 {
        let response = Json(ErrorResponse {
            error: "Webhook ID too long (max 255 characters)".to_string(),
            code: "INVALID_WEBHOOK_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // EDGE CASE: Check byte length (prevent unicode abuse)
    if trimmed_id.as_bytes().len() > 255 {
        let response = Json(ErrorResponse {
            error: "Webhook ID byte length exceeds maximum".to_string(),
            code: "INVALID_WEBHOOK_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // EDGE CASE: Check for control characters and path traversal
    if trimmed_id.chars().any(|c| c.is_control() || c == '\0' || c == '/' || c == '\\' || c == '.') {
        let response = Json(ErrorResponse {
            error: "Webhook ID contains invalid characters".to_string(),
            code: "INVALID_WEBHOOK_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    if !trimmed_id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        let response = Json(ErrorResponse {
            error: "Webhook ID can only contain letters, numbers, underscores, and hyphens".to_string(),
            code: "INVALID_WEBHOOK_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    info!("Enabling webhook: {}", id);
    
    match state.webhook_manager.enable_webhook(&id) {
        Ok(_) => {
            (StatusCode::OK, Json(serde_json::json!({
                "success": true,
                "message": format!("Webhook {} enabled", id)
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to enable webhook: {}", e);
            let response = Json(ErrorResponse {
                error: sanitize_error_message(&format!("Failed to enable webhook: {}", e), "ENABLE_WEBHOOK_ERROR"),
                code: "ENABLE_WEBHOOK_ERROR".to_string(),
            });
            (StatusCode::INTERNAL_SERVER_ERROR, response).into_response()
        }
    }
}

/// Disable a webhook
async fn disable_webhook_handler(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // SECURITY: Validate webhook ID
    // EDGE CASE: Handle empty, whitespace-only, unicode, control characters
    let trimmed_id = id.trim();
    
    if trimmed_id.is_empty() {
        let response = Json(ErrorResponse {
            error: "Webhook ID cannot be empty or whitespace only".to_string(),
            code: "INVALID_WEBHOOK_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    if trimmed_id.len() > 255 {
        let response = Json(ErrorResponse {
            error: "Webhook ID too long (max 255 characters)".to_string(),
            code: "INVALID_WEBHOOK_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // EDGE CASE: Check byte length (prevent unicode abuse)
    if trimmed_id.as_bytes().len() > 255 {
        let response = Json(ErrorResponse {
            error: "Webhook ID byte length exceeds maximum".to_string(),
            code: "INVALID_WEBHOOK_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    // EDGE CASE: Check for control characters and path traversal
    if trimmed_id.chars().any(|c| c.is_control() || c == '\0' || c == '/' || c == '\\' || c == '.') {
        let response = Json(ErrorResponse {
            error: "Webhook ID contains invalid characters".to_string(),
            code: "INVALID_WEBHOOK_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    if !trimmed_id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        let response = Json(ErrorResponse {
            error: "Webhook ID can only contain letters, numbers, underscores, and hyphens".to_string(),
            code: "INVALID_WEBHOOK_ID".to_string(),
        });
        return (StatusCode::BAD_REQUEST, response).into_response();
    }
    
    info!("Disabling webhook: {}", id);
    
    match state.webhook_manager.disable_webhook(&id) {
        Ok(_) => {
            (StatusCode::OK, Json(serde_json::json!({
                "success": true,
                "message": format!("Webhook {} disabled", id)
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to disable webhook: {}", e);
            let response = Json(ErrorResponse {
                error: sanitize_error_message(&format!("Failed to disable webhook: {}", e), "DISABLE_WEBHOOK_ERROR"),
                code: "DISABLE_WEBHOOK_ERROR".to_string(),
            });
            (StatusCode::INTERNAL_SERVER_ERROR, response).into_response()
        }
    }
}


/// Load schema from schema.nyn file
async fn load_schema_handler(State(state): State<ApiState>) -> impl IntoResponse {
    use crate::schema_loader;
    use std::path::Path;
    
    info!("Loading schema from ./schema/schema.nyn");
    
    let schema_dir = Path::new("./schema");
    if !schema_dir.exists() {
        let response = Json(ErrorResponse {
            error: "Schema directory not found: ./schema".to_string(),
            code: "SCHEMA_DIR_NOT_FOUND".to_string(),
        });
        return (StatusCode::NOT_FOUND, response).into_response();
    }
    
    match schema_loader::load_schema(
        &schema_dir.join("schema.nyn"),
        state.db_manager.clone(),
        state.storage.clone(),
    ).await {
        Ok(table_ids) => {
            info!("Schema loaded successfully: {} tables created", table_ids.len());
            (StatusCode::OK, Json(serde_json::json!({
                "success": true,
                "message": format!("Schema loaded: {} tables created", table_ids.len()),
                "tables": table_ids.iter().map(|(name, id)| serde_json::json!({
                    "name": name,
                    "id": id.0
                })).collect::<Vec<_>>()
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to load schema: {}", e);
            let response = Json(ErrorResponse {
                error: format!("Failed to load schema: {}", e),
                code: "SCHEMA_LOAD_ERROR".to_string(),
            });
            (StatusCode::INTERNAL_SERVER_ERROR, response).into_response()
        }
    }
}

/// Load seeds from seeds.nyn file
async fn load_seeds_handler(State(state): State<ApiState>) -> impl IntoResponse {
    use crate::schema_loader;
    use std::path::Path;
    use std::collections::HashMap;
    
    info!("Loading seeds from ./schema/seeds.nyn");
    
    let schema_dir = Path::new("./schema");
    if !schema_dir.exists() {
        let response = Json(ErrorResponse {
            error: "Schema directory not found: ./schema".to_string(),
            code: "SCHEMA_DIR_NOT_FOUND".to_string(),
        });
        return (StatusCode::NOT_FOUND, response).into_response();
    }
    
    // Get all existing tables to map names to IDs
    let db_id = match state.db_manager.get_database_by_name("default") {
        Some(id) => id,
        None => {
            let response = Json(ErrorResponse {
                error: "Default database not found".to_string(),
                code: "DATABASE_NOT_FOUND".to_string(),
            });
            return (StatusCode::NOT_FOUND, response).into_response();
        }
    };
    
    let tables = match state.db_manager.list_tables(db_id) {
        Ok(tables) => tables,
        Err(e) => {
            let response = Json(ErrorResponse {
                error: format!("Failed to list tables: {}", e),
                code: "LIST_TABLES_ERROR".to_string(),
            });
            return (StatusCode::INTERNAL_SERVER_ERROR, response).into_response();
        }
    };
    
    let mut table_ids = HashMap::new();
    for table in tables {
        table_ids.insert(table.name, table.table_id);
    }
    
    match schema_loader::load_seeds(
        &schema_dir.join("seeds.nyn"),
        &table_ids,
        state.db_manager.clone(),
        state.storage.clone(),
    ).await {
        Ok(_) => {
            info!("Seeds loaded successfully");
            (StatusCode::OK, Json(serde_json::json!({
                "success": true,
                "message": "Seeds loaded successfully"
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to load seeds: {}", e);
            let response = Json(ErrorResponse {
                error: format!("Failed to load seeds: {}", e),
                code: "SEEDS_LOAD_ERROR".to_string(),
            });
            (StatusCode::INTERNAL_SERVER_ERROR, response).into_response()
        }
    }
}

/// Load both schema and seeds (spawn)
async fn spawn_schema_handler(State(state): State<ApiState>) -> impl IntoResponse {
    use crate::schema_loader;
    use std::path::Path;
    
    info!("Spawning schema and seeds from ./schema");
    
    let schema_dir = Path::new("./schema");
    if !schema_dir.exists() {
        let response = Json(ErrorResponse {
            error: "Schema directory not found: ./schema".to_string(),
            code: "SCHEMA_DIR_NOT_FOUND".to_string(),
        });
        return (StatusCode::NOT_FOUND, response).into_response();
    }
    
    match schema_loader::load_schema_and_seeds(
        schema_dir,
        state.db_manager.clone(),
        state.storage.clone(),
    ).await {
        Ok(_) => {
            info!("Schema and seeds spawned successfully");
            (StatusCode::OK, Json(serde_json::json!({
                "success": true,
                "message": "Schema and seeds loaded successfully"
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to spawn schema/seeds: {}", e);
            let response = Json(ErrorResponse {
                error: format!("Failed to spawn schema/seeds: {}", e),
                code: "SPAWN_ERROR".to_string(),
            });
            (StatusCode::INTERNAL_SERVER_ERROR, response).into_response()
        }
    }
}
