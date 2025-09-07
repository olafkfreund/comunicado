//! Application Event Handlers
//!
//! This module implements event handlers that subscribe to events and perform
//! actual operations like email management, calendar operations, and UI updates.

use crate::events::bus::{EventHandler, HandlerPriority};
use crate::events::types::{
    AccountEvent, AccountEventData, AppEvent, AppEventData, CalendarEvent, CalendarEventData,
    EmailEvent, EmailEventData, UIEvent, UIEventData,
};
use crate::events::{initialize_event_bus, EventError};
use std::sync::{Arc, Mutex};

/// Email operations handler that processes EmailEvent types
pub struct EmailEventHandler {
    /// Reference to email operations service for all email operations
    email_operations: Option<Arc<crate::email::EmailOperationsService>>,
}

impl EmailEventHandler {
    pub fn new() -> Self {
        Self {
            email_operations: None,
        }
    }

    /// Set email operations service reference
    pub fn set_email_operations(&mut self, operations: Arc<crate::email::EmailOperationsService>) {
        self.email_operations = Some(operations);
    }
}

impl EventHandler<EmailEventData> for EmailEventHandler {
    fn handle(&mut self, event: &EmailEventData) -> Result<(), EventError> {
        match &event.event {
            EmailEvent::EmailDeleted {
                account_id,
                email_id,
            } => {
                tracing::info!(
                    "Processing email deletion: {} in account {}",
                    email_id,
                    account_id
                );

                if let Some(operations) = &self.email_operations {
                    let operations = Arc::clone(operations);
                    let account_id = account_id.clone();
                    let email_id = *email_id;

                    // Spawn async task for email deletion
                    tokio::spawn(async move {
                        match operations
                            .delete_email_by_id(&account_id, email_id, "INBOX")
                            .await
                        {
                            Ok(_) => {
                                tracing::info!(
                                    "Successfully deleted email {} from account {}",
                                    email_id,
                                    account_id
                                );
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to delete email {} from account {}: {}",
                                    email_id,
                                    account_id,
                                    e
                                );
                            }
                        }
                    });
                } else {
                    tracing::warn!("No email operations service available for deletion");
                }
            }

            EmailEvent::EmailMarkedRead {
                account_id,
                email_id,
            } => {
                tracing::info!(
                    "Processing mark as read: {} in account {}",
                    email_id,
                    account_id
                );

                if let Some(operations) = &self.email_operations {
                    let operations = Arc::clone(operations);
                    let account_id = account_id.clone();
                    let email_id = *email_id;

                    tokio::spawn(async move {
                        match operations
                            .mark_email_read_by_id(&account_id, email_id, "INBOX")
                            .await
                        {
                            Ok(_) => {
                                tracing::info!(
                                    "Successfully marked email {} as read in account {}",
                                    email_id,
                                    account_id
                                );
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to mark email {} as read in account {}: {}",
                                    email_id,
                                    account_id,
                                    e
                                );
                            }
                        }
                    });
                } else {
                    tracing::warn!("No email operations service available for marking read");
                }
            }

