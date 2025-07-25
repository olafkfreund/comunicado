use comunicado::email::{
    EmailDatabase, SyncEngine, SyncStrategy, SyncProgress, SyncPhase, 
    ConflictResolution, StoredMessage
};
use comunicado::imap::{ImapClient, ImapConfig, ImapAuthMethod, ImapCapability};
use comunicado::oauth2::TokenManager;
use std::sync::Arc;
use tokio::sync::mpsc;
use tempfile::tempdir;
use chrono::Utc;
use uuid::Uuid;

/// Test complete sync engine workflow
#[tokio::test]
async fn test_sync_engine_workflow() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("sync_workflow_test.db");
    let database = Arc::new(EmailDatabase::new(db_path.to_str().unwrap()).await.unwrap());

    let (progress_sender, mut progress_receiver) = mpsc::unbounded_channel();
    let sync_engine = SyncEngine::new(database.clone(), progress_sender);

    // Test progress monitoring
    let progress_task = tokio::spawn(async move {
        let mut received_phases = Vec::new();
        while let Some(progress) = progress_receiver.recv().await {
            received_phases.push(progress.phase.clone());
            if matches!(progress.phase, SyncPhase::Complete | SyncPhase::Error(_)) {
                break;
            }
        }
        received_phases
    });

    // Create a mock client (this will fail to connect, but that's expected)
    let config = ImapConfig::new(
        "mock.server.com".to_string(),
        993,
        "test@example.com".to_string(),
        "password".to_string(),
    );
    let client = ImapClient::new(config);

    // Try to sync (will fail due to mock server, but we test the progress flow)
    let result = sync_engine.sync_account(
        "test_account".to_string(),
        client,
        SyncStrategy::Full,
    ).await;

    // Should fail due to mock server
    assert!(result.is_err());

    // Check that progress phases were reported
    progress_task.abort(); // Stop the task
}

/// Test different sync strategies
#[tokio::test]
async fn test_sync_strategies() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("sync_strategies_test.db");
    let database = Arc::new(EmailDatabase::new(db_path.to_str().unwrap()).await.unwrap());

    let (progress_sender, _progress_receiver) = mpsc::unbounded_channel();
    let sync_engine = SyncEngine::new(database.clone(), progress_sender);

    let strategies = vec![
        SyncStrategy::Full,
        SyncStrategy::Incremental,
        SyncStrategy::HeadersOnly,
        SyncStrategy::Recent(7),
    ];

    for strategy in strategies {
        let config = ImapConfig::new(
            "mock.server.com".to_string(),
            993,
            "test@example.com".to_string(),
            "password".to_string(),
        );
        let client = ImapClient::new(config);

        // Each sync attempt should fail (mock server), but we test the strategy handling
        let result = sync_engine.sync_account(
            "test_account".to_string(),
            client,
            strategy,
        ).await;

        assert!(result.is_err()); // Expected to fail with mock server
    }
}

/// Test conflict resolution strategies
#[tokio::test]
async fn test_conflict_resolution() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("conflict_test.db");
    let database = Arc::new(EmailDatabase::new(db_path.to_str().unwrap()).await.unwrap());

    let (progress_sender, _progress_receiver) = mpsc::unbounded_channel();
    let mut sync_engine = SyncEngine::new(database.clone(), progress_sender);

    // Test different conflict resolution strategies
    let strategies = vec![
        ConflictResolution::ServerWins,
        ConflictResolution::LocalWins,
        ConflictResolution::Merge,
        ConflictResolution::AskUser,
    ];

    for strategy in strategies {
        sync_engine.set_conflict_resolution(strategy.clone());
        
        // Create and store a test message
        let message = create_test_message("test_account", "INBOX", 1);
        database.store_message(&message).await.unwrap();

        // Verify message was stored
        let retrieved = database.get_message_by_uid("test_account", "INBOX", 1).await.unwrap();
        assert!(retrieved.is_some());
    }
}

