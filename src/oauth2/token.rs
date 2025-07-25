use crate::oauth2::{OAuth2Error, OAuth2Result, TokenResponse, AccountConfig};
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
        let needs_refresh = {
            let tokens = self.tokens.read().await;
            if let Some(token_pair) = tokens.get(account_id) {
                token_pair.access_token.needs_refresh(5) // 5 minute buffer
            } else {
                return Ok(None);
            }
        };
        
        if needs_refresh {
            self.refresh_access_token(account_id).await?;
        }
        
        self.get_access_token(account_id).await
    }
    
    /// Refresh access token using refresh token
    /// Note: This is a simplified version that doesn't actually refresh tokens
    /// In a full implementation, this would need access to OAuth2 clients
    pub async fn refresh_access_token(&self, account_id: &str) -> OAuth2Result<AccessToken> {
        let tokens = self.tokens.read().await;
        let token_pair = tokens.get(account_id)
            .ok_or_else(|| OAuth2Error::InvalidToken(
                format!("No tokens found for account {}", account_id)
            ))?;
        
        // For now, just return the existing token
        // In a full implementation, this would refresh via the OAuth2 client
        Ok(token_pair.access_token.clone())
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
        let access_token = self.get_valid_access_token(account_id).await?
            .ok_or_else(|| OAuth2Error::InvalidToken(
                format!("No valid access token for account {}", account_id)
            ))?;
        
        // XOAUTH2 format: user=username\x01auth=Bearer token\x01\x01
        let auth_string = format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            username,
            access_token.token
        );
        
        // Base64 encode the auth string
        use base64::{Engine as _, engine::general_purpose};
        Ok(general_purpose::STANDARD.encode(auth_string))
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