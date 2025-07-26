use crate::imap::{ImapClient, ImapConfig, ImapResult, ImapError, ImapAuthMethod};
use crate::oauth2::{TokenManager, SecureStorage, AccountConfig as OAuth2AccountConfig};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};

/// IMAP account information
#[derive(Debug, Clone)]
pub struct ImapAccount {
    pub account_id: String,
    pub display_name: String,
    pub email_address: String,
    pub config: ImapConfig,
    pub is_default: bool,
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
}

impl ImapAccount {
    /// Create a new IMAP account
    pub fn new(
        account_id: String,
        display_name: String,
        email_address: String,
        config: ImapConfig,
    ) -> Self {
        Self {
            account_id,
            display_name,
            email_address,
            config,
            is_default: false,
            last_sync: None,
        }
    }
    
    /// Create account from OAuth2 account config
    pub fn from_oauth2_config(oauth2_config: &OAuth2AccountConfig) -> Self {
        let config = match oauth2_config.provider.as_str() {
            "gmail" => ImapConfig::gmail_oauth2(
                oauth2_config.email_address.clone(),
                oauth2_config.account_id.clone(),
            ),
            "outlook" => ImapConfig::outlook_oauth2(
                oauth2_config.email_address.clone(),
                oauth2_config.account_id.clone(),
            ),
            "yahoo" => ImapConfig::yahoo_oauth2(
                oauth2_config.email_address.clone(),
                oauth2_config.account_id.clone(),
            ),
            _ => ImapConfig::new_oauth2(
                oauth2_config.imap_server.clone(),
                oauth2_config.imap_port,
                oauth2_config.email_address.clone(),
                oauth2_config.account_id.clone(),
            ),
        };
        
        Self::new(
            oauth2_config.account_id.clone(),
            oauth2_config.display_name.clone(),
            oauth2_config.email_address.clone(),
            config,
        )
    }
    
    /// Mark as default account
    pub fn set_default(mut self, is_default: bool) -> Self {
        self.is_default = is_default;
        self
    }
    
    /// Update last sync time
    pub fn update_last_sync(&mut self) {
        self.last_sync = Some(chrono::Utc::now());
    }
}

/// Connection pool for IMAP clients
struct ConnectionPool {
    clients: HashMap<String, Arc<Mutex<ImapClient>>>,
    max_connections: usize,
}

impl ConnectionPool {
    fn new(max_connections: usize) -> Self {
        Self {
            clients: HashMap::new(),
            max_connections,
        }
    }
    
    async fn get_or_create_client(
        &mut self,
        account: &ImapAccount,
        token_manager: Option<&TokenManager>,
    ) -> ImapResult<Arc<Mutex<ImapClient>>> {
        if !self.clients.contains_key(&account.account_id) {
            if self.clients.len() >= self.max_connections {
                // Remove least recently used connection (simplified LRU)
                if let Some((oldest_id, _)) = self.clients.iter().next() {
                    let oldest_id = oldest_id.clone();
                    self.clients.remove(&oldest_id);
                }
            }
            
            let client = match &account.config.auth_method {
                ImapAuthMethod::OAuth2 { .. } => {
                    let token_manager = token_manager
                        .ok_or_else(|| ImapError::authentication("Token manager required for OAuth2"))?;
                    ImapClient::new_with_oauth2(account.config.clone(), token_manager.clone())
                }
                ImapAuthMethod::Password(_) => {
                    ImapClient::new(account.config.clone())
                }
            };
            
            self.clients.insert(account.account_id.clone(), Arc::new(Mutex::new(client)));
        }
        
        Ok(self.clients.get(&account.account_id).unwrap().clone())
    }
    
    fn disconnect_account(&mut self, account_id: &str) {
        self.clients.remove(account_id);
    }
    
    fn disconnect_all(&mut self) {
        self.clients.clear();
    }
}

/// Manager for multiple IMAP accounts
pub struct ImapAccountManager {
    accounts: Arc<RwLock<HashMap<String, ImapAccount>>>,
    connection_pool: Arc<RwLock<ConnectionPool>>,
    token_manager: Option<TokenManager>,
    storage: SecureStorage,
    default_account: Option<String>,
}

