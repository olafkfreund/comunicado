//! Account setup abstractions
//!
//! This module provides trait abstractions for account setup operations,
//! allowing the onboarding flow to be decoupled from specific implementations.

use anyhow::Result;
use async_trait::async_trait;
use crate::oauth2::AccountConfig as Account;
use crate::theme::Theme;

/// Trait for account setup operations
#[async_trait]
pub trait AccountSetupProvider {
    /// Setup a new account through the provider's flow
    async fn setup_account(&mut self) -> Result<Option<Account>>;
    
    /// Check if the provider is available
    fn is_available(&self) -> bool;
    
    /// Get the provider's display name
    fn provider_name(&self) -> &'static str;
    
    /// Get supported account types
    fn supported_types(&self) -> Vec<&'static str>;
}

/// OAuth2-based account setup provider
pub struct OAuth2AccountProvider {
    inner: Box<dyn OAuth2SetupWizard>,
}

/// Trait abstraction for OAuth2 setup wizards
#[async_trait]
pub trait OAuth2SetupWizard: Send {
    /// Run the OAuth2 setup flow
    async fn run(&mut self) -> Result<Option<Account>>;
    
    /// Check if OAuth2 is configured
    fn is_configured(&self) -> bool;
    
    /// Get supported providers
    fn supported_providers(&self) -> Vec<String>;
}

impl OAuth2AccountProvider {
    /// Create a new OAuth2 account provider
    pub fn new(wizard: Box<dyn OAuth2SetupWizard>) -> Self {
        Self {
            inner: wizard,
        }
    }
    
    /// Create with default OAuth2 wizard
    pub fn with_default() -> Result<Self> {
        // This would be injected or configured
        let wizard = crate::oauth2::SetupWizard::new()?;
        Ok(Self::new(Box::new(DefaultOAuth2Wrapper::new(wizard))))
    }
}

#[async_trait]
impl AccountSetupProvider for OAuth2AccountProvider {
    async fn setup_account(&mut self) -> Result<Option<Account>> {
        self.inner.run().await
    }
    
    fn is_available(&self) -> bool {
        self.inner.is_configured()
    }
    
    fn provider_name(&self) -> &'static str {
        "OAuth2"
    }
    
    fn supported_types(&self) -> Vec<&'static str> {
        vec!["Gmail", "Outlook", "Yahoo", "Custom OAuth2"]
    }
}

/// Wrapper to adapt the existing SetupWizard to our trait
struct DefaultOAuth2Wrapper {
    wizard: crate::oauth2::SetupWizard,
}

impl DefaultOAuth2Wrapper {
    fn new(wizard: crate::oauth2::SetupWizard) -> Self {
        Self { wizard }
    }
}

#[async_trait]
impl OAuth2SetupWizard for DefaultOAuth2Wrapper {
    async fn run(&mut self) -> Result<Option<Account>> {
        match self.wizard.run().await {
            Ok(Some(account)) => Ok(Some(account)),
            Ok(None) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("OAuth2 setup failed: {}", e)),
        }
    }
    
    fn is_configured(&self) -> bool {
        // Check if OAuth2 credentials are available
        true // Simplified for now
    }
    
    fn supported_providers(&self) -> Vec<String> {
        vec![
            "Google".to_string(),
            "Microsoft".to_string(), 
            "Yahoo".to_string(),
        ]
    }
}

/// Simplified account setup provider
pub struct SimplifiedAccountProvider {
    theme: Theme,
}

impl SimplifiedAccountProvider {
    pub fn new(theme: Theme) -> Self {
        Self { theme }
    }
}

#[async_trait]
impl AccountSetupProvider for SimplifiedAccountProvider {
    async fn setup_account(&mut self) -> Result<Option<Account>> {
        let wrapper = crate::oauth2::simple_wrapper::SimpleSetupWrapper::new(self.theme.clone());
        let mut boxed_wrapper: Box<dyn OAuth2SetupWizard> = Box::new(wrapper);
        boxed_wrapper.run().await
    }
    
