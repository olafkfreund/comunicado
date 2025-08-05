//! Tests for the encryption module

use super::*;
use crate::encryption::types::{
    KeyCapabilities, MessageSecurityStatus, SignatureValidity, TrustLevel
};

#[tokio::test]
async fn test_encryption_manager_creation() {
    let result = EncryptionManager::new();
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_config_operations() {
    let manager = EncryptionManager::new().unwrap();
    
    let default_config = manager.get_config().await;
    assert!(!default_config.always_sign);
    assert!(!default_config.auto_encrypt);
    
    let mut new_config = default_config;
    new_config.always_sign = true;
    new_config.auto_encrypt = true;
    
    manager.set_config(new_config.clone()).await;
    
    let updated_config = manager.get_config().await;
    assert_eq!(updated_config.always_sign, new_config.always_sign);
    assert_eq!(updated_config.auto_encrypt, new_config.auto_encrypt);
}

#[test]
fn test_key_info_capabilities() {
    let key_info = KeyInfo {
        key_id: "test_key".to_string(),
        fingerprint: "test_fingerprint".to_string(),
        user_ids: vec!["Test User <test@example.com>".to_string()],
        email: Some("test@example.com".to_string()),
        creation_date: None,
        expiration_date: None,
        is_secret: false,
        is_expired: false,
        is_revoked: false,
        trust_level: TrustLevel::Full,
        capabilities: KeyCapabilities {
            can_encrypt: true,
            can_sign: true,
            can_certify: false,
            can_authenticate: false,
        },
    };
    
    assert!(key_info.can_encrypt());
    assert!(key_info.can_sign());
    assert_eq!(key_info.primary_identity(), Some("test@example.com"));
}

#[test]
fn test_expired_key() {
    let expired_key = KeyInfo {
        key_id: "expired_key".to_string(),
        fingerprint: "expired_fingerprint".to_string(),
        user_ids: vec!["Expired User <expired@example.com>".to_string()],
        email: Some("expired@example.com".to_string()),
        creation_date: None,
        expiration_date: Some(chrono::Utc::now() - chrono::Duration::days(1)),
        is_secret: false,
        is_expired: true,
        is_revoked: false,
        trust_level: TrustLevel::Full,
        capabilities: KeyCapabilities {
            can_encrypt: true,
            can_sign: true,
            can_certify: false,
            can_authenticate: false,
        },
    };
    
    assert!(!expired_key.can_encrypt()); // Expired keys can't encrypt
    assert!(!expired_key.can_sign()); // Expired keys can't sign
}

#[test]
fn test_message_security_status() {
    let unencrypted_unsigned = MessageSecurityStatus::none();
    assert!(!unencrypted_unsigned.has_security());
    assert_eq!(unencrypted_unsigned.summary(), "No encryption or signatures");
    
    let encrypted_status = EncryptionStatus::encrypted_and_decrypted(
        vec!["test@example.com".to_string()],
        Some("AES256".to_string())
    );
    
    let signed_status = DecryptionStatus::signed(vec![
        SignatureInfo {
            key_id: "test_key".to_string(),
            signer_email: Some("test@example.com".to_string()),
            signer_name: Some("Test User".to_string()),
            creation_time: Some(chrono::Utc::now()),
            validity: SignatureValidity::Valid,
            trust_level: TrustLevel::Full,
        }
    ]);
    
    let encrypted_and_signed = MessageSecurityStatus {
        encryption: encrypted_status,
        signatures: signed_status,
    };
    
    assert!(encrypted_and_signed.has_security());
    assert_eq!(encrypted_and_signed.summary(), "Encrypted and signed ✓");
}

#[test]
fn test_trust_level_display() {
    assert_eq!(TrustLevel::Ultimate.to_string(), "Ultimate");
    assert_eq!(TrustLevel::Full.to_string(), "Full");
    assert_eq!(TrustLevel::Marginal.to_string(), "Marginal");
    assert_eq!(TrustLevel::Never.to_string(), "Never");
    assert_eq!(TrustLevel::Unknown.to_string(), "Unknown");
}

#[test]  
fn test_signature_validity_display() {
    assert_eq!(SignatureValidity::Valid.to_string(), "Valid");
    assert_eq!(SignatureValidity::Invalid.to_string(), "Invalid");
    assert_eq!(SignatureValidity::KeyNotFound.to_string(), "Key Not Found");
    assert_eq!(SignatureValidity::KeyExpired.to_string(), "Key Expired"); 
    assert_eq!(SignatureValidity::KeyRevoked.to_string(), "Key Revoked");
    assert_eq!(SignatureValidity::BadSignature.to_string(), "Bad Signature");
    assert_eq!(SignatureValidity::Unknown.to_string(), "Unknown");
}