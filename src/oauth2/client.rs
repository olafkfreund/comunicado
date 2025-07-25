use crate::oauth2::{
    OAuth2Error, OAuth2Result, ProviderConfig, TokenResponse, 
    AuthorizationCode, PkceChallenge, AccountConfig
};
use reqwest::Client as HttpClient;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};
use url::Url;

/// OAuth2 client for handling authentication flows
pub struct OAuth2Client {
    config: ProviderConfig,
    http_client: HttpClient,
    callback_server: Option<Arc<Mutex<CallbackServer>>>,
}

impl OAuth2Client {
    /// Create a new OAuth2 client
    pub fn new(config: ProviderConfig) -> OAuth2Result<Self> {
        config.validate()?;
        
        let http_client = HttpClient::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| OAuth2Error::NetworkError(e))?;
        
        Ok(Self {
            config,
            http_client,
            callback_server: None,
        })
    }
    
    /// Start the OAuth2 authorization flow
    pub async fn start_authorization(&mut self) -> OAuth2Result<AuthorizationRequest> {
        // Generate PKCE challenge if supported
        let pkce_challenge = if self.config.uses_pkce {
            Some(PkceChallenge::new())
        } else {
            None
        };
        
        // Generate state parameter for CSRF protection
        let state = generate_random_string(32);
        
        // Build authorization URL
        let auth_url = self.build_authorization_url(&state, &pkce_challenge)?;
        
        // Start callback server
        let callback_server = CallbackServer::new(8080)?;
        self.callback_server = Some(Arc::new(Mutex::new(callback_server)));
        
        Ok(AuthorizationRequest {
            authorization_url: auth_url,
            state: state.clone(),
            code_verifier: pkce_challenge.map(|p| p.code_verifier),
        })
    }
    
    /// Wait for authorization callback
    pub async fn wait_for_authorization(&mut self, timeout_secs: u64) -> OAuth2Result<AuthorizationCode> {
        let callback_server = self.callback_server
            .as_ref()
            .ok_or_else(|| OAuth2Error::InvalidConfig("Authorization not started".to_string()))?;
        
        let server = callback_server.lock().await;
        let result = timeout(
            Duration::from_secs(timeout_secs),
            server.wait_for_callback()
        ).await;
        
        match result {
            Ok(Ok(callback)) => Ok(callback),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(OAuth2Error::AuthorizationTimeout),
        }
    }
    
    /// Exchange authorization code for tokens
    pub async fn exchange_code(&self, auth_code: &AuthorizationCode) -> OAuth2Result<TokenResponse> {
        let mut params = HashMap::new();
        params.insert("grant_type", "authorization_code");
        params.insert("code", &auth_code.code);
        params.insert("redirect_uri", &self.config.redirect_uri);
        params.insert("client_id", &self.config.client_id);
        
        // Add client secret if required
        if let Some(client_secret) = &self.config.client_secret {
            params.insert("client_secret", client_secret);
        }
        
        // Add PKCE code verifier if used
        if self.config.uses_pkce {
            params.insert("code_verifier", &auth_code.code_verifier);
        }
        
        let response = self.http_client
            .post(&self.config.token_url)
            .form(&params)
            .send()
            .await
            .map_err(OAuth2Error::NetworkError)?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(OAuth2Error::TokenExchangeFailed(error_text));
        }
        
        let token_data: Value = response.json().await
            .map_err(OAuth2Error::NetworkError)?;
        
        self.parse_token_response(token_data)
    }
    
    /// Refresh access token using refresh token
    pub async fn refresh_token(&self, refresh_token: &str) -> OAuth2Result<TokenResponse> {
        if !self.config.supports_refresh {
            return Err(OAuth2Error::TokenRefreshFailed(
                "Provider does not support token refresh".to_string()
            ));
        }
        
        let mut params = HashMap::new();
        params.insert("grant_type", "refresh_token");
        params.insert("refresh_token", refresh_token);
        params.insert("client_id", &self.config.client_id);
        
        if let Some(client_secret) = &self.config.client_secret {
            params.insert("client_secret", client_secret);
        }
        
        let response = self.http_client
            .post(&self.config.token_url)
            .form(&params)
            .send()
            .await
            .map_err(OAuth2Error::NetworkError)?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(OAuth2Error::TokenRefreshFailed(error_text));
        }
        
        let token_data: Value = response.json().await
            .map_err(OAuth2Error::NetworkError)?;
        
        self.parse_token_response(token_data)
    }
    
    /// Get user info using access token (for email address detection)
    pub async fn get_user_info(&self, access_token: &str) -> OAuth2Result<UserInfo> {
        let user_info_url = match self.config.provider {
            crate::oauth2::OAuth2Provider::Gmail => "https://www.googleapis.com/oauth2/v2/userinfo",
            crate::oauth2::OAuth2Provider::Outlook => "https://graph.microsoft.com/v1.0/me",
            crate::oauth2::OAuth2Provider::Yahoo => "https://api.login.yahoo.com/openid/v1/userinfo",
            crate::oauth2::OAuth2Provider::Custom(_) => {
                return Err(OAuth2Error::InvalidConfig(
                    "User info endpoint not configured for custom provider".to_string()
                ));
            }
        };
        
        let response = self.http_client
            .get(user_info_url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(OAuth2Error::NetworkError)?;
        
        if !response.status().is_success() {
            return Err(OAuth2Error::NetworkError(
                reqwest::Error::from(response.error_for_status().unwrap_err())
            ));
        }
        
        let user_data: Value = response.json().await
            .map_err(OAuth2Error::NetworkError)?;
        
        self.parse_user_info(user_data)
    }
    
    /// Create complete account configuration
    pub async fn create_account_config(
        &self,
        token_response: &TokenResponse,
        display_name: Option<String>,
    ) -> OAuth2Result<AccountConfig> {
        // Get user info to determine email address
        let user_info = self.get_user_info(&token_response.access_token).await?;
        
        let mut account_config = AccountConfig::new(
            display_name.unwrap_or_else(|| user_info.name.clone()),
            user_info.email,
            self.config.provider.as_str().to_string(),
        );
        
        // Set server configuration
        account_config.imap_server = self.config.imap_server.clone();
        account_config.imap_port = self.config.imap_port;
        account_config.smtp_server = self.config.smtp_server.clone();
        account_config.smtp_port = self.config.smtp_port;
        
        // Update with token information
        account_config.update_tokens(token_response);
        
        Ok(account_config)
    }
    
    /// Build authorization URL with all required parameters
    fn build_authorization_url(
        &self,
        state: &str,
        pkce_challenge: &Option<PkceChallenge>,
    ) -> OAuth2Result<String> {
        let mut url = Url::parse(&self.config.authorization_url)?;
        
        {
            let mut query_pairs = url.query_pairs_mut();
            query_pairs.append_pair("response_type", "code");
            query_pairs.append_pair("client_id", &self.config.client_id);
            query_pairs.append_pair("redirect_uri", &self.config.redirect_uri);
            query_pairs.append_pair("scope", &self.config.scope_string());
            query_pairs.append_pair("state", state);
            
            // Add PKCE parameters if used
            if let Some(pkce) = pkce_challenge {
                query_pairs.append_pair("code_challenge", &pkce.code_challenge);
                query_pairs.append_pair("code_challenge_method", &pkce.code_challenge_method);
            }
            
            // Add additional provider-specific parameters
            for (key, value) in &self.config.additional_params {
                query_pairs.append_pair(key, value);
            }
        }
        
        Ok(url.to_string())
    }
    
    /// Parse token response from different providers
    fn parse_token_response(&self, data: Value) -> OAuth2Result<TokenResponse> {
        let access_token = data["access_token"]
            .as_str()
            .ok_or_else(|| OAuth2Error::InvalidToken("Missing access_token".to_string()))?
            .to_string();
        
        let refresh_token = data["refresh_token"]
            .as_str()
            .map(|s| s.to_string());
        
        let token_type = data["token_type"]
            .as_str()
            .unwrap_or("Bearer")
            .to_string();
        
        let expires_in = data["expires_in"]
            .as_u64();
        
        let scope = data["scope"]
            .as_str()
            .map(|s| s.to_string());
        
        Ok(TokenResponse {
            access_token,
            refresh_token,
            token_type,
            expires_in,
            scope,
        })
    }
    
    /// Parse user info from different providers
    fn parse_user_info(&self, data: Value) -> OAuth2Result<UserInfo> {
        let (email, name) = match self.config.provider {
            crate::oauth2::OAuth2Provider::Gmail => {
                let email = data["email"]
                    .as_str()
                    .ok_or_else(|| OAuth2Error::InvalidToken("Missing email in user info".to_string()))?
                    .to_string();
                let name = data["name"]
                    .as_str()
                    .unwrap_or(&email)
                    .to_string();
                (email, name)
            },
            crate::oauth2::OAuth2Provider::Outlook => {
                let email = data["mail"]
                    .as_str()
                    .or_else(|| data["userPrincipalName"].as_str())
                    .ok_or_else(|| OAuth2Error::InvalidToken("Missing email in user info".to_string()))?
                    .to_string();
                let name = data["displayName"]
                    .as_str()
                    .unwrap_or(&email)
                    .to_string();
                (email, name)
            },
            crate::oauth2::OAuth2Provider::Yahoo => {
                let email = data["email"]
                    .as_str()
                    .ok_or_else(|| OAuth2Error::InvalidToken("Missing email in user info".to_string()))?
                    .to_string();
                let name = data["name"]
                    .as_str()
                    .unwrap_or(&email)
                    .to_string();
                (email, name)
            },
            crate::oauth2::OAuth2Provider::Custom(_) => {
                return Err(OAuth2Error::InvalidConfig(
                    "User info parsing not configured for custom provider".to_string()
                ));
            }
        };
        
        Ok(UserInfo { email, name })
    }
    
    /// Get provider configuration
    pub fn config(&self) -> &ProviderConfig {
        &self.config
    }
}

