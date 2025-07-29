use crate::smtp::{SmtpAuth, SmtpConfig, SmtpError, SmtpResult, SmtpSecurity};
use std::collections::HashMap;

/// Provider-specific SMTP configurations
#[derive(Debug, Clone)]
pub struct SmtpProviderConfig {
    pub name: String,
    pub display_name: String,
    pub smtp_server: String,
    pub smtp_port: u16,
    pub security: SmtpSecurity,
    pub auth_url: Option<String>,
    pub token_url: Option<String>,
    pub scopes: Vec<String>,
}

impl SmtpProviderConfig {
    /// Create SMTP config for this provider with OAuth2 credentials
    pub fn create_config(&self, username: String, access_token: String) -> SmtpConfig {
        SmtpConfig::new(
            self.smtp_server.clone(),
            self.smtp_port,
            SmtpAuth::OAuth2 {
                username,
                access_token,
            },
        )
        .with_security(self.security.clone())
    }

    /// Create SMTP config with plain authentication
    pub fn create_config_plain(&self, username: String, password: String) -> SmtpConfig {
        SmtpConfig::new(
            self.smtp_server.clone(),
            self.smtp_port,
            SmtpAuth::Plain { username, password },
        )
        .with_security(self.security.clone())
    }
}

/// SMTP provider registry
#[derive(Clone)]
pub struct SmtpProviderRegistry {
    providers: HashMap<String, SmtpProviderConfig>,
}

impl SmtpProviderRegistry {
    /// Create a new provider registry with default providers
    pub fn new() -> Self {
        let mut providers = HashMap::new();

        // Gmail
        providers.insert(
            "gmail".to_string(),
            SmtpProviderConfig {
                name: "gmail".to_string(),
                display_name: "Gmail".to_string(),
                smtp_server: "smtp.gmail.com".to_string(),
                smtp_port: 587,
                security: SmtpSecurity::StartTls,
                auth_url: Some("https://accounts.google.com/o/oauth2/v2/auth".to_string()),
                token_url: Some("https://oauth2.googleapis.com/token".to_string()),
                scopes: vec![
                    "https://mail.google.com/".to_string(),
                    "https://www.googleapis.com/auth/gmail.send".to_string(),
                ],
            },
        );

        // Outlook
        providers.insert(
            "outlook".to_string(),
            SmtpProviderConfig {
                name: "outlook".to_string(),
                display_name: "Outlook/Hotmail".to_string(),
                smtp_server: "smtp-mail.outlook.com".to_string(),
                smtp_port: 587,
                security: SmtpSecurity::StartTls,
                auth_url: Some(
                    "https://login.microsoftonline.com/common/oauth2/v2.0/authorize".to_string(),
                ),
                token_url: Some(
                    "https://login.microsoftonline.com/common/oauth2/v2.0/token".to_string(),
                ),
                scopes: vec![
                    "https://outlook.office.com/SMTP.Send".to_string(),
                    "offline_access".to_string(),
                ],
            },
        );

        // Yahoo
        providers.insert(
            "yahoo".to_string(),
            SmtpProviderConfig {
                name: "yahoo".to_string(),
                display_name: "Yahoo Mail".to_string(),
                smtp_server: "smtp.mail.yahoo.com".to_string(),
                smtp_port: 587,
                security: SmtpSecurity::StartTls,
                auth_url: Some("https://api.login.yahoo.com/oauth2/request_auth".to_string()),
                token_url: Some("https://api.login.yahoo.com/oauth2/get_token".to_string()),
                scopes: vec!["mail-w".to_string()],
            },
        );

        // iCloud
        providers.insert(
            "icloud".to_string(),
            SmtpProviderConfig {
                name: "icloud".to_string(),
                display_name: "iCloud".to_string(),
                smtp_server: "smtp.mail.me.com".to_string(),
                smtp_port: 587,
                security: SmtpSecurity::StartTls,
                auth_url: None, // iCloud uses app-specific passwords
                token_url: None,
                scopes: vec![],
            },
        );

        // ProtonMail Bridge
        providers.insert(
            "protonmail".to_string(),
            SmtpProviderConfig {
                name: "protonmail".to_string(),
                display_name: "ProtonMail (Bridge)".to_string(),
                smtp_server: "127.0.0.1".to_string(),
                smtp_port: 1025,
                security: SmtpSecurity::StartTls,
                auth_url: None,
                token_url: None,
                scopes: vec![],
            },
        );

        Self { providers }
    }

