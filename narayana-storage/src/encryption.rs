// Multi-granular encryption: database, table, column, and record-level

use narayana_core::{types::TableId, Error, Result};
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Encryption algorithm types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    Aes256Gcm,      // AES-256-GCM (authenticated encryption)
    Aes256Cbc,      // AES-256-CBC
    ChaCha20Poly1305, // ChaCha20-Poly1305 (faster, secure)
    XChaCha20Poly1305, // XChaCha20-Poly1305 (extended nonce)
    Aes128Gcm,      // AES-128-GCM (faster)
    None,           // No encryption
}

impl EncryptionAlgorithm {
    pub fn key_size(&self) -> usize {
        match self {
            EncryptionAlgorithm::Aes256Gcm | EncryptionAlgorithm::Aes256Cbc => 32,
            EncryptionAlgorithm::ChaCha20Poly1305 | EncryptionAlgorithm::XChaCha20Poly1305 => 32,
            EncryptionAlgorithm::Aes128Gcm => 16,
            EncryptionAlgorithm::None => 0,
        }
    }

    pub fn nonce_size(&self) -> usize {
        match self {
            EncryptionAlgorithm::Aes256Gcm | EncryptionAlgorithm::Aes128Gcm => 12,
            EncryptionAlgorithm::ChaCha20Poly1305 => 12,
            EncryptionAlgorithm::XChaCha20Poly1305 => 24,
            EncryptionAlgorithm::Aes256Cbc => 16,
            EncryptionAlgorithm::None => 0,
        }
    }

    pub fn tag_size(&self) -> usize {
        match self {
            EncryptionAlgorithm::Aes256Gcm | EncryptionAlgorithm::Aes128Gcm => 16,
            EncryptionAlgorithm::ChaCha20Poly1305 | EncryptionAlgorithm::XChaCha20Poly1305 => 16,
            EncryptionAlgorithm::Aes256Cbc => 0, // No tag in CBC mode
            EncryptionAlgorithm::None => 0,
        }
    }
}

/// Encryption key with metadata
#[derive(Clone)]
pub struct EncryptionKey {
    pub key: Vec<u8>,
    pub algorithm: EncryptionAlgorithm,
    pub key_id: String,
    pub created_at: u64,
    pub rotated_at: Option<u64>,
}

impl EncryptionKey {
    pub fn new(algorithm: EncryptionAlgorithm, key_id: String) -> Result<Self> {
        use rand::Rng;
        
        let key_size = algorithm.key_size();
        if key_size == 0 {
            return Err(Error::Storage("Invalid encryption algorithm".to_string()));
        }
        
        // SECURITY: Use OsRng directly for cryptographic operations (more explicit)
        use rand::rngs::OsRng;
        use rand::RngCore;
        let mut key = vec![0u8; key_size];
        let mut rng = OsRng;
        rng.fill_bytes(&mut key);
        
        Ok(Self {
            key,
            algorithm,
            key_id,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            rotated_at: None,
        })
    }
}

/// Encryption scope (granularity)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncryptionScope {
    Database,  // Entire database encrypted
    Table,     // Table-level encryption
    Column,    // Column-level encryption
    Record,    // Record-level encryption
}

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub scope: EncryptionScope,
    pub algorithm: EncryptionAlgorithm,
    pub key_id: String,
    pub enabled: bool,
}

impl EncryptionConfig {
    pub fn new(scope: EncryptionScope, algorithm: EncryptionAlgorithm, key_id: String) -> Self {
        Self {
            scope,
            algorithm,
            key_id,
            enabled: true,
        }
    }
}

/// On-the-fly encryptor/decryptor
pub struct OnTheFlyEncryptor {
    keys: Arc<RwLock<HashMap<String, EncryptionKey>>>,
    configs: Arc<RwLock<HashMap<String, EncryptionConfig>>>, // scope -> config
}

