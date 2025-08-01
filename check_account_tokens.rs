// Quick utility to check account token status
// Run with: cargo run --bin check-account-tokens

use comunicado::oauth2::storage::SecureStorage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Account Token Status Check");
    println!("============================");
    
    // Initialize account storage
    let account_storage = SecureStorage::new("comunicado".to_string())?;
    
    // Load all accounts
    match account_storage.load_all_accounts() {
        Ok(accounts) => {
            if accounts.is_empty() {
                println!("ℹ️  No accounts found");
                return Ok(());
            }
            
            println!("📊 Found {} account(s):", accounts.len());
            println!();
            
            for account in &accounts {
                println!("🔑 Account: {}", account.account_id);
                println!("   Email: {}", account.email_address);
                println!("   Provider: {}", account.provider);
                println!("   Access Token: {}", if account.access_token.is_empty() { "❌ EMPTY" } else { "✅ Present" });
                println!("   Refresh Token: {}", if account.refresh_token.is_some() { "✅ Present" } else { "❌ Missing" });
                
                if let Some(expires_at) = account.token_expires_at {
                    let now = chrono::Utc::now();
                    if now > expires_at {
                        println!("   Expiration: ❌ EXPIRED at {}", expires_at.format("%Y-%m-%d %H:%M:%S UTC"));
                        println!("   Time since expiry: {}", format_duration(now - expires_at));
                    } else {
                        println!("   Expiration: ✅ Valid until {}", expires_at.format("%Y-%m-%d %H:%M:%S UTC"));
                        println!("   Time remaining: {}", format_duration(expires_at - now));
                    }
                } else {
                    println!("   Expiration: ⚠️  No expiration time set");
                }
                
                println!("   Status: {}", if account.is_token_expired() { "🔴 EXPIRED" } else { "🟢 VALID" });
                println!();
            }
            
            // Summary
            let expired_count = accounts.iter().filter(|a| a.is_token_expired()).count();
            let valid_count = accounts.len() - expired_count;
            
            println!("📈 Summary:");
            println!("   🟢 Valid accounts: {}", valid_count);
            println!("   🔴 Expired accounts: {}", expired_count);
            
            if expired_count > 0 {
                println!();
                println!("💡 To fix expired accounts:");
                println!("   1. Run: comunicado auth --account <account_id>");
                println!("   2. Or use the app's account management (Ctrl+A)");
                println!("   3. Re-authenticate to get fresh tokens");
            }
        }
        Err(e) => {
            println!("❌ Failed to load accounts: {}", e);
        }
    }
    
    Ok(())
}

fn format_duration(duration: chrono::Duration) -> String {
    let total_seconds = duration.num_seconds().abs();
    let days = total_seconds / 86400;
    let hours = (total_seconds % 86400) / 3600;
    let minutes = (total_seconds % 3600) / 60;
    
    if days > 0 {
        format!("{} days, {} hours", days, hours)
    } else if hours > 0 {
        format!("{} hours, {} minutes", hours, minutes)
    } else {
        format!("{} minutes", minutes)
    }
}