            EmailEvent::EmailMarkedUnread {
                account_id,
                email_id,
            } => {
                tracing::info!(
                    "Processing mark as unread: {} in account {}",
                    email_id,
                    account_id
                );

                if let Some(operations) = &self.email_operations {
                    let operations = Arc::clone(operations);
                    let account_id = account_id.clone();
                    let email_id = *email_id;

                    tokio::spawn(async move {
                        match operations
                            .mark_email_unread_by_id(&account_id, email_id, "INBOX")
                            .await
                        {
                            Ok(_) => {
                                tracing::info!(
                                    "Successfully marked email {} as unread in account {}",
                                    email_id,
                                    account_id
                                );
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to mark email {} as unread in account {}: {}",
                                    email_id,
                                    account_id,
                                    e
                                );
                            }
                        }
                    });
                } else {
                    tracing::warn!("No email operations service available for marking unread");
                }
            }

            EmailEvent::EmailArchived {
                account_id,
                email_id,
            } => {
                tracing::info!(
                    "Processing email archive: {} in account {}",
                    email_id,
                    account_id
                );

                if let Some(operations) = &self.email_operations {
                    let operations = Arc::clone(operations);
                    let account_id = account_id.clone();
                    let email_id = *email_id;

                    tokio::spawn(async move {
                        match operations
                            .archive_email_by_id(&account_id, email_id, "INBOX")
                            .await
                        {
                            Ok(_) => {
                                tracing::info!(
                                    "Successfully archived email {} from account {}",
                                    email_id,
                                    account_id
                                );
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to archive email {} from account {}: {}",
                                    email_id,
                                    account_id,
                                    e
                                );
                            }
                        }
                    });
                } else {
                    tracing::warn!("No email operations service available for archiving");
                }
            }

            EmailEvent::EmailFlagged {
                account_id,
                email_id,
            } => {
                tracing::info!(
                    "Processing email flagging: {} in account {}",
                    email_id,
                    account_id
                );

                if let Some(operations) = &self.email_operations {
                    let operations = Arc::clone(operations);
                    let account_id = account_id.clone();
                    let email_id = *email_id;

                    tokio::spawn(async move {
                        match operations
                            .toggle_email_flag_by_id(&account_id, email_id, "INBOX")
                            .await
                        {
                            Ok(_) => {
                                tracing::info!(
                                    "Successfully flagged email {} in account {}",
                                    email_id,
                                    account_id
                                );
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to flag email {} in account {}: {}",
                                    email_id,
                                    account_id,
                                    e
                                );
                            }
                        }
                    });
                } else {
                    tracing::warn!("No email operations service available for flagging");
                }
            }

            EmailEvent::EmailComposed { draft_id } => {
                tracing::info!("Processing email composition: draft {}", draft_id);
                // Email composition is typically handled by UI components
                // The event here just logs that composition occurred
                tracing::info!("Email composition event logged for draft {}", draft_id);
            }

            EmailEvent::EmailReplied {
                original_id,
                reply_id,
            } => {
                tracing::info!(
                    "Processing email reply: {} replying to {}",
                    reply_id,
                    original_id
                );
                // Reply events are typically handled by the compose UI
                tracing::info!("Email reply event logged: {} -> {}", original_id, reply_id);
            }

            EmailEvent::EmailForwarded {
                original_id,
                forward_id,
            } => {
                tracing::info!(
                    "Processing email forward: {} forwarding {}",
                    forward_id,
                    original_id
                );
                // Forward events are typically handled by the compose UI
                tracing::info!(
                    "Email forward event logged: {} -> {}",
                    original_id,
                    forward_id
                );
            }

            EmailEvent::EmailSent {
                account_id,
                email_id,
            } => {
                tracing::info!(
                    "Processing email send: {} from account {}",
                    email_id,
                    account_id
                );
                // Email sending is typically handled by SMTP service
                // This event just confirms that sending occurred
                tracing::info!(
                    "Email sent event logged for {} in account {}",
                    email_id,
                    account_id
                );
            }

            EmailEvent::EmailReceived {
                account_id,
                email_id,
            } => {
                tracing::info!(
                    "Processing new email: {} in account {}",
                    email_id,
                    account_id
                );
                // Email receiving is handled by IMAP sync service
                // This event just logs that a new email was received
                tracing::info!("New email received: {} in account {}", email_id, account_id);
            }

            EmailEvent::SearchStarted { query, scope } => {
                tracing::info!(
                    "Processing search start: '{}' with scope {:?}",
                    query,
                    scope
                );

                if let Some(operations) = &self.email_operations {
                    let _operations = Arc::clone(operations);
                    let query = query.clone();
                    let _scope = scope.clone();

                    // Spawn async task for search operation
                    tokio::spawn(async move {
                        tracing::info!("Starting search for query: '{}'", query);

                        // TODO: Implement actual search through EmailOperationsService
                        // For now, we just log the search initiation
                        // In the future, this would:
                        // 1. Get the appropriate database/account based on scope
                        // 2. Execute the search query
                        // 3. Publish SearchCompleted or SearchFailed events

                        tracing::info!("Search initiated for: '{}'", query);
                    });
                } else {
                    tracing::warn!("No email operations service available for search");
                }
            }

            EmailEvent::SearchCompleted { query, results } => {
                tracing::info!(
                    "Processing search completion: '{}' found {} results",
                    query,
                    results.len()
                );

                // Log search completion and optionally update UI
                tracing::info!(
                    "Search completed for '{}': {} results found",
                    query,
                    results.len()
                );

                // TODO: Update search UI with results
                // This could publish additional UI events to refresh search results
            }

            EmailEvent::SearchFailed { query, error } => {
                tracing::error!(
                    "Processing search failure: '{}' failed with: {}",
                    query,
                    error
                );

                // Log search failure and optionally show error in UI
                tracing::error!("Search failed for '{}': {}", query, error);

                // TODO: Update search UI with error message
                // This could publish additional UI events to show error state
            }

            _ => {
                tracing::debug!("Unhandled email event: {:?}", event.event);
            }
        }

        Ok(())
    }

    fn priority(&self) -> HandlerPriority {
        HandlerPriority::High
    }
}

