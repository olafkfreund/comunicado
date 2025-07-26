use crate::smtp::{
    SmtpClient, SmtpProviderRegistry, SmtpResult, SmtpError, SendResult, EmailMessage
};
use crate::ui::EmailComposeData;
use crate::oauth2::TokenManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// SMTP service for managing email sending across multiple accounts
#[derive(Clone)]
pub struct SmtpService {
    provider_registry: SmtpProviderRegistry,
    clients: Arc<RwLock<HashMap<String, SmtpClient>>>,
    token_manager: Arc<TokenManager>,
}

impl SmtpService {
    /// Create a new SMTP service
    pub fn new(token_manager: Arc<TokenManager>) -> Self {
        Self {
            provider_registry: SmtpProviderRegistry::new(),
            clients: Arc::new(RwLock::new(HashMap::new())),
            token_manager,
        }
    }
    
    /// Initialize SMTP client for an account
    pub async fn initialize_account(
        &self,
        account_id: &str,
        provider: &str,
        username: &str,
        access_token: &str,
    ) -> SmtpResult<()> {
        // Create SMTP configuration
        let config = self.provider_registry.create_smtp_config(
            provider,
            username.to_string(),
            access_token.to_string(),
        )?;
        
        // Create SMTP client with OAuth2 support
        let client = SmtpClient::new_with_oauth2(config, self.token_manager.as_ref().clone()).await?;
        
        // Test the connection
        client.test_connection().await?;
        
        // Store the client
        let mut clients = self.clients.write().await;
        clients.insert(account_id.to_string(), client);
        
        tracing::info!("SMTP client initialized for account: {}", account_id);
        Ok(())
    }
    
    /// Initialize SMTP client with custom configuration
    pub async fn initialize_custom_account(
        &self,
        account_id: &str,
        smtp_server: &str,
        smtp_port: u16,
        username: &str,
        access_token: &str,
        use_tls: bool,
    ) -> SmtpResult<()> {
        // Create custom SMTP configuration
        let config = SmtpProviderRegistry::create_custom_smtp_config(
            smtp_server.to_string(),
            smtp_port,
            username.to_string(),
            access_token.to_string(),
            use_tls,
        )?;
        
        // Create SMTP client with OAuth2 support
        let client = SmtpClient::new_with_oauth2(config, self.token_manager.as_ref().clone()).await?;
        
        // Test the connection
        client.test_connection().await?;
        
        // Store the client
        let mut clients = self.clients.write().await;
        clients.insert(account_id.to_string(), client);
        
        tracing::info!("Custom SMTP client initialized for account: {}", account_id);
        Ok(())
    }
    
    /// Send an email using the compose UI data
    pub async fn send_email(
        &self,
        account_id: &str,
        from_address: &str,
        compose_data: &EmailComposeData,
    ) -> SmtpResult<SendResult> {
        // Create email message from compose data
        let email_message = EmailMessage::from_compose_data(compose_data, from_address.to_string())?;
        
        // Validate the message
        email_message.validate()?;
        
        // Get the SMTP client for this account
        let clients = self.clients.read().await;
        let client = clients.get(account_id)
            .ok_or_else(|| SmtpError::InvalidConfig(format!("No SMTP client found for account: {}", account_id)))?;
        
        // Convert to lettre message
        let message = email_message.to_lettre_message()?;
        
        // Send the email with automatic token refresh
        let mut client_clone = client.clone();
        drop(clients); // Release the read lock
        
        client_clone.send_with_refresh(message, account_id).await
    }
    
    /// Send a pre-built email message
    pub async fn send_message(
        &self,
        account_id: &str,
        email_message: &EmailMessage,
    ) -> SmtpResult<SendResult> {
        // Validate the message
        email_message.validate()?;
        
        // Get the SMTP client for this account
        let clients = self.clients.read().await;
        let client = clients.get(account_id)
            .ok_or_else(|| SmtpError::InvalidConfig(format!("No SMTP client found for account: {}", account_id)))?;
        
        // Convert to lettre message
        let message = email_message.to_lettre_message()?;
        
        // Send the email with automatic token refresh
        let mut client_clone = client.clone();
        drop(clients); // Release the read lock
        
        client_clone.send_with_refresh(message, account_id).await
    }
    
    /// Send a reply email
    pub async fn send_reply(
        &self,
        account_id: &str,
        from_address: &str,
        original_message: &EmailMessage,
        reply_body: &str,
        reply_all: bool,
    ) -> SmtpResult<SendResult> {
        // Create reply message
        let reply_message = EmailMessage::create_reply(
            original_message,
            from_address.to_string(),
            reply_body.to_string(),
            reply_all,
        );
        
        // Send the reply
        self.send_message(account_id, &reply_message).await
    }
    
    /// Send a forward email
    pub async fn send_forward(
        &self,
        account_id: &str,
        from_address: &str,
        original_message: &EmailMessage,
        forward_body: &str,
        recipients: Vec<String>,
    ) -> SmtpResult<SendResult> {
        // Create forward message
        let mut forward_message = EmailMessage::create_forward(
            original_message,
            from_address.to_string(),
            forward_body.to_string(),
        );
        
        // Set recipients
        forward_message.to = recipients;
        
        // Send the forward
        self.send_message(account_id, &forward_message).await
    }
    
    /// Test SMTP connection for an account
    pub async fn test_connection(&self, account_id: &str) -> SmtpResult<()> {
        let clients = self.clients.read().await;
        let client = clients.get(account_id)
            .ok_or_else(|| SmtpError::InvalidConfig(format!("No SMTP client found for account: {}", account_id)))?;
        
        client.test_connection().await
    }
    
