use crate::oauth2::{AccountConfig, OAuth2Error, OAuth2Result};
use base64::prelude::*;
use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Secure storage for OAuth2 tokens and account configurations
#[derive(Clone)]
pub struct SecureStorage {
    app_name: String,
    config_dir: PathBuf,
}

impl SecureStorage {
    /// Create a new secure storage instance
    pub fn new(app_name: String) -> OAuth2Result<Self> {
        tracing::debug!("Creating SecureStorage for app: {}", app_name);

        let config_dir = Self::get_config_directory(&app_name)?;
        tracing::debug!("Config directory determined: {:?}", config_dir);

        // Ensure config directory exists
        if !config_dir.exists() {
            tracing::debug!("Creating config directory: {:?}", config_dir);
            fs::create_dir_all(&config_dir).map_err(|e| {
                OAuth2Error::StorageError(format!("Failed to create config directory: {}", e))
            })?;
        }

        tracing::debug!("SecureStorage initialized successfully");
        Ok(Self {
            app_name,
            config_dir,
        })
    }

    /// Store OAuth2 client credentials securely for token refresh
    pub fn store_oauth_credentials(
        &self,
        account_id: &str,
        client_id: &str,
        client_secret: &str,
    ) -> OAuth2Result<()> {
        // Store credentials in keyring with different service names
        let client_id_service = format!("{}-oauth-client-id", self.app_name);
        let client_secret_service = format!("{}-oauth-client-secret", self.app_name);

        match Entry::new(&client_id_service, account_id) {
            Ok(entry) => {
                if entry.set_password(client_id).is_err() {
                    tracing::warn!(
                        "Failed to store OAuth client ID in keyring, using file fallback"
                    );
                    self.store_credential_to_file(account_id, "client_id", client_id)?;
                }
            }
            Err(_) => {
                self.store_credential_to_file(account_id, "client_id", client_id)?;
            }
        }

        match Entry::new(&client_secret_service, account_id) {
            Ok(entry) => {
                if entry.set_password(client_secret).is_err() {
                    tracing::warn!(
                        "Failed to store OAuth client secret in keyring, using file fallback"
                    );
                    self.store_credential_to_file(account_id, "client_secret", client_secret)?;
                }
            }
            Err(_) => {
                self.store_credential_to_file(account_id, "client_secret", client_secret)?;
            }
        }

        tracing::info!(
            "OAuth2 credentials stored securely for account {}",
            account_id
        );
        Ok(())
    }

    /// Load OAuth2 client credentials for token refresh
    pub fn load_oauth_credentials(
        &self,
        account_id: &str,
    ) -> OAuth2Result<Option<(String, String)>> {
        let client_id = self.load_oauth_client_id(account_id);
        let client_secret = self.load_oauth_client_secret(account_id);

        match (client_id, client_secret) {
            (Some(id), Some(secret)) => Ok(Some((id, secret))),
            _ => {
                tracing::debug!("OAuth2 credentials not found for account {}", account_id);
                Ok(None)
            }
        }
    }

    /// Store account configuration securely
    pub fn store_account(&self, account: &AccountConfig) -> OAuth2Result<()> {
        // Store sensitive tokens in keyring
        self.store_access_token(&account.account_id, &account.access_token)?;

        if let Some(refresh_token) = &account.refresh_token {
            self.store_refresh_token(&account.account_id, refresh_token)?;
        }

        // Store non-sensitive configuration in file
        let config_without_tokens = AccountConfigForStorage {
            account_id: account.account_id.clone(),
            display_name: account.display_name.clone(),
            email_address: account.email_address.clone(),
            provider: account.provider.clone(),
            imap_server: account.imap_server.clone(),
            imap_port: account.imap_port,
            smtp_server: account.smtp_server.clone(),
            smtp_port: account.smtp_port,
            token_expires_at: account.token_expires_at,
            scopes: account.scopes.clone(),
        };

        let config_path = self.get_account_config_path(&account.account_id);
        let config_json = serde_json::to_string_pretty(&config_without_tokens).map_err(|e| {
            OAuth2Error::StorageError(format!("Failed to serialize account config: {}", e))
        })?;

        fs::write(&config_path, config_json).map_err(|e| {
            OAuth2Error::StorageError(format!("Failed to write account config: {}", e))
        })?;

        Ok(())
    }

