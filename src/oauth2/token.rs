use crate::oauth2::{OAuth2Error, OAuth2Result, TokenResponse};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Access token with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessToken {
    pub token: String,
    pub token_type: String,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub scopes: Vec<String>,
}

impl AccessToken {
    pub fn new(token: String, token_type: String) -> Self {
        Self {
            token,
            token_type,
            expires_at: None,
            scopes: Vec::new(),
        }
    }
    
    pub fn from_response(response: &TokenResponse) -> Self {
        let expires_at = response.expires_in.map(|seconds| {
            chrono::Utc::now() + chrono::Duration::seconds(seconds as i64)
        });
        
        let scopes = response.scope
            .as_ref()
            .map(|s| s.split_whitespace().map(|scope| scope.to_string()).collect())
            .unwrap_or_default();
        
        Self {
            token: response.access_token.clone(),
            token_type: response.token_type.clone(),
            expires_at,
            scopes,
        }
    }
    
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            chrono::Utc::now() > expires_at
        } else {
            false
        }
    }
    
    pub fn needs_refresh(&self, buffer_minutes: i64) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = chrono::Utc::now();
            let buffer = chrono::Duration::minutes(buffer_minutes);
            now + buffer > expires_at
        } else {
            false
        }
    }
    
    pub fn authorization_header(&self) -> String {
        format!("{} {}", self.token_type, self.token)
    }
}

/// Refresh token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshToken {
    pub token: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl RefreshToken {
    pub fn new(token: String) -> Self {
        Self {
            token,
            created_at: chrono::Utc::now(),
        }
    }
}

