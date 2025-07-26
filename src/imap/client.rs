use crate::imap::{
    ImapConnection, ImapConfig, ImapError, ImapResult, ImapCapability, 
    ImapFolder, ImapMessage, MessageFlag, SearchCriteria, ImapAuthMethod,
    IdleNotification, IdleNotificationService
};
use crate::oauth2::TokenManager;
use crate::imap::connection::ConnectionState;
use crate::imap::protocol::ImapProtocol;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// High-level IMAP client
pub struct ImapClient {
    connection: ImapConnection,
    capabilities: Vec<ImapCapability>,
    selected_folder: Option<String>,
    folders_cache: HashMap<String, ImapFolder>,
    token_manager: Option<TokenManager>,
    idle_service: Option<Arc<IdleNotificationService>>,
}

impl ImapClient {
    /// Create a new IMAP client
    pub fn new(config: ImapConfig) -> Self {
        Self {
            connection: ImapConnection::new(config),
            capabilities: Vec::new(),
            selected_folder: None,
            folders_cache: HashMap::new(),
            token_manager: None,
            idle_service: None,
        }
    }
    
    /// Create a new IMAP client with OAuth2 token manager
    pub fn new_with_oauth2(config: ImapConfig, token_manager: TokenManager) -> Self {
        Self {
            connection: ImapConnection::new(config),
            capabilities: Vec::new(),
            selected_folder: None,
            folders_cache: HashMap::new(),
            token_manager: Some(token_manager),
            idle_service: None,
        }
    }
    
    /// Connect to the IMAP server
    pub async fn connect(&mut self) -> ImapResult<()> {
        self.connection.connect().await?;
        
        // Fetch server capabilities
        self.capabilities = self.get_capabilities().await?;
        
        Ok(())
    }
    
    /// Disconnect from the IMAP server
    pub async fn disconnect(&mut self) -> ImapResult<()> {
        self.connection.disconnect().await?;
        self.capabilities.clear();
        self.selected_folder = None;
        self.folders_cache.clear();
        Ok(())
    }
    
    /// Check if connected to server
    pub fn is_connected(&self) -> bool {
        self.connection.is_connected()
    }
    
    /// Check if authenticated with server
    pub fn is_authenticated(&self) -> bool {
        self.connection.is_authenticated()
    }
    
    /// Authenticate with the server
    pub async fn authenticate(&mut self) -> ImapResult<()> {
        if !self.connection.is_connected() {
            return Err(ImapError::invalid_state("Not connected"));
        }
        
        if self.connection.is_authenticated() {
            return Ok(()); // Already authenticated
        }
        
        let config = self.connection.config();
        
        match &config.auth_method {
            ImapAuthMethod::OAuth2 { account_id } => {
                // Use OAuth2 XOAUTH2 authentication
                tracing::info!("Using OAuth2 XOAUTH2 authentication for account: {}", account_id);
                
                if !self.capabilities.contains(&ImapCapability::AuthXOAuth2) {
                    tracing::error!("Server does not support XOAUTH2. Available capabilities: {:?}", self.capabilities);
                    return Err(ImapError::authentication("Server does not support XOAUTH2"));
                }
                
                let token_manager = self.token_manager.as_ref()
                    .ok_or_else(|| {
                        tracing::error!("Token manager not configured for OAuth2 authentication");
                        ImapError::authentication("Token manager not configured for OAuth2")
                    })?;
                
                tracing::debug!("Creating XOAUTH2 string for user: {}", config.username);
                let xoauth2_string = token_manager
                    .create_xoauth2_string(account_id, &config.username)
                    .await
                    .map_err(|e| {
                        tracing::error!("Failed to create XOAUTH2 string for account {}: {}", account_id, e);
                        ImapError::authentication(&format!("Failed to create XOAUTH2 string: {}", e))
                    })?;
                
                let command = ImapProtocol::format_authenticate_xoauth2(&xoauth2_string);
                tracing::debug!("Sending XOAUTH2 authentication command");
                let response = self.connection.send_command(&command).await
                    .map_err(|e| {
                        tracing::error!("XOAUTH2 authentication command failed: {}", e);
                        e
                    })?;
                tracing::info!("XOAUTH2 authentication successful. Response: {}", response.lines().next().unwrap_or(""));
            }
            ImapAuthMethod::Password(password) => {
                // Use traditional password authentication
                if self.capabilities.contains(&ImapCapability::AuthPlain) {
                    let command = ImapProtocol::format_authenticate_plain(&config.username, password)?;
                    let _response = self.connection.send_command(&command).await?;
                } else {
                    // Fall back to LOGIN
                    let command = ImapProtocol::format_login(&config.username, password);
                    let _response = self.connection.send_command(&command).await?;
                }
            }
        }
        
        self.connection.set_state(ConnectionState::Authenticated);
        Ok(())
    }
    