    /// Get a provider configuration by name
    pub fn get_provider(&self, name: &str) -> Option<&SmtpProviderConfig> {
        self.providers.get(name)
    }

    /// Get all available providers
    pub fn get_all_providers(&self) -> Vec<&SmtpProviderConfig> {
        self.providers.values().collect()
    }

    /// Get providers that support OAuth2
    pub fn get_oauth2_providers(&self) -> Vec<&SmtpProviderConfig> {
        self.providers
            .values()
            .filter(|p| p.auth_url.is_some() && p.token_url.is_some())
            .collect()
    }

    /// Add a custom provider
    pub fn add_provider(&mut self, config: SmtpProviderConfig) {
        self.providers.insert(config.name.clone(), config);
    }

    /// Create SMTP config from provider name and credentials
    pub fn create_smtp_config(
        &self,
        provider_name: &str,
        username: String,
        access_token: String,
    ) -> SmtpResult<SmtpConfig> {
        let provider = self.get_provider(provider_name).ok_or_else(|| {
            SmtpError::InvalidConfig(format!("Unknown provider: {}", provider_name))
        })?;

        Ok(provider.create_config(username, access_token))
    }

    /// Create SMTP config with plain authentication
    pub fn create_smtp_config_plain(
        &self,
        provider_name: &str,
        username: String,
        password: String,
    ) -> SmtpResult<SmtpConfig> {
        let provider = self.get_provider(provider_name).ok_or_else(|| {
            SmtpError::InvalidConfig(format!("Unknown provider: {}", provider_name))
        })?;

        Ok(provider.create_config_plain(username, password))
    }

    /// Create custom SMTP config for unlisted providers
    pub fn create_custom_smtp_config(
        smtp_server: String,
        smtp_port: u16,
        username: String,
        access_token: String,
        use_tls: bool,
    ) -> SmtpResult<SmtpConfig> {
        if smtp_server.is_empty() {
            return Err(SmtpError::InvalidConfig(
                "SMTP server cannot be empty".to_string(),
            ));
        }

        if smtp_port == 0 {
            return Err(SmtpError::InvalidConfig(
                "SMTP port cannot be zero".to_string(),
            ));
        }

        let security = if use_tls {
            SmtpSecurity::StartTls
        } else {
            SmtpSecurity::None
        };

        Ok(SmtpConfig::new(
            smtp_server,
            smtp_port,
            SmtpAuth::OAuth2 {
                username,
                access_token,
            },
        )
        .with_security(security))
    }

    /// Get provider by SMTP server
    pub fn get_provider_by_server(&self, smtp_server: &str) -> Option<&SmtpProviderConfig> {
        self.providers
            .values()
            .find(|p| p.smtp_server == smtp_server)
    }

    /// Detect provider from email domain
    pub fn detect_provider_from_email(&self, email: &str) -> Option<&SmtpProviderConfig> {
        if let Some(domain) = email.split('@').nth(1) {
            match domain.to_lowercase().as_str() {
                "gmail.com" | "googlemail.com" => self.get_provider("gmail"),
                "outlook.com" | "hotmail.com" | "live.com" | "msn.com" => {
                    self.get_provider("outlook")
                }
                "yahoo.com" | "yahoo.co.uk" | "yahoo.fr" | "ymail.com" => {
                    self.get_provider("yahoo")
                }
                "icloud.com" | "me.com" | "mac.com" => self.get_provider("icloud"),
                "protonmail.com" | "protonmail.ch" | "pm.me" => self.get_provider("protonmail"),
                _ => None,
            }
        } else {
            None
        }
    }
}

impl Default for SmtpProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility functions for SMTP provider management
pub mod utils {
    use super::*;