/// Calendar operations handler that processes CalendarEvent types
pub struct CalendarEventHandler {
    /// Reference to calendar manager for operations
    calendar_manager: Option<Arc<crate::calendar::CalendarManager>>,
}

impl CalendarEventHandler {
    pub fn new() -> Self {
        Self {
            calendar_manager: None,
        }
    }

    /// Set calendar manager reference
    pub fn set_calendar_manager(&mut self, calendar: Arc<crate::calendar::CalendarManager>) {
        self.calendar_manager = Some(calendar);
    }
}

impl EventHandler<CalendarEventData> for CalendarEventHandler {
    fn handle(&mut self, event: &CalendarEventData) -> Result<(), EventError> {
        match &event.event {
            CalendarEvent::EventCreated {
                calendar_id,
                event_id,
            } => {
                tracing::info!(
                    "Processing calendar event creation notification: {} in calendar {}",
                    event_id,
                    calendar_id
                );

                if let Some(_manager) = &self.calendar_manager {
                    let calendar_id = calendar_id.clone();
                    let event_id = event_id.clone();

                    // Spawn async task for post-creation processing
                    tokio::spawn(async move {
                        // Log the creation and optionally trigger UI refresh or sync
                        tracing::info!("Event {} created in calendar {}", event_id, calendar_id);

                        // Optional: Trigger calendar sync or UI refresh
                        // This could publish additional events to notify UI components
                        tracing::debug!("Event creation notification processed for {}", event_id);
                    });
                } else {
                    tracing::warn!("No calendar manager available for event creation processing");
                }
            }

            CalendarEvent::EventUpdated {
                calendar_id,
                event_id,
            } => {
                tracing::info!(
                    "Processing calendar event update notification: {} in calendar {}",
                    event_id,
                    calendar_id
                );

                if let Some(_manager) = &self.calendar_manager {
                    let calendar_id = calendar_id.clone();
                    let event_id = event_id.clone();

                    // Spawn async task for post-update processing
                    tokio::spawn(async move {
                        // Log the update and optionally trigger UI refresh
                        tracing::info!("Event {} updated in calendar {}", event_id, calendar_id);

                        // Optional: Trigger UI refresh to show updated event details
                        // This could publish additional events to notify UI components
                        tracing::debug!("Event update notification processed for {}", event_id);
                    });
                } else {
                    tracing::warn!("No calendar manager available for event update processing");
                }
            }

            CalendarEvent::EventDeleted {
                calendar_id,
                event_id,
            } => {
                tracing::info!(
                    "Processing calendar event deletion: {} in calendar {}",
                    event_id,
                    calendar_id
                );

                if let Some(manager) = &self.calendar_manager {
                    let manager = Arc::clone(manager);
                    let event_id = event_id.clone();

                    // Spawn async task for event deletion
                    tokio::spawn(async move {
                        match manager.delete_event(&event_id).await {
                            Ok(deleted) => {
                                if deleted {
                                    tracing::info!(
                                        "Successfully deleted calendar event {}",
                                        event_id
                                    );
                                } else {
                                    tracing::warn!(
                                        "Calendar event {} was not found or already deleted",
                                        event_id
                                    );
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to delete calendar event {}: {}",
                                    event_id,
                                    e
                                );
                            }
                        }
                    });
                } else {
                    tracing::warn!("No calendar manager available for event deletion");
                }
            }

            CalendarEvent::CalendarSynced {
                calendar_id,
                event_count,
            } => {
                tracing::info!(
                    "Processing calendar sync completion: {} with {} events",
                    calendar_id,
                    event_count
                );

                if let Some(_manager) = &self.calendar_manager {
                    let calendar_id = calendar_id.clone();
                    let event_count = *event_count;

                    // Spawn async task for post-sync operations
                    tokio::spawn(async move {
                        // Log successful sync completion
                        tracing::info!(
                            "Calendar {} synchronized with {} events",
                            calendar_id,
                            event_count
                        );

                        // Optional: Trigger additional sync verification or UI refresh notifications
                        // This could publish additional events to refresh UI components
                        tracing::debug!("Calendar sync completion processed for {}", calendar_id);
                    });
                } else {
                    tracing::warn!("No calendar manager available for sync processing");
                }
            }

            CalendarEvent::InvitationReceived { event_id, from } => {
                tracing::info!("Processing calendar invitation: {} from {}", event_id, from);
                // TODO: Show invitation notification/popup
                tracing::info!("Would show invitation from {} for event {}", from, event_id);
            }

            CalendarEvent::InvitationAccepted { event_id } => {
                tracing::info!("Processing invitation acceptance: {}", event_id);

                if let Some(manager) = &self.calendar_manager {
                    let manager = Arc::clone(manager);
                    let event_id = event_id.clone();

                    // Spawn async task for RSVP response
                    tokio::spawn(async move {
                        // TODO: Get actual attendee email from current user or event data
                        let attendee_email = "user@example.com"; // Placeholder
                        match manager
                            .rsvp_to_event(
                                &event_id,
                                attendee_email,
                                crate::calendar::event::AttendeeStatus::Accepted,
                            )
                            .await
                        {
                            Ok(_) => {
                                tracing::info!(
                                    "Successfully sent RSVP acceptance for event {}",
                                    event_id
                                );
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to send RSVP acceptance for event {}: {}",
                                    event_id,
                                    e
                                );
                            }
                        }
                    });
                } else {
                    tracing::warn!("No calendar manager available for RSVP acceptance");
                }
            }

            CalendarEvent::InvitationDeclined { event_id } => {
                tracing::info!("Processing invitation decline: {}", event_id);

                if let Some(manager) = &self.calendar_manager {
                    let manager = Arc::clone(manager);
                    let event_id = event_id.clone();

                    // Spawn async task for RSVP response
                    tokio::spawn(async move {
                        // TODO: Get actual attendee email from current user or event data
                        let attendee_email = "user@example.com"; // Placeholder
                        match manager
                            .rsvp_to_event(
                                &event_id,
                                attendee_email,
                                crate::calendar::event::AttendeeStatus::Declined,
                            )
                            .await
                        {
                            Ok(_) => {
                                tracing::info!(
                                    "Successfully sent RSVP decline for event {}",
                                    event_id
                                );
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to send RSVP decline for event {}: {}",
                                    event_id,
                                    e
                                );
                            }
                        }
                    });
                } else {
                    tracing::warn!("No calendar manager available for RSVP decline");
                }
            }