/// Token manager for handling OAuth2 tokens
#[derive(Clone)]
pub struct TokenManager {
    tokens: Arc<RwLock<HashMap<String, TokenPair>>>,
    storage: Option<Arc<crate::oauth2::SecureStorage>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenPair {
    access_token: AccessToken,
    refresh_token: Option<RefreshToken>,
    account_id: String,
    provider: String,
}

impl TokenManager {
    /// Create a new token manager
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
            storage: None,
        }
    }
    
    /// Create a new token manager with storage backend
    pub fn new_with_storage(storage: Arc<crate::oauth2::SecureStorage>) -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
            storage: Some(storage),
        }
    }
    
    /// Store tokens for an account
    pub async fn store_tokens(
        &self,
        account_id: String,
        provider: String,
        token_response: &TokenResponse,
    ) -> OAuth2Result<()> {
        let access_token = AccessToken::from_response(token_response);
        let refresh_token = token_response.refresh_token
            .as_ref()
            .map(|token| RefreshToken::new(token.clone()));
        
        let token_pair = TokenPair {
            access_token,
            refresh_token,
            account_id: account_id.clone(),
            provider,
        };
        
        let mut tokens = self.tokens.write().await;
        tokens.insert(account_id, token_pair);
        
        Ok(())
    }
    
    /// Get access token for an account
    pub async fn get_access_token(&self, account_id: &str) -> OAuth2Result<Option<AccessToken>> {
        let tokens = self.tokens.read().await;
        if let Some(token_pair) = tokens.get(account_id) {
            Ok(Some(token_pair.access_token.clone()))
        } else {
            Ok(None)
        }
    }
    
    /// Get a valid access token, refreshing if necessary
    pub async fn get_valid_access_token(&self, account_id: &str) -> OAuth2Result<Option<AccessToken>> {
        let (needs_refresh, is_expired) = {
            let tokens = self.tokens.read().await;
            if let Some(token_pair) = tokens.get(account_id) {
                (
                    token_pair.access_token.needs_refresh(5), // 5 minute buffer
                    token_pair.access_token.is_expired()
                )
            } else {
                return Ok(None);
            }
        };
        
        if needs_refresh || is_expired {
            tracing::info!("Token for account {} needs refresh (expired: {}, needs_refresh: {})", 
                         account_id, is_expired, needs_refresh);
            
            match self.refresh_access_token(account_id).await {
                Ok(refreshed_token) => {
                    tracing::info!("Successfully refreshed token for account {}", account_id);
                    return Ok(Some(refreshed_token));
                }
                Err(e) => {
                    tracing::warn!("Failed to refresh token for account {}: {}", account_id, e);
                    // If refresh fails and token is expired, return None to indicate re-auth needed
                    if is_expired {
                        return Ok(None);
                    }
                    // If not expired but refresh failed, still try to use existing token
                }
            }
        }
        
        self.get_access_token(account_id).await
    }
    
    /// Refresh access token using refresh token
    pub async fn refresh_access_token(&self, account_id: &str) -> OAuth2Result<AccessToken> {
        let (refresh_token, provider) = {
            let tokens = self.tokens.read().await;
            let token_pair = tokens.get(account_id)
                .ok_or_else(|| OAuth2Error::InvalidToken(
                    format!("No tokens found for account {}", account_id)
                ))?;
            
            let refresh_token = token_pair.refresh_token.as_ref()
                .ok_or_else(|| OAuth2Error::InvalidToken(
                    format!("No refresh token available for account {}", account_id)
                ))?
                .token
                .clone();
                
            (refresh_token, token_pair.provider.clone())
        };
        
        // Try to refresh the token using the OAuth2 client
        match self.refresh_token_with_provider(&refresh_token, &provider).await {
            Ok(new_token_response) => {
                // Update stored tokens with new response
                self.store_tokens(account_id.to_string(), provider.clone(), &new_token_response).await?;
                
                // Update persistent storage if available
                if let Some(ref storage) = self.storage {
                    storage.update_tokens(
                        account_id,
                        &new_token_response.access_token,
                        new_token_response.refresh_token.as_deref(),
                        new_token_response.expires_in.map(|seconds| {
                            chrono::Utc::now() + chrono::Duration::seconds(seconds as i64)
                        }),
                    ).map_err(|e| OAuth2Error::StorageError(format!("Failed to update tokens in storage: {}", e)))?;
                }
                
                // Return the new access token
                let new_access_token = AccessToken::from_response(&new_token_response);
                tracing::info!("Successfully refreshed token for account {}", account_id);
                Ok(new_access_token)
            }
            Err(e) => {
                tracing::error!("Failed to refresh token for account {}: {}", account_id, e);
                Err(OAuth2Error::TokenRefreshFailed(
                    format!("Token refresh failed for account {}: {}. Please re-authenticate using the OAuth2 setup wizard", account_id, e)
                ))
            }
        }
    }
    
    /// Refresh token using the appropriate OAuth2 provider
    async fn refresh_token_with_provider(&self, refresh_token: &str, provider: &str) -> OAuth2Result<TokenResponse> {
        use crate::oauth2::{ProviderConfig, OAuth2Client};
        
        // First, get the account ID for this refresh token to load stored credentials
        let account_id = {
            let tokens = self.tokens.read().await;
            tokens.iter()
                .find(|(_, pair)| {
                    pair.refresh_token.as_ref()
                        .map(|rt| rt.token == refresh_token)
                        .unwrap_or(false)
                })
                .map(|(id, _)| id.clone())
        };
        
        let account_id = account_id.ok_or_else(|| {
            OAuth2Error::InvalidToken("Cannot find account for refresh token".to_string())
        })?;
        
        // Load stored OAuth2 credentials for this account
        let (client_id, client_secret) = if let Some(ref storage) = self.storage {
            match storage.load_oauth_credentials(&account_id)? {
                Some((id, secret)) => (id, secret),
                None => {
                    // Fallback to environment variables
                    tracing::warn!("No stored OAuth2 credentials found for account {}, trying environment variables", account_id);
                    match (std::env::var("GMAIL_CLIENT_ID"), std::env::var("GMAIL_CLIENT_SECRET")) {
                        (Ok(id), Ok(secret)) => (id, secret),
                        _ => {
                            return Err(OAuth2Error::InvalidToken(
                                format!("No OAuth2 credentials available for token refresh. Account {} needs re-authentication.", account_id)
                            ));
                        }
                    }
                }
            }
        } else {
            // No storage backend, try environment variables
            match (std::env::var("GMAIL_CLIENT_ID"), std::env::var("GMAIL_CLIENT_SECRET")) {
                (Ok(id), Ok(secret)) => (id, secret),
                _ => {
                    return Err(OAuth2Error::InvalidToken(
                        "No OAuth2 credentials available for token refresh".to_string()
                    ));
                }
            }
        };
        
        // Create provider configuration with stored credentials
        let config = match provider {
            "gmail" => {
                let mut config = ProviderConfig::gmail();
                config.client_id = client_id;
                config.client_secret = Some(client_secret);
                config
            }
            _ => {
                return Err(OAuth2Error::InvalidProvider(
                    format!("Token refresh not supported for provider: {}", provider)
                ));
            }
        };
        
        // Create OAuth2 client and refresh token
        let client = OAuth2Client::new(config)?;
        tracing::info!("Refreshing token for account {} using stored credentials", account_id);
        client.refresh_token(refresh_token).await
    }
    
    /// Remove tokens for an account
    pub async fn remove_tokens(&self, account_id: &str) -> OAuth2Result<()> {
        let mut tokens = self.tokens.write().await;
        tokens.remove(account_id);
        Ok(())
    }
    
    /// Check if account has valid tokens
    pub async fn has_valid_tokens(&self, account_id: &str) -> bool {
        let tokens = self.tokens.read().await;
        if let Some(token_pair) = tokens.get(account_id) {
            !token_pair.access_token.is_expired() || token_pair.refresh_token.is_some()
        } else {
            false
        }
    }
    
    /// Get all account IDs with tokens
    pub async fn get_account_ids(&self) -> Vec<String> {
        let tokens = self.tokens.read().await;
        tokens.keys().cloned().collect()
    }
    
    
    /// Create XOAUTH2 SASL string for IMAP authentication
    pub async fn create_xoauth2_string(&self, account_id: &str, username: &str) -> OAuth2Result<String> {
        println!("DEBUG: create_xoauth2_string called for account: {}, username: {}", account_id, username);
        tracing::debug!("Creating XOAUTH2 string for account '{}' and username '{}'", account_id, username);
        
        // First check if we have any tokens at all
        let account_ids = self.get_account_ids().await;
        println!("DEBUG: TokenManager has tokens for {} accounts: {:?}", account_ids.len(), account_ids);
        tracing::debug!("TokenManager has tokens for {} accounts: {:?}", account_ids.len(), account_ids);
        
        let access_token = self.get_valid_access_token(account_id).await?
            .ok_or_else(|| {
                println!("DEBUG: No valid access token found for account: {}", account_id);
                tracing::error!("No valid access token found for account '{}'. Available accounts: {:?}", account_id, account_ids);
                OAuth2Error::InvalidToken(
                    format!("No valid access token for account {}", account_id)
                )
            })?;
        
        println!("DEBUG: Got access token, length: {}, expires_at: {:?}", access_token.token.len(), access_token.expires_at);
        tracing::debug!("Found access token for account '{}': token type '{}', expires at {:?}", 
                       account_id, access_token.token_type, access_token.expires_at);
        
        // XOAUTH2 format: user=username\x01auth=Bearer token\x01\x01
        let auth_string = format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            username,
            access_token.token
        );
        
        println!("DEBUG: Auth string created, length: {}", auth_string.len());
        
        // Base64 encode the auth string
        use base64::{Engine as _, engine::general_purpose};
        let encoded = general_purpose::STANDARD.encode(auth_string);
        println!("DEBUG: Base64 encoded XOAUTH2 string, length: {}", encoded.len());
        tracing::debug!("Generated XOAUTH2 string length: {} characters", encoded.len());
        Ok(encoded)
    }
    
    /// Validate token format and expiration
    pub fn validate_token(&self, token: &AccessToken) -> OAuth2Result<()> {
        if token.token.is_empty() {
            return Err(OAuth2Error::InvalidToken("Empty access token".to_string()));
        }
        
        if token.is_expired() {
            return Err(OAuth2Error::InvalidToken("Access token has expired".to_string()));
        }
        
        Ok(())
    }
    
    /// Get token statistics
    pub async fn get_token_stats(&self) -> TokenStats {
        let tokens = self.tokens.read().await;
        let total_accounts = tokens.len();
        let mut expired_tokens = 0;
        let mut expiring_soon = 0;
        let mut valid_tokens = 0;
        
        for token_pair in tokens.values() {
            if token_pair.access_token.is_expired() {
                expired_tokens += 1;
            } else if token_pair.access_token.needs_refresh(60) { // 1 hour buffer
                expiring_soon += 1;
            } else {
                valid_tokens += 1;
            }
        }
        
        TokenStats {
            total_accounts,
            valid_tokens,
            expired_tokens,
            expiring_soon,
        }
    }
    
    /// Diagnose token issues for a specific account
    pub async fn diagnose_account_tokens(&self, account_id: &str) -> TokenDiagnosis {
        let tokens = self.tokens.read().await;
        
        if let Some(token_pair) = tokens.get(account_id) {
            let access_token = &token_pair.access_token;
            let has_refresh_token = token_pair.refresh_token.is_some();
            
            if access_token.is_expired() {
                if has_refresh_token {
                    TokenDiagnosis::ExpiredWithRefresh {
                        account_id: account_id.to_string(),
                        expired_at: access_token.expires_at,
                        can_refresh: true,
                    }
                } else {
                    TokenDiagnosis::ExpiredNoRefresh {
                        account_id: account_id.to_string(),
                        expired_at: access_token.expires_at,
                    }
                }
            } else if access_token.needs_refresh(5) {
                TokenDiagnosis::ExpiringSoon {
                    account_id: account_id.to_string(),
                    expires_at: access_token.expires_at,
                    has_refresh_token,
                }
            } else {
                TokenDiagnosis::Valid {
                    account_id: account_id.to_string(),
                    expires_at: access_token.expires_at,
                }
            }
        } else {
            TokenDiagnosis::NotFound {
                account_id: account_id.to_string(),
            }
        }
    }
    
    /// Get diagnosis for all accounts
    pub async fn diagnose_all_accounts(&self) -> Vec<TokenDiagnosis> {
        let account_ids = self.get_account_ids().await;
        let mut diagnoses = Vec::new();
        
        for account_id in account_ids {
            diagnoses.push(self.diagnose_account_tokens(&account_id).await);
        }
        
        diagnoses
    }
}