impl ImapAccountManager {
    /// Create a new account manager
    pub fn new() -> ImapResult<Self> {
        let storage = SecureStorage::new("comunicado".to_string())
            .map_err(|e| ImapError::storage(&format!("Failed to create secure storage: {}", e)))?;
        
        Ok(Self {
            accounts: Arc::new(RwLock::new(HashMap::new())),
            connection_pool: Arc::new(RwLock::new(ConnectionPool::new(10))), // Max 10 connections
            token_manager: None,
            storage,
            default_account: None,
        })
    }
    
    /// Create account manager with OAuth2 token manager
    pub fn new_with_oauth2(token_manager: TokenManager) -> ImapResult<Self> {
        let mut manager = Self::new()?;
        manager.token_manager = Some(token_manager);
        Ok(manager)
    }
    
    /// Load accounts from storage
    pub async fn load_accounts(&mut self) -> ImapResult<()> {
        // Load OAuth2 accounts
        let oauth2_accounts = self.storage.load_all_accounts()
            .map_err(|e| ImapError::storage(&format!("Failed to load OAuth2 accounts: {}", e)))?;
        
        let mut accounts = self.accounts.write().await;
        
        for oauth2_config in oauth2_accounts {
            let account = ImapAccount::from_oauth2_config(&oauth2_config);
            
            // Set first account as default if none exists
            let is_default = self.default_account.is_none();
            if is_default {
                self.default_account = Some(account.account_id.clone());
            }
            
            accounts.insert(
                account.account_id.clone(),
                account.set_default(is_default),
            );
        }
        
        Ok(())
    }
    
    /// Add a new account
    pub async fn add_account(&mut self, account: ImapAccount) -> ImapResult<()> {
        let mut accounts = self.accounts.write().await;
        
        // Set as default if it's the first account
        let is_default = accounts.is_empty();
        if is_default {
            self.default_account = Some(account.account_id.clone());
        }
        
        accounts.insert(
            account.account_id.clone(),
            account.set_default(is_default),
        );
        
        Ok(())
    }
    
    /// Remove an account
    pub async fn remove_account(&mut self, account_id: &str) -> ImapResult<()> {
        let mut accounts = self.accounts.write().await;
        
        if accounts.remove(account_id).is_some() {
            // Disconnect the account
            let mut pool = self.connection_pool.write().await;
            pool.disconnect_account(account_id);
            
            // Update default account if necessary
            if self.default_account.as_ref() == Some(&account_id.to_string()) {
                self.default_account = accounts.keys().next().cloned();
            }
        }
        
        Ok(())
    }
    
    /// Get account by ID
    pub async fn get_account(&self, account_id: &str) -> Option<ImapAccount> {
        let accounts = self.accounts.read().await;
        accounts.get(account_id).cloned()
    }
    
    /// Get all accounts
    pub async fn get_all_accounts(&self) -> Vec<ImapAccount> {
        let accounts = self.accounts.read().await;
        accounts.values().cloned().collect()
    }
    
    /// Get default account
    pub async fn get_default_account(&self) -> Option<ImapAccount> {
        if let Some(default_id) = &self.default_account {
            self.get_account(default_id).await
        } else {
            None
        }
    }
    
    /// Set default account
    pub async fn set_default_account(&mut self, account_id: &str) -> ImapResult<()> {
        let mut accounts = self.accounts.write().await;
        
        if accounts.contains_key(account_id) {
            // Remove default flag from all accounts
            for account in accounts.values_mut() {
                account.is_default = false;
            }
            
            // Set new default
            if let Some(account) = accounts.get_mut(account_id) {
                account.is_default = true;
                self.default_account = Some(account_id.to_string());
            }
            
            Ok(())
        } else {
            Err(ImapError::not_found(&format!("Account {} not found", account_id)))
        }
    }
    