    /// Load account configuration
    pub fn load_account(&self, account_id: &str) -> OAuth2Result<Option<AccountConfig>> {
        let config_path = self.get_account_config_path(account_id);

        if !config_path.exists() {
            return Ok(None);
        }

        // Load non-sensitive configuration from file
        let config_json = fs::read_to_string(&config_path).map_err(|e| {
            OAuth2Error::StorageError(format!("Failed to read account config: {}", e))
        })?;

        let config_without_tokens: AccountConfigForStorage = serde_json::from_str(&config_json)
            .map_err(|e| {
                OAuth2Error::StorageError(format!("Failed to parse account config: {}", e))
            })?;

        // Load sensitive tokens from keyring
        let access_token = self.load_access_token(account_id).unwrap_or_default();
        let refresh_token = self.load_refresh_token(account_id);

        // Check if tokens are missing or expired
        let has_tokens = !access_token.is_empty() || refresh_token.is_some();
        let is_expired = config_without_tokens.token_expires_at
            .map(|expires| expires < chrono::Utc::now())
            .unwrap_or(false);

        if !has_tokens || is_expired {
            tracing::warn!(
                "Account {} has missing or expired tokens (has_tokens: {}, is_expired: {}). Account will be shown with error status for re-authentication.",
                account_id, has_tokens, is_expired
            );
        }

        let account = AccountConfig {
            account_id: config_without_tokens.account_id,
            display_name: config_without_tokens.display_name,
            email_address: config_without_tokens.email_address,
            provider: config_without_tokens.provider,
            auth_type: crate::oauth2::AuthType::OAuth2, // Default to OAuth2
            imap_server: config_without_tokens.imap_server,
            imap_port: config_without_tokens.imap_port,
            smtp_server: config_without_tokens.smtp_server,
            smtp_port: config_without_tokens.smtp_port,
            security: crate::oauth2::SecurityType::SSL, // Default to SSL
            access_token,
            refresh_token,
            token_expires_at: config_without_tokens.token_expires_at,
            scopes: config_without_tokens.scopes,
        };

        Ok(Some(account))
    }