    /// Get server capabilities
    pub async fn get_capabilities(&mut self) -> ImapResult<Vec<ImapCapability>> {
        let response = self.connection.send_command("CAPABILITY").await?;
        ImapProtocol::parse_capabilities(&response)
    }
    
    /// List folders
    pub async fn list_folders(&mut self, reference: &str, pattern: &str) -> ImapResult<Vec<ImapFolder>> {
        if !self.connection.is_authenticated() {
            return Err(ImapError::invalid_state("Not authenticated"));
        }
        
        let command = ImapProtocol::format_list(reference, pattern);
        let response = self.connection.send_command(&command).await?;
        let folders = ImapProtocol::parse_folders(&response)?;
        
        // Update cache
        for folder in &folders {
            self.folders_cache.insert(folder.full_name.clone(), folder.clone());
        }
        
        Ok(folders)
    }
    
    /// List subscribed folders
    pub async fn list_subscribed_folders(&mut self, reference: &str, pattern: &str) -> ImapResult<Vec<ImapFolder>> {
        if !self.connection.is_authenticated() {
            return Err(ImapError::invalid_state("Not authenticated"));
        }
        
        let command = ImapProtocol::format_lsub(reference, pattern);
        let response = self.connection.send_command(&command).await?;
        ImapProtocol::parse_folders(&response)
    }
    
    /// Select a folder for operations
    pub async fn select_folder(&mut self, folder_name: &str) -> ImapResult<ImapFolder> {
        if !self.connection.is_authenticated() {
            return Err(ImapError::invalid_state("Not authenticated"));
        }
        
        let command = ImapProtocol::format_select(folder_name);
        let response = self.connection.send_command(&command).await?;
        let mut folder = ImapProtocol::parse_select_response(&response)?;
        
        // Update folder with name information
        folder.name = folder_name.to_string();
        folder.full_name = folder_name.to_string();
        
        // Update connection state
        self.connection.set_state(ConnectionState::Selected(folder_name.to_string()));
        self.selected_folder = Some(folder_name.to_string());
        
        // Update cache
        self.folders_cache.insert(folder_name.to_string(), folder.clone());
        
        Ok(folder)
    }
    
    /// Examine a folder (read-only)
    pub async fn examine_folder(&mut self, folder_name: &str) -> ImapResult<ImapFolder> {
        if !self.connection.is_authenticated() {
            return Err(ImapError::invalid_state("Not authenticated"));
        }
        
        let command = ImapProtocol::format_examine(folder_name);
        let response = self.connection.send_command(&command).await?;
        let mut folder = ImapProtocol::parse_select_response(&response)?;
        
        folder.name = folder_name.to_string();
        folder.full_name = folder_name.to_string();
        
        Ok(folder)
    }
    
    /// Fetch messages by sequence numbers
    pub async fn fetch_messages(&mut self, sequence_set: &str, items: &[&str]) -> ImapResult<Vec<ImapMessage>> {
        if self.selected_folder.is_none() {
            return Err(ImapError::invalid_state("No folder selected"));
        }
        
        let command = ImapProtocol::format_fetch(sequence_set, items);
        let response = self.connection.send_command(&command).await?;
        ImapProtocol::parse_fetch_response(&response)
    }
    