    /// Get IMAP client for account
    pub async fn get_client(&self, account_id: &str) -> ImapResult<Arc<Mutex<ImapClient>>> {
        println!("DEBUG: get_client() called for account: {}", account_id);
        tracing::debug!("Getting IMAP client for account: '{}'", account_id);
        
        println!("DEBUG: About to acquire accounts.read() lock");
        let accounts = self.accounts.read().await;
        println!("DEBUG: Acquired accounts.read() lock, {} accounts available", accounts.len());
        tracing::debug!("IMAP AccountManager has {} accounts: {:?}", accounts.len(), accounts.keys().collect::<Vec<_>>());
        
        let account = accounts.get(account_id)
            .ok_or_else(|| {
                println!("DEBUG: Account '{}' not found! Available: {:?}", account_id, accounts.keys().collect::<Vec<_>>());
                tracing::error!("Account '{}' not found in IMAP manager. Available accounts: {:?}", account_id, accounts.keys().collect::<Vec<_>>());
                ImapError::not_found(&format!("Account {} not found", account_id))
            })?;
        
        println!("DEBUG: Found account, about to acquire connection_pool.write() lock");
        let mut pool = self.connection_pool.write().await;
        println!("DEBUG: Acquired connection_pool.write() lock");
        println!("DEBUG: About to call pool.get_or_create_client()");
        let client_arc = pool.get_or_create_client(account, self.token_manager.as_ref()).await?;
        println!("DEBUG: Got client_arc from pool, about to ensure connection/auth");
        
        // Ensure client is connected and authenticated
        {
            println!("DEBUG: About to acquire client.lock()");
            let mut client = client_arc.lock().await;
            println!("DEBUG: Acquired client.lock(), checking connection status");
            if !client.is_connected() {
                println!("DEBUG: Client not connected, attempting connection...");
                tracing::info!("Connecting to IMAP server for account: {}", account_id);
                
                // Add timeout to connection attempt
                let connection_result = tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    client.connect()
                ).await;
                
                match connection_result {
                    Ok(Ok(())) => {
                        println!("DEBUG: Connection successful");
                        tracing::info!("Successfully connected to IMAP server for account: {}", account_id);
                    }
                    Ok(Err(e)) => {
                        println!("DEBUG: Connection failed: {}", e);
                        return Err(e);
                    }
                    Err(_) => {
                        println!("DEBUG: Connection timed out after 30 seconds");
                        return Err(ImapError::Timeout);
                    }
                }
            }
            
            if !client.is_authenticated() {
                println!("DEBUG: Client not authenticated, attempting authentication...");
                tracing::info!("Authenticating IMAP connection for account: {}", account_id);
                
                // Add timeout to authentication attempt
                let auth_result = tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    client.authenticate()
                ).await;
                
                match auth_result {
                    Ok(Ok(())) => {
                        println!("DEBUG: Authentication successful");
                        tracing::info!("Successfully authenticated IMAP connection for account: {}", account_id);
                    }
                    Ok(Err(e)) => {
                        println!("DEBUG: Authentication failed: {}", e);
                        tracing::error!("IMAP authentication failed for account {}: {}", account_id, e);
                        return Err(e);
                    }
                    Err(_) => {
                        println!("DEBUG: Authentication timed out after 30 seconds");
                        return Err(ImapError::Timeout);
                    }
                }
            }
        }
        
        Ok(client_arc)
    }
    
    /// Test connection for an account
    pub async fn test_connection(&self, account_id: &str) -> ImapResult<bool> {
        match self.get_client(account_id).await {
            Ok(client_arc) => {
                // Try to get capabilities to test the connection
                let mut client = client_arc.lock().await;
                match client.get_capabilities().await {
                    Ok(_) => Ok(true),
                    Err(_) => Ok(false),
                }
            }
            Err(_) => Ok(false),
        }
    }
    
    /// Sync all accounts
    pub async fn sync_all_accounts(&mut self) -> ImapResult<Vec<(String, Result<(), ImapError>)>> {
        let account_ids: Vec<String> = {
            let accounts = self.accounts.read().await;
            accounts.keys().cloned().collect()
        };
        
        let mut results = Vec::new();
        
        for account_id in account_ids {
            let result = self.sync_account(&account_id).await;
            results.push((account_id, result));
        }
        
        Ok(results)
    }
    
    /// Sync a specific account
    pub async fn sync_account(&mut self, account_id: &str) -> ImapResult<()> {
        // Get client and perform basic sync operations
        let client_arc = self.get_client(account_id).await?;
        
        // List folders to verify connection
        {
            let mut client = client_arc.lock().await;
            let _folders = client.list_folders("", "*").await?;
        }
        
        // Update last sync time
        {
            let mut accounts = self.accounts.write().await;
            if let Some(account) = accounts.get_mut(account_id) {
                account.update_last_sync();
            }
        }
        
        Ok(())
    }
    
    /// Disconnect all accounts
    pub async fn disconnect_all(&mut self) -> ImapResult<()> {
        let mut pool = self.connection_pool.write().await;
        pool.disconnect_all();
        Ok(())
    }
    
    /// Get account statistics
    pub async fn get_statistics(&self) -> AccountManagerStats {
        let accounts = self.accounts.read().await;
        let total_accounts = accounts.len();
        let oauth2_accounts = accounts.values()
            .filter(|a| matches!(a.config.auth_method, ImapAuthMethod::OAuth2 { .. }))
            .count();
        let password_accounts = total_accounts - oauth2_accounts;
        let connected_accounts = 0; // TODO: Track connected accounts
        
        AccountManagerStats {
            total_accounts,
            oauth2_accounts,
            password_accounts,
            connected_accounts,
            default_account: self.default_account.clone(),
        }
    }
}

