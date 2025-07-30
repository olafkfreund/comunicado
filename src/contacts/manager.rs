#[cfg(test)]
use crate::contacts::ContactEmail;
use crate::contacts::{
    AddressBookStats, Contact, ContactSearchCriteria, ContactSource, ContactsDatabase,
    ContactsError, ContactsProvider, ContactsResult, GoogleContactsProvider,
    OutlookContactsProvider,
};
use crate::oauth2::TokenManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Central contacts manager that orchestrates all contact operations
pub struct ContactsManager {
    database: ContactsDatabase,
    providers: HashMap<String, Box<dyn ContactsProvider>>,
    token_manager: TokenManager,
    sync_enabled: Arc<RwLock<bool>>,
}

impl ContactsManager {
    /// Create a new contacts manager
    pub async fn new(
        database: ContactsDatabase,
        token_manager: TokenManager,
    ) -> ContactsResult<Self> {
        let mut providers: HashMap<String, Box<dyn ContactsProvider>> = HashMap::new();

        // Initialize Google Contacts provider
        let google_provider = GoogleContactsProvider::new(token_manager.clone());
        providers.insert("google".to_string(), Box::new(google_provider));

        // Initialize Outlook Contacts provider
        let outlook_provider = OutlookContactsProvider::new(token_manager.clone());
        providers.insert("outlook".to_string(), Box::new(outlook_provider));

        Ok(Self {
            database,
            providers,
            token_manager,
            sync_enabled: Arc::new(RwLock::new(true)),
        })
    }

    /// Search contacts across all sources
    pub async fn search_contacts(
        &self,
        criteria: &ContactSearchCriteria,
    ) -> ContactsResult<Vec<Contact>> {
        self.database.search_contacts(criteria).await
    }

    /// Get a specific contact by ID
    pub async fn get_contact(&self, id: i64) -> ContactsResult<Option<Contact>> {
        self.database.get_contact(id).await
    }

    /// Find contact by email address
    pub async fn find_contact_by_email(&self, email: &str) -> ContactsResult<Option<Contact>> {
        self.database.find_contact_by_email(email).await
    }

    /// Find contacts by partial email match (for autocomplete)
    pub async fn find_contacts_by_email_prefix(&self, email_prefix: &str, limit: usize) -> ContactsResult<Vec<Contact>> {
        self.database.find_contacts_by_email_prefix(email_prefix, limit).await
    }

    /// Create a new contact
    pub async fn create_contact(&self, mut contact: Contact) -> ContactsResult<Contact> {
        // Store locally first
        self.database.store_contact(&mut contact).await?;

        // Sync to remote provider if applicable
        if let Some(account_id) = contact.source.account_id() {
            if let Some(provider) = self.get_provider_for_source(&contact.source) {
                match provider.create_contact(account_id, &contact).await {
                    Ok(remote_contact) => {
                        // Update local contact with remote data
                        let mut updated_contact = remote_contact;
                        updated_contact.id = contact.id; // Keep local ID
                        updated_contact.synced_at = Some(chrono::Utc::now());
                        self.database.store_contact(&mut updated_contact).await?;
                        return Ok(updated_contact);
                    }
                    Err(e) => {
                        tracing::error!("Failed to sync contact to remote provider: {}", e);
                        // Continue with local contact
                    }
                }
            }
        }

        Ok(contact)
    }

    /// Update an existing contact
    pub async fn update_contact(&self, mut contact: Contact) -> ContactsResult<Contact> {
        // Update locally first
        contact.updated_at = chrono::Utc::now();
        self.database.store_contact(&mut contact).await?;

        // Sync to remote provider if applicable
        if let Some(account_id) = contact.source.account_id() {
            if let Some(provider) = self.get_provider_for_source(&contact.source) {
                match provider.update_contact(account_id, &contact).await {
                    Ok(remote_contact) => {
                        // Update local contact with remote data
                        let mut updated_contact = remote_contact;
                        updated_contact.id = contact.id; // Keep local ID
                        updated_contact.synced_at = Some(chrono::Utc::now());
                        self.database.store_contact(&mut updated_contact).await?;
                        return Ok(updated_contact);
                    }
                    Err(e) => {
                        tracing::error!("Failed to sync contact update to remote provider: {}", e);
                        // Continue with local contact
                    }
                }
            }
        }

        Ok(contact)
    }

    /// Delete a contact
    pub async fn delete_contact(&self, id: i64) -> ContactsResult<()> {
        // Get the contact first to check if we need to delete from remote
        let contact = self.database.get_contact(id).await?;

        if let Some(contact) = contact {
            // Delete from remote provider if applicable
            if let Some(account_id) = contact.source.account_id() {
                if let Some(provider) = self.get_provider_for_source(&contact.source) {
                    if let Err(e) = provider
                        .delete_contact(account_id, &contact.external_id)
                        .await
                    {
                        tracing::error!("Failed to delete contact from remote provider: {}", e);
                        // Continue with local deletion
                    }
                }
            }
        }

        // Delete locally
        self.database.delete_contact(id).await
    }

