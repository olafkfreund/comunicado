pub mod database;
pub mod providers;
pub mod manager;
pub mod sync;
pub mod ui;

pub use database::{ContactsDatabase, Contact, ContactEmail, ContactPhone, ContactGroup};
pub use providers::{ContactsProvider, GoogleContactsProvider, OutlookContactsProvider};
pub use manager::ContactsManager;
pub use sync::{ContactsSyncEngine, SyncProgress as ContactsSyncProgress};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Contact management errors
#[derive(Error, Debug)]
pub enum ContactsError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    
    #[error("API error: {0}")]
    ApiError(String),
    
    #[error("Sync error: {0}")]
    SyncError(String),
    
    #[error("Authentication error: {0}")]
    AuthError(String),
    
    #[error("Invalid contact data: {0}")]
    InvalidData(String),
    
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
}

pub type ContactsResult<T> = Result<T, ContactsError>;

/// Contact source (which provider/account)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContactSource {
    Google { account_id: String },
    Outlook { account_id: String },
    Local,
}

impl ContactSource {
    pub fn account_id(&self) -> Option<&str> {
        match self {
            ContactSource::Google { account_id } => Some(account_id),
            ContactSource::Outlook { account_id } => Some(account_id),
            ContactSource::Local => None,
        }
    }
    
    pub fn provider_name(&self) -> &str {
        match self {
            ContactSource::Google { .. } => "Google",
            ContactSource::Outlook { .. } => "Outlook",
            ContactSource::Local => "Local",
        }
    }
}

/// Contact search criteria
#[derive(Debug, Clone)]
pub struct ContactSearchCriteria {
    pub query: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
    pub phone: Option<String>,
    pub group: Option<String>,
    pub source: Option<ContactSource>,
    pub limit: Option<usize>,
}

impl ContactSearchCriteria {
    pub fn new() -> Self {
        Self {
            query: None,
            email: None,
            name: None,
            phone: None,
            group: None,
            source: None,
            limit: Some(50),
        }
    }
    
    pub fn with_query(mut self, query: String) -> Self {
        self.query = Some(query);
        self
    }
    
    pub fn with_email(mut self, email: String) -> Self {
        self.email = Some(email);
        self
    }
    
    pub fn with_source(mut self, source: ContactSource) -> Self {
        self.source = Some(source);
        self
    }
    
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

impl Default for ContactSearchCriteria {
    fn default() -> Self {
        Self::new()
    }
}

/// Address book statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressBookStats {
    pub total_contacts: usize,
    pub google_contacts: usize,
    pub outlook_contacts: usize,
    pub local_contacts: usize,
    pub contacts_with_email: usize,
    pub contacts_with_phone: usize,
    pub groups_count: usize,
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for AddressBookStats {
    fn default() -> Self {
        Self {
            total_contacts: 0,
            google_contacts: 0,
            outlook_contacts: 0,
            local_contacts: 0,
            contacts_with_email: 0,
            contacts_with_phone: 0,
            groups_count: 0,
            last_sync: None,
        }
    }
}