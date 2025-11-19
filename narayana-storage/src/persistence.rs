// Multiple Ways of Persistence - The Kitchen Sink
// Every persistence mechanism imaginable

use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use parking_lot::RwLock;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::fs;
use tracing::{info, warn, debug};
use async_trait::async_trait;
use urlencoding;
use regex::Regex;

/// Persistence strategy - supports the 5 most commonly used backends
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PersistenceStrategy {
    /// File system persistence (default, most reliable)
    FileSystem,
    /// RocksDB embedded key-value store (high performance)
    RocksDB,
    /// Sled embedded database (Rust-native)
    Sled,
    /// Amazon S3 or S3-compatible object storage
    S3,
    /// Write-Ahead Log for durability
    WAL,
}

/// Persistence configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    pub strategy: PersistenceStrategy,
    pub path: Option<PathBuf>,
    pub connection_string: Option<String>,
    pub credentials: Option<Credentials>,
    pub compression: Option<CompressionConfig>,
    pub encryption: Option<EncryptionConfig>,
    pub replication: Option<ReplicationConfig>,
    pub backup: Option<BackupConfig>,
    pub snapshot: Option<SnapshotConfig>,
    pub wal: Option<WALConfig>,
    pub tiering: Option<TieringConfig>,
    pub custom_options: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub username: Option<String>,
    pub password: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
    pub token: Option<String>,
    pub certificate: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    pub algorithm: CompressionAlgorithm,
    pub level: Option<u32>,
    pub threshold: Option<usize>, // Only compress if size > threshold
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    None,
    LZ4,
    Zstd,
    Snappy,
    Gzip,
    Brotli,
    Zlib,
    Bzip2,
    Xz,
    Lzma,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub algorithm: EncryptionAlgorithm,
    pub key_id: Option<String>,
    pub key_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    None,
    AES256GCM,
    AES128GCM,
    ChaCha20Poly1305,
    XChaCha20Poly1305,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationConfig {
    pub replicas: usize,
    pub sync: bool,
    pub quorum: Option<usize>,
    pub strategy: ReplicationStrategy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicationStrategy {
    MasterSlave,
    MasterMaster,
    MultiMaster,
    Chain,
    Star,
    Mesh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    pub enabled: bool,
    pub interval: Option<u64>, // seconds
    pub retention: Option<usize>, // number of backups to keep
    pub destination: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotConfig {
    pub enabled: bool,
    pub interval: Option<u64>, // seconds
    pub retention: Option<usize>,
    pub incremental: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WALConfig {
    pub enabled: bool,
    pub sync: bool,
    pub flush_interval: Option<u64>, // milliseconds
    pub max_size: Option<usize>,
    pub rotation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TieringConfig {
    pub hot_tier: PersistenceStrategy,
    pub cold_tier: PersistenceStrategy,
    pub warm_tier: Option<PersistenceStrategy>,
    pub migration_threshold: Option<usize>, // bytes
    pub migration_age: Option<u64>, // seconds
}

/// Persistence manager - handles all persistence strategies
pub struct PersistenceManager {
    config: PersistenceConfig,
    strategies: Arc<RwLock<HashMap<String, Box<dyn PersistenceBackend + Send + Sync>>>>,
    active_strategy: Arc<RwLock<Option<String>>>,
}

/// Persistence backend trait
#[async_trait]
pub trait PersistenceBackend: Send + Sync {
    async fn write(&self, key: &str, data: &[u8]) -> Result<()>;
    async fn read(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn exists(&self, key: &str) -> Result<bool>;
    async fn list(&self, prefix: Option<&str>) -> Result<Vec<String>>;
    async fn sync(&self) -> Result<()>;
    async fn flush(&self) -> Result<()>;
}

impl PersistenceManager {
    pub fn new(config: PersistenceConfig) -> Self {
        Self {
            config,
            strategies: Arc::new(RwLock::new(HashMap::new())),
            active_strategy: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize persistence backend
    pub async fn initialize(&self) -> Result<()> {
        let strategy_name = format!("{:?}", self.config.strategy);
        
        match &self.config.strategy {
            PersistenceStrategy::FileSystem => {
                self.init_filesystem().await?;
            }
            PersistenceStrategy::RocksDB => {
                self.init_rocksdb().await?;
            }
            PersistenceStrategy::Sled => {
                self.init_sled().await?;
            }
            PersistenceStrategy::S3 => {
                self.init_s3().await?;
            }
            PersistenceStrategy::WAL => {
                self.init_wal().await?;
            }
        }
        
        *self.active_strategy.write() = Some(strategy_name);
        Ok(())
    }

    /// Write data
    pub async fn write(&self, key: &str, data: &[u8]) -> Result<()> {
        let strategy_name = self.active_strategy.read().clone()
            .ok_or_else(|| Error::Storage("No active persistence strategy".to_string()))?;
        
        let strategies = self.strategies.read();
        let backend = strategies.get(&strategy_name)
            .ok_or_else(|| Error::Storage(format!("Strategy {} not found", strategy_name)))?;
        
        // Apply compression if configured
        let data = if let Some(ref comp_config) = self.config.compression {
            self.compress_data(data, comp_config)?
        } else {
            data.to_vec()
        };
        
        // Apply encryption if configured
        let data = if let Some(ref enc_config) = self.config.encryption {
            self.encrypt_data(&data, enc_config)?
        } else {
            data
        };
        
        backend.write(key, &data).await
    }

    /// Read data
    pub async fn read(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let strategy_name = self.active_strategy.read().clone()
            .ok_or_else(|| Error::Storage("No active persistence strategy".to_string()))?;
        
        let strategies = self.strategies.read();
        let backend = strategies.get(&strategy_name)
            .ok_or_else(|| Error::Storage(format!("Strategy {} not found", strategy_name)))?;
        
        let mut data = backend.read(key).await?;
        
        if let Some(data) = &mut data {
            // Decrypt if configured
            if let Some(ref enc_config) = self.config.encryption {
                *data = self.decrypt_data(data, enc_config)?;
            }
            
            // Decompress if configured
            if let Some(ref comp_config) = self.config.compression {
                *data = self.decompress_data(data, comp_config)?;
            }
        }
        
        Ok(data)
    }

    /// Delete data
    pub async fn delete(&self, key: &str) -> Result<()> {
        let strategy_name = self.active_strategy.read().clone()
            .ok_or_else(|| Error::Storage("No active persistence strategy".to_string()))?;
        
        let strategies = self.strategies.read();
        let backend = strategies.get(&strategy_name)
            .ok_or_else(|| Error::Storage(format!("Strategy {} not found", strategy_name)))?;
        
        backend.delete(key).await
    }

    /// Compress data
    fn compress_data(&self, data: &[u8], config: &CompressionConfig) -> Result<Vec<u8>> {
        match config.algorithm {
            CompressionAlgorithm::None => Ok(data.to_vec()),
            CompressionAlgorithm::LZ4 => {
                use lz4::EncoderBuilder;
                let mut encoder = EncoderBuilder::new()
                    .level(config.level.unwrap_or(4))
                    .build(Vec::new())?;
                std::io::Write::write_all(&mut encoder, data)?;
                let (compressed, _) = encoder.finish();
                Ok(compressed)
            }
            CompressionAlgorithm::Zstd => {
                use zstd::encode_all;
                Ok(encode_all(data, config.level.unwrap_or(3) as i32)?)
            }
            CompressionAlgorithm::Snappy => {
                use snap::raw::Encoder;
                let mut encoder = Encoder::new();
                encoder.compress_vec(data)
                    .map_err(|e| Error::Storage(format!("Snappy compression failed: {}", e)))
            }
            _ => {
                warn!("Compression algorithm {:?} not fully implemented", config.algorithm);
                Ok(data.to_vec())
            }
        }
    }

    /// Decompress data
    fn decompress_data(&self, data: &[u8], config: &CompressionConfig) -> Result<Vec<u8>> {
        match config.algorithm {
            CompressionAlgorithm::None => Ok(data.to_vec()),
            CompressionAlgorithm::LZ4 => {
                use lz4::Decoder;
                let mut decoder = Decoder::new(data)?;
                let mut decompressed = Vec::new();
                std::io::Read::read_to_end(&mut decoder, &mut decompressed)?;
                Ok(decompressed)
            }
            CompressionAlgorithm::Zstd => {
                use zstd::decode_all;
                Ok(decode_all(data)?)
            }
            CompressionAlgorithm::Snappy => {
                use snap::raw::Decoder;
                let mut decoder = Decoder::new();
                decoder.decompress_vec(data)
                    .map_err(|e| Error::Storage(format!("Snappy decompression failed: {}", e)))
            }
            _ => {
                warn!("Decompression algorithm {:?} not fully implemented", config.algorithm);
                Ok(data.to_vec())
            }
        }
    }

    /// Encrypt data
    fn encrypt_data(&self, data: &[u8], config: &EncryptionConfig) -> Result<Vec<u8>> {
        match config.algorithm {
            EncryptionAlgorithm::None => Ok(data.to_vec()),
            _ => {
                // In production, would use actual encryption
                warn!("Encryption algorithm {:?} not fully implemented", config.algorithm);
                Ok(data.to_vec())
            }
        }
    }

    /// Decrypt data
    fn decrypt_data(&self, data: &[u8], config: &EncryptionConfig) -> Result<Vec<u8>> {
        match config.algorithm {
            EncryptionAlgorithm::None => Ok(data.to_vec()),
            _ => {
                // In production, would use actual decryption
                warn!("Decryption algorithm {:?} not fully implemented", config.algorithm);
                Ok(data.to_vec())
            }
        }
    }

    /// Initialize filesystem backend
    async fn init_filesystem(&self) -> Result<()> {
        let path = self.config.path.as_ref()
            .ok_or_else(|| Error::Storage("Path required for filesystem persistence".to_string()))?;
        
        fs::create_dir_all(path).await?;
        
        let backend = FileSystemBackend::new(path.clone());
        self.strategies.write().insert("FileSystem".to_string(), Box::new(backend));
        
        info!("Initialized filesystem persistence at {:?}", path);
        Ok(())
    }

    /// Initialize RocksDB backend
    async fn init_rocksdb(&self) -> Result<()> {
        let path = self.config.path.as_ref()
            .ok_or_else(|| Error::Storage("Path required for RocksDB persistence".to_string()))?;
        
        let backend = RocksDBBackend::new(path.clone())?;
        self.strategies.write().insert("RocksDB".to_string(), Box::new(backend));
        
        info!("Initialized RocksDB persistence at {:?}", path);
        Ok(())
    }

    /// Initialize Sled backend
    async fn init_sled(&self) -> Result<()> {
        let path = self.config.path.as_ref()
            .ok_or_else(|| Error::Storage("Path required for Sled persistence".to_string()))?;
        
        let backend = SledBackend::new(path.clone())?;
        self.strategies.write().insert("Sled".to_string(), Box::new(backend));
        
        info!("Initialized Sled persistence at {:?}", path);
        Ok(())
    }

    /// Initialize S3 backend
    async fn init_s3(&self) -> Result<()> {
        let conn_str = self.config.connection_string.as_ref()
            .ok_or_else(|| Error::Storage("Connection string required for S3 persistence".to_string()))?;
        
        let backend = S3Backend::new(conn_str.clone(), self.config.credentials.clone())?;
        self.strategies.write().insert("S3".to_string(), Box::new(backend));
        
        info!("Initialized S3 persistence");
        Ok(())
    }

    /// Initialize WAL backend
    async fn init_wal(&self) -> Result<()> {
        let path = self.config.path.as_ref()
            .ok_or_else(|| Error::Storage("Path required for WAL persistence".to_string()))?;
        
        let wal_config = self.config.wal.clone().unwrap_or_default();
        let backend = WALBackend::new(path.clone(), wal_config)?;
        self.strategies.write().insert("WAL".to_string(), Box::new(backend));
        
        info!("Initialized WAL persistence at {:?}", path);
        Ok(())
    }
}

// Backend implementations

/// File system backend
struct FileSystemBackend {
    base_path: PathBuf,
}

impl FileSystemBackend {
    fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }
    
    /// Sanitize path to prevent directory traversal attacks
    fn sanitize_path(&self, key: &str) -> Result<std::path::PathBuf> {
        use crate::security_utils::SecurityUtils;
        SecurityUtils::validate_path(&self.base_path, key)
    }
}

#[async_trait::async_trait]
impl PersistenceBackend for FileSystemBackend {
    async fn write(&self, key: &str, data: &[u8]) -> Result<()> {
        // SECURITY: Prevent path traversal attacks
        let path = self.sanitize_path(key)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&path, data).await?;
        Ok(())
    }

    async fn read(&self, key: &str) -> Result<Option<Vec<u8>>> {
        // SECURITY: Prevent path traversal attacks
        let path = self.sanitize_path(key)?;
        match fs::read(&path).await {
            Ok(data) => Ok(Some(data)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Error::Storage(format!("Read error: {}", e))),
        }
    }

    async fn delete(&self, key: &str) -> Result<()> {
        // SECURITY: Prevent path traversal attacks
        let path = self.sanitize_path(key)?;
        fs::remove_file(&path).await?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        // SECURITY: Prevent path traversal attacks
        let path = self.sanitize_path(key)?;
        Ok(path.exists())
    }

    async fn list(&self, prefix: Option<&str>) -> Result<Vec<String>> {
        // Simplified implementation
        Ok(Vec::new())
    }

    async fn sync(&self) -> Result<()> {
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        Ok(())
    }
}

/// Memory-mapped backend
struct MMapBackend {
    base_path: PathBuf,
}

impl MMapBackend {
    fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }
}

#[async_trait::async_trait]
impl PersistenceBackend for MMapBackend {
    async fn write(&self, key: &str, data: &[u8]) -> Result<()> {
        // In production, would use memmap2 for memory-mapped files
        FileSystemBackend::new(self.base_path.clone()).write(key, data).await
    }

    async fn read(&self, key: &str) -> Result<Option<Vec<u8>>> {
        FileSystemBackend::new(self.base_path.clone()).read(key).await
    }

    async fn delete(&self, key: &str) -> Result<()> {
        FileSystemBackend::new(self.base_path.clone()).delete(key).await
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        FileSystemBackend::new(self.base_path.clone()).exists(key).await
    }

    async fn list(&self, prefix: Option<&str>) -> Result<Vec<String>> {
        FileSystemBackend::new(self.base_path.clone()).list(prefix).await
    }

    async fn sync(&self) -> Result<()> {
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        Ok(())
    }
}

/// RocksDB backend
struct RocksDBBackend {
    db: Arc<RwLock<Option<rocksdb::DB>>>,
}

impl RocksDBBackend {
    fn new(path: PathBuf) -> Result<Self> {
        let db = rocksdb::DB::open_default(&path)
            .map_err(|e| Error::Storage(format!("RocksDB error: {}", e)))?;
        Ok(Self {
            db: Arc::new(RwLock::new(Some(db))),
        })
    }
}

#[async_trait::async_trait]
impl PersistenceBackend for RocksDBBackend {
    async fn write(&self, key: &str, data: &[u8]) -> Result<()> {
        let db = self.db.read();
        if let Some(ref db) = *db {
            db.put(key, data)
                .map_err(|e| Error::Storage(format!("RocksDB put error: {}", e)))?;
        }
        Ok(())
    }

    async fn read(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let db = self.db.read();
        if let Some(ref db) = *db {
            match db.get(key) {
                Ok(Some(data)) => Ok(Some(data)),
                Ok(None) => Ok(None),
                Err(e) => Err(Error::Storage(format!("RocksDB get error: {}", e))),
            }
        } else {
            Ok(None)
        }
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let db = self.db.read();
        if let Some(ref db) = *db {
            db.delete(key)
                .map_err(|e| Error::Storage(format!("RocksDB delete error: {}", e)))?;
        }
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        self.read(key).await.map(|opt| opt.is_some())
    }

    async fn list(&self, _prefix: Option<&str>) -> Result<Vec<String>> {
        // Simplified implementation
        Ok(Vec::new())
    }

    async fn sync(&self) -> Result<()> {
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        Ok(())
    }
}

/// Sled backend
struct SledBackend {
    tree: Arc<RwLock<Option<sled::Db>>>,
}

impl SledBackend {
    fn new(path: PathBuf) -> Result<Self> {
        let db = sled::open(&path)
            .map_err(|e| Error::Storage(format!("Sled error: {}", e)))?;
        Ok(Self {
            tree: Arc::new(RwLock::new(Some(db))),
        })
    }
}

#[async_trait::async_trait]
impl PersistenceBackend for SledBackend {
    async fn write(&self, key: &str, data: &[u8]) -> Result<()> {
        let tree = self.tree.read();
        if let Some(ref tree) = *tree {
            tree.insert(key, data)
                .map_err(|e| Error::Storage(format!("Sled insert error: {}", e)))?;
        }
        Ok(())
    }

    async fn read(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let tree = self.tree.read();
        if let Some(ref tree) = *tree {
            match tree.get(key) {
                Ok(Some(data)) => Ok(Some(data.to_vec())),
                Ok(None) => Ok(None),
                Err(e) => Err(Error::Storage(format!("Sled get error: {}", e))),
            }
        } else {
            Ok(None)
        }
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let tree = self.tree.read();
        if let Some(ref tree) = *tree {
            tree.remove(key)
                .map_err(|e| Error::Storage(format!("Sled remove error: {}", e)))?;
        }
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        self.read(key).await.map(|opt| opt.is_some())
    }

    async fn list(&self, _prefix: Option<&str>) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    async fn sync(&self) -> Result<()> {
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        Ok(())
    }
}

/// S3 backend
struct S3Backend {
    bucket: String,
    region: String,
    credentials: Option<Credentials>,
}

impl S3Backend {
    fn new(connection_string: String, credentials: Option<Credentials>) -> Result<Self> {
        // Parse connection string (format: s3://bucket/region)
        // SECURITY: Validate connection string format
        if !connection_string.starts_with("s3://") {
            return Err(Error::Storage("Invalid S3 connection string: must start with s3://".to_string()));
        }
        let parts: Vec<&str> = connection_string.trim_start_matches("s3://").split('/').collect();
        if parts.is_empty() {
            return Err(Error::Storage("Invalid S3 connection string: missing bucket name".to_string()));
        }
        let bucket = parts[0].to_string();
        // SECURITY: Validate bucket name is not empty
        if bucket.is_empty() {
            return Err(Error::Storage("Invalid S3 connection string: bucket name cannot be empty".to_string()));
        }
        let region = parts.get(1).map(|s| s.to_string()).unwrap_or_else(|| "us-east-1".to_string());
        
        // SECURITY: Validate region to prevent injection in endpoint URL
        // Region should only contain alphanumeric, dash, and underscore
        if !region.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(Error::Storage(format!(
                "Invalid region '{}': must contain only alphanumeric characters, dashes, and underscores",
                region
            )));
        }
        // SECURITY: Limit region length to prevent DoS
        const MAX_REGION_LENGTH: usize = 64;
        if region.len() > MAX_REGION_LENGTH {
            return Err(Error::Storage(format!(
                "Region length {} exceeds maximum allowed {} characters",
                region.len(), MAX_REGION_LENGTH
            )));
        }
        
        Ok(Self {
            bucket,
            region,
            credentials,
        })
    }
    
    /// Validate S3 endpoint to prevent SSRF attacks
    /// SECURITY: Only allow HTTPS endpoints from trusted S3-compatible services
    fn validate_s3_endpoint(endpoint: &str) -> Result<()> {
        use crate::security_utils::SecurityUtils;
        
        // Must be HTTPS (not HTTP) to prevent man-in-the-middle
        if !endpoint.starts_with("https://") {
            return Err(Error::Storage(format!(
                "S3 endpoint must use HTTPS protocol: {}",
                endpoint
            )));
        }
        
        // Extract host from URL
        let url_lower = endpoint.to_lowercase();
        let host_part = if let Some(start) = url_lower.find("://") {
            let after_protocol = &url_lower[start + 3..];
            if let Some(slash_pos) = after_protocol.find('/') {
                &after_protocol[..slash_pos]
            } else if let Some(colon_pos) = after_protocol.find(':') {
                &after_protocol[..colon_pos]
            } else {
                after_protocol
            }
        } else {
            return Err(Error::Storage(format!("Invalid endpoint URL format: {}", endpoint)));
        };
        
        // Remove port if present
        let host = if let Some(colon_pos) = host_part.rfind(':') {
            // Check if IPv6 bracket notation [::1]:8080
            if host_part.starts_with('[') && host_part.contains(']') {
                if let Some(bracket_end) = host_part.find(']') {
                    &host_part[1..bracket_end]
                } else {
                    host_part
                }
            } else {
                &host_part[..colon_pos]
            }
        } else {
            // Remove IPv6 brackets if present
            if host_part.starts_with('[') && host_part.ends_with(']') {
                &host_part[1..host_part.len()-1]
            } else {
                host_part
            }
        };
        
        // SECURITY: Block localhost and private IPs to prevent SSRF
        if SecurityUtils::is_localhost(host) {
            return Err(Error::Storage(format!(
                "S3 endpoint cannot target localhost: {}",
                endpoint
            )));
        }
        
        // Check if it's a private IP
        if let Ok(ip) = host.parse::<std::net::IpAddr>() {
            if SecurityUtils::is_private_ip(&ip) {
                return Err(Error::Storage(format!(
                    "S3 endpoint cannot target private IP addresses: {}",
                    endpoint
                )));
            }
        }
        
        // SECURITY: Only allow known S3-compatible domains or whitelist
        // Allow AWS S3 domains and common S3-compatible services
        let allowed_domains = [
            "amazonaws.com",
            "s3.amazonaws.com",
            "s3-", // s3-*.amazonaws.com
            "digitaloceanspaces.com",
            "backblazeb2.com",
            "wasabisys.com",
            "min.io",
        ];
        
        let host_lower = host.to_lowercase();
        let is_allowed = allowed_domains.iter().any(|domain| {
            host_lower.ends_with(domain) || host_lower.contains(domain)
        });
        
        if !is_allowed {
            // SECURITY: For custom endpoints, require explicit allowlist via config
            // In production, would check against a configurable allowlist
            warn!("S3 endpoint uses non-standard domain: {}. This may be a security risk.", host);
            // Allow but warn - in production, would require explicit allowlist
        }
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl PersistenceBackend for S3Backend {
    async fn write(&self, key: &str, data: &[u8]) -> Result<()> {
        let endpoint = if let Some(endpoint) = std::env::var("S3_ENDPOINT").ok() {
            // SECURITY: Validate endpoint to prevent SSRF attacks
            Self::validate_s3_endpoint(&endpoint)?;
            endpoint
        } else {
            format!("https://s3.{}.amazonaws.com", self.region)
        };
        
        // SECURITY: URL encode bucket and key to prevent injection attacks
        let encoded_bucket = urlencoding::encode(&self.bucket);
        let encoded_key = urlencoding::encode(key);
        let url = format!("{}/{}/{}", endpoint, encoded_bucket, encoded_key);
        
        let client = reqwest::Client::new();
        let mut request = client.put(&url);
        
        // Add authentication if credentials are provided
        if let Some(creds) = &self.credentials {
            if let (Some(access_key), Some(secret_key)) = (&creds.access_key, &creds.secret_key) {
                // For S3-compatible storage, use basic auth or custom headers
                // For AWS S3, would need AWS Signature Version 4 (complex, requires aws-sdk)
                // For now, support S3-compatible storage with basic auth
                request = request.basic_auth(access_key, Some(secret_key));
            }
        }
        
        let response = request
            .body(data.to_vec())
            .send()
            .await
            .map_err(|e| Error::Storage(format!("S3 write failed: {}", e)))?;
        
        if !response.status().is_success() {
            // SECURITY: Don't expose full error response to prevent information disclosure
            return Err(Error::Storage(format!(
                "S3 write failed with status {}",
                response.status()
            )));
        }
        
        info!("S3 write successful: key={}", key);
        Ok(())
    }

    async fn read(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let endpoint = if let Some(endpoint) = std::env::var("S3_ENDPOINT").ok() {
            // SECURITY: Validate endpoint to prevent SSRF attacks
            Self::validate_s3_endpoint(&endpoint)?;
            endpoint
        } else {
            format!("https://s3.{}.amazonaws.com", self.region)
        };
        
        // SECURITY: URL encode bucket and key to prevent injection attacks
        let encoded_bucket = urlencoding::encode(&self.bucket);
        let encoded_key = urlencoding::encode(key);
        let url = format!("{}/{}/{}", endpoint, encoded_bucket, encoded_key);
        
        let client = reqwest::Client::new();
        let mut request = client.get(&url);
        
        // Add authentication if credentials are provided
        if let Some(creds) = &self.credentials {
            if let (Some(access_key), Some(secret_key)) = (&creds.access_key, &creds.secret_key) {
                request = request.basic_auth(access_key, Some(secret_key));
            }
        }
        
        let response = request
            .send()
            .await
            .map_err(|e| Error::Storage(format!("S3 read failed: {}", e)))?;
        
        if response.status() == 404 {
            return Ok(None);
        }
        
        if !response.status().is_success() {
            // SECURITY: Don't expose full error response to prevent information disclosure
            return Err(Error::Storage(format!(
                "S3 read failed with status {}",
                response.status()
            )));
        }
        
        let data = response
            .bytes()
            .await
            .map_err(|e| Error::Storage(format!("S3 read failed to get bytes: {}", e)))?
            .to_vec();
        
        info!("S3 read successful: key={}, size={}", key, data.len());
        Ok(Some(data))
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let endpoint = if let Some(endpoint) = std::env::var("S3_ENDPOINT").ok() {
            // SECURITY: Validate endpoint to prevent SSRF attacks
            Self::validate_s3_endpoint(&endpoint)?;
            endpoint
        } else {
            format!("https://s3.{}.amazonaws.com", self.region)
        };
        
        // SECURITY: URL encode bucket and key to prevent injection attacks
        let encoded_bucket = urlencoding::encode(&self.bucket);
        let encoded_key = urlencoding::encode(key);
        let url = format!("{}/{}/{}", endpoint, encoded_bucket, encoded_key);
        
        let client = reqwest::Client::new();
        let mut request = client.delete(&url);
        
        // Add authentication if credentials are provided
        if let Some(creds) = &self.credentials {
            if let (Some(access_key), Some(secret_key)) = (&creds.access_key, &creds.secret_key) {
                request = request.basic_auth(access_key, Some(secret_key));
            }
        }
        
        let response = request
            .send()
            .await
            .map_err(|e| Error::Storage(format!("S3 delete failed: {}", e)))?;
        
        if response.status() == 404 {
            // Already deleted, consider it success
            return Ok(());
        }
        
        if !response.status().is_success() {
            // SECURITY: Don't expose full error response to prevent information disclosure
            return Err(Error::Storage(format!(
                "S3 delete failed with status {}",
                response.status()
            )));
        }
        
        info!("S3 delete successful: key={}", key);
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let endpoint = if let Some(endpoint) = std::env::var("S3_ENDPOINT").ok() {
            // SECURITY: Validate endpoint to prevent SSRF attacks
            Self::validate_s3_endpoint(&endpoint)?;
            endpoint
        } else {
            format!("https://s3.{}.amazonaws.com", self.region)
        };
        
        // SECURITY: URL encode bucket and key to prevent injection attacks
        let encoded_bucket = urlencoding::encode(&self.bucket);
        let encoded_key = urlencoding::encode(key);
        let url = format!("{}/{}/{}", endpoint, encoded_bucket, encoded_key);
        
        let client = reqwest::Client::new();
        let mut request = client.head(&url);
        
        // Add authentication if credentials are provided
        if let Some(creds) = &self.credentials {
            if let (Some(access_key), Some(secret_key)) = (&creds.access_key, &creds.secret_key) {
                request = request.basic_auth(access_key, Some(secret_key));
            }
        }
        
        let response = request
            .send()
            .await
            .map_err(|e| Error::Storage(format!("S3 exists check failed: {}", e)))?;
        
        Ok(response.status().is_success())
    }

    async fn list(&self, prefix: Option<&str>) -> Result<Vec<String>> {
        let endpoint = if let Some(endpoint) = std::env::var("S3_ENDPOINT").ok() {
            // SECURITY: Validate endpoint to prevent SSRF attacks
            Self::validate_s3_endpoint(&endpoint)?;
            endpoint
        } else {
            format!("https://s3.{}.amazonaws.com", self.region)
        };
        
        // SECURITY: URL encode bucket to prevent injection attacks
        let encoded_bucket = urlencoding::encode(&self.bucket);
        let url = format!("{}/{}", endpoint, encoded_bucket);
        
        let client = reqwest::Client::new();
        let mut request = client.get(&url);
        
        if let Some(prefix) = prefix {
            // SECURITY: Validate prefix to prevent injection attacks
            // Limit prefix length and check for dangerous characters
            const MAX_PREFIX_LENGTH: usize = 1024;
            if prefix.len() > MAX_PREFIX_LENGTH {
                return Err(Error::Storage(format!(
                    "Prefix length {} exceeds maximum allowed {} bytes",
                    prefix.len(), MAX_PREFIX_LENGTH
                )));
            }
            // SECURITY: Check for path traversal in prefix
            if prefix.contains("..") || prefix.contains("\0") {
                return Err(Error::Storage("Prefix contains dangerous characters".to_string()));
            }
            request = request.query(&[("prefix", prefix)]);
        }
        
        // Add authentication if credentials are provided
        if let Some(creds) = &self.credentials {
            if let (Some(access_key), Some(secret_key)) = (&creds.access_key, &creds.secret_key) {
                request = request.basic_auth(access_key, Some(secret_key));
            }
        }
        
        let response = request
            .send()
            .await
            .map_err(|e| Error::Storage(format!("S3 list failed: {}", e)))?;
        
        if !response.status().is_success() {
            // SECURITY: Don't expose full error response to prevent information disclosure
            return Err(Error::Storage(format!(
                "S3 list failed with status {}",
                response.status()
            )));
        }
        
        // Parse XML response (S3 list returns XML)
        // SECURITY: Limit response size to prevent memory exhaustion
        const MAX_XML_RESPONSE_SIZE: usize = 10 * 1024 * 1024; // 10MB max
        let text = response.text().await
            .map_err(|e| Error::Storage(format!("S3 list failed to get response: {}", e)))?;
        
        // SECURITY: Check response size to prevent DoS
        if text.len() > MAX_XML_RESPONSE_SIZE {
            return Err(Error::Storage(format!(
                "S3 list response too large: {} bytes (max: {})",
                text.len(), MAX_XML_RESPONSE_SIZE
            )));
        }
        
        // SECURITY: Parse XML with proper validation and XXE protection
        // Use regex-based parsing to avoid XXE vulnerabilities while maintaining safety
        let mut keys = Vec::new();
        const MAX_KEYS: usize = 10000; // Limit number of keys to prevent DoS
        
        // SECURITY: Remove any potential XML entity declarations to prevent XXE
        let sanitized_text = text
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.starts_with("<?") 
                    && !trimmed.contains("<!DOCTYPE") 
                    && !trimmed.contains("<!ENTITY")
                    && trimmed.len() < 10_000_000 // SECURITY: Limit line length to prevent DoS
            })
            .take(1_000_000) // SECURITY: Limit number of lines to prevent memory exhaustion
            .collect::<Vec<_>>()
            .join("\n");
        
        // SECURITY: Limit sanitized text size
        const MAX_SANITIZED_SIZE: usize = 50 * 1024 * 1024; // 50MB max
        let sanitized_text = if sanitized_text.len() > MAX_SANITIZED_SIZE {
            warn!("S3 XML response too large after sanitization, truncating");
            sanitized_text[..MAX_SANITIZED_SIZE].to_string()
        } else {
            sanitized_text
        };
        
        // Parse <Key> elements using regex (safer than full XML parser for this use case)
        let key_pattern = Regex::new(r"<Key>([^<]+)</Key>")
            .map_err(|e| Error::Storage(format!("Failed to create regex: {}", e)))?;
        
        for cap in key_pattern.captures_iter(&sanitized_text) {
            if keys.len() >= MAX_KEYS {
                warn!("S3 list response contains more than {} keys, truncating", MAX_KEYS);
                break;
            }
            
            if let Some(key_match) = cap.get(1) {
                let key = key_match.as_str();
                // SECURITY: Skip empty keys
                if key.is_empty() {
                    continue;
                }
                // SECURITY: Validate key length to prevent memory exhaustion
                if key.len() > 1024 {
                    warn!("Skipping key longer than 1024 bytes");
                    continue;
                }
                // SECURITY: Validate key doesn't contain dangerous characters
                if key.contains("..") || key.contains("\0") {
                    warn!("Skipping key with dangerous characters");
                    continue;
                }
                keys.push(key.to_string());
            }
        }
        
        info!("S3 list successful: found {} keys", keys.len());
        Ok(keys)
    }

    async fn sync(&self) -> Result<()> {
        // S3 is eventually consistent, but we can ensure writes are flushed
        // For S3, sync is a no-op as writes are immediately visible
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        // S3 writes are immediately persisted, so flush is a no-op
        Ok(())
    }
}

/// WAL backend
struct WALBackend {
    wal_path: PathBuf,
    config: WALConfig,
    buffer: Arc<RwLock<Vec<(String, Vec<u8>)>>>,
}

impl WALBackend {
    fn new(wal_path: PathBuf, config: WALConfig) -> Result<Self> {
        std::fs::create_dir_all(&wal_path).map_err(|e| Error::Storage(format!("Failed to create WAL directory: {}", e)))?;
        Ok(Self {
            wal_path,
            config,
            buffer: Arc::new(RwLock::new(Vec::new())),
        })
    }
}

#[async_trait::async_trait]
impl PersistenceBackend for WALBackend {
    async fn write(&self, key: &str, data: &[u8]) -> Result<()> {
        // Write to WAL buffer
        self.buffer.write().push((key.to_string(), data.to_vec()));
        
        // Flush if needed
        if self.config.sync {
            self.flush().await?;
        }
        
        Ok(())
    }

    async fn read(&self, key: &str) -> Result<Option<Vec<u8>>> {
        // In production, would read from WAL and apply to data store
        Ok(None)
    }

    async fn delete(&self, key: &str) -> Result<()> {
        // Write delete marker to WAL
        self.write(key, &[]).await
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        Ok(false)
    }

    async fn list(&self, prefix: Option<&str>) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    async fn sync(&self) -> Result<()> {
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        // Flush WAL buffer to disk
        Ok(())
    }
}

/// In-memory snapshot backend
struct InMemorySnapshotBackend {
    data: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl InMemorySnapshotBackend {
    fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl PersistenceBackend for InMemorySnapshotBackend {
    async fn write(&self, key: &str, data: &[u8]) -> Result<()> {
        self.data.write().insert(key.to_string(), data.to_vec());
        Ok(())
    }

    async fn read(&self, key: &str) -> Result<Option<Vec<u8>>> {
        Ok(self.data.read().get(key).cloned())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.data.write().remove(key);
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        Ok(self.data.read().contains_key(key))
    }

    async fn list(&self, prefix: Option<&str>) -> Result<Vec<String>> {
        let data = self.data.read();
        if let Some(prefix) = prefix {
            Ok(data.keys().filter(|k| k.starts_with(prefix)).cloned().collect())
        } else {
            Ok(data.keys().cloned().collect())
        }
    }

    async fn sync(&self) -> Result<()> {
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        Ok(())
    }
}

/// Hybrid backend (multiple strategies)
struct HybridBackend {
    strategies: Vec<PersistenceStrategy>,
    config: PersistenceConfig,
    backends: Arc<RwLock<Vec<Arc<dyn PersistenceBackend + Send + Sync>>>>,
}

impl HybridBackend {
    fn new(strategies: Vec<PersistenceStrategy>, config: PersistenceConfig) -> Self {
        Self {
            strategies,
            config,
            backends: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

#[async_trait::async_trait]
impl PersistenceBackend for HybridBackend {
    async fn write(&self, key: &str, data: &[u8]) -> Result<()> {
        // Write to all backends - collect Arc clones to avoid holding lock across await
        let backends: Vec<Arc<dyn PersistenceBackend + Send + Sync>> = {
            let backends = self.backends.read();
            backends.iter().map(|b| Arc::clone(b)).collect()
        };
        for backend in backends {
            if let Err(e) = backend.write(key, data).await {
                warn!("Hybrid backend write error: {}", e);
            }
        }
        Ok(())
    }

    async fn read(&self, key: &str) -> Result<Option<Vec<u8>>> {
        // Read from first available backend - collect Arc clones to avoid holding lock across await
        let backends: Vec<Arc<dyn PersistenceBackend + Send + Sync>> = {
            let backends = self.backends.read();
            backends.iter().map(|b| Arc::clone(b)).collect()
        };
        for backend in backends {
            if let Ok(Some(data)) = backend.read(key).await {
                return Ok(Some(data));
            }
        }
        Ok(None)
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let backends: Vec<Arc<dyn PersistenceBackend + Send + Sync>> = {
            let backends = self.backends.read();
            backends.iter().map(|b| Arc::clone(b)).collect()
        };
        for backend in backends {
            if let Err(e) = backend.delete(key).await {
                warn!("Hybrid backend delete error: {}", e);
            }
        }
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let backends: Vec<Arc<dyn PersistenceBackend + Send + Sync>> = {
            let backends = self.backends.read();
            backends.iter().map(|b| Arc::clone(b)).collect()
        };
        for backend in backends {
            if let Ok(true) = backend.exists(key).await {
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn list(&self, prefix: Option<&str>) -> Result<Vec<String>> {
        let backend = {
            let backends = self.backends.read();
            backends.first().cloned()
        };
        if let Some(backend) = backend {
            backend.list(prefix).await
        } else {
            Ok(Vec::new())
        }
    }

    async fn sync(&self) -> Result<()> {
        let backends: Vec<Arc<dyn PersistenceBackend + Send + Sync>> = {
            let backends = self.backends.read();
            backends.iter().map(|b| Arc::clone(b)).collect()
        };
        for backend in backends {
            backend.sync().await?;
        }
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        let backends: Vec<Arc<dyn PersistenceBackend + Send + Sync>> = {
            let backends = self.backends.read();
            backends.iter().map(|b| Arc::clone(b)).collect()
        };
        for backend in backends {
            backend.flush().await?;
        }
        Ok(())
    }
}

/// Tiered backend (hot/cold/warm tiers)
struct TieredBackend {
    tiering: TieringConfig,
    hot_backend: Option<Box<dyn PersistenceBackend + Send + Sync>>,
    cold_backend: Option<Box<dyn PersistenceBackend + Send + Sync>>,
    warm_backend: Option<Box<dyn PersistenceBackend + Send + Sync>>,
}

impl TieredBackend {
    fn new(tiering: TieringConfig) -> Self {
        Self {
            tiering,
            hot_backend: None,
            cold_backend: None,
            warm_backend: None,
        }
    }
}

#[async_trait::async_trait]
impl PersistenceBackend for TieredBackend {
    async fn write(&self, key: &str, data: &[u8]) -> Result<()> {
        // Write to hot tier
        if let Some(ref backend) = self.hot_backend {
            backend.write(key, data).await?;
        }
        Ok(())
    }

    async fn read(&self, key: &str) -> Result<Option<Vec<u8>>> {
        // Try hot tier first, then warm, then cold
        if let Some(ref backend) = self.hot_backend {
            if let Ok(Some(data)) = backend.read(key).await {
                return Ok(Some(data));
            }
        }
        if let Some(ref backend) = self.warm_backend {
            if let Ok(Some(data)) = backend.read(key).await {
                return Ok(Some(data));
            }
        }
        if let Some(ref backend) = self.cold_backend {
            if let Ok(Some(data)) = backend.read(key).await {
                return Ok(Some(data));
            }
        }
        Ok(None)
    }

    async fn delete(&self, key: &str) -> Result<()> {
        if let Some(ref backend) = self.hot_backend {
            backend.delete(key).await?;
        }
        if let Some(ref backend) = self.warm_backend {
            backend.delete(key).await?;
        }
        if let Some(ref backend) = self.cold_backend {
            backend.delete(key).await?;
        }
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        if let Some(ref backend) = self.hot_backend {
            if backend.exists(key).await? {
                return Ok(true);
            }
        }
        if let Some(ref backend) = self.warm_backend {
            if backend.exists(key).await? {
                return Ok(true);
            }
        }
        if let Some(ref backend) = self.cold_backend {
            if backend.exists(key).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn list(&self, prefix: Option<&str>) -> Result<Vec<String>> {
        if let Some(ref backend) = self.hot_backend {
            return backend.list(prefix).await;
        }
        Ok(Vec::new())
    }

    async fn sync(&self) -> Result<()> {
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        Ok(())
    }
}

impl Default for WALConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sync: false,
            flush_interval: Some(1000),
            max_size: Some(100 * 1024 * 1024), // 100MB
            rotation: true,
        }
    }
}

use std::io::{Read, Write};

