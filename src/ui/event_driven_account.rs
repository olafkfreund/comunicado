//! Event-Driven Account Management Operations
//!
//! This module provides event-driven implementations for account management operations,
//! replacing direct method calls with event publishing to decouple components.

use crate::events::types::{AccountEvent, AccountEventData};
use crate::events::{publish, EventError};
use std::time::Instant;

/// Event-driven account operations handler
pub struct EventDrivenAccountHandler {
    current_account_id: Option<String>,
    selected_provider: Option<String>,
}

impl EventDrivenAccountHandler {
    pub fn new() -> Self {
        Self {
            current_account_id: None,
            selected_provider: None,
        }
    }

    /// Set the currently active account
    pub fn set_current_account(&mut self, account_id: String) {
        self.current_account_id = Some(account_id);
    }

    /// Set the currently selected provider
    pub fn set_selected_provider(&mut self, provider: String) {
        self.selected_provider = Some(provider);
    }

    /// Update the status for an account (placeholder implementation)
    pub fn update_status(&mut self, account_id: String, _status: String) {
        // Placeholder implementation for future account status tracking
        self.current_account_id = Some(account_id);
    }

    /// Add a new account using events
    pub fn add_account(&self, account_id: String, provider: String) -> Result<(), EventError> {
        let event = AccountEventData::new(AccountEvent::AccountAdded {
            account_id: account_id.clone(),
            provider: provider.clone(),
        });

        publish(event)?;

        tracing::info!(
            "Published account addition for account {} (provider: {})",
            account_id,
            provider
        );
        Ok(())
    }

    /// Remove an account using events
    pub fn remove_account(&self, account_id: String) -> Result<(), EventError> {
        let event = AccountEventData::new(AccountEvent::AccountRemoved {
            account_id: account_id.clone(),
        });

        publish(event)?;

        tracing::info!("Published account removal for account {}", account_id);
        Ok(())
    }

    /// Remove the currently selected account
    pub fn remove_current_account(&self) -> Result<(), EventError> {
        if let Some(account_id) = &self.current_account_id {
            self.remove_account(account_id.clone())?;
        }
        Ok(())
    }

    /// Update an existing account using events
    pub fn update_account(&self, account_id: String) -> Result<(), EventError> {
        let event = AccountEventData::new(AccountEvent::AccountUpdated {
            account_id: account_id.clone(),
        });

        publish(event)?;

        tracing::info!("Published account update for account {}", account_id);
        Ok(())
    }

    /// Connect to an account using events
    pub fn connect_account(&self, account_id: String) -> Result<(), EventError> {
        let event = AccountEventData::new(AccountEvent::AccountConnected {
            account_id: account_id.clone(),
        });

        publish(event)?;

        tracing::info!("Published account connection for account {}", account_id);
        Ok(())
    }

    /// Disconnect from an account using events
    pub fn disconnect_account(&self, account_id: String) -> Result<(), EventError> {
        let event = AccountEventData::new(AccountEvent::AccountDisconnected {
            account_id: account_id.clone(),
        });

        publish(event)?;

        tracing::info!("Published account disconnection for account {}", account_id);
        Ok(())
    }

    /// Start account sync using events
    pub fn start_account_sync(&self, account_id: String) -> Result<(), EventError> {
        let event = AccountEventData::new(AccountEvent::AccountSyncStarted {
            account_id: account_id.clone(),
        });

        publish(event)?;

        tracing::info!("Published account sync start for account {}", account_id);
        Ok(())
    }

    /// Complete account sync using events
    pub fn complete_account_sync(
        &self,
        account_id: String,
        start_time: Instant,
    ) -> Result<(), EventError> {
        let duration_ms = start_time.elapsed().as_millis() as u64;
        let event = AccountEventData::new(AccountEvent::AccountSyncCompleted {
            account_id: account_id.clone(),
            duration_ms,
        });

        publish(event)?;

        tracing::info!(
            "Published account sync completion for account {} ({}ms)",
            account_id,
            duration_ms
        );
        Ok(())
    }

    /// Fail account sync using events
    pub fn fail_account_sync(&self, account_id: String, error: String) -> Result<(), EventError> {
        let event = AccountEventData::new(AccountEvent::AccountSyncFailed {
            account_id: account_id.clone(),
            error: error.clone(),
        });

        publish(event)?;

        tracing::warn!(
            "Published account sync failure for account {}: {}",
            account_id,
            error
        );
        Ok(())
    }

    /// Refresh account authentication using events
    pub fn refresh_account_auth(&self, account_id: String) -> Result<(), EventError> {
        let event = AccountEventData::new(AccountEvent::AccountAuthRefreshed {
            account_id: account_id.clone(),
        });

        publish(event)?;

        tracing::info!("Published auth refresh for account {}", account_id);
        Ok(())
    }

    /// Fail account authentication using events
    pub fn fail_account_auth(&self, account_id: String, error: String) -> Result<(), EventError> {
        let event = AccountEventData::new(AccountEvent::AccountAuthFailed {
            account_id: account_id.clone(),
            error: error.clone(),
        });

        publish(event)?;

        tracing::warn!(
            "Published auth failure for account {}: {}",
            account_id,
            error
        );
        Ok(())
    }
}

/// Event-driven account state management
pub struct EventDrivenAccountState {
    current_account: Option<String>,
    connected_accounts: Vec<String>,
    syncing_accounts: Vec<String>,
}

impl EventDrivenAccountState {
    pub fn new() -> Self {
        Self {
            current_account: None,
            connected_accounts: Vec::new(),
            syncing_accounts: Vec::new(),
        }
    }