impl Default for TokenManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Token statistics
#[derive(Debug, Clone)]
pub struct TokenStats {
    pub total_accounts: usize,
    pub valid_tokens: usize,
    pub expired_tokens: usize,
    pub expiring_soon: usize,
}

/// Token diagnosis for an account
#[derive(Debug, Clone)]
pub enum TokenDiagnosis {
    Valid {
        account_id: String,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    },
    ExpiringSoon {
        account_id: String,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
        has_refresh_token: bool,
    },
    ExpiredWithRefresh {
        account_id: String,
        expired_at: Option<chrono::DateTime<chrono::Utc>>,
        can_refresh: bool,
    },
    ExpiredNoRefresh {
        account_id: String,
        expired_at: Option<chrono::DateTime<chrono::Utc>>,
    },
    NotFound {
        account_id: String,
    },
}

impl TokenDiagnosis {
    /// Get a user-friendly description of the diagnosis
    pub fn description(&self) -> String {
        match self {
            TokenDiagnosis::Valid { account_id, expires_at } => {
                if let Some(expires) = expires_at {
                    format!("Account '{}' has a valid token that expires at {}", account_id, expires.format("%Y-%m-%d %H:%M:%S UTC"))
                } else {
                    format!("Account '{}' has a valid token with no expiration", account_id)
                }
            },
            TokenDiagnosis::ExpiringSoon { account_id, expires_at, has_refresh_token } => {
                let expires_str = expires_at
                    .map(|e| e.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|| "unknown time".to_string());
                
                if *has_refresh_token {
                    format!("Account '{}' token expires soon at {} (can be refreshed)", account_id, expires_str)
                } else {
                    format!("Account '{}' token expires soon at {} (requires re-authentication)", account_id, expires_str)
                }
            },
            TokenDiagnosis::ExpiredWithRefresh { account_id, expired_at, .. } => {
                let expired_str = expired_at
                    .map(|e| e.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|| "unknown time".to_string());
                
                format!("Account '{}' token expired at {} (can be refreshed)", account_id, expired_str)
            },
            TokenDiagnosis::ExpiredNoRefresh { account_id, expired_at } => {
                let expired_str = expired_at
                    .map(|e| e.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|| "unknown time".to_string());
                
                format!("Account '{}' token expired at {} (requires re-authentication)", account_id, expired_str)
            },
            TokenDiagnosis::NotFound { account_id } => {
                format!("No tokens found for account '{}'", account_id)
            },
        }
    }
    