    /// Fetch messages by UIDs
    pub async fn uid_fetch_messages(&mut self, uid_set: &str, items: &[&str]) -> ImapResult<Vec<ImapMessage>> {
        if self.selected_folder.is_none() {
            return Err(ImapError::invalid_state("No folder selected"));
        }
        
        let command = ImapProtocol::format_uid_fetch(uid_set, items);
        let response = self.connection.send_command(&command).await?;
        ImapProtocol::parse_fetch_response(&response)
    }
    
    /// Search for messages
    pub async fn search(&mut self, criteria: &SearchCriteria) -> ImapResult<Vec<u32>> {
        if self.selected_folder.is_none() {
            return Err(ImapError::invalid_state("No folder selected"));
        }
        
        let command = ImapProtocol::format_search(criteria);
        let response = self.connection.send_command(&command).await?;
        ImapProtocol::parse_search_response(&response)
    }
    
    /// Search for messages by UID
    pub async fn uid_search(&mut self, criteria: &SearchCriteria) -> ImapResult<Vec<u32>> {
        if self.selected_folder.is_none() {
            return Err(ImapError::invalid_state("No folder selected"));
        }
        
        let command = ImapProtocol::format_uid_search(criteria);
        let response = self.connection.send_command(&command).await?;
        ImapProtocol::parse_search_response(&response)
    }
    
    /// Set message flags
    pub async fn store_flags(&mut self, sequence_set: &str, flags: &[MessageFlag], replace: bool) -> ImapResult<()> {
        if self.selected_folder.is_none() {
            return Err(ImapError::invalid_state("No folder selected"));
        }
        
        let action = if replace { "FLAGS" } else { "+FLAGS" };
        let command = ImapProtocol::format_store(sequence_set, flags, action);
        let _response = self.connection.send_command(&command).await?;
        Ok(())
    }
    
    /// Set message flags by UID
    pub async fn uid_store_flags(&mut self, uid_set: &str, flags: &[MessageFlag], replace: bool) -> ImapResult<()> {
        if self.selected_folder.is_none() {
            return Err(ImapError::invalid_state("No folder selected"));
        }
        
        let action = if replace { "FLAGS" } else { "+FLAGS" };
        let command = ImapProtocol::format_uid_store(uid_set, flags, action);
        let _response = self.connection.send_command(&command).await?;
        Ok(())
    }
    
    /// Remove message flags
    pub async fn remove_flags(&mut self, sequence_set: &str, flags: &[MessageFlag]) -> ImapResult<()> {
        if self.selected_folder.is_none() {
            return Err(ImapError::invalid_state("No folder selected"));
        }
        
        let command = ImapProtocol::format_store(sequence_set, flags, "-FLAGS");
        let _response = self.connection.send_command(&command).await?;
        Ok(())
    }
    
    /// Remove message flags by UID
    pub async fn uid_remove_flags(&mut self, uid_set: &str, flags: &[MessageFlag]) -> ImapResult<()> {
        if self.selected_folder.is_none() {
            return Err(ImapError::invalid_state("No folder selected"));
        }
        
        let command = ImapProtocol::format_uid_store(uid_set, flags, "-FLAGS");
        let _response = self.connection.send_command(&command).await?;
        Ok(())
    }
    
    /// Copy messages to another folder
    pub async fn copy_messages(&mut self, sequence_set: &str, destination: &str) -> ImapResult<()> {
        if self.selected_folder.is_none() {
            return Err(ImapError::invalid_state("No folder selected"));
        }
        
        let command = ImapProtocol::format_copy(sequence_set, destination);
        let _response = self.connection.send_command(&command).await?;
        Ok(())
    }
    
