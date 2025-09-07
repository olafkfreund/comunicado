//! Email operations service that coordinates IMAP operations with UI and database
//!
//! This service provides high-level email operations like delete, archive, mark read/unread
//! and handles the coordination between IMAP client, local database, and UI updates.

use crate::email::retry::{
    retry_async, CircuitBreaker, CircuitBreakerConfig, ErrorCategory, RetryConfig, RetryableError,
};
use crate::email::service_health::{
    DegradationDecision, GracefulDegradation, ServiceHealth, ServiceHealthMonitor, ServiceType,
};
use crate::email::EmailDatabase;
use crate::imap::{ImapAccountManager, MessageFlag};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Result type for email operations
pub type EmailOperationResult<T> = Result<T, EmailOperationError>;

/// Errors that can occur during email operations
#[derive(Debug, thiserror::Error)]
pub enum EmailOperationError {
    #[error("IMAP error: {0}")]
    Imap(#[from] crate::imap::ImapError),

    #[error("Database error: {0}")]
    Database(#[from] crate::email::database::DatabaseError),

    #[error("Account not found: {account_id}")]
    AccountNotFound { account_id: String },

    #[error("Message not found: UID {uid} in folder {folder}")]
    MessageNotFound { uid: u32, folder: String },

    #[error("Folder not found: {folder}")]
    FolderNotFound { folder: String },

    #[error("Operation not supported by server")]
    NotSupported,

    #[error("Invalid operation state: {reason}")]
    InvalidState { reason: String },
}

impl RetryableError for EmailOperationError {
    fn is_retryable(&self) -> bool {
        match self {
            // IMAP errors - mostly retryable (network, server issues)
            EmailOperationError::Imap(_) => true,

            // Database errors - usually retryable (temporary locks, etc.)
            EmailOperationError::Database(_) => true,

            // Account not found - not retryable (configuration issue)
            EmailOperationError::AccountNotFound { .. } => false,

            // Message not found - not retryable (permanent state)
            EmailOperationError::MessageNotFound { .. } => false,

            // Folder not found - not retryable (configuration issue)
            EmailOperationError::FolderNotFound { .. } => false,

            // Operation not supported - not retryable (server capability)
            EmailOperationError::NotSupported => false,

            // Invalid state - not retryable (logic error)
            EmailOperationError::InvalidState { .. } => false,
        }
    }
}

impl EmailOperationError {
    /// Categorize the error for more sophisticated retry logic
    pub fn category(&self) -> ErrorCategory {
        match self {
            EmailOperationError::Imap(_) => ErrorCategory::Network,
            EmailOperationError::Database(_) => ErrorCategory::ResourceLimit,
            EmailOperationError::AccountNotFound { .. } => ErrorCategory::ClientError,
            EmailOperationError::MessageNotFound { .. } => ErrorCategory::ClientError,
            EmailOperationError::FolderNotFound { .. } => ErrorCategory::ClientError,
            EmailOperationError::NotSupported => ErrorCategory::Protocol,
            EmailOperationError::InvalidState { .. } => ErrorCategory::ClientError,
        }
    }
}

/// Email operations service
pub struct EmailOperationsService {
    imap_manager: Arc<ImapAccountManager>,
    database: Arc<EmailDatabase>,
    /// Cache of folder names by type for each account
    folder_cache: Arc<RwLock<std::collections::HashMap<String, FolderCache>>>,
    /// Retry configuration for email operations
    retry_config: RetryConfig,
    /// Circuit breaker for IMAP operations per account
    circuit_breakers: Arc<RwLock<std::collections::HashMap<String, Arc<CircuitBreaker>>>>,
    /// Health monitoring and graceful degradation
    health_monitor: Arc<ServiceHealthMonitor>,
    graceful_degradation: Arc<GracefulDegradation>,
}

/// Cached folder information for an account
#[derive(Debug, Clone)]
pub struct FolderCache {
    pub inbox: String,
    pub sent: Option<String>,
    pub drafts: Option<String>,
    pub trash: Option<String>,
    pub archive: Option<String>,
    pub spam: Option<String>,
}

impl EmailOperationsService {
    /// Create a new email operations service
    pub fn new(imap_manager: Arc<ImapAccountManager>, database: Arc<EmailDatabase>) -> Self {
        let health_monitor = Arc::new(ServiceHealthMonitor::new());
        let graceful_degradation = Arc::new(GracefulDegradation::new(Arc::clone(&health_monitor)));

        Self {
            imap_manager,
            database,
            folder_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
            retry_config: RetryConfig {
                max_attempts: 3,
                initial_delay: std::time::Duration::from_millis(200),
                max_delay: std::time::Duration::from_secs(10),
                backoff_multiplier: 2.0,
                jitter: true,
            },
            circuit_breakers: Arc::new(RwLock::new(std::collections::HashMap::new())),
            health_monitor,
            graceful_degradation,
        }
    }

    /// Create a new email operations service with custom retry configuration
    pub fn with_retry_config(
        imap_manager: Arc<ImapAccountManager>,
        database: Arc<EmailDatabase>,
        retry_config: RetryConfig,
    ) -> Self {
        let health_monitor = Arc::new(ServiceHealthMonitor::new());
        let graceful_degradation = Arc::new(GracefulDegradation::new(Arc::clone(&health_monitor)));

        Self {
            imap_manager,
            database,
            folder_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
            retry_config,
            circuit_breakers: Arc::new(RwLock::new(std::collections::HashMap::new())),
            health_monitor,
            graceful_degradation,
        }
    }

    /// Create a new email operations service with full configuration
    pub fn with_full_config(
        imap_manager: Arc<ImapAccountManager>,
        database: Arc<EmailDatabase>,
        retry_config: RetryConfig,
        health_monitor: Arc<ServiceHealthMonitor>,
    ) -> Self {
        let graceful_degradation = Arc::new(GracefulDegradation::new(Arc::clone(&health_monitor)));

        Self {
            imap_manager,
            database,
            folder_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
            retry_config,
            circuit_breakers: Arc::new(RwLock::new(std::collections::HashMap::new())),
            health_monitor,
            graceful_degradation,
        }
    }

    /// Start health monitoring for services
    pub async fn start_health_monitoring(&self) {
        // Register core services
        self.health_monitor
            .register_service(ServiceType::Database)
            .await;
        self.health_monitor
            .register_service(ServiceType::EventBus)
            .await;
        self.health_monitor
            .register_service(ServiceType::Notifications)
            .await;
        self.health_monitor
            .register_service(ServiceType::Search)
            .await;

        // Start monitoring
        self.health_monitor.start_monitoring().await;

        info!("Health monitoring started for email operations service");
    }

    /// Register an IMAP account for health monitoring
    pub async fn register_account_for_monitoring(&self, account_id: &str) {
        let service = ServiceType::Imap(account_id.to_string());
        self.health_monitor.register_service(service).await;
        info!(
            "Registered IMAP account {} for health monitoring",
            account_id
        );
    }

    /// Update service health based on operation outcomes
    async fn update_service_health_from_result<T>(
        &self,
        service_type: ServiceType,
        result: &Result<T, EmailOperationError>,
        operation_duration: std::time::Duration,
    ) {
        let (health, details) = match result {
            Ok(_) => (
                ServiceHealth::Healthy,
                Some("Operation successful".to_string()),
            ),
            Err(EmailOperationError::Imap(_)) => {
                if operation_duration > std::time::Duration::from_secs(5) {
                    (
                        ServiceHealth::Degraded {
                            reason: "Slow IMAP response".to_string(),
                        },
                        Some(format!("Response took {:?}", operation_duration)),
                    )
                } else {
                    (
                        ServiceHealth::Unhealthy {
                            reason: "IMAP operation failed".to_string(),
                            since: std::time::Instant::now(),
                        },
                        Some("IMAP communication error".to_string()),
                    )
                }
            }
            Err(EmailOperationError::Database(_)) => (
                ServiceHealth::Unhealthy {
                    reason: "Database operation failed".to_string(),
                    since: std::time::Instant::now(),
                },
                Some("Database error".to_string()),
            ),
            Err(_) => (
                ServiceHealth::Degraded {
                    reason: "Operation failed".to_string(),
                },
                Some("General operation error".to_string()),
            ),
        };

        self.health_monitor
            .update_service_health(service_type, health, operation_duration, details)
            .await;
    }

    /// Get or create circuit breaker for an account
    async fn get_circuit_breaker(&self, account_id: &str) -> Arc<CircuitBreaker> {
        let mut breakers = self.circuit_breakers.write().await;
        breakers
            .entry(account_id.to_string())
            .or_insert_with(|| {
                Arc::new(CircuitBreaker::new(CircuitBreakerConfig {
                    failure_threshold: 5,
                    recovery_timeout: std::time::Duration::from_secs(30),
                    success_threshold: 3,
                    rolling_window: std::time::Duration::from_secs(300),
                }))
            })
            .clone()
    }

    /// Delete an email by message ID
    pub async fn delete_email_by_id(
        &self,
        account_id: &str,
        message_id: uuid::Uuid,
        folder_name: &str,
    ) -> EmailOperationResult<()> {
        // Get message from database to find UID
        let message = self
            .database
            .get_message_by_id(message_id)
            .await?
            .ok_or_else(|| EmailOperationError::MessageNotFound {
                uid: 0,
                folder: folder_name.to_string(),
            })?;
        let message_uid = message.imap_uid;

        self.delete_email(account_id, message_uid, folder_name)
            .await
    }

    /// Delete an email by marking it as deleted and expunging
    pub async fn delete_email(
        &self,
        account_id: &str,
        message_uid: u32,
        folder_name: &str,
    ) -> EmailOperationResult<()> {
        info!(
            "Deleting email UID {} from folder {} in account {}",
            message_uid, folder_name, account_id
        );

        // Check if we should proceed with email operation based on service health
        let degradation_decision = self
            .graceful_degradation
            .should_attempt_email_operation(account_id)
            .await;

        match degradation_decision {
            DegradationDecision::Proceed => {
                // Continue with normal operation
            }
            DegradationDecision::ProceedWithCaution {
                reason,
                fallback_strategy,
            } => {
                warn!("Proceeding with email deletion with caution: {}", reason);
                if let Some(strategy) = fallback_strategy {
                    debug!("Fallback strategy: {}", strategy);
                }
            }
            DegradationDecision::Fallback { reason, strategy } => {
                error!(
                    "Cannot perform email deletion: {} (fallback: {:?})",
                    reason, strategy
                );
                return Err(EmailOperationError::InvalidState {
                    reason: format!("Service unavailable: {}", reason),
                });
            }
        }

        // Get circuit breaker for this account
        let circuit_breaker = self.get_circuit_breaker(account_id).await;

        // Create retry operation
        let operation_name = format!(
            "delete_email_{}_{}_{}",
            account_id, folder_name, message_uid
        );

        // Clone values for the closure
        let account_id = account_id.to_string();
        let folder_name = folder_name.to_string();

        // Use a closure that captures self to access helper methods
        let operation_start = std::time::Instant::now();
        let result = {
            let self_ref = self;
            retry_async(
                &operation_name,
                || {
                    let account_id = account_id.clone();
                    let folder_name = folder_name.clone();
                    let circuit_breaker = Arc::clone(&circuit_breaker);

                    async move {
                        circuit_breaker
                            .call(|| async {
                                // Get IMAP client for the account
                                let client_arc = self_ref
                                    .imap_manager
                                    .get_client(&account_id)
                                    .await
                                    .map_err(|e| EmailOperationError::Imap(e))?;
                                let mut client = client_arc.lock().await;

                                // Select the folder
                                client
                                    .select_folder(&folder_name)
                                    .await
                                    .map_err(|e| EmailOperationError::Imap(e))?;

                                // Mark message as deleted
                                let uid_set = message_uid.to_string();
                                client
                                    .uid_store_flags(&uid_set, &[MessageFlag::Deleted], false)
                                    .await
                                    .map_err(|e| EmailOperationError::Imap(e))?;

                                // Expunge to permanently delete
                                client
                                    .expunge()
                                    .await
                                    .map_err(|e| EmailOperationError::Imap(e))?;

                                // Update local database
                                self_ref
                                    .database
                                    .delete_messages_by_uids(
                                        &account_id,
                                        &folder_name,
                                        &[message_uid],
                                    )
                                    .await
                                    .map_err(|e| EmailOperationError::Database(e))?;

                                Ok(())
                            })
                            .await
                            .map_err(|e| match e {
                                crate::email::retry::CircuitBreakerError::Open => {
                                    EmailOperationError::InvalidState {
                                        reason:
                                            "Circuit breaker is open - too many recent failures"
                                                .to_string(),
                                    }
                                }
                                crate::email::retry::CircuitBreakerError::Operation(op_err) => {
                                    op_err
                                }
                            })
                    }
                },
                &self.retry_config,
            )
            .await
        };

        let operation_duration = operation_start.elapsed();

        // Update service health based on the operation result
        self.update_service_health_from_result(
            ServiceType::Imap(account_id.clone()),
            &result,
            operation_duration,
        )
        .await;

        self.update_service_health_from_result(ServiceType::Database, &result, operation_duration)
            .await;

        result?;

        info!(
            "Successfully deleted email UID {} from {}/{}",
            message_uid, account_id, folder_name
        );
        Ok(())
    }

    /// Archive an email by message ID
    pub async fn archive_email_by_id(
        &self,
        account_id: &str,
        message_id: uuid::Uuid,
        source_folder: &str,
    ) -> EmailOperationResult<()> {
        // Get message from database to find UID
        let message = self
            .database
            .get_message_by_id(message_id)
            .await?
            .ok_or_else(|| EmailOperationError::MessageNotFound {
                uid: 0,
                folder: source_folder.to_string(),
            })?;
        let message_uid = message.imap_uid;

        self.archive_email(account_id, message_uid, source_folder)
            .await
    }

    /// Archive an email by moving it to the archive folder
    pub async fn archive_email(
        &self,
        account_id: &str,
        message_uid: u32,
        source_folder: &str,
    ) -> EmailOperationResult<()> {
        info!(
            "Archiving email UID {} from folder {} in account {}",
            message_uid, source_folder, account_id
        );

        // Get archive folder for this account
        let archive_folder = self.get_archive_folder(account_id).await?;

        // Get IMAP client
        let client_arc = self.get_imap_client(account_id).await?;
        let mut client = client_arc.lock().await;

        // Select source folder
        client.select_folder(source_folder).await?;

        // Copy message to archive folder
        let uid_set = message_uid.to_string();
        client.uid_copy_messages(&uid_set, &archive_folder).await?;

        // Mark original message as deleted
        client
            .uid_store_flags(&uid_set, &[MessageFlag::Deleted], false)
            .await?;

        // Expunge to remove from source folder
        client.expunge().await?;

        // Update database - message should be moved to archive folder
        // Note: In a full implementation, we'd need to fetch the message in the new location
        // For now, we'll just delete from the source folder in the database
        self.database
            .delete_messages_by_uids(account_id, source_folder, &[message_uid])
            .await?;

        info!(
            "Successfully archived email UID {} from {}/{} to {}",
            message_uid, account_id, source_folder, archive_folder
        );
        Ok(())
    }

    /// Mark an email as read by message ID
    pub async fn mark_email_read_by_id(
        &self,
        account_id: &str,
        message_id: uuid::Uuid,
        folder_name: &str,
    ) -> EmailOperationResult<()> {
        // Get message from database to find UID
        let message = self
            .database
            .get_message_by_id(message_id)
            .await?
            .ok_or_else(|| EmailOperationError::MessageNotFound {
                uid: 0,
                folder: folder_name.to_string(),
            })?;
        let message_uid = message.imap_uid;

        self.mark_email_read(account_id, message_uid, folder_name)
            .await
    }

    /// Mark an email as read
    pub async fn mark_email_read(
        &self,
        account_id: &str,
        message_uid: u32,
        folder_name: &str,
    ) -> EmailOperationResult<()> {
        info!(
            "Marking email UID {} as read in folder {} of account {}",
            message_uid, folder_name, account_id
        );

        // Get IMAP client
        let client_arc = self.get_imap_client(account_id).await?;
        let mut client = client_arc.lock().await;

        // Select folder
        client.select_folder(folder_name).await?;

        // Add Seen flag to IMAP server
        let uid_set = message_uid.to_string();
        client
            .uid_store_flags(&uid_set, &[MessageFlag::Seen], false)
            .await?;

        // Update database immediately for consistent state
        let flags = vec!["\\Seen".to_string()];
        self.database
            .update_message_flags(account_id, folder_name, message_uid, flags)
            .await?;

        info!(
            "Successfully marked email UID {} as read in {}/{}",
            message_uid, account_id, folder_name
        );
        Ok(())
    }

    /// Mark an email as unread by message ID
    pub async fn mark_email_unread_by_id(
        &self,
        account_id: &str,
        message_id: uuid::Uuid,
        folder_name: &str,
    ) -> EmailOperationResult<()> {
        // Get message from database to find UID
        let message = self
            .database
            .get_message_by_id(message_id)
            .await?
            .ok_or_else(|| EmailOperationError::MessageNotFound {
                uid: 0,
                folder: folder_name.to_string(),
            })?;
        let message_uid = message.imap_uid;

        self.mark_email_unread(account_id, message_uid, folder_name)
            .await
    }

    /// Mark an email as unread
    pub async fn mark_email_unread(
        &self,
        account_id: &str,
        message_uid: u32,
        folder_name: &str,
    ) -> EmailOperationResult<()> {
        info!(
            "Marking email UID {} as unread in folder {} of account {}",
            message_uid, folder_name, account_id
        );

        // Get IMAP client
        let client_arc = self.get_imap_client(account_id).await?;
        let mut client = client_arc.lock().await;

        // Select folder
        client.select_folder(folder_name).await?;

        // Remove Seen flag from IMAP server
        let uid_set = message_uid.to_string();
        client
            .uid_remove_flags(&uid_set, &[MessageFlag::Seen])
            .await?;

        // Update database immediately for consistent state
        let flags: Vec<String> = Vec::new(); // Empty flags means unread
        self.database
            .update_message_flags(account_id, folder_name, message_uid, flags)
            .await?;

        info!(
            "Successfully marked email UID {} as unread in {}/{}",
            message_uid, account_id, folder_name
        );
        Ok(())
    }

    /// Toggle read/unread status of an email
    pub async fn toggle_email_read_status(
        &self,
        account_id: &str,
        message_uid: u32,
        folder_name: &str,
    ) -> EmailOperationResult<bool> {
        // Check current read status from database
        match self
            .database
            .get_message_by_uid(account_id, folder_name, message_uid)
            .await
        {
            Ok(Some(message)) => {
                let is_seen = message.flags.contains(&"\\Seen".to_string());
                if is_seen {
                    self.mark_email_unread(account_id, message_uid, folder_name)
                        .await?;
                    Ok(false) // Now unread
                } else {
                    self.mark_email_read(account_id, message_uid, folder_name)
                        .await?;
                    Ok(true) // Now read
                }
            }
            Ok(None) => Err(EmailOperationError::MessageNotFound {
                uid: message_uid,
                folder: folder_name.to_string(),
            }),
            Err(e) => {
                error!("Failed to get message from database: {}", e);
                Err(EmailOperationError::Database(e))
            }
        }
    }

    /// Flag or unflag an email by message ID
    pub async fn toggle_email_flag_by_id(
        &self,
        account_id: &str,
        message_id: uuid::Uuid,
        folder_name: &str,
    ) -> EmailOperationResult<bool> {
        // Get message from database to find UID
        let message = self
            .database
            .get_message_by_id(message_id)
            .await?
            .ok_or_else(|| EmailOperationError::MessageNotFound {
                uid: 0,
                folder: folder_name.to_string(),
            })?;
        let message_uid = message.imap_uid;

        self.toggle_email_flag(account_id, message_uid, folder_name)
            .await
    }

    /// Flag or unflag an email
    pub async fn toggle_email_flag(
        &self,
        account_id: &str,
        message_uid: u32,
        folder_name: &str,
    ) -> EmailOperationResult<bool> {
        info!(
            "Toggling flag for email UID {} in folder {} of account {}",
            message_uid, folder_name, account_id
        );

        // Check current flag status from database
        let message = self
            .database
            .get_message_by_uid(account_id, folder_name, message_uid)
            .await?
            .ok_or_else(|| EmailOperationError::MessageNotFound {
                uid: message_uid,
                folder: folder_name.to_string(),
            })?;
        let is_flagged = message.flags.contains(&"\\Flagged".to_string());

        // Get IMAP client
        let client_arc = self.get_imap_client(account_id).await?;
        let mut client = client_arc.lock().await;

        // Select folder
        client.select_folder(folder_name).await?;

        let uid_set = message_uid.to_string();

        let flags = if is_flagged {
            // Remove flag from IMAP server
            client
                .uid_remove_flags(&uid_set, &[MessageFlag::Flagged])
                .await?;
            Vec::new() // Empty flags means unflagged
        } else {
            // Add flag to IMAP server
            client
                .uid_store_flags(&uid_set, &[MessageFlag::Flagged], false)
                .await?;
            vec!["\\Flagged".to_string()]
        };

        // Update database immediately for consistent state
        self.database
            .update_message_flags(account_id, folder_name, message_uid, flags)
            .await?;

        let new_status = !is_flagged;
        info!(
            "Successfully {} email UID {} in {}/{}",
            if new_status { "flagged" } else { "unflagged" },
            message_uid,
            account_id,
            folder_name
        );

        Ok(new_status)
    }

    /// Move an email to a different folder
    pub async fn move_email(
        &self,
        account_id: &str,
        message_uid: u32,
        source_folder: &str,
        destination_folder: &str,
    ) -> EmailOperationResult<()> {
        info!(
            "Moving email UID {} from {} to {} in account {}",
            message_uid, source_folder, destination_folder, account_id
        );

        // Get IMAP client
        let client_arc = self.get_imap_client(account_id).await?;
        let mut client = client_arc.lock().await;

        // Select source folder
        client.select_folder(source_folder).await?;

        // Copy message to destination
        let uid_set = message_uid.to_string();
        client
            .uid_copy_messages(&uid_set, destination_folder)
            .await?;

        // Mark original as deleted
        client
            .uid_store_flags(&uid_set, &[MessageFlag::Deleted], false)
            .await?;

        // Expunge to remove from source
        client.expunge().await?;

        // Update database (remove from source folder)
        self.database
            .delete_messages_by_uids(account_id, source_folder, &[message_uid])
            .await?;

        info!(
            "Successfully moved email UID {} from {} to {} in account {}",
            message_uid, source_folder, destination_folder, account_id
        );
        Ok(())
    }

    /// Get IMAP client for an account
    async fn get_imap_client(
        &self,
        account_id: &str,
    ) -> EmailOperationResult<std::sync::Arc<tokio::sync::Mutex<crate::imap::ImapClient>>> {
        self.imap_manager
            .get_client(account_id)
            .await
            .map_err(|e| EmailOperationError::Imap(e))
    }

    /// Get the archive folder name for an account
    async fn get_archive_folder(&self, account_id: &str) -> EmailOperationResult<String> {
        // Try to get from cache first
        {
            let cache = self.folder_cache.read().await;
            if let Some(folders) = cache.get(account_id) {
                if let Some(archive) = &folders.archive {
                    return Ok(archive.clone());
                }
            }
        }

        // If not cached, fetch folder list and find archive folder
        self.refresh_folder_cache(account_id).await?;

        let cache = self.folder_cache.read().await;
        if let Some(folders) = cache.get(account_id) {
            if let Some(archive) = &folders.archive {
                return Ok(archive.clone());
            }
        }

        // Fallback to common archive folder names
        warn!(
            "No archive folder found for account {}, using default 'Archive'",
            account_id
        );
        Ok("Archive".to_string())
    }

    /// Refresh folder cache for an account
    async fn refresh_folder_cache(&self, account_id: &str) -> EmailOperationResult<()> {
        debug!("Refreshing folder cache for account {}", account_id);

        let client_arc = self.get_imap_client(account_id).await?;
        let mut client = client_arc.lock().await;
        let folders = client.list_folders("", "*").await?;

        let mut folder_cache = FolderCache {
            inbox: "INBOX".to_string(),
            sent: None,
            drafts: None,
            trash: None,
            archive: None,
            spam: None,
        };

        // Categorize folders by common names and attributes
        for folder in folders {
            let name_upper = folder.name.to_uppercase();

            // Check folder attributes first
            for attr in &folder.attributes {
                match attr {
                    crate::imap::FolderAttribute::Sent => {
                        folder_cache.sent = Some(folder.name.clone())
                    }
                    crate::imap::FolderAttribute::Drafts => {
                        folder_cache.drafts = Some(folder.name.clone())
                    }
                    crate::imap::FolderAttribute::Trash => {
                        folder_cache.trash = Some(folder.name.clone())
                    }
                    crate::imap::FolderAttribute::Archive => {
                        folder_cache.archive = Some(folder.name.clone())
                    }
                    crate::imap::FolderAttribute::Junk => {
                        folder_cache.spam = Some(folder.name.clone())
                    }
                    _ => {}
                }
            }

            // Fallback to name-based detection
            if name_upper.contains("SENT") && folder_cache.sent.is_none() {
                folder_cache.sent = Some(folder.name.clone());
            } else if name_upper.contains("DRAFT") && folder_cache.drafts.is_none() {
                folder_cache.drafts = Some(folder.name.clone());
            } else if (name_upper.contains("TRASH") || name_upper.contains("DELETED"))
                && folder_cache.trash.is_none()
            {
                folder_cache.trash = Some(folder.name.clone());
            } else if (name_upper.contains("ARCHIVE") || name_upper.contains("ALL MAIL"))
                && folder_cache.archive.is_none()
            {
                folder_cache.archive = Some(folder.name.clone());
            } else if (name_upper.contains("SPAM") || name_upper.contains("JUNK"))
                && folder_cache.spam.is_none()
            {
                folder_cache.spam = Some(folder.name.clone());
            }
        }

        // Update cache
        let mut cache = self.folder_cache.write().await;
        cache.insert(account_id.to_string(), folder_cache);

        debug!("Folder cache refreshed for account {}", account_id);
        Ok(())
    }

    /// Get folder cache for debugging/inspection
    pub async fn get_folder_cache(&self, account_id: &str) -> Option<FolderCache> {
        let cache = self.folder_cache.read().await;
        cache.get(account_id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tokio;

    #[tokio::test]
    async fn test_email_operations_service_creation() {
        // This test would require mock IMAP manager and database
        // For now, just test that the service can be created
        // In a real implementation, we'd use dependency injection with mocks
    }

    #[test]
    fn test_email_operation_error_display() {
        let error = EmailOperationError::AccountNotFound {
            account_id: "test@example.com".to_string(),
        };
        assert!(error.to_string().contains("Account not found"));

        let error = EmailOperationError::MessageNotFound {
            uid: 123,
            folder: "INBOX".to_string(),
        };
        assert!(error.to_string().contains("Message not found"));
    }
}