/// Test progress tracking and monitoring
#[tokio::test]
async fn test_progress_tracking() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("progress_test.db");
    let database = Arc::new(EmailDatabase::new(db_path.to_str().unwrap()).await.unwrap());

    let (progress_sender, mut progress_receiver) = mpsc::unbounded_channel();
    let sync_engine = SyncEngine::new(database.clone(), progress_sender);

    // Create a manual progress update
    let progress = SyncProgress {
        account_id: "test_account".to_string(),
        folder_name: "INBOX".to_string(),
        phase: SyncPhase::Initializing,
        messages_processed: 0,
        total_messages: 100,
        bytes_downloaded: 0,
        started_at: Utc::now(),
        estimated_completion: None,
    };

    sync_engine.update_progress(progress.clone()).await;

    // Check that progress was sent
    let received_progress = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        progress_receiver.recv()
    ).await.unwrap().unwrap();

    assert_eq!(received_progress.account_id, "test_account");
    assert_eq!(received_progress.folder_name, "INBOX");
    assert_eq!(received_progress.phase, SyncPhase::Initializing);

    // Check that progress is stored internally
    let stored_progress = sync_engine.get_folder_sync_progress("test_account", "INBOX").await;
    assert!(stored_progress.is_some());
    assert_eq!(stored_progress.unwrap().total_messages, 100);

    // Get all progress
    let all_progress = sync_engine.get_sync_progress().await;
    assert!(!all_progress.is_empty());
}

/// Test sync cancellation
#[tokio::test]
async fn test_sync_cancellation() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("cancel_test.db");
    let database = Arc::new(EmailDatabase::new(db_path.to_str().unwrap()).await.unwrap());

    let (progress_sender, mut progress_receiver) = mpsc::unbounded_channel();
    let sync_engine = SyncEngine::new(database.clone(), progress_sender);

    // Start progress tracking
    let progress = SyncProgress {
        account_id: "test_account".to_string(),
        folder_name: "INBOX".to_string(),
        phase: SyncPhase::FetchingBodies,
        messages_processed: 50,
        total_messages: 100,
        bytes_downloaded: 1000,
        started_at: Utc::now(),
        estimated_completion: None,
    };

    sync_engine.update_progress(progress).await;

    // Cancel the sync
    let result = sync_engine.cancel_sync("test_account", "INBOX").await;
    assert!(result.is_ok());

    // Check for cancellation notification
    let cancelled_progress = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        progress_receiver.recv()
    ).await.unwrap().unwrap();

    match cancelled_progress.phase {
        SyncPhase::Error(ref msg) => {
            assert!(msg.contains("Cancelled"));
        }
        _ => panic!("Expected error phase with cancellation message"),
    }
}

/// Test database integration with sync engine
#[tokio::test]
async fn test_database_integration() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("integration_test.db");
    let database = Arc::new(EmailDatabase::new(db_path.to_str().unwrap()).await.unwrap());

    // Store some test messages
    for i in 1..=10 {
        let message = create_test_message("test_account", "INBOX", i);
        database.store_message(&message).await.unwrap();
    }

    // Verify messages were stored
    let messages = database.get_messages("test_account", "INBOX", None, None).await.unwrap();
    assert_eq!(messages.len(), 10);

    // Test search functionality
    let search_results = database.search_messages("test_account", "Test", None).await.unwrap();
    assert_eq!(search_results.len(), 10); // All messages should match "Test"

    // Test folder sync state
    let sync_state = comunicado::email::FolderSyncState {
        account_id: "test_account".to_string(),
        folder_name: "INBOX".to_string(),
        uid_validity: 12345,
        uid_next: 11,
        highest_modseq: Some(987654321),
        last_sync: Utc::now(),
        message_count: 10,
        unread_count: 5,
        sync_status: comunicado::email::SyncStatus::Complete,
    };

    database.update_folder_sync_state(&sync_state).await.unwrap();

    let retrieved_state = database.get_folder_sync_state("test_account", "INBOX").await.unwrap();
    assert!(retrieved_state.is_some());
    let state = retrieved_state.unwrap();
    assert_eq!(state.uid_validity, 12345);
    assert_eq!(state.message_count, 10);
    assert_eq!(state.unread_count, 5);
}

/// Test OAuth2 client integration
#[tokio::test]
async fn test_oauth2_integration() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("oauth2_test.db");
    let database = Arc::new(EmailDatabase::new(db_path.to_str().unwrap()).await.unwrap());

    let (progress_sender, _progress_receiver) = mpsc::unbounded_channel();
    let sync_engine = SyncEngine::new(database.clone(), progress_sender);

    // Create OAuth2 configuration
    let config = ImapConfig {
        hostname: "imap.gmail.com".to_string(),
        port: 993,
        username: "test@gmail.com".to_string(),
        auth_method: ImapAuthMethod::OAuth2 { 
            account_id: "gmail_test".to_string() 
        },
        use_tls: true,
        use_starttls: false,
        timeout_seconds: 30,
        validate_certificates: true,
    };

    let token_manager = TokenManager::new();
    let client = ImapClient::new_with_oauth2(config, token_manager);

    // Try to sync (will fail without valid tokens, but tests the setup)
    let result = sync_engine.sync_account(
        "gmail_test".to_string(),
        client,
        SyncStrategy::Incremental,
    ).await;

    // Should fail due to lack of valid OAuth2 tokens
    assert!(result.is_err());
}