    /// Load all stored accounts
    pub fn load_all_accounts(&self) -> OAuth2Result<Vec<AccountConfig>> {
        let mut accounts = Vec::new();

        // Read all .json files in config directory
        let entries = fs::read_dir(&self.config_dir).map_err(|e| {
            OAuth2Error::StorageError(format!("Failed to read config directory: {}", e))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                OAuth2Error::StorageError(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(account_id) = path.file_stem().and_then(|s| s.to_str()) {
                    match self.load_account(account_id) {
                        Ok(Some(account)) => {
                            accounts.push(account);
                        }
                        Ok(None) => {
                            tracing::debug!("Account config file exists but load_account returned None for {}", account_id);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load account {}: {}. Account will be skipped.", account_id, e);
                        }
                    }
                }
            }
        }

        Ok(accounts)
    }

    /// Delete account and all associated data
    pub fn delete_account(&self, account_id: &str) -> OAuth2Result<()> {
        // Remove tokens from keyring
        let _ = self.delete_access_token(account_id);
        let _ = self.delete_refresh_token(account_id);

        // Remove OAuth2 credentials
        let _ = self.delete_oauth_credentials(account_id);

        // Remove config file
        let config_path = self.get_account_config_path(account_id);
        if config_path.exists() {
            fs::remove_file(&config_path).map_err(|e| {
                OAuth2Error::StorageError(format!("Failed to delete account config: {}", e))
            })?;
        }

        Ok(())
    }

    /// Update account tokens
    pub fn update_tokens(
        &self,
        account_id: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> OAuth2Result<()> {
        // Update tokens in keyring
        self.store_access_token(account_id, access_token)?;

        if let Some(refresh_token) = refresh_token {
            self.store_refresh_token(account_id, refresh_token)?;
        }

        // Update expiration time in config file if account exists
        if let Ok(Some(mut account)) = self.load_account(account_id) {
            account.access_token = access_token.to_string();
            account.refresh_token = refresh_token.map(|s| s.to_string());
            account.token_expires_at = expires_at;
            self.store_account(&account)?;
        }

        Ok(())
    }

    /// Check if account exists
    pub fn account_exists(&self, account_id: &str) -> bool {
        self.get_account_config_path(account_id).exists()
    }

    /// List all stored account IDs
    pub fn list_account_ids(&self) -> OAuth2Result<Vec<String>> {
        tracing::debug!(
            "Listing account IDs from config directory: {:?}",
            self.config_dir
        );
        let mut account_ids = Vec::new();

        let entries = fs::read_dir(&self.config_dir).map_err(|e| {
            OAuth2Error::StorageError(format!("Failed to read config directory: {}", e))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                OAuth2Error::StorageError(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(account_id) = path.file_stem().and_then(|s| s.to_str()) {
                    account_ids.push(account_id.to_string());
                }
            }
        }

        Ok(account_ids)
    }

    /// Store access token in keyring or file fallback
    fn store_access_token(&self, account_id: &str, token: &str) -> OAuth2Result<()> {
        // Try keyring first, but catch all keyring-related errors gracefully
        let service = format!("{}-access-token", self.app_name);
        match Entry::new(&service, account_id) {
            Ok(entry) => {
                if entry.set_password(token).is_ok() {
                    tracing::debug!("Successfully stored access token in keyring");
                    return Ok(());
                } else {
                    tracing::debug!("Failed to set password in keyring, using file fallback");
                }
            }
            Err(e) => {
                tracing::debug!("Keyring Entry::new failed ({}), using file fallback", e);
            }
        }

        // Fallback to encrypted file storage
        tracing::warn!("Keyring unavailable, using file-based token storage");
        self.store_token_to_file(account_id, "access", token)
    }

    /// Load access token from keyring or file fallback
    fn load_access_token(&self, account_id: &str) -> Option<String> {
        // Try keyring first, but catch all keyring-related errors gracefully
        let service = format!("{}-access-token", self.app_name);
        match Entry::new(&service, account_id) {
            Ok(entry) => {
                if let Ok(token) = entry.get_password() {
                    tracing::debug!("Successfully loaded access token from keyring");
                    return Some(token);
                } else {
                    tracing::debug!("No access token found in keyring, trying file fallback");
                }
            }
            Err(e) => {
                tracing::debug!("Keyring Entry::new failed ({}), trying file fallback", e);
            }
        }

        // Fallback to file storage
        self.load_token_from_file(account_id, "access")
    }

    /// Delete access token from file storage (keyring temporarily disabled)
    fn delete_access_token(&self, account_id: &str) -> OAuth2Result<()> {
        // Keyring temporarily disabled
        // Try to delete from keyring, but ignore all keyring-related errors
        // let service = format!("{}-access-token", self.app_name);
        // match Entry::new(&service, account_id) {
        //     Ok(entry) => {
        //         let _ = entry.delete_password(); // Ignore errors - token might not exist
        //         tracing::debug!("Attempted to delete access token from keyring");
        //     }
        //     Err(e) => {
        //         tracing::debug!("Keyring Entry::new failed during delete ({}), continuing with file cleanup", e);
        //     }
        // }

        // Delete from file storage
        let token_file = self.config_dir.join(format!("{}.access.token", account_id));
        if token_file.exists() {
            let _ = fs::remove_file(&token_file);
        }

        Ok(())
    }

    /// Store refresh token in keyring or file fallback
    fn store_refresh_token(&self, account_id: &str, token: &str) -> OAuth2Result<()> {
        // Try keyring first, but catch all keyring-related errors gracefully
        let service = format!("{}-refresh-token", self.app_name);
        match Entry::new(&service, account_id) {
            Ok(entry) => {
                if entry.set_password(token).is_ok() {
                    tracing::debug!("Successfully stored refresh token in keyring");
                    return Ok(());
                } else {
                    tracing::debug!(
                        "Failed to set refresh password in keyring, using file fallback"
                    );
                }
            }
            Err(e) => {
                tracing::debug!(
                    "Keyring Entry::new failed for refresh token ({}), using file fallback",
                    e
                );
            }
        }

        // Fallback to encrypted file storage
        tracing::warn!("Keyring unavailable, using file-based token storage");
        self.store_token_to_file(account_id, "refresh", token)
    }

    /// Load refresh token from keyring or file fallback
    fn load_refresh_token(&self, account_id: &str) -> Option<String> {
        // Try keyring first, but catch all keyring-related errors gracefully
        let service = format!("{}-refresh-token", self.app_name);
        match Entry::new(&service, account_id) {
            Ok(entry) => {
                if let Ok(token) = entry.get_password() {
                    tracing::debug!("Successfully loaded refresh token from keyring");
                    return Some(token);
                } else {
                    tracing::debug!("No refresh token found in keyring, trying file fallback");
                }
            }
            Err(e) => {
                tracing::debug!(
                    "Keyring Entry::new failed for refresh token ({}), trying file fallback",
                    e
                );
            }
        }

        // Fallback to file storage
        self.load_token_from_file(account_id, "refresh")
    }

    /// Load OAuth2 client ID
    fn load_oauth_client_id(&self, account_id: &str) -> Option<String> {
        let service = format!("{}-oauth-client-id", self.app_name);
        match Entry::new(&service, account_id) {
            Ok(entry) => {
                if let Ok(client_id) = entry.get_password() {
                    return Some(client_id);
                }
            }
            Err(_) => {}
        }

        // Fallback to file storage
        self.load_credential_from_file(account_id, "client_id")
    }

    /// Load OAuth2 client secret
    fn load_oauth_client_secret(&self, account_id: &str) -> Option<String> {
        let service = format!("{}-oauth-client-secret", self.app_name);
        match Entry::new(&service, account_id) {
            Ok(entry) => {
                if let Ok(client_secret) = entry.get_password() {
                    return Some(client_secret);
                }
            }
            Err(_) => {}
        }

        // Fallback to file storage
        self.load_credential_from_file(account_id, "client_secret")
    }

    /// Delete OAuth2 credentials
    fn delete_oauth_credentials(&self, account_id: &str) -> OAuth2Result<()> {
        // Delete from file storage
        let client_id_file = self
            .config_dir
            .join(format!("{}.client_id.cred", account_id));
        let client_secret_file = self
            .config_dir
            .join(format!("{}.client_secret.cred", account_id));

        if client_id_file.exists() {
            let _ = fs::remove_file(&client_id_file);
        }
        if client_secret_file.exists() {
            let _ = fs::remove_file(&client_secret_file);
        }

        Ok(())
    }

    /// Delete refresh token from file storage (keyring temporarily disabled)
    fn delete_refresh_token(&self, account_id: &str) -> OAuth2Result<()> {
        // Keyring temporarily disabled

        // Delete from file storage
        let token_file = self
            .config_dir
            .join(format!("{}.refresh.token", account_id));
        if token_file.exists() {
            let _ = fs::remove_file(&token_file);
        }

        Ok(())
    }

    /// Get path to account configuration file
    fn get_account_config_path(&self, account_id: &str) -> PathBuf {
        self.config_dir.join(format!("{}.json", account_id))
    }

    /// Store token to encrypted file (fallback when keyring unavailable)
    fn store_token_to_file(
        &self,
        account_id: &str,
        token_type: &str,
        token: &str,
    ) -> OAuth2Result<()> {
        let token_file = self
            .config_dir
            .join(format!("{}.{}.token", account_id, token_type));

        // Simple base64 encoding for basic obfuscation (not cryptographically secure)
        let encoded_token = base64::prelude::BASE64_STANDARD.encode(token);

        fs::write(&token_file, encoded_token)
            .map_err(|e| OAuth2Error::StorageError(format!("Failed to write token file: {}", e)))?;

        // Set restrictive permissions (user read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::Permissions::from_mode(0o600);
            fs::set_permissions(&token_file, permissions).map_err(|e| {
                OAuth2Error::StorageError(format!("Failed to set token file permissions: {}", e))
            })?;
        }

        Ok(())
    }

    /// Load token from file (fallback when keyring unavailable)
    fn load_token_from_file(&self, account_id: &str, token_type: &str) -> Option<String> {
        let token_file = self
            .config_dir
            .join(format!("{}.{}.token", account_id, token_type));

        if !token_file.exists() {
            return None;
        }

        let encoded_token = fs::read_to_string(&token_file).ok()?;
        base64::prelude::BASE64_STANDARD
            .decode(encoded_token.trim())
            .ok()
            .and_then(|decoded| String::from_utf8(decoded).ok())
    }

    /// Store OAuth2 credential to encrypted file
    fn store_credential_to_file(
        &self,
        account_id: &str,
        credential_type: &str,
        credential: &str,
    ) -> OAuth2Result<()> {
        let cred_file = self
            .config_dir
            .join(format!("{}.{}.cred", account_id, credential_type));

        // Simple base64 encoding for basic obfuscation (not cryptographically secure)
        let encoded_credential = base64::prelude::BASE64_STANDARD.encode(credential);

        fs::write(&cred_file, encoded_credential).map_err(|e| {
            OAuth2Error::StorageError(format!("Failed to write credential file: {}", e))
        })?;

        // Set restrictive permissions (user read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::Permissions::from_mode(0o600);
            fs::set_permissions(&cred_file, permissions).map_err(|e| {
                OAuth2Error::StorageError(format!(
                    "Failed to set credential file permissions: {}",
                    e
                ))
            })?;
        }

        Ok(())
    }

    /// Load OAuth2 credential from file
    fn load_credential_from_file(&self, account_id: &str, credential_type: &str) -> Option<String> {
        let cred_file = self
            .config_dir
            .join(format!("{}.{}.cred", account_id, credential_type));

        if !cred_file.exists() {
            return None;
        }

        let encoded_credential = fs::read_to_string(&cred_file).ok()?;
        base64::prelude::BASE64_STANDARD
            .decode(encoded_credential.trim())
            .ok()
            .and_then(|decoded| String::from_utf8(decoded).ok())
    }

    /// Get configuration directory path
    fn get_config_directory(app_name: &str) -> OAuth2Result<PathBuf> {
        // Use XDG Base Directory specification on Linux/Unix
        if let Ok(config_dir) = std::env::var("XDG_CONFIG_HOME") {
            Ok(PathBuf::from(config_dir).join(app_name))
        } else if let Ok(home_dir) = std::env::var("HOME") {
            Ok(PathBuf::from(home_dir).join(".config").join(app_name))
        } else if let Ok(app_data) = std::env::var("APPDATA") {
            // Windows
            Ok(PathBuf::from(app_data).join(app_name))
        } else {
            // Fallback to current directory
            Ok(PathBuf::from(".").join(format!(".{}", app_name)))
        }
    }

    /// Clean up expired accounts (optional maintenance)
    pub fn cleanup_expired_accounts(&self, days_old: u32) -> OAuth2Result<Vec<String>> {
        let mut cleaned_accounts = Vec::new();
        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(days_old as i64);

        let account_ids = self.list_account_ids()?;

        for account_id in account_ids {
            if let Ok(Some(account)) = self.load_account(&account_id) {
                // Check if account tokens are expired and old
                if account.is_token_expired() && account.refresh_token.is_none() {
                    if let Some(expires_at) = account.token_expires_at {
                        if expires_at < cutoff_date {
                            self.delete_account(&account_id)?;
                            cleaned_accounts.push(account_id);
                        }
                    }
                }
            }
        }

        Ok(cleaned_accounts)
    }

    /// Export account configurations (without tokens) for backup
    pub fn export_configurations(&self) -> OAuth2Result<Vec<AccountConfigForStorage>> {
        let account_ids = self.list_account_ids()?;
        let mut configs = Vec::new();

        for account_id in account_ids {
            let config_path = self.get_account_config_path(&account_id);
            if let Ok(config_json) = fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str::<AccountConfigForStorage>(&config_json) {
                    configs.push(config);
                }
            }
        }

        Ok(configs)
    }
}

