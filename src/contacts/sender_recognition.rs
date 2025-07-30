use crate::contacts::{Contact, ContactsManager, ContactsResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Information about a recognized email sender
#[derive(Debug, Clone)]
pub struct SenderInfo {
    pub display_name: String,
    pub contact_name: Option<String>,
    pub company: Option<String>,
    pub is_known_contact: bool,
    pub contact_id: Option<i64>,
}

impl SenderInfo {
    /// Create sender info from raw email address (unknown contact)
    pub fn from_email(email: &str) -> Self {
        // Extract name from email if it exists (e.g., "John Doe <john@example.com>")
        let display_name = if email.contains('<') {
            let parts: Vec<&str> = email.split('<').collect();
            if parts.len() > 1 {
                parts[0].trim().trim_matches('"').to_string()
            } else {
                email.to_string()
            }
        } else {
            email.to_string()
        };

        Self {
            display_name,
            contact_name: None,
            company: None,
            is_known_contact: false,
            contact_id: None,
        }
    }

    /// Create sender info from a known contact
    pub fn from_contact(contact: &Contact, _email: &str) -> Self {
        let display_name = if !contact.display_name.is_empty() {
            contact.display_name.clone()
        } else {
            contact.full_name()
        };

        Self {
            display_name: display_name.clone(),
            contact_name: Some(contact.full_name()),
            company: contact.company.clone(),
            is_known_contact: true,
            contact_id: contact.id,
        }
    }

    /// Get the best display name for UI purposes
    pub fn best_display_name(&self) -> &str {
        if self.is_known_contact {
            if let Some(ref contact_name) = self.contact_name {
                if !contact_name.is_empty() {
                    return contact_name;
                }
            }
        }
        &self.display_name
    }

    /// Get a short display name (for limited space)
    pub fn short_display_name(&self, max_length: usize) -> String {
        let name = self.best_display_name();
        if name.len() <= max_length {
            name.to_string()
        } else {
            format!("{}...", &name[..max_length.saturating_sub(3)])
        }
    }

    /// Get company information if available
    pub fn company_info(&self) -> Option<&str> {
        self.company.as_deref()
    }
}

/// Service for recognizing email senders and looking up contact information
pub struct SenderRecognitionService {
    contacts_manager: Arc<ContactsManager>,
    // Cache to avoid frequent database lookups
    cache: Arc<RwLock<HashMap<String, SenderInfo>>>,
    cache_size_limit: usize,
}

impl SenderRecognitionService {
    /// Create a new sender recognition service
    pub fn new(contacts_manager: Arc<ContactsManager>) -> Self {
        Self {
            contacts_manager,
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_size_limit: 500, // Cache up to 500 sender lookups
        }
    }

    /// Look up sender information by email address
    pub async fn lookup_sender(&self, email: &str) -> ContactsResult<SenderInfo> {
        // Normalize email address (lowercase, trim)
        let normalized_email = email.trim().to_lowercase();

        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(sender_info) = cache.get(&normalized_email) {
                return Ok(sender_info.clone());
            }
        }

        // Extract actual email address from "Name <email>" format
        let email_address = self.extract_email_address(&normalized_email);

        // Look up in contacts database
        let sender_info = match self.contacts_manager.find_contact_by_email(&email_address).await? {
            Some(contact) => SenderInfo::from_contact(&contact, &email_address),
            None => SenderInfo::from_email(email),
        };

        // Update cache
        self.update_cache(normalized_email, sender_info.clone()).await;

        Ok(sender_info)
    }

    /// Look up multiple senders at once (batch operation)
    pub async fn lookup_senders(&self, emails: &[String]) -> ContactsResult<HashMap<String, SenderInfo>> {
        let mut results = HashMap::new();

        for email in emails {
            let sender_info = self.lookup_sender(email).await?;
            results.insert(email.clone(), sender_info);
        }

        Ok(results)
    }

    /// Extract the email address from formats like "John Doe <john@example.com>"
    fn extract_email_address(&self, full_email: &str) -> String {
        if let Some(start) = full_email.find('<') {
            if let Some(end) = full_email.find('>') {
                return full_email[start + 1..end].to_string();
            }
        }
        full_email.to_string()
    }

    /// Update the cache with new sender information
    async fn update_cache(&self, email: String, sender_info: SenderInfo) {
        let mut cache = self.cache.write().await;

        // If cache is at limit, remove oldest entries
        if cache.len() >= self.cache_size_limit {
            // Simple strategy: clear half the cache when limit is reached
            let keys_to_remove: Vec<String> = cache.keys().take(cache.len() / 2).cloned().collect();
            for key in keys_to_remove {
                cache.remove(&key);
            }
        }

        cache.insert(email, sender_info);
    }

    /// Clear the sender recognition cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        (cache.len(), self.cache_size_limit)
    }

    /// Find contacts that could match an email for suggestion purposes
    pub async fn suggest_contacts_for_email(&self, email_prefix: &str, limit: usize) -> ContactsResult<Vec<Contact>> {
        self.contacts_manager.find_contacts_by_email_prefix(email_prefix, limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contacts::{ContactEmail, ContactSource, ContactsDatabase, ContactsManager};
    use crate::oauth2::TokenManager;
    use tempfile;

    async fn create_test_service() -> SenderRecognitionService {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_contacts.db");
        let database_url = format!("sqlite:{}", db_path.display());
        
        let database = ContactsDatabase::new(&database_url).await.unwrap();
        let token_manager = TokenManager::new(&database_url).await.unwrap();
        let contacts_manager = ContactsManager::new(database, token_manager).await.unwrap();
        
        SenderRecognitionService::new(Arc::new(contacts_manager))
    }

    #[tokio::test]
    async fn test_extract_email_address() {
        let service = create_test_service().await;
        
        assert_eq!(
            service.extract_email_address("john@example.com"),
            "john@example.com"
        );
        
        assert_eq!(
            service.extract_email_address("John Doe <john@example.com>"),
            "john@example.com"
        );
        
        assert_eq!(
            service.extract_email_address("\"John Doe\" <john@example.com>"),
            "john@example.com"
        );
    }

    #[tokio::test]
    async fn test_sender_info_from_email() {
        let sender_info = SenderInfo::from_email("john@example.com");
        assert_eq!(sender_info.display_name, "john@example.com");
        assert!(!sender_info.is_known_contact);
        assert!(sender_info.contact_name.is_none());

        let sender_info = SenderInfo::from_email("John Doe <john@example.com>");
        assert_eq!(sender_info.display_name, "John Doe");
        assert!(!sender_info.is_known_contact);
    }

    #[tokio::test]
    async fn test_sender_info_from_contact() {
        let mut contact = Contact::new(
            "1".to_string(),
            ContactSource::Local,
            "John Doe".to_string(),
        );
        contact.first_name = Some("John".to_string());
        contact.last_name = Some("Doe".to_string());
        contact.company = Some("Acme Corp".to_string());
        contact.emails.push(ContactEmail::new("john@example.com".to_string(), "work".to_string()));

        let sender_info = SenderInfo::from_contact(&contact, "john@example.com");
        assert_eq!(sender_info.display_name, "John Doe");
        assert!(sender_info.is_known_contact);
        assert_eq!(sender_info.contact_name, Some("John Doe".to_string()));
        assert_eq!(sender_info.company, Some("Acme Corp".to_string()));
    }
}