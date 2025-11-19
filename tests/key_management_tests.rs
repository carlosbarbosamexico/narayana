// Tests for key management

use narayana_storage::key_management::*;

#[test]
fn test_key_manager_creation() {
    let master_key = vec![0u8; 32];
    let manager = KeyManager::new(master_key);
    // Should create successfully
}

#[test]
fn test_generate_key() {
    let master_key = vec![0u8; 32];
    let manager = KeyManager::new(master_key);
    let key = manager.generate_key("test-key".to_string(), 32).unwrap();
    assert_eq!(key.id, "test-key");
    assert_eq!(key.key.len(), 32);
}

#[test]
fn test_get_key() {
    let master_key = vec![0u8; 32];
    let manager = KeyManager::new(master_key);
    manager.generate_key("test-key".to_string(), 32).unwrap();
    
    let retrieved = manager.get_key("test-key");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id, "test-key");
}

#[test]
fn test_rotate_key() {
    let master_key = vec![0u8; 32];
    let manager = KeyManager::new(master_key);
    manager.generate_key("test-key".to_string(), 32).unwrap();
    
    let old_key = manager.get_key("test-key").unwrap();
    manager.rotate_key("test-key").unwrap();
    let new_key = manager.get_key("test-key").unwrap();
    
    assert_ne!(old_key.key, new_key.key);
}

#[test]
fn test_delete_key() {
    let master_key = vec![0u8; 32];
    let manager = KeyManager::new(master_key);
    manager.generate_key("test-key".to_string(), 32).unwrap();
    
    manager.delete_key("test-key").unwrap();
    assert!(manager.get_key("test-key").is_none());
}

#[test]
fn test_derive_key_from_password_pbkdf2() {
    let salt = b"salt";
    let key = KeyManager::derive_key_from_password(
        "password",
        salt,
        PasswordDerivationAlgorithm::Pbkdf2,
    ).unwrap();
    
    assert_eq!(key.len(), 32);
}

#[test]
fn test_derive_key_from_password_argon2() {
    let salt = b"salt";
    let key = KeyManager::derive_key_from_password(
        "password",
        salt,
        PasswordDerivationAlgorithm::Argon2,
    ).unwrap();
    
    assert_eq!(key.len(), 32);
}

#[test]
fn test_derive_key_from_password_scrypt() {
    let salt = b"salt";
    let key = KeyManager::derive_key_from_password(
        "password",
        salt,
        PasswordDerivationAlgorithm::Scrypt,
    ).unwrap();
    
    assert_eq!(key.len(), 32);
}

#[test]
fn test_key_expiration() {
    let master_key = vec![0u8; 32];
    let manager = KeyManager::new(master_key);
    let mut key = manager.generate_key("test-key".to_string(), 32).unwrap();
    
    // Set expiration
    key.expires_at = Some(std::time::Instant::now() + std::time::Duration::from_secs(3600));
    manager.store_key(key).unwrap();
    
    let retrieved = manager.get_key("test-key");
    assert!(retrieved.is_some());
    assert!(retrieved.unwrap().expires_at.is_some());
}

#[test]
fn test_list_keys() {
    let master_key = vec![0u8; 32];
    let manager = KeyManager::new(master_key);
    
    manager.generate_key("key-1".to_string(), 32).unwrap();
    manager.generate_key("key-2".to_string(), 32).unwrap();
    manager.generate_key("key-3".to_string(), 32).unwrap();
    
    let keys = manager.list_keys();
    assert_eq!(keys.len(), 3);
}

