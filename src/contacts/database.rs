use crate::contacts::{
    AddressBookStats, ContactSearchCriteria, ContactSource, ContactsError, ContactsResult,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite};

/// Contact information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: Option<i64>,
    pub external_id: String, // ID from provider (Google/Outlook)
    pub source: ContactSource,
    pub display_name: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub company: Option<String>,
    pub job_title: Option<String>,
    pub emails: Vec<ContactEmail>,
    pub phones: Vec<ContactPhone>,
    pub groups: Vec<String>,
    pub notes: Option<String>,
    pub photo_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub synced_at: Option<DateTime<Utc>>,
    pub etag: Option<String>, // For sync optimization
}

impl Contact {
    pub fn new(external_id: String, source: ContactSource, display_name: String) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            external_id,
            source,
            display_name,
            first_name: None,
            last_name: None,
            company: None,
            job_title: None,
            emails: Vec::new(),
            phones: Vec::new(),
            groups: Vec::new(),
            notes: None,
            photo_url: None,
            created_at: now,
            updated_at: now,
            synced_at: None,
            etag: None,
        }
    }

    pub fn primary_email(&self) -> Option<&ContactEmail> {
        self.emails
            .iter()
            .find(|e| e.is_primary)
            .or_else(|| self.emails.first())
    }

    pub fn primary_phone(&self) -> Option<&ContactPhone> {
        self.phones
            .iter()
            .find(|p| p.is_primary)
            .or_else(|| self.phones.first())
    }

    pub fn full_name(&self) -> String {
        match (&self.first_name, &self.last_name) {
            (Some(first), Some(last)) => format!("{} {}", first, last),
            (Some(first), None) => first.clone(),
            (None, Some(last)) => last.clone(),
            (None, None) => self.display_name.clone(),
        }
    }

    pub fn search_text(&self) -> String {
        let mut text = vec![self.display_name.clone(), self.full_name()];

        if let Some(company) = &self.company {
            text.push(company.clone());
        }

        for email in &self.emails {
            text.push(email.address.clone());
        }

        for phone in &self.phones {
            text.push(phone.number.clone());
        }

        text.join(" ").to_lowercase()
    }
}

/// Contact email address
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContactEmail {
    pub address: String,
    pub label: String, // "work", "home", "other"
    pub is_primary: bool,
}

impl ContactEmail {
    pub fn new(address: String, label: String) -> Self {
        Self {
            address,
            label,
            is_primary: false,
        }
    }

    pub fn primary(address: String, label: String) -> Self {
        Self {
            address,
            label,
            is_primary: true,
        }
    }
}

/// Contact phone number
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContactPhone {
    pub number: String,
    pub label: String, // "mobile", "work", "home", "other"
    pub is_primary: bool,
}

impl ContactPhone {
    pub fn new(number: String, label: String) -> Self {
        Self {
            number,
            label,
            is_primary: false,
        }
    }

    pub fn primary(number: String, label: String) -> Self {
        Self {
            number,
            label,
            is_primary: true,
        }
    }
}

