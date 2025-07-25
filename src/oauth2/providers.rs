use crate::oauth2::{OAuth2Scope, OAuth2Error, OAuth2Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported OAuth2 providers
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OAuth2Provider {
    Gmail,
    Outlook,
    Yahoo,
    Custom(String),
}

impl OAuth2Provider {
    pub fn as_str(&self) -> &str {
        match self {
            OAuth2Provider::Gmail => "gmail",
            OAuth2Provider::Outlook => "outlook",
            OAuth2Provider::Yahoo => "yahoo",
            OAuth2Provider::Custom(name) => name,
        }
    }
    
    pub fn display_name(&self) -> &str {
        match self {
            OAuth2Provider::Gmail => "Gmail",
            OAuth2Provider::Outlook => "Outlook/Hotmail",
            OAuth2Provider::Yahoo => "Yahoo Mail",
            OAuth2Provider::Custom(name) => name,
        }
    }
    
    pub fn from_str(s: &str) -> OAuth2Result<Self> {
        match s.to_lowercase().as_str() {
            "gmail" => Ok(OAuth2Provider::Gmail),
            "outlook" => Ok(OAuth2Provider::Outlook),
            "yahoo" => Ok(OAuth2Provider::Yahoo),
            _ => Err(OAuth2Error::InvalidProvider(s.to_string())),
        }
    }
}

/// OAuth2 provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider: OAuth2Provider,
    pub client_id: String,
    pub client_secret: Option<String>, // Some providers don't require client secret for PKCE
    pub authorization_url: String,
    pub token_url: String,
    pub redirect_uri: String,
    pub scopes: Vec<OAuth2Scope>,
    pub additional_params: HashMap<String, String>,
    pub imap_server: String,
    pub imap_port: u16,
    pub smtp_server: String,
    pub smtp_port: u16,
    pub supports_refresh: bool,
    pub uses_pkce: bool,
}

impl ProviderConfig {
    /// Create Gmail configuration
    /// Note: In production, these would come from app registration
    pub fn gmail() -> Self {
        Self {
            provider: OAuth2Provider::Gmail,
            client_id: "your-gmail-client-id.apps.googleusercontent.com".to_string(),
            client_secret: Some("your-gmail-client-secret".to_string()),
            authorization_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            token_url: "https://oauth2.googleapis.com/token".to_string(),
            redirect_uri: "http://localhost:8080/oauth/callback".to_string(), // Standard desktop app redirect
            scopes: vec![
                OAuth2Scope::GmailModify, // Less restrictive than GmailFull for development
                OAuth2Scope::GmailReadonly, // Ensures we can read emails
            ],
            additional_params: {
                let mut params = HashMap::new();
                params.insert("access_type".to_string(), "offline".to_string());
                params.insert("prompt".to_string(), "consent".to_string());
                params
            },
            imap_server: "imap.gmail.com".to_string(),
            imap_port: 993,
            smtp_server: "smtp.gmail.com".to_string(),
            smtp_port: 587,
            supports_refresh: true,
            uses_pkce: true,
        }
    }
    
    /// Create Outlook/Hotmail configuration
    pub fn outlook() -> Self {
        Self {
            provider: OAuth2Provider::Outlook,
            client_id: "your-outlook-client-id".to_string(),
            client_secret: None, // Outlook supports PKCE without client secret
            authorization_url: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize".to_string(),
            token_url: "https://login.microsoftonline.com/common/oauth2/v2.0/token".to_string(),
            redirect_uri: "http://localhost:8080/oauth/callback".to_string(),
            scopes: vec![
                OAuth2Scope::OutlookMailReadWrite,
                OAuth2Scope::OutlookMailSend,
                OAuth2Scope::OutlookOfflineAccess,
            ],
            additional_params: HashMap::new(),
            imap_server: "outlook.office365.com".to_string(),
            imap_port: 993,
            smtp_server: "smtp-mail.outlook.com".to_string(),
            smtp_port: 587,
            supports_refresh: true,
            uses_pkce: true,
        }
    }
    
    /// Create Yahoo configuration
    pub fn yahoo() -> Self {
        Self {
            provider: OAuth2Provider::Yahoo,
            client_id: "your-yahoo-client-id".to_string(),
            client_secret: Some("your-yahoo-client-secret".to_string()),
            authorization_url: "https://api.login.yahoo.com/oauth2/request_auth".to_string(),
            token_url: "https://api.login.yahoo.com/oauth2/get_token".to_string(),
            redirect_uri: "http://localhost:8080/oauth/callback".to_string(),
            scopes: vec![
                OAuth2Scope::YahooMailRead,
                OAuth2Scope::YahooMailWrite,
            ],
            additional_params: HashMap::new(),
            imap_server: "imap.mail.yahoo.com".to_string(),
            imap_port: 993,
            smtp_server: "smtp.mail.yahoo.com".to_string(),
            smtp_port: 587,
            supports_refresh: true,
            uses_pkce: false, // Yahoo doesn't support PKCE
        }
    }
    
