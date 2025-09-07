//! Encryption manager that coordinates GPG operations and integrates with the email system

use super::gpg::{GpgBackend, GpgConfig, SystemGpgBackend};
use super::types::{
    EncryptionConfig, EncryptionError, EncryptionResult, KeyInfo, MessageSecurityStatus,
    SecureEmailContent,
};
use crate::email::StoredMessage;
// async_trait import removed - not used in this file
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Main encryption manager
pub struct EncryptionManager {
    /// GPG backend for crypto operations
    backend: Arc<dyn GpgBackend>,

    /// Configuration
    config: Arc<RwLock<EncryptionConfig>>,

    /// Cached keys for performance
    key_cache: Arc<RwLock<std::collections::HashMap<String, KeyInfo>>>,
}

impl EncryptionManager {
    /// Create a new encryption manager with system GPG backend
    pub fn new() -> EncryptionResult<Self> {
        let gpg_config = GpgConfig::default();
        let backend = Arc::new(SystemGpgBackend::new(gpg_config));

        Ok(Self {
            backend,
            config: Arc::new(RwLock::new(EncryptionConfig::default())),
            key_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        })
    }

    /// Create encryption manager with custom GPG configuration
    pub fn with_gpg_config(gpg_config: GpgConfig) -> EncryptionResult<Self> {
        let backend = Arc::new(SystemGpgBackend::new(gpg_config));

        Ok(Self {
            backend,
            config: Arc::new(RwLock::new(EncryptionConfig::default())),
            key_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        })
    }