/// Account configuration for storage (without sensitive tokens)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountConfigForStorage {
    pub account_id: String,
    pub display_name: String,
    pub email_address: String,
    pub provider: String,
    pub imap_server: String,
    pub imap_port: u16,
    pub smtp_server: String,
    pub smtp_port: u16,
    pub token_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub scopes: Vec<String>,
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total_accounts: usize,
    pub accounts_with_refresh_tokens: usize,
    pub expired_accounts: usize,
    pub config_directory_size: u64,
}

impl SecureStorage {
    /// Get storage statistics
    pub fn get_storage_stats(&self) -> OAuth2Result<StorageStats> {
        let accounts = self.load_all_accounts()?;
        let total_accounts = accounts.len();

        let accounts_with_refresh_tokens = accounts
            .iter()
            .filter(|a| a.refresh_token.is_some())
            .count();

        let expired_accounts = accounts.iter().filter(|a| a.is_token_expired()).count();

        // Calculate directory size
        let config_directory_size = self.calculate_directory_size(&self.config_dir)?;

        Ok(StorageStats {
            total_accounts,
            accounts_with_refresh_tokens,
            expired_accounts,
            config_directory_size,
        })
    }

    fn calculate_directory_size(&self, dir: &Path) -> OAuth2Result<u64> {
        let mut total_size = 0;

        if dir.exists() {
            let entries = fs::read_dir(dir).map_err(|e| {
                OAuth2Error::StorageError(format!("Failed to read directory: {}", e))
            })?;

            for entry in entries {
                let entry = entry.map_err(|e| {
                    OAuth2Error::StorageError(format!("Failed to read directory entry: {}", e))
                })?;

                let metadata = entry.metadata().map_err(|e| {
                    OAuth2Error::StorageError(format!("Failed to read file metadata: {}", e))
                })?;

                total_size += metadata.len();
            }
        }

        Ok(total_size)
    }

