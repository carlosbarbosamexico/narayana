// Security module for encrypted connections and secure authentication

use axum::{
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
    extract::Request,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use tracing::{error, warn, info};

/// JWT claims for authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // Subject (user ID)
    pub exp: usize,  // Expiration time
    pub iat: usize, // Issued at
    pub roles: Vec<String>, // User roles
}

/// Secure token manager for authentication
pub struct TokenManager {
    encoding_key: Arc<Mutex<EncodingKey>>,
    decoding_key: Arc<Mutex<DecodingKey>>,
    secret: Arc<RwLock<String>>, // Rotated secrets
}

impl TokenManager {
    pub fn new(secret: String) -> Self {
        let encoding_key = EncodingKey::from_secret(secret.as_ref());
        let decoding_key = DecodingKey::from_secret(secret.as_ref());
        
        Self {
            encoding_key: Arc::new(Mutex::new(encoding_key)),
            decoding_key: Arc::new(Mutex::new(decoding_key)),
            secret: Arc::new(RwLock::new(secret)),
        }
    }

    /// Generate a secure JWT token
    pub fn generate_token(&self, user_id: String, roles: Vec<String>) -> Result<String, SecurityError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as usize;
        
        let claims = Claims {
            sub: user_id,
            exp: now + 3600, // 1 hour expiration
            iat: now,
            roles,
        };

        let encoding_key = self.encoding_key.lock().unwrap();
        encode(&Header::default(), &claims, &*encoding_key)
            .map_err(|e| SecurityError::TokenGeneration(e.to_string()))
    }

    /// Verify and decode a JWT token
    pub fn verify_token(&self, token: &str) -> Result<Claims, SecurityError> {
        let validation = Validation::default();
        let decoding_key = self.decoding_key.lock().unwrap();
        let token_data = decode::<Claims>(token, &*decoding_key, &validation)
            .map_err(|e| SecurityError::TokenVerification(e.to_string()))?;
        
        Ok(token_data.claims)
    }

    /// Rotate secret keys securely
    /// 
    /// This updates the secret and encoding/decoding keys. In production,
    /// you would want to support a grace period where both old and new keys
    /// are accepted to allow existing tokens to continue working during rotation.
    pub async fn rotate_secret(&self, new_secret: String) {
        // Update secret
        let mut secret = self.secret.write().await;
        *secret = new_secret.clone();
        drop(secret); // Release write lock
        
        // Update encoding and decoding keys atomically
        // Note: This will invalidate all existing tokens immediately
        // For production, consider implementing a dual-key system with grace period
        let encoding_key = EncodingKey::from_secret(new_secret.as_ref());
        let decoding_key = DecodingKey::from_secret(new_secret.as_ref());
        
        {
            let mut enc_key = self.encoding_key.lock().unwrap();
            *enc_key = encoding_key;
        }
        
        {
            let mut dec_key = self.decoding_key.lock().unwrap();
            *dec_key = decoding_key;
        }
        
        // In a production system, you might want to:
        // 1. Store both old and new keys temporarily
        // 2. Accept tokens signed with either key during grace period
        // 3. Log rotation events for audit purposes
        // For now, we do immediate rotation which invalidates old tokens
        info!("JWT secret key rotated successfully");
    }
}

/// API key manager for secure API access
pub struct ApiKeyManager {
    keys: Arc<RwLock<std::collections::HashMap<String, ApiKeyInfo>>>,
}

#[derive(Clone, Debug)]
pub struct ApiKeyInfo {
    pub key_hash: String, // Hashed key, never store plaintext
    pub permissions: Vec<String>,
    pub created_at: std::time::SystemTime,
    pub expires_at: Option<std::time::SystemTime>,
}

