use crate::oauth2::{OAuth2Error, OAuth2Result, OAuth2Provider, ProviderConfig, OAuth2Client, AccountConfig};
use serde::Deserialize;
use std::fs;
use std::path::Path;

/// Google OAuth2 credentials file format (downloaded from Google Cloud Console)
#[derive(Debug, Deserialize)]
struct GoogleCredentialsFile {
    installed: GoogleInstalledApp,
}

#[derive(Debug, Deserialize)]
struct GoogleInstalledApp {
    client_id: String,
    project_id: String,
    auth_uri: String,
    token_uri: String,
    auth_provider_x509_cert_url: String,
    client_secret: String,
    redirect_uris: Vec<String>,
}

/// OAuth2 file importer for setting up accounts from credential files
pub struct OAuth2FileImporter;

impl OAuth2FileImporter {
    /// Import Google OAuth2 credentials from a JSON file and perform complete account setup
    pub async fn import_google_credentials(
        file_path: &str,
        email: &str,
        display_name: Option<String>,
    ) -> OAuth2Result<AccountConfig> {
        tracing::info!("Importing Google OAuth2 credentials from file: {}", file_path);
        
        // Read and parse the credentials file
        let credentials = Self::read_google_credentials_file(file_path)?;
        
        // Create provider config from the credentials
        let config = Self::create_provider_config_from_credentials(&credentials)?;
        
        // Create OAuth2 client
        let mut oauth_client = OAuth2Client::new(config)?;
        
        // Start OAuth2 authorization flow
        tracing::info!("Starting OAuth2 authorization for {}", email);
        let auth_request = oauth_client.start_authorization().await?;
        
        // Display authorization instructions to user
        Self::display_authorization_instructions(&auth_request)?;
        
        // Wait for authorization callback
        tracing::info!("Waiting for OAuth2 authorization callback...");
        let auth_code = oauth_client.wait_for_authorization(300).await?; // 5 minute timeout
        
        // Exchange authorization code for tokens
        tracing::info!("Exchanging authorization code for tokens");
        let token_response = oauth_client.exchange_code(&auth_code).await?;
        
        // Create account configuration
        let account_config = oauth_client.create_account_config(
            &token_response, 
            display_name
        ).await?;
        
        tracing::info!("Account setup completed successfully for {}", account_config.email_address);
        Ok(account_config)
    }
    
    /// Read Google credentials file
    fn read_google_credentials_file(file_path: &str) -> OAuth2Result<GoogleCredentialsFile> {
        if !Path::new(file_path).exists() {
            return Err(OAuth2Error::InvalidConfig(
                format!("Credentials file not found: {}", file_path)
            ));
        }
        
        let file_content = fs::read_to_string(file_path)
            .map_err(|e| OAuth2Error::StorageError(
                format!("Failed to read credentials file: {}", e)
            ))?;
        
        let credentials: GoogleCredentialsFile = serde_json::from_str(&file_content)
            .map_err(|e| OAuth2Error::InvalidConfig(
                format!("Invalid credentials file format: {}", e)
            ))?;
        
        Ok(credentials)
    }
    
    /// Create provider config from Google credentials
    fn create_provider_config_from_credentials(credentials: &GoogleCredentialsFile) -> OAuth2Result<ProviderConfig> {
        let config = ProviderConfig::gmail()
            .with_credentials(
                credentials.installed.client_id.clone(),
                Some(credentials.installed.client_secret.clone())
            );
        
        config.validate()?;
        Ok(config)
    }
    
    /// Display authorization instructions to the user
    fn display_authorization_instructions(auth_request: &crate::oauth2::client::AuthorizationRequest) -> OAuth2Result<()> {
        println!("\n🔐 OAuth2 Authorization Required");
        println!("═══════════════════════════════════════════════════════════════");
        println!();
        println!("📋 Steps to authorize Comunicado:");
        println!("   1. Click the URL below (or copy it to your browser)");
        println!("   2. Log in to your Google account");
        println!("   3. Grant permission to Comunicado");
        println!("   4. Return to this terminal");
        println!();
        println!("🌐 Authorization URL:");
        println!("   {}", auth_request.authorization_url);
        println!();
        println!("📡 Callback server running on port: {}", auth_request.callback_port);
        println!("⏱️  Waiting for authorization (timeout: 5 minutes)...");
        println!();
        println!("💡 Tip: If the browser doesn't open automatically, copy the URL above");
        println!("   and paste it into your browser manually.");
        println!();
        
        // Try to open the URL in the browser automatically
        if let Err(e) = Self::open_browser_url(&auth_request.authorization_url) {
            println!("⚠️  Could not open browser automatically: {}", e);
            println!("   Please copy and paste the URL above into your browser.");
        } else {
            println!("✅ Browser should open automatically with the authorization page");
        }
        
        println!();
        
        Ok(())
    }
    
