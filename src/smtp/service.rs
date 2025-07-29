use crate::smtp::{
    SmtpClient, SmtpProviderRegistry, SmtpResult, SmtpError, SendResult, EmailMessage
};
use crate::ui::EmailComposeData;
use crate::oauth2::TokenManager;
use crate::email::{EmailDatabase, database::StoredDraft};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// SMTP service for managing email sending across multiple accounts
#[derive(Clone)]
pub struct SmtpService {
    provider_registry: SmtpProviderRegistry,
    clients: Arc<RwLock<HashMap<String, SmtpClient>>>,
    token_manager: Arc<TokenManager>,
    database: Arc<EmailDatabase>,
}

impl SmtpService {
    /// Create a new SMTP service
    pub fn new(token_manager: Arc<TokenManager>, database: Arc<EmailDatabase>) -> Self {
        Self {
            provider_registry: SmtpProviderRegistry::new(),
            clients: Arc::new(RwLock::new(HashMap::new())),
            token_manager,
            database,
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
    
    /// Send RSVP response for calendar invitation
    pub async fn send_rsvp_response(
        &self,
        account_id: &str,
        from_address: &str,
        organizer_email: &str,
        meeting_title: &str,
        meeting_uid: &str,
        response: &str, // "ACCEPTED", "DECLINED", "TENTATIVE"
        comment: Option<String>,
        original_request_ical: &str,
    ) -> SmtpResult<SendResult> {
        // Create RSVP response email
        let rsvp_message = EmailMessage::create_rsvp_response(
            from_address.to_string(),
            organizer_email.to_string(),
            meeting_title,
            meeting_uid,
            response,
            comment,
            original_request_ical,
        )?;
        
        // Send the RSVP response
        self.send_message(account_id, &rsvp_message).await
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
    
    /// Save draft email to database
    pub async fn save_draft(
        &self,
        account_id: &str,
        compose_data: &EmailComposeData,
    ) -> SmtpResult<String> {
        // Basic validation (don't require all fields for drafts)
        if compose_data.subject.is_empty() && compose_data.body.is_empty() {
            return Err(SmtpError::MessageFormatError("Draft must have either subject or body".to_string()));
        }
        
        // Create draft from compose data
        let draft = StoredDraft::from_compose_data(account_id.to_string(), compose_data, false);
        let draft_id = draft.id.clone();
        
        // Save to database
        self.database.save_draft(&draft).await
            .map_err(|e| SmtpError::InvalidConfig(format!("Failed to save draft: {}", e)))?;
        
        tracing::info!("Draft saved for account {}: {}", account_id, draft_id);
        Ok(draft_id)
    }
    
    /// Load draft email from database
    pub async fn load_draft(&self, account_id: &str, draft_id: &str) -> SmtpResult<EmailComposeData> {
        match self.database.load_draft(draft_id).await {
            Ok(Some(draft)) => {
                // Verify the draft belongs to the correct account
                if draft.account_id != account_id {
                    return Err(SmtpError::InvalidConfig(format!("Draft {} does not belong to account {}", draft_id, account_id)));
                }
                Ok(draft.to_compose_data())
            }
            Ok(None) => {
                Err(SmtpError::InvalidConfig(format!("Draft not found: {}", draft_id)))
            }
            Err(e) => {
                Err(SmtpError::InvalidConfig(format!("Failed to load draft: {}", e)))
            }
        }
    }
    
    /// Delete draft email from database
    pub async fn delete_draft(&self, account_id: &str, draft_id: &str) -> SmtpResult<()> {
        // First verify the draft exists and belongs to the account
        match self.database.load_draft(draft_id).await {
            Ok(Some(draft)) => {
                if draft.account_id != account_id {
                    return Err(SmtpError::InvalidConfig(format!("Draft {} does not belong to account {}", draft_id, account_id)));
                }
            }
            Ok(None) => {
                return Err(SmtpError::InvalidConfig(format!("Draft not found: {}", draft_id)));
            }
            Err(e) => {
                return Err(SmtpError::InvalidConfig(format!("Failed to verify draft: {}", e)));
            }
        }
        
        // Delete the draft
        match self.database.delete_draft(draft_id).await {
            Ok(true) => {
                tracing::info!("Draft deleted for account {}: {}", account_id, draft_id);
                Ok(())
            }
            Ok(false) => {
                Err(SmtpError::InvalidConfig(format!("Draft {} was not found during deletion", draft_id)))
            }
            Err(e) => {
                Err(SmtpError::InvalidConfig(format!("Failed to delete draft: {}", e)))
            }
        }
    }
    
    /// List all drafts for an account
    pub async fn list_drafts(&self, account_id: &str) -> SmtpResult<Vec<StoredDraft>> {
        self.database.load_drafts_for_account(account_id).await
            .map_err(|e| SmtpError::InvalidConfig(format!("Failed to list drafts: {}", e)))
    }
    
    /// Update an existing draft with new compose data
    pub async fn update_draft(
        &self,
        account_id: &str,
        draft_id: &str,
        compose_data: &EmailComposeData,
        auto_saved: bool,
    ) -> SmtpResult<()> {
        // Load existing draft to verify ownership
        let mut draft = match self.database.load_draft(draft_id).await {
            Ok(Some(draft)) => {
                if draft.account_id != account_id {
                    return Err(SmtpError::InvalidConfig(format!("Draft {} does not belong to account {}", draft_id, account_id)));
                }
                draft
            }
            Ok(None) => {
                return Err(SmtpError::InvalidConfig(format!("Draft not found: {}", draft_id)));
            }
            Err(e) => {
                return Err(SmtpError::InvalidConfig(format!("Failed to load draft: {}", e)));
            }
        };
        
        // Update with new data
        draft.update_from_compose_data(compose_data, auto_saved);
        
        // Save updated draft
        self.database.save_draft(&draft).await
            .map_err(|e| SmtpError::InvalidConfig(format!("Failed to update draft: {}", e)))?;
        
        tracing::info!("Draft updated for account {}: {} (auto_saved: {})", account_id, draft_id, auto_saved);
        Ok(())
    }
    
    /// Auto-save draft with compose data
    pub async fn auto_save_draft(
        &self,
        account_id: &str,
        compose_data: &EmailComposeData,
        existing_draft_id: Option<&str>,
    ) -> SmtpResult<String> {
        if let Some(draft_id) = existing_draft_id {
            // Update existing draft
            self.update_draft(account_id, draft_id, compose_data, true).await?;
            Ok(draft_id.to_string())
        } else {
            // Create new auto-saved draft
            let mut draft = StoredDraft::from_compose_data(account_id.to_string(), compose_data, true);
            draft.auto_saved = true;
            let draft_id = draft.id.clone();
            
            self.database.save_draft(&draft).await
                .map_err(|e| SmtpError::InvalidConfig(format!("Failed to auto-save draft: {}", e)))?;
            
            tracing::debug!("Auto-saved draft for account {}: {}", account_id, draft_id);
            Ok(draft_id)
        }
    }
    
    /// Clean up old auto-saved drafts (call periodically)
    pub async fn cleanup_old_auto_saved_drafts(&self, older_than_hours: i64) -> SmtpResult<u64> {
        self.database.cleanup_old_auto_saved_drafts(older_than_hours).await
            .map_err(|e| SmtpError::InvalidConfig(format!("Failed to cleanup drafts: {}", e)))
    }
}

/// Builder for creating SMTP service configurations
pub struct SmtpServiceBuilder {
    token_manager: Option<Arc<TokenManager>>,
    database: Option<Arc<EmailDatabase>>,
}

impl SmtpServiceBuilder {
    pub fn new() -> Self {
        Self {
            token_manager: None,
            database: None,
        }
    }
    
    pub fn with_token_manager(mut self, token_manager: Arc<TokenManager>) -> Self {
        self.token_manager = Some(token_manager);
        self
    }
    
    pub fn with_database(mut self, database: Arc<EmailDatabase>) -> Self {
        self.database = Some(database);
        self
    }
    
    pub fn build(self) -> SmtpResult<SmtpService> {
        let token_manager = self.token_manager
            .ok_or_else(|| SmtpError::InvalidConfig("Token manager is required".to_string()))?;
        let database = self.database
            .ok_or_else(|| SmtpError::InvalidConfig("Database is required".to_string()))?;
        
        Ok(SmtpService::new(token_manager, database))
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
        let token_manager = Arc::new(TokenManager::new());
        let database = Arc::new(EmailDatabase::new_in_memory().await.unwrap());
        let service = SmtpService::new(token_manager, database);
        
        // Check that service starts with no configured accounts
        assert_eq!(service.get_configured_accounts().await.len(), 0);
        assert!(!service.is_account_configured("test_account").await);
    }
    
    #[tokio::test]
    async fn test_provider_detection() {
        let token_manager = Arc::new(TokenManager::new());
        let database = Arc::new(EmailDatabase::new_in_memory().await.unwrap());
        let service = SmtpService::new(token_manager, database);
        
        assert_eq!(service.detect_provider_from_email("user@gmail.com"), Some("gmail".to_string()));
        assert_eq!(service.detect_provider_from_email("user@outlook.com"), Some("outlook".to_string()));
        assert_eq!(service.detect_provider_from_email("user@unknown.com"), None);
    }
    
    #[tokio::test]
    async fn test_draft_operations() {
        let token_manager = Arc::new(TokenManager::new());
        let database = Arc::new(EmailDatabase::new_in_memory().await.unwrap());
        let service = SmtpService::new(token_manager, database);
        
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