/// Contact group/label
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactGroup {
    pub id: Option<i64>,
    pub external_id: String,
    pub source: ContactSource,
    pub name: String,
    pub description: Option<String>,
    pub contact_count: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ContactGroup {
    pub fn new(external_id: String, source: ContactSource, name: String) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            external_id,
            source,
            name,
            description: None,
            contact_count: 0,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Contacts database manager
pub struct ContactsDatabase {
    pub pool: Pool<Sqlite>,
}

impl ContactsDatabase {
    /// Create a new contacts database
    pub async fn new(database_url: &str) -> ContactsResult<Self> {
        let pool = Pool::<Sqlite>::connect(database_url)
            .await
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        let db = Self { pool };
        db.init_tables().await?;
        Ok(db)
    }

    /// Initialize database tables
    async fn init_tables(&self) -> ContactsResult<()> {
        // Contacts table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS contacts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                external_id TEXT NOT NULL,
                source_type TEXT NOT NULL,
                source_account_id TEXT,
                display_name TEXT NOT NULL,
                first_name TEXT,
                last_name TEXT,
                company TEXT,
                job_title TEXT,
                notes TEXT,
                photo_url TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                synced_at TEXT,
                etag TEXT,
                UNIQUE(external_id, source_type, source_account_id)
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        // Contact emails table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS contact_emails (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                contact_id INTEGER NOT NULL,
                address TEXT NOT NULL,
                label TEXT NOT NULL DEFAULT 'other',
                is_primary BOOLEAN NOT NULL DEFAULT 0,
                FOREIGN KEY (contact_id) REFERENCES contacts (id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        // Contact phones table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS contact_phones (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                contact_id INTEGER NOT NULL,
                number TEXT NOT NULL,
                label TEXT NOT NULL DEFAULT 'other',
                is_primary BOOLEAN NOT NULL DEFAULT 0,
                FOREIGN KEY (contact_id) REFERENCES contacts (id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        // Contact groups table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS contact_groups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                external_id TEXT NOT NULL,
                source_type TEXT NOT NULL,
                source_account_id TEXT,
                name TEXT NOT NULL,
                description TEXT,
                contact_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(external_id, source_type, source_account_id)
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        // Contact group memberships table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS contact_group_memberships (
                contact_id INTEGER NOT NULL,
                group_id INTEGER NOT NULL,
                PRIMARY KEY (contact_id, group_id),
                FOREIGN KEY (contact_id) REFERENCES contacts (id) ON DELETE CASCADE,
                FOREIGN KEY (group_id) REFERENCES contact_groups (id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        // Create indexes for better search performance
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_contacts_display_name ON contacts (display_name)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_contact_emails_address ON contact_emails (address)",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_contacts_source ON contacts (source_type, source_account_id)")
            .execute(&self.pool)
            .await
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Store or update a contact
    pub async fn store_contact(&self, contact: &mut Contact) -> ContactsResult<()> {
        let (source_type, source_account_id) = match &contact.source {
            ContactSource::Google { account_id } => ("google", Some(account_id.as_str())),
            ContactSource::Outlook { account_id } => ("outlook", Some(account_id.as_str())),
            ContactSource::Local => ("local", None),
        };

        // Insert or update contact
        let result = sqlx::query(
            r#"
            INSERT INTO contacts (
                external_id, source_type, source_account_id, display_name,
                first_name, last_name, company, job_title, notes, photo_url,
                created_at, updated_at, synced_at, etag
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(external_id, source_type, source_account_id) DO UPDATE SET
                display_name = excluded.display_name,
                first_name = excluded.first_name,
                last_name = excluded.last_name,
                company = excluded.company,
                job_title = excluded.job_title,
                notes = excluded.notes,
                photo_url = excluded.photo_url,
                updated_at = excluded.updated_at,
                synced_at = excluded.synced_at,
                etag = excluded.etag
            "#,
        )
        .bind(&contact.external_id)
        .bind(source_type)
        .bind(source_account_id)
        .bind(&contact.display_name)
        .bind(&contact.first_name)
        .bind(&contact.last_name)
        .bind(&contact.company)
        .bind(&contact.job_title)
        .bind(&contact.notes)
        .bind(&contact.photo_url)
        .bind(&contact.created_at.to_rfc3339())
        .bind(&contact.updated_at.to_rfc3339())
        .bind(contact.synced_at.map(|dt| dt.to_rfc3339()))
        .bind(&contact.etag)
        .execute(&self.pool)
        .await
        .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        // Get the contact ID
        let contact_id = if contact.id.is_none() {
            result.last_insert_rowid()
        } else {
            contact.id.unwrap()
        };

        contact.id = Some(contact_id);

        // Clear existing emails and phones
        sqlx::query("DELETE FROM contact_emails WHERE contact_id = ?")
            .bind(contact_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        sqlx::query("DELETE FROM contact_phones WHERE contact_id = ?")
            .bind(contact_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        // Insert emails
        for email in &contact.emails {
            sqlx::query(
                "INSERT INTO contact_emails (contact_id, address, label, is_primary) VALUES (?, ?, ?, ?)"
            )
            .bind(contact_id)
            .bind(&email.address)
            .bind(&email.label)
            .bind(email.is_primary)
            .execute(&self.pool)
            .await
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;
        }

        // Insert phones
        for phone in &contact.phones {
            sqlx::query(
                "INSERT INTO contact_phones (contact_id, number, label, is_primary) VALUES (?, ?, ?, ?)"
            )
            .bind(contact_id)
            .bind(&phone.number)
            .bind(&phone.label)
            .bind(phone.is_primary)
            .execute(&self.pool)
            .await
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;
        }

        Ok(())
    }

    /// Search contacts
    pub async fn search_contacts(
        &self,
        criteria: &ContactSearchCriteria,
    ) -> ContactsResult<Vec<Contact>> {
        let mut query = "SELECT c.*, 
                               GROUP_CONCAT(DISTINCT e.address || '|' || e.label || '|' || e.is_primary) as emails,
                               GROUP_CONCAT(DISTINCT p.number || '|' || p.label || '|' || p.is_primary) as phones
                        FROM contacts c
                        LEFT JOIN contact_emails e ON c.id = e.contact_id
                        LEFT JOIN contact_phones p ON c.id = p.contact_id
                        WHERE 1=1".to_string();

        let mut params: Vec<String> = Vec::new();

        if let Some(query_text) = &criteria.query {
            query.push_str(" AND (c.display_name LIKE ? OR c.first_name LIKE ? OR c.last_name LIKE ? OR c.company LIKE ?)");
            let pattern = format!("%{}%", query_text);
            params.extend(vec![
                pattern.clone(),
                pattern.clone(),
                pattern.clone(),
                pattern,
            ]);
        }

        if let Some(email) = &criteria.email {
            query.push_str(
                " AND c.id IN (SELECT contact_id FROM contact_emails WHERE address LIKE ?)",
            );
            params.push(format!("%{}%", email));
        }

        if let Some(source) = &criteria.source {
            match source {
                ContactSource::Google { account_id } => {
                    query.push_str(" AND c.source_type = 'google' AND c.source_account_id = ?");
                    params.push(account_id.clone());
                }
                ContactSource::Outlook { account_id } => {
                    query.push_str(" AND c.source_type = 'outlook' AND c.source_account_id = ?");
                    params.push(account_id.clone());
                }
                ContactSource::Local => {
                    query.push_str(" AND c.source_type = 'local'");
                }
            }
        }

        query.push_str(" GROUP BY c.id ORDER BY c.display_name");

        if let Some(limit) = criteria.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        let mut query_builder = sqlx::query(&query);
        for param in &params {
            query_builder = query_builder.bind(param);
        }

        let rows = query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        let mut contacts = Vec::new();
        for row in rows {
            let contact = self.contact_from_row(&row)?;
            contacts.push(contact);
        }

        Ok(contacts)
    }

    /// Get contact by ID
    pub async fn get_contact(&self, id: i64) -> ContactsResult<Option<Contact>> {
        let query = "SELECT c.*, 
                           GROUP_CONCAT(DISTINCT e.address || '|' || e.label || '|' || e.is_primary) as emails,
                           GROUP_CONCAT(DISTINCT p.number || '|' || p.label || '|' || p.is_primary) as phones
                    FROM contacts c
                    LEFT JOIN contact_emails e ON c.id = e.contact_id
                    LEFT JOIN contact_phones p ON c.id = p.contact_id
                    WHERE c.id = ?
                    GROUP BY c.id";

        let row = sqlx::query(query)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        if let Some(row) = row {
            Ok(Some(self.contact_from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    /// Delete contact
    pub async fn delete_contact(&self, id: i64) -> ContactsResult<()> {
        sqlx::query("DELETE FROM contacts WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Find contact by email address
    pub async fn find_contact_by_email(&self, email: &str) -> ContactsResult<Option<Contact>> {
        let query = "SELECT c.*, 
                           GROUP_CONCAT(DISTINCT e.address || '|' || e.label || '|' || e.is_primary) as emails,
                           GROUP_CONCAT(DISTINCT p.number || '|' || p.label || '|' || p.is_primary) as phones
                    FROM contacts c
                    LEFT JOIN contact_emails e ON c.id = e.contact_id
                    LEFT JOIN contact_phones p ON c.id = p.contact_id
                    WHERE c.id IN (SELECT contact_id FROM contact_emails WHERE address = ?)
                    GROUP BY c.id
                    LIMIT 1";

        let row = sqlx::query(query)
            .bind(email)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        if let Some(row) = row {
            Ok(Some(self.contact_from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    /// Find contacts by partial email match (for autocomplete)
    pub async fn find_contacts_by_email_prefix(&self, email_prefix: &str, limit: usize) -> ContactsResult<Vec<Contact>> {
        let query = "SELECT c.*, 
                           GROUP_CONCAT(DISTINCT e.address || '|' || e.label || '|' || e.is_primary) as emails,
                           GROUP_CONCAT(DISTINCT p.number || '|' || p.label || '|' || p.is_primary) as phones
                    FROM contacts c
                    LEFT JOIN contact_emails e ON c.id = e.contact_id
                    LEFT JOIN contact_phones p ON c.id = p.contact_id
                    WHERE c.id IN (SELECT contact_id FROM contact_emails WHERE address LIKE ?)
                    GROUP BY c.id
                    ORDER BY c.display_name
                    LIMIT ?";

        let rows = sqlx::query(query)
            .bind(format!("{}%", email_prefix))
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        let mut contacts = Vec::new();
        for row in rows {
            let contact = self.contact_from_row(&row)?;
            contacts.push(contact);
        }

        Ok(contacts)
    }

    /// Get address book statistics
    pub async fn get_stats(&self) -> ContactsResult<AddressBookStats> {
        let total_contacts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM contacts")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        let google_contacts: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM contacts WHERE source_type = 'google'")
                .fetch_one(&self.pool)
                .await
                .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        let outlook_contacts: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM contacts WHERE source_type = 'outlook'")
                .fetch_one(&self.pool)
                .await
                .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        let local_contacts: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM contacts WHERE source_type = 'local'")
                .fetch_one(&self.pool)
                .await
                .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        let contacts_with_email: i64 =
            sqlx::query_scalar("SELECT COUNT(DISTINCT contact_id) FROM contact_emails")
                .fetch_one(&self.pool)
                .await
                .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        let contacts_with_phone: i64 =
            sqlx::query_scalar("SELECT COUNT(DISTINCT contact_id) FROM contact_phones")
                .fetch_one(&self.pool)
                .await
                .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        let groups_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM contact_groups")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        let last_sync: Option<String> =
            sqlx::query_scalar("SELECT MAX(synced_at) FROM contacts WHERE synced_at IS NOT NULL")
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ContactsError::DatabaseError(e.to_string()))?
                .flatten();

        let last_sync_parsed = last_sync.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        });

        Ok(AddressBookStats {
            total_contacts: total_contacts as usize,
            google_contacts: google_contacts as usize,
            outlook_contacts: outlook_contacts as usize,
            local_contacts: local_contacts as usize,
            contacts_with_email: contacts_with_email as usize,
            contacts_with_phone: contacts_with_phone as usize,
            groups_count: groups_count as usize,
            last_sync: last_sync_parsed,
        })
    }

    /// Helper method to convert row to contact
    pub fn contact_from_row(&self, row: &sqlx::sqlite::SqliteRow) -> ContactsResult<Contact> {
        let id: i64 = row
            .try_get("id")
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;
        let external_id: String = row
            .try_get("external_id")
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;
        let source_type: String = row
            .try_get("source_type")
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;
        let source_account_id: Option<String> = row
            .try_get("source_account_id")
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        let source = match source_type.as_str() {
            "google" => ContactSource::Google {
                account_id: source_account_id.unwrap_or_default(),
            },
            "outlook" => ContactSource::Outlook {
                account_id: source_account_id.unwrap_or_default(),
            },
            "local" => ContactSource::Local,
            _ => ContactSource::Local,
        };

        let display_name: String = row
            .try_get("display_name")
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;
        let first_name: Option<String> = row
            .try_get("first_name")
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;
        let last_name: Option<String> = row
            .try_get("last_name")
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;
        let company: Option<String> = row
            .try_get("company")
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;
        let job_title: Option<String> = row
            .try_get("job_title")
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;
        let notes: Option<String> = row
            .try_get("notes")
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;
        let photo_url: Option<String> = row
            .try_get("photo_url")
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        let created_at_str: String = row
            .try_get("created_at")
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;
        let updated_at_str: String = row
            .try_get("updated_at")
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;
        let synced_at_str: Option<String> = row
            .try_get("synced_at")
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;
        let etag: Option<String> = row
            .try_get("etag")
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?;

        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?
            .with_timezone(&Utc);
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?
            .with_timezone(&Utc);
        let synced_at = synced_at_str
            .map(|s| DateTime::parse_from_rfc3339(&s))
            .transpose()
            .map_err(|e| ContactsError::DatabaseError(e.to_string()))?
            .map(|dt| dt.with_timezone(&Utc));

        // Parse emails
        let emails_str: Option<String> = row.try_get("emails").ok();
        let mut emails = Vec::new();
        if let Some(emails_data) = emails_str {
            for email_data in emails_data.split(',') {
                let parts: Vec<&str> = email_data.split('|').collect();
                if parts.len() >= 3 {
                    emails.push(ContactEmail {
                        address: parts[0].to_string(),
                        label: parts[1].to_string(),
                        is_primary: parts[2] == "1",
                    });
                }
            }
        }

        // Parse phones
        let phones_str: Option<String> = row.try_get("phones").ok();
        let mut phones = Vec::new();
        if let Some(phones_data) = phones_str {
            for phone_data in phones_data.split(',') {
                let parts: Vec<&str> = phone_data.split('|').collect();
                if parts.len() >= 3 {
                    phones.push(ContactPhone {
                        number: parts[0].to_string(),
                        label: parts[1].to_string(),
                        is_primary: parts[2] == "1",
                    });
                }
            }
        }

        Ok(Contact {
            id: Some(id),
            external_id,
            source,
            display_name,
            first_name,
            last_name,
            company,
            job_title,
            emails,
            phones,
            groups: Vec::new(), // TODO: Load groups
            notes,
            photo_url,
            created_at,
            updated_at,
            synced_at,
            etag,
        })
    }
}
