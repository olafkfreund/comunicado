#!/usr/bin/env cargo run --example
//! OAuth2 Token Refresh Test
//! 
//! This example tests the OAuth2 token refresh functionality for Gmail.
//! 
//! Setup:
//! 1. Set environment variables for your Gmail OAuth2 credentials:
//!    export GMAIL_CLIENT_ID="your-client-id.apps.googleusercontent.com"
//!    export GMAIL_CLIENT_SECRET="your-client-secret"
//! 
//! 2. Run the test:
//!    cargo run --example token_refresh_test

use anyhow::Result;
use comunicado::oauth2::{SecureStorage, TokenManager, TokenDiagnosis};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    println!("OAuth2 Token Refresh Test");
    println!("========================\n");

    // Check for required environment variables
    let client_id = std::env::var("GMAIL_CLIENT_ID")
        .map_err(|_| anyhow::anyhow!("GMAIL_CLIENT_ID environment variable not set"))?;
    let client_secret = std::env::var("GMAIL_CLIENT_SECRET")
        .map_err(|_| anyhow::anyhow!("GMAIL_CLIENT_SECRET environment variable not set"))?;

    println!("✅ Found OAuth2 credentials in environment");
    println!("   Client ID: {}...{}", &client_id[..20], &client_id[client_id.len()-20..]);
    println!();

    // Initialize storage
    let storage = Arc::new(SecureStorage::new("comunicado".to_string())
        .map_err(|e| anyhow::anyhow!("Failed to initialize storage: {}", e))?);

    // Get all stored account IDs
    let account_ids = storage.list_account_ids()
        .map_err(|e| anyhow::anyhow!("Failed to list accounts: {}", e))?;

    if account_ids.is_empty() {
        println!("❌ No accounts found in storage.");
        println!("   Please set up OAuth2 authentication first by running the main application.");
        return Ok(());
    }

    // Create TokenManager with storage
    let token_manager = TokenManager::new_with_storage(storage.clone());

    // Find Gmail accounts and try to refresh their tokens
    for account_id in &account_ids {
        if let Ok(Some(account)) = storage.load_account(account_id) {
            if account.provider != "gmail" {
                println!("⏭️  Skipping non-Gmail account: {} ({})", account.display_name, account.provider);
                continue;
            }

            println!("🔍 Testing token refresh for Gmail account: {} ({})", account.display_name, account.email_address);
            
            // Load tokens into token manager
            let token_response = comunicado::oauth2::TokenResponse {
                access_token: account.access_token.clone(),
                refresh_token: account.refresh_token.clone(),
                token_type: "Bearer".to_string(),
                expires_in: account.token_expires_at.map(|expires_at| {
                    let now = chrono::Utc::now();
                    let duration = expires_at.signed_duration_since(now);
                    duration.num_seconds().max(0) as u64
                }),
                scope: Some(account.scopes.join(" ")),
            };
            
            if let Err(e) = token_manager.store_tokens(
                account_id.clone(),
                account.provider.clone(),
                &token_response,
            ).await {
                println!("   ❌ Failed to load tokens into manager: {}", e);
                continue;
            }
            
            // Get current token status
            let diagnosis = token_manager.diagnose_account_tokens(account_id).await;
            println!("   📊 Current status: {}", diagnosis.description());
            
            // Test token refresh if needed
            if diagnosis.needs_action() {
                println!("   🔄 Attempting token refresh...");
                
                match token_manager.refresh_access_token(account_id).await {
                    Ok(new_token) => {
                        println!("   ✅ Token refresh successful!");
                        if let Some(expires_at) = new_token.expires_at {
                            println!("   📅 New expiration: {}", expires_at.format("%Y-%m-%d %H:%M:%S UTC"));
                        }
                        
                        // Verify the refreshed token works
                        match token_manager.get_valid_access_token(account_id).await {
                            Ok(Some(valid_token)) => {
                                println!("   ✅ Refreshed token is valid and ready for use");
                                println!("   🔑 Token length: {} characters", valid_token.token.len());
                            }
                            Ok(None) => {
                                println!("   ⚠️  Warning: No valid token available after refresh");
                            }
                            Err(e) => {
                                println!("   ❌ Error getting valid token: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("   ❌ Token refresh failed: {}", e);
                        println!("   💡 This might be because:");
                        println!("      • Refresh token has expired");
                        println!("      • OAuth2 credentials don't match those used during setup");
                        println!("      • Gmail API access has been revoked");
                        println!("      • Network connectivity issues");
                    }
                }
            } else {
                println!("   ✅ Token is valid, no refresh needed");
            }
            
            println!();
        }
    }

    println!("🔧 Next Steps:");
    println!("==============");
    println!();
    println!("If token refresh succeeded:");
    println!("• The application should now work without authentication errors");
    println!("• Tokens are automatically saved to persistent storage");
    println!("• Future IMAP connections will use the refreshed token");
    println!();
    println!("If token refresh failed:");
    println!("• Delete the account from settings in the main application");
    println!("• Re-add the account using the OAuth2 setup wizard");
    println!("• Make sure Gmail API is properly configured in Google Cloud Console");

    Ok(())
}