    /// Update OAuth2 token for an account
    pub async fn update_oauth2_token(&self, account_id: &str, new_token: &str) -> SmtpResult<()> {
        let mut clients = self.clients.write().await;
        if let Some(client) = clients.get_mut(account_id) {
            client.update_oauth2_token(new_token.to_string()).await?;
            tracing::info!("Updated OAuth2 token for SMTP account: {}", account_id);
            Ok(())
        } else {
            Err(SmtpError::InvalidConfig(format!("No SMTP client found for account: {}", account_id)))
        }
    }
    
    /// Remove an account's SMTP client
    pub async fn remove_account(&self, account_id: &str) -> bool {
        let mut clients = self.clients.write().await;
        clients.remove(account_id).is_some()
    }
    
    /// Get list of configured accounts
    pub async fn get_configured_accounts(&self) -> Vec<String> {
        let clients = self.clients.read().await;
        clients.keys().cloned().collect()
    }
    
    /// Check if an account has SMTP configured
    pub async fn is_account_configured(&self, account_id: &str) -> bool {
        let clients = self.clients.read().await;
        clients.contains_key(account_id)
    }
    
    /// Get provider registry for configuration purposes
    pub fn provider_registry(&self) -> &SmtpProviderRegistry {
        &self.provider_registry
    }
    
    /// Detect provider from email address
    pub fn detect_provider_from_email(&self, email: &str) -> Option<String> {
        self.provider_registry
            .detect_provider_from_email(email)
            .map(|provider| provider.name.clone())
    }
    
    /// Get OAuth2 scopes for a provider
    pub fn get_oauth2_scopes(&self, provider: &str) -> Vec<String> {
        self.provider_registry
            .get_provider(provider)
            .map(|p| p.scopes.clone())
            .unwrap_or_default()
    }
    
    /// Save draft email (placeholder for future implementation)
    pub async fn save_draft(
        &self,
        account_id: &str,
        compose_data: &EmailComposeData,
    ) -> SmtpResult<String> {
        // TODO: Implement draft saving to local storage
        // For now, just validate the data and return a draft ID
        
        let email_message = EmailMessage::from_compose_data(compose_data, "placeholder@example.com".to_string())?;
        
        // Basic validation (don't require all fields for drafts)
        if email_message.subject.is_empty() && email_message.body_text.is_empty() {
            return Err(SmtpError::MessageFormatError("Draft must have either subject or body".to_string()));
        }
        
        // Generate a draft ID
        let draft_id = uuid::Uuid::new_v4().to_string();
        
        tracing::info!("Draft saved for account {}: {}", account_id, draft_id);
        Ok(draft_id)
    }
    
    /// Load draft email (placeholder for future implementation)  
    pub async fn load_draft(&self, _account_id: &str, _draft_id: &str) -> SmtpResult<EmailComposeData> {
        // TODO: Implement draft loading from local storage
        // For now, return an error
        Err(SmtpError::InvalidConfig(format!("Draft not found: {}", _draft_id)))
    }
    
    /// Delete draft email (placeholder for future implementation)
    pub async fn delete_draft(&self, account_id: &str, draft_id: &str) -> SmtpResult<()> {
        // TODO: Implement draft deletion from local storage
        // For now, just log the operation
        tracing::info!("Draft deleted for account {}: {}", account_id, draft_id);
        Ok(())
    }
}

/// Builder for creating SMTP service configurations
pub struct SmtpServiceBuilder {
    token_manager: Option<Arc<TokenManager>>,
}

impl SmtpServiceBuilder {
    pub fn new() -> Self {
        Self {
            token_manager: None,
        }
    }
    
    pub fn with_token_manager(mut self, token_manager: Arc<TokenManager>) -> Self {
        self.token_manager = Some(token_manager);
        self
    }
    
    pub fn build(self) -> SmtpResult<SmtpService> {
        let token_manager = self.token_manager
            .ok_or_else(|| SmtpError::InvalidConfig("Token manager is required".to_string()))?;
        
        Ok(SmtpService::new(token_manager))
    }
}

impl Default for SmtpServiceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_smtp_service_creation() {
        let token_manager = Arc::new(TokenManager::new("test_client_id", "test_client_secret"));
        let service = SmtpService::new(token_manager);
        
        // Check that service starts with no configured accounts
        assert_eq!(service.get_configured_accounts().await.len(), 0);
        assert!(!service.is_account_configured("test_account").await);
    }
    
    #[test]
    fn test_provider_detection() {
        let token_manager = Arc::new(TokenManager::new("test_client_id", "test_client_secret"));
        let service = SmtpService::new(token_manager);
        
        assert_eq!(service.detect_provider_from_email("user@gmail.com"), Some("gmail".to_string()));
        assert_eq!(service.detect_provider_from_email("user@outlook.com"), Some("outlook".to_string()));
        assert_eq!(service.detect_provider_from_email("user@unknown.com"), None);
    }
    
    #[tokio::test]
    async fn test_draft_operations() {
        let token_manager = Arc::new(TokenManager::new("test_client_id", "test_client_secret"));
        let service = SmtpService::new(token_manager);
        
        let compose_data = EmailComposeData {
            to: "recipient@example.com".to_string(),
            cc: String::new(),
            bcc: String::new(),
            subject: "Test Subject".to_string(),
            body: "Test body content".to_string(),
        };
        
        // Test saving a draft
        let draft_id = service.save_draft("test_account", &compose_data).await.unwrap();
        assert!(!draft_id.is_empty());
        
        // Test deleting a draft
        assert!(service.delete_draft("test_account", &draft_id).await.is_ok());
    }
}