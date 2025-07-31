// Test script to verify account loading works correctly
use comunicado::oauth2::SecureStorage;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("Testing account loading...");
    
    // Create secure storage
    let storage = SecureStorage::new("comunicado".to_string())?;
    
    // List account IDs
    let account_ids = storage.list_account_ids()?;
    println!("Found account IDs: {:?}", account_ids);
    
    // Load all accounts
    let accounts = storage.load_all_accounts()?;
    println!("Loaded {} accounts:", accounts.len());
    
    for account in &accounts {
        println!("Account: {} ({})", account.display_name, account.email_address);
        println!("  Provider: {}", account.provider);
        println!("  Access token empty: {}", account.access_token.is_empty());
        println!("  Has refresh token: {}", account.refresh_token.is_some());
        println!("  Token expired: {}", account.is_token_expired());
        if let Some(expires_at) = account.token_expires_at {
            println!("  Expires at: {}", expires_at);
        }
        println!();
    }
    
    Ok(())
}