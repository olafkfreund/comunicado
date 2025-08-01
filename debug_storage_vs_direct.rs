// Compare storage load vs direct SecureStorage calls
// Run with: cargo run --bin debug-storage-vs-direct

use comunicado::oauth2::storage::SecureStorage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Storage vs Direct Load Comparison");
    println!("===================================");
    
    let now = chrono::Utc::now();
    println!("🕐 Current UTC time: {}", now.format("%Y-%m-%d %H:%M:%S UTC"));
    println!();
    
    let storage = SecureStorage::new("comunicado".to_string())?;
    
    // Method 1: load_all_accounts() (what our token checker uses)
    match storage.load_all_accounts() {
        Ok(accounts) => {
            println!("📊 Method 1: load_all_accounts()");
            for account in &accounts {
                println!("   Account: {}", account.account_id);
                println!("   Expires: {:?}", account.token_expires_at);
                println!("   is_token_expired(): {}", account.is_token_expired());
                println!();
            }
        }
        Err(e) => println!("❌ load_all_accounts() failed: {}", e),
    }
    
    // Method 2: load_account() for each account (what the app might use)
    let accounts_list = storage.list_accounts()?;
    println!("📊 Method 2: load_account() for each ID");
    for account_from_list in accounts_list {
        let account_id_str = &account_from_list.account_id;
        match storage.load_account(account_id_str) {
            Ok(Some(account)) => {
                println!("   Account: {}", account.account_id);
                println!("   Expires: {:?}", account.token_expires_at);
                println!("   is_token_expired(): {}", account.is_token_expired());
                
                if let Some(expires_at) = account.token_expires_at {
                    let manual_check = now > expires_at;
                    println!("   Manual check (now > expires): {}", manual_check);
                    println!("   Diff: {} seconds", (expires_at - now).num_seconds());
                }
                println!();
            }
            Ok(None) => println!("   ❌ Account {} not found", account_id_str),
            Err(e) => println!("   ❌ Failed to load {}: {}", account_id_str, e),
        }
    }
    
    Ok(())
}