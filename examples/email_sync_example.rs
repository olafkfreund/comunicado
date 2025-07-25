use comunicado::email::{EmailDatabase, SyncEngine, SyncStrategy, SyncProgress, SyncPhase};
use comunicado::imap::{ImapClient, ImapConfig, ImapAuthMethod};
use comunicado::oauth2::TokenManager;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn, error, Level};
use tracing_subscriber;

/// Example demonstrating email synchronization with the sync engine
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("Starting email synchronization example");

    // Create database
    let db_path = "./example_emails.db";
    let database = Arc::new(EmailDatabase::new(db_path).await?);
    info!("Database created at: {}", db_path);

    // Create progress channel
    let (progress_sender, mut progress_receiver) = mpsc::unbounded_channel();

    // Create sync engine
    let mut sync_engine = SyncEngine::new(database.clone(), progress_sender);
    sync_engine.set_conflict_resolution(comunicado::email::ConflictResolution::ServerWins);

    // Spawn progress monitoring task
    let progress_task = tokio::spawn(async move {
        while let Some(progress) = progress_receiver.recv().await {
            match progress.phase {
                SyncPhase::Initializing => {
                    info!("📧 Starting sync for {}:{}", progress.account_id, progress.folder_name);
                }
                SyncPhase::CheckingFolders => {
                    info!("📁 Checking folders for {}:{}", progress.account_id, progress.folder_name);
                }
                SyncPhase::FetchingHeaders => {
                    info!("📄 Fetching headers for {}:{}", progress.account_id, progress.folder_name);
                }
                SyncPhase::FetchingBodies => {
                    info!("📨 Fetching message bodies ({}/{}) for {}:{}", 
                          progress.messages_processed, progress.total_messages,
                          progress.account_id, progress.folder_name);
                }
                SyncPhase::ProcessingChanges => {
                    info!("⚙️  Processing changes for {}:{}", progress.account_id, progress.folder_name);
                }
                SyncPhase::Complete => {
                    info!("✅ Sync complete for {}:{} ({} messages processed)", 
                          progress.account_id, progress.folder_name, progress.messages_processed);
                }
                SyncPhase::Error(ref error) => {
                    error!("❌ Sync error for {}:{}: {}", 
                           progress.account_id, progress.folder_name, error);
                }
            }
        }
    });

    // Example 1: Gmail OAuth2 sync
    info!("\n=== Example 1: Gmail OAuth2 Sync ===");
    
    let gmail_config = ImapConfig {
        hostname: "imap.gmail.com".to_string(),
        port: 993,
        username: "user@gmail.com".to_string(),
        auth_method: ImapAuthMethod::OAuth2 { 
            account_id: "gmail_account".to_string() 
        },
        use_tls: true,
        use_starttls: false,
        timeout_seconds: 30,
        validate_certificates: true,
    };

    // Create token manager (in real usage, this would have valid tokens)
    let token_manager = TokenManager::new();
    let gmail_client = ImapClient::new_with_oauth2(gmail_config, token_manager);

    // Note: This would fail in real usage without valid OAuth2 tokens
    // For demonstration, we'll show the setup and catch the error
    match sync_engine.sync_account(
        "gmail_account".to_string(),
        gmail_client,
        SyncStrategy::Incremental,
    ).await {
        Ok(_) => info!("Gmail sync completed successfully"),
        Err(e) => warn!("Gmail sync failed (expected in example): {}", e),
    }

    // Example 2: Different sync strategies
    info!("\n=== Example 2: Different Sync Strategies ===");

    let outlook_config = ImapConfig {
        hostname: "outlook.office365.com".to_string(),
        port: 993,
        username: "user@outlook.com".to_string(),
        auth_method: ImapAuthMethod::OAuth2 { 
            account_id: "outlook_account".to_string() 
        },
        use_tls: true,
        use_starttls: false,
        timeout_seconds: 30,
        validate_certificates: true,
    };

    let outlook_client = ImapClient::new_with_oauth2(outlook_config, TokenManager::new());

    // Try different sync strategies
    let strategies = vec![
        ("Full Sync", SyncStrategy::Full),
        ("Incremental Sync", SyncStrategy::Incremental),
        ("Headers Only", SyncStrategy::HeadersOnly),
        ("Recent (7 days)", SyncStrategy::Recent(7)),
    ];

    for (name, strategy) in strategies {
        info!("Testing {}", name);
        match sync_engine.sync_account(
            "outlook_account".to_string(),
            ImapClient::new_with_oauth2(outlook_config.clone(), TokenManager::new()),
            strategy,
        ).await {
            Ok(_) => info!("{} completed", name),
            Err(e) => warn!("{} failed (expected): {}", name, e),
        }
    }

    // Example 3: Monitor sync progress
    info!("\n=== Example 3: Sync Progress Monitoring ===");

    // Get current sync progress for all operations
    let all_progress = sync_engine.get_sync_progress().await;
    info!("Active sync operations: {}", all_progress.len());

    for (key, progress) in all_progress {
        info!("Sync {}: {:?} ({}/{})", 
              key, 
              progress.phase, 
              progress.messages_processed, 
              progress.total_messages);
    }

    // Example 4: Database operations
    info!("\n=== Example 4: Database Operations ===");

    // Get database statistics
    let stats = database.get_stats().await?;
    info!("Database stats:");
    info!("  Messages: {}", stats.message_count);
    info!("  Unread: {}", stats.unread_count);
    info!("  Accounts: {}", stats.account_count);
    info!("  Folders: {}", stats.folder_count);
    info!("  Size: {} bytes", stats.db_size_bytes);

    // Example search
    let search_results = database.search_messages("gmail_account", "important", Some(10)).await?;
    info!("Search results for 'important': {} messages", search_results.len());

    // Example message retrieval
    let inbox_messages = database.get_messages("gmail_account", "INBOX", Some(5), None).await?;
    info!("Recent INBOX messages: {}", inbox_messages.len());

    for message in inbox_messages {
        info!("  📧 {} from {} ({})", 
              message.subject, 
              message.from_addr, 
              message.date.format("%Y-%m-%d %H:%M"));
    }

    // Example 5: Conflict resolution demonstration
    info!("\n=== Example 5: Conflict Resolution ===");

    // Set different conflict resolution strategies
    sync_engine.set_conflict_resolution(comunicado::email::ConflictResolution::ServerWins);
    info!("Set conflict resolution to: Server Wins");

    sync_engine.set_conflict_resolution(comunicado::email::ConflictResolution::LocalWins);
    info!("Set conflict resolution to: Local Wins");

    sync_engine.set_conflict_resolution(comunicado::email::ConflictResolution::Merge);
    info!("Set conflict resolution to: Merge");

    // Example 6: Performance monitoring
    info!("\n=== Example 6: Performance Monitoring ===");

    use std::time::Instant;

    let start = Instant::now();
    
    // Simulate some database operations
    for i in 0..100 {
        let _ = database.get_messages("test_account", "INBOX", Some(1), Some(i)).await;
    }
    
    let duration = start.elapsed();
    info!("100 database queries took: {:?}", duration);
    info!("Average query time: {:?}", duration / 100);

    // Cleanup
    progress_task.abort();
    
    info!("\n=== Email Sync Example Complete ===");
    info!("Database file created: {}", db_path);
    info!("To explore the database:");
    info!("  sqlite3 {}", db_path);
    info!("  .tables");
    info!("  SELECT COUNT(*) FROM messages;");

    Ok(())
}