impl ApiKeyManager {
    pub fn new() -> Self {
        Self {
            keys: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Generate a new API key (returns the key only once)
    pub async fn generate_key(&self, permissions: Vec<String>) -> Result<String, SecurityError> {
        // SECURITY: Use cryptographically secure hash (SHA-256) instead of DefaultHasher
        // DefaultHasher is vulnerable to hash collision attacks and timing attacks
        use sha2::{Sha256, Digest};
        use uuid::Uuid;
        
        // Generate secure random key
        let key = format!("nar_{}", Uuid::new_v4().to_string().replace("-", ""));
        
        // Hash the key immediately (never store plaintext) using SHA-256
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let key_hash = format!("{:x}", hasher.finalize());
        
        let info = ApiKeyInfo {
            key_hash: key_hash.clone(),
            permissions,
            created_at: std::time::SystemTime::now(),
            expires_at: None,
        };
        
        // SECURITY: Store by hash, not by original key (prevents key exposure)
        let mut keys = self.keys.write().await;
        keys.insert(key_hash, info);
        
        Ok(key)
    }

    /// Verify API key (returns permissions if valid)
    /// SECURITY: Fixed - now correctly looks up by hash, not original key
    /// SECURITY: Use SHA-256 instead of DefaultHasher to prevent hash collision attacks
    pub async fn verify_key(&self, key: &str) -> Result<Vec<String>, SecurityError> {
        use sha2::{Sha256, Digest};
        
        // Hash the provided key to match stored hash using SHA-256
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let key_hash = format!("{:x}", hasher.finalize());
        
        let keys = self.keys.read().await;
        // SECURITY: Look up by hash, not original key
        if let Some(info) = keys.get(&key_hash) {
            // Check expiration
            if let Some(expires_at) = info.expires_at {
                if expires_at < std::time::SystemTime::now() {
                    return Err(SecurityError::KeyExpired);
                }
            }
            
            Ok(info.permissions.clone())
        } else {
            Err(SecurityError::InvalidKey)
        }
    }

    /// Revoke an API key
    /// SECURITY: Fixed - now correctly revokes by hash
    /// SECURITY: Use SHA-256 instead of DefaultHasher to prevent hash collision attacks
    pub async fn revoke_key(&self, key: &str) -> Result<(), SecurityError> {
        use sha2::{Sha256, Digest};
        
        // Hash the provided key to match stored hash using SHA-256
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let key_hash = format!("{:x}", hasher.finalize());
        
        let mut keys = self.keys.write().await;
        // SECURITY: Remove by hash, not original key
        keys.remove(&key_hash);
        Ok(())
    }
}

/// Security errors
#[derive(Debug)]
pub enum SecurityError {
    TokenGeneration(String),
    TokenVerification(String),
    InvalidKey,
    KeyExpired,
    Unauthorized,
    Forbidden,
    EncryptionFailed(String),
}

impl std::fmt::Display for SecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityError::TokenGeneration(e) => write!(f, "Token generation failed: {}", e),
            SecurityError::TokenVerification(e) => write!(f, "Token verification failed: {}", e),
            SecurityError::InvalidKey => write!(f, "Invalid API key"),
            SecurityError::KeyExpired => write!(f, "API key expired"),
            SecurityError::Unauthorized => write!(f, "Unauthorized"),
            SecurityError::Forbidden => write!(f, "Forbidden"),
            SecurityError::EncryptionFailed(e) => write!(f, "Encryption/decryption failed: {}", e),
        }
    }
}

impl std::error::Error for SecurityError {}

/// Authentication middleware for Axum
/// SECURITY: Fixed authentication bypass - now requires proper token verification
pub async fn auth_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract authorization header
    let auth_header = headers.get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Check for Bearer token
    if let Some(token) = auth_header.strip_prefix("Bearer ") {
        // SECURITY: Proper token verification required
        // Try to get TokenManager from request extensions
        // SECURITY: Proper JWT verification should be enabled in production
        if let Some(token_manager) = request.extensions().get::<std::sync::Arc<TokenManager>>() {
            // Verify token signature and expiration
            match token_manager.verify_token(token) {
                Ok(_claims) => {
                    // Token is valid, proceed
                    return Ok(next.run(request).await);
                }
                Err(_) => {
                    // Invalid token
                    return Err(StatusCode::UNAUTHORIZED);
                }
            }
        }
        
        // SECURITY: Fallback validation removed - require proper TokenManager
        // Weak fallback validation is a security risk
        warn!("JWT token validation failed: TokenManager not available in request extensions");
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Check for API key
    if let Some(api_key) = headers.get("x-api-key")
        .and_then(|h| h.to_str().ok()) {
        // SECURITY: Proper API key verification required
        // Try to get ApiKeyManager from request extensions
        // SECURITY: Proper API key verification should be enabled in production
        if let Some(api_key_manager) = request.extensions().get::<std::sync::Arc<ApiKeyManager>>() {
            // Verify API key against stored hashes
            match api_key_manager.verify_key(api_key).await {
                Ok(_permissions) => {
                    // API key is valid, proceed
                    return Ok(next.run(request).await);
                }
                Err(_) => {
                    // Invalid API key
                    return Err(StatusCode::UNAUTHORIZED);
                }
            }
        }
        
        // SECURITY: No fallback validation - require ApiKeyManager
        // Fallback validation is a security risk - allows bypass if manager not configured
        warn!("API key validation failed: ApiKeyManager not available in request extensions");
        return Err(StatusCode::UNAUTHORIZED);
    }

    Err(StatusCode::UNAUTHORIZED)
}