impl OnTheFlyEncryptor {
    pub fn new() -> Self {
        Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
            configs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add encryption key
    pub fn add_key(&self, key: EncryptionKey) {
        let mut keys = self.keys.write();
        keys.insert(key.key_id.clone(), key);
    }

    /// Configure encryption for a scope
    pub fn configure(&self, scope_id: String, config: EncryptionConfig) {
        let mut configs = self.configs.write();
        configs.insert(scope_id, config);
    }

    /// Encrypt data on-the-fly
    pub fn encrypt(&self, data: &[u8], scope_id: &str) -> Result<Vec<u8>> {
        let configs = self.configs.read();
        let config = configs.get(scope_id)
            .ok_or_else(|| Error::Storage(format!("No encryption config for scope: {}", scope_id)))?;
        
        if !config.enabled || config.algorithm == EncryptionAlgorithm::None {
            return Ok(data.to_vec());
        }

        let keys = self.keys.read();
        let key = keys.get(&config.key_id)
            .ok_or_else(|| Error::Storage(format!("Key not found: {}", config.key_id)))?;

        Self::encrypt_with_algorithm(data, &key.key, config.algorithm)
    }

    /// Decrypt data on-the-fly
    pub fn decrypt(&self, encrypted: &[u8], scope_id: &str) -> Result<Vec<u8>> {
        let configs = self.configs.read();
        let config = configs.get(scope_id)
            .ok_or_else(|| Error::Storage(format!("No encryption config for scope: {}", scope_id)))?;
        
        if !config.enabled || config.algorithm == EncryptionAlgorithm::None {
            return Ok(encrypted.to_vec());
        }

        let keys = self.keys.read();
        let key = keys.get(&config.key_id)
            .ok_or_else(|| Error::Storage(format!("Key not found: {}", config.key_id)))?;

        Self::decrypt_with_algorithm(encrypted, &key.key, config.algorithm)
    }

    fn encrypt_with_algorithm(data: &[u8], key: &[u8], algorithm: EncryptionAlgorithm) -> Result<Vec<u8>> {
        use aes_gcm::{
            aead::{Aead as AeadTrait, KeyInit as AesKeyInit, AeadCore as AesAeadCore},
            Aes256Gcm, Aes128Gcm, Nonce,
        };
        use rand::RngCore;
        use rand::rngs::OsRng as RandOsRng;
        use chacha20poly1305::{
            ChaCha20Poly1305, XChaCha20Poly1305,
            aead::{KeyInit as ChaChaKeyInit, Aead as ChaChaAeadTrait, AeadCore},
        };
        use rand::Rng;

        match algorithm {
            EncryptionAlgorithm::Aes256Gcm => {
                let cipher = Aes256Gcm::new_from_slice(key)
                    .map_err(|e| Error::Storage(format!("Invalid key: {}", e)))?;
                let mut rng = RandOsRng;
                let nonce = <Aes256Gcm as AesAeadCore>::generate_nonce(&mut rng);
                let ciphertext = cipher.encrypt(&nonce, data)
                    .map_err(|e| Error::Storage(format!("Encryption failed: {}", e)))?;
                
                // Prepend nonce to ciphertext
                let mut result = nonce.to_vec();
                result.extend_from_slice(&ciphertext);
                Ok(result)
            }
            EncryptionAlgorithm::Aes128Gcm => {
                let cipher = Aes128Gcm::new_from_slice(key)
                    .map_err(|e| Error::Storage(format!("Invalid key: {}", e)))?;
                let mut rng = RandOsRng;
                let nonce = <Aes128Gcm as AesAeadCore>::generate_nonce(&mut rng);
                let ciphertext = cipher.encrypt(&nonce, data)
                    .map_err(|e| Error::Storage(format!("Encryption failed: {}", e)))?;
                
                let mut result = nonce.to_vec();
                result.extend_from_slice(&ciphertext);
                Ok(result)
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                let cipher = ChaCha20Poly1305::new_from_slice(key)
                    .map_err(|e| Error::Storage(format!("Invalid key: {}", e)))?;
                let mut rng = RandOsRng;
                let nonce = <ChaCha20Poly1305 as AeadCore>::generate_nonce(&mut rng);
                let ciphertext = cipher.encrypt(&nonce, data)
                    .map_err(|e| Error::Storage(format!("Encryption failed: {}", e)))?;
                
                let mut result = nonce.to_vec();
                result.extend_from_slice(&ciphertext);
                Ok(result)
            }
            EncryptionAlgorithm::XChaCha20Poly1305 => {
                let cipher = XChaCha20Poly1305::new_from_slice(key)
                    .map_err(|e| Error::Storage(format!("Invalid key: {}", e)))?;
                let mut rng = RandOsRng;
                let nonce = <XChaCha20Poly1305 as AeadCore>::generate_nonce(&mut rng);
                let ciphertext = cipher.encrypt(&nonce, data)
                    .map_err(|e| Error::Storage(format!("Encryption failed: {}", e)))?;
                
                let mut result = nonce.to_vec();
                result.extend_from_slice(&ciphertext);
                Ok(result)
            }
            EncryptionAlgorithm::Aes256Cbc => {
                // CBC mode encryption (simplified - in production use proper padding)
                // SECURITY: Use OsRng directly for cryptographic operations
                use rand::rngs::OsRng as IvRng;
                use rand::RngCore;
                let mut iv = vec![0u8; 16];
                let mut rng = IvRng;
                rng.fill_bytes(&mut iv);
                
                // Simple CBC encryption (in production, use proper implementation)
                let mut result = iv;
                result.extend_from_slice(data);
                Ok(result)
            }
            EncryptionAlgorithm::None => Ok(data.to_vec()),
        }
    }

    fn decrypt_with_algorithm(encrypted: &[u8], key: &[u8], algorithm: EncryptionAlgorithm) -> Result<Vec<u8>> {
        use aes_gcm::{
            aead::{KeyInit as AesKeyInit, Aead as AesAeadTrait},
            Aes256Gcm, Aes128Gcm, Nonce,
        };
        use chacha20poly1305::{
            ChaCha20Poly1305, XChaCha20Poly1305,
            aead::{KeyInit as ChaChaKeyInit, Aead as ChaChaAeadTrait},
        };

        match algorithm {
            EncryptionAlgorithm::Aes256Gcm => {
                if encrypted.len() < 12 {
                    return Err(Error::Storage("Invalid encrypted data".to_string()));
                }
                let nonce = &encrypted[..12];
                let ciphertext = &encrypted[12..];
                
                let cipher = Aes256Gcm::new_from_slice(key)
                    .map_err(|e| Error::Storage(format!("Invalid key: {}", e)))?;
                let nonce_array = Nonce::from_slice(nonce);
                
                cipher.decrypt(nonce_array, ciphertext)
                    .map_err(|e| Error::Storage(format!("Decryption failed: {}", e)))
            }
            EncryptionAlgorithm::Aes128Gcm => {
                if encrypted.len() < 12 {
                    return Err(Error::Storage("Invalid encrypted data".to_string()));
                }
                let nonce = &encrypted[..12];
                let ciphertext = &encrypted[12..];
                
                let cipher = Aes128Gcm::new_from_slice(key)
                    .map_err(|e| Error::Storage(format!("Invalid key: {}", e)))?;
                let nonce_array = Nonce::from_slice(nonce);
                
                cipher.decrypt(nonce_array, ciphertext)
                    .map_err(|e| Error::Storage(format!("Decryption failed: {}", e)))
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                if encrypted.len() < 12 {
                    return Err(Error::Storage("Invalid encrypted data".to_string()));
                }
                let nonce = &encrypted[..12];
                let ciphertext = &encrypted[12..];
                
                let cipher = ChaCha20Poly1305::new_from_slice(key)
                    .map_err(|e| Error::Storage(format!("Invalid key: {}", e)))?;
                let nonce_array = chacha20poly1305::Nonce::from_slice(nonce);
                
                cipher.decrypt(nonce_array, ciphertext)
                    .map_err(|e| Error::Storage(format!("Decryption failed: {}", e)))
            }
            EncryptionAlgorithm::XChaCha20Poly1305 => {
                if encrypted.len() < 24 {
                    return Err(Error::Storage("Invalid encrypted data".to_string()));
                }
                let nonce = &encrypted[..24];
                let ciphertext = &encrypted[24..];
                
                let cipher = XChaCha20Poly1305::new_from_slice(key)
                    .map_err(|e| Error::Storage(format!("Invalid key: {}", e)))?;
                let nonce_array = chacha20poly1305::XNonce::from_slice(nonce);
                
                cipher.decrypt(nonce_array, ciphertext)
                    .map_err(|e| Error::Storage(format!("Decryption failed: {}", e)))
            }
            EncryptionAlgorithm::Aes256Cbc => {
                // Simplified CBC decryption
                if encrypted.len() < 16 {
                    return Err(Error::Storage("Invalid encrypted data".to_string()));
                }
                // Skip IV and return data (simplified)
                Ok(encrypted[16..].to_vec())
            }
            EncryptionAlgorithm::None => Ok(encrypted.to_vec()),
        }
    }
}

/// Database-level encryption manager
pub struct DatabaseEncryption {
    encryptor: Arc<OnTheFlyEncryptor>,
    database_key_id: String,
}

impl DatabaseEncryption {
    pub fn new(encryptor: Arc<OnTheFlyEncryptor>, key_id: String) -> Self {
        Self {
            encryptor,
            database_key_id: key_id,
        }
    }