    /// Copy messages by UID to another folder
    pub async fn uid_copy_messages(&mut self, uid_set: &str, destination: &str) -> ImapResult<()> {
        if self.selected_folder.is_none() {
            return Err(ImapError::invalid_state("No folder selected"));
        }
        
        let command = ImapProtocol::format_uid_copy(uid_set, destination);
        let _response = self.connection.send_command(&command).await?;
        Ok(())
    }
    
    /// Expunge deleted messages
    pub async fn expunge(&mut self) -> ImapResult<()> {
        if self.selected_folder.is_none() {
            return Err(ImapError::invalid_state("No folder selected"));
        }
        
        let command = ImapProtocol::format_expunge();
        let _response = self.connection.send_command(&command).await?;
        Ok(())
    }
    
    /// Create a new folder
    pub async fn create_folder(&mut self, folder_name: &str) -> ImapResult<()> {
        if !self.connection.is_authenticated() {
            return Err(ImapError::invalid_state("Not authenticated"));
        }
        
        let command = ImapProtocol::format_create(folder_name);
        let _response = self.connection.send_command(&command).await?;
        Ok(())
    }
    
    /// Delete a folder
    pub async fn delete_folder(&mut self, folder_name: &str) -> ImapResult<()> {
        if !self.connection.is_authenticated() {
            return Err(ImapError::invalid_state("Not authenticated"));
        }
        
        let command = ImapProtocol::format_delete(folder_name);
        let _response = self.connection.send_command(&command).await?;
        
        // Remove from cache
        self.folders_cache.remove(folder_name);
        
        Ok(())
    }
    
    /// Rename a folder
    pub async fn rename_folder(&mut self, old_name: &str, new_name: &str) -> ImapResult<()> {
        if !self.connection.is_authenticated() {
            return Err(ImapError::invalid_state("Not authenticated"));
        }
        
        let command = ImapProtocol::format_rename(old_name, new_name);
        let _response = self.connection.send_command(&command).await?;
        
        // Update cache
        if let Some(folder) = self.folders_cache.remove(old_name) {
            let mut updated_folder = folder;
            updated_folder.name = new_name.to_string();
            updated_folder.full_name = new_name.to_string();
            self.folders_cache.insert(new_name.to_string(), updated_folder);
        }
        
        Ok(())
    }
    
    /// Subscribe to a folder
    pub async fn subscribe_folder(&mut self, folder_name: &str) -> ImapResult<()> {
        if !self.connection.is_authenticated() {
            return Err(ImapError::invalid_state("Not authenticated"));
        }
        
        let command = ImapProtocol::format_subscribe(folder_name);
        let _response = self.connection.send_command(&command).await?;
        Ok(())
    }
    
    /// Unsubscribe from a folder
    pub async fn unsubscribe_folder(&mut self, folder_name: &str) -> ImapResult<()> {
        if !self.connection.is_authenticated() {
            return Err(ImapError::invalid_state("Not authenticated"));
        }
        
        let command = ImapProtocol::format_unsubscribe(folder_name);
        let _response = self.connection.send_command(&command).await?;
        Ok(())
    }
    
    /// Get folder status
    pub async fn get_folder_status(&mut self, folder_name: &str, items: &[&str]) -> ImapResult<ImapFolder> {
        if !self.connection.is_authenticated() {
            return Err(ImapError::invalid_state("Not authenticated"));
        }
        
        let command = ImapProtocol::format_status(folder_name, items);
        let _response = self.connection.send_command(&command).await?;
        
        // Parse STATUS response (simplified)
        let folder = ImapFolder::new(folder_name.to_string(), folder_name.to_string());
        
        // TODO: Parse STATUS response properly
        // For now, return basic folder info
        
        Ok(folder)
    }
    
    /// Initialize IDLE notification service
    pub fn init_idle_service(&mut self) -> ImapResult<()> {
        if self.idle_service.is_some() {
            return Ok(()); // Already initialized
        }
        
        // Create a shared connection wrapper for IDLE
        let connection = Arc::new(Mutex::new(ImapConnection::new(self.connection.config().clone())));
        let idle_service = Arc::new(IdleNotificationService::new(connection));
        
        self.idle_service = Some(idle_service);
        Ok(())
    }
    
