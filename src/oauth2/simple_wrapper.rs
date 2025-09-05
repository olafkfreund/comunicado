use crate::oauth2::{OAuth2Error, launch_simple_setup, AuthType, SecurityType};
use crate::ui::account_setup::OAuth2SetupWizard;
use crate::theme::Theme;
use async_trait::async_trait;

/// Wrapper to integrate SimpleSetupWizard with AccountSetupManager
pub struct SimpleSetupWrapper {
    theme: Theme,
}

impl SimpleSetupWrapper {
    pub fn new(theme: Theme) -> Self {
        Self { theme }
    }
}

#[async_trait]
impl OAuth2SetupWizard for SimpleSetupWrapper {
    async fn run(&mut self) -> anyhow::Result<Option<crate::oauth2::AccountConfig>> {
        match launch_simple_setup(self.theme.clone()).await {
            Ok(Some(account_id)) => {
                // For now, return a placeholder account config
                // In real implementation, we'd load the created account
                tracing::info!("Simple setup completed for account: {}", account_id);
                Ok(Some(create_placeholder_account(&account_id)))
            }
            Ok(None) => Ok(None),
            Err(OAuth2Error::StorageError(msg)) if msg.contains("cancelled") => Ok(None),
            Err(e) => Err(anyhow::anyhow!("Simple setup failed: {}", e)),
        }
    }
    
    fn is_configured(&self) -> bool {
        true // Simple setup is always available
    }
    
    fn supported_providers(&self) -> Vec<String> {
        vec![
            "Gmail (Quick Setup)".to_string(),
            "Outlook (Quick Setup)".to_string(),
        ]
    }
}

fn create_placeholder_account(account_id: &str) -> crate::oauth2::AccountConfig {
    use crate::oauth2::AccountConfig;
    use chrono::Utc;
    
    let (email, provider, imap_server, smtp_server) = if account_id.contains("gmail") {
        ("user@gmail.com", "gmail", "imap.gmail.com", "smtp.gmail.com")
    } else if account_id.contains("outlook") {
        ("user@outlook.com", "outlook", "outlook.office365.com", "smtp-mail.outlook.com")
    } else {
        ("user@example.com", "unknown", "imap.example.com", "smtp.example.com")
    };

    AccountConfig {
        account_id: account_id.to_string(),
        display_name: "Quick Setup Account".to_string(),
        email_address: email.to_string(),
        provider: provider.to_string(),
        auth_type: AuthType::OAuth2,
        imap_server: imap_server.to_string(),
        imap_port: 993,
        smtp_server: smtp_server.to_string(),
        smtp_port: 587,
        security: SecurityType::SSL,
        access_token: "placeholder_access_token".to_string(),
        refresh_token: Some("placeholder_refresh_token".to_string()),
        token_expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
        scopes: vec![
            "https://www.googleapis.com/auth/gmail.readonly".to_string(),
            "https://www.googleapis.com/auth/gmail.modify".to_string(),
        ],
    }
}