/// Authorization request information
#[derive(Debug)]
pub struct AuthorizationRequest {
    pub authorization_url: String,
    pub state: String,
    pub code_verifier: Option<String>,
}

/// User information from OAuth2 provider
#[derive(Debug, Clone)]
pub struct UserInfo {
    pub email: String,
    pub name: String,
}

/// Simple HTTP server for OAuth2 callback
struct CallbackServer {
    port: u16,
}

impl CallbackServer {
    fn new(port: u16) -> OAuth2Result<Self> {
        Ok(Self { port })
    }
    
    async fn wait_for_callback(&self) -> OAuth2Result<AuthorizationCode> {
        // This is a simplified implementation
        // In production, you'd use a proper HTTP server like warp or axum
        
        use std::io::Read;
        use std::net::TcpListener;
        
        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.port))
            .map_err(|e| OAuth2Error::StorageError(format!("Failed to bind to port {}: {}", self.port, e)))?;
        
        // Wait for incoming connection
        let (mut stream, _) = listener.accept()
            .map_err(|e| OAuth2Error::AuthorizationFailed(format!("Failed to accept connection: {}", e)))?;
        
        // Read HTTP request
        let mut buffer = [0; 4096];
        let bytes_read = stream.read(&mut buffer)
            .map_err(|e| OAuth2Error::AuthorizationFailed(format!("Failed to read request: {}", e)))?;
        
        let request = String::from_utf8_lossy(&buffer[..bytes_read]);
        
        // Send response
        let response = "HTTP/1.1 200 OK\r\n\r\n<html><body><h1>Authorization successful!</h1><p>You can close this window and return to Comunicado.</p></body></html>";
        std::io::Write::write_all(&mut stream, response.as_bytes())
            .map_err(|e| OAuth2Error::AuthorizationFailed(format!("Failed to send response: {}", e)))?;
        
        // Parse callback URL from request
        self.parse_callback_from_request(&request)
    }
    
    fn parse_callback_from_request(&self, request: &str) -> OAuth2Result<AuthorizationCode> {
        // Extract the first line (GET request)
        let first_line = request.lines().next()
            .ok_or_else(|| OAuth2Error::AuthorizationFailed("Invalid HTTP request".to_string()))?;
        
        // Extract URL from "GET /oauth/callback?code=...&state=... HTTP/1.1"
        let url_part = first_line.split_whitespace().nth(1)
            .ok_or_else(|| OAuth2Error::AuthorizationFailed("Invalid HTTP request format".to_string()))?;
        
        // Parse URL
        let full_url = format!("http://localhost{}", url_part);
        let url = Url::parse(&full_url)
            .map_err(|e| OAuth2Error::AuthorizationFailed(format!("Failed to parse callback URL: {}", e)))?;
        
        // Extract parameters
        let mut code = None;
        let mut state = None;
        
        for (key, value) in url.query_pairs() {
            match key.as_ref() {
                "code" => code = Some(value.to_string()),
                "state" => state = Some(value.to_string()),
                "error" => {
                    return Err(OAuth2Error::AuthorizationFailed(
                        format!("OAuth2 error: {}", value)
                    ));
                }
                _ => {}
            }
        }
        
        let code = code.ok_or_else(|| {
            OAuth2Error::AuthorizationFailed("Missing authorization code".to_string())
        })?;
        
        let state = state.ok_or_else(|| {
            OAuth2Error::AuthorizationFailed("Missing state parameter".to_string())
        })?;
        
        Ok(AuthorizationCode {
            code,
            state,
            code_verifier: String::new(), // Will be filled by the client
        })
    }
}