    /// Create custom provider configuration
    pub fn custom(
        name: String,
        client_id: String,
        client_secret: Option<String>,
        authorization_url: String,
        token_url: String,
        imap_server: String,
        imap_port: u16,
        smtp_server: String,
        smtp_port: u16,
    ) -> Self {
        Self {
            provider: OAuth2Provider::Custom(name),
            client_id,
            client_secret,
            authorization_url,
            token_url,
            redirect_uri: "http://localhost:8080/oauth/callback".to_string(),
            scopes: Vec::new(),
            additional_params: HashMap::new(),
            imap_server,
            imap_port,
            smtp_server,
            smtp_port,
            supports_refresh: true,
            uses_pkce: true,
        }
    }
    
    /// Get all supported providers
    pub fn supported_providers() -> Vec<OAuth2Provider> {
        vec![
            OAuth2Provider::Gmail,
            OAuth2Provider::Outlook,
            OAuth2Provider::Yahoo,
        ]
    }
    
    /// Get provider configuration by provider type
    pub fn get_config(provider: &OAuth2Provider) -> OAuth2Result<Self> {
        match provider {
            OAuth2Provider::Gmail => Ok(Self::gmail()),
            OAuth2Provider::Outlook => Ok(Self::outlook()),
            OAuth2Provider::Yahoo => Ok(Self::yahoo()),
            OAuth2Provider::Custom(_) => Err(OAuth2Error::InvalidConfig(
                "Custom provider configuration must be provided".to_string()
            )),
        }
    }
    
    /// Update client credentials (for production use)
    pub fn with_credentials(mut self, client_id: String, client_secret: Option<String>) -> Self {
        self.client_id = client_id;
        self.client_secret = client_secret;
        self
    }
    
    /// Update redirect URI
    pub fn with_redirect_uri(mut self, redirect_uri: String) -> Self {
        self.redirect_uri = redirect_uri;
        self
    }
    
    /// Add custom scopes
    pub fn with_scopes(mut self, scopes: Vec<OAuth2Scope>) -> Self {
        self.scopes = scopes;
        self
    }
    
    /// Add additional authorization parameters
    pub fn with_additional_params(mut self, params: HashMap<String, String>) -> Self {
        self.additional_params.extend(params);
        self
    }
    