    /// Start monitoring a folder for real-time updates
    pub async fn start_folder_monitoring(&mut self, folder_name: String) -> ImapResult<()> {
        if !self.capabilities.contains(&ImapCapability::Idle) {
            return Err(ImapError::not_supported("IDLE not supported by server"));
        }
        
        // Initialize IDLE service if not already done
        if self.idle_service.is_none() {
            self.init_idle_service()?;
        }
        
        let idle_service = self.idle_service.as_ref()
            .ok_or_else(|| ImapError::invalid_state("IDLE service not initialized"))?;
        
        idle_service.start_monitoring(folder_name).await?;
        Ok(())
    }
    
    /// Stop folder monitoring
    pub async fn stop_folder_monitoring(&mut self) -> ImapResult<()> {
        if let Some(idle_service) = &self.idle_service {
            idle_service.stop_monitoring().await?;
        }
        Ok(())
    }
    
    /// Add a callback for IDLE notifications
    pub async fn add_idle_callback<F>(&self, callback: F) -> ImapResult<()> 
    where
        F: Fn(IdleNotification) + Send + Sync + 'static,
    {
        if let Some(idle_service) = &self.idle_service {
            idle_service.add_callback(callback).await;
            Ok(())
        } else {
            Err(ImapError::invalid_state("IDLE service not initialized"))
        }
    }
    
    /// Get IDLE statistics
    pub async fn get_idle_stats(&self) -> Option<crate::imap::IdleStats> {
        if let Some(idle_service) = &self.idle_service {
            Some(idle_service.get_stats().await)
        } else {
            None
        }
    }
    
    /// Start IDLE mode (legacy method - kept for compatibility)
    pub async fn idle(&mut self) -> ImapResult<()> {
        if !self.capabilities.contains(&ImapCapability::Idle) {
            return Err(ImapError::not_supported("IDLE not supported by server"));
        }
        
        if self.selected_folder.is_none() {
            return Err(ImapError::invalid_state("No folder selected"));
        }
        
        let command = ImapProtocol::format_idle();
        let _response = self.connection.send_command(&command).await?;
        Ok(())
    }
    
    /// Stop IDLE mode (legacy method - kept for compatibility)
    pub async fn done(&mut self) -> ImapResult<()> {
        let command = ImapProtocol::format_done();
        let _response = self.connection.send_command(&command).await?;
        Ok(())
    }
    
    /// Get connection state
    pub fn connection_state(&self) -> &ConnectionState {
        self.connection.state()
    }
    
    
    /// Get selected folder name
    pub fn selected_folder(&self) -> Option<&String> {
        self.selected_folder.as_ref()
    }
    
    
    /// Get server greeting
    pub fn greeting(&self) -> Option<&String> {
        self.connection.greeting()
    }
    
    /// Get cached folder information
    pub fn get_cached_folder(&self, folder_name: &str) -> Option<&ImapFolder> {
        self.folders_cache.get(folder_name)
    }
    
    /// Get all cached folders
    pub fn cached_folders(&self) -> impl Iterator<Item = &ImapFolder> {
        self.folders_cache.values()
    }
    
    /// Get client capabilities (for testing)
    pub fn capabilities(&self) -> &[ImapCapability] {
        &self.capabilities
    }
    
    /// Set capabilities (for testing)
    pub fn set_capabilities(&mut self, capabilities: Vec<ImapCapability>) {
        self.capabilities = capabilities;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_client_creation() {
        let config = ImapConfig::new(
            "imap.example.com".to_string(),
            993,
            "user@example.com".to_string(),
            "password".to_string(),
        );
        
        let client = ImapClient::new(config);
        assert!(!client.is_connected());
        assert!(!client.is_authenticated());
        assert!(client.selected_folder().is_none());
        assert_eq!(client.capabilities().len(), 0);
    }
}