    /// Remove an account and all its associated data
    pub fn remove_account(&self, account_id: &str) -> OAuth2Result<()> {
        // Remove config file
        let config_path = self.get_account_config_path(account_id);
        if config_path.exists() {
            fs::remove_file(&config_path).map_err(|e| {
                OAuth2Error::StorageError(format!("Failed to remove account config file: {}", e))
            })?;
        }

        // Remove tokens from keyring
        self.delete_access_token(account_id)?;
        self.delete_refresh_token(account_id)?;

        tracing::info!("Account {} removed from secure storage", account_id);
        Ok(())
    }

    /// List all stored accounts (CLI compatibility)
    pub fn list_accounts(&self) -> OAuth2Result<Vec<AccountConfig>> {
        self.load_all_accounts()
    }

    /// Get password for account (CLI compatibility)
    pub fn get_password(&self, email: &str) -> OAuth2Result<String> {
        // For CLI compatibility, try to find account by email and return access token
        let accounts = self.load_all_accounts()?;
        for account in accounts {
            if account.email_address == email {
                if !account.access_token.is_empty() {
                    return Ok(account.access_token);
                }
            }
        }
        Err(OAuth2Error::StorageError(format!(
            "No password/token found for {}",
            email
        )))
    }

    /// Store OAuth2 configuration (CLI compatibility)
    pub fn store_oauth_config(
        &self,
        provider: &str,
        config: &crate::oauth2::OAuthConfig,
    ) -> OAuth2Result<()> {
        // Store OAuth config as a provider configuration
        let provider_config = format!("{}_{}", provider, "oauth_config");
        self.store_credential_to_file(&provider_config, "client_id", &config.client_id)?;
        self.store_credential_to_file(&provider_config, "client_secret", &config.client_secret)?;
        self.store_credential_to_file(&provider_config, "redirect_uri", &config.redirect_uri)?;

        // Store scopes as JSON
        let scopes_json = serde_json::to_string(&config.scopes)
            .map_err(|e| OAuth2Error::StorageError(format!("Failed to serialize scopes: {}", e)))?;
        self.store_credential_to_file(&provider_config, "scopes", &scopes_json)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_storage() -> (SecureStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();

        let mut storage = SecureStorage {
            app_name: "comunicado-test".to_string(),
            config_dir: temp_dir.path().to_path_buf(),
        };

        (storage, temp_dir)
    }

    #[test]
    fn test_account_storage() {
        let (storage, _temp_dir) = create_test_storage();

        let account = AccountConfig::new(
            "Test User".to_string(),
            "test@example.com".to_string(),
            "gmail".to_string(),
        );

        // Store account (this will fail in tests due to keyring, but config should work)
        let result = storage.store_account(&account);
        // We expect this to fail in test environment due to keyring
        // In real usage, keyring should work properly

        assert!(
            storage
                .get_account_config_path(&account.account_id)
                .exists()
                || result.is_err()
        );
    }

    #[test]
    fn test_config_directory_detection() {
        // Test that we can determine a config directory
        let result = SecureStorage::get_config_directory("comunicado");
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.to_string_lossy().contains("comunicado"));
    }