/// Rate limiting middleware for security
pub struct RateLimiter {
    requests: Arc<RwLock<std::collections::HashMap<String, Vec<std::time::Instant>>>>,
    max_requests: usize,
    window_seconds: u64,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_seconds: u64) -> Self {
        Self {
            requests: Arc::new(RwLock::new(std::collections::HashMap::new())),
            max_requests,
            window_seconds,
        }
    }

    pub async fn check_rate_limit(&self, identifier: &str) -> Result<(), SecurityError> {
        // SECURITY: Validate identifier to prevent DoS via hash collision attacks
        if identifier.len() > 256 {
            return Err(SecurityError::Forbidden); // Reject extremely long identifiers
        }
        
        let mut requests = self.requests.write().await;
        let now = std::time::Instant::now();
        let window = std::time::Duration::from_secs(self.window_seconds);
        
        // SECURITY: Prevent unbounded HashMap growth - cleanup old entries periodically
        if requests.len() > 100_000 {
            // Cleanup entries with no recent activity
            requests.retain(|_id, times| {
                times.retain(|&time| now.duration_since(time) < window * 2);
                !times.is_empty()
            });
        }
        
        let entry = requests.entry(identifier.to_string()).or_insert_with(Vec::new);
        
        // Remove old requests outside the window
        entry.retain(|&time| now.duration_since(time) < window);
        
        if entry.len() >= self.max_requests {
            return Err(SecurityError::Forbidden);
        }
        
        entry.push(now);
        Ok(())
    }
}

/// Secure configuration manager (no keys in logs)
pub struct SecureConfig {
    secrets: Arc<RwLock<std::collections::HashMap<String, SecureValue>>>,
}

#[derive(Clone)]
enum SecureValue {
    String(String),
    Bytes(Vec<u8>),
}

impl SecureConfig {
    pub fn new() -> Self {
        Self {
            secrets: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Store a secret value (never logged)
    pub async fn set_secret(&self, key: &str, value: String) {
        let mut secrets = self.secrets.write().await;
        secrets.insert(key.to_string(), SecureValue::String(value));
    }

    /// Get a secret value (never logged)
    pub async fn get_secret(&self, key: &str) -> Option<String> {
        let secrets = self.secrets.read().await;
        match secrets.get(key) {
            Some(SecureValue::String(s)) => Some(s.clone()),
            _ => None,
        }
    }

    /// Load secrets from environment (secure)
    pub fn from_env() -> Self {
        let config = Self::new();
        // In production, would load from secure vault or environment
        config
    }
}

impl std::fmt::Debug for SecureConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print secrets in debug output
        write!(f, "SecureConfig {{ secrets: [REDACTED] }}")
    }
}

/// Encryption at rest manager
pub struct EncryptionManager {
    master_key: Arc<RwLock<Vec<u8>>>, // Encrypted master key
}

