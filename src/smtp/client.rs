use crate::smtp::{SmtpError, SmtpResult, SmtpAuth, SmtpSecurity, SendResult};
use crate::oauth2::TokenManager;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message,
    transport::smtp::{
        authentication::{Credentials, Mechanism},
        client::{Tls, TlsParameters},
        PoolConfig,
    },
};
use std::time::Duration;

/// SMTP client configuration
#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub server: String,
    pub port: u16,
    pub security: SmtpSecurity,
    pub auth: SmtpAuth,
    pub timeout: Duration,
    pub pool_max_size: u32,
}

impl SmtpConfig {
    pub fn new(server: String, port: u16, auth: SmtpAuth) -> Self {
        Self {
            server,
            port,
            security: SmtpSecurity::StartTls,
            auth,
            timeout: Duration::from_secs(30),
            pool_max_size: 10,
        }
    }
    
    pub fn with_security(mut self, security: SmtpSecurity) -> Self {
        self.security = security;
        self
    }
    
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    
    pub fn with_pool_size(mut self, max_size: u32) -> Self {
        self.pool_max_size = max_size;
        self
    }
}

/// SMTP client for sending emails
#[derive(Clone)]
pub struct SmtpClient {
    config: SmtpConfig,
    transport: AsyncSmtpTransport<lettre::Tokio1Executor>,
    token_manager: Option<TokenManager>,
}

impl SmtpClient {
    /// Create a new SMTP client
    pub async fn new(config: SmtpConfig) -> SmtpResult<Self> {
        let transport = Self::build_transport(&config).await?;
        
        Ok(Self {
            config,
            transport,
            token_manager: None,
        })
    }
    
    /// Create a new SMTP client with OAuth2 token manager
    pub async fn new_with_oauth2(config: SmtpConfig, token_manager: TokenManager) -> SmtpResult<Self> {
        let transport = Self::build_transport(&config).await?;
        
        Ok(Self {
            config,
            transport,
            token_manager: Some(token_manager),
        })
    }
    
    /// Build the lettre transport from configuration
    async fn build_transport(config: &SmtpConfig) -> SmtpResult<AsyncSmtpTransport<lettre::Tokio1Executor>> {
        let _server_addr = format!("{}:{}", config.server, config.port);
        
        // Create builder
        let mut builder = AsyncSmtpTransport::<lettre::Tokio1Executor>::builder_dangerous(&config.server)
            .port(config.port)
            .timeout(Some(config.timeout))
            .pool_config(PoolConfig::new().max_size(config.pool_max_size));
        
        // Configure TLS
        match config.security {
            SmtpSecurity::None => {
                builder = builder.tls(Tls::None);
            }
            SmtpSecurity::StartTls => {
                let tls_parameters = TlsParameters::builder(config.server.clone())
                    .dangerous_accept_invalid_certs(false)
                    .dangerous_accept_invalid_hostnames(false)
                    .build()
                    .map_err(|e| SmtpError::ConnectionFailed(format!("TLS configuration failed: {}", e)))?;
                
                builder = builder.tls(Tls::Opportunistic(tls_parameters));
            }
            SmtpSecurity::Tls => {
                let tls_parameters = TlsParameters::builder(config.server.clone())
                    .dangerous_accept_invalid_certs(false)
                    .dangerous_accept_invalid_hostnames(false)
                    .build()
                    .map_err(|e| SmtpError::ConnectionFailed(format!("TLS configuration failed: {}", e)))?;
                
                builder = builder.tls(Tls::Required(tls_parameters));
            }
        }
        
        // Configure authentication
        builder = match &config.auth {
            SmtpAuth::OAuth2 { username, access_token } => {
                let oauth2_creds = Credentials::new(username.clone(), access_token.clone());
                builder
                    .credentials(oauth2_creds)
                    .authentication(vec![Mechanism::Xoauth2])
            }
            SmtpAuth::Plain { username, password } => {
                let plain_creds = Credentials::new(username.clone(), password.clone());
                builder
                    .credentials(plain_creds)
                    .authentication(vec![Mechanism::Plain])
            }
            SmtpAuth::Login { username, password } => {
                let login_creds = Credentials::new(username.clone(), password.clone());
                builder
                    .credentials(login_creds)
                    .authentication(vec![Mechanism::Login])
            }
        };
        
        let transport = builder.build();
        Ok(transport)
    }
    
    /// Send an email message
    pub async fn send(&self, message: Message) -> SmtpResult<SendResult> {
        // Extract message ID for result
        let message_id = message.headers()
            .get_raw("Message-ID")
            .and_then(|h| Some(h.to_string()))
            .unwrap_or_else(|| format!("<{}@{}>", uuid::Uuid::new_v4(), self.config.server));
        
        // Get recipients for result tracking
        let envelope = message.envelope();
        let to_addresses: Vec<String> = envelope.to().iter().map(|addr| addr.to_string()).collect();
        
        // Send the message
        let result = self.transport.send(message).await;
        
        match result {
            Ok(response) => {
                let send_result = SendResult {
                    message_id,
                    accepted_recipients: to_addresses,
                    rejected_recipients: Vec::new(),
                    sent_at: chrono::Utc::now(),
                };
                
                tracing::info!("Email sent successfully: {} to {} recipients", 
                             send_result.message_id, send_result.accepted_recipients.len());
                
                Ok(send_result)
            }
            Err(e) => {
                tracing::error!("Failed to send email: {}", e);
                Err(SmtpError::SendFailed(e.to_string()))
            }
        }
    }
    