    /// Validate SMTP configuration
    pub fn validate_smtp_config(config: &SmtpConfig) -> SmtpResult<()> {
        if config.server.is_empty() {
            return Err(SmtpError::InvalidConfig(
                "SMTP server cannot be empty".to_string(),
            ));
        }

        if config.port == 0 {
            return Err(SmtpError::InvalidConfig(
                "SMTP port cannot be zero".to_string(),
            ));
        }

        match &config.auth {
            SmtpAuth::OAuth2 {
                username,
                access_token,
            } => {
                if username.is_empty() {
                    return Err(SmtpError::InvalidConfig(
                        "Username cannot be empty".to_string(),
                    ));
                }
                if access_token.is_empty() {
                    return Err(SmtpError::InvalidConfig(
                        "Access token cannot be empty".to_string(),
                    ));
                }
            }
            SmtpAuth::Plain { username, password } | SmtpAuth::Login { username, password } => {
                if username.is_empty() {
                    return Err(SmtpError::InvalidConfig(
                        "Username cannot be empty".to_string(),
                    ));
                }
                if password.is_empty() {
                    return Err(SmtpError::InvalidConfig(
                        "Password cannot be empty".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Get recommended security settings for a port
    pub fn get_recommended_security(port: u16) -> SmtpSecurity {
        match port {
            25 => SmtpSecurity::StartTls,
            465 => SmtpSecurity::Tls,
            587 => SmtpSecurity::StartTls,
            _ => SmtpSecurity::StartTls,
        }
    }

    /// Check if a provider supports OAuth2
    pub fn provider_supports_oauth2(provider: &SmtpProviderConfig) -> bool {
        provider.auth_url.is_some() && provider.token_url.is_some()
    }

    /// Get OAuth2 scopes for a provider
    pub fn get_oauth2_scopes(provider: &SmtpProviderConfig) -> Vec<String> {
        provider.scopes.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_registry() {
        let registry = SmtpProviderRegistry::new();

        // Test getting providers
        assert!(registry.get_provider("gmail").is_some());
        assert!(registry.get_provider("outlook").is_some());
        assert!(registry.get_provider("yahoo").is_some());
        assert!(registry.get_provider("nonexistent").is_none());

        // Test OAuth2 providers
        let oauth2_providers = registry.get_oauth2_providers();
        assert!(!oauth2_providers.is_empty());

        // Test provider detection from email
        assert_eq!(
            registry
                .detect_provider_from_email("user@gmail.com")
                .unwrap()
                .name,
            "gmail"
        );
        assert_eq!(
            registry
                .detect_provider_from_email("user@outlook.com")
                .unwrap()
                .name,
            "outlook"
        );
        assert_eq!(
            registry
                .detect_provider_from_email("user@yahoo.com")
                .unwrap()
                .name,
            "yahoo"
        );
        assert!(registry
            .detect_provider_from_email("user@unknown.com")
            .is_none());
    }

    #[test]
    fn test_smtp_config_creation() {
        let registry = SmtpProviderRegistry::new();

        let config = registry
            .create_smtp_config(
                "gmail",
                "test@gmail.com".to_string(),
                "fake_token".to_string(),
            )
            .unwrap();

        assert_eq!(config.server, "smtp.gmail.com");
        assert_eq!(config.port, 587);
        assert_eq!(config.security, SmtpSecurity::StartTls);

        if let SmtpAuth::OAuth2 {
            username,
            access_token,
        } = &config.auth
        {
            assert_eq!(username, "test@gmail.com");
            assert_eq!(access_token, "fake_token");
        } else {
            panic!("Expected OAuth2 authentication");
        }
    }

    #[test]
    fn test_custom_smtp_config() {
        let config = SmtpProviderRegistry::create_custom_smtp_config(
            "smtp.example.com".to_string(),
            587,
            "user@example.com".to_string(),
            "token123".to_string(),
            true,
        )
        .unwrap();

        assert_eq!(config.server, "smtp.example.com");
        assert_eq!(config.port, 587);
        assert_eq!(config.security, SmtpSecurity::StartTls);
    }

    #[test]
    fn test_config_validation() {
        let config = SmtpConfig::new(
            "smtp.gmail.com".to_string(),
            587,
            SmtpAuth::OAuth2 {
                username: "test@gmail.com".to_string(),
                access_token: "token".to_string(),
            },
        );

        assert!(utils::validate_smtp_config(&config).is_ok());

        // Test invalid config
        let invalid_config = SmtpConfig::new(
            "".to_string(),
            587,
            SmtpAuth::OAuth2 {
                username: "test@gmail.com".to_string(),
                access_token: "token".to_string(),
            },
        );

        assert!(utils::validate_smtp_config(&invalid_config).is_err());
    }

    #[test]
    fn test_security_recommendations() {
        assert_eq!(utils::get_recommended_security(25), SmtpSecurity::StartTls);
        assert_eq!(utils::get_recommended_security(465), SmtpSecurity::Tls);
        assert_eq!(utils::get_recommended_security(587), SmtpSecurity::StartTls);
        assert_eq!(
            utils::get_recommended_security(2525),
            SmtpSecurity::StartTls
        );
    }
}