impl EncryptionManager {
    pub fn new() -> Self {
        Self {
            master_key: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Encrypt data at rest using AES-256-GCM
    pub async fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, SecurityError> {
        use aes_gcm::{
            aead::{Aead, AeadCore, KeyInit, OsRng},
            Aes256Gcm, Nonce,
        };
        use pbkdf2::pbkdf2_hmac;
        use sha2::Sha256 as Sha256Hash;
        
        let master_key = self.master_key.read().await;
        if master_key.is_empty() {
            return Err(SecurityError::Unauthorized);
        }
        
        // SECURITY: Derive encryption key using PBKDF2 for key stretching
        // This prevents rainbow table attacks and provides proper key derivation
        
        const PBKDF2_ITERATIONS: u32 = 100_000; // OWASP recommended minimum
        const SALT: &[u8] = b"narayana-encryption-salt"; // In production, use random per-key salt
        
        let mut key_bytes = [0u8; 32]; // 256 bits for AES-256
        pbkdf2_hmac::<Sha256Hash>(
            &*master_key,
            SALT,
            PBKDF2_ITERATIONS,
            &mut key_bytes,
        );
        
        // Create AES-256-GCM cipher
        let cipher = Aes256Gcm::new_from_slice(&key_bytes)
            .map_err(|e| SecurityError::EncryptionFailed(format!("Failed to create cipher: {}", e)))?;
        
        // Generate random nonce (96 bits for GCM)
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        
        // Encrypt data
        let ciphertext = cipher.encrypt(&nonce, data)
            .map_err(|e| SecurityError::EncryptionFailed(format!("Encryption failed: {}", e)))?;
        
        // Prepend nonce to ciphertext (nonce is 12 bytes for GCM)
        let mut result = Vec::with_capacity(nonce.len() + ciphertext.len());
        result.extend_from_slice(&nonce);
        result.extend_from_slice(&ciphertext);
        
        Ok(result)
    }

    /// Decrypt data at rest using AES-256-GCM
    pub async fn decrypt(&self, encrypted: &[u8]) -> Result<Vec<u8>, SecurityError> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce,
        };
        use sha2::{Sha256, Digest};
        
        let master_key = self.master_key.read().await;
        if master_key.is_empty() {
            return Err(SecurityError::Unauthorized);
        }
        
        // Nonce is 12 bytes for GCM
        if encrypted.len() < 12 {
            return Err(SecurityError::EncryptionFailed("Encrypted data too short".to_string()));
        }
        
        // Extract nonce and ciphertext
        let nonce = Nonce::from_slice(&encrypted[0..12]);
        let ciphertext = &encrypted[12..];
        
        // SECURITY: Derive decryption key using PBKDF2 for key stretching
        use pbkdf2::pbkdf2_hmac;
        use sha2::Sha256 as Sha256Hash;
        
        const PBKDF2_ITERATIONS: u32 = 100_000;
        const SALT: &[u8] = b"narayana-encryption-salt";
        
        let mut key_bytes = [0u8; 32];
        pbkdf2_hmac::<Sha256Hash>(
            &*master_key,
            SALT,
            PBKDF2_ITERATIONS,
            &mut key_bytes,
        );
        
        // Create AES-256-GCM cipher
        let cipher = Aes256Gcm::new_from_slice(&key_bytes)
            .map_err(|e| SecurityError::EncryptionFailed(format!("Failed to create cipher: {}", e)))?;
        
        // Decrypt data
        let plaintext = cipher.decrypt(nonce, ciphertext)
            .map_err(|e| SecurityError::EncryptionFailed(format!("Decryption failed: {}", e)))?;
        
        Ok(plaintext)
    }

    /// Set master key (from secure source)
    pub async fn set_master_key(&self, key: Vec<u8>) {
        let mut master_key = self.master_key.write().await;
        *master_key = key;
    }
}

/// Audit logger for security events (no sensitive data)
pub struct AuditLogger;

impl AuditLogger {
    pub fn log_auth_success(user_id: &str) {
        tracing::info!("AUTH_SUCCESS: user={}", user_id);
    }

    pub fn log_auth_failure(user_id: Option<&str>, reason: &str) {
        if let Some(uid) = user_id {
            tracing::warn!("AUTH_FAILURE: user={}, reason={}", uid, reason);
        } else {
            tracing::warn!("AUTH_FAILURE: user=unknown, reason={}", reason);
        }
    }

    pub fn log_api_access(api_key: &str, endpoint: &str) {
        // Log only key prefix, never full key
        let prefix = &api_key[..api_key.len().min(10)];
        tracing::info!("API_ACCESS: key={}..., endpoint={}", prefix, endpoint);
    }

    pub fn log_security_event(event: &str, details: &str) {
        tracing::warn!("SECURITY_EVENT: event={}, details={}", event, details);
    }
}

/// CORS security configuration
pub struct CorsConfig {
    allowed_origins: Vec<String>,
    allowed_methods: Vec<String>,
    allowed_headers: Vec<String>,
    allow_credentials: bool,
}

impl CorsConfig {
    pub fn new() -> Self {
        Self {
            allowed_origins: Vec::new(),
            allowed_methods: vec!["GET".to_string(), "POST".to_string()],
            allowed_headers: vec!["Content-Type".to_string(), "Authorization".to_string()],
            allow_credentials: true,
        }
    }

    pub fn with_origins(mut self, origins: Vec<String>) -> Self {
        self.allowed_origins = origins;
        self
    }

    pub fn build_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        
        if !self.allowed_origins.is_empty() {
            // SECURITY: Safe unwrap - we checked length above
            if let Ok(origin) = self.allowed_origins[0].parse() {
                headers.insert("access-control-allow-origin", origin);
            }
        }
        
        // SECURITY: Safe unwrap - join creates valid header value
        if let Ok(methods) = self.allowed_methods.join(", ").parse() {
            headers.insert("access-control-allow-methods", methods);
        }
        
        if let Ok(header_names) = self.allowed_headers.join(", ").parse() {
            headers.insert("access-control-allow-headers", header_names);
        }
        
        headers
    }
}