            _ => {
                tracing::debug!("Unhandled calendar event: {:?}", event.event);
            }
        }

        Ok(())
    }

    fn priority(&self) -> HandlerPriority {
        HandlerPriority::Normal
    }
}

/// Account operations handler that processes AccountEvent types
pub struct AccountEventHandler {
    /// Reference to account manager for operations
    imap_manager: Option<Arc<Mutex<crate::imap::ImapAccountManager>>>,
}

impl AccountEventHandler {
    pub fn new() -> Self {
        Self { imap_manager: None }
    }

    /// Set IMAP account manager reference
    pub fn set_imap_manager(&mut self, imap: Arc<Mutex<crate::imap::ImapAccountManager>>) {
        self.imap_manager = Some(imap);
    }
}

impl EventHandler<AccountEventData> for AccountEventHandler {
    fn handle(&mut self, event: &AccountEventData) -> Result<(), EventError> {
        match &event.event {
            AccountEvent::AccountSyncStarted { account_id } => {
                tracing::info!("Processing account sync start: {}", account_id);

                if let Some(_manager) = &self.imap_manager {
                    // TODO: Trigger actual sync operation
                    tracing::info!("Would start sync for account {}", account_id);
                } else {
                    tracing::warn!("No IMAP manager available for sync");
                }
            }

            AccountEvent::AccountSyncCompleted {
                account_id,
                duration_ms,
            } => {
                tracing::info!(
                    "Processing account sync completion: {} ({}ms)",
                    account_id,
                    duration_ms
                );
                // TODO: Update UI with sync results
                tracing::info!("Account {} sync completed in {}ms", account_id, duration_ms);
            }

            AccountEvent::AccountSyncFailed { account_id, error } => {
                tracing::warn!(
                    "Processing account sync failure: {} - {}",
                    account_id,
                    error
                );
                // TODO: Show error notification
                tracing::warn!("Account {} sync failed: {}", account_id, error);
            }

            AccountEvent::AccountAdded {
                account_id,
                provider,
            } => {
                tracing::info!("Processing account addition: {} ({})", account_id, provider);
                // TODO: Trigger account setup UI
                tracing::info!(
                    "Would add account {} with provider {}",
                    account_id,
                    provider
                );
            }

            AccountEvent::AccountRemoved { account_id } => {
                tracing::info!("Processing account removal: {}", account_id);

                if let Some(_manager) = &self.imap_manager {
                    // TODO: Remove account from manager
                    tracing::info!("Would remove account {}", account_id);
                } else {
                    tracing::warn!("No IMAP manager available for account removal");
                }
            }

            _ => {
                tracing::debug!("Unhandled account event: {:?}", event.event);
            }
        }

        Ok(())
    }