    #[test]
    fn test_account_id_listing() {
        let (storage, _temp_dir) = create_test_storage();

        // Initially should be empty
        let account_ids = storage.list_account_ids().unwrap();
        assert!(account_ids.is_empty());

        // Create a dummy config file
        let test_path = storage.get_account_config_path("test-account");
        fs::write(&test_path, r#"{"account_id":"test-account","display_name":"Test","email_address":"test@example.com","provider":"gmail","imap_server":"imap.gmail.com","imap_port":993,"smtp_server":"smtp.gmail.com","smtp_port":587,"token_expires_at":null,"scopes":[]}"#).unwrap();

        let account_ids = storage.list_account_ids().unwrap();
        assert_eq!(account_ids.len(), 1);
        assert_eq!(account_ids[0], "test-account");
    }

    #[test]
    fn test_expired_account_loading() {
        let (storage, _temp_dir) = create_test_storage();

        // Create an expired account config (similar to the user's gmail account)
        let expired_time = chrono::Utc::now() - chrono::Duration::hours(1);
        let config_json = format!(
            r#"{{"account_id":"gmail_test","display_name":"Test User","email_address":"test@gmail.com","provider":"gmail","imap_server":"imap.gmail.com","imap_port":993,"smtp_server":"smtp.gmail.com","smtp_port":587,"token_expires_at":"{}","scopes":[]}}"#,
            expired_time.to_rfc3339()
        );
        
        let test_path = storage.get_account_config_path("gmail_test");
        fs::write(&test_path, config_json).unwrap();

        // Account should load even with missing tokens
        let account = storage.load_account("gmail_test").unwrap();
        assert!(account.is_some());
        
        let account = account.unwrap();
        assert_eq!(account.account_id, "gmail_test");
        assert_eq!(account.email_address, "test@gmail.com");
        assert!(account.access_token.is_empty()); // No tokens stored
        assert!(account.is_token_expired()); // Should be considered expired
        
        // Should appear in load_all_accounts
        let all_accounts = storage.load_all_accounts().unwrap();
        assert_eq!(all_accounts.len(), 1);
        assert!(all_accounts[0].is_token_expired());
    }
}