    fn is_available(&self) -> bool {
        true
    }
    
    fn provider_name(&self) -> &'static str {
        "Quick Setup"
    }
    
    fn supported_types(&self) -> Vec<&'static str> {
        vec!["Gmail (1-Click)", "Outlook (1-Click)", "Auto-Detect"]
    }
}

/// Manual account setup provider
pub struct ManualAccountProvider;

impl ManualAccountProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AccountSetupProvider for ManualAccountProvider {
    async fn setup_account(&mut self) -> Result<Option<Account>> {
        // This would show a manual configuration form
        // For now, return None to indicate cancellation
        Ok(None)
    }
    
    fn is_available(&self) -> bool {
        true
    }
    
    fn provider_name(&self) -> &'static str {
        "Manual Setup"
    }
    
    fn supported_types(&self) -> Vec<&'static str> {
        vec!["IMAP", "POP3", "Exchange"]
    }
}

/// Multi-provider account setup manager
pub struct AccountSetupManager {
    providers: Vec<Box<dyn AccountSetupProvider>>,
    current_provider: usize,
}

impl AccountSetupManager {
    /// Create a new account setup manager
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            current_provider: 0,
        }
    }
    
    /// Add a provider to the manager
    pub fn add_provider(&mut self, provider: Box<dyn AccountSetupProvider>) {
        self.providers.push(provider);
    }
    
    /// Create with default providers
    pub fn with_defaults() -> Result<Self> {
        let mut manager = Self::new();
        
        // Add simplified provider first (highest priority)
        let theme = Theme::default();
        manager.add_provider(Box::new(SimplifiedAccountProvider::new(theme)));
        
        // Add OAuth2 provider if available
        if let Ok(oauth2_provider) = OAuth2AccountProvider::with_default() {
            manager.add_provider(Box::new(oauth2_provider));
        }
        
        // Add manual provider as fallback
        manager.add_provider(Box::new(ManualAccountProvider::new()));
        
        Ok(manager)
    }
    
    /// Get available providers
    pub fn available_providers(&self) -> Vec<&str> {
        self.providers.iter()
            .filter(|p| p.is_available())
            .map(|p| p.provider_name())
            .collect()
    }
    
    /// Select provider by index
    pub fn select_provider(&mut self, index: usize) -> Result<()> {
        if index >= self.providers.len() {
            anyhow::bail!("Provider index {} out of range", index);
        }
        self.current_provider = index;
        Ok(())
    }
    
    /// Setup account using current provider
    pub async fn setup_current(&mut self) -> Result<Option<Account>> {
        if let Some(provider) = self.providers.get_mut(self.current_provider) {
            if provider.is_available() {
                return provider.setup_account().await;
            }
        }
        Ok(None)
    }
    
    /// Try all providers until one succeeds
    pub async fn setup_with_fallback(&mut self) -> Result<Option<Account>> {
        for i in 0..self.providers.len() {
            self.current_provider = i;
            if let Ok(Some(account)) = self.setup_current().await {
                return Ok(Some(account));
            }
        }
        Ok(None)
    }
}

impl Default for AccountSetupManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_manual_provider() {
        let mut provider = ManualAccountProvider::new();
        assert!(provider.is_available());
        assert_eq!(provider.provider_name(), "Manual Setup");
        
        // Should return None (no actual setup implemented)
        let result = provider.setup_account().await.unwrap();
        assert!(result.is_none());
    }
    
    #[tokio::test]
    async fn test_account_manager() {
        let mut manager = AccountSetupManager::new();
        manager.add_provider(Box::new(ManualAccountProvider::new()));
        
        assert_eq!(manager.available_providers(), vec!["Manual Setup"]);
        
        // Should successfully select provider
        assert!(manager.select_provider(0).is_ok());
        
        // Should fail to select non-existent provider
        assert!(manager.select_provider(10).is_err());
    }
}