    /// Change the current account
    pub fn change_current_account(&mut self, account_id: String) -> Result<(), EventError> {
        self.current_account = Some(account_id.clone());
        // TODO: Publish account change event
        tracing::debug!("Changed current account to: {}", account_id);
        Ok(())
    }

    /// Set the active account (simpler interface for the change_current_account method)
    pub fn set_active_account(&mut self, account_id: String) {
        self.current_account = Some(account_id);
    }

    /// Mark account as connected
    pub fn mark_account_connected(&mut self, account_id: String) -> Result<(), EventError> {
        if !self.connected_accounts.contains(&account_id) {
            self.connected_accounts.push(account_id.clone());
        }
        tracing::debug!("Marked account as connected: {}", account_id);
        Ok(())
    }

    /// Mark account as disconnected
    pub fn mark_account_disconnected(&mut self, account_id: String) -> Result<(), EventError> {
        self.connected_accounts.retain(|id| id != &account_id);
        tracing::debug!("Marked account as disconnected: {}", account_id);
        Ok(())
    }

    /// Mark account as syncing
    pub fn mark_account_syncing(&mut self, account_id: String) -> Result<(), EventError> {
        if !self.syncing_accounts.contains(&account_id) {
            self.syncing_accounts.push(account_id.clone());
        }
        tracing::debug!("Marked account as syncing: {}", account_id);
        Ok(())
    }

    /// Mark account sync as complete
    pub fn mark_account_sync_complete(&mut self, account_id: String) -> Result<(), EventError> {
        self.syncing_accounts.retain(|id| id != &account_id);
        tracing::debug!("Marked account sync complete: {}", account_id);
        Ok(())
    }

    /// Get current account
    pub fn current_account(&self) -> &Option<String> {
        &self.current_account
    }

    /// Get connected accounts
    pub fn connected_accounts(&self) -> &Vec<String> {
        &self.connected_accounts
    }

    /// Get syncing accounts
    pub fn syncing_accounts(&self) -> &Vec<String> {
        &self.syncing_accounts
    }

    /// Check if account is connected
    pub fn is_account_connected(&self, account_id: &str) -> bool {
        self.connected_accounts.contains(&account_id.to_string())
    }

    /// Check if account is syncing
    pub fn is_account_syncing(&self, account_id: &str) -> bool {
        self.syncing_accounts.contains(&account_id.to_string())
    }
}

/// Account action types for command processing migration
#[derive(Debug, Clone)]
pub enum AccountAction {
    AddAccount {
        account_id: String,
        provider: String,
    },
    RemoveAccount {
        account_id: String,
    },
    UpdateAccount {
        account_id: String,
    },
    ConnectAccount {
        account_id: String,
    },
    DisconnectAccount {
        account_id: String,
    },
    SyncAccount {
        account_id: String,
    },
    RefreshAuth {
        account_id: String,
    },
    SwitchAccount {
        account_id: String,
    },
}

/// Migration helper for account command actions
pub struct AccountMigrationHelper;

impl AccountMigrationHelper {
    /// Handle account-related command actions with event-driven system
    pub fn handle_account_action(
        action: &AccountAction,
        account_handler: &EventDrivenAccountHandler,
        account_state: &mut EventDrivenAccountState,
    ) -> Result<(), EventError> {
        match action {
            AccountAction::AddAccount {
                account_id,
                provider,
            } => {
                account_handler.add_account(account_id.clone(), provider.clone())?;
            }
            AccountAction::RemoveAccount { account_id } => {
                account_handler.remove_account(account_id.clone())?;
                account_state.mark_account_disconnected(account_id.clone())?;
            }
            AccountAction::UpdateAccount { account_id } => {
                account_handler.update_account(account_id.clone())?;
            }
            AccountAction::ConnectAccount { account_id } => {
                account_handler.connect_account(account_id.clone())?;
                account_state.mark_account_connected(account_id.clone())?;
            }
            AccountAction::DisconnectAccount { account_id } => {
                account_handler.disconnect_account(account_id.clone())?;
                account_state.mark_account_disconnected(account_id.clone())?;
            }
            AccountAction::SyncAccount { account_id } => {
                account_handler.start_account_sync(account_id.clone())?;
                account_state.mark_account_syncing(account_id.clone())?;
            }
            AccountAction::RefreshAuth { account_id } => {
                account_handler.refresh_account_auth(account_id.clone())?;
            }
            AccountAction::SwitchAccount { account_id } => {
                account_state.change_current_account(account_id.clone())?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::initialize_event_bus;

    #[test]
    fn test_event_driven_account_handler() {
        let _bus = initialize_event_bus();

        let handler = EventDrivenAccountHandler::new();

        // Test account addition
        assert!(handler
            .add_account("acc1".to_string(), "gmail".to_string())
            .is_ok());

        // Test account connection
        assert!(handler.connect_account("acc1".to_string()).is_ok());

        // Test account sync
        assert!(handler.start_account_sync("acc1".to_string()).is_ok());

        // Test account removal
        assert!(handler.remove_account("acc1".to_string()).is_ok());
    }

    #[test]
    fn test_event_driven_account_state() {
        let mut state = EventDrivenAccountState::new();

        assert!(state.current_account().is_none());

        assert!(state.change_current_account("acc1".to_string()).is_ok());
        assert_eq!(state.current_account(), &Some("acc1".to_string()));

        assert!(state.mark_account_connected("acc1".to_string()).is_ok());
        assert!(state.is_account_connected("acc1"));

        assert!(state.mark_account_syncing("acc1".to_string()).is_ok());
        assert!(state.is_account_syncing("acc1"));
    }
}