    /// Sync all contacts from all configured accounts
    pub async fn sync_all_contacts(&self) -> ContactsResult<SyncSummary> {
        let sync_enabled = *self.sync_enabled.read().await;
        if !sync_enabled {
            return Err(ContactsError::SyncError("Sync is disabled".to_string()));
        }

        let mut summary = SyncSummary::default();

        // Get all configured accounts
        let account_ids = self.token_manager.get_account_ids().await;

        for account_id in account_ids {
            // Infer provider from account ID format (provider_email_com)
            let provider_type = if account_id.starts_with("gmail_") {
                "google"
            } else if account_id.starts_with("outlook_") {
                "outlook"
            } else {
                continue; // Skip unknown providers
            };

            match self.sync_account_contacts(&account_id, provider_type).await {
                Ok(account_summary) => {
                    summary.merge(account_summary);
                }
                Err(e) => {
                    tracing::error!("Failed to sync contacts for account {}: {}", account_id, e);
                    summary
                        .errors
                        .push(format!("Account {}: {}", account_id, e));
                }
            }
        }

        summary.total_duration = summary.start_time.elapsed();
        Ok(summary)
    }

    /// Sync contacts for a specific account
    pub async fn sync_account_contacts(
        &self,
        account_id: &str,
        provider_type: &str,
    ) -> ContactsResult<SyncSummary> {
        let provider = self.providers.get(provider_type).ok_or_else(|| {
            ContactsError::SyncError(format!("Unknown provider: {}", provider_type))
        })?;

        let mut summary = SyncSummary::new(account_id.to_string());

        // Fetch contacts from remote provider
        let remote_contacts = provider.fetch_contacts(account_id).await?;
        summary.fetched_count = remote_contacts.len();

        // Process each contact
        for remote_contact in remote_contacts {
            let contact_id = remote_contact.external_id.clone();
            match self
                .process_contact_sync(remote_contact, account_id, provider_type)
                .await
            {
                Ok(SyncResult::Created) => summary.created_count += 1,
                Ok(SyncResult::Updated) => summary.updated_count += 1,
                Ok(SyncResult::Skipped) => summary.skipped_count += 1,
                Err(e) => {
                    summary
                        .errors
                        .push(format!("Contact {}: {}", contact_id, e));
                }
            }
        }

        summary.total_duration = summary.start_time.elapsed();
        Ok(summary)
    }

    /// Get address book statistics
    pub async fn get_stats(&self) -> ContactsResult<AddressBookStats> {
        self.database.get_stats().await
    }

    /// Enable or disable automatic sync
    pub async fn set_sync_enabled(&self, enabled: bool) {
        let mut sync_enabled = self.sync_enabled.write().await;
        *sync_enabled = enabled;
    }

    /// Check if sync is enabled
    pub async fn is_sync_enabled(&self) -> bool {
        *self.sync_enabled.read().await
    }

    /// Search contacts by email address (for autocomplete)
    pub async fn search_by_email(
        &self,
        email_query: &str,
        limit: Option<usize>,
    ) -> ContactsResult<Vec<Contact>> {
        let criteria = ContactSearchCriteria::new()
            .with_email(email_query.to_string())
            .with_limit(limit.unwrap_or(10));

        self.search_contacts(&criteria).await
    }


    /// Get contacts for a specific account
    pub async fn get_account_contacts(
        &self,
        account_id: &str,
        provider_type: &str,
    ) -> ContactsResult<Vec<Contact>> {
        let source = match provider_type {
            "google" => ContactSource::Google {
                account_id: account_id.to_string(),
            },
            "outlook" => ContactSource::Outlook {
                account_id: account_id.to_string(),
            },
            _ => {
                return Err(ContactsError::InvalidData(format!(
                    "Unknown provider: {}",
                    provider_type
                )))
            }
        };

        let criteria = ContactSearchCriteria::new().with_source(source);
        self.search_contacts(&criteria).await
    }

    /// Get frequently contacted people (for email composition suggestions)
    pub async fn get_frequent_contacts(&self, limit: usize) -> ContactsResult<Vec<Contact>> {
        // TODO: Implement frequency tracking based on email interactions
        // For now, just return recent contacts
        let criteria = ContactSearchCriteria::new().with_limit(limit);
        self.search_contacts(&criteria).await
    }