    /// Send email with automatic token refresh for OAuth2
    pub async fn send_with_refresh(&mut self, message: Message, account_id: &str) -> SmtpResult<SendResult> {
        // First try to send with current token
        match self.send(message.clone()).await {
            Ok(result) => return Ok(result),
            Err(SmtpError::AuthenticationFailed(_)) | Err(SmtpError::NetworkError(_)) => {
                // Try to refresh the token if we have a token manager
                if let Some(ref token_manager) = self.token_manager {
                    tracing::info!("SMTP authentication failed, attempting token refresh for account: {}", account_id);
                    
                    match token_manager.refresh_access_token(account_id).await {
                        Ok(new_token) => {
                            // Update the configuration with new token
                            if let SmtpAuth::OAuth2 { ref mut access_token, .. } = &mut self.config.auth {
                                *access_token = new_token.token;
                                
                                // Rebuild transport with new token
                                self.transport = Self::build_transport(&self.config).await?;
                                
                                // Retry sending
                                return self.send(message).await;
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to refresh OAuth2 token: {}", e);
                            return Err(SmtpError::OAuth2Error(e.to_string()));
                        }
                    }
                }
            }
            Err(e) => return Err(e),
        }
        
        // If we get here, the original error wasn't authentication-related
        self.send(message).await
    }
    
    /// Test the SMTP connection
    pub async fn test_connection(&self) -> SmtpResult<()> {
        // Create a test transport to verify connection
        let test_transport = Self::build_transport(&self.config).await?;
        
        // Test connection by sending NOOP
        match test_transport.test_connection().await {
            Ok(_) => {
                tracing::info!("SMTP connection test successful for {}:{}", self.config.server, self.config.port);
                Ok(())
            }
            Err(e) => {
                tracing::error!("SMTP connection test failed: {}", e);
                Err(SmtpError::ConnectionFailed(e.to_string()))
            }
        }
    }
    
    /// Get the current configuration
    pub fn config(&self) -> &SmtpConfig {
        &self.config
    }
    
    /// Update OAuth2 access token
    pub async fn update_oauth2_token(&mut self, new_token: String) -> SmtpResult<()> {
        if let SmtpAuth::OAuth2 { ref mut access_token, .. } = &mut self.config.auth {
            *access_token = new_token;
            
            // Rebuild transport with new token
            self.transport = Self::build_transport(&self.config).await?;
            
            tracing::info!("Updated OAuth2 token for SMTP client");
            Ok(())
        } else {
            Err(SmtpError::InvalidConfig("Not configured for OAuth2 authentication".to_string()))
        }
    }
}

/// Builder for SMTP configuration from OAuth2 provider
impl SmtpConfig {
    /// Create SMTP config for Gmail with OAuth2
    pub fn gmail_oauth2(username: String, access_token: String) -> Self {
        Self::new(
            "smtp.gmail.com".to_string(),
            587,
            SmtpAuth::OAuth2 { username, access_token },
        )
        .with_security(SmtpSecurity::StartTls)
    }
    
    /// Create SMTP config for Outlook with OAuth2
    pub fn outlook_oauth2(username: String, access_token: String) -> Self {
        Self::new(
            "smtp-mail.outlook.com".to_string(),
            587,
            SmtpAuth::OAuth2 { username, access_token },
        )
        .with_security(SmtpSecurity::StartTls)
    }
    
    /// Create SMTP config for Yahoo with OAuth2
    pub fn yahoo_oauth2(username: String, access_token: String) -> Self {
        Self::new(
            "smtp.mail.yahoo.com".to_string(),
            587,
            SmtpAuth::OAuth2 { username, access_token },
        )
        .with_security(SmtpSecurity::StartTls)
    }
    
    /// Create SMTP config from OAuth2 provider configuration
    pub fn from_provider(
        _provider: &str,
        smtp_server: &str,
        smtp_port: u16,
        username: String,
        access_token: String,
    ) -> SmtpResult<Self> {
        if smtp_server.is_empty() {
            return Err(SmtpError::InvalidConfig("SMTP server cannot be empty".to_string()));
        }
        
        if smtp_port == 0 {
            return Err(SmtpError::InvalidConfig("SMTP port cannot be zero".to_string()));
        }
        
        Ok(Self::new(
            smtp_server.to_string(),
            smtp_port,
            SmtpAuth::OAuth2 { username, access_token },
        )
        .with_security(SmtpSecurity::StartTls))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_smtp_config_creation() {
        let config = SmtpConfig::gmail_oauth2(
            "test@gmail.com".to_string(),
            "fake_token".to_string(),
        );
        
        assert_eq!(config.server, "smtp.gmail.com");
        assert_eq!(config.port, 587);
        assert_eq!(config.security, SmtpSecurity::StartTls);
        
        if let SmtpAuth::OAuth2 { username, access_token } = &config.auth {
            assert_eq!(username, "test@gmail.com");
            assert_eq!(access_token, "fake_token");
        } else {
            panic!("Expected OAuth2 authentication");
        }
    }
    
    #[test]
    fn test_smtp_config_builder() {
        let config = SmtpConfig::new(
            "smtp.example.com".to_string(),
            465,
            SmtpAuth::Plain {
                username: "user".to_string(),
                password: "pass".to_string(),
            },
        )
        .with_security(SmtpSecurity::Tls)
        .with_timeout(Duration::from_secs(60))
        .with_pool_size(5);
        
        assert_eq!(config.server, "smtp.example.com");
        assert_eq!(config.port, 465);
        assert_eq!(config.security, SmtpSecurity::Tls);
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.pool_max_size, 5);
    }
}