    fn priority(&self) -> HandlerPriority {
        HandlerPriority::High
    }
}

/// UI state handler that processes UI events and updates interface state
pub struct UIEventHandler {
    /// Current UI state tracking
    current_pane: Option<crate::events::types::FocusedPane>,
    current_mode: Option<crate::events::types::UIMode>,
}

impl UIEventHandler {
    pub fn new() -> Self {
        Self {
            current_pane: None,
            current_mode: None,
        }
    }
}

impl EventHandler<UIEventData> for UIEventHandler {
    fn handle(&mut self, event: &UIEventData) -> Result<(), EventError> {
        match &event.event {
            UIEvent::PaneChanged { from, to } => {
                tracing::info!("Processing pane change: {:?} -> {:?}", from, to);
                self.current_pane = Some(to.clone());
                // TODO: Trigger UI pane focus change
                tracing::info!("UI pane changed from {:?} to {:?}", from, to);
            }

            UIEvent::ModeChanged { from, to } => {
                tracing::info!("Processing mode change: {:?} -> {:?}", from, to);
                self.current_mode = Some(to.clone());
                // TODO: Trigger UI mode change
                tracing::info!("UI mode changed from {:?} to {:?}", from, to);
            }

            UIEvent::KeyPressed { key } => {
                tracing::debug!("Processing key press: {:?}", key);
                // Key events are handled by UI integration, but we can track them here
            }

            UIEvent::WindowResized { new_size } => {
                tracing::info!("Processing window resize: {:?}", new_size);
                // TODO: Trigger UI layout recalculation
                tracing::info!("Window resized to {:?}", new_size);
            }

            UIEvent::ThemeChanged { theme_name } => {
                tracing::info!("Processing theme change: {}", theme_name);
                // TODO: Apply new theme
                tracing::info!("Theme changed to: {}", theme_name);
            }

            _ => {
                tracing::debug!("Unhandled UI event: {:?}", event.event);
            }
        }

        Ok(())
    }

