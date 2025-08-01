pub mod client;
pub mod providers;
pub mod storage;
pub mod token;
pub mod wizard;

pub use client::OAuth2Client;
pub use providers::{OAuth2Provider, ProviderConfig, ProviderDetector};
pub use storage::SecureStorage;
pub use token::{
    AccessToken, RefreshStats, RefreshToken, TokenDiagnosis, TokenManager, TokenRefreshScheduler,
    TokenStats,
};
pub use wizard::SetupWizard;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// OAuth2 authentication errors
#[derive(Error, Debug)]
pub enum OAuth2Error {
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Authorization failed: {0}")]
    AuthorizationFailed(String),

    #[error("Token exchange failed: {0}")]
    TokenExchangeFailed(String),

    #[error("Token refresh failed: {0}")]
    TokenRefreshFailed(String),

    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("URL parse error: {0}")]
    UrlParseError(#[from] url::ParseError),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("User cancelled authorization")]
    UserCancelled,

    #[error("Timeout waiting for authorization")]
    AuthorizationTimeout,

    #[error("Invalid provider: {0}")]
    InvalidProvider(String),
}

pub type OAuth2Result<T> = Result<T, OAuth2Error>;

/// OAuth2 authorization code with PKCE
#[derive(Debug, Clone)]
pub struct AuthorizationCode {
    pub code: String,
    pub state: String,
    pub code_verifier: String,
}

/// OAuth2 token response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub expires_in: Option<u64>,
    pub scope: Option<String>,
}

/// OAuth2 scope definitions
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OAuth2Scope {
    // Gmail scopes
    GmailReadonly,
    GmailModify,
    GmailFull,

    // Google profile scopes
    OpenId,
    Profile,
    Email,

    // Google Contacts scope
    GoogleContacts,
    
    // Google Calendar scope
    GoogleCalendar,

    // Outlook scopes
    OutlookMailRead,
    OutlookMailReadWrite,
    OutlookMailSend,
    OutlookOfflineAccess,

    // Outlook Contacts scope
    OutlookContacts,

    // Yahoo scopes
    YahooMailRead,
    YahooMailWrite,

    // Custom scope
    Custom(String),
}

impl OAuth2Scope {
    pub fn as_str(&self) -> &str {
        match self {
            OAuth2Scope::GmailReadonly => "https://www.googleapis.com/auth/gmail.readonly",
            OAuth2Scope::GmailModify => "https://www.googleapis.com/auth/gmail.modify",
            OAuth2Scope::GmailFull => "https://mail.google.com/",
            OAuth2Scope::OpenId => "openid",
            OAuth2Scope::Profile => "profile",
            OAuth2Scope::Email => "email",
            OAuth2Scope::GoogleContacts => "https://www.googleapis.com/auth/contacts.readonly",
            OAuth2Scope::GoogleCalendar => "https://www.googleapis.com/auth/calendar",
            OAuth2Scope::OutlookMailRead => "https://graph.microsoft.com/Mail.Read",
            OAuth2Scope::OutlookMailReadWrite => "https://graph.microsoft.com/Mail.ReadWrite",
            OAuth2Scope::OutlookMailSend => "https://graph.microsoft.com/Mail.Send",
            OAuth2Scope::OutlookOfflineAccess => "offline_access",
            OAuth2Scope::OutlookContacts => "https://graph.microsoft.com/Contacts.Read",
            OAuth2Scope::YahooMailRead => "mail-r",
            OAuth2Scope::YahooMailWrite => "mail-w",
            OAuth2Scope::Custom(s) => s,
        }
    }

    pub fn to_string(&self) -> String {
        self.as_str().to_string()
    }
}

/// PKCE (Proof Key for Code Exchange) implementation
pub struct PkceChallenge {
    pub code_verifier: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
}

impl PkceChallenge {
    /// Generate a new PKCE challenge
    pub fn new() -> Self {
        use rand::Rng;
        use sha2::{Digest, Sha256};

        // Generate random code verifier (43-128 characters)
        let code_verifier: String = (0..128)
            .map(|_| {
                let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
                chars[rand::thread_rng().gen_range(0..chars.len())] as char
            })
            .collect();

        // Generate code challenge (SHA256 hash of verifier, base64url encoded)
        let mut hasher = Sha256::new();
        hasher.update(code_verifier.as_bytes());
        let challenge_bytes = hasher.finalize();

        use base64::{engine::general_purpose, Engine as _};
        let code_challenge = general_purpose::URL_SAFE_NO_PAD.encode(challenge_bytes);

        Self {
            code_verifier,
            code_challenge,
            code_challenge_method: "S256".to_string(),
        }
    }
}

impl Default for PkceChallenge {
    fn default() -> Self {
        Self::new()
    }
}

/// OAuth2 client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

/// Authentication type for accounts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthType {
    OAuth2,
    Password,
}

/// Security type for connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityType {
    None,
    StartTLS,
    SSL,
}

/// Account configuration after OAuth2 setup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountConfig {
    pub account_id: String,
    pub display_name: String,
    pub email_address: String,
    pub provider: String,
    pub auth_type: AuthType,
    pub imap_server: String,
    pub imap_port: u16,
    pub smtp_server: String,
    pub smtp_port: u16,
    pub security: SecurityType,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub scopes: Vec<String>,
}

impl AccountConfig {
    pub fn new(display_name: String, email_address: String, provider: String) -> Self {
        let account_id = format!(
            "{}_{}",
            provider.to_lowercase(),
            email_address.replace('@', "_").replace('.', "_")
        );

        Self {
            account_id,
            display_name,
            email_address,
            provider,
            auth_type: AuthType::OAuth2,
            imap_server: String::new(),
            imap_port: 993,
            smtp_server: String::new(),
            smtp_port: 587,
            security: SecurityType::SSL,
            access_token: String::new(),
            refresh_token: None,
            token_expires_at: None,
            scopes: Vec::new(),
        }
    }

    pub fn is_token_expired(&self) -> bool {
        // Consider expired if access token is missing or empty
        if self.access_token.is_empty() {
            return true;
        }
        
        // Check expiration time
        if let Some(expires_at) = self.token_expires_at {
            chrono::Utc::now() > expires_at
        } else {
            false
        }
    }

    pub fn needs_refresh(&self) -> bool {
        // Consider token expired if it expires within 5 minutes
        if let Some(expires_at) = self.token_expires_at {
            let now = chrono::Utc::now();
            let buffer = chrono::Duration::minutes(5);
            now + buffer > expires_at
        } else {
            false
        }
    }

    pub fn update_tokens(&mut self, token_response: &TokenResponse) {
        self.access_token = token_response.access_token.clone();

        if let Some(refresh_token) = &token_response.refresh_token {
            self.refresh_token = Some(refresh_token.clone());
        }

        if let Some(expires_in) = token_response.expires_in {
            self.token_expires_at =
                Some(chrono::Utc::now() + chrono::Duration::seconds(expires_in as i64));
        }

        if let Some(scope) = &token_response.scope {
            self.scopes = scope.split_whitespace().map(|s| s.to_string()).collect();
        }
    }
}
