use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Test database connection and query like TUI would do
    let db_path = dirs::config_dir()
        .ok_or("Cannot find config directory")?
        .join("comunicado")
        .join("databases")
        .join("email.db");

    println!("📊 Testing database: {}", db_path.display());

    // Create database connection (same as TUI)
    let database = comunicado::email::database::EmailDatabase::new_with_mode(
        db_path.to_str().unwrap(), 
        true
    ).await?;
    
    println!("✅ Database connection established");

    // Test the exact query that TUI would use
    let account_id = "olaf_loken_gmail_com";
    let folder_name = "INBOX";
    
    println!("🔍 Querying: account_id='{}', folder_name='{}'", account_id, folder_name);
    
    let messages = database.get_messages(account_id, folder_name, Some(100), None).await?;
    
    println!("📧 Found {} messages", messages.len());
    
    if !messages.is_empty() {
        println!("First 3 messages:");
        for (i, msg) in messages.iter().take(3).enumerate() {
            println!("  {}: {}", i + 1, msg.subject);
        }
    } else {
        println!("❌ No messages found!");
        
        // Debug: Check what's actually in the database
        println!("🔍 Checking what accounts exist in database...");
        let accounts_query = sqlx::query("SELECT DISTINCT account_id FROM messages")
            .fetch_all(&database.pool)
            .await?;
        
        for row in accounts_query {
            let account: String = row.get("account_id");
            println!("  Account: {}", account);
        }
        
        println!("🔍 Checking what folders exist for account...");
        let folders_query = sqlx::query("SELECT DISTINCT folder_name FROM messages WHERE account_id = ?")
            .bind(account_id)
            .fetch_all(&database.pool)
            .await?;
            
        for row in folders_query {
            let folder: String = row.get("folder_name");
            println!("  Folder: {}", folder);
        }
    }
    
    Ok(())
}