    /// Get recommended action for this diagnosis
    pub fn recommended_action(&self) -> String {
        match self {
            TokenDiagnosis::Valid { .. } => {
                "No action needed - token is valid".to_string()
            },
            TokenDiagnosis::ExpiringSoon { has_refresh_token, .. } => {
                if *has_refresh_token {
                    "Token will be automatically refreshed when needed".to_string()
                } else {
                    "Consider re-authenticating before the token expires".to_string()
                }
            },
            TokenDiagnosis::ExpiredWithRefresh { account_id, .. } => {
                format!("Run automatic token refresh or re-authenticate account '{}'", account_id)
            },
            TokenDiagnosis::ExpiredNoRefresh { account_id, .. } |
            TokenDiagnosis::NotFound { account_id } => {
                format!("Re-authenticate account '{}' using the OAuth2 setup wizard", account_id)
            },
        }
    }
    
    /// Check if this diagnosis indicates a problem that needs user action
    pub fn needs_action(&self) -> bool {
        matches!(self, 
            TokenDiagnosis::ExpiredWithRefresh { .. } |
            TokenDiagnosis::ExpiredNoRefresh { .. } |
            TokenDiagnosis::NotFound { .. }
        )
    }
}

/// Token refresh scheduler for automatic token refresh
pub struct TokenRefreshScheduler {
    token_manager: Arc<TokenManager>,
    refresh_interval: chrono::Duration,
}