    /// Create encryption manager with custom backend
    pub fn with_backend(backend: Arc<dyn GpgBackend>) -> Self {
        Self {
            backend,
            config: Arc::new(RwLock::new(EncryptionConfig::default())),
            key_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Get the current configuration
    pub async fn get_config(&self) -> EncryptionConfig {
        self.config.read().await.clone()
    }

    /// Update configuration
    pub async fn set_config(&self, new_config: EncryptionConfig) {
        let mut config = self.config.write().await;
        *config = new_config;
        info!("Encryption configuration updated");
    }

    /// List all available keys
    pub async fn list_keys(&self, secret_only: bool) -> EncryptionResult<Vec<KeyInfo>> {
        debug!("Listing GPG keys (secret_only: {})", secret_only);

        let keys = self.backend.list_keys(secret_only).await?;

        // Update cache
        let mut cache = self.key_cache.write().await;
        for key in &keys {
            cache.insert(key.key_id.clone(), key.clone());
            cache.insert(key.fingerprint.clone(), key.clone());
            if let Some(ref email) = key.email {
                cache.insert(email.clone(), key.clone());
            }
        }

        info!("Retrieved {} keys from GPG", keys.len());
        Ok(keys)
    }

    /// Get information about a specific key
    pub async fn get_key_info(&self, key_id: &str) -> EncryptionResult<KeyInfo> {
        // Check cache first
        {
            let cache = self.key_cache.read().await;
            if let Some(cached_key) = cache.get(key_id) {
                debug!("Returning cached key info for: {}", key_id);
                return Ok(cached_key.clone());
            }
        }

        debug!("Fetching key info from GPG for: {}", key_id);

        let key_info = self.backend.get_key_info(key_id).await?;

        // Update cache
        let mut cache = self.key_cache.write().await;
        cache.insert(key_info.key_id.clone(), key_info.clone());
        cache.insert(key_info.fingerprint.clone(), key_info.clone());
        if let Some(ref email) = key_info.email {
            cache.insert(email.clone(), key_info.clone());
        }

        Ok(key_info)
    }

    /// Find keys for email addresses
    pub async fn find_keys_for_emails(&self, emails: &[String]) -> EncryptionResult<Vec<KeyInfo>> {
        let mut found_keys = Vec::new();

        // First, try to get from cache
        {
            let cache = self.key_cache.read().await;
            for email in emails {
                if let Some(key) = cache.get(email) {
                    if key.can_encrypt() {
                        found_keys.push(key.clone());
                    }
                }
            }
        }

        // If we found keys for all emails, return them
        if found_keys.len() == emails.len() {
            return Ok(found_keys);
        }

        // Otherwise, refresh key list and try again
        info!("Refreshing key list to find keys for email addresses");
        let all_keys = self.list_keys(false).await?;

        found_keys.clear();
        for email in emails {
            if let Some(key) = all_keys.iter().find(|k| {
                k.email.as_ref() == Some(email) || k.user_ids.iter().any(|uid| uid.contains(email))
            }) {
                if key.can_encrypt() {
                    found_keys.push(key.clone());
                } else {
                    warn!("Key found for {} but cannot encrypt", email);
                }
            } else {
                warn!("No encryption key found for email: {}", email);
            }
        }

        Ok(found_keys)
    }

    /// Encrypt email content for recipients
    pub async fn encrypt_email(
        &self,
        content: &str,
        recipients: &[String],
        sign_with: Option<&str>,
    ) -> EncryptionResult<String> {
        info!("Encrypting email for {} recipients", recipients.len());

        // Validate that we have keys for all recipients
        let recipient_keys = self.find_keys_for_emails(recipients).await?;
        if recipient_keys.len() != recipients.len() {
            let missing: Vec<_> = recipients
                .iter()
                .filter(|email| {
                    !recipient_keys
                        .iter()
                        .any(|key| key.email.as_ref() == Some(email))
                })
                .collect();
            return Err(EncryptionError::KeyNotFound(format!(
                "Missing keys for: {}",
                missing
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }

        // If signing is requested, validate signing key
        if let Some(signer) = sign_with {
            let signing_key = self.get_key_info(signer).await?;
            if !signing_key.can_sign() {
                return Err(EncryptionError::InvalidKey(format!(
                    "Key {} cannot be used for signing",
                    signer
                )));
            }
        }

        debug!("All recipient keys validated, proceeding with encryption");
        self.backend.encrypt(content, recipients, sign_with).await
    }

    /// Decrypt email content
    pub async fn decrypt_email(
        &self,
        encrypted_content: &str,
    ) -> EncryptionResult<SecureEmailContent> {
        debug!("Decrypting email content");
        self.backend.decrypt_email(encrypted_content).await
    }

    /// Sign email content
    pub async fn sign_email(
        &self,
        content: &str,
        key_id: Option<&str>,
    ) -> EncryptionResult<String> {
        let signer = if let Some(key_id) = key_id {
            key_id.to_string()
        } else {
            // Use default signing key from config
            let config = self.config.read().await;
            config.default_signing_key.clone().ok_or_else(|| {
                EncryptionError::ConfigError("No default signing key configured".to_string())
            })?
        };

        info!("Signing email with key: {}", signer);

        // Validate signing key
        let signing_key = self.get_key_info(&signer).await?;
        if !signing_key.can_sign() {
            return Err(EncryptionError::InvalidKey(format!(
                "Key {} cannot be used for signing",
                signer
            )));
        }

        self.backend.sign(content, &signer).await
    }

    /// Process an incoming email to decrypt/verify
    pub async fn process_incoming_email(
        &self,
        message: &mut StoredMessage,
    ) -> EncryptionResult<MessageSecurityStatus> {
        let content = message.body_text.as_ref().ok_or_else(|| {
            EncryptionError::ConfigError("No message content to process".to_string())
        })?;

        debug!("Processing incoming email for encryption/signatures");

        let secure_content = self.decrypt_email(content).await?;

        // Update message with decrypted content if available
        if let Some(ref decrypted) = secure_content.decrypted_content {
            message.body_text = Some(decrypted.clone());
            info!("Email successfully decrypted");
        }

        Ok(secure_content.security_status)
    }

    /// Prepare outgoing email for encryption/signing based on configuration
    pub async fn prepare_outgoing_email(
        &self,
        content: &str,
        recipients: &[String],
    ) -> EncryptionResult<(String, MessageSecurityStatus)> {
        let config = self.config.read().await;
        let mut final_content = content.to_string();
        let mut security_status = MessageSecurityStatus::none();

        // Check if we should encrypt
        let should_encrypt = config.auto_encrypt && !recipients.is_empty();

        // Check if we should sign
        let should_sign = config.always_sign && config.default_signing_key.is_some();

        if should_encrypt {
            debug!("Auto-encryption enabled, checking for recipient keys");
            match self.find_keys_for_emails(recipients).await {
                Ok(keys) if keys.len() == recipients.len() => {
                    info!(
                        "Encrypting outgoing email for {} recipients",
                        recipients.len()
                    );

                    let sign_with = if should_sign {
                        config.default_signing_key.as_deref()
                    } else {
                        None
                    };

                    match self
                        .encrypt_email(&final_content, recipients, sign_with)
                        .await
                    {
                        Ok(encrypted) => {
                            final_content = encrypted;
                            security_status.encryption =
                                super::types::EncryptionStatus::encrypted_and_decrypted(
                                    recipients.to_vec(),
                                    Some("AES256".to_string()), // TODO: Get actual algorithm
                                );

                            if should_sign {
                                security_status.signatures =
                                    super::types::DecryptionStatus::signed(Vec::new());
                            }
                        }
                        Err(e) => {
                            warn!("Failed to encrypt email: {}", e);
                            // Continue without encryption
                        }
                    }
                }
                Ok(_) => {
                    warn!("Not all recipients have encryption keys available");
                }
                Err(e) => {
                    warn!("Failed to find recipient keys: {}", e);
                }
            }
        } else if should_sign {
            debug!("Signing outgoing email");
            match self.sign_email(&final_content, None).await {
                Ok(signed) => {
                    final_content = signed;
                    security_status.signatures = super::types::DecryptionStatus::signed(Vec::new());
                }
                Err(e) => {
                    warn!("Failed to sign email: {}", e);
                    // Continue without signing
                }
            }
        }

        Ok((final_content, security_status))
    }

    /// Import a key from armored text
    pub async fn import_key(&self, key_data: &str) -> EncryptionResult<Vec<String>> {
        info!("Importing GPG key");

        let imported_keys = self.backend.import_key(key_data).await?;

        // Clear cache to force refresh
        self.key_cache.write().await.clear();

        info!("Successfully imported {} key(s)", imported_keys.len());
        Ok(imported_keys)
    }

    /// Export a key
    pub async fn export_key(&self, key_id: &str, include_secret: bool) -> EncryptionResult<String> {
        info!(
            "Exporting key: {} (include_secret: {})",
            key_id, include_secret
        );

        // Validate key exists
        let _key_info = self.get_key_info(key_id).await?;

        self.backend.export_key(key_id, include_secret).await
    }

    /// Generate a new key pair
    pub async fn generate_key(
        &self,
        name: &str,
        email: &str,
        comment: Option<&str>,
    ) -> EncryptionResult<String> {
        info!("Generating new GPG key for: {} <{}>", name, email);

        let key_id = self.backend.generate_key(name, email, comment).await?;

        // Clear cache to pick up the new key
        self.key_cache.write().await.clear();

        info!("Successfully generated new key: {}", key_id);
        Ok(key_id)
    }

    /// Get a summary of available keys
    pub async fn get_key_summary(&self) -> EncryptionResult<KeySummary> {
        let keys = self.list_keys(false).await?;
        let secret_keys = self.list_keys(true).await?;

        let encryption_keys = keys.iter().filter(|k| k.can_encrypt()).count();
        let signing_keys = keys.iter().filter(|k| k.can_sign()).count();
        let expired_keys = keys.iter().filter(|k| k.is_expired).count();
        let revoked_keys = keys.iter().filter(|k| k.is_revoked).count();

        Ok(KeySummary {
            total_keys: keys.len(),
            secret_keys: secret_keys.len(),
            encryption_keys,
            signing_keys,
            expired_keys,
            revoked_keys,
        })
    }

    /// Clear the key cache (useful after key operations)
    pub async fn clear_cache(&self) {
        self.key_cache.write().await.clear();
        debug!("GPG key cache cleared");
    }
}

/// Summary of available keys
#[derive(Debug, Clone)]
pub struct KeySummary {
    pub total_keys: usize,
    pub secret_keys: usize,
    pub encryption_keys: usize,
    pub signing_keys: usize,
    pub expired_keys: usize,
    pub revoked_keys: usize,
}

impl KeySummary {
    /// Get a human-readable description
    pub fn description(&self) -> String {
        format!(
            "{} total keys ({} secret), {} can encrypt, {} can sign, {} expired, {} revoked",
            self.total_keys,
            self.secret_keys,
            self.encryption_keys,
            self.signing_keys,
            self.expired_keys,
            self.revoked_keys
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_encryption_manager_creation() {
        let result = EncryptionManager::new();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_config_operations() {
        let manager = EncryptionManager::new().unwrap();

        let config = manager.get_config().await;
        assert!(!config.always_sign);

        let mut new_config = config;
        new_config.always_sign = true;
        manager.set_config(new_config).await;

        let updated_config = manager.get_config().await;
        assert!(updated_config.always_sign);
    }
}