/// Generate a random string for state parameter
fn generate_random_string(length: usize) -> String {
    use rand::Rng;
    
    let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    (0..length)
        .map(|_| {
            let idx = rand::thread_rng().gen_range(0..chars.len());
            chars[idx] as char
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oauth2::ProviderConfig;
    
    #[test]
    fn test_authorization_url_building() {
        let config = ProviderConfig::gmail().with_credentials(
            "test-client-id".to_string(),
            Some("test-client-secret".to_string())
        );
        
        let client = OAuth2Client::new(config).unwrap();
        let pkce = Some(PkceChallenge::new());
        let url = client.build_authorization_url("test-state", &pkce).unwrap();
        
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=test-client-id"));
        assert!(url.contains("state=test-state"));
        assert!(url.contains("code_challenge="));
    }
    
    #[test]
    fn test_pkce_challenge_generation() {
        let pkce = PkceChallenge::new();
        
        assert_eq!(pkce.code_verifier.len(), 128);
        assert!(!pkce.code_challenge.is_empty());
        assert_eq!(pkce.code_challenge_method, "S256");
    }
    
    #[test]
    fn test_random_string_generation() {
        let s1 = generate_random_string(32);
        let s2 = generate_random_string(32);
        
        assert_eq!(s1.len(), 32);
        assert_eq!(s2.len(), 32);
        assert_ne!(s1, s2); // Very unlikely to be the same
    }
}