impl Default for ImapAccountManager {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

/// Account manager statistics
#[derive(Debug, Clone)]
pub struct AccountManagerStats {
    pub total_accounts: usize,
    pub oauth2_accounts: usize,
    pub password_accounts: usize,
    pub connected_accounts: usize,
    pub default_account: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_account_manager_creation() {
        let manager = ImapAccountManager::new().unwrap();
        let stats = manager.get_statistics().await;
        
        assert_eq!(stats.total_accounts, 0);
        assert_eq!(stats.oauth2_accounts, 0);
        assert_eq!(stats.password_accounts, 0);
    }
    
    #[tokio::test]
    async fn test_add_account() {
        let mut manager = ImapAccountManager::new().unwrap();
        
        let config = ImapConfig::gmail("test@gmail.com".to_string(), "password".to_string());
        let account = ImapAccount::new(
            "test-account".to_string(),
            "Test User".to_string(),
            "test@gmail.com".to_string(),
            config,
        );
        
        manager.add_account(account).await.unwrap();
        
        let stats = manager.get_statistics().await;
        assert_eq!(stats.total_accounts, 1);
        assert_eq!(stats.password_accounts, 1);
        
        let retrieved = manager.get_account("test-account").await;
        assert!(retrieved.is_some());
        assert!(retrieved.unwrap().is_default);
    }
    
    #[tokio::test]
    async fn test_default_account_management() {
        let mut manager = ImapAccountManager::new().unwrap();
        
        // Add first account - should be default
        let config1 = ImapConfig::gmail("test1@gmail.com".to_string(), "password".to_string());
        let account1 = ImapAccount::new(
            "account1".to_string(),
            "User 1".to_string(),
            "test1@gmail.com".to_string(),
            config1,
        );
        manager.add_account(account1).await.unwrap();
        
        // Add second account - should not be default
        let config2 = ImapConfig::outlook("test2@outlook.com".to_string(), "password".to_string());
        let account2 = ImapAccount::new(
            "account2".to_string(),
            "User 2".to_string(),
            "test2@outlook.com".to_string(),
            config2,
        );
        manager.add_account(account2).await.unwrap();
        
        // First account should be default
        let default = manager.get_default_account().await.unwrap();
        assert_eq!(default.account_id, "account1");
        
        // Change default
        manager.set_default_account("account2").await.unwrap();
        let default = manager.get_default_account().await.unwrap();
        assert_eq!(default.account_id, "account2");
    }
    
    #[test]
    fn test_imap_account_from_oauth2() {
        let oauth2_config = OAuth2AccountConfig::new(
            "Test User".to_string(),
            "test@gmail.com".to_string(),
            "gmail".to_string(),
        );
        
        let account = ImapAccount::from_oauth2_config(&oauth2_config);
        
        assert_eq!(account.email_address, "test@gmail.com");
        assert_eq!(account.display_name, "Test User");
        assert!(matches!(account.config.auth_method, ImapAuthMethod::OAuth2 { .. }));
    }
}