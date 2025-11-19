// OAuth2 support for secure authentication

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// OAuth2 provider configuration
#[derive(Debug, Clone)]
pub struct OAuth2Provider {
    pub client_id: String,
    pub client_secret: String, // Stored securely, never logged
    pub auth_url: String,
    pub token_url: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

impl OAuth2Provider {
    pub fn new(
        client_id: String,
        client_secret: String,
        auth_url: String,
        token_url: String,
        redirect_uri: String,
        scopes: Vec<String>,
    ) -> Self {
        Self {
            client_id,
            client_secret,
            auth_url,
            token_url,
            redirect_uri,
            scopes,
        }
    }

    /// Generate authorization URL
    pub fn auth_url(&self, state: &str) -> String {
        let scopes = self.scopes.join(" ");
        format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
            self.auth_url,
            self.client_id,
            urlencoding::encode(&self.redirect_uri),
            urlencoding::encode(&scopes),
            state
        )
    }

    /// Exchange authorization code for token
    pub async fn exchange_code(&self, code: &str) -> Result<OAuth2Token, OAuth2Error> {
        let client = reqwest::Client::new();
        
        let mut params = HashMap::new();
        params.insert("grant_type", "authorization_code");
        params.insert("code", code);
        params.insert("redirect_uri", &self.redirect_uri);
        params.insert("client_id", &self.client_id);
        params.insert("client_secret", &self.client_secret);

        let response = client
            .post(&self.token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| OAuth2Error::RequestFailed(e.to_string()))?;

        let token: OAuth2Token = response
            .json()
            .await
            .map_err(|e| OAuth2Error::ParseFailed(e.to_string()))?;

        Ok(token)
    }
}

/// OAuth2 token response
#[derive(Debug, Deserialize, Serialize)]
pub struct OAuth2Token {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<u64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

/// OAuth2 manager
pub struct OAuth2Manager {
    providers: Arc<RwLock<HashMap<String, OAuth2Provider>>>,
}

impl OAuth2Manager {
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_provider(&self, name: String, provider: OAuth2Provider) {
        let mut providers = self.providers.write().await;
        providers.insert(name, provider);
    }

    pub async fn get_provider(&self, name: &str) -> Option<OAuth2Provider> {
        let providers = self.providers.read().await;
        providers.get(name).cloned()
    }

    pub async fn generate_auth_url(&self, provider_name: &str, state: &str) -> Result<String, OAuth2Error> {
        let provider = self.get_provider(provider_name)
            .await
            .ok_or(OAuth2Error::ProviderNotFound)?;
        
        Ok(provider.auth_url(state))
    }
}

/// OAuth2 errors
#[derive(Debug)]
pub enum OAuth2Error {
    ProviderNotFound,
    RequestFailed(String),
    ParseFailed(String),
    InvalidCode,
    TokenExpired,
}

impl std::fmt::Display for OAuth2Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OAuth2Error::ProviderNotFound => write!(f, "OAuth2 provider not found"),
            OAuth2Error::RequestFailed(e) => write!(f, "OAuth2 request failed: {}", e),
            OAuth2Error::ParseFailed(e) => write!(f, "Failed to parse OAuth2 response: {}", e),
            OAuth2Error::InvalidCode => write!(f, "Invalid authorization code"),
            OAuth2Error::TokenExpired => write!(f, "OAuth2 token expired"),
        }
    }
}

impl std::error::Error for OAuth2Error {}

/// Secure session manager
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    session_timeout: std::time::Duration,
}

#[derive(Clone, Debug)]
pub struct Session {
    pub user_id: String,
    pub created_at: std::time::Instant,
    pub expires_at: std::time::Instant,
    pub ip_address: Option<String>,
}

impl SessionManager {
    pub fn new(session_timeout: std::time::Duration) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            session_timeout,
        }
    }

    pub async fn create_session(&self, user_id: String, ip_address: Option<String>) -> String {
        use uuid::Uuid;
        
        let session_id = Uuid::new_v4().to_string();
        let now = std::time::Instant::now();
        
        let session = Session {
            user_id,
            created_at: now,
            expires_at: now + self.session_timeout,
            ip_address,
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session);
        
        session_id
    }

    pub async fn get_session(&self, session_id: &str) -> Option<Session> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get(session_id) {
            if session.expires_at > std::time::Instant::now() {
                return Some(session.clone());
            } else {
                sessions.remove(session_id);
            }
        }
        
        None
    }

    pub async fn invalidate_session(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id);
    }

    pub async fn cleanup_expired(&self) {
        let mut sessions = self.sessions.write().await;
        let now = std::time::Instant::now();
        sessions.retain(|_, session| session.expires_at > now);
    }
}