impl TokenRefreshScheduler {
    pub fn new(token_manager: Arc<TokenManager>) -> Self {
        Self {
            token_manager,
            refresh_interval: chrono::Duration::hours(1), // Check every hour
        }
    }
    
    /// Start the token refresh scheduler
    pub async fn start(&self) -> OAuth2Result<()> {
        let token_manager = Arc::clone(&self.token_manager);
        let interval = self.refresh_interval;
        
        tokio::spawn(async move {
            let mut refresh_interval = tokio::time::interval(
                tokio::time::Duration::from_secs(interval.num_seconds() as u64)
            );
            
            loop {
                refresh_interval.tick().await;
                
                if let Err(e) = Self::refresh_expiring_tokens(&token_manager).await {
                    tracing::warn!("Failed to refresh expiring tokens: {}", e);
                }
            }
        });
        
        Ok(())
    }
    
    async fn refresh_expiring_tokens(token_manager: &TokenManager) -> OAuth2Result<()> {
        let account_ids = token_manager.get_account_ids().await;
        
        for account_id in account_ids {
            // Check if token needs refresh (30 minute buffer)
            let needs_refresh = {
                let tokens = token_manager.tokens.read().await;
                tokens.get(&account_id)
                    .map(|pair| pair.access_token.needs_refresh(30))
                    .unwrap_or(false)
            };
            
            if needs_refresh {
                match token_manager.refresh_access_token(&account_id).await {
                    Ok(_) => {
                        tracing::info!("Successfully refreshed token for account {}", account_id);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to refresh token for account {}: {}", account_id, e);
                    }
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_access_token_creation() {
        let token_response = TokenResponse {
            access_token: "test-token".to_string(),
            refresh_token: Some("refresh-token".to_string()),
            token_type: "Bearer".to_string(),
            expires_in: Some(3600),
            scope: Some("read write".to_string()),
        };
        
        let access_token = AccessToken::from_response(&token_response);
        
        assert_eq!(access_token.token, "test-token");
        assert_eq!(access_token.token_type, "Bearer");
        assert!(!access_token.is_expired());
        assert_eq!(access_token.scopes, vec!["read", "write"]);
    }
    
    #[test]
    fn test_access_token_expiration() {
        let mut access_token = AccessToken::new("token".to_string(), "Bearer".to_string());
        
        // Token without expiration should not be expired
        assert!(!access_token.is_expired());
        assert!(!access_token.needs_refresh(5));
        
        // Set expiration in the past
        access_token.expires_at = Some(chrono::Utc::now() - chrono::Duration::hours(1));
        assert!(access_token.is_expired());
        assert!(access_token.needs_refresh(5));
        
        // Set expiration in the near future
        access_token.expires_at = Some(chrono::Utc::now() + chrono::Duration::minutes(2));
        assert!(!access_token.is_expired());
        assert!(access_token.needs_refresh(5)); // Within 5 minute buffer
    }
    
    #[test]
    fn test_authorization_header() {
        let access_token = AccessToken::new("test-token".to_string(), "Bearer".to_string());
        assert_eq!(access_token.authorization_header(), "Bearer test-token");
    }
    
    #[tokio::test]
    async fn test_token_manager_storage() {
        let token_manager = TokenManager::new();
        
        let token_response = TokenResponse {
            access_token: "test-token".to_string(),
            refresh_token: Some("refresh-token".to_string()),
            token_type: "Bearer".to_string(),
            expires_in: Some(3600),
            scope: Some("read".to_string()),
        };
        
        token_manager.store_tokens(
            "test-account".to_string(),
            "gmail".to_string(),
            &token_response,
        ).await.unwrap();
        
        let access_token = token_manager.get_access_token("test-account").await.unwrap();
        assert!(access_token.is_some());
        assert_eq!(access_token.unwrap().token, "test-token");
        
        assert!(token_manager.has_valid_tokens("test-account").await);
        assert!(!token_manager.has_valid_tokens("nonexistent").await);
    }
    
    #[tokio::test]
    async fn test_token_manager_stats() {
        let token_manager = TokenManager::new();
        
        // Add a valid token
        let valid_token = TokenResponse {
            access_token: "valid-token".to_string(),
            refresh_token: None,
            token_type: "Bearer".to_string(),
            expires_in: Some(3600),
            scope: None,
        };
        
        token_manager.store_tokens(
            "valid-account".to_string(),
            "gmail".to_string(),
            &valid_token,
        ).await.unwrap();
        
        let stats = token_manager.get_token_stats().await;
        assert_eq!(stats.total_accounts, 1);
        assert_eq!(stats.valid_tokens, 1);
        assert_eq!(stats.expired_tokens, 0);
    }
}