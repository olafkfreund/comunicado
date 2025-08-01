#!/usr/bin/env cargo run --example
//! OAuth2 Token Diagnostics Tool
//!
//! This example helps diagnose OAuth2 token issues for Gmail IMAP authentication.
//! Run with: cargo run --example token_diagnostics

use anyhow::Result;
use comunicado::oauth2::{SecureStorage, TokenDiagnosis, TokenManager};

#[tokio::main]
async fn main() -> Result<()> {
    println!("OAuth2 Token Diagnostics Tool");
    println!("============================\n");

    // Initialize storage to load accounts
    let storage = SecureStorage::new("comunicado".to_string())
        .map_err(|e| anyhow::anyhow!("Failed to initialize storage: {}", e))?;

    // Get all stored account IDs
    let account_ids = storage
        .list_account_ids()
        .map_err(|e| anyhow::anyhow!("Failed to list accounts: {}", e))?;

    if account_ids.is_empty() {
        println!("❌ No accounts found in storage.");
        println!("   Please set up OAuth2 authentication first by running the main application.");
        return Ok(());
    }

    println!("📋 Found {} account(s) in storage:\n", account_ids.len());

    // Create TokenManager and load tokens
    let token_manager = TokenManager::new();

    for account_id in &account_ids {
        if let Ok(Some(account)) = storage.load_account(account_id) {
            println!(
                "🔍 Account: {} ({})",
                account.display_name, account.email_address
            );
            println!("   Provider: {}", account.provider);
            println!("   Account ID: {}", account_id);

            // Show stored scopes
            println!("   📋 Stored Scopes:");
            for scope in &account.scopes {
                println!("      • {}", scope);
            }
            
            // Load tokens into token manager
            let token_response = comunicado::oauth2::TokenResponse {
                access_token: account.access_token.clone(),
                refresh_token: account.refresh_token.clone(),
                token_type: "Bearer".to_string(),
                scope: Some(account.scopes.join(" ")),
                expires_in: account.token_expires_at.map(|expires_at| {
                    let now = chrono::Utc::now();
                    let duration = expires_at.signed_duration_since(now);
                    duration.num_seconds().max(0) as u64
                }),
            };

            if let Err(e) = token_manager
                .store_tokens(
                    account_id.clone(),
                    account.provider.clone(),
                    &token_response,
                )
                .await
            {
                println!("   ⚠️  Warning: Failed to load tokens into manager: {}", e);
                continue;
            }

            // Diagnose token status
            let diagnosis = token_manager.diagnose_account_tokens(account_id).await;

            match diagnosis {
                TokenDiagnosis::Valid { expires_at, .. } => {
                    println!("   ✅ Status: Token is valid");
                    if let Some(expires) = expires_at {
                        println!("   📅 Expires: {}", expires.format("%Y-%m-%d %H:%M:%S UTC"));
                    }
                }
                TokenDiagnosis::ExpiringSoon {
                    expires_at,
                    has_refresh_token,
                    ..
                } => {
                    println!("   ⚠️  Status: Token expires soon");
                    if let Some(expires) = expires_at {
                        println!("   📅 Expires: {}", expires.format("%Y-%m-%d %H:%M:%S UTC"));
                    }
                    if has_refresh_token {
                        println!("   🔄 Refresh: Available (will auto-refresh)");
                    } else {
                        println!("   ❌ Refresh: Not available (manual re-auth needed)");
                    }
                }
                TokenDiagnosis::ExpiredWithRefresh { expired_at, .. } => {
                    println!("   ❌ Status: Token expired");
                    if let Some(expired) = expired_at {
                        println!("   📅 Expired: {}", expired.format("%Y-%m-%d %H:%M:%S UTC"));
                    }
                    println!("   🔄 Refresh: Available (but automatic refresh not implemented)");
                }
                TokenDiagnosis::ExpiredNoRefresh { expired_at, .. } => {
                    println!("   ❌ Status: Token expired");
                    if let Some(expired) = expired_at {
                        println!("   📅 Expired: {}", expired.format("%Y-%m-%d %H:%M:%S UTC"));
                    }
                    println!("   ❌ Refresh: Not available");
                }
                TokenDiagnosis::NotFound { .. } => {
                    println!("   ❌ Status: No tokens found");
                }
            }

            println!("   💡 Action: {}", diagnosis.recommended_action());

            // Check OAuth2 scope
            if account
                .scopes
                .contains(&"https://mail.google.com/".to_string())
            {
                println!("   ✅ Gmail Scope: Full Gmail access available");
            } else {
                println!("   ⚠️  Gmail Scope: Limited access - may need re-authentication");
                println!("      Current scopes: {}", account.scopes.join(", "));
            }

            println!();
        }
    }

    // Show summary and recommendations
    println!("🔧 Troubleshooting Guide:");
    println!("======================");
    println!();
    println!("❌ If you see 'Token expired' errors:");
    println!("   1. The OAuth2 access token has expired");
    println!("   2. Gmail returned HTTP 400 status during IMAP authentication");
    println!("   3. Current automatic token refresh is not fully implemented");
    println!();
    println!("✅ To fix the issue:");
    println!("   1. Run the main application: cargo run");
    println!("   2. Delete the expired account from settings");
    println!("   3. Re-add the account using OAuth2 setup wizard");
    println!("   4. This will get fresh tokens with proper expiration");
    println!();
    println!("🔍 For Gmail OAuth2 setup issues:");
    println!("   • Ensure Gmail API is enabled in Google Cloud Console");
    println!("   • Verify OAuth consent screen is configured");
    println!("   • Check that your email is in test users list");
    println!("   • Required scope: https://mail.google.com/");
    println!();
    println!("📝 Debug logs are written to: comunicado.log");

    Ok(())
}
