use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use sqlx::{SqlitePool, Row};
use tracing::{info, debug};
use serde::{Deserialize, Serialize};

use crate::mobile::kde_connect::types::{SmsMessage, SmsConversation, ContactInfo, MessageType, Attachment};
use crate::mobile::Result;

/// Comprehensive SMS/MMS message storage with SQLite backend
pub struct MessageStore {
    pool: SqlitePool,
    database_path: PathBuf,
}

/// Statistics about stored messages and conversations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageStoreStats {
    pub conversation_count: usize,
    pub message_count: usize,
    pub unread_conversation_count: usize,
    pub active_notification_count: usize,
    pub database_size_bytes: u64,
    pub oldest_message_date: Option<DateTime<Utc>>,
    pub newest_message_date: Option<DateTime<Utc>>,
    pub storage_efficiency: f64, // Percentage of space utilized efficiently
}

/// Query parameters for filtering messages and conversations
#[derive(Debug, Clone, Default)]
pub struct MessageQuery {
    pub conversation_id: Option<i64>,
    pub contact_filter: Option<String>,
    pub message_type: Option<MessageType>,
    pub read_status: Option<bool>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub search_text: Option<String>,
}

/// Database operations for SMS/MMS storage
impl MessageStore {
    /// Create a new message store with the specified database path
    pub async fn new<P: AsRef<Path>>(database_path: P) -> Result<Self> {
        let path = database_path.as_ref().to_path_buf();
        
        // Ensure the parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(crate::mobile::MobileError::IoError)?;
        }

        let database_url = format!("sqlite:{}", path.display());
        info!("Initializing SMS/MMS message store at: {}", database_url);

        let pool = SqlitePool::connect(&database_url).await
            .map_err(crate::mobile::MobileError::DatabaseError)?;

        let store = Self {
            pool,
            database_path: path,
        };

        // Initialize the database schema
        store.initialize_schema().await?;
        