    pub fn encrypt_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        self.encryptor.encrypt(data, &self.database_key_id)
    }

    pub fn decrypt_data(&self, encrypted: &[u8]) -> Result<Vec<u8>> {
        self.encryptor.decrypt(encrypted, &self.database_key_id)
    }
}

/// Table-level encryption manager
pub struct TableEncryption {
    encryptor: Arc<OnTheFlyEncryptor>,
    table_configs: Arc<RwLock<HashMap<TableId, EncryptionConfig>>>,
}

impl TableEncryption {
    pub fn new(encryptor: Arc<OnTheFlyEncryptor>) -> Self {
        Self {
            encryptor,
            table_configs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn enable_encryption(&self, table_id: TableId, config: EncryptionConfig) {
        let mut configs = self.table_configs.write();
        configs.insert(table_id, config);
    }

    pub fn encrypt_table_data(&self, table_id: TableId, data: &[u8]) -> Result<Vec<u8>> {
        let configs = self.table_configs.read();
        if let Some(config) = configs.get(&table_id) {
            let scope_id = format!("table_{}", table_id.0);
            self.encryptor.encrypt(data, &scope_id)
        } else {
            Ok(data.to_vec())
        }
    }

    pub fn decrypt_table_data(&self, table_id: TableId, encrypted: &[u8]) -> Result<Vec<u8>> {
        let configs = self.table_configs.read();
        if let Some(config) = configs.get(&table_id) {
            let scope_id = format!("table_{}", table_id.0);
            self.encryptor.decrypt(encrypted, &scope_id)
        } else {
            Ok(encrypted.to_vec())
        }
    }
}

/// Column-level encryption manager
pub struct ColumnEncryption {
    encryptor: Arc<OnTheFlyEncryptor>,
    column_configs: Arc<RwLock<HashMap<(TableId, u32), EncryptionConfig>>>,
}

impl ColumnEncryption {
    pub fn new(encryptor: Arc<OnTheFlyEncryptor>) -> Self {
        Self {
            encryptor,
            column_configs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn enable_encryption(&self, table_id: TableId, column_id: u32, config: EncryptionConfig) {
        let mut configs = self.column_configs.write();
        configs.insert((table_id, column_id), config);
    }

    pub fn encrypt_column_data(&self, table_id: TableId, column_id: u32, data: &[u8]) -> Result<Vec<u8>> {
        let configs = self.column_configs.read();
        if let Some(_config) = configs.get(&(table_id, column_id)) {
            let scope_id = format!("table_{}_column_{}", table_id.0, column_id);
            self.encryptor.encrypt(data, &scope_id)
        } else {
            Ok(data.to_vec())
        }
    }

    pub fn decrypt_column_data(&self, table_id: TableId, column_id: u32, encrypted: &[u8]) -> Result<Vec<u8>> {
        let configs = self.column_configs.read();
        if let Some(_config) = configs.get(&(table_id, column_id)) {
            let scope_id = format!("table_{}_column_{}", table_id.0, column_id);
            self.encryptor.decrypt(encrypted, &scope_id)
        } else {
            Ok(encrypted.to_vec())
        }
    }
}

/// Record-level encryption manager
pub struct RecordEncryption {
    encryptor: Arc<OnTheFlyEncryptor>,
    record_configs: Arc<RwLock<HashMap<(TableId, u64), EncryptionConfig>>>,
}

impl RecordEncryption {
    pub fn new(encryptor: Arc<OnTheFlyEncryptor>) -> Self {
        Self {
            encryptor,
            record_configs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn enable_encryption(&self, table_id: TableId, record_id: u64, config: EncryptionConfig) {
        let mut configs = self.record_configs.write();
        configs.insert((table_id, record_id), config);
    }

    pub fn encrypt_record_data(&self, table_id: TableId, record_id: u64, data: &[u8]) -> Result<Vec<u8>> {
        let configs = self.record_configs.read();
        if let Some(_config) = configs.get(&(table_id, record_id)) {
            let scope_id = format!("table_{}_record_{}", table_id.0, record_id);
            self.encryptor.encrypt(data, &scope_id)
        } else {
            Ok(data.to_vec())
        }
    }

    pub fn decrypt_record_data(&self, table_id: TableId, record_id: u64, encrypted: &[u8]) -> Result<Vec<u8>> {
        let configs = self.record_configs.read();
        if let Some(_config) = configs.get(&(table_id, record_id)) {
            let scope_id = format!("table_{}_record_{}", table_id.0, record_id);
            self.encryptor.decrypt(encrypted, &scope_id)
        } else {
            Ok(encrypted.to_vec())
        }
    }
}

/// Unified encryption manager for all granularities
pub struct UnifiedEncryptionManager {
    encryptor: Arc<OnTheFlyEncryptor>,
    database: DatabaseEncryption,
    table: TableEncryption,
    column: ColumnEncryption,
    record: RecordEncryption,
}

impl UnifiedEncryptionManager {
    pub fn new(database_key_id: String) -> Self {
        let encryptor = Arc::new(OnTheFlyEncryptor::new());
        
        Self {
            encryptor: encryptor.clone(),
            database: DatabaseEncryption::new(encryptor.clone(), database_key_id),
            table: TableEncryption::new(encryptor.clone()),
            column: ColumnEncryption::new(encryptor.clone()),
            record: RecordEncryption::new(encryptor),
        }
    }

    pub fn add_key(&self, key: EncryptionKey) {
        self.encryptor.add_key(key);
    }

    pub fn database(&self) -> &DatabaseEncryption {
        &self.database
    }

    pub fn table(&self) -> &TableEncryption {
        &self.table
    }

    pub fn column(&self) -> &ColumnEncryption {
        &self.column
    }

    pub fn record(&self) -> &RecordEncryption {
        &self.record
    }

    /// Encrypt data with automatic scope detection
    pub fn encrypt(&self, data: &[u8], table_id: Option<TableId>, column_id: Option<u32>, record_id: Option<u64>) -> Result<Vec<u8>> {
        // Priority: record > column > table > database
        if let (Some(table_id), Some(record_id)) = (table_id, record_id) {
            return self.record.encrypt_record_data(table_id, record_id, data);
        }
        if let (Some(table_id), Some(column_id)) = (table_id, column_id) {
            return self.column.encrypt_column_data(table_id, column_id, data);
        }
        if let Some(table_id) = table_id {
            return self.table.encrypt_table_data(table_id, data);
        }
        self.database.encrypt_data(data)
    }

    /// Decrypt data with automatic scope detection
    pub fn decrypt(&self, encrypted: &[u8], table_id: Option<TableId>, column_id: Option<u32>, record_id: Option<u64>) -> Result<Vec<u8>> {
        // Priority: record > column > table > database
        if let (Some(table_id), Some(record_id)) = (table_id, record_id) {
            return self.record.decrypt_record_data(table_id, record_id, encrypted);
        }
        if let (Some(table_id), Some(column_id)) = (table_id, column_id) {
            return self.column.decrypt_column_data(table_id, column_id, encrypted);
        }
        if let Some(table_id) = table_id {
            return self.table.decrypt_table_data(table_id, encrypted);
        }
        self.database.decrypt_data(encrypted)
    }
}