    fn priority(&self) -> HandlerPriority {
        HandlerPriority::High
    }
}

/// Application event handler for system-level events
pub struct ApplicationEventHandler;

impl ApplicationEventHandler {
    pub fn new() -> Self {
        Self
    }
}

impl EventHandler<AppEventData> for ApplicationEventHandler {
    fn handle(&mut self, event: &AppEventData) -> Result<(), EventError> {
        match &event.event {
            AppEvent::AppShuttingDown => {
                tracing::info!("Processing application shutdown");
                // TODO: Trigger graceful shutdown sequence
                tracing::info!("Application shutdown requested");
            }

            AppEvent::AppStarted {
                version,
                startup_time_ms,
            } => {
                tracing::info!(
                    "Processing application startup: v{} ({}ms)",
                    version,
                    startup_time_ms
                );
                // TODO: Complete startup initialization
                tracing::info!("Application v{} started in {}ms", version, startup_time_ms);
            }

            AppEvent::ConfigLoaded { config_path } => {
                tracing::info!("Processing config load: {}", config_path);
                // TODO: Apply loaded configuration
                tracing::info!("Configuration loaded from: {}", config_path);
            }

            _ => {
                tracing::debug!("Unhandled app event: {:?}", event.event);
            }
        }

        Ok(())
    }

    fn priority(&self) -> HandlerPriority {
        HandlerPriority::Critical
    }
}

/// Event handler registration and management
pub struct EventHandlerRegistry {
    email_handler: Arc<Mutex<EmailEventHandler>>,
    calendar_handler: Arc<Mutex<CalendarEventHandler>>,
    account_handler: Arc<Mutex<AccountEventHandler>>,
    ui_handler: Arc<Mutex<UIEventHandler>>,
    app_handler: Arc<Mutex<ApplicationEventHandler>>,
}

impl EventHandlerRegistry {
    pub fn new() -> Self {
        Self {
            email_handler: Arc::new(Mutex::new(EmailEventHandler::new())),
            calendar_handler: Arc::new(Mutex::new(CalendarEventHandler::new())),
            account_handler: Arc::new(Mutex::new(AccountEventHandler::new())),
            ui_handler: Arc::new(Mutex::new(UIEventHandler::new())),
            app_handler: Arc::new(Mutex::new(ApplicationEventHandler::new())),
        }
    }

    /// Register all event handlers with the event bus
    pub fn register_all_handlers(&self) -> Result<(), EventError> {
        let bus = initialize_event_bus();
        let mut bus_lock = bus
            .lock()
            .map_err(|_| EventError::ProcessingFailed("Failed to lock event bus".to_string()))?;

        // Register email event handler
        bus_lock.subscribe::<EmailEventData, _>(EmailEventHandlerWrapper {
            handler: self.email_handler.clone(),
        });

        // Register calendar event handler
        bus_lock.subscribe::<CalendarEventData, _>(CalendarEventHandlerWrapper {
            handler: self.calendar_handler.clone(),
        });

        // Register account event handler
        bus_lock.subscribe::<AccountEventData, _>(AccountEventHandlerWrapper {
            handler: self.account_handler.clone(),
        });

        // Register UI event handler
        bus_lock.subscribe::<UIEventData, _>(UIEventHandlerWrapper {
            handler: self.ui_handler.clone(),
        });

        // Register app event handler
        bus_lock.subscribe::<AppEventData, _>(AppEventHandlerWrapper {
            handler: self.app_handler.clone(),
        });

        tracing::info!("All event handlers registered successfully");
        Ok(())
    }

    /// Get references to handlers for configuration
    pub fn email_handler(&self) -> Arc<Mutex<EmailEventHandler>> {
        self.email_handler.clone()
    }