    /// Get scope strings for authorization request
    pub fn scope_string(&self) -> String {
        self.scopes
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
    
    /// Validate configuration
    pub fn validate(&self) -> OAuth2Result<()> {
        if self.client_id.is_empty() {
            return Err(OAuth2Error::InvalidConfig("Client ID is required".to_string()));
        }
        
        if !self.uses_pkce && self.client_secret.is_none() {
            return Err(OAuth2Error::InvalidConfig(
                "Client secret is required when PKCE is not used".to_string()
            ));
        }
        
        if self.authorization_url.is_empty() {
            return Err(OAuth2Error::InvalidConfig("Authorization URL is required".to_string()));
        }
        
        if self.token_url.is_empty() {
            return Err(OAuth2Error::InvalidConfig("Token URL is required".to_string()));
        }
        
        if self.redirect_uri.is_empty() {
            return Err(OAuth2Error::InvalidConfig("Redirect URI is required".to_string()));
        }
        
        if self.imap_server.is_empty() {
            return Err(OAuth2Error::InvalidConfig("IMAP server is required".to_string()));
        }
        
        if self.smtp_server.is_empty() {
            return Err(OAuth2Error::InvalidConfig("SMTP server is required".to_string()));
        }
        
        Ok(())
    }
    
    /// Get provider-specific setup instructions
    pub fn setup_instructions(&self) -> Vec<String> {
        match &self.provider {
            OAuth2Provider::Gmail => vec![
                "To set up Gmail OAuth2 access:".to_string(),
                "".to_string(),
                "STEP 1: Configure OAuth Consent Screen".to_string(),
                "1. Go to Google Cloud Console (console.cloud.google.com)".to_string(),
                "2. Create a new project or select an existing one".to_string(),
                "3. Go to 'APIs & Services' > 'OAuth consent screen'".to_string(),
                "4. Choose 'External' user type (unless using Google Workspace)".to_string(),
                "5. Fill in required fields:".to_string(),
                "   - App name: Comunicado Email Client".to_string(),
                "   - User support email: your email".to_string(),
                "   - Developer contact: your email".to_string(),
                "6. Add scopes: Add 'https://www.googleapis.com/auth/gmail.readonly'".to_string(),
                "   and 'https://www.googleapis.com/auth/gmail.send'".to_string(),
                "7. Add test users: Add your Gmail address to test users list".to_string(),
                "8. Save and continue through all steps".to_string(),
                "".to_string(),
                "STEP 2: Enable Gmail API".to_string(),
                "9. Go to 'APIs & Services' > 'Library'".to_string(),
                "10. Search for 'Gmail API' and click 'Enable'".to_string(),
                "".to_string(),
                "STEP 3: Create OAuth Client".to_string(),
                "11. Go to 'APIs & Services' > 'Credentials'".to_string(),
                "12. Click '+ CREATE CREDENTIALS' > 'OAuth client ID'".to_string(),
                "13. Select 'Desktop application' as the application type".to_string(),
                "14. Give your OAuth client a name (e.g., 'Comunicado Desktop')".to_string(),
                "15. Optional: Add authorized redirect URIs:".to_string(),
                "    - http://localhost:8080/oauth/callback".to_string(),
                "    - http://localhost:8081/oauth/callback".to_string(),
                "    - http://localhost:8082/oauth/callback".to_string(),
                "    (This helps avoid port conflicts)".to_string(),
                "16. Click 'CREATE'".to_string(),
                "17. Copy the 'Client ID' from the popup".to_string(),
                "18. Copy the 'Client Secret' (required for desktop apps)".to_string(),
                "".to_string(),
                "IMPORTANT NOTES:".to_string(),
                "• Your app will be in 'Testing' mode initially - only test users can access it".to_string(),
                "• Desktop applications automatically use localhost redirects".to_string(),
                "• The app must be verified by Google for production use with external users".to_string(),
                "• While testing, you may see security warnings - this is normal".to_string(),
                "".to_string(),
                "TROUBLESHOOTING:".to_string(),
                "• Error 400 'invalid_request': Check OAuth consent screen configuration".to_string(),
                "• 'App not verified': Add your email to test users list".to_string(),
                "• 'Access blocked': Ensure Gmail API is enabled".to_string(),
                "• 'redirect_uri_mismatch' with Desktop app:".to_string(),
                "  - Try changing to 'Web application' type instead".to_string(),
                "  - Add redirect URIs: http://localhost:8080/oauth/callback".to_string(),
                "  - Google sometimes requires explicit redirect URI registration".to_string(),
                "".to_string(),
                "For production use, submit your app for Google verification.".to_string(),
            ],
            OAuth2Provider::Outlook => vec![
                "To set up Outlook OAuth2 access:".to_string(),
                "".to_string(),
                "1. Go to Azure Portal (portal.azure.com)".to_string(),
                "2. Navigate to 'Azure Active Directory' > 'App registrations'".to_string(),
                "3. Click 'New registration'".to_string(),
                "4. Enter a name for your application (e.g., 'Comunicado')".to_string(),
                "5. Select 'Accounts in any organizational directory and personal Microsoft accounts'".to_string(),
                "6. Under 'Redirect URI', select 'Web' and enter:".to_string(),
                "   http://localhost:8080/oauth/callback".to_string(),
                "   (Note: Comunicado will use ports 8080-8089 automatically)".to_string(),
                "7. Click 'Register'".to_string(),
                "8. Copy the 'Application (client) ID' from the Overview page".to_string(),
                "9. Go to 'Certificates & secrets' > 'Client secrets'".to_string(),
                "10. Click 'New client secret' and copy the value".to_string(),
                "11. Go to 'API permissions' > 'Add a permission'".to_string(),
                "12. Select 'Microsoft Graph' > 'Delegated permissions'".to_string(),
                "13. Add: Mail.Read, Mail.ReadWrite, Mail.Send, offline_access".to_string(),
                "14. Click 'Grant admin consent' (if you have admin rights)".to_string(),
            ],
            OAuth2Provider::Yahoo => vec![
                "To set up Yahoo OAuth2 access:".to_string(),
                "".to_string(),
                "1. Go to Yahoo Developer Network (developer.yahoo.com)".to_string(),
                "2. Sign in with your Yahoo account".to_string(),
                "3. Click 'Create an App'".to_string(),
                "4. Fill in the application details:".to_string(),
                "   - Application Name: Comunicado".to_string(),
                "   - Application Type: Web Application".to_string(),
                "5. Under 'Redirect URI(s)', enter:".to_string(),
                "   http://localhost:8080/oauth/callback".to_string(),
                "   (Note: Comunicado will use ports 8080-8089 automatically)".to_string(),
                "6. Select the required permissions:".to_string(),
                "   - Mail (Read/Write)".to_string(),
                "7. Click 'Create App'".to_string(),
                "8. Copy both the Client ID and Client Secret".to_string(),
                "".to_string(),
                "Note: Yahoo requires both Client ID and Client Secret.".to_string(),
            ],
            OAuth2Provider::Custom(name) => vec![
                format!("To use {} with OAuth2:", name),
                "1. Check provider's OAuth2 documentation".to_string(),
                "2. Register application with provider".to_string(),
                "3. Configure redirect URI: http://localhost:8080/oauth/callback".to_string(),
                "4. Obtain client credentials".to_string(),
                "5. Configure IMAP/SMTP server settings".to_string(),
            ],
        }
    }
}

/// Provider detection helpers
pub struct ProviderDetector;

impl ProviderDetector {
    /// Detect provider from email address
    pub fn detect_from_email(email: &str) -> Option<OAuth2Provider> {
        let domain = email.split('@').nth(1)?.to_lowercase();
        
        match domain.as_str() {
            "gmail.com" | "googlemail.com" => Some(OAuth2Provider::Gmail),
            "outlook.com" | "hotmail.com" | "live.com" | "msn.com" => Some(OAuth2Provider::Outlook),
            "yahoo.com" | "yahoo.co.uk" | "yahoo.ca" | "yahoo.au" => Some(OAuth2Provider::Yahoo),
            _ => None,
        }
    }
    
