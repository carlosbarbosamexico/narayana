// TLS/SSL support for encrypted connections

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{error, info};
use rustls::{ServerConfig, Certificate, PrivateKey};
use rustls_pemfile::{certs, pkcs8_private_keys, rsa_private_keys};

/// TLS configuration manager
pub struct TlsConfig {
    cert_path: PathBuf,
    key_path: PathBuf,
    config: Option<Arc<ServerConfig>>,
}

impl TlsConfig {
    /// Create TLS config from certificate files
    pub async fn from_files(cert_path: impl AsRef<Path>, key_path: impl AsRef<Path>) -> Result<Self, TlsError> {
        use tokio::fs;
        
        let cert_path = cert_path.as_ref().to_path_buf();
        let key_path = key_path.as_ref().to_path_buf();

        // Validate files exist
        if !cert_path.exists() {
            return Err(TlsError::CertFileNotFound);
        }
        if !key_path.exists() {
            return Err(TlsError::KeyFileNotFound);
        }

        // Read certificate and key files
        let cert_data = fs::read(&cert_path)
            .await
            .map_err(|e| TlsError::LoadFailed(format!("Failed to read certificate: {}", e)))?;
        
        let key_data = fs::read(&key_path)
            .await
            .map_err(|e| TlsError::LoadFailed(format!("Failed to read key: {}", e)))?;

        // Parse certificates - rustls_pemfile returns Result<Vec<Vec<u8>>>
        let mut cert_reader = cert_data.as_slice();
        let cert_bytes = certs(&mut cert_reader)
            .map_err(|e| TlsError::LoadFailed(format!("Failed to parse certificate: {}", e)))?;
        
        let cert_chain: Vec<Certificate> = cert_bytes
            .into_iter()
            .map(|cert| Certificate(cert.into()))
            .collect();
        
        if cert_chain.is_empty() {
            return Err(TlsError::LoadFailed("No certificates found in certificate file".to_string()));
        }

        // Parse private key
        let mut key_reader = key_data.as_slice();
        let key_bytes = pkcs8_private_keys(&mut key_reader)
            .map_err(|e| TlsError::LoadFailed(format!("Failed to parse PKCS8 key: {}", e)))?;
        
        let mut keys: Vec<PrivateKey> = key_bytes
            .into_iter()
            .map(|key| PrivateKey(key))
            .collect();
        
        // Try RSA keys if PKCS8 didn't work
        if keys.is_empty() {
            let mut key_reader = key_data.as_slice();
            let rsa_key_bytes = rsa_private_keys(&mut key_reader)
                .map_err(|e| TlsError::LoadFailed(format!("Failed to parse RSA key: {}", e)))?;
            keys = rsa_key_bytes
                .into_iter()
                .map(|key| PrivateKey(key))
                .collect();
        }
        
        if keys.is_empty() {
            return Err(TlsError::LoadFailed("No private keys found in key file".to_string()));
        }

        let key = keys[0].clone();

        // Build server config
        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert_chain, key)
            .map_err(|e| TlsError::LoadFailed(format!("Failed to build TLS config: {}", e)))?;

        info!("TLS configuration loaded successfully");