    pub fn calendar_handler(&self) -> Arc<Mutex<CalendarEventHandler>> {
        self.calendar_handler.clone()
    }

    pub fn account_handler(&self) -> Arc<Mutex<AccountEventHandler>> {
        self.account_handler.clone()
    }

    /// Configure email operations service for all handlers that need it
    pub fn set_email_operations(&self, operations: Arc<crate::email::EmailOperationsService>) {
        if let Ok(mut handler) = self.email_handler.lock() {
            handler.set_email_operations(operations);
        }
    }
}

/// Wrapper for email event handler to implement EventHandler trait
struct EmailEventHandlerWrapper {
    handler: Arc<Mutex<EmailEventHandler>>,
}

impl EventHandler<EmailEventData> for EmailEventHandlerWrapper {
    fn handle(&mut self, event: &EmailEventData) -> Result<(), EventError> {
        let mut handler = match self.handler.lock() {
            Ok(handler) => handler,
            Err(_) => {
                return Err(EventError::ProcessingFailed(
                    "Failed to lock email handler".to_string(),
                ))
            }
        };
        handler.handle(event)
    }

    fn priority(&self) -> HandlerPriority {
        HandlerPriority::High
    }
}

/// Wrapper for calendar event handler
struct CalendarEventHandlerWrapper {
    handler: Arc<Mutex<CalendarEventHandler>>,
}

impl EventHandler<CalendarEventData> for CalendarEventHandlerWrapper {
    fn handle(&mut self, event: &CalendarEventData) -> Result<(), EventError> {
        let mut handler = match self.handler.lock() {
            Ok(handler) => handler,
            Err(_) => {
                return Err(EventError::ProcessingFailed(
                    "Failed to lock calendar handler".to_string(),
                ))
            }
        };
        handler.handle(event)
    }

    fn priority(&self) -> HandlerPriority {
        HandlerPriority::Normal
    }
}

/// Wrapper for account event handler
struct AccountEventHandlerWrapper {
    handler: Arc<Mutex<AccountEventHandler>>,
}

impl EventHandler<AccountEventData> for AccountEventHandlerWrapper {
    fn handle(&mut self, event: &AccountEventData) -> Result<(), EventError> {
        let mut handler = match self.handler.lock() {
            Ok(handler) => handler,
            Err(_) => {
                return Err(EventError::ProcessingFailed(
                    "Failed to lock account handler".to_string(),
                ))
            }
        };
        handler.handle(event)
    }

    fn priority(&self) -> HandlerPriority {
        HandlerPriority::High
    }
}

/// Wrapper for UI event handler
struct UIEventHandlerWrapper {
    handler: Arc<Mutex<UIEventHandler>>,
}

impl EventHandler<UIEventData> for UIEventHandlerWrapper {
    fn handle(&mut self, event: &UIEventData) -> Result<(), EventError> {
        let mut handler = match self.handler.lock() {
            Ok(handler) => handler,
            Err(_) => {
                return Err(EventError::ProcessingFailed(
                    "Failed to lock UI handler".to_string(),
                ))
            }
        };
        handler.handle(event)
    }

    fn priority(&self) -> HandlerPriority {
        HandlerPriority::High
    }
}

/// Wrapper for app event handler
struct AppEventHandlerWrapper {
    handler: Arc<Mutex<ApplicationEventHandler>>,
}

impl EventHandler<AppEventData> for AppEventHandlerWrapper {
    fn handle(&mut self, event: &AppEventData) -> Result<(), EventError> {
        let mut handler = match self.handler.lock() {
            Ok(handler) => handler,
            Err(_) => {
                return Err(EventError::ProcessingFailed(
                    "Failed to lock app handler".to_string(),
                ))
            }
        };
        handler.handle(event)
    }

    fn priority(&self) -> HandlerPriority {
        HandlerPriority::Critical
    }
}

/// Global function to register all event handlers
/// Creates an EventHandlerRegistry and registers all handlers with the event bus
pub fn register_all_handlers() -> Result<(), EventError> {
    let registry = EventHandlerRegistry::new();
    registry.register_all_handlers()
}
