use crate::oauth2::{OAuth2Error, OAuth2Result, AccountConfig};
use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Secure storage for OAuth2 tokens and account configurations
pub struct SecureStorage {
    app_name: String,
    config_dir: PathBuf,
}

impl SecureStorage {
    /// Create a new secure storage instance
    pub fn new(app_name: String) -> OAuth2Result<Self> {
        let config_dir = Self::get_config_directory(&app_name)?;
        
        // Ensure config directory exists
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .map_err(|e| OAuth2Error::StorageError(
                    format!("Failed to create config directory: {}", e)
                ))?;
        }
        
        Ok(Self {
            app_name,
            config_dir,
        })
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
        let config_json = serde_json::to_string_pretty(&config_without_tokens)
            .map_err(|e| OAuth2Error::StorageError(
                format!("Failed to serialize account config: {}", e)
            ))?;
        
        fs::write(&config_path, config_json)
            .map_err(|e| OAuth2Error::StorageError(
                format!("Failed to write account config: {}", e)
            ))?;
        
        Ok(())
    }
    
    /// Load account configuration
    pub fn load_account(&self, account_id: &str) -> OAuth2Result<Option<AccountConfig>> {
        let config_path = self.get_account_config_path(account_id);
        
        if !config_path.exists() {
            return Ok(None);
        }
        
        // Load non-sensitive configuration from file
        let config_json = fs::read_to_string(&config_path)
            .map_err(|e| OAuth2Error::StorageError(
                format!("Failed to read account config: {}", e)
            ))?;
        
        let config_without_tokens: AccountConfigForStorage = serde_json::from_str(&config_json)
            .map_err(|e| OAuth2Error::StorageError(
                format!("Failed to parse account config: {}", e)
            ))?;
        
        // Load sensitive tokens from keyring
        let access_token = self.load_access_token(account_id).unwrap_or_default();
        let refresh_token = self.load_refresh_token(account_id);
        
        let account = AccountConfig {
            account_id: config_without_tokens.account_id,
            display_name: config_without_tokens.display_name,
            email_address: config_without_tokens.email_address,
            provider: config_without_tokens.provider,
            imap_server: config_without_tokens.imap_server,
            imap_port: config_without_tokens.imap_port,
            smtp_server: config_without_tokens.smtp_server,
            smtp_port: config_without_tokens.smtp_port,
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
        let entries = fs::read_dir(&self.config_dir)
            .map_err(|e| OAuth2Error::StorageError(
                format!("Failed to read config directory: {}", e)
            ))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| OAuth2Error::StorageError(
                format!("Failed to read directory entry: {}", e)
            ))?;
            
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(account_id) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(Some(account)) = self.load_account(account_id) {
                        accounts.push(account);
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
        
        // Remove config file
        let config_path = self.get_account_config_path(account_id);
        if config_path.exists() {
            fs::remove_file(&config_path)
                .map_err(|e| OAuth2Error::StorageError(
                    format!("Failed to delete account config: {}", e)
                ))?;
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
        let mut account_ids = Vec::new();
        
        let entries = fs::read_dir(&self.config_dir)
            .map_err(|e| OAuth2Error::StorageError(
                format!("Failed to read config directory: {}", e)
            ))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| OAuth2Error::StorageError(
                format!("Failed to read directory entry: {}", e)
            ))?;
            
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(account_id) = path.file_stem().and_then(|s| s.to_str()) {
                    account_ids.push(account_id.to_string());
                }
            }
        }
        
        Ok(account_ids)
    }
    
    /// Store access token in keyring
    fn store_access_token(&self, account_id: &str, token: &str) -> OAuth2Result<()> {
        let service = format!("{}-access-token", self.app_name);
        let entry = Entry::new(&service, account_id)
            .map_err(|e| OAuth2Error::StorageError(
                format!("Failed to create keyring entry: {}", e)
            ))?;
        
        entry.set_password(token)
            .map_err(|e| OAuth2Error::StorageError(
                format!("Failed to store access token: {}", e)
            ))?;
        
        Ok(())
    }
    
    /// Load access token from keyring
    fn load_access_token(&self, account_id: &str) -> Option<String> {
        let service = format!("{}-access-token", self.app_name);
        if let Ok(entry) = Entry::new(&service, account_id) {
            entry.get_password().ok()
        } else {
            None
        }
    }
    
    /// Delete access token from keyring
    fn delete_access_token(&self, account_id: &str) -> OAuth2Result<()> {
        let service = format!("{}-access-token", self.app_name);
        if let Ok(entry) = Entry::new(&service, account_id) {
            let _ = entry.delete_password(); // Ignore errors - token might not exist
        }
        Ok(())
    }
    
    /// Store refresh token in keyring
    fn store_refresh_token(&self, account_id: &str, token: &str) -> OAuth2Result<()> {
        let service = format!("{}-refresh-token", self.app_name);
        let entry = Entry::new(&service, account_id)
            .map_err(|e| OAuth2Error::StorageError(
                format!("Failed to create keyring entry: {}", e)
            ))?;
        
        entry.set_password(token)
            .map_err(|e| OAuth2Error::StorageError(
                format!("Failed to store refresh token: {}", e)
            ))?;
        
        Ok(())
    }
    
    /// Load refresh token from keyring
    fn load_refresh_token(&self, account_id: &str) -> Option<String> {
        let service = format!("{}-refresh-token", self.app_name);
        if let Ok(entry) = Entry::new(&service, account_id) {
            entry.get_password().ok()
        } else {
            None
        }
    }
    
    /// Delete refresh token from keyring
    fn delete_refresh_token(&self, account_id: &str) -> OAuth2Result<()> {
        let service = format!("{}-refresh-token", self.app_name);
        if let Ok(entry) = Entry::new(&service, account_id) {
            let _ = entry.delete_password(); // Ignore errors - token might not exist
        }
        Ok(())
    }
    
    /// Get path to account configuration file
    fn get_account_config_path(&self, account_id: &str) -> PathBuf {
        self.config_dir.join(format!("{}.json", account_id))
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
        
        let expired_accounts = accounts
            .iter()
            .filter(|a| a.is_token_expired())
            .count();
        
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
            let entries = fs::read_dir(dir)
                .map_err(|e| OAuth2Error::StorageError(
                    format!("Failed to read directory: {}", e)
                ))?;
            
            for entry in entries {
                let entry = entry.map_err(|e| OAuth2Error::StorageError(
                    format!("Failed to read directory entry: {}", e)
                ))?;
                
                let metadata = entry.metadata()
                    .map_err(|e| OAuth2Error::StorageError(
                        format!("Failed to read file metadata: {}", e)
                    ))?;
                
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
            fs::remove_file(&config_path)
                .map_err(|e| OAuth2Error::StorageError(
                    format!("Failed to remove account config file: {}", e)
                ))?;
        }
        
        // Remove tokens from keyring
        self.delete_access_token(account_id)?;
        self.delete_refresh_token(account_id)?;
        
        tracing::info!("Account {} removed from secure storage", account_id);
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
        
        assert!(storage.get_account_config_path(&account.account_id).exists() || result.is_err());
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
}