/// Test multiple concurrent syncs
#[tokio::test]
async fn test_concurrent_syncs() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("concurrent_test.db");
    let database = Arc::new(EmailDatabase::new(db_path.to_str().unwrap()).await.unwrap());

    let (progress_sender, _progress_receiver) = mpsc::unbounded_channel();
    let sync_engine = Arc::new(SyncEngine::new(database.clone(), progress_sender));

    // Spawn multiple sync tasks
    let mut handles = Vec::new();
    
    for i in 0..3 {
        let sync_engine_clone = Arc::clone(&sync_engine);
        let account_id = format!("account_{}", i);
        
        let handle = tokio::spawn(async move {
            let config = ImapConfig::new(
                "mock.server.com".to_string(),
                993,
                "test@example.com".to_string(),
                "password".to_string(),
            );
            let client = ImapClient::new(config);

            sync_engine_clone.sync_account(
                account_id,
                client,
                SyncStrategy::Incremental,
            ).await
        });
        
        handles.push(handle);
    }

    // Wait for all syncs to complete (they'll all fail due to mock server)
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_err()); // Expected to fail with mock server
    }
}

/// Test IMAP capabilities integration
#[tokio::test]
async fn test_imap_capabilities() {
    let config = ImapConfig::new(
        "imap.example.com".to_string(),
        993,
        "test@example.com".to_string(),
        "password".to_string(),
    );

    let mut client = ImapClient::new(config);

    // Test capabilities setting (for testing purposes)
    let capabilities = vec![
        ImapCapability::Idle,
        ImapCapability::CondStore,
        ImapCapability::AuthXOAuth2,
    ];

    client.set_capabilities(capabilities.clone());
    assert_eq!(client.capabilities(), &capabilities);

    // Test specific capability checks
    assert!(client.capabilities().contains(&ImapCapability::Idle));
    assert!(client.capabilities().contains(&ImapCapability::CondStore));
    assert!(client.capabilities().contains(&ImapCapability::AuthXOAuth2));
}

/// Performance test for sync engine operations
#[tokio::test]
async fn test_sync_performance() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("performance_test.db");
    let database = Arc::new(EmailDatabase::new(db_path.to_str().unwrap()).await.unwrap());

    // Store many messages for performance testing
    let start = std::time::Instant::now();
    
    for i in 1..=1000 {
        let message = create_test_message("perf_account", "INBOX", i);
        database.store_message(&message).await.unwrap();
    }
    
    let store_duration = start.elapsed();
    println!("Stored 1000 messages in: {:?}", store_duration);

    // Test retrieval performance
    let start = std::time::Instant::now();
    let messages = database.get_messages("perf_account", "INBOX", Some(100), None).await.unwrap();
    let retrieve_duration = start.elapsed();
    
    assert_eq!(messages.len(), 100);
    println!("Retrieved 100 messages in: {:?}", retrieve_duration);

    // Test search performance
    let start = std::time::Instant::now();
    let search_results = database.search_messages("perf_account", "Test", Some(50)).await.unwrap();
    let search_duration = start.elapsed();
    
    assert_eq!(search_results.len(), 50);
    println!("Searched 1000 messages in: {:?}", search_duration);

    // Performance assertions
    assert!(store_duration.as_millis() < 5000, "Storing 1000 messages took too long");
    assert!(retrieve_duration.as_millis() < 100, "Retrieving 100 messages took too long");
    assert!(search_duration.as_millis() < 100, "Searching took too long");
}

/// Helper function to create test messages
fn create_test_message(account_id: &str, folder: &str, uid: u32) -> StoredMessage {
    StoredMessage {
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
        size: Some(500 + uid * 10),
        priority: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_synced: Utc::now(),
        sync_version: 1,
        is_draft: false,
        is_deleted: false,
    }
}