/// Helper function to create a mock stored message for testing
pub fn create_mock_message(account_id: &str, folder: &str, uid: u32) -> comunicado::email::StoredMessage {
    use chrono::Utc;
    use uuid::Uuid;

    comunicado::email::StoredMessage {
        id: Uuid::new_v4(),
        account_id: account_id.to_string(),
        folder_name: folder.to_string(),
        imap_uid: uid,
        message_id: Some(format!("test-{}@example.com", uid)),
        thread_id: None,
        in_reply_to: None,
        references: vec![],
        subject: format!("Test Message {}", uid),
        from_addr: "sender@example.com".to_string(),
        from_name: Some("Test Sender".to_string()),
        to_addrs: vec!["recipient@example.com".to_string()],
        cc_addrs: vec![],
        bcc_addrs: vec![],
        reply_to: None,
        date: Utc::now(),
        body_text: Some(format!("This is test message number {}", uid)),
        body_html: Some(format!("<p>This is test message number <b>{}</b></p>", uid)),
        attachments: vec![],
        flags: vec!["\\Seen".to_string()],
        labels: vec![],
        size: Some(500 + uid * 10), // Vary the size
        priority: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_synced: Utc::now(),
        sync_version: 1,
        is_draft: false,
        is_deleted: false,
    }
}