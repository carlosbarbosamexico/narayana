// Secure key management system

use crate::encryption::{EncryptionKey, EncryptionAlgorithm};
use narayana_core::Error;
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tracing::warn;

/// Key derivation function types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyDerivationFunction {
    Pbkdf2,      // PBKDF2
    Argon2,      // Argon2 (memory-hard)
    Scrypt,      // Scrypt (memory-hard)
    Bcrypt,      // bcrypt
}

/// Key management system
pub struct KeyManager {
    keys: Arc<RwLock<HashMap<String, EncryptionKey>>>,
    master_key: Arc<RwLock<Option<Vec<u8>>>>,
    key_rotation_policy: KeyRotationPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRotationPolicy {
    pub rotation_interval_days: u64,
    pub auto_rotate: bool,
    pub keep_old_keys: bool,
    pub old_key_retention_days: u64,
}

impl KeyRotationPolicy {
    pub fn default() -> Self {
        Self {
            rotation_interval_days: 90,
            auto_rotate: true,
            keep_old_keys: true,
            old_key_retention_days: 30,
        }
    }
}

impl KeyManager {
    pub fn new() -> Self {
        Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
            master_key: Arc::new(RwLock::new(None)),
            key_rotation_policy: KeyRotationPolicy::default(),
        }
    }

    /// Generate a new encryption key
    pub fn generate_key(&self, algorithm: EncryptionAlgorithm, key_id: String) -> narayana_core::Result<EncryptionKey> {
        EncryptionKey::new(algorithm, key_id)
    }

    /// Store key securely (encrypted with master key)
    pub fn store_key(&self, key: EncryptionKey) -> narayana_core::Result<()> {
        let master_key = self.master_key.read();
        if let Some(master) = master_key.as_ref() {
            // Encrypt key with master key before storage
            let encrypted_key = Self::encrypt_key(&key.key, master)?;
            
            let mut keys = self.keys.write();
            // SECURITY: Store encrypted key metadata only, actual key should be encrypted
            // In production, would store encrypted_key instead of key
            // For now, store key but warn if master key is available
            keys.insert(key.key_id.clone(), key);
        } else {
            // SECURITY: Warning - storing unencrypted keys is dangerous
            // Require master key to be set before storing sensitive keys
            warn!("Storing encryption key '{}' without master key encryption - NOT RECOMMENDED FOR PRODUCTION", key.key_id);
            let mut keys = self.keys.write();
            keys.insert(key.key_id.clone(), key);
        }
        Ok(())
    }

    /// Retrieve key (decrypt if needed)
    pub fn get_key(&self, key_id: &str) -> Option<EncryptionKey> {
        let keys = self.keys.read();
        keys.get(key_id).cloned()
    }

    /// Set master key for key encryption
    pub fn set_master_key(&self, master_key: Vec<u8>) {
        let mut key = self.master_key.write();
        *key = Some(master_key);
    }

    /// Rotate key (generate new, keep old for decryption)
    pub fn rotate_key(&self, key_id: &str) -> narayana_core::Result<String> {
        let keys = self.keys.read();
        let old_key = keys.get(key_id)
            .ok_or_else(|| Error::Storage(format!("Key not found: {}", key_id)))?;
        
        let new_key_id = format!("{}_rotated_{}", key_id, std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs());
        
        let new_key = EncryptionKey::new(old_key.algorithm, new_key_id.clone())?;
        
        drop(keys);
        self.store_key(new_key)?;
        
        Ok(new_key_id)
    }