        info!("SMS/MMS message store initialized successfully");
        Ok(store)
    }

    /// Initialize the database schema with all necessary tables
    async fn initialize_schema(&self) -> Result<()> {
        debug!("Initializing database schema for SMS/MMS storage");

        // Create conversations table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS conversations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                thread_id INTEGER NOT NULL UNIQUE,
                display_name TEXT NOT NULL,
                last_message_date INTEGER NOT NULL,
                unread_count INTEGER NOT NULL DEFAULT 0,
                is_archived BOOLEAN NOT NULL DEFAULT FALSE,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                metadata TEXT -- JSON metadata for additional info
            )
        "#)
        .execute(&self.pool)
        .await
        .map_err(crate::mobile::MobileError::DatabaseError)?;

        // Create messages table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY,
                conversation_id INTEGER NOT NULL,
                body TEXT NOT NULL,
                message_type TEXT NOT NULL,
                is_read BOOLEAN NOT NULL DEFAULT FALSE,
                date_sent INTEGER NOT NULL,
                date_received INTEGER NOT NULL,
                sub_id INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY (conversation_id) REFERENCES conversations (id) ON DELETE CASCADE
            )
        "#)
        .execute(&self.pool)
        .await
        .map_err(crate::mobile::MobileError::DatabaseError)?;

        // Create contacts table for conversation participants
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS conversation_contacts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                conversation_id INTEGER NOT NULL,
                phone_number TEXT NOT NULL,
                display_name TEXT,
                is_primary BOOLEAN NOT NULL DEFAULT FALSE,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (conversation_id) REFERENCES conversations (id) ON DELETE CASCADE,
                UNIQUE(conversation_id, phone_number)
            )
        "#)
        .execute(&self.pool)
        .await
        .map_err(crate::mobile::MobileError::DatabaseError)?;

        // Create attachments table for MMS content
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS message_attachments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                message_id INTEGER NOT NULL,
                filename TEXT NOT NULL,
                mime_type TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                data BLOB,
                is_downloaded BOOLEAN NOT NULL DEFAULT FALSE,
                download_url TEXT,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (message_id) REFERENCES messages (id) ON DELETE CASCADE
            )
        "#)
        .execute(&self.pool)
        .await
        .map_err(crate::mobile::MobileError::DatabaseError)?;

        // Create indexes for better query performance
        let indexes = [
            "CREATE INDEX IF NOT EXISTS idx_conversations_thread_id ON conversations (thread_id)",
            "CREATE INDEX IF NOT EXISTS idx_conversations_last_message_date ON conversations (last_message_date DESC)",
            "CREATE INDEX IF NOT EXISTS idx_conversations_unread ON conversations (unread_count) WHERE unread_count > 0",
            "CREATE INDEX IF NOT EXISTS idx_messages_conversation_id ON messages (conversation_id)",
            "CREATE INDEX IF NOT EXISTS idx_messages_date_sent ON messages (date_sent DESC)",
            "CREATE INDEX IF NOT EXISTS idx_messages_read_status ON messages (is_read)",
            "CREATE INDEX IF NOT EXISTS idx_messages_body_fts ON messages (body)",
            "CREATE INDEX IF NOT EXISTS idx_contacts_conversation_id ON conversation_contacts (conversation_id)",
            "CREATE INDEX IF NOT EXISTS idx_contacts_phone_number ON conversation_contacts (phone_number)",
            "CREATE INDEX IF NOT EXISTS idx_attachments_message_id ON message_attachments (message_id)",
        ];

        for index_sql in indexes {
            sqlx::query(index_sql)
                .execute(&self.pool)
                .await
                .map_err(crate::mobile::MobileError::DatabaseError)?;
        }

        // Enable Write-Ahead Logging for better concurrent access
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&self.pool)
            .await
            .map_err(crate::mobile::MobileError::DatabaseError)?;

        // Enable foreign key constraints
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&self.pool)
            .await
            .map_err(crate::mobile::MobileError::DatabaseError)?;

        debug!("Database schema initialization completed");
        Ok(())
    }

    /// Store a new SMS message, creating conversation if needed
    pub async fn store_message(&self, message: SmsMessage) -> Result<i64> {
        debug!("Storing SMS message: ID={}, Thread={}", message.id, message.thread_id);

        let mut tx = self.pool.begin().await
            .map_err(crate::mobile::MobileError::DatabaseError)?;

        // Find or create conversation
        let conversation_id = self.find_or_create_conversation(&mut tx, &message).await?;

        // Insert the message
        let now = Utc::now().timestamp();
        let message_type_str = match message.message_type {
            MessageType::Sms => "sms",
            MessageType::Mms => "mms",
        };

        sqlx::query(r#"
            INSERT OR REPLACE INTO messages 
            (id, conversation_id, body, message_type, is_read, date_sent, date_received, sub_id, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(message.id)
        .bind(conversation_id)
        .bind(&message.body)
        .bind(message_type_str)
        .bind(message.read)
        .bind(message.date)
        .bind(now * 1000) // Current time as received time
        .bind(message.sub_id)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(crate::mobile::MobileError::DatabaseError)?;

        // Store attachments if any
        for attachment in &message.attachments {
            self.store_attachment(&mut tx, message.id, attachment).await?;
        }

        // Update conversation metadata
        self.update_conversation_metadata(&mut tx, conversation_id, &message).await?;

        tx.commit().await
            .map_err(crate::mobile::MobileError::DatabaseError)?;

        info!("Successfully stored message ID {} in conversation {}", message.id, conversation_id);
        Ok(conversation_id)
    }

    /// Find existing conversation or create a new one
    async fn find_or_create_conversation(
        &self, 
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, 
        message: &SmsMessage
    ) -> Result<i64> {
        // Try to find existing conversation by thread_id
        if let Ok(row) = sqlx::query("SELECT id FROM conversations WHERE thread_id = ?")
            .bind(message.thread_id)
            .fetch_one(&mut **tx)
            .await 
        {
            return Ok(row.get::<i64, _>("id"));
        }

        // Create new conversation
        let now = Utc::now().timestamp();
        let display_name = self.generate_conversation_display_name(&message.addresses);

        let result = sqlx::query(r#"
            INSERT INTO conversations 
            (thread_id, display_name, last_message_date, unread_count, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
        "#)
        .bind(message.thread_id)
        .bind(&display_name)
        .bind(message.date)
        .bind(if message.read { 0 } else { 1 })
        .bind(now)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(crate::mobile::MobileError::DatabaseError)?;

        let conversation_id = result.last_insert_rowid();

        // Store conversation contacts
        for (index, address) in message.addresses.iter().enumerate() {
            let contact_info = ContactInfo::from_address(address);
            sqlx::query(r#"
                INSERT OR IGNORE INTO conversation_contacts 
                (conversation_id, phone_number, display_name, is_primary, created_at)
                VALUES (?, ?, ?, ?, ?)
            "#)
            .bind(conversation_id)
            .bind(&contact_info.address)
            .bind(contact_info.display_name.as_deref())
            .bind(index == 0) // First contact is primary
            .bind(now)
            .execute(&mut **tx)
            .await
            .map_err(crate::mobile::MobileError::DatabaseError)?;
        }

        debug!("Created new conversation {} for thread {}", conversation_id, message.thread_id);
        Ok(conversation_id)
    }

    /// Store message attachment
    async fn store_attachment(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        message_id: i32,
        attachment: &Attachment,
    ) -> Result<()> {
        let now = Utc::now().timestamp();
        
        sqlx::query(r#"
            INSERT INTO message_attachments 
            (message_id, filename, mime_type, file_size, data, is_downloaded, download_url, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(message_id)
        .bind(&attachment.filename)
        .bind(&attachment.mime_type)
        .bind(attachment.file_size)
        .bind(attachment.data.as_deref())
        .bind(attachment.data.is_some())
        .bind(attachment.download_url.as_deref())
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(crate::mobile::MobileError::DatabaseError)?;
        
        Ok(())
    }

    /// Update conversation metadata after adding a message
    async fn update_conversation_metadata(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        conversation_id: i64,
        message: &SmsMessage,
    ) -> Result<()> {
        let now = Utc::now().timestamp();
        
        // Update last message date and increment unread count if needed
        if message.read {
            sqlx::query(r#"
                UPDATE conversations 
                SET last_message_date = ?, updated_at = ?
                WHERE id = ?
            "#)
            .bind(message.date)
            .bind(now)
            .bind(conversation_id)
            .execute(&mut **tx)
            .await
            .map_err(crate::mobile::MobileError::DatabaseError)?;
        } else {
            sqlx::query(r#"
                UPDATE conversations 
                SET last_message_date = ?, unread_count = unread_count + 1, updated_at = ?
                WHERE id = ?
            "#)
            .bind(message.date)
            .bind(now)
            .bind(conversation_id)
            .execute(&mut **tx)
            .await
            .map_err(crate::mobile::MobileError::DatabaseError)?;
        }
        
        Ok(())
    }

    /// Generate display name for conversation based on participants
    fn generate_conversation_display_name(&self, addresses: &[String]) -> String {
        if addresses.is_empty() {
            return "Unknown".to_string();
        }

        if addresses.len() == 1 {
            let contact = ContactInfo::from_address(&addresses[0]);
            return contact.display_text().to_string();
        }

        // For group conversations, show first few participants
        let names: Vec<String> = addresses.iter()
            .take(3)
            .map(|addr| ContactInfo::from_address(addr).display_text().to_string())
            .collect();

        if addresses.len() > 3 {
            format!("{} and {} others", names.join(", "), addresses.len() - 3)
        } else {
            names.join(", ")
        }
    }

    /// Retrieve conversations with optional filtering and pagination
    pub async fn get_conversations(&self, query: &MessageQuery) -> Result<Vec<SmsConversation>> {
        debug!("Retrieving conversations with query: {:?}", query);

        let mut sql = String::from(r#"
            SELECT c.id, c.thread_id, c.display_name, c.last_message_date, 
                   c.unread_count, c.is_archived, c.created_at,
                   GROUP_CONCAT(cc.phone_number, ',') as phone_numbers,
                   GROUP_CONCAT(cc.display_name, ',') as contact_names
            FROM conversations c
            LEFT JOIN conversation_contacts cc ON c.id = cc.conversation_id
        "#);

        let mut conditions = Vec::new();
        let mut bindings = Vec::new();

        if let Some(contact_filter) = &query.contact_filter {
            conditions.push("(cc.phone_number LIKE ? OR cc.display_name LIKE ?)");
            let pattern = format!("%{}%", contact_filter);
            bindings.push(pattern.clone());
            bindings.push(pattern);
        }

        if let Some(date_from) = query.date_from {
            conditions.push("c.last_message_date >= ?");
            bindings.push((date_from.timestamp() * 1000).to_string());
        }

        if let Some(date_to) = query.date_to {
            conditions.push("c.last_message_date <= ?");
            bindings.push((date_to.timestamp() * 1000).to_string());
        }

        if !conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }

        sql.push_str(" GROUP BY c.id ORDER BY c.last_message_date DESC");

        if let Some(limit) = query.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
            if let Some(offset) = query.offset {
                sql.push_str(&format!(" OFFSET {}", offset));
            }
        }

        let mut query_builder = sqlx::query(&sql);
        for binding in bindings {
            query_builder = query_builder.bind(binding);
        }

        let rows = query_builder.fetch_all(&self.pool).await
            .map_err(crate::mobile::MobileError::DatabaseError)?;

        let mut conversations = Vec::new();
        for row in rows {
            let phone_numbers: String = row.get("phone_numbers");
            let contact_names: Option<String> = row.try_get("contact_names").ok();

            let participants = self.parse_conversation_participants(&phone_numbers, contact_names.as_deref());

            let conversation = SmsConversation {
                id: row.get::<i64, _>("id"),
                thread_id: row.get::<i64, _>("thread_id"),
                display_name: row.get("display_name"),
                participants,
                last_message_date: DateTime::from_timestamp(row.get::<i64, _>("last_message_date") / 1000, 0)
                    .unwrap_or_else(|| Utc::now()),
                unread_count: row.get::<i64, _>("unread_count") as i32,
                is_archived: row.get("is_archived"),
                message_count: 0, // Will be filled by separate query if needed
                messages: Vec::new(), // Not loaded by default for performance
            };

            conversations.push(conversation);
        }

        debug!("Retrieved {} conversations", conversations.len());
        Ok(conversations)
    }

    /// Parse conversation participants from database results
    fn parse_conversation_participants(&self, phone_numbers: &str, contact_names: Option<&str>) -> Vec<ContactInfo> {
        let phones: Vec<&str> = phone_numbers.split(',').collect();
        let names: Vec<&str> = contact_names
            .map(|names| names.split(',').collect())
            .unwrap_or_else(|| vec![""; phones.len()]);

        phones.into_iter()
            .zip(names.into_iter())
            .map(|(phone, name)| {
                ContactInfo::new(
                    phone.to_string(),
                    if name.is_empty() { None } else { Some(name.to_string()) }
                )
            })
            .collect()
    }

    /// Get messages for a specific conversation
    pub async fn get_messages(&self, conversation_id: i64, query: &MessageQuery) -> Result<Vec<SmsMessage>> {
        debug!("Retrieving messages for conversation {}", conversation_id);

        let mut sql = String::from(r#"
            SELECT m.id, m.body, m.message_type, m.is_read, m.date_sent, m.sub_id,
                   c.thread_id,
                   GROUP_CONCAT(cc.phone_number, ',') as addresses
            FROM messages m
            JOIN conversations c ON m.conversation_id = c.id
            LEFT JOIN conversation_contacts cc ON c.id = cc.conversation_id
            WHERE m.conversation_id = ?
        "#);

        let mut bindings = vec![conversation_id.to_string()];
        let mut conditions = Vec::new();

        if let Some(message_type) = &query.message_type {
            let type_str = match message_type {
                MessageType::Sms => "sms",
                MessageType::Mms => "mms",
            };
            conditions.push("m.message_type = ?");
            bindings.push(type_str.to_string());
        }

        if let Some(read_status) = query.read_status {
            conditions.push("m.is_read = ?");
            bindings.push(read_status.to_string());
        }

        if let Some(search_text) = &query.search_text {
            conditions.push("m.body LIKE ?");
            bindings.push(format!("%{}%", search_text));
        }

        if let Some(date_from) = query.date_from {
            conditions.push("m.date_sent >= ?");
            bindings.push((date_from.timestamp() * 1000).to_string());
        }

        if let Some(date_to) = query.date_to {
            conditions.push("m.date_sent <= ?");
            bindings.push((date_to.timestamp() * 1000).to_string());
        }

        if !conditions.is_empty() {
            sql.push_str(" AND ");
            sql.push_str(&conditions.join(" AND "));
        }

        sql.push_str(" GROUP BY m.id ORDER BY m.date_sent ASC");

        if let Some(limit) = query.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
            if let Some(offset) = query.offset {
                sql.push_str(&format!(" OFFSET {}", offset));
            }
        }

        let mut query_builder = sqlx::query(&sql);
        for binding in bindings {
            query_builder = query_builder.bind(binding);
        }

        let rows = query_builder.fetch_all(&self.pool).await
            .map_err(crate::mobile::MobileError::DatabaseError)?;

        let mut messages = Vec::new();
        for row in rows {
            let message_type = match row.get::<String, _>("message_type").as_str() {
                "mms" => MessageType::Mms,
                _ => MessageType::Sms,
            };

            let addresses: String = row.get("addresses");
            let address_list: Vec<String> = addresses.split(',').map(|s| s.to_string()).collect();

            // Load attachments for this message
            let attachments = self.get_message_attachments(row.get::<i32, _>("id")).await?;

            let message = SmsMessage {
                id: row.get("id"),
                body: row.get("body"),
                addresses: address_list,
                date: row.get("date_sent"),
                message_type,
                read: row.get("is_read"),
                thread_id: row.get("thread_id"),
                sub_id: row.get("sub_id"),
                attachments,
            };

            messages.push(message);
        }

        debug!("Retrieved {} messages for conversation {}", messages.len(), conversation_id);
        Ok(messages)
    }

    /// Get attachments for a specific message
    async fn get_message_attachments(&self, message_id: i32) -> Result<Vec<Attachment>> {
        let rows = sqlx::query(r#"
            SELECT filename, mime_type, file_size, data, is_downloaded, download_url
            FROM message_attachments 
            WHERE message_id = ?
            ORDER BY id
        "#)
        .bind(message_id)
        .fetch_all(&self.pool)
        .await
        .map_err(crate::mobile::MobileError::DatabaseError)?;

        let attachments = rows.into_iter().map(|row| {
            Attachment {
                part_id: 0, // Not stored separately, use default
                filename: row.get("filename"),
                mime_type: row.get("mime_type"),
                file_size: row.get("file_size"),
                data: row.get("data"),
                download_url: row.get("download_url"),
            }
        }).collect();

        Ok(attachments)
    }

    /// Mark conversation as read (all messages)
    pub async fn mark_conversation_read(&self, conversation_id: i64) -> Result<()> {
        debug!("Marking conversation {} as read", conversation_id);

        let mut tx = self.pool.begin().await
            .map_err(crate::mobile::MobileError::DatabaseError)?;

        // Mark all messages in conversation as read
        sqlx::query("UPDATE messages SET is_read = TRUE WHERE conversation_id = ?")
            .bind(conversation_id)
            .execute(&mut *tx)
            .await
            .map_err(crate::mobile::MobileError::DatabaseError)?;

        // Reset unread count for conversation
        let now = Utc::now().timestamp();
        sqlx::query("UPDATE conversations SET unread_count = 0, updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(conversation_id)
            .execute(&mut *tx)
            .await
            .map_err(crate::mobile::MobileError::DatabaseError)?;

        tx.commit().await
            .map_err(crate::mobile::MobileError::DatabaseError)?;

        info!("Marked conversation {} as read", conversation_id);
        Ok(())
    }

    /// Mark specific message as read
    pub async fn mark_message_read(&self, message_id: i32) -> Result<()> {
        debug!("Marking message {} as read", message_id);

        let mut tx = self.pool.begin().await
            .map_err(crate::mobile::MobileError::DatabaseError)?;

        // Get conversation ID and current read status
        let row = sqlx::query("SELECT conversation_id, is_read FROM messages WHERE id = ?")
            .bind(message_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(crate::mobile::MobileError::DatabaseError)?;

        let conversation_id: i64 = row.get("conversation_id");
        let was_read: bool = row.get("is_read");

        if !was_read {
            // Mark message as read
            sqlx::query("UPDATE messages SET is_read = TRUE WHERE id = ?")
                .bind(message_id)
                .execute(&mut *tx)
                .await
                .map_err(crate::mobile::MobileError::DatabaseError)?;

            // Decrement conversation unread count
            let now = Utc::now().timestamp();
            sqlx::query(r#"
                UPDATE conversations 
                SET unread_count = MAX(0, unread_count - 1), updated_at = ? 
                WHERE id = ?
            "#)
            .bind(now)
            .bind(conversation_id)
            .execute(&mut *tx)
            .await
            .map_err(crate::mobile::MobileError::DatabaseError)?;
        }

        tx.commit().await
            .map_err(crate::mobile::MobileError::DatabaseError)?;

        info!("Marked message {} as read", message_id);
        Ok(())
    }

    /// Archive or unarchive a conversation
    pub async fn archive_conversation(&self, conversation_id: i64, archived: bool) -> Result<()> {
        debug!("Setting conversation {} archive status to {}", conversation_id, archived);

        let now = Utc::now().timestamp();
        sqlx::query("UPDATE conversations SET is_archived = ?, updated_at = ? WHERE id = ?")
            .bind(archived)
            .bind(now)
            .bind(conversation_id)
            .execute(&self.pool)
            .await
            .map_err(crate::mobile::MobileError::DatabaseError)?;

        info!("Set conversation {} archive status to {}", conversation_id, archived);
        Ok(())
    }

    /// Delete old messages based on retention policy
    pub async fn cleanup_old_messages(&self, retention_days: i64) -> Result<u64> {
        debug!("Cleaning up messages older than {} days", retention_days);

        let cutoff_timestamp = (Utc::now().timestamp() - (retention_days * 24 * 60 * 60)) * 1000;

        let result = sqlx::query("DELETE FROM messages WHERE date_sent < ?")
            .bind(cutoff_timestamp)
            .execute(&self.pool)
            .await
            .map_err(crate::mobile::MobileError::DatabaseError)?;

        let deleted_count = result.rows_affected();

        // Clean up conversations that no longer have messages
        sqlx::query(r#"
            DELETE FROM conversations 
            WHERE id NOT IN (SELECT DISTINCT conversation_id FROM messages)
        "#)
        .execute(&self.pool)
        .await
        .map_err(crate::mobile::MobileError::DatabaseError)?;

        // Run VACUUM to reclaim space
        sqlx::query("VACUUM")
            .execute(&self.pool)
            .await
            .map_err(crate::mobile::MobileError::DatabaseError)?;

        info!("Cleaned up {} old messages", deleted_count);
        Ok(deleted_count)
    }

    /// Get comprehensive storage statistics
    pub async fn get_stats(&self) -> Result<MessageStoreStats> {
        debug!("Collecting message store statistics");

        // Get conversation count
        let conversation_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM conversations")
            .fetch_one(&self.pool)
            .await
            .map_err(crate::mobile::MobileError::DatabaseError)?;

        // Get message count
        let message_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages")
            .fetch_one(&self.pool)
            .await
            .map_err(crate::mobile::MobileError::DatabaseError)?;

        // Get unread conversation count
        let unread_conversation_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM conversations WHERE unread_count > 0"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(crate::mobile::MobileError::DatabaseError)?;

        // Get database file size
        let database_size_bytes = match tokio::fs::metadata(&self.database_path).await {
            Ok(metadata) => metadata.len(),
            Err(_) => 0,
        };

        // Get date range
        let date_range_row = sqlx::query(r#"
            SELECT 
                MIN(date_sent) as oldest_date,
                MAX(date_sent) as newest_date
            FROM messages
        "#)
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::mobile::MobileError::DatabaseError)?;

        let (oldest_message_date, newest_message_date) = if let Some(row) = date_range_row {
            let oldest: Option<i64> = row.get("oldest_date");
            let newest: Option<i64> = row.get("newest_date");
            
            (
                oldest.and_then(|ts| DateTime::from_timestamp(ts / 1000, 0)),
                newest.and_then(|ts| DateTime::from_timestamp(ts / 1000, 0))
            )
        } else {
            (None, None)
        };

        // Calculate storage efficiency (simplified metric)
        let storage_efficiency = if message_count > 0 {
            let avg_message_size = database_size_bytes as f64 / message_count as f64;
            (avg_message_size / 1024.0).min(1.0) * 100.0 // Cap at 100%
        } else {
            100.0
        };

        let stats = MessageStoreStats {
            conversation_count: conversation_count as usize,
            message_count: message_count as usize,
            unread_conversation_count: unread_conversation_count as usize,
            active_notification_count: unread_conversation_count as usize, // Simplified
            database_size_bytes,
            oldest_message_date,
            newest_message_date,
            storage_efficiency,
        };

        debug!("Message store statistics: {:?}", stats);
        Ok(stats)
    }

    /// Search messages across all conversations
    pub async fn search_messages(&self, search_query: &str, limit: Option<i64>) -> Result<Vec<(SmsMessage, String)>> {
        debug!("Searching messages for query: '{}'", search_query);

        let sql = r#"
            SELECT m.id, m.body, m.message_type, m.is_read, m.date_sent, m.sub_id,
                   c.thread_id, c.display_name as conversation_name,
                   GROUP_CONCAT(cc.phone_number, ',') as addresses
            FROM messages m
            JOIN conversations c ON m.conversation_id = c.id
            LEFT JOIN conversation_contacts cc ON c.id = cc.conversation_id
            WHERE m.body LIKE ?
            GROUP BY m.id
            ORDER BY m.date_sent DESC
            LIMIT ?
        "#;

        let search_pattern = format!("%{}%", search_query);
        let limit_value = limit.unwrap_or(100);

        let rows = sqlx::query(sql)
            .bind(search_pattern)
            .bind(limit_value)
            .fetch_all(&self.pool)
            .await
            .map_err(crate::mobile::MobileError::DatabaseError)?;

        let mut results = Vec::new();
        for row in rows {
            let message_type = match row.get::<String, _>("message_type").as_str() {
                "mms" => MessageType::Mms,
                _ => MessageType::Sms,
            };

            let addresses: String = row.get("addresses");
            let address_list: Vec<String> = addresses.split(',').map(|s| s.to_string()).collect();

            let attachments = self.get_message_attachments(row.get::<i32, _>("id")).await?;

            let message = SmsMessage {
                id: row.get("id"),
                body: row.get("body"),
                addresses: address_list,
                date: row.get("date_sent"),
                message_type,
                read: row.get("is_read"),
                thread_id: row.get("thread_id"),
                sub_id: row.get("sub_id"),
                attachments,
            };

            let conversation_name: String = row.get("conversation_name");
            results.push((message, conversation_name));
        }

        debug!("Found {} messages matching search query", results.len());
        Ok(results)
    }

    /// Get the database pool for advanced operations
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Close the database connection pool
    pub async fn close(&self) {
        info!("Closing SMS/MMS message store");
        self.pool.close().await;
    }
}

// Re-export types for easier access

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use chrono::Utc;

    async fn create_test_store() -> MessageStore {
        MessageStore::new(":memory:").await.unwrap()
    }

    fn create_test_message() -> SmsMessage {
        SmsMessage {
            id: 1,
            body: "Hello, this is a test message!".to_string(),
            addresses: vec!["+1234567890".to_string()],
            date: Utc::now().timestamp() * 1000,
            message_type: MessageType::Sms,
            read: false,
            thread_id: 1,
            sub_id: 1,
            attachments: vec![],
        }
    }

    #[tokio::test]
    async fn test_store_and_retrieve_message() {
        let store = create_test_store().await;
        let message = create_test_message();

        // Store the message
        let conversation_id = store.store_message(message.clone()).await.unwrap();
        assert!(conversation_id > 0);

        // Retrieve conversations
        let query = MessageQuery::default();
        let conversations = store.get_conversations(&query).await.unwrap();
        assert_eq!(conversations.len(), 1);
        
        let conversation = &conversations[0];
        assert_eq!(conversation.thread_id, message.thread_id);
        assert_eq!(conversation.unread_count, 1); // Message was unread
        assert!(!conversation.participants.is_empty());

        // Retrieve messages for the conversation
        let messages = store.get_messages(conversation_id, &query).await.unwrap();
        assert_eq!(messages.len(), 1);
        
        let retrieved_message = &messages[0];
        assert_eq!(retrieved_message.id, message.id);
        assert_eq!(retrieved_message.body, message.body);
        assert_eq!(retrieved_message.thread_id, message.thread_id);
    }

    #[tokio::test]
    async fn test_mark_message_read() {
        let store = create_test_store().await;
        let message = create_test_message();

        // Store unread message
        let _conversation_id = store.store_message(message.clone()).await.unwrap();

        // Mark message as read
        store.mark_message_read(message.id).await.unwrap();

        // Verify conversation unread count decreased
        let query = MessageQuery::default();
        let conversations = store.get_conversations(&query).await.unwrap();
        assert_eq!(conversations[0].unread_count, 0);
    }

    #[tokio::test]
    async fn test_search_messages() {
        let store = create_test_store().await;
        
        let mut message1 = create_test_message();
        message1.id = 1;
        message1.body = "Hello world from Alice".to_string();
        
        let mut message2 = create_test_message();
        message2.id = 2;
        message2.body = "Goodbye world from Bob".to_string();
        message2.thread_id = 2;

        // Store both messages
        store.store_message(message1).await.unwrap();
        store.store_message(message2).await.unwrap();

        // Search for "world"
        let results = store.search_messages("world", Some(10)).await.unwrap();
        assert_eq!(results.len(), 2);

        // Search for "Alice"
        let results = store.search_messages("Alice", Some(10)).await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].0.body.contains("Alice"));
    }

    #[tokio::test]
    async fn test_message_with_attachment() {
        let store = create_test_store().await;
        
        let attachment = Attachment {
            part_id: 1,
            mime_type: "image/jpeg".to_string(),
            filename: "photo.jpg".to_string(),
            file_size: 1024,
            data: Some(vec![0xFF, 0xD8, 0xFF]), // JPEG header
            download_url: None,
        };

        let mut message = create_test_message();
        message.message_type = MessageType::Mms;
        message.attachments = vec![attachment];

        // Store MMS message with attachment
        let conversation_id = store.store_message(message.clone()).await.unwrap();

        // Retrieve and verify attachment
        let query = MessageQuery::default();
        let messages = store.get_messages(conversation_id, &query).await.unwrap();
        assert_eq!(messages.len(), 1);
        
        let retrieved_message = &messages[0];
        assert_eq!(retrieved_message.attachments.len(), 1);
        
        let retrieved_attachment = &retrieved_message.attachments[0];
        assert_eq!(retrieved_attachment.filename, "photo.jpg");
        assert_eq!(retrieved_attachment.mime_type, "image/jpeg");
        assert_eq!(retrieved_attachment.file_size, 1024);
    }

    #[tokio::test]
    async fn test_storage_stats() {
        let store = create_test_store().await;
        
        // Store a few messages
        for i in 1..=5 {
            let mut message = create_test_message();
            message.id = i;
            message.thread_id = i as i64;
            store.store_message(message).await.unwrap();
        }

        let stats = store.get_stats().await.unwrap();
        assert_eq!(stats.conversation_count, 5);
        assert_eq!(stats.message_count, 5);
        assert_eq!(stats.unread_conversation_count, 5); // All messages were unread
        assert!(stats.database_size_bytes > 0);
    }

    #[tokio::test]
    async fn test_conversation_archiving() {
        let store = create_test_store().await;
        let message = create_test_message();

        let conversation_id = store.store_message(message).await.unwrap();

        // Archive the conversation
        store.archive_conversation(conversation_id, true).await.unwrap();

        // Verify it's archived
        let query = MessageQuery::default();
        let conversations = store.get_conversations(&query).await.unwrap();
        assert!(conversations[0].is_archived);

        // Unarchive
        store.archive_conversation(conversation_id, false).await.unwrap();
        let conversations = store.get_conversations(&query).await.unwrap();
        assert!(!conversations[0].is_archived);
    }

    #[tokio::test]
    async fn test_cleanup_old_messages() {
        let store = create_test_store().await;
        
        // Store a message
        let message = create_test_message();
        store.store_message(message).await.unwrap();

        // Verify message exists
        let stats_before = store.get_stats().await.unwrap();
        assert_eq!(stats_before.message_count, 1);

        // Clean up messages older than 0 days (should delete all)
        let deleted_count = store.cleanup_old_messages(0).await.unwrap();
        assert_eq!(deleted_count, 1);

        // Verify message was deleted
        let stats_after = store.get_stats().await.unwrap();
        assert_eq!(stats_after.message_count, 0);
    }
}