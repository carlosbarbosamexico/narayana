// Tests for encryption

use narayana_storage::encryption::*;
use narayana_storage::key_management::*;

#[test]
fn test_encryption_config_creation() {
    let config = EncryptionConfig {
        algorithm: EncryptionAlgorithm::Aes256Gcm,
        key_id: "test-key".to_string(),
    };
    assert_eq!(config.algorithm, EncryptionAlgorithm::Aes256Gcm);
    assert_eq!(config.key_id, "test-key");
}

#[test]
fn test_data_encryptor_creation() {
    let key_manager = KeyManager::new(vec![0u8; 32]);
    let encryptor = DataEncryptor::new(key_manager);
    // Should create successfully
}

#[test]
fn test_encrypt_decrypt_aes256_gcm() {
    let key_manager = KeyManager::new(vec![0u8; 32]);
    let encryptor = DataEncryptor::new(key_manager);
    
    let key_id = encryptor.key_manager.generate_key("test-key".to_string(), 32).unwrap().id;
    let config = EncryptionConfig {
        algorithm: EncryptionAlgorithm::Aes256Gcm,
        key_id: key_id.clone(),
    };
    
    let data = b"test data to encrypt";
    let encrypted = encryptor.encrypt(data, &config).unwrap();
    assert_ne!(encrypted, data);
    
    let decrypted = encryptor.decrypt(&encrypted, &config).unwrap();
    assert_eq!(decrypted, data);
}

#[test]
fn test_encrypt_decrypt_chacha20() {
    let key_manager = KeyManager::new(vec![0u8; 32]);
    let encryptor = DataEncryptor::new(key_manager);
    
    let key_id = encryptor.key_manager.generate_key("test-key".to_string(), 32).unwrap().id;
    let config = EncryptionConfig {
        algorithm: EncryptionAlgorithm::ChaCha20Poly1305,
        key_id: key_id.clone(),
    };
    
    let data = b"test data";
    let encrypted = encryptor.encrypt(data, &config).unwrap();
    let decrypted = encryptor.decrypt(&encrypted, &config).unwrap();
    assert_eq!(decrypted, data);
}

#[test]
fn test_encrypt_none() {
    let key_manager = KeyManager::new(vec![0u8; 32]);
    let encryptor = DataEncryptor::new(key_manager);
    
    let config = EncryptionConfig {
        algorithm: EncryptionAlgorithm::None,
        key_id: "none".to_string(),
    };
    
    let data = b"test data";
    let encrypted = encryptor.encrypt(data, &config).unwrap();
    assert_eq!(encrypted, data);
}

#[test]
fn test_multi_granular_encryption_database() {
    let manager = MultiGranularEncryption::new();
    let db_id = "test_db".to_string();
    
    manager.set_database_encryption(db_id.clone(), EncryptionAlgorithm::Aes256Gcm, "key-1".to_string()).unwrap();
    let config = manager.get_database_encryption(&db_id).unwrap();
    assert_eq!(config.algorithm, EncryptionAlgorithm::Aes256Gcm);
}

#[test]
fn test_multi_granular_encryption_table() {
    let manager = MultiGranularEncryption::new();
    let table_id = narayana_core::types::TableId(1);
    
    manager.set_table_encryption(table_id, EncryptionAlgorithm::Aes256Gcm, "key-1".to_string()).unwrap();
    let config = manager.get_table_encryption(&table_id).unwrap();
    assert_eq!(config.algorithm, EncryptionAlgorithm::Aes256Gcm);
}

#[test]
fn test_multi_granular_encryption_column() {
    let manager = MultiGranularEncryption::new();
    let table_id = narayana_core::types::TableId(1);
    let column_id = 0u32;
    
    manager.set_column_encryption(table_id, column_id, EncryptionAlgorithm::Aes256Gcm, "key-1".to_string()).unwrap();
    let config = manager.get_column_encryption(&table_id, column_id).unwrap();
    assert_eq!(config.algorithm, EncryptionAlgorithm::Aes256Gcm);
}

#[test]
fn test_multi_granular_encryption_record() {
    let manager = MultiGranularEncryption::new();
    let table_id = narayana_core::types::TableId(1);
    let record_id = 123u64;
    
    manager.set_record_encryption(table_id, record_id, EncryptionAlgorithm::Aes256Gcm, "key-1".to_string()).unwrap();
    let config = manager.get_record_encryption(&table_id, record_id).unwrap();
    assert_eq!(config.algorithm, EncryptionAlgorithm::Aes256Gcm);
}

#[test]
fn test_encryption_hierarchy() {
    let manager = MultiGranularEncryption::new();
    let table_id = narayana_core::types::TableId(1);
    let column_id = 0u32;
    let record_id = 123u64;
    
    // Set database encryption
    manager.set_database_encryption("test_db".to_string(), EncryptionAlgorithm::Aes256Gcm, "db-key".to_string()).unwrap();
    
    // Set table encryption (overrides database)
    manager.set_table_encryption(table_id, EncryptionAlgorithm::ChaCha20Poly1305, "table-key".to_string()).unwrap();
    
    // Set column encryption (overrides table)
    manager.set_column_encryption(table_id, column_id, EncryptionAlgorithm::Aes256Gcm, "column-key".to_string()).unwrap();
    
    // Set record encryption (overrides column)
    manager.set_record_encryption(table_id, record_id, EncryptionAlgorithm::ChaCha20Poly1305, "record-key".to_string()).unwrap();
    
    // Record encryption should take precedence
    let config = manager.get_encryption_for_record(&table_id, record_id, column_id).unwrap();
    assert_eq!(config.algorithm, EncryptionAlgorithm::ChaCha20Poly1305);
}