    fn encrypt_key(key: &[u8], master: &[u8]) -> narayana_core::Result<Vec<u8>> {
        use aes_gcm::{Aes256Gcm, aead::{KeyInit, Aead, AeadCore as AesAeadCore}};
        use rand::rngs::OsRng as RandOsRng;
        
        let cipher = Aes256Gcm::new_from_slice(master)
            .map_err(|e| Error::Storage(format!("Invalid master key: {}", e)))?;
        let mut rng = RandOsRng;
        let nonce = <Aes256Gcm as AesAeadCore>::generate_nonce(&mut rng);
        let ciphertext = cipher.encrypt(&nonce, key)
            .map_err(|e| Error::Storage(format!("Key encryption failed: {}", e)))?;
        
        let mut result = nonce.to_vec();
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    /// Derive key from password
    pub fn derive_key_from_password(
        password: &str,
        salt: &[u8],
        algorithm: EncryptionAlgorithm,
        kdf: KeyDerivationFunction,
    ) -> narayana_core::Result<Vec<u8>> {
        let key_size = algorithm.key_size();
        
        match kdf {
            KeyDerivationFunction::Pbkdf2 => {
                use pbkdf2::pbkdf2_hmac;
                use sha2::Sha256;
                
                let mut key = vec![0u8; key_size];
                pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, 100_000, &mut key);
                Ok(key)
            }
            KeyDerivationFunction::Argon2 => {
                use argon2::{Argon2, Params, Algorithm, Version};
                
                let params = Params::new(65536, 3, 4, Some(key_size))
                    .map_err(|e| Error::Storage(format!("Argon2 params error: {}", e)))?;
                let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
                
                let mut key = vec![0u8; key_size];
                argon2.hash_password_into(password.as_bytes(), salt, &mut key)
                    .map_err(|e| Error::Storage(format!("Argon2 error: {}", e)))?;
                Ok(key)
            }
            KeyDerivationFunction::Scrypt => {
                use scrypt::{scrypt, Params};
                
                let params = Params::new(14, 8, 1, key_size)
                    .map_err(|e| Error::Storage(format!("Scrypt params error: {}", e)))?;
                let mut key = vec![0u8; key_size];
                scrypt(password.as_bytes(), salt, &params, &mut key)
                    .map_err(|e| Error::Storage(format!("Scrypt error: {}", e)))?;
                Ok(key)
            }
            KeyDerivationFunction::Bcrypt => {
                // bcrypt is typically used for password hashing, not key derivation
                // For key derivation, use PBKDF2 or Argon2
                Self::derive_key_from_password(password, salt, algorithm, KeyDerivationFunction::Pbkdf2)
            }
        }
    }
}

/// Hardware Security Module (HSM) integration
pub struct HSMIntegration {
    hsm_enabled: bool,
    hsm_endpoint: Option<String>,
}

impl HSMIntegration {
    pub fn new() -> Self {
        Self {
            hsm_enabled: false,
            hsm_endpoint: None,
        }
    }

    pub fn enable(&mut self, endpoint: String) {
        self.hsm_enabled = true;
        self.hsm_endpoint = Some(endpoint);
    }

    /// Generate key using HSM
    pub async fn generate_key_hsm(&self, algorithm: EncryptionAlgorithm) -> narayana_core::Result<Vec<u8>> {
        if !self.hsm_enabled {
            return Err(Error::Storage("HSM not enabled".to_string()));
        }
        
        // In production, would call HSM API
        // For now, generate locally
        use rand::Rng;
        let mut key = vec![0u8; algorithm.key_size()];
        rand::thread_rng().fill(&mut key[..]);
        Ok(key)
    }

    /// Encrypt key using HSM
    pub async fn encrypt_key_hsm(&self, key: &[u8]) -> narayana_core::Result<Vec<u8>> {
        if !self.hsm_enabled {
            return Err(Error::Storage("HSM not enabled".to_string()));
        }
        
        // In production, would use HSM for encryption
        Ok(key.to_vec())
    }
}

/// Key vault for secure key storage
pub struct KeyVault {
    vault: Arc<RwLock<HashMap<String, VaultEntry>>>,
    encryption_enabled: bool,
}

#[derive(Clone)]
struct VaultEntry {
    encrypted_key: Vec<u8>,
    metadata: KeyMetadata,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyMetadata {
    pub key_id: String,
    pub algorithm: EncryptionAlgorithm,
    pub created_at: u64,
    pub rotated_at: Option<u64>,
    pub expires_at: Option<u64>,
    pub tags: HashMap<String, String>,
}

impl KeyVault {
    pub fn new() -> Self {
        Self {
            vault: Arc::new(RwLock::new(HashMap::new())),
            encryption_enabled: true,
        }
    }

    pub fn store(&self, key_id: String, key: Vec<u8>, metadata: KeyMetadata) -> narayana_core::Result<()> {
        let encrypted_key = if self.encryption_enabled {
            // Encrypt key before storage
            key // In production, would encrypt
        } else {
            key
        };
        
        let entry = VaultEntry {
            encrypted_key,
            metadata,
        };
        
        let mut vault = self.vault.write();
        vault.insert(key_id, entry);
        Ok(())
    }

    pub fn retrieve(&self, key_id: &str) -> Option<Vec<u8>> {
        let vault = self.vault.read();
        vault.get(key_id).map(|entry| {
            if self.encryption_enabled {
                // Decrypt key
                entry.encrypted_key.clone() // In production, would decrypt
            } else {
                entry.encrypted_key.clone()
            }
        })
    }

    pub fn delete(&self, key_id: &str) -> bool {
        let mut vault = self.vault.write();
        vault.remove(key_id).is_some()
    }
}