    /// Attempt to open URL in the default browser
    fn open_browser_url(url: &str) -> Result<(), String> {
        use std::process::Command;
        
        let commands = if cfg!(target_os = "linux") {
            vec!["xdg-open", "firefox", "chromium", "google-chrome"]
        } else if cfg!(target_os = "macos") {
            vec!["open"]
        } else if cfg!(target_os = "windows") {
            vec!["start", "explorer"]
        } else {
            return Err("Unsupported platform for automatic browser opening".to_string());
        };
        
        for command in commands {
            match Command::new(command).arg(url).spawn() {
                Ok(_) => {
                    tracing::info!("Successfully opened browser with command: {}", command);
                    return Ok(());
                }
                Err(e) => {
                    tracing::debug!("Failed to open browser with {}: {}", command, e);
                    continue;
                }
            }
        }
        
        Err("Failed to open browser with any available command".to_string())
    }
    
    /// Validate email address format
    pub fn validate_email(email: &str) -> bool {
        email.contains('@') && email.contains('.')
    }
    
    /// Auto-detect provider from email address
    pub fn detect_provider_from_email(email: &str) -> Option<OAuth2Provider> {
        let domain = email.split('@').nth(1)?.to_lowercase();
        
        match domain.as_str() {
            "gmail.com" | "googlemail.com" => Some(OAuth2Provider::Gmail),
            "outlook.com" | "hotmail.com" | "live.com" | "msn.com" => Some(OAuth2Provider::Outlook),
            "yahoo.com" | "yahoo.co.uk" | "yahoo.ca" | "yahoo.au" => Some(OAuth2Provider::Yahoo),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_email_validation() {
        assert!(OAuth2FileImporter::validate_email("test@gmail.com"));
        assert!(OAuth2FileImporter::validate_email("user@example.org"));
        assert!(!OAuth2FileImporter::validate_email("invalid-email"));
        assert!(!OAuth2FileImporter::validate_email("@gmail.com"));
        assert!(!OAuth2FileImporter::validate_email("test@"));
    }
    
    #[test]
    fn test_provider_detection() {
        assert_eq!(
            OAuth2FileImporter::detect_provider_from_email("test@gmail.com"),
            Some(OAuth2Provider::Gmail)
        );
        assert_eq!(
            OAuth2FileImporter::detect_provider_from_email("user@outlook.com"),
            Some(OAuth2Provider::Outlook)
        );
        assert_eq!(
            OAuth2FileImporter::detect_provider_from_email("person@yahoo.com"),
            Some(OAuth2Provider::Yahoo)
        );
        assert_eq!(
            OAuth2FileImporter::detect_provider_from_email("someone@custom.com"),
            None
        );
    }
    
    #[test]
    fn test_google_credentials_parsing() {
        let test_json = r#"{
            "installed": {
                "client_id": "test-client-id.apps.googleusercontent.com",
                "project_id": "test-project",
                "auth_uri": "https://accounts.google.com/o/oauth2/auth",
                "token_uri": "https://oauth2.googleapis.com/token",
                "auth_provider_x509_cert_url": "https://www.googleapis.com/oauth2/v1/certs",
                "client_secret": "test-client-secret",
                "redirect_uris": ["http://localhost"]
            }
        }"#;
        
        let credentials: GoogleCredentialsFile = serde_json::from_str(test_json).unwrap();
        assert_eq!(credentials.installed.client_id, "test-client-id.apps.googleusercontent.com");
        assert_eq!(credentials.installed.client_secret, "test-client-secret");
        assert_eq!(credentials.installed.project_id, "test-project");
    }
}