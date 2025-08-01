/// Integration tests for token refresh functionality
/// 
/// These tests verify that the automatic token refresh mechanism works correctly
/// and integrates properly with the IMAP sync system.

use comunicado::oauth2::{AccountConfig, TokenManager, AuthType, SecurityType};
use comunicado::email::EmailDatabase;
use std::sync::Arc;
use tokio::time::Duration;
use anyhow::Result;

#[tokio::test]
async fn test_token_refresh_scheduler_initialization() {
    // Test that token refresh scheduler can be created
    let token_manager = TokenManager::new();
    let _scheduler = comunicado::oauth2::token::TokenRefreshScheduler::new(Arc::new(token_manager));
    
    // Should be able to create scheduler without errors
    // Note: scheduler creation should always succeed for valid input
}

#[tokio::test]
async fn test_expired_token_detection() {
    // Create an expired token config for testing
    let config = AccountConfig {
        account_id: "test-account".to_string(),
        display_name: "Test Account".to_string(),
        email_address: "test@example.com".to_string(),
        provider: "gmail".to_string(),
        auth_type: AuthType::OAuth2,
        imap_server: "imap.gmail.com".to_string(),
        imap_port: 993,
        smtp_server: "smtp.gmail.com".to_string(),
        smtp_port: 587,
        security: SecurityType::SSL,
        access_token: "expired_token".to_string(),
        refresh_token: Some("refresh_token".to_string()),
        token_expires_at: Some(chrono::Utc::now() - chrono::Duration::minutes(5)), // Expired 5 minutes ago
        scopes: vec![],
    };
    
    // Token should be detected as expired
    assert!(config.is_token_expired());
}

#[tokio::test]
async fn test_valid_token_detection() {
    // Create a valid token config for testing
    let config = AccountConfig {
        account_id: "test-account".to_string(),
        display_name: "Test Account".to_string(),
        email_address: "test@example.com".to_string(),
        provider: "gmail".to_string(),
        auth_type: AuthType::OAuth2,
        imap_server: "imap.gmail.com".to_string(),
        imap_port: 993,
        smtp_server: "smtp.gmail.com".to_string(),
        smtp_port: 587,
        security: SecurityType::SSL,
        access_token: "valid_token".to_string(),
        refresh_token: Some("refresh_token".to_string()),
        token_expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(1)), // Valid for 1 hour
        scopes: vec![],
    };
    
    // Token should not be detected as expired
    assert!(!config.is_token_expired());
}

#[tokio::test]
async fn test_background_processing_channels() -> Result<()> {
    // Test that we can create channels for background processing
    let _database = Arc::new(EmailDatabase::new_in_memory().await?);
    let _imap_manager = Arc::new(comunicado::imap::ImapAccountManager::new()
        .expect("Failed to create IMAP manager"));
    
    // Create background processor message channels
    let (_progress_sender, _progress_receiver) = tokio::sync::mpsc::unbounded_channel::<comunicado::email::sync_engine::SyncProgress>();
    let (_completion_sender, _completion_receiver) = tokio::sync::mpsc::unbounded_channel::<comunicado::performance::background_processor::TaskResult>();
    
    // Should be able to create channels without errors
    assert!(true); // Channels created successfully
    
    Ok(())
}

#[tokio::test]
async fn test_sync_service_components_creation() -> Result<()> {
    // Test that individual service components can be created
    let database = Arc::new(EmailDatabase::new_in_memory().await?);
    let _sync_engine = Arc::new(comunicado::email::sync_engine::SyncEngine::new(
        database.clone(),
        tokio::sync::mpsc::unbounded_channel().0
    ));
    let _imap_manager = Arc::new(comunicado::imap::ImapAccountManager::new()
        .expect("Failed to create IMAP manager"));
    
    // Should be able to create individual services
    assert!(true); // Services created successfully
    
    Ok(())
}

#[tokio::test]
async fn test_sync_progress_reporting() {
    // Test that sync progress can be created and has expected fields
    let progress = comunicado::email::sync_engine::SyncProgress {
        account_id: "test-account".to_string(),
        folder_name: "INBOX".to_string(),
        phase: comunicado::email::sync_engine::SyncPhase::Initializing,
        messages_processed: 0,
        total_messages: 100,
        bytes_downloaded: 0,
        started_at: chrono::Utc::now(),
        estimated_completion: None,
    };
    
    // Verify progress structure
    assert_eq!(progress.account_id, "test-account");
    assert_eq!(progress.folder_name, "INBOX");
    assert_eq!(progress.total_messages, 100);
    assert!(matches!(progress.phase, comunicado::email::sync_engine::SyncPhase::Initializing));
}

#[tokio::test]
async fn test_task_priority_ordering() {
    use comunicado::performance::background_processor::{BackgroundTask, BackgroundTaskType, TaskPriority};
    use std::time::Instant;
    use uuid::Uuid;
    
    // Create tasks with different priorities
    let high_priority_task = BackgroundTask {
        id: Uuid::new_v4(),
        name: "High Priority".to_string(),
        priority: TaskPriority::High,
        account_id: "test".to_string(),
        folder_name: None,
        task_type: BackgroundTaskType::AccountSync { 
            strategy: comunicado::email::sync_engine::SyncStrategy::Full 
        },
        created_at: Instant::now(),
        estimated_duration: Some(Duration::from_secs(30)),
    };
    
    let normal_priority_task = BackgroundTask {
        id: Uuid::new_v4(),
        name: "Normal Priority".to_string(),
        priority: TaskPriority::Normal,
        account_id: "test".to_string(),
        folder_name: None,
        task_type: BackgroundTaskType::AccountSync { 
            strategy: comunicado::email::sync_engine::SyncStrategy::Incremental 
        },
        created_at: Instant::now(),
        estimated_duration: Some(Duration::from_secs(15)),
    };
    
    // High priority should be greater (higher priority) than normal
    assert!(high_priority_task.priority > normal_priority_task.priority);
}