    /// Get all common email domains for each provider
    pub fn get_provider_domains(provider: &OAuth2Provider) -> Vec<&'static str> {
        match provider {
            OAuth2Provider::Gmail => vec!["gmail.com", "googlemail.com"],
            OAuth2Provider::Outlook => vec!["outlook.com", "hotmail.com", "live.com", "msn.com"],
            OAuth2Provider::Yahoo => vec!["yahoo.com", "yahoo.co.uk", "yahoo.ca", "yahoo.au"],
            OAuth2Provider::Custom(_) => vec![],
        }
    }
    
    /// Check if email domain is supported by any provider
    pub fn is_supported_domain(email: &str) -> bool {
        Self::detect_from_email(email).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_provider_from_str() {
        assert_eq!(OAuth2Provider::from_str("gmail").unwrap(), OAuth2Provider::Gmail);
        assert_eq!(OAuth2Provider::from_str("outlook").unwrap(), OAuth2Provider::Outlook);
        assert_eq!(OAuth2Provider::from_str("yahoo").unwrap(), OAuth2Provider::Yahoo);
        assert!(OAuth2Provider::from_str("invalid").is_err());
    }
    
    #[test]
    fn test_provider_display_names() {
        assert_eq!(OAuth2Provider::Gmail.display_name(), "Gmail");
        assert_eq!(OAuth2Provider::Outlook.display_name(), "Outlook/Hotmail");
        assert_eq!(OAuth2Provider::Yahoo.display_name(), "Yahoo Mail");
    }
    
    #[test]
    fn test_provider_detection() {
        assert_eq!(
            ProviderDetector::detect_from_email("user@gmail.com"),
            Some(OAuth2Provider::Gmail)
        );
        assert_eq!(
            ProviderDetector::detect_from_email("user@outlook.com"),
            Some(OAuth2Provider::Outlook)
        );
        assert_eq!(
            ProviderDetector::detect_from_email("user@yahoo.com"),
            Some(OAuth2Provider::Yahoo)
        );
        assert_eq!(
            ProviderDetector::detect_from_email("user@example.com"),
            None
        );
    }
    
    #[test]
    fn test_provider_config_validation() {
        let config = ProviderConfig::gmail();
        // This will fail because we're using placeholder credentials
        assert!(config.validate().is_err());
        
        let valid_config = config.with_credentials(
            "real-client-id".to_string(),
            Some("real-client-secret".to_string())
        );
        assert!(valid_config.validate().is_ok());
    }
    
    #[test]
    fn test_scope_string() {
        let config = ProviderConfig::gmail();
        let scope_string = config.scope_string();
        assert!(scope_string.contains("https://mail.google.com/"));
    }
    
    #[test]
    fn test_supported_providers() {
        let providers = ProviderConfig::supported_providers();
        assert_eq!(providers.len(), 3);
        assert!(providers.contains(&OAuth2Provider::Gmail));
        assert!(providers.contains(&OAuth2Provider::Outlook));
        assert!(providers.contains(&OAuth2Provider::Yahoo));
    }
}