        Ok(Self {
            cert_path,
            key_path,
            config: Some(Arc::new(config)),
        })
    }

    /// Create TLS config from bytes (for embedded certificates)
    pub async fn from_bytes(cert: &[u8], key: &[u8]) -> Result<Self, TlsError> {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create temporary files (for path storage, actual parsing uses bytes directly)
        let mut cert_file = NamedTempFile::new()
            .map_err(|e| TlsError::TempFile(e.to_string()))?;
        cert_file.write_all(cert)
            .map_err(|e| TlsError::WriteFailed(e.to_string()))?;
        let cert_path = cert_file.path().to_path_buf();

        let mut key_file = NamedTempFile::new()
            .map_err(|e| TlsError::TempFile(e.to_string()))?;
        key_file.write_all(key)
            .map_err(|e| TlsError::WriteFailed(e.to_string()))?;
        let key_path = key_file.path().to_path_buf();

        // Parse certificates
        let mut cert_reader = cert;
        let cert_bytes = certs(&mut cert_reader)
            .map_err(|e| TlsError::LoadFailed(format!("Failed to parse certificate: {}", e)))?;
        
        let cert_chain: Vec<Certificate> = cert_bytes
            .into_iter()
            .map(|cert| Certificate(cert.into()))
            .collect();
        
        if cert_chain.is_empty() {
            return Err(TlsError::LoadFailed("No certificates found".to_string()));
        }

        // Parse private key
        let mut key_reader = key;
        let key_bytes = pkcs8_private_keys(&mut key_reader)
            .map_err(|e| TlsError::LoadFailed(format!("Failed to parse PKCS8 key: {}", e)))?;
        
        let mut keys: Vec<PrivateKey> = key_bytes
            .into_iter()
            .map(|key| PrivateKey(key))
            .collect();
        
        // Try RSA keys if PKCS8 didn't work
        if keys.is_empty() {
            let mut key_reader = key;
            let rsa_key_bytes = rsa_private_keys(&mut key_reader)
                .map_err(|e| TlsError::LoadFailed(format!("Failed to parse RSA key: {}", e)))?;
            keys = rsa_key_bytes
                .into_iter()
                .map(|key| PrivateKey(key))
                .collect();
        }
        
        if keys.is_empty() {
            return Err(TlsError::LoadFailed("No private keys found".to_string()));
        }

        let key = keys[0].clone();

        // Build server config
        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert_chain, key)
            .map_err(|e| TlsError::LoadFailed(format!("Failed to build TLS config: {}", e)))?;

        Ok(Self {
            cert_path,
            key_path,
            config: Some(Arc::new(config)),
        })
    }

    /// Get Rustls config
    pub fn config(&self) -> Option<Arc<ServerConfig>> {
        self.config.clone()
    }

    /// Reload TLS config (for certificate rotation)
    pub async fn reload(&mut self) -> Result<(), TlsError> {
        use tokio::fs;
        
        // Read certificate and key files
        let cert_data = fs::read(&self.cert_path)
            .await
            .map_err(|e| TlsError::LoadFailed(format!("Failed to read certificate: {}", e)))?;
        
        let key_data = fs::read(&self.key_path)
            .await
            .map_err(|e| TlsError::LoadFailed(format!("Failed to read key: {}", e)))?;

        // Parse certificates
        let mut cert_reader = cert_data.as_slice();
        let cert_bytes = certs(&mut cert_reader)
            .map_err(|e| TlsError::LoadFailed(format!("Failed to parse certificate: {}", e)))?;
        
        let cert_chain: Vec<Certificate> = cert_bytes
            .into_iter()
            .map(|cert| Certificate(cert.into()))
            .collect();
        
        if cert_chain.is_empty() {
            return Err(TlsError::LoadFailed("No certificates found in certificate file".to_string()));
        }

        // Parse private key
        let mut key_reader = key_data.as_slice();
        let key_bytes = pkcs8_private_keys(&mut key_reader)
            .map_err(|e| TlsError::LoadFailed(format!("Failed to parse PKCS8 key: {}", e)))?;
        
        let mut keys: Vec<PrivateKey> = key_bytes
            .into_iter()
            .map(|key| PrivateKey(key))
            .collect();
        
        // Try RSA keys if PKCS8 didn't work
        if keys.is_empty() {
            let mut key_reader = key_data.as_slice();
            let rsa_key_bytes = rsa_private_keys(&mut key_reader)
                .map_err(|e| TlsError::LoadFailed(format!("Failed to parse RSA key: {}", e)))?;
            keys = rsa_key_bytes
                .into_iter()
                .map(|key| PrivateKey(key))
                .collect();
        }
        
        if keys.is_empty() {
            return Err(TlsError::LoadFailed("No private keys found in key file".to_string()));
        }

        let key = keys[0].clone();

        // Build server config
        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert_chain, key)
            .map_err(|e| TlsError::LoadFailed(format!("Failed to build TLS config: {}", e)))?;

        self.config = Some(Arc::new(config));
        info!("TLS configuration reloaded successfully");
        Ok(())
    }
}

/// TLS errors
#[derive(Debug)]
pub enum TlsError {
    CertFileNotFound,
    KeyFileNotFound,
    LoadFailed(String),
    TempFile(String),
    WriteFailed(String),
}

impl std::fmt::Display for TlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TlsError::CertFileNotFound => write!(f, "Certificate file not found"),
            TlsError::KeyFileNotFound => write!(f, "Key file not found"),
            TlsError::LoadFailed(e) => write!(f, "Failed to load TLS config: {}", e),
            TlsError::TempFile(e) => write!(f, "Failed to create temp file: {}", e),
            TlsError::WriteFailed(e) => write!(f, "Failed to write file: {}", e),
        }
    }
}

impl std::error::Error for TlsError {}

/// Self-signed certificate generator for development
pub struct SelfSignedCert;

impl SelfSignedCert {
    /// Generate a self-signed certificate (development only)
    pub fn generate(domain: &str) -> Result<(Vec<u8>, Vec<u8>), TlsError> {
        // In production, use proper certificate generation
        // For now, return placeholder
        let cert = format!("-----BEGIN CERTIFICATE-----\nDEVELOPMENT CERT FOR {}\n-----END CERTIFICATE-----", domain);
        let key = "-----BEGIN PRIVATE KEY-----\nDEVELOPMENT KEY\n-----END PRIVATE KEY-----";
        
        Ok((cert.as_bytes().to_vec(), key.as_bytes().to_vec()))
    }
}

/// TLS version configuration
#[derive(Debug, Clone, Copy)]
pub enum TlsVersion {
    Tls12,
    Tls13,
    All,
}

impl TlsVersion {
    pub fn to_rustls_config(&self) -> &'static str {
        match self {
            TlsVersion::Tls12 => "TLSv1.2",
            TlsVersion::Tls13 => "TLSv1.3",
            TlsVersion::All => "TLSv1.2+TLSv1.3",
        }
    }
}

/// Certificate pinning for enhanced security
pub struct CertificatePinner {
    pinned_certs: Vec<Vec<u8>>,
}

impl CertificatePinner {
    pub fn new() -> Self {
        Self {
            pinned_certs: Vec::new(),
        }
    }

    pub fn add_cert(&mut self, cert: Vec<u8>) {
        self.pinned_certs.push(cert);
    }

    pub fn verify(&self, cert: &[u8]) -> bool {
        self.pinned_certs.iter().any(|pinned| pinned == cert)
    }
}