    /// Process individual contact sync
    async fn process_contact_sync(
        &self,
        mut remote_contact: Contact,
        account_id: &str,
        provider_type: &str,
    ) -> ContactsResult<SyncResult> {
        // Check if contact already exists locally
        let source = match provider_type {
            "google" => ContactSource::Google {
                account_id: account_id.to_string(),
            },
            "outlook" => ContactSource::Outlook {
                account_id: account_id.to_string(),
            },
            _ => {
                return Err(ContactsError::InvalidData(format!(
                    "Unknown provider: {}",
                    provider_type
                )))
            }
        };

        // Search for existing contact by external_id and source
        let criteria = ContactSearchCriteria::new().with_source(source);
        let existing_contacts = self.search_contacts(&criteria).await?;

        let existing_contact = existing_contacts
            .iter()
            .find(|c| c.external_id == remote_contact.external_id);

        match existing_contact {
            Some(local_contact) => {
                // Contact exists, check if update is needed
                if self.needs_update(local_contact, &remote_contact) {
                    remote_contact.id = local_contact.id;
                    remote_contact.created_at = local_contact.created_at;
                    remote_contact.synced_at = Some(chrono::Utc::now());

                    self.database.store_contact(&mut remote_contact).await?;
                    Ok(SyncResult::Updated)
                } else {
                    Ok(SyncResult::Skipped)
                }
            }
            None => {
                // New contact, store it
                remote_contact.synced_at = Some(chrono::Utc::now());
                self.database.store_contact(&mut remote_contact).await?;
                Ok(SyncResult::Created)
            }
        }
    }

    /// Check if a contact needs to be updated
    fn needs_update(&self, local: &Contact, remote: &Contact) -> bool {
        // Compare important fields to determine if update is needed
        local.display_name != remote.display_name
            || local.first_name != remote.first_name
            || local.last_name != remote.last_name
            || local.company != remote.company
            || local.job_title != remote.job_title
            || local.emails != remote.emails
            || local.phones != remote.phones
            || local.etag != remote.etag
            || local.updated_at < remote.updated_at
    }

    /// Get provider for a contact source
    fn get_provider_for_source(
        &self,
        source: &ContactSource,
    ) -> Option<&Box<dyn ContactsProvider>> {
        match source {
            ContactSource::Google { .. } => self.providers.get("google"),
            ContactSource::Outlook { .. } => self.providers.get("outlook"),
            ContactSource::Local => None,
        }
    }
}

/// Result of syncing a single contact
#[derive(Debug, Clone, PartialEq)]
enum SyncResult {
    Created,
    Updated,
    Skipped,
}

/// Summary of a sync operation
#[derive(Debug, Clone)]
pub struct SyncSummary {
    pub account_id: String,
    pub start_time: std::time::Instant,
    pub total_duration: std::time::Duration,
    pub fetched_count: usize,
    pub created_count: usize,
    pub updated_count: usize,
    pub skipped_count: usize,
    pub errors: Vec<String>,
}

impl SyncSummary {
    fn new(account_id: String) -> Self {
        Self {
            account_id,
            start_time: std::time::Instant::now(),
            total_duration: std::time::Duration::default(),
            fetched_count: 0,
            created_count: 0,
            updated_count: 0,
            skipped_count: 0,
            errors: Vec::new(),
        }
    }

    fn merge(&mut self, other: SyncSummary) {
        self.fetched_count += other.fetched_count;
        self.created_count += other.created_count;
        self.updated_count += other.updated_count;
        self.skipped_count += other.skipped_count;
        self.errors.extend(other.errors);
        self.total_duration += other.total_duration;
    }

    pub fn success_count(&self) -> usize {
        self.created_count + self.updated_count + self.skipped_count
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

impl Default for SyncSummary {
    fn default() -> Self {
        Self::new("all".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_needs_update() {
        let database = ContactsDatabase::new(":memory:").await.unwrap();
        let token_manager = TokenManager::new();
        let manager = ContactsManager::new(database, token_manager).await.unwrap();

        let mut contact1 = Contact::new(
            "test-id".to_string(),
            ContactSource::Google {
                account_id: "test@gmail.com".to_string(),
            },
            "John Doe".to_string(),
        );

        let mut contact2 = contact1.clone();

        // Same contacts should not need update
        assert!(!manager.needs_update(&contact1, &contact2));

        // Different display name should need update
        contact2.display_name = "Jane Doe".to_string();
        assert!(manager.needs_update(&contact1, &contact2));

        // Different emails should need update
        contact2.display_name = contact1.display_name.clone(); // Reset
        contact2.emails.push(ContactEmail::new(
            "john@example.com".to_string(),
            "work".to_string(),
        ));
        assert!(manager.needs_update(&contact1, &contact2));

        // Different etag should need update
        contact2.emails = contact1.emails.clone(); // Reset
        contact2.etag = Some("different-etag".to_string());
        assert!(manager.needs_update(&contact1, &contact2));
    }

    #[test]
    fn test_sync_summary() {
        let mut summary1 = SyncSummary::new("account1".to_string());
        summary1.created_count = 5;
        summary1.updated_count = 3;
        summary1.errors.push("Error 1".to_string());

        let mut summary2 = SyncSummary::new("account2".to_string());
        summary2.created_count = 2;
        summary2.updated_count = 1;
        summary2.errors.push("Error 2".to_string());

        summary1.merge(summary2);

        assert_eq!(summary1.created_count, 7);
        assert_eq!(summary1.updated_count, 4);
        assert_eq!(summary1.success_count(), 11);
        assert_eq!(summary1.errors.len(), 2);
        assert!(summary1.has_errors());
    }
}
